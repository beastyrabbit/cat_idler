//! LAI.1 replacement-boundary acceptance harness.
//!
//! The target contracts come from `docs/leader-ai-overhaul/`. Green tests in this
//! file characterize already-canonical boundaries; red tests name behavior that
//! must remain red until the owning LAI implementation card lands.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    leader_ai::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot},
    leader_director::{DirectorPlan, direct_colony},
    movement::{JobDestinationContext, WorldPos, destination_for_job},
    types::BuildingType,
    world_tick::{TilePos, footprint_for, footprint_tiles},
};
use serde::Deserialize;

const CONTRACT_JSON: &str =
    include_str!("../../../docs/leader-ai-overhaul/fixtures/lai1_acceptance_contract.json");
const RELEASE_BASELINE_JSON: &str =
    include_str!("../../../docs/leader-ai-overhaul/fixtures/lai1_release_baseline.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptanceContract {
    schema_version: u32,
    planner_state: PlannerStateContract,
    belief_secrecy: BeliefSecrecyContract,
    spatial_cases: Vec<SpatialCase>,
    site_ref_variants: Vec<String>,
    workshop: WorkshopContract,
    shrine_favor: ShrineFavorContract,
    campaign: CampaignContract,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlannerStateContract {
    required_fields: Vec<String>,
    live_intent_cap: usize,
    terminal_intent_cap: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeliefSecrecyContract {
    reported_food: f64,
    hidden_food_variants: [f64; 2],
    report_level: u8,
    regeneration_visible: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpatialCase {
    category: String,
    objective_kind: String,
    work_kind: String,
    delivery_kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkshopContract {
    anchor: FixturePoint,
    width: i32,
    height: i32,
    tiles: Vec<FixturePoint>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
struct FixturePoint {
    x: i32,
    y: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShrineFavorContract {
    legacy_global_upgrade_points: f64,
    legacy_unspent_research_points: f64,
    expected_favor: f64,
    migration_version: u32,
    forbidden_production_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CampaignContract {
    days: u32,
    seeds_per_population: u32,
    seed_start: u32,
    populations: Vec<String>,
    tick_partitions_seconds: Vec<u32>,
    fresh_minimum_successes: u32,
    established_minimum_successes: u32,
    maximum_performance_regression_percent: u32,
}

fn contract() -> AcceptanceContract {
    serde_json::from_str(CONTRACT_JSON).expect("LAI.1 acceptance contract must be valid JSON")
}

fn legacy_snapshot(authoritative_food: f64) -> LeaderSnapshot {
    LeaderSnapshot {
        population: 15,
        workforce: Some(15.0),
        idle_cats: 6,
        employed_cats: 9,
        resources: LeaderResources {
            food: authoritative_food,
            refined: 5.0,
        },
        food_capacity: 200.0,
        food_drain_per_hour: Some(15.0),
        materials: 0.0,
        materials_capacity: 200.0,
        stone: 30.0,
        stone_capacity: 100.0,
        water: 120.0,
        water_capacity: 200.0,
        water_drain_per_hour: Some(18.0),
        housing: LeaderHousing {
            capacity: 20,
            committed: 0,
        },
        active_hunts: 0,
        active_quarries: 0,
        active_scouts: 0,
        active_water_fetchers: 0,
        has_quarry_site: true,
        has_water_site: true,
        has_frontier: true,
        den_plans_in_flight: 0,
        storage_plans_in_flight: 0,
        storehouse_count: 1,
        storehouse_cap: 2,
        workshops_needing_workers: 0,
        research_huts_needing_workers: None,
        smithies_needing_workers: None,
        has_barracks: None,
        warrior_count: None,
        training_in_flight: None,
        offering_in_flight: None,
        threat_band: None,
        starving: Some(false),
        officers: BTreeMap::new(),
    }
}

fn decision_kinds(plan: &DirectorPlan) -> Vec<&'static str> {
    plan.decisions
        .iter()
        .map(|decision| match decision {
            LeaderDecision::Hunt { .. } => "hunt",
            LeaderDecision::CancelHunts => "cancel_hunts",
            LeaderDecision::FetchWater { .. } => "fetch_water",
            LeaderDecision::Quarry { .. } => "quarry",
            LeaderDecision::Scout { .. } => "scout",
            LeaderDecision::BuildDen => "build_den",
            LeaderDecision::BuildStorage => "build_storage",
            LeaderDecision::AssignWorkshop { .. } => "assign_workshop",
            LeaderDecision::AssignResearch { .. } => "assign_research",
            LeaderDecision::AssignSmithy { .. } => "assign_smithy",
            LeaderDecision::TrainWarrior { .. } => "train_warrior",
            LeaderDecision::CancelTraining => "cancel_training",
            LeaderDecision::Tithe { .. } => "tithe",
            LeaderDecision::Offering => "offering",
        })
        .collect()
}

#[test]
fn planner_state_contract_freezes_versioned_bounds_and_required_sections() {
    let contract = contract();
    assert_eq!(contract.schema_version, 1);
    assert_eq!(contract.planner_state.live_intent_cap, 128);
    assert_eq!(contract.planner_state.terminal_intent_cap, 256);

    let fields = contract
        .planner_state
        .required_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "schemaVersion",
        "planningClock",
        "planningEpoch",
        "beliefStore",
        "liveIntents",
        "terminalIntents",
        "officerRequests",
        "standingOrders",
        "reservations",
        "researchQuota",
        "shrinePlanningState",
    ] {
        assert!(
            fields.contains(required),
            "missing planner field {required}"
        );
    }
}

#[test]
fn planner_with_identical_beliefs_ignores_hidden_truth_until_observed() {
    let secrecy = contract().belief_secrecy;
    assert_eq!(secrecy.reported_food, 60.0);
    assert_eq!(secrecy.report_level, 3);
    assert!(!secrecy.regeneration_visible);

    let low_hidden_truth = direct_colony(&legacy_snapshot(secrecy.hidden_food_variants[0]));
    let high_hidden_truth = direct_colony(&legacy_snapshot(secrecy.hidden_food_variants[1]));

    assert_eq!(
        low_hidden_truth, high_hidden_truth,
        "same persisted report must produce the same plan; authoritative stock is private"
    );
}

#[test]
fn typed_spatial_contract_keeps_objective_work_and_delivery_distinct() {
    let contract = contract();
    let variants = contract
        .site_ref_variants
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        variants,
        BTreeSet::from([
            "anchored_rectangle",
            "building",
            "exact_tile",
            "ordered_route",
            "ordered_tile_set",
            "resource_source",
            "shrine",
            "stockpile",
            "village_trade_endpoint",
        ])
    );

    let cases = contract.spatial_cases;
    let categories = cases
        .iter()
        .map(|case| case.category.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(categories.len(), cases.len(), "categories must be unique");
    for case in cases {
        assert_ne!(case.objective_kind, case.work_kind, "{}", case.category);
        assert_ne!(case.objective_kind, case.delivery_kind, "{}", case.category);
        assert_ne!(case.work_kind, case.delivery_kind, "{}", case.category);
    }
}

#[test]
fn hunt_without_revealed_reachable_source_is_blocked_without_objective() {
    let context = JobDestinationContext {
        anchor: WorldPos { x: 6.0, y: 6.0 },
        shrine: WorldPos { x: 6.0, y: 6.0 },
        food_tiles: &[],
        roll: 0.25,
        site: None,
        expansion_site: None,
        quarry_site: None,
        water_site: None,
        explore_site: None,
        gather_spot_site: None,
    };

    assert_eq!(
        destination_for_job("hunt_expedition", &context),
        None,
        "Hunt must never fabricate the legacy seeded/radial objective"
    );
}

#[test]
fn workshop_objective_uses_full_canonical_nine_tile_footprint() {
    let expected = contract().workshop;
    let anchor = TilePos {
        x: expected.anchor.x,
        y: expected.anchor.y,
    };
    let (width, height) = footprint_for(BuildingType::Workshop);
    let tiles = footprint_tiles(anchor, width, height)
        .into_iter()
        .map(|tile| FixturePoint {
            x: tile.x,
            y: tile.y,
        })
        .collect::<Vec<_>>();

    assert_eq!((width, height), (expected.width, expected.height));
    assert_eq!(tiles, expected.tiles);
    assert_eq!(tiles.len(), 9);
}

#[test]
fn shrine_favor_cutover_has_no_legacy_tithe_decision() {
    let target = contract().shrine_favor;
    assert_eq!(
        target.legacy_global_upgrade_points + target.legacy_unspent_research_points,
        target.expected_favor
    );
    assert_eq!(target.migration_version, 1);
    assert!(
        target
            .forbidden_production_paths
            .iter()
            .any(|path| path == "tithe")
    );

    let plan = direct_colony(&legacy_snapshot(200.0));
    assert!(
        !decision_kinds(&plan).contains(&"tithe"),
        "the immediate scalar Tithe path must be absent after the Favor cutover"
    );
}

#[test]
fn thirty_day_campaign_harness_freezes_matrix_partitions_and_thresholds() {
    let campaign = contract().campaign;
    assert_eq!(campaign.days, 30);
    assert_eq!(campaign.seeds_per_population, 100);
    assert_eq!(campaign.tick_partitions_seconds, [1, 60, 900, 3_600]);
    assert_eq!(campaign.fresh_minimum_successes, 85);
    assert_eq!(campaign.established_minimum_successes, 97);
    assert_eq!(campaign.maximum_performance_regression_percent, 25);

    let seeds = (campaign.seed_start..campaign.seed_start + campaign.seeds_per_population)
        .collect::<BTreeSet<_>>();
    assert_eq!(seeds.len(), 100);
    assert_eq!(campaign.populations.len(), 13);
    assert!(campaign.populations.iter().any(|name| name == "fresh"));
    assert!(
        campaign
            .populations
            .iter()
            .any(|name| name == "migrated_legacy")
    );
    assert!(
        campaign
            .populations
            .iter()
            .any(|name| name == "multi_colony_contention")
    );
}

#[test]
fn release_baseline_is_fixed_to_the_deterministic_thirty_day_fixture() {
    let baseline: serde_json::Value =
        serde_json::from_str(RELEASE_BASELINE_JSON).expect("LAI.1 release baseline JSON");
    assert_eq!(baseline["fixture"]["gameDays"], 30);
    assert_eq!(baseline["fixture"]["seed"], 20_240_712);
    assert_eq!(baseline["fixture"]["cadenceMs"], 900_000);
    assert_eq!(baseline["fixture"]["cargoProfile"], "release");
    assert_eq!(baseline["fixture"]["liveProvidersCalled"], false);
    assert_eq!(baseline["samples"].as_array().map(Vec::len), Some(3));
    assert!(
        baseline["medianWallSeconds"]
            .as_f64()
            .is_some_and(|value| value > 0.0)
    );
    assert!(
        baseline["medianPeakRssKiB"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    assert_eq!(baseline["allowedRegressionPercent"], 25);
    assert_eq!(baseline["determinismVerification"]["result"], "identical");
}
