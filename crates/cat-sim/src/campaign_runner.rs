//! Deterministic LAI.32 campaign runner and evidence hooks.
//!
//! This module executes bounded campaign worlds through the pure `world_tick`
//! entrypoint. It records red/green evidence from the current production state;
//! it does not call live providers, clocks, threads, networking, or filesystem.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    beliefs::ReportLevel,
    migration::{
        DEFAULT_FOOD_PER_CAT, DEFAULT_MATERIALS_FLOOR, DEFAULT_MATERIALS_PER_CAT,
        DEFAULT_WATER_PER_CAT, migration_construction_wealth,
    },
    officers::OfficerRole,
    task_runtime::TaskCategory,
    types::BuildingType,
    world_tick::{
        ColonyRuntime, DeathCause, EventKind, JobMetadata, TickReport, WorldState,
        WorldTickPhaseDiagnostic, colony_housing_capacity, footprint_for, found_global_colony,
        lai32_debug_reachable_food_sources, lai32_debug_reachable_hunt_sources,
        migration_game_minute_at, new_world, world_tick, world_tick_with_phase_observer,
    },
};

pub const LAI32_GAME_DAYS: u32 = 30;
pub const LAI32_GAME_HOURS: u32 = LAI32_GAME_DAYS * 24;
pub const LAI32_DEFAULT_CADENCE_MS: u64 = 900_000;
pub const LAI32_SEEDS_PER_SET: u32 = 100;
pub const LAI32_FRESH_SUCCESS_THRESHOLD: u32 = 85;
pub const LAI32_ESTABLISHED_SUCCESS_THRESHOLD: u32 = 97;
pub const LAI32_RELEASE_PROFILE_MAX_REGRESSION_PERCENT: u32 = 25;
pub const LAI32_BASELINE_MEDIAN_WALL_SECONDS: f64 = 18.77;
pub const LAI32_BASELINE_MEDIAN_PEAK_RSS_KIB: u64 = 11_960;
pub const LAI32_MAX_MEDIAN_WALL_SECONDS: f64 = 23.4625;
pub const LAI32_MAX_MEDIAN_PEAK_RSS_KIB: u64 = 14_950;
pub const LAI32_CAMPAIGN_START_MS: i64 = 1_000;

const MAX_CAMPAIGN_CATS: usize = 4_096;
const MAX_CAMPAIGN_JOBS: usize = 2_048;
const MAX_CAMPAIGN_BUILDINGS: usize = 2_048;
const MAX_CAMPAIGN_EVENTS: usize = 8_192;
const MAX_CAMPAIGN_VISIBLE_TASKS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lai32CampaignCategory {
    Fresh,
    Established,
    Mature,
    Scarcity,
    Personality,
    Injury,
    MultiColony,
    Contention,
    Hole,
    Research,
    DiplomacyTrade,
    RestartPartition,
}

