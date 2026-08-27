//! Data-driven whole-game playtest contract.
//!
//! The design documents define behavior families, while these stable records make
//! discovery, seed policy, milestones, and persistence checkpoints machine-checkable.

use serde::Serialize;

pub(crate) mod catalog_journeys;
pub(crate) mod scouting;
pub(crate) mod system_journeys;
pub(crate) mod weapons_leader;
pub(crate) mod worker_catalog;

pub(crate) const PRIMARY_SEED: u32 = 4_242;
pub(crate) const HIGH_RISK_SEEDS: &[u32] = &[7, 42, 99, PRIMARY_SEED, 0xCA97_A111];
pub(crate) const NIGHTLY_SEEDS: &[u32] = &[
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    42,
    99,
    PRIMARY_SEED,
    0xCA97_A111,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SeedTier {
    Primary,
    HighRisk,
    Nightly,
}

impl SeedTier {
    pub(crate) fn seeds(self) -> &'static [u32] {
        match self {
            Self::Primary => &[PRIMARY_SEED],
            Self::HighRisk => HIGH_RISK_SEEDS,
            Self::Nightly => NIGHTLY_SEEDS,
        }
    }
}

pub(crate) fn requested_seed_tier() -> SeedTier {
    match std::env::var("CAT_PLAYTEST_SEED_TIER").as_deref() {
        Ok("high-risk") => SeedTier::HighRisk,
        Ok("nightly") => SeedTier::Nightly,
        Ok("primary") | Err(_) => SeedTier::Primary,
        Ok(other) => {
            panic!("CAT_PLAYTEST_SEED_TIER must be primary, high-risk, or nightly; got {other:?}")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Milestone {
    pub(crate) id: &'static str,
    pub(crate) description: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScenarioSpec {
    pub(crate) id: &'static str,
    pub(crate) design_anchor: &'static str,
    pub(crate) initial_setup: &'static str,
    pub(crate) action_or_trigger: &'static str,
    pub(crate) milestones: &'static [Milestone],
    pub(crate) horizon_ms: i64,
    pub(crate) allowed_outcomes: &'static [&'static str],
    pub(crate) seed_tier: SeedTier,
    pub(crate) persistence_checkpoints: &'static [&'static str],
}

const SYSTEM_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "signed-control",
        description: "the relevant signed control or autonomous trigger is accepted",
    },
    Milestone {
        id: "physical-effect",
        description: "the authoritative physical lifecycle reaches its documented effect",
    },
    Milestone {
        id: "restart-equality",
        description: "save, restart, reconnect, and projected state comparison succeed",
    },
];

macro_rules! system_scenario {
    ($id:literal, $anchor:literal, $setup:literal, $trigger:literal) => {
        ScenarioSpec {
            id: $id,
            design_anchor: $anchor,
            initial_setup: $setup,
            action_or_trigger: $trigger,
            milestones: SYSTEM_MILESTONES,
            horizon_ms: 7 * 24 * 60 * 60 * 1_000,
            allowed_outcomes: &["documented deterministic outcome"],
            seed_tier: SeedTier::HighRisk,
            persistence_checkpoints: &["after physical effect"],
        }
    };
}

