//! Observed-state campaigns for GAME_VISION's “more useful work than cats” tension.
//!
//! The guided paths issue only `ClientAction`s selected from the current authoritative
//! state. They never grant stock, research, buildings, cats, or assignments behind the
//! action layer. Long live-cadence evidence is produced by the matching playtest example;
//! these checked-in proxy twins keep the same 48/200 game-hour horizons affordable in CI.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    labor_pressure::{LaborPressureSample, observe_labor_pressure},
    types::{JobKind, JobStatus},
    world_tick::{WorldState, found_colony, found_global_colony, new_world, world_tick},
};

const START: i64 = 10_000;
const GAME_HOUR_MS: i64 = 3_600_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "labor-pressure-session".to_owned(),
        player_id: "labor-pressure-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn signed_job(kind: proto::JobKind) -> proto::ClientAction {
    proto::ClientAction::RequestJob {
        session_id: "labor-pressure-session".to_owned(),
        nickname: "Labor Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        kind,
    }
}

fn signed_scout() -> proto::ClientAction {
    proto::ClientAction::DispatchScout {
        session_id: "labor-pressure-session".to_owned(),
        nickname: "Labor Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        mission: proto::ScoutMission::Explore,
    }
}

fn jobs_in_flight(world: &WorldState, kind: JobKind) -> usize {
    world.colonies[0]
        .jobs
        .iter()
        .filter(|job| {
            job.kind == kind && matches!(job.status, JobStatus::Queued | JobStatus::Active)
        })
        .count()
}

fn apply_if_possible(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) -> bool {
    apply_action(world, &action, &ctx(now_ms)).ok
}

fn new_campaign_world(seed: u32, communal: bool) -> WorldState {
    let mut world = new_world(seed);
    world.colonies.push(if communal {
        found_global_colony(seed, "colony-1", START, seed)
    } else {
        found_colony(seed, "colony-1", START, seed)
    });
    world
}

fn guide_survival_and_frontier(world: &mut WorldState, now_ms: i64) -> usize {
    let mut accepted = 0;
    for (wire, sim, target) in [
        (proto::JobKind::HuntExpedition, JobKind::HuntExpedition, 6),
        (proto::JobKind::FetchWater, JobKind::FetchWater, 2),
        (proto::JobKind::ForageFibre, JobKind::ForageFibre, 1),
    ] {
        while jobs_in_flight(world, sim) < target {
            if !apply_if_possible(world, signed_job(wire), now_ms) {
                break;
            }
            accepted += 1;
        }
    }
    while jobs_in_flight(world, JobKind::Explore) < 5 {
        if !apply_if_possible(world, signed_scout(), now_ms) {
            break;
        }
        accepted += 1;
    }
    if jobs_in_flight(world, JobKind::ExpandVillage) == 0
        && apply_if_possible(world, signed_job(proto::JobKind::ExpandVillage), now_ms)
    {
        accepted += 1;
    }
    accepted
}

