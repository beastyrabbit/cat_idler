//! LAI.32A deterministic 30-day campaign manifest and red harness.
//!
//! This target validates the campaign matrix fixture and names the future
//! production campaign evidence hooks. It deliberately does not call live
//! providers.

use std::collections::BTreeSet;
use std::{env, fs};

use cat_sim::campaign_runner::{
    Lai32CampaignMatrixEvidence, Lai32SeedSetEvidence, lai32_campaign_seed_set_by_id,
    lai32_probe_fresh_seed_320000_boundaries_90_to_120,
    lai32_probe_fresh_seed_320000_boundaries_205_to_213,
    lai32_probe_fresh_seed_320000_boundaries_214_to_230,
    lai32_probe_fresh_seed_320000_boundaries_2175_to_2185,
    lai32_probe_fresh_seed_320000_demography_gate,
    lai32_probe_fresh_seed_320000_first_survival_death,
    lai32_probe_fresh_seed_320000_generational_gate, lai32_probe_fresh_seed_320000_ticks_1_to_120,
    lai32_trace_fresh_colony_seed_320000,
};
use cat_sim::{
    Lai32CampaignCategory, Lai32CampaignRunner, Lai32CampaignScenario,
    assert_at_least_four_affordable_auto_research_commits, assert_exact_void_insight_conservation,
    assert_hidden_regeneration_secrecy_below_l4, assert_lai32_bounded_state_and_queues,
    assert_no_duplicate_void_research_trade_cargo_mutations,
    assert_no_starvation_caused_solely_by_endless_hole_demand, compare_lai32_against_lai1_baseline,
    compare_lai32_partitioned_snapshots_byte_equal, compare_lai32_restart_snapshots_byte_equal,
    measure_lai32_release_profile, record_lai32_wall_time_and_peak_rss,
    run_lai32_release_profile_campaign_matrix, run_lai32_restart_partition_campaign_matrix,
};
use serde::Deserialize;

const MANIFEST_JSON: &str =
    include_str!("../../../docs/leader-ai-overhaul/fixtures/lai32_campaign_manifest.json");
const BASELINE_JSON: &str =
    include_str!("../../../docs/leader-ai-overhaul/fixtures/lai1_release_baseline.json");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");