/// Behavior-family coverage exercised by the executable WebSocket journey modules.
pub(crate) const SYSTEM_SCENARIOS: &[ScenarioSpec] = &[
    system_scenario!(
        "fresh-world-survival-and-needs",
        "docs/GAME_VISION.md#founding-housing-breeding-and-migration",
        "fresh 15-adult three-Den founding",
        "unattended needs and emergency supply work"
    ),
    system_scenario!(
        "housing-breeding-migration-aging-extinction",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "founding plus reserved and unreserved beds",
        "pregnancy, migration, probation, aging, and extinction horizons"
    ),
    system_scenario!(
        "all-officers-vacant-and-assigned",
        "docs/GAME_VISION.md#manual-role-automation-one-building-at-a-time",
        "each officer domain first vacant and then staffed",
        "signed assign and unassign controls followed by owned work"
    ),
    system_scenario!(
        "research-blessings-and-shrine-work",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "affordable cat-research and blessing-owned nodes",
        "signed research, tithe, offering, and ritual actions"
    ),
    system_scenario!(
        "elections-voting-and-vote-kick",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "scheduled election and eligible stable player identities",
        "signed vote and vote-kick actions across reconnect"
    ),
    system_scenario!(
        "raids-training-and-defense",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "trained and untrained defenders with a bounded incoming raid",
        "signed training and defense controls"
    ),
    system_scenario!(
        "stockpiles-gather-spots-and-hauling",
        "docs/GAME_VISION.md#visible-stockpiles",
        "finite sources, destinations, and blocked/full boundaries",
        "signed designation and real carrier movement"
    ),
    system_scenario!(
        "roads-bridges-rail-and-shipping",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "revealed land, river crossings, track, docks, and vehicles",
        "signed infrastructure and finite route controls"
    ),
    system_scenario!(
        "traders-and-village-trade",
        "docs/GAME_VISION.md#production-chains-farming-lots-to-do-always-slightly-cat",
        "visiting trader and mutually discovered villages",
        "signed buy, sell, offer, accept, cancel, and caravan lifecycle"
    ),
    system_scenario!(
        "multi-village-selection-and-restart",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "global plus two owner-scoped villages",
        "found, discover, join, select, save, and reconnect"
    ),
    system_scenario!(
        "every-building-plan-build-staff-operate-persist",
        "docs/GAME_VISION.md#buildings-upgrade-tree",
        "one valid funded site for every constructible BuildingType",
        "signed plan followed by physical delivery, construction, staffing, and operation"
    ),
    system_scenario!(
        "every-recipe-conserved-station-work-and-delivery",
        "docs/IMPLEMENTATION_AUDIT.md#current-design-document-traceability",
        "finite exact inputs and staffed compatible station for every recipe",
        "signed queue edit followed by station-local work and physical output delivery"
    ),
    system_scenario!(
        "every-crop-designate-grow-yield-persist",
        "docs/GAME_VISION.md#production-chains-farming-lots-to-do-always-slightly-cat",
        "one legal exterior plot and staffed Field for each typed crop",
        "signed designation followed by physical field work, growth, harvest, delivery, and restart"
    ),
    system_scenario!(
        "every-finite-deposit-extract-carry-deplete-persist",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "one reachable finite deposit in every applicable fine-grained biome",
        "signed extraction followed by physical work, carrying, delivery, depletion, and restart"
    ),
    system_scenario!(
        "every-item-variant-persist-and-equipment-actions",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "one exact instance of every equipment kind, material, and quality combination",
        "snapshot projection and restart plus signed equip and unequip actions for functional families"
    ),
    system_scenario!(
        "every-resource-storage-roundtrip-and-transport-families",
        "docs/GAME_VISION.md#visible-stockpiles",
        "nonzero distinct values for every typed resource in authoritative and physical storage",
        "snapshot projection, SQLite restart equality, and composed physical transport journeys"
    ),
    system_scenario!(
        "shrine-demand-ritual-lifecycle",
        "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix",
        "a completed shrine, comfortable finite stores, a vacant Loremaster office, and idle cats placed away from the shrine",
        "signed RequestJob(Ritual), duplicate-demand rejection, physical shrine work, blessing return, and restart"
    ),
];

pub(crate) fn manifest() -> Vec<&'static ScenarioSpec> {
    SYSTEM_SCENARIOS
        .iter()
        .chain(scouting::SCENARIOS.iter())
        .chain(weapons_leader::SCENARIOS.iter())
        .chain(worker_catalog::SCENARIOS.iter())
        .collect()
}

pub(crate) fn executable_scenario_ids() -> Vec<&'static str> {
    system_journeys::EXECUTABLE_SCENARIO_IDS
        .iter()
        .chain(catalog_journeys::EXECUTABLE_SCENARIO_IDS)
        .chain(scouting::EXECUTABLE_SCENARIO_IDS)
        .chain(weapons_leader::EXECUTABLE_SCENARIO_IDS)
        .chain(worker_catalog::EXECUTABLE_SCENARIO_IDS)
        .copied()
        .collect()
}

