//! LAI.23A red harness for the single-path `world_tick` cutover.
//!
//! These tests intentionally inspect the orchestration root. They must stay red
//! until LAI.23 replaces the legacy runtime paths with the documented leaf-module
//! phase order; no production shim should be added only to satisfy these strings.

use cat_sim::{
    types::BuildingType,
    world_tick::{TilePos, footprint_for, footprint_tiles},
};

const WORLD_TICK: &str = include_str!("../src/world_tick.rs");
const PROJECTION: &str = include_str!("../src/player_projection.rs");
const README: &str = include_str!("../../../docs/leader-ai-overhaul/README.md");
const SPATIAL_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/spatial-task-contract.md");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");

fn missing_required<'a>(source: &str, required: &[&'a str]) -> Vec<&'a str> {
    required
        .iter()
        .copied()
        .filter(|needle| !source.contains(needle))
        .collect()
}

fn forbidden_present<'a>(source: &str, forbidden: &[(&'a str, &'a str)]) -> Vec<&'a str> {
    forbidden
        .iter()
        .filter_map(|(needle, reason)| source.contains(needle).then_some(*reason))
        .collect()
}

fn assert_ordered(source: &str, ordered: &[&str]) -> Result<(), String> {
    let mut cursor = 0;
    for needle in ordered {
        let Some(offset) = source[cursor..].find(needle) else {
            return Err(format!("missing ordered phase marker `{needle}`"));
        };
        cursor += offset + needle.len();
    }
    Ok(())
}

fn function_source<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing function signature `{signature}`"));
    let open = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing function body for `{signature}`"));
    let mut depth = 0_u32;
    for (offset, byte) in source.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated function body for `{signature}`");
}

fn cutover_root() -> &'static str {
    function_source(WORLD_TICK, "fn world_tick_inner")
}

fn cutover_implementation() -> &'static str {
    let start = WORLD_TICK
        .find("fn phase_lai23_01_authoritative")
        .expect("LAI.23 implementation starts at phase 01");
    let end = WORLD_TICK[start..]
        .find("/// Remove pre-fix ownership")
        .map(|offset| start + offset)
        .expect("LAI.23 implementation has a bounded production section");
    &WORLD_TICK[start..end]
}

#[test]
fn lai23_single_ordered_phase_path_is_installed() {
    assert!(README.contains("The single authoritative simulation order is:"));
    let expected = [
        "phase_lai23_01_authoritative_ecology_needs_hazards_emergencies",
        "phase_lai23_02_beliefs_reports_expiry_contradictions",
        "phase_lai23_03_leader_officer_review_boundaries",
        "phase_lai23_04_scheduler_workforce_spatial_reservations",
        "phase_lai23_05_visible_task_runtime_movement_cargo",
        "phase_lai23_06_shrine_favor_offerings",
        "phase_lai23_07_research_scholars_boosts",
        "phase_lai23_08_diplomacy_trade_contracts",
        "phase_lai23_09_stress_injury_prosthetic_lifecycle",
        "phase_lai23_10_report_safe_snapshots_events",
    ];

    assert!(
        missing_required(cutover_root(), &expected).is_empty(),
        "LAI.23 must install the exact single ordered phase path; missing: {:?}",
        missing_required(cutover_root(), &expected)
    );
    assert!(
        assert_ordered(cutover_root(), &expected).is_ok(),
        "{}",
        assert_ordered(cutover_root(), &expected).unwrap_err()
    );
}

#[test]
fn legacy_planner_director_reliability_tithe_and_research_schedules_are_removed() {
    assert!(WIRE_DOC.contains("removes the legacy `leader_director` runtime"));
    let forbidden = [
        (
            "leader_ai::{LeaderDecision",
            "legacy LeaderDecision planner surface is still imported",
        ),
        (
            "leader_director::{",
            "legacy leader_director runtime is still imported",
        ),
        (
            "automated_plan(",
            "legacy automated_plan still mutates runtime planning",
        ),
        (
            "direct_colony(",
            "legacy direct_colony path still reachable from world tick",
        ),
        (
            "policy.config.action_reliability",
            "per-action reliability miss path remains",
        ),
        (
            "next_base_roll(colony)",
            "legacy base-roll reliability RNG remains",
        ),
        (
            "phase_21_leader_capital_decisions_and_tithe",
            "legacy capital/tithe phase remains",
        ),
        (
            "LeaderDecision::Tithe",
            "immediate scalar tithe decision remains",
        ),
        (
            "automated_tithe_ready",
            "daily tithe cooldown helper remains",
        ),
        (
            "last_tithe_at",
            "legacy tithe cooldown timestamp remains in runtime",
        ),
        (
            "last_offering_at",
            "legacy offering cooldown timestamp remains in runtime",
        ),
        (
            "ritual_requested_at",
            "legacy immediate ritual request state remains in runtime",
        ),
        (
            "global_upgrade_points",
            "legacy spendable blessing/upgrade currency remains in world tick",
        ),
        (
            "phase_24_research",
            "legacy upgrade-tree research schedule remains",
        ),
        ("accrue_research(", "legacy research-point accrual remains"),
        (
            "cat_auto_unlock(",
            "legacy automatic research unlock remains",
        ),
        (
            "last_leader_research_choice_at",
            "legacy leader research cooldown timestamp remains",
        ),
    ];

    let present = forbidden_present(cutover_root(), &forbidden);
    assert!(
        present.is_empty(),
        "LAI.23 must remove every legacy planner/currency/schedule path: {present:?}"
    );
}