impl Lai32CampaignCategory {
    #[must_use]
    pub const fn success_threshold(self) -> u32 {
        match self {
            Self::Fresh | Self::Scarcity => LAI32_FRESH_SUCCESS_THRESHOLD,
            Self::Established
            | Self::Mature
            | Self::Personality
            | Self::Injury
            | Self::MultiColony
            | Self::Contention
            | Self::Hole
            | Self::Research
            | Self::DiplomacyTrade
            | Self::RestartPartition => LAI32_ESTABLISHED_SUCCESS_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignSeedSet {
    pub id: &'static str,
    pub category: Lai32CampaignCategory,
    pub seed_start: u32,
    pub seed_count: u32,
}

#[must_use]
pub const fn lai32_campaign_seed_sets() -> [Lai32CampaignSeedSet; 17] {
    [
        Lai32CampaignSeedSet {
            id: "fresh_colony",
            category: Lai32CampaignCategory::Fresh,
            seed_start: 320_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "established_colony",
            category: Lai32CampaignCategory::Established,
            seed_start: 321_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "mature_research_trade_colony",
            category: Lai32CampaignCategory::Mature,
            seed_start: 322_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_scarcity",
            category: Lai32CampaignCategory::Scarcity,
            seed_start: 323_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_devout",
            category: Lai32CampaignCategory::Personality,
            seed_start: 324_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_skeptical",
            category: Lai32CampaignCategory::Personality,
            seed_start: 325_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_mercantile",
            category: Lai32CampaignCategory::Personality,
            seed_start: 326_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_self_sufficient",
            category: Lai32CampaignCategory::Personality,
            seed_start: 327_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_bold",
            category: Lai32CampaignCategory::Personality,
            seed_start: 328_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "extreme_cautious",
            category: Lai32CampaignCategory::Personality,
            seed_start: 329_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "injury_prosthetic_stress",
            category: Lai32CampaignCategory::Injury,
            seed_start: 330_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "multi_colony",
            category: Lai32CampaignCategory::MultiColony,
            seed_start: 331_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "reservation_contention",
            category: Lai32CampaignCategory::Contention,
            seed_start: 332_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "hole_omission_bad_resource_choices",
            category: Lai32CampaignCategory::Hole,
            seed_start: 333_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "research_quota",
            category: Lai32CampaignCategory::Research,
            seed_start: 334_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "diplomacy_trade",
            category: Lai32CampaignCategory::DiplomacyTrade,
            seed_start: 335_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
        Lai32CampaignSeedSet {
            id: "restart_partition",
            category: Lai32CampaignCategory::RestartPartition,
            seed_start: 336_000,
            seed_count: LAI32_SEEDS_PER_SET,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignScenario {
    pub set_id: String,
    pub category: Lai32CampaignCategory,
    pub seed: u32,
    pub game_days: u32,
    pub cadence_ms: u64,
}

impl Lai32CampaignScenario {
    #[must_use]
    pub fn new(set_id: impl Into<String>, category: Lai32CampaignCategory, seed: u32) -> Self {
        Self {
            set_id: set_id.into(),
            category,
            seed,
            game_days: LAI32_GAME_DAYS,
            cadence_ms: LAI32_DEFAULT_CADENCE_MS,
        }
    }

    #[must_use]
    pub fn smoke(seed: u32) -> Self {
        Self {
            set_id: "small_smoke".to_owned(),
            category: Lai32CampaignCategory::Fresh,
            seed,
            game_days: 1,
            cadence_ms: LAI32_DEFAULT_CADENCE_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lai32Invariant {
    BoundedStateAndQueues,
    NoHoleOnlyStarvation,
    LeaderVariation,
    AutomaticResearchCommits,
    ExactVoidInsightConservation,
    HuntWaterWorkshopSpatial,
    HiddenRegenerationSecrecy,
    NoDuplicateMutations,
    TickPartitionTwins,
    RestartTwins,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32InvariantEvidence {
    pub invariant: Lai32Invariant,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignStateSummary {
    pub colony_count: usize,
    pub cat_count: usize,
    pub alive_cat_count: usize,
    pub live_job_count: usize,
    pub building_count: usize,
    pub event_count: usize,
    pub visible_task_count: usize,
    pub local_reservation_count: usize,
    pub world_reservation_count: usize,
    pub void_insight_balance_micro: u64,
    pub hole_credit_count: usize,
    pub automatic_research_commit_count: usize,
    pub leader_variation_score: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignOutcome {
    pub scenario: Lai32CampaignScenario,
    pub ticks_executed: u32,
    pub reset_count: u32,
    pub final_tick_ms: i64,
    pub state: Lai32CampaignStateSummary,
    pub invariants: Vec<Lai32InvariantEvidence>,
    pub deterministic_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32ResetTraceEvent {
    pub colony_id: String,
    pub tick_index: u32,
    pub now_ms: i64,
    pub reason: String,
    pub live_job_count: usize,
    pub visible_task_count: usize,
    pub resolved_spatial_task_count: usize,
    pub assigned_visible_task_count: usize,
    pub visible_task_stages: String,
    pub cat_task_summary: String,
    pub work_capable_cat_count: usize,
    pub local_reservation_count: usize,
    pub world_reservation_count: usize,
    pub food: u64,
    pub water: u64,
    pub critical_since_ms: Option<i64>,
    pub status: String,
    pub void_insight_balance_micro: u64,
    pub automatic_research_commit_count: usize,
    pub causal: Lai32CausalTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CausalTrace {
    pub alive_cat_count: usize,
    pub permanent_resident_count: usize,
    pub work_capable_cat_count: usize,
    pub pregnant_cat_count: usize,
    pub living_by_stage: BTreeMap<String, usize>,
    pub lifecycle_event_counts: BTreeMap<String, usize>,
    pub housing_capacity: u32,
    pub food_milli: u64,
    pub water_milli: u64,
    pub reported_food_milli: u64,
    pub reported_water_milli: u64,
    pub revealed_food_source_count: usize,
    pub revealed_food_units: u32,
    pub reachable_food_source_count: Option<usize>,
    pub reachable_food_units: Option<u32>,
    pub legal_hunt_source_count: usize,
    pub legal_hunt_units: u32,
    pub reachable_legal_hunt_source_count: Option<usize>,
    pub reachable_legal_hunt_units: Option<u32>,
    pub active_task_counts: BTreeMap<String, usize>,
    pub oldest_active_task_age_minutes: u64,
    pub active_task_details: Vec<String>,
    pub living_cat_details: Vec<String>,
    pub migration_gate: String,
    pub hole_pipeline: String,
    pub officer_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignResetTrace {
    pub scenario: Lai32CampaignScenario,
    pub reset_events: Vec<Lai32ResetTraceEvent>,
    pub final_outcome: Lai32CampaignOutcome,
}

impl Lai32CampaignOutcome {
    #[must_use]
    pub fn success(&self) -> bool {
        self.reset_count == 0 && self.invariants.iter().all(|invariant| invariant.passed)
    }

    #[must_use]
    pub fn failed_invariants(&self) -> Vec<Lai32Invariant> {
        self.invariants
            .iter()
            .filter(|invariant| !invariant.passed)
            .map(|invariant| invariant.invariant)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32SeedSetEvidence {
    pub set: Lai32CampaignSeedSet,
    pub outcomes: Vec<Lai32CampaignOutcome>,
    pub successes: u32,
    pub required_successes: u32,
}

impl Lai32SeedSetEvidence {
    #[must_use]
    pub fn threshold_met(&self) -> bool {
        self.successes >= self.required_successes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignMatrixEvidence {
    pub sets: Vec<Lai32SeedSetEvidence>,
}

impl Lai32CampaignMatrixEvidence {
    #[must_use]
    pub fn total_seed_count(&self) -> usize {
        self.sets.iter().map(|set| set.outcomes.len()).sum()
    }

    #[must_use]
    pub fn threshold_failures(&self) -> Vec<&Lai32SeedSetEvidence> {
        self.sets
            .iter()
            .filter(|set| !set.threshold_met())
            .collect()
    }

    #[must_use]
    pub fn invariant_failure_counts(&self) -> BTreeMap<Lai32Invariant, u32> {
        let mut counts = BTreeMap::new();
        for outcome in self.sets.iter().flat_map(|set| &set.outcomes) {
            for invariant in outcome
                .invariants
                .iter()
                .filter(|invariant| !invariant.passed)
            {
                *counts.entry(invariant.invariant).or_insert(0) += 1;
            }
        }
        counts
    }

    #[must_use]
    pub fn first_failure_examples(&self, max: usize) -> Vec<Lai32FailureExample> {
        let mut examples = Vec::new();
        for set in &self.sets {
            for outcome in &set.outcomes {
                for invariant in outcome
                    .invariants
                    .iter()
                    .filter(|invariant| !invariant.passed)
                {
                    examples.push(Lai32FailureExample {
                        set_id: set.set.id.to_owned(),
                        seed: outcome.scenario.seed,
                        invariant: invariant.invariant,
                        detail: invariant.detail.clone(),
                    });
                    if examples.len() >= max {
                        return examples;
                    }
                }
            }
        }
        examples
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32FailureExample {
    pub set_id: String,
    pub seed: u32,
    pub invariant: Lai32Invariant,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lai32CampaignFailure {
    pub invariant: &'static str,
    pub detail: String,
}

impl std::fmt::Display for Lai32CampaignFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.invariant, self.detail)
    }
}

impl std::error::Error for Lai32CampaignFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32CampaignPerformanceSample {
    pub wall_seconds: f64,
    pub peak_rss_kib: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lai32ReleaseProfileEvidence {
    pub samples: Vec<Lai32CampaignPerformanceSample>,
    pub median_wall_seconds: f64,
    pub median_peak_rss_kib: u64,
    pub max_wall_seconds: f64,
    pub max_peak_rss_kib: u64,
    pub allowed_regression_percent: u32,
    pub within_budget: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lai32CampaignRunner {
    pub max_ticks: Option<u32>,
    pub evaluate_restart_twins: bool,
}

/// Per-tick bounded diagnostics for the 61..=120 liveness window. This is
/// intentionally an explicit deterministic probe rather than wall-clock
/// instrumentation, so a later focused run can identify collection growth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lai32TickDiagnostic {
    pub tick: u32,
    pub visible_tasks: usize,
    pub resolved_spatial_tasks: usize,
    pub local_reservations: usize,
    pub world_reservations: usize,
    pub intents: usize,
    pub live_cats: usize,
    pub terminal_tasks: usize,
    pub active_tasks: usize,
    pub task_stages: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lai32ProbeBoundary {
    BeforeWorldBuild,
    AfterWorldBuild,
    BeforeTick,
    AfterTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lai32ProbeDiagnostic {
    pub boundary: Lai32ProbeBoundary,
    pub tick: Option<u32>,
    pub now_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lai32TickBoundaryDiagnostic {
    pub tick: u32,
    pub before_tick: bool,
    pub reset: Option<String>,
    pub run_number: u32,
    pub living_by_stage: BTreeMap<String, usize>,
    pub dead_cats: usize,
    pub pregnant_cats: usize,
    pub live_cat_states: Vec<String>,
    pub work_capable_cats: usize,
    pub hunt_anatomy_eligible: usize,
    pub water_anatomy_eligible: usize,
    pub hunt_willing: usize,
    pub water_willing: usize,
    pub raw_leader_id: Option<String>,
    pub runtime_leader_id: Option<String>,
    pub leader_effective_level: Option<u8>,
    pub leader_duty_minutes: u64,
    pub leader_cadence_minutes: Option<u32>,
    pub leader_forecast_horizon_hours: Option<u32>,
    pub food: String,
    pub water: String,
    pub carried_relief: String,
    pub report_food: String,
    pub report_water: String,
    pub hunt_resolution_diagnostic: String,
    pub available_food_sources: usize,
    pub full_hunt_load_sources: usize,
    pub available_food_units: u32,
    pub legal_hunt_sources: usize,
    pub full_legal_hunt_load_sources: usize,
    pub legal_hunt_units: u32,
    pub active_survival_tasks: usize,
    pub assigned_survival_tasks: usize,
    pub task_stages: BTreeMap<String, usize>,
    pub active_task_chains: Vec<String>,
    pub intents: Vec<String>,
    pub belief_diagnostics: Vec<String>,
    pub critical_since_ms: Option<i64>,
    pub lifecycle_events: BTreeMap<String, usize>,
    pub migration_departures: u64,
    pub complete_housing_buildings: BTreeMap<String, usize>,
    pub complete_fields: usize,
    pub in_flight_field_jobs: Vec<String>,
    pub active_expansion_jobs: Vec<String>,
    pub claimed_tile_count: usize,
    pub agricultural_tile_count: usize,
    pub farm_states: Vec<String>,
    pub food_chain_inventory: String,
    pub hole_pipeline: String,
    pub hole_review_diagnostic: String,
    pub hole_delivery_task_chains: Vec<String>,
    pub void_insight_balance_micro: u64,
    pub hole_credit_count: usize,
    pub owned_research_count: usize,
    pub research_commit_count: usize,
    pub automatic_research_commit_count: usize,
    pub personal_tasks: BTreeMap<String, usize>,
    pub imminent_cats: Vec<String>,
}

pub fn lai32_probe_fresh_seed_320000_boundaries_205_to_213(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    lai32_probe_fresh_seed_320000_boundaries(205, 213, &mut emit);
}

pub fn lai32_probe_fresh_seed_320000_boundaries_90_to_120(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    lai32_probe_fresh_seed_320000_boundaries(90, 120, &mut emit);
}

pub fn lai32_probe_fresh_seed_320000_boundaries_214_to_230(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    lai32_probe_fresh_seed_320000_boundaries(214, 230, &mut emit);
}

pub fn lai32_probe_fresh_seed_320000_boundaries_2175_to_2185(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    lai32_probe_fresh_seed_320000_boundaries(2_175, 2_185, &mut emit);
}

pub fn lai32_probe_fresh_seed_320000_demography_gate(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    let mut world = build_campaign_world(&scenario);
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    let sample_ticks = [140_u32, 144, 150, 180, 220, 300, 400, 600];
    for tick in 1..=*sample_ticks.last().expect("sample list is non-empty") {
        now_ms = now_ms.saturating_add(cadence_ms);
        let reports = world_tick(&mut world, now_ms);
        if !sample_ticks.contains(&tick) {
            continue;
        }
        if let Some(colony) = world.colonies.first() {
            let reset = reports
                .iter()
                .find_map(|report| report.reset_reason.map(|reason| format!("{reason:?}")));
            emit(lai32_boundary_snapshot(tick, false, reset, colony));
        }
    }
}

/// Emits the last at-risk pre-tick snapshot and the first post-tick snapshot
/// whose survival-death count increased. This keeps causal logging extensive
/// without printing hundreds of healthy campaign states.
pub fn lai32_probe_fresh_seed_320000_first_survival_death(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    let mut world = build_campaign_world(&scenario);
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    for tick in 1..=600 {
        now_ms = now_ms.saturating_add(cadence_ms);
        let Some(before_colony) = world.colonies.first() else {
            return;
        };
        let before_dead = before_colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_some())
            .count();
        let before_risk = before_colony.cats.iter().any(|cat| {
            cat.death_time.is_none()
                && (cat.needs.health < 100.0 || cat.needs.hunger < 35.0 || cat.needs.thirst < 35.0)
        });
        let before_snapshot =
            before_risk.then(|| lai32_boundary_snapshot(tick, true, None, before_colony));
        let reports = world_tick(&mut world, now_ms);
        let Some(after_colony) = world.colonies.first() else {
            return;
        };
        let after_dead = after_colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_some())
            .count();
        if after_dead > before_dead {
            if let Some(snapshot) = before_snapshot {
                emit(snapshot);
            }
            let reset = reports
                .iter()
                .find_map(|report| report.reset_reason.map(|reason| format!("{reason:?}")));
            emit(lai32_boundary_snapshot(tick, false, reset, after_colony));
            return;
        }
    }
}

pub fn lai32_probe_fresh_seed_320000_generational_gate(
    mut emit: impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    let mut world = build_campaign_world(&scenario);
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    let sample_ticks = [
        800_u32, 900, 1_000, 1_050, 1_080, 1_100, 1_110, 1_115, 1_120, 1_122, 1_123, 1_124,
    ];
    for tick in 1..=*sample_ticks.last().expect("sample list is non-empty") {
        now_ms = now_ms.saturating_add(cadence_ms);
        let reports = world_tick(&mut world, now_ms);
        if !sample_ticks.contains(&tick) {
            continue;
        }
        if let Some(colony) = world.colonies.first() {
            let reset = reports
                .iter()
                .find_map(|report| report.reset_reason.map(|reason| format!("{reason:?}")));
            emit(lai32_boundary_snapshot(tick, false, reset, colony));
        }
    }
}

fn lai32_probe_fresh_seed_320000_boundaries(
    first_tick: u32,
    last_tick: u32,
    emit: &mut impl FnMut(Lai32TickBoundaryDiagnostic),
) {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    let mut world = build_campaign_world(&scenario);
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    for tick in 1..=last_tick {
        now_ms = now_ms.saturating_add(cadence_ms);
        if !(first_tick..=last_tick).contains(&tick) {
            let _ = world_tick(&mut world, now_ms);
            continue;
        }
        if let Some(colony) = world.colonies.first() {
            emit(lai32_boundary_snapshot(tick, true, None, colony));
        }
        let reports = world_tick(&mut world, now_ms);
        if let Some(colony) = world.colonies.first() {
            let reset = reports
                .iter()
                .find_map(|report| report.reset_reason.map(|reason| format!("{reason:?}")));
            emit(lai32_boundary_snapshot(tick, false, reset, colony));
        }
    }
}

fn lai32_boundary_snapshot(
    tick: u32,
    before_tick: bool,
    reset: Option<String>,
    colony: &crate::world_tick::ColonyRuntime,
) -> Lai32TickBoundaryDiagnostic {
    use crate::{
        anatomy::HazardousJob,
        cat_willingness::{TaskPriority, TaskRisk, WillingnessContext, evaluate_willingness},
        life_sim::{can_work, get_life_stage},
        task_runtime::TaskCategory,
        workforce_matcher::refusal_bucket,
    };
    let mut living_by_stage = BTreeMap::new();
    let mut work_capable_cats = 0;
    let mut hunt_anatomy_eligible = 0;
    let mut water_anatomy_eligible = 0;
    let mut hunt_willing = 0;
    let mut water_willing = 0;
    let mut pregnant_cats = 0;
    let mut live_cat_states = Vec::new();
    let mut personal_tasks = BTreeMap::new();
    let mut imminent_cats = Vec::new();
    for cat in &colony.cats {
        if cat.death_time.is_some() {
            continue;
        }
        let stage = get_life_stage(cat.age_hours);
        *living_by_stage.entry(format!("{stage:?}")).or_insert(0) += 1;
        pregnant_cats += usize::from(cat.is_pregnant);
        *personal_tasks
            .entry(format!("{:?}", cat.current_task))
            .or_insert(0) += 1;
        let selected_personal_need = crate::world_tick::diagnostic_personal_need(cat);
        let personal_need_available = selected_personal_need.is_some_and(|task| {
            crate::world_tick::diagnostic_personal_need_is_available(colony, &cat.id, task)
        });
        let planner_cat_id = crate::planner_core::PlannerId::derive("cat", [cat.id.as_str()]);
        let locally_reserved = colony
            .leader_ai_runtime
            .scheduling
            .reservations
            .cat_is_busy(&planner_cat_id);
        let world_reserved = colony
            .leader_ai_runtime
            .scheduling
            .world_reservations
            .worker_is_reserved(&planner_cat_id);
        live_cat_states.push(format!(
            "{}:stage={stage:?},age={:.2},hunger={:.1},thirst={:.1},rest={:.1},health={:.1},task={:?},activity={:?},carrying={:?},pending_need={selected_personal_need:?},need_available={personal_need_available},local_reserved={locally_reserved},world_reserved={world_reserved},pregnant={},due_age={:?}",
            cat.id,
            cat.age_hours,
            cat.needs.hunger,
            cat.needs.thirst,
            cat.needs.rest,
            cat.needs.health,
            cat.current_task,
            cat.activity,
            cat.carrying.as_ref().map(|cargo| (cargo.kind, cargo.amount)),
            cat.is_pregnant,
            cat.pregnancy_due_age_hours
        ));
        if can_work(stage) {
            work_capable_cats += 1;
        }
        let Some(runtime) = colony.leader_ai_runtime.cat_physical.get(&cat.id) else {
            imminent_cats.push(format!("{}:missing_runtime", cat.id));
            continue;
        };
        let hunt = runtime.anatomy.job_capability(HazardousJob::Hunt);
        let hunt_ok = can_work(stage) && hunt.blocked.is_none();
        let water_ok = can_work(stage) && runtime.anatomy.movement_function_basis_points() > 0;
        hunt_anatomy_eligible += usize::from(hunt_ok);
        water_anatomy_eligible += usize::from(water_ok);
        let context = |category| WillingnessContext {
            refusal_bucket: refusal_bucket(320_000, &colony.id, &cat.id, category, u64::from(tick)),
            stress: runtime.stress.level,
            priority: TaskPriority::Emergency,
            risk: TaskRisk::High,
            pregnant: cat.is_pregnant,
            injured: runtime.anatomy != crate::anatomy::CatAnatomy::default(),
            safer_eligible_worker_exists: false,
        };
        hunt_willing +=
            usize::from(hunt_ok && evaluate_willingness(context("hunt")).accepts_assignment());
        water_willing +=
            usize::from(water_ok && evaluate_willingness(context("water")).accepts_assignment());
        if cat.needs.hunger < 20.0 || cat.needs.thirst < 20.0 || cat.death_time.is_some() {
            imminent_cats.push(format!(
                "{}:hunger={:.1},thirst={:.1},health={:.1}",
                cat.id, cat.needs.hunger, cat.needs.thirst, cat.needs.health
            ));
        }
    }
    let mut task_stages = BTreeMap::new();
    let mut active_survival_tasks = 0;
    let mut assigned_survival_tasks = 0;
    for task in colony.leader_ai_runtime.scheduling.visible_tasks.values() {
        *task_stages.entry(format!("{:?}", task.stage)).or_insert(0) += 1;
        if matches!(task.category, TaskCategory::Hunt | TaskCategory::FetchWater)
            && !task.stage.is_terminal()
        {
            active_survival_tasks += 1;
            assigned_survival_tasks += usize::from(!task.assigned_cat_ids.is_empty());
        }
    }
    let intents = colony
        .leader_ai_runtime
        .intents
        .iter()
        .map(|(_, intent)| {
            format!(
                "{:?}:{:?}:{}",
                intent.lifecycle.state, intent.kind_id, intent.target_id
            )
        })
        .collect();
    let active_task_chains = colony
        .leader_ai_runtime
        .scheduling
        .visible_tasks
        .iter()
        .filter(|(_, task)| !task.stage.is_terminal())
        .map(|(task_id, task)| {
            let objective = task
                .spatial
                .objective
                .as_ref()
                .map_or("none", |site| site.stable_id());
            let local_valid = task.reservation_id.as_ref().is_some_and(|reservation_id| {
                colony
                    .leader_ai_runtime
                    .scheduling
                    .reservations
                    .contains(reservation_id)
            });
            let world_valid = colony
                .leader_ai_runtime
                .scheduling
                .world_reservation_ids
                .get(task_id)
                .is_some_and(|reservation_id| {
                    colony
                        .leader_ai_runtime
                        .scheduling
                        .world_reservations
                        .contains(reservation_id)
                });
            format!(
                "{}:{:?}:{:?}:workers={:?}:objective={objective}:local={local_valid}:world={world_valid}:progress={}:cargo={:?}:blocked={:?}:updated={}",
                task_id.as_str(),
                task.category,
                task.stage,
                task.assigned_cat_ids,
                task.progress_basis_points,
                task.cargo,
                task.blocked_reason,
                task.updated_tick,
            )
        })
        .collect::<Vec<_>>();
    let belief_diagnostics = colony
        .leader_ai_runtime
        .beliefs
        .iter()
        .map(|(belief_id, record)| {
            format!(
                "{}:{:?}:level={:?}:value={:?}:confidence={}:effective_confidence={}:observed={}:expires={:?}:source={:?}:invalidated={}:contradictions={}",
                belief_id.as_str(),
                record.key.kind,
                record.report_level,
                record.value,
                record.confidence.get(),
                record
                    .effective_confidence(
                        colony
                            .leader_ai_runtime
                            .planner
                            .planning_clock
                            .max(record.observed_tick),
                    )
                    .get(),
                record.observed_tick,
                record.expires_tick,
                record.source,
                record.invalidated,
                record.contradiction_version,
            )
        })
        .collect::<Vec<_>>();
    let available_food_units = colony
        .world_tiles
        .values()
        .filter(|tile| colony.revealed_tiles.contains(&tile.pos) && tile.resources.food > 0)
        .map(|tile| tile.resources.food)
        .sum();
    let mut lifecycle_events = BTreeMap::new();
    for event in &colony.events {
        let key = match &event.kind {
            crate::world_tick::EventKind::Birth => Some("birth"),
            crate::world_tick::EventKind::Conception => Some("conception"),
            crate::world_tick::EventKind::Death(cause) => Some(match cause {
                crate::world_tick::DeathCause::OldAge => "death_old_age",
                crate::world_tick::DeathCause::Starvation => "death_starvation",
                crate::world_tick::DeathCause::Dehydration => "death_dehydration",
                crate::world_tick::DeathCause::StarvationAndDehydration => {
                    "death_starvation_and_dehydration"
                }
                crate::world_tick::DeathCause::Raid => "death_raid",
            }),
            crate::world_tick::EventKind::MigrationArrived => Some("migration_arrived"),
            crate::world_tick::EventKind::MigrationRetained => Some("migration_retained"),
            crate::world_tick::EventKind::MigrationDeparted => Some("migration_departed"),
            _ => None,
        };
        if let Some(key) = key {
            *lifecycle_events.entry(key.to_owned()).or_insert(0) += 1;
        }
    }
    let mut complete_housing_buildings = BTreeMap::new();
    for building in colony.buildings.iter().filter(|building| {
        building.is_complete
            && matches!(
                building.building_type,
                BuildingType::Den
                    | BuildingType::Beds
                    | BuildingType::Nursery
                    | BuildingType::ElderCorner
                    | BuildingType::FamilyHome
                    | BuildingType::ElderLodge
            )
    }) {
        *complete_housing_buildings
            .entry(format!("{:?}", building.building_type))
            .or_insert(0) += 1;
    }
    let complete_fields = colony
        .buildings
        .iter()
        .filter(|building| building.building_type == BuildingType::Field && building.is_complete)
        .count();
    let in_flight_field_jobs = colony
        .jobs
        .iter()
        .filter(|job| {
            matches!(
                job.status,
                crate::types::JobStatus::Queued | crate::types::JobStatus::Active
            ) && matches!(
                job.metadata,
                JobMetadata::Construction {
                    building_type: BuildingType::Field,
                    ..
                }
            )
        })
        .map(|job| {
            let readiness = match job.metadata {
                JobMetadata::Construction {
                    building_type: BuildingType::Field,
                    site: Some(site),
                    ..
                } => crate::world_tick::building_site_readiness_diagnostic(
                    colony,
                    site,
                    320_000,
                    BuildingType::Field,
                ),
                _ => "no-reserved-site".to_owned(),
            };
            format!(
                "{}:{:?}:worker={:?}:metadata={:?}:readiness={readiness}",
                job.id, job.status, job.assigned_cat, job.metadata,
            )
        })
        .collect();
    let farm_states = colony
        .farms
        .iter()
        .map(|plot| {
            let gather_id = crate::world_tick::farm_gather_spot_id(&plot.id);
            let gather_contents = colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == gather_id)
                .map(|pile| {
                    format!(
                        "food={:.3},grain={:.3},herbs={:.3},catnip={:.3}",
                        pile.contents.food,
                        pile.contents.grain,
                        pile.contents.herbs,
                        pile.contents.catnip
                    )
                })
                .unwrap_or_else(|| "missing".to_owned());
            format!(
                "{}:crop={:?}:tiles={}:harvest={:.3}:fertility={:.3}:growth={:.3}:phase={:?}:worker={:?}:pending={:.3}:gather={gather_contents}",
                plot.id,
                plot.crop,
                plot.tiles(),
                plot.harvest_amount(),
                plot.fertility,
                plot.growth_hours,
                plot.work_phase,
                plot.worker_id,
                plot.pending_output
            )
        })
        .collect();
    let active_expansion_jobs = colony
        .jobs
        .iter()
        .filter(|job| {
            job.kind == crate::types::JobKind::ExpandVillage
                && matches!(
                    job.status,
                    crate::types::JobStatus::Queued | crate::types::JobStatus::Active
                )
        })
        .map(|job| {
            format!(
                "{}:{:?}:worker={:?}:started={:?}:ends={:?}:metadata={:?}",
                job.id, job.status, job.assigned_cat, job.started_at, job.ends_at, job.metadata
            )
        })
        .collect();
    let hole = &colony.leader_ai_runtime.hole;
    let hole_pipeline = format!(
        "id={};nextOpening={};voidBalance={};activeFeed={};activeUpgrade={};credits={}",
        hole.hole_id,
        hole.next_opening_game_minute,
        hole.micro_void_balance,
        hole.active_feed.is_some(),
        hole.active_upgrade.is_some(),
        hole.credits().len(),
    );
    let hole_delivery_task_chains = colony
        .leader_ai_runtime
        .scheduling
        .visible_tasks
        .iter()
        .filter(|(_, task)| task.category == TaskCategory::HaulDelivery)
        .map(|(task_id, task)| {
            let local_valid = task.reservation_id.as_ref().is_some_and(|reservation_id| {
                colony
                    .leader_ai_runtime
                    .scheduling
                    .reservations
                    .contains(reservation_id)
            });
            let world = colony
                .leader_ai_runtime
                .scheduling
                .world_reservation_ids
                .get(task_id);
            let world_valid = world.is_some_and(|reservation_id| {
                colony
                    .leader_ai_runtime
                    .scheduling
                    .world_reservations
                    .contains(reservation_id)
            });
            format!(
                "{}:stage={:?}:workers={:?}:local={local_valid}:world={world_valid}:cargo={:?}:blocked={:?}:updated={}",
                task_id.as_str(),
                task.stage,
                task.assigned_cat_ids,
                task.cargo,
                task.blocked_reason,
                task.updated_tick,
            )
        })
        .collect::<Vec<_>>();
    let institution = colony.leader_ai_runtime.governance.officer_institution();
    let runtime_leader = institution.leader().cloned();
    let leader_effective_level: Option<crate::leader_planner::EffectiveLevel> = colony
        .leader_id
        .as_ref()
        .and_then(|leader_id| {
            colony
                .cats
                .iter()
                .find(|cat| cat.id == *leader_id && cat.death_time.is_none())
        })
        .and_then(|leader| {
            let level = if leader.stats.leadership >= 70.0 {
                5
            } else if leader.stats.leadership >= 40.0 {
                3
            } else {
                1
            };
            crate::leader_planner::EffectiveLevel::try_from(level).ok()
        });
    let leader_duty_minutes = runtime_leader.as_ref().map_or(0, |leader| {
        institution.leader_completed_duty_minutes(leader)
    });
    let research = &colony.leader_ai_runtime.research;
    Lai32TickBoundaryDiagnostic {
        tick,
        before_tick,
        reset,
        run_number: colony.run_number,
        living_by_stage,
        dead_cats: colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_some())
            .count(),
        pregnant_cats,
        live_cat_states,
        work_capable_cats,
        hunt_anatomy_eligible,
        water_anatomy_eligible,
        hunt_willing,
        water_willing,
        raw_leader_id: colony.leader_id.clone(),
        runtime_leader_id: runtime_leader.map(|id| id.to_string()),
        leader_effective_level: leader_effective_level.map(|level| level.get()),
        leader_duty_minutes,
        leader_cadence_minutes: leader_effective_level.map(|level| level.leader_cadence_minutes()),
        leader_forecast_horizon_hours: leader_effective_level
            .map(|level| level.forecast_horizon_hours()),
        food: format!("{:.3}", colony.resources.food),
        water: format!("{:.3}", colony.resources.water),
        carried_relief: colony
            .cats
            .iter()
            .filter_map(|cat| {
                cat.carrying
                    .as_ref()
                    .map(|cargo| format!("{}:{:?}:{:.3}", cat.id, cargo.kind, cargo.amount))
            })
            .collect::<Vec<_>>()
            .join(","),
        report_food: format!("{:.3}", colony.stock_ledger.reported.food),
        report_water: format!("{:.3}", colony.stock_ledger.reported.water),
        hunt_resolution_diagnostic: "canonical task routing is reported through scheduling state"
            .to_owned(),
        available_food_sources: colony
            .world_tiles
            .values()
            .filter(|tile| colony.revealed_tiles.contains(&tile.pos) && tile.resources.food > 0)
            .count(),
        full_hunt_load_sources: colony
            .world_tiles
            .values()
            .filter(|tile| colony.revealed_tiles.contains(&tile.pos) && tile.resources.food >= 24)
            .count(),
        available_food_units,
        legal_hunt_sources: colony
            .world_tiles
            .values()
            .filter(|tile| {
                colony.revealed_tiles.contains(&tile.pos)
                    && tile.tile_type == crate::types::TileType::CaveEntrance
                    && tile.resources.food > 0
            })
            .count(),
        full_legal_hunt_load_sources: colony
            .world_tiles
            .values()
            .filter(|tile| {
                colony.revealed_tiles.contains(&tile.pos)
                    && tile.tile_type == crate::types::TileType::CaveEntrance
                    && tile.resources.food >= 24
            })
            .count(),
        legal_hunt_units: colony
            .world_tiles
            .values()
            .filter(|tile| {
                colony.revealed_tiles.contains(&tile.pos)
                    && tile.tile_type == crate::types::TileType::CaveEntrance
            })
            .map(|tile| tile.resources.food)
            .sum(),
        active_survival_tasks,
        assigned_survival_tasks,
        task_stages,
        active_task_chains,
        intents,
        belief_diagnostics,
        critical_since_ms: colony.critical_since,
        lifecycle_events,
        migration_departures: colony.migration_departures,
        complete_housing_buildings,
        complete_fields,
        in_flight_field_jobs,
        active_expansion_jobs,
        claimed_tile_count: colony.claimed_tiles.len(),
        agricultural_tile_count: colony.agricultural_tiles.len(),
        farm_states,
        food_chain_inventory: format!(
            "food={:.3};fish={:.3};grain={:.3};flour={:.3};farmGatherGrain={:.3};millInputGrain={:.3};millInputFlour={:.3};millOutputFood={:.3}",
            colony.resources.food,
            colony.resources.fish,
            colony.resources.grain,
            colony.resources.flour,
            colony
                .stockpiles
                .iter()
                .filter(|pile| pile.id.starts_with("farm-gather:"))
                .map(|pile| pile.contents.grain)
                .sum::<f64>(),
            colony
                .buildings
                .iter()
                .filter(|building| building.building_type == BuildingType::Mill)
                .filter_map(|building| {
                    let id = crate::stockpiles::station_input_id(&building.id);
                    colony.stockpiles.iter().find(|pile| pile.id == id)
                })
                .map(|pile| pile.contents.grain)
                .sum::<f64>(),
            colony
                .buildings
                .iter()
                .filter(|building| building.building_type == BuildingType::Mill)
                .filter_map(|building| {
                    let id = crate::stockpiles::station_input_id(&building.id);
                    colony.stockpiles.iter().find(|pile| pile.id == id)
                })
                .map(|pile| pile.contents.flour)
                .sum::<f64>(),
            colony
                .buildings
                .iter()
                .filter(|building| building.building_type == BuildingType::Mill)
                .filter_map(|building| {
                    let id = crate::stockpiles::station_output_id(&building.id);
                    colony.stockpiles.iter().find(|pile| pile.id == id)
                })
                .map(|pile| pile.contents.food)
                .sum::<f64>(),
        ),
        hole_pipeline,
        hole_review_diagnostic: format!(
            "holeVersion={};policyEntries={};openClickTargets={}",
            colony.leader_ai_runtime.divine_hole.version,
            colony
                .leader_ai_runtime
                .divine_hole
                .edible_policy
                .entries
                .len(),
            colony.leader_ai_runtime.divine_hole.click_targets.len(),
        ),
        hole_delivery_task_chains,
        void_insight_balance_micro: research.void.balance.micro(),
        hole_credit_count: usize::try_from(research.void.credited_feed_through)
            .unwrap_or(usize::MAX),
        owned_research_count: research.owned_finite.len(),
        research_commit_count: research.leader_commits.len(),
        automatic_research_commit_count: research.leader_commits.len(),
        personal_tasks,
        imminent_cats,
    }
}

pub fn lai32_probe_fresh_seed_320000_ticks_1_to_120(
    mut emit_tick: impl FnMut(Lai32TickDiagnostic),
    mut emit_phase: impl FnMut(WorldTickPhaseDiagnostic),
    mut emit_probe: impl FnMut(Lai32ProbeDiagnostic),
) {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    emit_probe(Lai32ProbeDiagnostic {
        boundary: Lai32ProbeBoundary::BeforeWorldBuild,
        tick: None,
        now_ms: None,
    });
    let mut world = build_campaign_world(&scenario);
    emit_probe(Lai32ProbeDiagnostic {
        boundary: Lai32ProbeBoundary::AfterWorldBuild,
        tick: None,
        now_ms: Some(LAI32_CAMPAIGN_START_MS),
    });
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    for tick in 1..=120 {
        now_ms = now_ms.saturating_add(cadence_ms);
        emit_probe(Lai32ProbeDiagnostic {
            boundary: Lai32ProbeBoundary::BeforeTick,
            tick: Some(tick),
            now_ms: Some(now_ms),
        });
        let _ = world_tick_with_phase_observer(&mut world, now_ms, &mut emit_phase);
        emit_probe(Lai32ProbeDiagnostic {
            boundary: Lai32ProbeBoundary::AfterTick,
            tick: Some(tick),
            now_ms: Some(now_ms),
        });
        let Some(colony) = world.colonies.first() else {
            continue;
        };
        let mut task_stages = BTreeMap::new();
        for task in colony.leader_ai_runtime.scheduling.visible_tasks.values() {
            *task_stages.entry(format!("{:?}", task.stage)).or_insert(0) += 1;
        }
        let terminal_tasks = colony
            .leader_ai_runtime
            .scheduling
            .visible_tasks
            .values()
            .filter(|task| task.stage.is_terminal())
            .count();
        emit_tick(Lai32TickDiagnostic {
            tick,
            visible_tasks: colony.leader_ai_runtime.scheduling.visible_tasks.len(),
            resolved_spatial_tasks: colony
                .leader_ai_runtime
                .scheduling
                .resolved_spatial_tasks
                .len(),
            local_reservations: colony.leader_ai_runtime.scheduling.reservations.len(),
            world_reservations: colony.leader_ai_runtime.scheduling.world_reservations.len(),
            intents: colony.leader_ai_runtime.intents.iter().count(),
            live_cats: colony
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .count(),
            terminal_tasks,
            active_tasks: colony
                .leader_ai_runtime
                .scheduling
                .visible_tasks
                .len()
                .saturating_sub(terminal_tasks),
            task_stages,
        });
    }
}

impl Lai32CampaignRunner {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_ticks: None,
            evaluate_restart_twins: true,
        }
    }

    #[must_use]
    pub const fn bounded_smoke(max_ticks: u32) -> Self {
        Self {
            max_ticks: Some(max_ticks),
            evaluate_restart_twins: true,
        }
    }

    #[must_use]
    pub const fn release_horizon() -> Self {
        Self {
            max_ticks: None,
            evaluate_restart_twins: false,
        }
    }

    #[must_use]
    pub const fn restart_partition_horizon() -> Self {
        Self {
            max_ticks: None,
            evaluate_restart_twins: true,
        }
    }

    #[must_use]
    pub fn run(&self, scenario: Lai32CampaignScenario) -> Lai32CampaignOutcome {
        let mut world = build_campaign_world(&scenario);
        let (ticks_executed, reset_count, final_tick_ms) =
            run_world_for_scenario(&mut world, &scenario, self.max_ticks);
        outcome_from_world(
            scenario,
            &world,
            ticks_executed,
            reset_count,
            final_tick_ms,
            self.evaluate_restart_twins,
        )
    }

    #[must_use]
    pub fn run_seed_set(&self, set: Lai32CampaignSeedSet) -> Lai32SeedSetEvidence {
        let outcomes = (0..set.seed_count)
            .map(|offset| {
                self.run(Lai32CampaignScenario::new(
                    set.id,
                    set.category,
                    set.seed_start + offset,
                ))
            })
            .collect::<Vec<_>>();
        seed_set_evidence(set, outcomes)
    }
}

impl Default for Lai32CampaignRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn lai32_run_30_day_campaign(scenario: Lai32CampaignScenario) -> Lai32CampaignOutcome {
    Lai32CampaignRunner::new().run(scenario)
}

#[must_use]
pub fn lai32_run_small_red_smoke_campaigns() -> Vec<Lai32CampaignOutcome> {
    let runner = Lai32CampaignRunner::bounded_smoke(8);
    [320_000, 321_000]
        .into_iter()
        .map(|seed| runner.run(Lai32CampaignScenario::smoke(seed)))
        .collect()
}

#[must_use]
pub fn lai32_trace_fresh_colony_seed_320000() -> Lai32CampaignResetTrace {
    let scenario =
        Lai32CampaignScenario::new("fresh_colony", Lai32CampaignCategory::Fresh, 320_000);
    let mut world = build_campaign_world(&scenario);
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    let mut reset_events = Vec::new();
    let mut reset_count = 0_u32;
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    let horizon_ticks = u64::from(scenario.game_days)
        .saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        .saturating_mul(1_000)
        / scenario.cadence_ms.max(1);
    let ticks = horizon_ticks.min(u64::from(u32::MAX)) as u32;
    for tick_index in 1..=ticks {
        now_ms = now_ms.saturating_add(cadence_ms);
        let before_state = summarize_world(&world);
        let before_visible_stages = visible_task_stage_summary(&world);
        let before_resolved_spatial_tasks = resolved_spatial_task_count(&world);
        let before_assigned_visible_tasks = assigned_visible_task_count(&world);
        let before_cat_tasks = cat_task_summary(&world);
        let before_work_capable_cats = work_capable_cat_count(&world);
        let before_food = world_resource_floor(&world, |resources| resources.food + resources.fish);
        let before_water = world_resource_floor(&world, |resources| resources.water);
        let before_critical_since = world
            .colonies
            .first()
            .and_then(|colony| colony.critical_since);
        let before_status = world
            .colonies
            .first()
            .map(|colony| format!("{:?}", colony.status))
            .unwrap_or_else(|| "MissingColony".to_owned());
        let before_causal = world
            .colonies
            .iter()
            .map(|colony| (colony.id.clone(), lai32_colony_causal_trace(colony, now_ms)))
            .collect::<BTreeMap<_, _>>();
        let reports = world_tick(&mut world, now_ms);
        for report in reports
            .iter()
            .filter(|report| report.reset_reason.is_some())
        {
            reset_count = reset_count.saturating_add(1);
            reset_events.push(Lai32ResetTraceEvent {
                colony_id: report.colony_id.clone(),
                tick_index,
                now_ms,
                reason: format!("{:?}", report.reset_reason.expect("filtered")),
                live_job_count: before_state.live_job_count,
                visible_task_count: before_state.visible_task_count,
                resolved_spatial_task_count: before_resolved_spatial_tasks,
                assigned_visible_task_count: before_assigned_visible_tasks,
                visible_task_stages: before_visible_stages.clone(),
                cat_task_summary: before_cat_tasks.clone(),
                work_capable_cat_count: before_work_capable_cats,
                local_reservation_count: before_state.local_reservation_count,
                world_reservation_count: before_state.world_reservation_count,
                food: before_food,
                water: before_water,
                critical_since_ms: before_critical_since,
                status: before_status.clone(),
                void_insight_balance_micro: before_state.void_insight_balance_micro,
                automatic_research_commit_count: before_state.automatic_research_commit_count,
                causal: before_causal
                    .get(&report.colony_id)
                    .cloned()
                    .expect("reported colony existed before its tick"),
            });
        }
    }
    let final_outcome =
        outcome_from_world(scenario.clone(), &world, ticks, reset_count, now_ms, false);
    Lai32CampaignResetTrace {
        scenario,
        reset_events,
        final_outcome,
    }
}

#[must_use]
pub fn run_lai32_release_profile_campaign_matrix() -> Lai32CampaignMatrixEvidence {
    let runner = Lai32CampaignRunner::release_horizon();
    let sets = lai32_campaign_seed_sets()
        .into_iter()
        .map(|set| runner.run_seed_set(set))
        .collect();
    Lai32CampaignMatrixEvidence { sets }
}

#[must_use]
pub fn lai32_campaign_seed_set_by_id(set_id: &str) -> Option<Lai32CampaignSeedSet> {
    lai32_campaign_seed_sets()
        .into_iter()
        .find(|set| set.id == set_id)
}

#[must_use]
pub fn run_lai32_release_profile_campaign_shard(
    set_id: &str,
) -> Option<Lai32CampaignMatrixEvidence> {
    let runner = Lai32CampaignRunner::release_horizon();
    let set = lai32_campaign_seed_set_by_id(set_id)?;
    Some(Lai32CampaignMatrixEvidence {
        sets: vec![runner.run_seed_set(set)],
    })
}

#[must_use]
pub fn run_lai32_restart_partition_campaign_matrix() -> Lai32CampaignMatrixEvidence {
    let runner = Lai32CampaignRunner::restart_partition_horizon();
    let restart_partition = lai32_campaign_seed_sets()
        .into_iter()
        .filter(|set| set.category == Lai32CampaignCategory::RestartPartition)
        .map(|set| runner.run_seed_set(set))
        .collect();
    Lai32CampaignMatrixEvidence {
        sets: restart_partition,
    }
}

pub fn assert_lai32_fresh_success_threshold_85_of_100(
    evidence: &Lai32SeedSetEvidence,
) -> Result<(), Lai32CampaignFailure> {
    assert_threshold(
        "assert_lai32_fresh_success_threshold_85_of_100",
        evidence,
        LAI32_FRESH_SUCCESS_THRESHOLD,
    )
}

pub fn assert_lai32_established_success_threshold_97_of_100(
    evidence: &Lai32SeedSetEvidence,
) -> Result<(), Lai32CampaignFailure> {
    assert_threshold(
        "assert_lai32_established_success_threshold_97_of_100",
        evidence,
        LAI32_ESTABLISHED_SUCCESS_THRESHOLD,
    )
}

pub fn assert_lai32_bounded_state_and_queues(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::BoundedStateAndQueues,
        "assert_lai32_bounded_state_and_queues",
    )
}

pub fn assert_no_starvation_caused_solely_by_endless_hole_demand(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::NoHoleOnlyStarvation,
        "assert_no_starvation_caused_solely_by_endless_hole_demand",
    )
}

pub fn assert_believable_good_and_bad_leader_variation(
    evidence: &Lai32SeedSetEvidence,
) -> Result<(), Lai32CampaignFailure> {
    let mut scores = evidence
        .outcomes
        .iter()
        .map(|outcome| outcome.state.leader_variation_score);
    let Some(first) = scores.next() else {
        return Err(failure(
            "assert_believable_good_and_bad_leader_variation",
            "seed-set evidence is empty",
        ));
    };
    let varied = scores.any(|score| score != first);
    if varied {
        Ok(())
    } else {
        Err(failure(
            "assert_believable_good_and_bad_leader_variation",
            "all seed outcomes produced the same leader-variation score",
        ))
    }
}

pub fn assert_at_least_four_affordable_auto_research_commits(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    if outcome.state.automatic_research_commit_count >= 4 {
        Ok(())
    } else {
        Err(failure(
            "assert_at_least_four_affordable_auto_research_commits",
            format!(
                "observed {} automatic research commits",
                outcome.state.automatic_research_commit_count
            ),
        ))
    }
}

pub fn assert_exact_void_insight_conservation(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::ExactVoidInsightConservation,
        "assert_exact_void_insight_conservation",
    )
}

pub fn assert_hunt_water_workshop_spatial_invariants(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::HuntWaterWorkshopSpatial,
        "assert_hunt_water_workshop_spatial_invariants",
    )
}

pub fn assert_hidden_regeneration_secrecy_below_l4(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::HiddenRegenerationSecrecy,
        "assert_hidden_regeneration_secrecy_below_l4",
    )
}

pub fn assert_no_duplicate_void_research_trade_cargo_mutations(
    outcome: &Lai32CampaignOutcome,
) -> Result<(), Lai32CampaignFailure> {
    assert_outcome_invariant(
        outcome,
        Lai32Invariant::NoDuplicateMutations,
        "assert_no_duplicate_void_research_trade_cargo_mutations",
    )
}

pub fn assert_lai32_tick_partition_twins(
    scenario: &Lai32CampaignScenario,
) -> Result<(), Lai32CampaignFailure> {
    if compare_lai32_partitioned_snapshots_byte_equal(scenario) {
        Ok(())
    } else {
        Err(failure(
            "assert_lai32_tick_partition_twins",
            "partitioned tick schedule produced a different deterministic fingerprint",
        ))
    }
}

pub fn assert_lai32_restart_twins(
    scenario: &Lai32CampaignScenario,
) -> Result<(), Lai32CampaignFailure> {
    if compare_lai32_restart_snapshots_byte_equal(scenario) {
        Ok(())
    } else {
        Err(failure(
            "assert_lai32_restart_twins",
            "restart twin produced a different deterministic fingerprint",
        ))
    }
}

#[must_use]
pub fn compare_lai32_partitioned_snapshots_byte_equal(scenario: &Lai32CampaignScenario) -> bool {
    let mut direct = build_campaign_world(scenario);
    let mut partitioned = build_campaign_world(scenario);
    let total_step_ms = [1_000_i64, 60_000, 900_000, 3_600_000]
        .into_iter()
        .fold(0_i64, i64::saturating_add);
    let direct_tick = LAI32_CAMPAIGN_START_MS.saturating_add(total_step_ms);
    let _ = world_tick(&mut direct, direct_tick);
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    for step_ms in [1_000_i64, 60_000, 900_000, 3_600_000] {
        now_ms = now_ms.saturating_add(step_ms);
        let _ = world_tick(&mut partitioned, now_ms);
    }
    direct_tick == now_ms && world_fingerprint(&direct) == world_fingerprint(&partitioned)
}

#[must_use]
pub fn compare_lai32_restart_snapshots_byte_equal(scenario: &Lai32CampaignScenario) -> bool {
    let mut uninterrupted = build_campaign_world(scenario);
    let mut restarted = build_campaign_world(scenario);
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    let first = LAI32_CAMPAIGN_START_MS.saturating_add(cadence_ms);
    let second = first.saturating_add(cadence_ms);
    let _ = world_tick(&mut uninterrupted, first);
    let _ = world_tick(&mut uninterrupted, second);
    let _ = world_tick(&mut restarted, first);
    let mut restarted = restarted.clone();
    let _ = world_tick(&mut restarted, second);
    world_fingerprint(&uninterrupted) == world_fingerprint(&restarted)
}

#[must_use]
pub fn record_lai32_wall_time_and_peak_rss(
    wall_seconds: f64,
    peak_rss_kib: u64,
) -> Lai32CampaignPerformanceSample {
    Lai32CampaignPerformanceSample {
        wall_seconds,
        peak_rss_kib,
    }
}

pub fn measure_lai32_release_profile(
    samples: &[Lai32CampaignPerformanceSample],
) -> Result<Lai32ReleaseProfileEvidence, Lai32CampaignFailure> {
    if samples.is_empty()
        || samples
            .iter()
            .any(|sample| !sample.wall_seconds.is_finite() || sample.wall_seconds < 0.0)
    {
        return Err(failure(
            "measure_lai32_release_profile",
            "samples must be non-empty with finite nonnegative wall seconds",
        ));
    }
    let mut wall = samples
        .iter()
        .map(|sample| sample.wall_seconds)
        .collect::<Vec<_>>();
    wall.sort_by(f64::total_cmp);
    let mut rss = samples
        .iter()
        .map(|sample| sample.peak_rss_kib)
        .collect::<Vec<_>>();
    rss.sort_unstable();
    let median_wall_seconds = median_f64(&wall);
    let median_peak_rss_kib = median_u64(&rss);
    let evidence = Lai32ReleaseProfileEvidence {
        samples: samples.to_vec(),
        median_wall_seconds,
        median_peak_rss_kib,
        max_wall_seconds: LAI32_MAX_MEDIAN_WALL_SECONDS,
        max_peak_rss_kib: LAI32_MAX_MEDIAN_PEAK_RSS_KIB,
        allowed_regression_percent: LAI32_RELEASE_PROFILE_MAX_REGRESSION_PERCENT,
        within_budget: median_wall_seconds <= LAI32_MAX_MEDIAN_WALL_SECONDS
            && median_peak_rss_kib <= LAI32_MAX_MEDIAN_PEAK_RSS_KIB,
    };
    Ok(evidence)
}

pub fn compare_lai32_against_lai1_baseline(
    evidence: &Lai32ReleaseProfileEvidence,
) -> Result<(), Lai32CampaignFailure> {
    if evidence.within_budget {
        Ok(())
    } else {
        Err(failure(
            "compare_lai32_against_lai1_baseline",
            format!(
                "median wall {:.4}s / RSS {} KiB exceeds {:.4}s / {} KiB",
                evidence.median_wall_seconds,
                evidence.median_peak_rss_kib,
                evidence.max_wall_seconds,
                evidence.max_peak_rss_kib
            ),
        ))
    }
}

fn build_campaign_world(scenario: &Lai32CampaignScenario) -> WorldState {
    let mut world = new_world(scenario.seed);
    let mut colony = found_global_colony(
        scenario.seed,
        format!("{}-{}", scenario.set_id, scenario.seed),
        LAI32_CAMPAIGN_START_MS,
        scenario.seed,
    );
    match scenario.category {
        Lai32CampaignCategory::Established
        | Lai32CampaignCategory::Mature
        | Lai32CampaignCategory::Research
        | Lai32CampaignCategory::DiplomacyTrade
        | Lai32CampaignCategory::RestartPartition => {
            colony.resources.food = colony.resources.food.max(200.0);
            colony.resources.water = colony.resources.water.max(200.0);
            colony.resources.materials = colony.resources.materials.max(200.0);
        }
        Lai32CampaignCategory::Scarcity => {
            colony.resources.food = colony.resources.food.min(12.0);
            colony.resources.water = colony.resources.water.min(12.0);
        }
        Lai32CampaignCategory::Fresh
        | Lai32CampaignCategory::Personality
        | Lai32CampaignCategory::Injury
        | Lai32CampaignCategory::MultiColony
        | Lai32CampaignCategory::Contention
        | Lai32CampaignCategory::Hole => {}
    }
    world.colonies.push(colony);
    world
}

fn run_world_for_scenario(
    world: &mut WorldState,
    scenario: &Lai32CampaignScenario,
    max_ticks: Option<u32>,
) -> (u32, u32, i64) {
    let cadence_ms = i64::try_from(scenario.cadence_ms).unwrap_or(i64::MAX);
    let horizon_ticks = u64::from(scenario.game_days)
        .saturating_mul(24)
        .saturating_mul(60)
        .saturating_mul(60)
        .saturating_mul(1_000)
        / scenario.cadence_ms.max(1);
    let ticks = max_ticks
        .map(u64::from)
        .unwrap_or(horizon_ticks)
        .min(u64::from(u32::MAX)) as u32;
    let mut reset_count = 0_u32;
    let mut now_ms = LAI32_CAMPAIGN_START_MS;
    for _ in 0..ticks {
        now_ms = now_ms.saturating_add(cadence_ms);
        reset_count = reset_count.saturating_add(count_resets(&world_tick(world, now_ms)));
    }
    (ticks, reset_count, now_ms)
}

fn count_resets(reports: &[TickReport]) -> u32 {
    reports
        .iter()
        .filter(|report| report.reset_reason.is_some())
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn outcome_from_world(
    scenario: Lai32CampaignScenario,
    world: &WorldState,
    ticks_executed: u32,
    reset_count: u32,
    final_tick_ms: i64,
    evaluate_restart_twins: bool,
) -> Lai32CampaignOutcome {
    let state = summarize_world(world);
    let mut invariants = vec![
        invariant(
            Lai32Invariant::BoundedStateAndQueues,
            bounded_state_and_queues(&state),
            format!(
                "cats={}, jobs={}, buildings={}, events={}, visibleTasks={}",
                state.cat_count,
                state.live_job_count,
                state.building_count,
                state.event_count,
                state.visible_task_count
            ),
        ),
        invariant(
            Lai32Invariant::NoHoleOnlyStarvation,
            reset_count == 0 && state.live_job_count <= MAX_CAMPAIGN_JOBS,
            format!(
                "resetCount={reset_count}, liveJobs={}",
                state.live_job_count
            ),
        ),
        invariant(
            Lai32Invariant::LeaderVariation,
            state.leader_variation_score > 0,
            format!("score={}", state.leader_variation_score),
        ),
        invariant(
            Lai32Invariant::AutomaticResearchCommits,
            state.automatic_research_commit_count >= 4,
            format!(
                "automaticResearchCommits={}",
                state.automatic_research_commit_count
            ),
        ),
        invariant(
            Lai32Invariant::ExactVoidInsightConservation,
            void_insight_conservation(world),
            format!(
                "voidInsightBalanceMicro={}, holeCredits={}",
                state.void_insight_balance_micro, state.hole_credit_count
            ),
        ),
        invariant(
            Lai32Invariant::HuntWaterWorkshopSpatial,
            hunt_water_workshop_spatial_invariants(world),
            "Hunt/Water visible tasks have objectives and Workshop footprint is 3x3".to_owned(),
        ),
        invariant(
            Lai32Invariant::HiddenRegenerationSecrecy,
            hidden_regeneration_secrecy_below_l4(),
            "ReportLevel <= 3 does not project regeneration".to_owned(),
        ),
        invariant(
            Lai32Invariant::NoDuplicateMutations,
            no_duplicate_mutations(world),
            "Void/research/trade/task cargo identifiers are unique in observed state".to_owned(),
        ),
    ];
    if evaluate_restart_twins {
        invariants.push(invariant(
            Lai32Invariant::TickPartitionTwins,
            compare_lai32_partitioned_snapshots_byte_equal(&scenario),
            "bounded partition twin fingerprint comparison".to_owned(),
        ));
        invariants.push(invariant(
            Lai32Invariant::RestartTwins,
            compare_lai32_restart_snapshots_byte_equal(&scenario),
            "bounded restart twin fingerprint comparison".to_owned(),
        ));
    }
    Lai32CampaignOutcome {
        scenario,
        ticks_executed,
        reset_count,
        final_tick_ms,
        state,
        invariants,
        deterministic_fingerprint: world_fingerprint(world),
    }
}

fn seed_set_evidence(
    set: Lai32CampaignSeedSet,
    outcomes: Vec<Lai32CampaignOutcome>,
) -> Lai32SeedSetEvidence {
    let successes = outcomes
        .iter()
        .filter(|outcome| outcome.success())
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    Lai32SeedSetEvidence {
        set,
        outcomes,
        successes,
        required_successes: set.category.success_threshold(),
    }
}

fn summarize_world(world: &WorldState) -> Lai32CampaignStateSummary {
    let mut summary = Lai32CampaignStateSummary {
        colony_count: world.colonies.len(),
        cat_count: 0,
        alive_cat_count: 0,
        live_job_count: 0,
        building_count: 0,
        event_count: 0,
        visible_task_count: 0,
        local_reservation_count: 0,
        world_reservation_count: 0,
        void_insight_balance_micro: 0,
        hole_credit_count: 0,
        automatic_research_commit_count: 0,
        leader_variation_score: 0,
    };
    for colony in &world.colonies {
        summary.cat_count += colony.cats.len();
        summary.alive_cat_count += colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count();
        summary.live_job_count += colony
            .jobs
            .iter()
            .filter(|job| job.completed_at.is_none())
            .count();
        summary.building_count += colony.buildings.len();
        summary.event_count += colony.events.len();
        let runtime = &colony.leader_ai_runtime;
        summary.visible_task_count += runtime.scheduling.visible_tasks.len();
        summary.local_reservation_count += runtime.scheduling.reservations.len();
        summary.world_reservation_count += runtime.scheduling.world_reservations.len();
        summary.void_insight_balance_micro = summary
            .void_insight_balance_micro
            .saturating_add(runtime.research.void.balance.micro());
        summary.hole_credit_count = summary.hole_credit_count.saturating_add(
            usize::try_from(runtime.research.void.credited_feed_through).unwrap_or(usize::MAX),
        );
        summary.automatic_research_commit_count += runtime.research.leader_commits.len();
        summary.leader_variation_score = summary.leader_variation_score.saturating_add(
            runtime
                .planner
                .planning_epoch
                .saturating_add(runtime.intents.iter().len() as u64)
                .saturating_add(runtime.officer_requests.iter().len() as u64)
                .saturating_add(runtime.scheduling.visible_tasks.len() as u64)
                .saturating_add(runtime.research.owned_finite.len() as u64)
                .saturating_add(colony.events.len() as u64),
        );
    }
    summary
}

fn lai32_colony_causal_trace(colony: &ColonyRuntime, now_ms: i64) -> Lai32CausalTrace {
    let probationary_ids = colony
        .migration_state
        .probationary_migrants
        .iter()
        .map(|migrant| migrant.id.as_str())
        .collect::<BTreeSet<_>>();
    let living = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .collect::<Vec<_>>();
    let permanent_resident_count = living
        .iter()
        .filter(|cat| !probationary_ids.contains(cat.id.as_str()))
        .count();
    let work_capable_cat_count = living
        .iter()
        .filter(|cat| crate::life_sim::can_work(crate::life_sim::get_life_stage(cat.age_hours)))
        .count();
    let pregnant_cat_count = living.iter().filter(|cat| cat.is_pregnant).count();
    let mut living_by_stage = BTreeMap::new();
    for cat in &living {
        *living_by_stage
            .entry(format!(
                "{:?}",
                crate::life_sim::get_life_stage(cat.age_hours)
            ))
            .or_default() += 1;
    }

    let mut lifecycle_event_counts = BTreeMap::new();
    for event in &colony.events {
        let label = match &event.kind {
            EventKind::Birth => Some("birth"),
            EventKind::Conception => Some("conception"),
            EventKind::Death(cause) => Some(match cause {
                DeathCause::OldAge => "death_old_age",
                DeathCause::Starvation => "death_starvation",
                DeathCause::Dehydration => "death_dehydration",
                DeathCause::StarvationAndDehydration => "death_starvation_and_dehydration",
                DeathCause::Raid => "death_raid",
            }),
            EventKind::MigrationArrived => Some("migration_arrived"),
            EventKind::MigrationRetained => Some("migration_retained"),
            EventKind::MigrationDeparted => Some("migration_departed"),
            _ => None,
        };
        if let Some(label) = label {
            *lifecycle_event_counts.entry(label.to_owned()).or_default() += 1;
        }
    }

    let current_game_minute = migration_game_minute_at(colony, now_ms);
    let mut active_task_counts = BTreeMap::new();
    let mut oldest_active_task_age_minutes = 0_u64;
    let mut active_task_details = Vec::new();
    let detailed_trace = colony.critical_since.is_some();
    for task in colony
        .leader_ai_runtime
        .scheduling
        .visible_tasks
        .values()
        .filter(|task| !task.stage.is_terminal())
    {
        *active_task_counts
            .entry(format!("{:?}:{:?}", task.category, task.stage))
            .or_default() += 1;
        oldest_active_task_age_minutes = oldest_active_task_age_minutes
            .max(current_game_minute.saturating_sub(task.updated_tick));
        if detailed_trace {
            active_task_details.push(format!(
                "{}:{:?}:{:?}:ageMin={}:workers={:?}:source={:?}:work={:?}:delivery={:?}",
                task.id.as_str(),
                task.category,
                task.stage,
                current_game_minute.saturating_sub(task.updated_tick),
                task.assigned_cat_ids,
                task.spatial.objective.as_ref().map(|site| site.stable_id()),
                task.spatial
                    .work_positions
                    .first()
                    .map(|slot| slot.site.stable_id()),
                task.spatial
                    .delivery_endpoint
                    .as_ref()
                    .map(|site| site.stable_id()),
            ));
        }
    }
    active_task_details.sort();
    active_task_details.truncate(24);
    let living_cat_details = if colony.critical_since.is_some() {
        living
            .iter()
            .map(|cat| {
                format!(
                    "{}:age={:.2}:hunger={:.2}:thirst={:.2}:health={:.2}:task={:?}:activity={:?}:position=({:.2},{:.2}):destination={:?}",
                    cat.id,
                    cat.age_hours,
                    cat.needs.hunger,
                    cat.needs.thirst,
                    cat.needs.health,
                    cat.current_task,
                    cat.activity,
                    cat.position.x,
                    cat.position.y,
                    cat.destination,
                )
            })
            .take(24)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let (revealed_food_source_count, revealed_food_units) = colony
        .world_tiles
        .values()
        .filter(|tile| colony.revealed_tiles.contains(&tile.pos) && tile.resources.food > 0)
        .fold((0_usize, 0_u32), |(count, units), tile| {
            (count + 1, units.saturating_add(tile.resources.food))
        });
    // Reachability walks are intentionally conditional: the ordinary trace stays
    // cheap, while the final critical window gains exact route-aware evidence.
    let reachable_food = colony
        .critical_since
        .is_some()
        .then(|| lai32_debug_reachable_food_sources(colony))
        .flatten();
    let reachable_legal_hunt = colony
        .critical_since
        .is_some()
        .then(|| lai32_debug_reachable_hunt_sources(colony))
        .flatten();
    let (legal_hunt_source_count, legal_hunt_units) = colony
        .world_tiles
        .values()
        .filter(|tile| {
            colony.revealed_tiles.contains(&tile.pos)
                && tile.tile_type == crate::types::TileType::CaveEntrance
                && tile.resources.food > 0
        })
        .fold((0_usize, 0_u32), |(count, units), tile| {
            (count + 1, units.saturating_add(tile.resources.food))
        });

    let housing_capacity = colony_housing_capacity(colony)
        .floor()
        .clamp(0.0, f64::from(u32::MAX)) as u32;
    let population =
        permanent_resident_count.saturating_add(colony.migration_state.probationary_migrants.len());
    let population_f64 = population as f64;
    let food = colony.resources.food + colony.resources.fish;
    let water = colony.resources.water;
    let construction_wealth = migration_construction_wealth(
        colony.resources.materials,
        colony.resources.planks,
        colony.resources.blocks,
        colony.resources.lumber,
    );
    let required_food = DEFAULT_FOOD_PER_CAT * population_f64;
    let required_water = DEFAULT_WATER_PER_CAT * population_f64;
    let required_construction =
        (DEFAULT_MATERIALS_PER_CAT * population_f64).max(DEFAULT_MATERIALS_FLOOR);
    let migration_gate = format!(
        "elapsedMin={current_game_minute};population={population};pending={};housing={housing_capacity};pregnancyReservations={pregnant_cat_count};food={food:.3}/{required_food:.3}:{};water={water:.3}/{required_water:.3}:{};construction={construction_wealth:.3}/{required_construction:.3}:{};crisis={};lastBucket={:?}",
        colony.migration_state.probationary_migrants.len(),
        food >= required_food,
        water >= required_water,
        construction_wealth >= required_construction,
        colony.critical_since.is_some()
            || colony.active_raid.is_some()
            || matches!(
                colony.status,
                crate::entities::ColonyStatus::Struggling | crate::entities::ColonyStatus::Dead
            ),
        colony.migration_state.last_evaluated_cohort_bucket,
    );

    let hole = &colony.leader_ai_runtime.hole;
    let hole_pipeline = format!(
        "id={};nextOpening={};voidBalance={};activeFeed={};activeUpgrade={};credits={}",
        hole.hole_id,
        hole.next_opening_game_minute,
        hole.micro_void_balance,
        hole.active_feed.is_some(),
        hole.active_upgrade.is_some(),
        hole.credits().len(),
    );
    let officer_state = format!(
        "leader={:?};filled={}",
        colony
            .leader_ai_runtime
            .governance
            .officer_institution()
            .leader()
            .map(ToString::to_string),
        OfficerRole::ALL
            .iter()
            .filter_map(|role| {
                colony
                    .leader_ai_runtime
                    .governance
                    .officer_institution()
                    .officer(*role)
                    .map(|id| format!("{role:?}={id}"))
            })
            .collect::<Vec<_>>()
            .join(",")
    );

    Lai32CausalTrace {
        alive_cat_count: living.len(),
        permanent_resident_count,
        work_capable_cat_count,
        pregnant_cat_count,
        living_by_stage,
        lifecycle_event_counts,
        housing_capacity,
        food_milli: finite_milli(food),
        water_milli: finite_milli(water),
        reported_food_milli: finite_milli(
            colony.stock_ledger.reported.food + colony.stock_ledger.reported.fish,
        ),
        reported_water_milli: finite_milli(colony.stock_ledger.reported.water),
        revealed_food_source_count,
        revealed_food_units,
        reachable_food_source_count: reachable_food.map(|(count, _)| count),
        reachable_food_units: reachable_food.map(|(_, units)| units),
        legal_hunt_source_count,
        legal_hunt_units,
        reachable_legal_hunt_source_count: reachable_legal_hunt.map(|(count, _)| count),
        reachable_legal_hunt_units: reachable_legal_hunt.map(|(_, units)| units),
        active_task_counts,
        oldest_active_task_age_minutes,
        active_task_details,
        living_cat_details,
        migration_gate,
        hole_pipeline,
        officer_state,
    }
}

fn finite_milli(value: f64) -> u64 {
    if !value.is_finite() {
        return 0;
    }
    (value.max(0.0) * 1_000.0)
        .round()
        .clamp(0.0, u64::MAX as f64) as u64
}

fn visible_task_stage_summary(world: &WorldState) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for task in world
        .colonies
        .iter()
        .flat_map(|colony| colony.leader_ai_runtime.scheduling.visible_tasks.values())
    {
        *counts
            .entry(format!("{:?}:{:?}", task.category, task.stage))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(stage, count)| format!("{stage}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn assigned_visible_task_count(world: &WorldState) -> usize {
    world
        .colonies
        .iter()
        .flat_map(|colony| colony.leader_ai_runtime.scheduling.visible_tasks.values())
        .filter(|task| !task.assigned_cat_ids.is_empty())
        .count()
}

fn resolved_spatial_task_count(world: &WorldState) -> usize {
    world
        .colonies
        .iter()
        .map(|colony| {
            colony
                .leader_ai_runtime
                .scheduling
                .resolved_spatial_tasks
                .len()
        })
        .sum()
}

fn cat_task_summary(world: &WorldState) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for cat in world
        .colonies
        .iter()
        .flat_map(|colony| colony.cats.iter())
        .filter(|cat| cat.death_time.is_none())
    {
        let label = cat
            .current_task
            .map(|task| format!("{task:?}"))
            .unwrap_or_else(|| "None".to_owned());
        *counts.entry(label).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(task, count)| format!("{task}={count}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn work_capable_cat_count(world: &WorldState) -> usize {
    world
        .colonies
        .iter()
        .flat_map(|colony| colony.cats.iter())
        .filter(|cat| {
            cat.death_time.is_none()
                && crate::life_sim::can_work(crate::life_sim::get_life_stage(cat.age_hours))
        })
        .count()
}

fn world_resource_floor(
    world: &WorldState,
    resource: impl Fn(&crate::entities::Resources) -> f64,
) -> u64 {
    world
        .colonies
        .iter()
        .map(|colony| resource(&colony.resources).floor().max(0.0) as u64)
        .sum()
}

fn bounded_state_and_queues(state: &Lai32CampaignStateSummary) -> bool {
    state.cat_count <= MAX_CAMPAIGN_CATS
        && state.live_job_count <= MAX_CAMPAIGN_JOBS
        && state.building_count <= MAX_CAMPAIGN_BUILDINGS
        && state.event_count <= MAX_CAMPAIGN_EVENTS
        && state.visible_task_count <= MAX_CAMPAIGN_VISIBLE_TASKS
}

fn void_insight_conservation(world: &WorldState) -> bool {
    world.colonies.iter().all(|colony| {
        let ledger = &colony.leader_ai_runtime.research.void;
        serde_json::to_value(ledger)
            .and_then(serde_json::from_value::<crate::progression_research::VoidInsightLedger>)
            .is_ok()
    })
}

fn hunt_water_workshop_spatial_invariants(world: &WorldState) -> bool {
    let workshop_size_ok = footprint_for(BuildingType::Workshop) == (3, 3);
    let task_spatial_ok = world.colonies.iter().all(|colony| {
        colony
            .leader_ai_runtime
            .scheduling
            .visible_tasks
            .values()
            .all(|task| match task.category {
                TaskCategory::Hunt | TaskCategory::FetchWater => task.spatial.objective.is_some(),
                TaskCategory::WorkshopWork => task
                    .spatial
                    .footprint()
                    .is_none_or(|footprint| footprint.width == 3 && footprint.height == 3),
                TaskCategory::Fish
                | TaskCategory::Quarry
                | TaskCategory::Logging
                | TaskCategory::Replant
                | TaskCategory::BuildingConstruction
                | TaskCategory::RoadConstruction
                | TaskCategory::StationWork
                | TaskCategory::FarmWork
                | TaskCategory::HaulDelivery
                | TaskCategory::StockpileTransfer
                | TaskCategory::FibreForage
                | TaskCategory::Scout
                | TaskCategory::Expansion
                | TaskCategory::OfferingRitual
                | TaskCategory::Training
                | TaskCategory::Accounting
                | TaskCategory::Eat
                | TaskCategory::Drink
                | TaskCategory::Sleep => true,
            })
    });
    workshop_size_ok && task_spatial_ok
}

fn hidden_regeneration_secrecy_below_l4() -> bool {
    !ReportLevel::One.regeneration_visible()
        && !ReportLevel::Two.regeneration_visible()
        && !ReportLevel::Three.regeneration_visible()
        && ReportLevel::Four.regeneration_visible()
}

fn no_duplicate_mutations(world: &WorldState) -> bool {
    world.colonies.iter().all(|colony| {
        let runtime = &colony.leader_ai_runtime;
        unique_strings(
            runtime
                .scheduling
                .visible_tasks
                .values()
                .filter_map(|task| task.cargo.as_ref().map(|cargo| cargo.cargo_id.as_str())),
        ) && serde_json::to_value(&runtime.research.void)
            .and_then(serde_json::from_value::<crate::progression_research::VoidInsightLedger>)
            .is_ok()
            && runtime.research.leader_commits.len() <= runtime.research.version as usize
            && runtime.trade.summary().active_contract_count
                <= runtime.trade.summary().contract_count
    })
}

fn unique_strings<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.into_iter().all(|value| seen.insert(value))
}

fn invariant(invariant: Lai32Invariant, passed: bool, detail: String) -> Lai32InvariantEvidence {
    Lai32InvariantEvidence {
        invariant,
        passed,
        detail,
    }
}

fn assert_outcome_invariant(
    outcome: &Lai32CampaignOutcome,
    invariant: Lai32Invariant,
    name: &'static str,
) -> Result<(), Lai32CampaignFailure> {
    match outcome
        .invariants
        .iter()
        .find(|evidence| evidence.invariant == invariant)
    {
        Some(evidence) if evidence.passed => Ok(()),
        Some(evidence) => Err(failure(name, evidence.detail.clone())),
        None => Err(failure(name, "invariant evidence is missing")),
    }
}

fn assert_threshold(
    name: &'static str,
    evidence: &Lai32SeedSetEvidence,
    expected: u32,
) -> Result<(), Lai32CampaignFailure> {
    if evidence.successes >= expected {
        Ok(())
    } else {
        Err(failure(
            name,
            format!(
                "observed {} successes, required {expected}",
                evidence.successes
            ),
        ))
    }
}

fn failure(name: &'static str, detail: impl Into<String>) -> Lai32CampaignFailure {
    Lai32CampaignFailure {
        invariant: name,
        detail: detail.into(),
    }
}

fn world_fingerprint(world: &WorldState) -> String {
    let mut colonies = world.colonies.iter().collect::<Vec<_>>();
    colonies.sort_by(|left, right| left.id.cmp(&right.id));
    let mut parts = vec![format!("world:{}", world.world_seed)];
    for colony in colonies {
        let runtime = &colony.leader_ai_runtime;
        parts.push(format!(
            "colony:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            colony.id,
            colony.last_tick,
            colony.cats.len(),
            colony.jobs.len(),
            colony.buildings.len(),
            colony.resources.food.to_bits(),
            colony.resources.water.to_bits(),
            runtime.planner.planning_epoch,
            runtime.research.void.balance.micro(),
            runtime.research.version,
        ));
        for job in &colony.jobs {
            parts.push(format!(
                "job:{}:{:?}:{:?}:{}:{}",
                job.id,
                job.kind,
                job.status,
                job.created_at,
                job.completed_at.unwrap_or(0)
            ));
        }
        for building in &colony.buildings {
            parts.push(format!(
                "building:{}:{:?}:{}:{}:{}",
                building.id,
                building.building_type,
                building.position.x,
                building.position.y,
                building.worker_count()
            ));
        }
        for task in runtime.scheduling.visible_tasks.values() {
            parts.push(format!(
                "task:{}:{:?}:{:?}:{}",
                task.id.as_str(),
                task.category,
                task.stage,
                task.updated_tick
            ));
        }
    }
    parts.join("|")
}

fn median_f64(values: &[f64]) -> f64 {
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn median_u64(values: &[u64]) -> u64 {
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        values[mid - 1].saturating_add(values[mid]) / 2
    } else {
        values[mid]
    }
}