fn guide_longitudinal(world: &mut WorldState, now_ms: i64, game_hour: i64) -> usize {
    let mut accepted = 0;
    for (wire, sim, target) in [
        (proto::JobKind::HuntExpedition, JobKind::HuntExpedition, 6),
        (proto::JobKind::FetchWater, JobKind::FetchWater, 2),
        (proto::JobKind::ForageFibre, JobKind::ForageFibre, 1),
    ] {
        while jobs_in_flight(world, sim) < target {
            if !apply_if_possible(world, signed_job(wire), now_ms) {
                break;
            }
            accepted += 1;
        }
    }
    while jobs_in_flight(world, JobKind::Explore) < 1 {
        if !apply_if_possible(world, signed_scout(), now_ms) {
            break;
        }
        accepted += 1;
    }
    if game_hour % 24 == 0
        && jobs_in_flight(world, JobKind::ExpandVillage) == 0
        && apply_if_possible(world, signed_job(proto::JobKind::ExpandVillage), now_ms)
    {
        accepted += 1;
    }
    accepted
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CampaignOutcome {
    reset_count: usize,
    living: usize,
    final_food_positive: bool,
    final_water_positive: bool,
    min_assigned: usize,
    max_assigned: usize,
    min_active: usize,
    max_active: usize,
    max_idle: usize,
    max_useful_demand: usize,
    pressure_hours: usize,
    manual_actions: usize,
    labor_denials: usize,
    officer_states_seen: [bool; 8],
}

fn run_campaign(
    seed: u32,
    communal: bool,
    hours: i64,
    cadence_ms: i64,
    guided: bool,
) -> CampaignOutcome {
    let mut world = new_campaign_world(seed, communal);
    let steps = hours * GAME_HOUR_MS / cadence_ms;
    let sample_every = (GAME_HOUR_MS / cadence_ms).max(1);
    let mut resets = 0;
    let mut min_assigned = usize::MAX;
    let mut max_assigned = 0;
    let mut min_active = usize::MAX;
    let mut max_active = 0;
    let mut max_idle = 0;
    let mut max_useful_demand = 0;
    let mut pressure_hours = 0;
    let mut manual_actions = 0;
    let mut labor_denials = 0;
    let mut officer_states_seen = [false; 8];

    for step in 1..=steps {
        let now = START + step * cadence_ms;
        resets += usize::from(world_tick(&mut world, now)[0].reset_reason.is_some());
        if guided && step % sample_every == 0 {
            manual_actions += if step == sample_every {
                guide_survival_and_frontier(&mut world, now)
            } else {
                guide_longitudinal(&mut world, now, step * cadence_ms / GAME_HOUR_MS)
            };
            // Spend the remaining player-visible capacity on additional physical
            // food expeditions. These are not no-op jobs: the founding population
            // continuously consumes their finite deliveries, and every expedition
            // walks to a real hunting site and returns cargo through storage.
            loop {
                let sample = observe_labor_pressure(&world.colonies[0]);
                if sample.assigned_cats >= sample.living_cats {
                    break;
                }
                if !apply_if_possible(&mut world, signed_job(proto::JobKind::HuntExpedition), now) {
                    break;
                }
                manual_actions += 1;
            }
            // When the guide has filled the roster, probe one more ordinary physical
            // hunt through the signed boundary. A workforce-only denial is direct
            // evidence that useful demand remains after every paw is committed.
            let before = observe_labor_pressure(&world.colonies[0]);
            if before.assigned_cats == before.living_cats {
                let result = apply_action(
                    &mut world,
                    &signed_job(proto::JobKind::HuntExpedition),
                    &ctx(now),
                );
                if !result.ok
                    && result
                        .message
                        .as_deref()
                        .is_some_and(|message| message == "No available worker.")
                {
                    labor_denials += 1;
                }
            }
        }
        if step % sample_every == 0 {
            let sample = observe_labor_pressure(&world.colonies[0]);
            min_assigned = min_assigned.min(sample.assigned_cats);
            max_assigned = max_assigned.max(sample.assigned_cats);
            min_active = min_active.min(sample.active_cats);
            max_active = max_active.max(sample.active_cats);
            max_idle = max_idle.max(sample.idle_cats);
            max_useful_demand = max_useful_demand.max(sample.useful_demand());
            pressure_hours += usize::from(sample.demand_exceeds_labor());
            officer_states_seen[sample.manual_only_offices] = true;
        }
    }
    let final_sample = observe_labor_pressure(&world.colonies[0]);
    CampaignOutcome {
        reset_count: resets,
        living: final_sample.living_cats,
        final_food_positive: world.colonies[0].resources.food > 0.0,
        final_water_positive: world.colonies[0].resources.water > 0.0,
        min_assigned: if min_assigned == usize::MAX {
            0
        } else {
            min_assigned
        },
        max_assigned,
        min_active: if min_active == usize::MAX {
            0
        } else {
            min_active
        },
        max_active,
        max_idle,
        max_useful_demand,
        pressure_hours,
        manual_actions,
        labor_denials,
        officer_states_seen,
    }
}

#[test]
fn fresh_player_can_exhaust_labor_with_only_meaningful_physical_orders() {
    for seed in [7_u32, 555, 2024] {
        let mut world = new_campaign_world(seed, false);
        let accepted = guide_survival_and_frontier(&mut world, START + 1);
        let sample = observe_labor_pressure(&world.colonies[0]);
        assert_eq!(
            sample.assigned_cats, sample.living_cats,
            "seed {seed}: {sample:?}"
        );
        assert!(
            accepted >= 14,
            "seed {seed}: accepted only {accepted} useful orders"
        );
        assert_eq!(
            sample.useful_demand(),
            sample.living_cats + 1,
            "seed {seed}: the founding processor should remain a sourced vacancy"
        );
        assert!(sample.demand_exceeds_labor(), "seed {seed}: {sample:?}");
        let denied = apply_action(
            &mut world,
            &signed_job(proto::JobKind::HuntExpedition),
            &ctx(START + 2),
        );
        assert!(!denied.ok);
        assert_eq!(denied.message.as_deref(), Some("No available worker."));
    }
}

#[test]
fn passive_48_hour_proxy_twins_survive_without_hidden_specialist_automation() {
    for (seed, communal) in [(7_u32, false), (555, false), (2024, false), (7, true)] {
        let first = run_campaign(seed, communal, 48, 5 * 60_000, false);
        let second = run_campaign(seed, communal, 48, 5 * 60_000, false);
        assert_eq!(first, second, "seed {seed}, communal={communal}");
        assert_eq!(
            first.reset_count, 0,
            "seed {seed}, communal={communal}: {first:?}"
        );
        assert!(first.final_food_positive && first.final_water_positive);
        assert!(first.officer_states_seen[7]);
        assert_eq!(first.manual_actions, 0);
    }
}

#[test]
fn guided_colonies_keep_visible_labor_pressure_through_200_game_hours() {
    for seed in [7_u32, 555, 2024] {
        let first = run_campaign(seed, false, 200, 5 * 60_000, true);
        let second = run_campaign(seed, false, 200, 5 * 60_000, true);
        assert_eq!(first, second, "seed {seed}");
        assert_eq!(first.reset_count, 0, "seed {seed}: {first:?}");
        assert!(first.final_food_positive && first.final_water_positive);
        assert!(first.manual_actions >= 15, "seed {seed}: {first:?}");
        assert!(first.max_assigned >= 14, "seed {seed}: {first:?}");
        assert!(first.max_active > 0, "seed {seed}: {first:?}");
        assert!(first.max_useful_demand > 0, "seed {seed}: {first:?}");
        assert!(
            first.labor_denials > 0,
            "seed {seed}: no observed labor shortage"
        );
        assert!(first.officer_states_seen[7]);
    }
}

// Run explicitly when collecting release evidence. Three personal seeds plus the
// communal founding take several wall-clock minutes at the exact server cadence.
#[test]
#[ignore = "release playtest: exact 48-hour one-second cadence"]
fn passive_48_hour_live_cadence_twins() {
    for (seed, communal) in [(7_u32, false), (555, false), (2024, false), (7, true)] {
        let first = run_campaign(seed, communal, 48, 1_000, false);
        let second = run_campaign(seed, communal, 48, 1_000, false);
        assert_eq!(first, second, "seed {seed}, communal={communal}");
        assert_eq!(
            first.reset_count, 0,
            "seed {seed}, communal={communal}: {first:?}"
        );
        assert!(first.final_food_positive && first.final_water_positive);
    }
}

#[test]
fn telemetry_shape_is_stable_for_docs() {
    let sample = LaborPressureSample {
        living_cats: 15,
        assigned_cats: 15,
        useful_assigned_cats: 15,
        active_cats: 14,
        idle_cats: 0,
        active_job_slots: 10,
        staffed_station_slots: 3,
        staffed_farm_slots: 1,
        staffed_transport_slots: 1,
        sourced_station_vacancies: 1,
        workable_farm_vacancies: 0,
        manual_only_offices: 4,
    };
    assert_eq!(sample.useful_demand(), 16);
    assert!(sample.demand_exceeds_labor());
}