const LIB_RS: &str = include_str!("../src/lib.rs");
const WORLD_TICK: &str = include_str!("../src/world_tick.rs");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CampaignManifest {
    schema_version: u32,
    card: String,
    live_providers_called: bool,
    duration: DurationContract,
    success_thresholds: SuccessThresholds,
    sets: Vec<CampaignSet>,
    required_invariants: Vec<String>,
    release_profile: ReleaseProfile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DurationContract {
    game_days: u32,
    game_hours: u32,
    default_cadence_ms: u64,
    tick_partitions_seconds: Vec<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuccessThresholds {
    fresh_minimum_successes: u32,
    established_minimum_successes: u32,
    seeds_per_set: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CampaignSet {
    id: String,
    category: String,
    seed_start: u32,
    seed_count: u32,
    description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseProfile {
    baseline_fixture: String,
    profile: String,
    build_command: String,
    measurement_command: String,
    baseline_median_wall_seconds: f64,
    baseline_median_peak_rss_ki_b: u64,
    allowed_regression_percent: u32,
    maximum_median_wall_seconds: f64,
    maximum_median_peak_rss_ki_b: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Lai1ReleaseBaseline {
    median_wall_seconds: f64,
    median_peak_rss_ki_b: u64,
    allowed_regression_percent: u32,
}

fn manifest() -> CampaignManifest {
    serde_json::from_str(MANIFEST_JSON).expect("LAI.32 campaign manifest must be valid JSON")
}

fn baseline() -> Lai1ReleaseBaseline {
    serde_json::from_str(BASELINE_JSON).expect("LAI.1 release baseline must be valid JSON")
}

fn missing_required<'a>(source: &str, required: &[&'a str]) -> Vec<&'a str> {
    required
        .iter()
        .copied()
        .filter(|needle| !source.contains(needle))
        .collect()
}

fn assert_future_runner_symbols(test_name: &str, required: &[&str]) {
    let joined = format!("{LIB_RS}\n{WORLD_TICK}");
    let missing = missing_required(&joined, required);
    assert!(
        missing.is_empty(),
        "{test_name} is still red: missing LAI.32 campaign runner/evidence symbols {missing:?}"
    );
}

#[test]
fn campaign_manifest_defines_every_required_100_seed_set() {
    assert!(TESTING_DOC.contains("Deterministic 30-day campaigns"));
    let manifest = manifest();
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.card, "LAI.32");
    assert!(!manifest.live_providers_called);
    assert_eq!(manifest.duration.game_days, 30);
    assert_eq!(manifest.duration.game_hours, 720);
    assert_eq!(manifest.duration.default_cadence_ms, 900_000);
    assert_eq!(
        manifest.duration.tick_partitions_seconds,
        [1, 60, 900, 3600]
    );

    let expected_ids = BTreeSet::from([
        "fresh_colony",
        "established_colony",
        "mature_research_trade_colony",
        "extreme_scarcity",
        "extreme_devout",
        "extreme_skeptical",
        "extreme_mercantile",
        "extreme_self_sufficient",
        "extreme_bold",
        "extreme_cautious",
        "injury_prosthetic_stress",
        "multi_colony",
        "reservation_contention",
        "hole_omission_bad_resource_choices",
        "research_quota",
        "diplomacy_trade",
        "restart_partition",
    ]);
    let actual_ids = manifest
        .sets
        .iter()
        .map(|set| set.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_ids, expected_ids);
    assert!(manifest.sets.iter().all(|set| {
        set.seed_count == 100
            && set.seed_start >= 320_000
            && !set.category.is_empty()
            && !set.description.is_empty()
    }));

    let mut all_seeds = BTreeSet::new();
    for set in &manifest.sets {
        for offset in 0..set.seed_count {
            assert!(all_seeds.insert(set.seed_start + offset));
        }
    }
    assert_eq!(all_seeds.len(), manifest.sets.len() * 100);
}

#[test]
fn manifest_thresholds_invariants_and_release_budget_match_lai1_baseline() {
    let manifest = manifest();
    let baseline = baseline();
    assert_eq!(manifest.success_thresholds.fresh_minimum_successes, 85);
    assert_eq!(
        manifest.success_thresholds.established_minimum_successes,
        97
    );
    assert_eq!(manifest.success_thresholds.seeds_per_set, 100);

    let invariants = manifest
        .required_invariants
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "bounded_state_and_queues",
        "no_starvation_caused_solely_by_endless_hole_demand",
        "believable_good_and_bad_leader_variation",
        "at_least_four_normal_affordable_automatic_research_commits_in_30_days",
        "exact_void_insight_conservation",
        "hunt_water_workshop_spatial_invariants",
        "hidden_regeneration_secrecy_below_effective_report_level_4",
        "no_duplicate_mutations_or_replay",
        "tick_partition_twins",
        "restart_twins",
    ] {
        assert!(
            invariants.contains(required),
            "missing invariant {required}"
        );
    }

    assert_eq!(
        manifest.release_profile.baseline_fixture,
        "docs/leader-ai-overhaul/fixtures/lai1_release_baseline.json"
    );
    assert_eq!(manifest.release_profile.profile, "release");
    assert!(manifest.release_profile.build_command.contains("--release"));
    assert!(
        manifest
            .release_profile
            .measurement_command
            .contains("VERIFY_DETERMINISM=1")
    );
    assert_eq!(
        manifest.release_profile.baseline_median_wall_seconds,
        baseline.median_wall_seconds
    );
    assert_eq!(
        manifest.release_profile.baseline_median_peak_rss_ki_b,
        baseline.median_peak_rss_ki_b
    );
    assert_eq!(
        manifest.release_profile.allowed_regression_percent,
        baseline.allowed_regression_percent
    );
    assert_eq!(manifest.release_profile.allowed_regression_percent, 25);
    assert!(manifest.release_profile.maximum_median_wall_seconds <= 23.4625);
    assert!(manifest.release_profile.maximum_median_peak_rss_ki_b <= 14_950);
}

#[test]
fn small_smoke_campaign_entrypoint_is_red_until_runner_exists() {
    assert_future_runner_symbols(
        "small_smoke_campaign_entrypoint_is_red_until_runner_exists",
        &[
            "Lai32CampaignRunner",
            "Lai32CampaignScenario",
            "Lai32CampaignOutcome",
            "lai32_run_30_day_campaign",
            "lai32_run_small_red_smoke_campaigns",
        ],
    );
}