#[test]
fn scenario_manifest_has_unique_stable_ids_and_valid_seed_tiers() {
    let scenarios = manifest();
    assert!(
        !scenarios.is_empty(),
        "whole-game scenario manifest is empty"
    );
    let mut ids = scenarios
        .iter()
        .map(|scenario| scenario.id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    for pair in ids.windows(2) {
        assert_ne!(pair[0], pair[1], "duplicate scenario id {}", pair[0]);
    }
    for scenario in scenarios {
        assert!(
            scenario
                .id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'),
            "scenario id must be lowercase kebab-case: {}",
            scenario.id
        );
        assert!(
            !scenario.design_anchor.is_empty(),
            "{} has no design anchor",
            scenario.id
        );
        assert!(
            !scenario.milestones.is_empty(),
            "{} has no milestones",
            scenario.id
        );
        assert!(
            scenario.horizon_ms > 0,
            "{} has no time horizon",
            scenario.id
        );
        assert!(!scenario.seed_tier.seeds().is_empty());
    }
    assert_eq!(NIGHTLY_SEEDS.len(), 32);
}

#[test]
fn deterministic_seed_cohorts_match_the_playtest_contract_exactly() {
    assert_eq!(PRIMARY_SEED, 4_242);
    assert_eq!(
        HIGH_RISK_SEEDS,
        &[7, 42, 99, 4_242, 0xCA97_A111],
        "the high-risk cohort is a stable replay contract"
    );
    assert_eq!(
        NIGHTLY_SEEDS,
        &[
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
            16,
            17,
            18,
            19,
            20,
            21,
            22,
            23,
            24,
            25,
            26,
            27,
            42,
            99,
            4_242,
            0xCA97_A111,
        ]
    );
    assert_eq!(
        NIGHTLY_SEEDS
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        NIGHTLY_SEEDS.len(),
        "nightly seeds must not silently repeat work"
    );
    assert!(
        HIGH_RISK_SEEDS
            .iter()
            .all(|seed| NIGHTLY_SEEDS.contains(seed)),
        "nightly must contain the complete high-risk cohort"
    );
}

fn normalized_anchor(value: &str) -> String {
    let mut normalized = String::new();
    let mut separator_pending = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator_pending && !normalized.is_empty() {
                normalized.push('-');
            }
            normalized.push(character);
            separator_pending = false;
        } else if character.is_whitespace() || character == '-' {
            separator_pending = true;
        }
    }
    normalized
}

fn design_document(path: &str) -> Option<&'static str> {
    match path {
        "docs/GAME_VISION.md" => Some(include_str!("../../../../docs/GAME_VISION.md")),
        "docs/IMPLEMENTATION_AUDIT.md" => {
            Some(include_str!("../../../../docs/IMPLEMENTATION_AUDIT.md"))
        }
        _ => None,
    }
}

