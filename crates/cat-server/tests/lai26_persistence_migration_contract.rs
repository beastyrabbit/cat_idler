//! LAI.26A red SQLite persistence/migration contract.

const PERSISTENCE: &str = include_str!("../src/persistence.rs");
const LEADER_AI_PERSISTENCE: &str = include_str!("../src/leader_ai_persistence.rs");
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");

fn missing_required<'a>(source: &str, required: &'a [(&str, &str)]) -> Vec<&'a str> {
    required
        .iter()
        .filter_map(|(needle, reason)| (!source.contains(needle)).then_some(*reason))
        .collect()
}

fn forbidden_present<'a>(source: &str, forbidden: &'a [(&str, &str)]) -> Vec<&'a str> {
    forbidden
        .iter()
        .filter_map(|(needle, reason)| source.contains(needle).then_some(*reason))
        .collect()
}

#[test]
fn lai26_schema_version_and_transactional_world_migration_are_installed() {
    assert!(WIRE_DOC.contains("## LAI.26 SQLite migration and restart contract"));
    let persistence_sources = format!("{PERSISTENCE}\n{LEADER_AI_PERSISTENCE}");
    let required = [
        (
            "LAI26_SCHEMA_VERSION",
            "missing strict LAI.26 schema version",
        ),
        (
            "leader_ai_migration_marker",
            "missing idempotent migration marker table/column",
        ),
        ("sourceSchemaVersion", "missing marker source schema"),
        ("targetSchemaVersion", "missing marker target schema"),
        ("saveIdentity", "missing marker world/save identity"),
        (
            "conversionEventCount",
            "missing marker conversion event total",
        ),
        (
            "conversionMicroFavorTotal",
            "missing marker conversion micro-Favor total",
        ),
        (
            "begin_lai26_world_migration_transaction",
            "missing per-world transaction boundary",
        ),
        (
            "commit_lai26_world_migration_transaction",
            "missing explicit migration commit",
        ),
        (
            "rollback_lai26_world_migration_transaction",
            "missing explicit migration rollback",
        ),
        (
            "quarantine_lai26_malformed_save",
            "missing malformed-save quarantine path",
        ),
        (
            "reject_lai26_partial_migration",
            "missing partial-save rejection path",
        ),
    ];

    let missing = missing_required(&persistence_sources, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must install one transactional per-world/save migration with an idempotent marker; missing: {missing:?}"
    );
}