#[test]
fn campaign_success_thresholds_are_asserted_by_runner_not_docs_only() {
    assert_future_runner_symbols(
        "campaign_success_thresholds_are_asserted_by_runner_not_docs_only",
        &[
            "assert_lai32_fresh_success_threshold_85_of_100",
            "assert_lai32_established_success_threshold_97_of_100",
            "assert_lai32_bounded_state_and_queues",
            "assert_no_starvation_caused_solely_by_endless_hole_demand",
            "assert_believable_good_and_bad_leader_variation",
        ],
    );
}

#[test]
fn campaign_progression_spatial_privacy_and_replay_invariants_are_runner_outputs() {
    assert_future_runner_symbols(
        "campaign_progression_spatial_privacy_and_replay_invariants_are_runner_outputs",
        &[
            "assert_at_least_four_affordable_auto_research_commits",
            "assert_exact_void_insight_conservation",
            "assert_hunt_water_workshop_spatial_invariants",
            "assert_hidden_regeneration_secrecy_below_l4",
            "assert_no_duplicate_void_research_trade_cargo_mutations",
        ],
    );
}

#[test]
fn restart_partition_and_release_profile_evidence_hooks_are_present() {
    assert_future_runner_symbols(
        "restart_partition_and_release_profile_evidence_hooks_are_present",
        &[
            "assert_lai32_tick_partition_twins",
            "assert_lai32_restart_twins",
            "Lai32CampaignPerformanceSample",
            "measure_lai32_release_profile",
            "LAI32_RELEASE_PROFILE_MAX_REGRESSION_PERCENT",
        ],
    );
}

#[test]
fn small_smoke_runner_executes_world_tick_and_records_structured_evidence() {
    let outcomes = cat_sim::lai32_run_small_red_smoke_campaigns();
    assert_eq!(outcomes.len(), 2);
    for outcome in &outcomes {
        assert_eq!(outcome.ticks_executed, 8);
        assert!(outcome.final_tick_ms > 1_000);
        assert!(!outcome.deterministic_fingerprint.is_empty());
        assert_lai32_bounded_state_and_queues(outcome).expect("smoke state remains bounded");
        assert_exact_void_insight_conservation(outcome)
            .expect("Hole credit and Void Insight conservation validates");
        assert_hidden_regeneration_secrecy_below_l4(outcome)
            .expect("low report levels do not expose regeneration");
        assert_no_duplicate_void_research_trade_cargo_mutations(outcome)
            .expect("observed mutation IDs are unique");
        let automatic = outcome
            .invariants
            .iter()
            .find(|evidence| {
                evidence.invariant
                    == cat_sim::campaign_runner::Lai32Invariant::AutomaticResearchCommits
            })
            .expect("automatic research evidence exists");
        assert_eq!(
            automatic.passed,
            outcome.state.automatic_research_commit_count >= 4
        );
    }
}

#[test]
fn runner_is_deterministic_for_identical_seed_and_configuration() {
    let runner = Lai32CampaignRunner::bounded_smoke(4);
    let scenario = Lai32CampaignScenario::new(
        "determinism_smoke",
        Lai32CampaignCategory::Established,
        321_042,
    );
    let first = runner.run(scenario.clone());
    let second = runner.run(scenario);
    assert_eq!(
        first.deterministic_fingerprint,
        second.deterministic_fingerprint
    );
    assert_eq!(first.invariants, second.invariants);
}

#[test]
fn restart_partition_hooks_are_deterministic_boolean_evidence_not_fake_passes() {
    let scenario = Lai32CampaignScenario::new(
        "partition_smoke",
        Lai32CampaignCategory::RestartPartition,
        336_001,
    );
    assert_eq!(
        compare_lai32_partitioned_snapshots_byte_equal(&scenario),
        compare_lai32_partitioned_snapshots_byte_equal(&scenario)
    );
    assert_eq!(
        compare_lai32_restart_snapshots_byte_equal(&scenario),
        compare_lai32_restart_snapshots_byte_equal(&scenario)
    );
}

#[test]
fn fresh_colony_seed_320000_no_longer_enters_reset_loop_and_progresses() {
    let trace = lai32_trace_fresh_colony_seed_320000();
    assert!(
        trace.reset_events.is_empty(),
        "first reset trace must stay empty after LAI.32E fix: {:?}",
        trace.reset_events.first()
    );
    assert_eq!(trace.final_outcome.scenario.seed, 320_000);
    assert_eq!(trace.final_outcome.ticks_executed, 2_880);
    assert_eq!(trace.final_outcome.reset_count, 0);
    assert!(
        trace.final_outcome.state.visible_task_count > 0,
        "visible LAI.23 tasks should persist bounded progress evidence"
    );
    assert!(
        trace.final_outcome.state.hole_credit_count >= 4,
        "physical Hole feeds should credit at least four exact Void Insight events"
    );
    assert_at_least_four_affordable_auto_research_commits(&trace.final_outcome)
        .expect("automatic research commits progress under the canonical rolling quota");
    assert_no_starvation_caused_solely_by_endless_hole_demand(&trace.final_outcome)
        .expect("fresh colony avoids the reset/liveJobs starvation artifact");
}