#[test]
fn scenario_specs_are_complete_and_anchor_real_design_headings() {
    let mut failures = Vec::new();
    for scenario in manifest() {
        if scenario.initial_setup.trim().is_empty() {
            failures.push(format!("{} has no initial setup", scenario.id));
        }
        if scenario.action_or_trigger.trim().is_empty() {
            failures.push(format!("{} has no action or trigger", scenario.id));
        }
        if scenario.allowed_outcomes.is_empty() {
            failures.push(format!("{} has no allowed outcomes", scenario.id));
        }
        if scenario.persistence_checkpoints.is_empty() {
            failures.push(format!("{} has no persistence checkpoints", scenario.id));
        }

        let milestone_ids = scenario
            .milestones
            .iter()
            .map(|milestone| milestone.id)
            .collect::<std::collections::BTreeSet<_>>();
        if milestone_ids.len() != scenario.milestones.len() {
            failures.push(format!("{} has duplicate milestone IDs", scenario.id));
        }
        if scenario
            .milestones
            .iter()
            .any(|milestone| milestone.id.is_empty() || milestone.description.trim().is_empty())
        {
            failures.push(format!("{} has an incomplete milestone", scenario.id));
        }
        if scenario
            .allowed_outcomes
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != scenario.allowed_outcomes.len()
        {
            failures.push(format!("{} has duplicate allowed outcomes", scenario.id));
        }
        if scenario
            .persistence_checkpoints
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != scenario.persistence_checkpoints.len()
        {
            failures.push(format!(
                "{} has duplicate persistence checkpoints",
                scenario.id
            ));
        }

        let link = scenario
            .design_anchor
            .split(" / ")
            .next()
            .unwrap_or(scenario.design_anchor);
        let Some((path, fragment)) = link.split_once('#') else {
            failures.push(format!(
                "{} has a non-link design anchor {:?}",
                scenario.id, scenario.design_anchor
            ));
            continue;
        };
        let Some(document) = design_document(path) else {
            failures.push(format!("{} anchors unknown document {path:?}", scenario.id));
            continue;
        };
        let fragment = normalized_anchor(fragment);
        let headings = document
            .lines()
            .filter_map(|line| line.strip_prefix('#'))
            .map(str::trim_start)
            .map(normalized_anchor)
            .collect::<std::collections::BTreeSet<_>>();
        if fragment.is_empty() || !headings.contains(&fragment) {
            failures.push(format!(
                "{} anchors missing heading {path}#{fragment}",
                scenario.id
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "scenario metadata contract failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_manifest_entry_has_an_executable_websocket_runner() {
    let manifest_ids = manifest()
        .into_iter()
        .map(|scenario| scenario.id)
        .collect::<std::collections::BTreeSet<_>>();
    let executable = executable_scenario_ids();
    let executable_ids = executable
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        executable.len(),
        executable_ids.len(),
        "an executable scenario ID is registered more than once"
    );
    assert_eq!(
        executable_ids, manifest_ids,
        "scenario records and executable WebSocket runners must remain one-to-one"
    );
}

#[test]
fn forgejo_and_nextest_files_keep_the_capped_execution_contract() {
    let quality = include_str!("../../../../.forgejo/workflows/quality.yaml");
    let nightly = include_str!("../../../../.forgejo/workflows/nightly-playtests.yaml");
    let coverage = include_str!("../../../../.forgejo/workflows/weekly-coverage.yaml");
    let nextest = include_str!("../../../../.config/nextest.toml");
    let testing = include_str!("../../../../docs/TESTING.md");
    let required_quality = [
        "workflow_dispatch:",
        "cancel-in-progress: true",
        "runs-on: cat-idler-heavy",
        "timeout-minutes: 150",
        "CAT_PLAYTEST_SEED_TIER: high-risk",
        "cargo nextest run --workspace --profile ci",
        "resources.txt",
        "runs-on: personal",
        "needs: quick",
        "timeout-minutes: 45",
        "Enforce browser transfer budget",
        "budget_bytes=\"$((12 * 1024 * 1024))\"",
        "target/playtest-traces/",
        "target/nextest/ci/",
        "GITHUB_STEP_SUMMARY",
    ];
    for needle in required_quality {
        assert!(quality.contains(needle), "quality workflow lost {needle:?}");
    }
    assert!(
        !quality.contains("--partition") && !quality.contains("nextest archive"),
        "complete execution must remain one dynamically scheduled Nextest run"
    );
    assert_eq!(
        quality
            .matches("cargo nextest run --workspace --profile ci")
            .count(),
        1,
        "the full gate must run one unpartitioned workspace inventory"
    );
    assert_eq!(
        quality.matches("timeout-minutes: 45").count(),
        2,
        "the quick and WASM jobs must retain their documented timeout"
    );
    for needle in [
        "45-minute timeout",
        "30 minutes",
        "120-minute per-test cap",
        "12 MiB gzip transfer ceiling",
    ] {
        assert!(testing.contains(needle), "testing docs lost {needle:?}");
    }
    for needle in [
        "workflow_dispatch:",
        "cron: \"30 8 * * *\"",
        "timeout-minutes: 180",
        "CAT_PLAYTEST_SEED_TIER: nightly",
        "runs-on: cat-idler-heavy",
        "cargo nextest run -p cat-server --profile nightly",
        "nightly-resources.txt",
        "target/nextest/nightly/",
        "target/playtest-traces/",
        "GITHUB_STEP_SUMMARY",
    ] {
        assert!(nightly.contains(needle), "nightly workflow lost {needle:?}");
    }
    for needle in [
        "workflow_dispatch:",
        "cron: \"30 10 * * 0\"",
        "timeout-minutes: 230",
        "runs-on: cat-idler-heavy",
        "coverage.json",
        "lcov.info",
        "coverage-html",
        "actual + 0.5",
        "steps.coverage-tests.outcome == 'success'",
        "resources.txt",
        "target/playtest-traces/",
        "GITHUB_STEP_SUMMARY",
    ] {
        assert!(
            coverage.contains(needle),
            "coverage workflow lost {needle:?}"
        );
    }
    for needle in [
        "[profile.ci]",
        "fail-fast = false",
        "test-threads = 2",
        "slow-timeout = { period = \"300s\", terminate-after = 6 }",
        "[profile.ci.junit]",
        "[profile.nightly]",
        "inherits = \"ci\"",
        "slow-timeout = { period = \"300s\", terminate-after = 24 }",
        "[profile.nightly.junit]",
        "test-group = 'singleton'",
    ] {
        assert!(nextest.contains(needle), "Nextest config lost {needle:?}");
    }
}