#[test]
fn world_tick_calls_completed_leaf_modules_instead_of_shadow_paths() {
    let required_leaf_symbols = [
        "beliefs::",
        "leader_planner::",
        "officer_expertise::",
        "officer_requests::",
        "scheduler::",
        "workforce_matcher::",
        "spatial_resolver::",
        "world_reservations::",
        "task_runtime::",
        "shrine_offerings::",
        "favor::",
        "research_manifest::",
        "research_purchase::",
        "divine_boosts::",
        "diplomacy::",
        "autonomous_trade::",
        "cat_stress::",
        "cat_willingness::",
        "injuries::",
        "prosthetics::",
    ];

    let missing = missing_required(cutover_implementation(), &required_leaf_symbols);
    assert!(
        missing.is_empty(),
        "world_tick must orchestrate completed leaves directly and not run a shadow planner; missing leaf calls/imports: {missing:?}"
    );
}

#[test]
fn spatial_execution_has_no_hunt_water_or_work_movement_fallbacks() {
    assert!(SPATIAL_DOC.contains("radial/seeded objective fallback"));
    assert!(SPATIAL_DOC.contains("Generic straight-line movement after pathfinding failure"));
    let forbidden = [
        (
            "destination_for_job",
            "job-kind destination fallback remains instead of typed spatial objectives",
        ),
        (
            "JobDestinationContext",
            "movement fallback context remains in world_tick orchestration",
        ),
        (
            "phase_17_legacy_emergency_hunt",
            "legacy non-spatial emergency Hunt phase remains",
        ),
        (
            "phase_17b_water_reserve_preemption",
            "legacy non-spatial Water preemption phase remains",
        ),
        ("LeaderPlanHunt", "legacy LeaderPlanHunt bridge job remains"),
        (
            "straight-line fallback",
            "straight-line fallback remains documented in code",
        ),
        (
            "radial",
            "radial/seeded fallback remains documented in code",
        ),
    ];

    let present = forbidden_present(cutover_root(), &forbidden);
    assert!(
        present.is_empty(),
        "LAI.23 must route Hunt/Water/work through spatial_resolver + task_runtime only: {present:?}"
    );
}

#[test]
fn duplicate_mutation_sites_are_removed_from_world_tick_root() {
    let forbidden = [
        (
            "OfferingCarry",
            "legacy OfferingCarry job metadata can mutate Shrine/Favor outside shrine_offerings",
        ),
        (
            "OfferingRitual",
            "legacy OfferingRitual job metadata can mutate Shrine/Favor outside shrine_offerings",
        ),
        (
            "advance_material_offering_logistics",
            "legacy material offering logistics duplicate the Shrine pipeline",
        ),
        (
            "resources.blessings",
            "legacy blessing projection duplicates exact FavorLedger",
        ),
        (
            "upgrade_tree.research_points",
            "legacy research points duplicate Favor research purchase",
        ),
        (
            "owned_node_ids",
            "legacy upgrade-tree owned-node mutation duplicates research progress",
        ),
    ];

    let present = forbidden_present(cutover_root(), &forbidden);
    assert!(
        present.is_empty(),
        "LAI.23 must leave mutation authority in the leaf ledgers only: {present:?}"
    );
}

#[test]
fn hidden_regeneration_fields_do_not_escape_report_projection() {
    assert!(TESTING_DOC.contains("No regeneration appears"));
    let forbidden_projection_fields = [
        (
            "last_replenished_at_ms",
            "protocol exposes exact replenishment timestamp rather than report-safe belief range",
        ),
        (
            "replenish",
            "protocol exposes replenishment wording outside report-safe projections",
        ),
    ];

    let present = forbidden_present(PROJECTION, &forbidden_projection_fields);
    assert!(
        present.is_empty(),
        "LAI.23 projection must prevent hidden regeneration leakage: {present:?}"
    );
}

#[test]
fn workshop_footprint_contract_remains_canonical_three_by_three() {
    let anchor = TilePos { x: 6, y: 6 };
    let (width, height) = footprint_for(BuildingType::Workshop);
    let tiles = footprint_tiles(anchor, width, height);

    assert_eq!((width, height), (3, 3));
    assert_eq!(tiles.len(), 9);
    assert_eq!(tiles.first(), Some(&TilePos { x: 6, y: 6 }));
    assert_eq!(tiles.last(), Some(&TilePos { x: 8, y: 8 }));
}

#[test]
fn restart_and_partition_divergence_guards_are_present_in_cutover_root() {
    let expected = [
        "lai23_revalidate_active_tasks_after_restart",
        "lai23_assert_no_duplicate_leaf_mutations_after_restart",
        "lai23_tick_partition_equivalence",
    ];

    let missing = missing_required(cutover_implementation(), &expected);
    assert!(
        missing.is_empty(),
        "LAI.23 needs explicit restart/partition guard calls before production cutover: {missing:?}"
    );
}