#[test]
fn legacy_research_and_blessing_balances_convert_to_exact_favor_once() {
    let required = [
        (
            "convert_legacy_upgrade_points_and_research_points_to_favor",
            "missing exact one-time Favor conversion",
        ),
        (
            "legacy_global_upgrade_points",
            "missing legacy global_upgrade_points read",
        ),
        (
            "legacy_unspent_research_points",
            "missing legacy unspent research_points read",
        ),
        ("micro_favor", "Favor conversion must use exact micro-Favor"),
        (
            "preserve_owned_study_ids",
            "missing owned-study preservation",
        ),
        (
            "reject_duplicate_favor_conversion_marker",
            "missing replay/double-mint guard",
        ),
        (
            "assert_no_legacy_research_currency_after_lai26",
            "missing old currency deletion check",
        ),
    ];
    let forbidden = [
        (
            "resources.blessings",
            "legacy blessing mirror must not remain a migrated Favor source",
        ),
        (
            "research_points +=",
            "research-point accrual must not survive migration",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    let present = forbidden_present(PERSISTENCE, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "LAI.26 must convert legacy upgrade/research balances to exact Favor once while preserving studies; missing: {missing:?}; forbidden present: {present:?}"
    );
}

#[test]
fn legacy_cats_traits_skills_personality_anatomy_and_prosthetics_migrate() {
    let required = [
        (
            "migrate_lai26_cat_attributes",
            "missing 0-100 to 1-20 attribute migration",
        ),
        ("migrate_lai26_cat_skills", "missing legacy skill migration"),
        (
            "backfill_lai26_personality_from_stable_id",
            "missing deterministic personality backfill",
        ),
        (
            "initialize_lai26_anatomy",
            "missing anatomy initialization/migration",
        ),
        (
            "migrate_lai26_injuries_and_treatment",
            "missing injury/treatment migration",
        ),
        (
            "migrate_lai26_prosthetic_items",
            "missing prosthetic item migration",
        ),
        (
            "validate_lai26_one_prosthetic_item_identity",
            "missing one-ID conservation validation",
        ),
        (
            "reject_lai26_impossible_cat_state",
            "missing impossible cat-state rejection",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must migrate legacy cat traits/skills/personality/anatomy/prosthetics with strict validation; missing: {missing:?}"
    );
}

#[test]
fn task_site_cargo_and_reservation_states_migrate_and_restart_equal() {
    let required = [
        (
            "migrate_lai26_legacy_job_metadata_to_site_refs",
            "missing legacy job metadata to SiteRef conversion",
        ),
        (
            "migrate_lai26_task_stage_state",
            "missing task stage persistence",
        ),
        (
            "migrate_lai26_task_cargo_state",
            "missing task cargo persistence",
        ),
        (
            "migrate_lai26_world_reservation_ledger",
            "missing world reservation ledger migration",
        ),
        (
            "revalidate_lai26_routes_on_restart",
            "missing restart route revalidation",
        ),
        (
            "revalidate_lai26_endpoints_on_restart",
            "missing restart endpoint revalidation",
        ),
        (
            "assert_lai26_restart_equality_for_every_task_stage",
            "missing every-stage restart equality guard",
        ),
        (
            "block_lai26_invalid_legacy_site_metadata",
            "missing blocked legacy task conversion",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must migrate task/site/cargo/reservation states and prove restart equality at every stage; missing: {missing:?}"
    );
}

#[test]
fn all_leader_ai_leaf_state_is_persisted_with_versions_and_fresh_defaults() {
    let required = [
        (
            "persist_lai26_planner_clock",
            "missing planner clock persistence",
        ),
        (
            "persist_lai26_planner_versions",
            "missing planner/domain version persistence",
        ),
        (
            "persist_lai26_beliefs_evidence_reports",
            "missing belief/evidence/report persistence",
        ),
        (
            "persist_lai26_officer_institution_and_requests",
            "missing officer/request persistence",
        ),
        (
            "persist_lai26_shrine_pipelines",
            "missing Shrine pipeline persistence",
        ),
        (
            "persist_lai26_favor_events",
            "missing Favor ledger/event persistence",
        ),
        (
            "persist_lai26_research_quota_insight_preparation",
            "missing research quota/Insight/preparation persistence",
        ),
        ("persist_lai26_divine_boosts", "missing boost persistence"),
        (
            "persist_lai26_diplomacy_trade_escrow_transit",
            "missing diplomacy/trade escrow/transit persistence",
        ),
        (
            "lai26_fresh_colony_defaults",
            "missing fresh default initializer",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must persist every leader-AI leaf state with versions and fresh defaults; missing: {missing:?}"
    );
}

#[test]
fn malformed_unknown_duplicate_dangling_negative_and_hidden_rows_fail_atomically() {
    assert!(TESTING_DOC.contains("Transactional migration rolls back"));
    let required = [
        (
            "reject_lai26_unknown_schema_version",
            "missing unknown-version rejection",
        ),
        (
            "reject_lai26_duplicate_stable_ids",
            "missing duplicate-ID rejection",
        ),
        (
            "reject_lai26_dangling_references",
            "missing dangling-reference rejection",
        ),
        (
            "reject_lai26_negative_favor",
            "missing negative-Favor rejection",
        ),
        (
            "reject_lai26_hidden_projection_fields",
            "missing hidden projection field rejection",
        ),
        (
            "reject_lai26_impossible_task_stage",
            "missing impossible-stage rejection",
        ),
        (
            "assert_lai26_no_partial_save_after_failure",
            "missing no-partial-save assertion",
        ),
        (
            "quarantine_lai26_bad_row_with_reason",
            "missing bounded bad-row quarantine reason",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 malformed rows must roll back atomically and quarantine/reject without partial save; missing: {missing:?}"
    );
}

#[test]
fn bounded_idempotency_results_survive_restart_without_double_mutation() {
    let required = [
        (
            "persist_lai26_idempotency_results",
            "missing bounded idempotency-result persistence",
        ),
        (
            "replay_lai26_idempotency_result_after_restart",
            "missing restart replay of idempotent results",
        ),
        (
            "reject_lai26_unbounded_idempotency_payload",
            "missing idempotency payload bounds",
        ),
        (
            "assert_lai26_no_duplicate_favor_event_after_replay",
            "missing duplicate Favor guard",
        ),
        (
            "assert_lai26_no_duplicate_reservation_after_replay",
            "missing duplicate reservation guard",
        ),
        (
            "assert_lai26_no_duplicate_trade_mutation_after_replay",
            "missing duplicate trade guard",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must persist bounded idempotency results and replay them after restart without double mutation; missing: {missing:?}"
    );
}

#[test]
fn cross_colony_ids_and_reservations_remain_isolated() {
    let required = [
        (
            "validate_lai26_colony_scoped_ids",
            "missing colony-scoped ID validation",
        ),
        (
            "validate_lai26_world_scoped_reservations",
            "missing world-scoped reservation validation",
        ),
        (
            "reject_lai26_cross_colony_private_reference",
            "missing cross-colony private reference rejection",
        ),
        (
            "reject_lai26_cross_colony_reservation_conflict_leak",
            "missing reservation loser leak guard",
        ),
        (
            "assert_lai26_selected_colony_restart_projection",
            "missing selected-colony restart projection guard",
        ),
        (
            "assert_lai26_public_trade_relationships_only",
            "missing public relationship/trade isolation guard",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must keep cross-colony IDs/reservations isolated while preserving public relationship/trade facts; missing: {missing:?}"
    );
}

#[test]
fn restart_equality_covers_every_runtime_stage_and_transition_fingerprint() {
    let required = [
        (
            "TransitionFingerprint",
            "missing persisted transition fingerprint type",
        ),
        (
            "assert_lai26_restart_equality_at_shrine_stage",
            "missing Shrine-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_at_research_stage",
            "missing research-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_at_boost_stage",
            "missing boost-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_at_treatment_stage",
            "missing treatment-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_at_prosthetic_stage",
            "missing prosthetic-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_at_trade_stage",
            "missing trade-stage restart equality",
        ),
        (
            "assert_lai26_restart_equality_after_quarantine",
            "missing quarantine restart equality",
        ),
    ];

    let missing = missing_required(PERSISTENCE, &required);
    assert!(
        missing.is_empty(),
        "LAI.26 must prove restart equality across every persisted runtime stage and transition fingerprint; missing: {missing:?}"
    );
}