#[test]
fn fresh_colony_seed_320000_death_releases_survival_capacity_before_long_run() {
    let outcome = Lai32CampaignRunner::bounded_smoke(120).run(Lai32CampaignScenario::new(
        "fresh_colony_death_regression",
        Lai32CampaignCategory::Fresh,
        320_000,
    ));
    assert!(
        outcome.reset_count == 0,
        "bounded death/replacement regression must not reset: {:?}",
        outcome
    );
    assert!(
        outcome.state.alive_cat_count >= 30,
        "finite founding and frontier caves must preserve every founder while allowing physical population growth"
    );
    assert_eq!(
        outcome.state.cat_count, outcome.state.alive_cat_count,
        "the bounded bridge must not hide an early founder death behind a migrant or birth"
    );
    assert!(outcome.state.visible_task_count <= 256);
}

#[test]
#[ignore = "single-slot liveness diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_ticks_61_to_120() {
    lai32_probe_fresh_seed_320000_ticks_1_to_120(
        |sample| println!("tick-summary {sample:?}"),
        |phase| println!("tick-phase {phase:?}"),
        |probe| println!("probe-boundary {probe:?}"),
    );
}

#[test]
#[ignore = "single-slot causal survival diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_boundaries_90_to_120() {
    lai32_probe_fresh_seed_320000_boundaries_90_to_120(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot pre/post reset diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_boundaries_205_to_213() {
    lai32_probe_fresh_seed_320000_boundaries_205_to_213(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot post-Field-readiness diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_boundaries_214_to_230() {
    lai32_probe_fresh_seed_320000_boundaries_214_to_230(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot pre/post demographic reset diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_boundaries_2175_to_2185() {
    lai32_probe_fresh_seed_320000_boundaries_2175_to_2185(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot conception-gate diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_demography_gate() {
    lai32_probe_fresh_seed_320000_demography_gate(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot first-death causal diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_first_survival_death() {
    lai32_probe_fresh_seed_320000_first_survival_death(|sample| println!("{sample:?}"));
}

#[test]
#[ignore = "single-slot generational-liveness diagnostic; do not run with campaign shards"]
fn diagnostic_fresh_seed_320000_generational_gate() {
    lai32_probe_fresh_seed_320000_generational_gate(|sample| println!("{sample:?}"));
}

#[test]
fn release_profile_measurement_uses_external_samples_without_clock_reads() {
    let samples = [
        record_lai32_wall_time_and_peak_rss(18.0, 11_000),
        record_lai32_wall_time_and_peak_rss(18.5, 11_500),
        record_lai32_wall_time_and_peak_rss(19.0, 12_000),
    ];
    let evidence = measure_lai32_release_profile(&samples).expect("valid deterministic samples");
    assert_eq!(evidence.median_wall_seconds, 18.5);
    assert_eq!(evidence.median_peak_rss_kib, 11_500);
    compare_lai32_against_lai1_baseline(&evidence).expect("sample is within LAI.1 budget");
}

fn print_matrix_summary(label: &str, evidence: &Lai32CampaignMatrixEvidence) {
    println!(
        "{label}: sets={}, seeds={}, thresholdFailures={}, invariantFailureCounts={:?}",
        evidence.sets.len(),
        evidence.total_seed_count(),
        evidence.threshold_failures().len(),
        evidence.invariant_failure_counts()
    );
    for set in &evidence.sets {
        println!(
            "{label}: set={} category={:?} successes={}/{} seeds={}",
            set.set.id,
            set.set.category,
            set.successes,
            set.required_successes,
            set.outcomes.len()
        );
    }
    println!(
        "{label}: firstFailureExamples={}",
        serde_json::to_string_pretty(&evidence.first_failure_examples(20))
            .expect("failure examples serialize")
    );
}

fn assert_matrix_meets_acceptance(label: &str, evidence: &Lai32CampaignMatrixEvidence) {
    let threshold_failures = evidence
        .threshold_failures()
        .into_iter()
        .map(|set| {
            format!(
                "{} {}/{}",
                set.set.id, set.successes, set.required_successes
            )
        })
        .collect::<Vec<_>>();
    let invariant_failures = evidence.invariant_failure_counts();
    assert!(
        threshold_failures.is_empty() && invariant_failures.is_empty(),
        "{label} below LAI.32 acceptance: thresholdFailures={threshold_failures:?}, invariantFailures={invariant_failures:?}, examples={:?}",
        evidence.first_failure_examples(10)
    );
}

fn write_matrix_artifact(label: &str, evidence: &Lai32CampaignMatrixEvidence) {
    let Ok(dir) = env::var("LAI32_CAMPAIGN_ARTIFACT_DIR") else {
        return;
    };
    fs::create_dir_all(&dir).expect("artifact directory is writable");
    let path = format!("{dir}/{label}.json");
    let json = serde_json::to_string_pretty(evidence).expect("matrix evidence serializes");
    fs::write(&path, json).expect("matrix evidence artifact is writable");
    println!("{label}: artifact={path}");
}

#[test]
#[ignore = "release-profile campaign matrix is a LAI.32/LAI.34 gate, not a smoke test"]
fn ignored_release_profile_full_campaign_matrix_meets_lai1_budget() {
    assert_future_runner_symbols(
        "ignored_release_profile_full_campaign_matrix_meets_lai1_budget",
        &[
            "run_lai32_release_profile_campaign_matrix",
            "record_lai32_wall_time_and_peak_rss",
            "compare_lai32_against_lai1_baseline",
        ],
    );
    let evidence = run_lai32_release_profile_campaign_matrix();
    print_matrix_summary("LAI32_RELEASE_MATRIX", &evidence);
    assert_eq!(evidence.sets.len(), 17);
    assert_eq!(evidence.total_seed_count(), 1_700);
    assert_matrix_meets_acceptance("LAI32_RELEASE_MATRIX", &evidence);
}

#[test]
#[ignore = "restart/partition campaign matrix is intentionally expensive"]
fn ignored_restart_partition_matrix_is_byte_equal() {
    assert_future_runner_symbols(
        "ignored_restart_partition_matrix_is_byte_equal",
        &[
            "run_lai32_restart_partition_campaign_matrix",
            "compare_lai32_partitioned_snapshots_byte_equal",
            "compare_lai32_restart_snapshots_byte_equal",
        ],
    );
    let evidence = run_lai32_restart_partition_campaign_matrix();
    print_matrix_summary("LAI32_RESTART_PARTITION_MATRIX", &evidence);
    assert_eq!(evidence.sets.len(), 1);
    assert_eq!(evidence.total_seed_count(), 100);
    assert_matrix_meets_acceptance("LAI32_RESTART_PARTITION_MATRIX", &evidence);
}

#[test]
#[ignore = "release-profile campaign shard is intentionally expensive"]
fn ignored_release_profile_campaign_shard_from_env() {
    let set_id = env::var("LAI32_CAMPAIGN_SET")
        .expect("set LAI32_CAMPAIGN_SET to one id from the LAI.32 manifest");
    let set = lai32_campaign_seed_set_by_id(&set_id)
        .unwrap_or_else(|| panic!("unknown LAI32_CAMPAIGN_SET {set_id}"));
    let runner = Lai32CampaignRunner::release_horizon();
    let label = format!("lai32_release_shard_{set_id}");
    let mut outcomes = Vec::new();
    for offset in 0..set.seed_count {
        let outcome = runner.run(Lai32CampaignScenario::new(
            set.id,
            set.category,
            set.seed_start + offset,
        ));
        println!(
            "{label}: seed={} success={} failedInvariants={:?} ticks={} resets={} finalTickMs={} fingerprint={}",
            outcome.scenario.seed,
            outcome.success(),
            outcome.failed_invariants(),
            outcome.ticks_executed,
            outcome.reset_count,
            outcome.final_tick_ms,
            outcome.deterministic_fingerprint
        );
        outcomes.push(outcome);
    }
    let successes = outcomes
        .iter()
        .filter(|outcome| outcome.success())
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    let set_evidence = Lai32SeedSetEvidence {
        set,
        outcomes,
        successes,
        required_successes: set.category.success_threshold(),
    };
    let evidence = Lai32CampaignMatrixEvidence {
        sets: vec![set_evidence],
    };
    print_matrix_summary(&label, &evidence);
    write_matrix_artifact(&label, &evidence);
    assert_matrix_meets_acceptance(&label, &evidence);
}
