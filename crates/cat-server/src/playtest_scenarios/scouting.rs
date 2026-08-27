//! Whole-game scouting journeys anchored in `docs/GAME_VISION.md` ("Knowledge must
//! come home") and `docs/IMPLEMENTATION_AUDIT.md` ("Shrine-return fog and resource
//! scouting").

use std::collections::BTreeSet;

use cat_protocol::{
    CatActivity, ClientAction, JobKind, JobStatus, ScoutMission, ScoutResource, TilePoint,
    WorldSnapshot,
};
use serde_json::Value;

use crate::playtest_harness::{
    FailureTrace, ObservedActionResult, SignedActor, WsClient, WsGameHarness, write_failure_trace,
};

use super::{Milestone, ScenarioSpec, SeedTier};

const SCOUT_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "signed-action-accepted",
        description: "the server accepts the authenticated DispatchScout action",
    },
    Milestone {
        id: "queued",
        description: "an Explore job with a named assignee is visible in the projected snapshot",
    },
    Milestone {
        id: "physical-outbound",
        description: "the assigned cat travels away from its pre-dispatch position",
    },
    Milestone {
        id: "provisional-fog",
        description: "the outbound scout exposes tentative tiles without committing them",
    },
    Milestone {
        id: "mid-search-restart",
        description: "save, restart, and reconnect preserve the in-flight notebook and job",
    },
    Milestone {
        id: "physical-return",
        description: "the same assigned cat visibly enters the Returning phase",
    },
    Milestone {
        id: "permanent-reveal",
        description: "shrine contact clears provisional fog and commits newly revealed tiles",
    },
    Milestone {
        id: "restart-persistence",
        description: "a second save, restart, and reconnect preserve delivered knowledge",
    },
];

const EXPLORE_OUTCOMES: &[&str] = &["new_lands"];
const WOOD_OUTCOMES: &[&str] = &["woodland", "no_woodland_yet"];
const FOOD_OUTCOMES: &[&str] = &["forage", "no_forage_yet"];
const WATER_OUTCOMES: &[&str] = &["water", "no_water_yet"];
const STONE_OUTCOMES: &[&str] = &["workable_stone", "no_workable_stone_yet"];
const PERSISTENCE_CHECKPOINTS: &[&str] = &["provisional-notebook", "committed-reveal"];

pub(crate) const SCENARIOS: &[ScenarioSpec] = &[
    ScenarioSpec {
        id: "scout-explore-shrine-return",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#shrine-return-fog-and-resource-scouting-2026-07-13 / Knowledge must come home",
        initial_setup: "fresh deterministic village with its exact founding reveal",
        action_or_trigger: "signed DispatchScout(Explore)",
        milestones: SCOUT_MILESTONES,
        // General exploration uses the production 30-minute bounded survey
        // deadline; leave five minutes for its physical shrine return.
        horizon_ms: 35 * 60_000,
        allowed_outcomes: EXPLORE_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: PERSISTENCE_CHECKPOINTS,
    },
    ScenarioSpec {
        id: "scout-wood-shrine-return",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#shrine-return-fog-and-resource-scouting-2026-07-13",
        initial_setup: "fresh deterministic village with hidden woodland beyond founding fog",
        action_or_trigger: "signed DispatchScout(Resource(Wood))",
        milestones: SCOUT_MILESTONES,
        horizon_ms: 240_000,
        allowed_outcomes: WOOD_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: PERSISTENCE_CHECKPOINTS,
    },
    ScenarioSpec {
        id: "scout-food-shrine-return",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#shrine-return-fog-and-resource-scouting-2026-07-13 / scout search semantics",
        initial_setup: "fresh deterministic village with hidden forage beyond founding fog",
        action_or_trigger: "signed DispatchScout(Resource(Food))",
        milestones: SCOUT_MILESTONES,
        horizon_ms: 240_000,
        allowed_outcomes: FOOD_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: PERSISTENCE_CHECKPOINTS,
    },
    ScenarioSpec {
        id: "scout-water-shrine-return",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#shrine-return-fog-and-resource-scouting-2026-07-13 / scout search semantics",
        initial_setup: "fresh deterministic village with hidden water beyond founding fog",
        action_or_trigger: "signed DispatchScout(Resource(Water))",
        milestones: SCOUT_MILESTONES,
        horizon_ms: 240_000,
        allowed_outcomes: WATER_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: PERSISTENCE_CHECKPOINTS,
    },
    ScenarioSpec {
        id: "scout-stone-shrine-return",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#shrine-return-fog-and-resource-scouting-2026-07-13 / scout search semantics",
        initial_setup: "fresh deterministic village with hidden workable stone beyond founding fog",
        action_or_trigger: "signed DispatchScout(Resource(Stone))",
        milestones: SCOUT_MILESTONES,
        horizon_ms: 240_000,
        allowed_outcomes: STONE_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: PERSISTENCE_CHECKPOINTS,
    },
];

pub(crate) const EXECUTABLE_SCENARIO_IDS: &[&str] = &[
    "scout-explore-shrine-return",
    "scout-wood-shrine-return",
    "scout-food-shrine-return",
    "scout-water-shrine-return",
    "scout-stone-shrine-return",
];

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScoutScenarioCase {
    pub(crate) spec: &'static ScenarioSpec,
    pub(crate) mission: ScoutMission,
}

pub(crate) const CASES: &[ScoutScenarioCase] = &[
    ScoutScenarioCase {
        spec: &SCENARIOS[0],
        mission: ScoutMission::Explore,
    },
    ScoutScenarioCase {
        spec: &SCENARIOS[1],
        mission: ScoutMission::Resource(ScoutResource::Wood),
    },
    ScoutScenarioCase {
        spec: &SCENARIOS[2],
        mission: ScoutMission::Resource(ScoutResource::Food),
    },
    ScoutScenarioCase {
        spec: &SCENARIOS[3],
        mission: ScoutMission::Resource(ScoutResource::Water),
    },
    ScoutScenarioCase {
        spec: &SCENARIOS[4],
        mission: ScoutMission::Resource(ScoutResource::Stone),
    },
];

#[derive(Debug)]
pub(crate) struct ScoutScenarioFailure {
    pub(crate) scenario_id: &'static str,
    pub(crate) seed: u32,
    pub(crate) last_completed_milestone: Option<&'static str>,
    pub(crate) simulated_ms: i64,
    pub(crate) reason: String,
    pub(crate) action_results: Vec<ObservedActionResult>,
    /// The complete projected wire snapshot supplies active jobs/cats, reported
    /// inventory, fog counts, and recent events without a privileged state read.
    pub(crate) last_snapshot: WorldSnapshot,
    pub(crate) restart_difference: Option<Value>,
}

impl ScoutScenarioFailure {
    fn write_trace(&self) -> Result<(), String> {
        write_failure_trace(&FailureTrace {
            scenario_id: self.scenario_id,
            seed: self.seed,
            last_completed_milestone: self.last_completed_milestone,
            simulated_time_ms: self.simulated_ms,
            action_results: &self.action_results,
            snapshot: &self.last_snapshot,
            restart_difference: self.restart_difference.as_ref(),
            failure: &self.reason,
        })
        .map(|_| ())
    }
}

/// Execute one bounded, ordered scout journey. Restart checkpoints happen only
/// after an observable milestone, so no sleep or exact-tick assumption is needed.
pub(crate) async fn run_scout_scenario(
    harness: &mut WsGameHarness,
    mut client: WsClient,
    actor: &SignedActor,
    case: ScoutScenarioCase,
    seed: u32,
) -> Result<(), ScoutScenarioFailure> {
    let baseline = client.snapshot().clone();
    let baseline_colony = selected_colony(&baseline);
    let baseline_revealed = tile_set(&baseline_colony.revealed_tiles);
    let baseline_jobs = baseline_colony
        .jobs
        .iter()
        .map(|job| job.id.as_str())
        .collect::<BTreeSet<_>>();
    let baseline_now = baseline.now;
    let mut action_results = client.action_results.clone();
    let mut simulated_ms = 0;
    let mut last_completed = None;
    let mut restart_difference = None;
    let mut last_snapshot = baseline.clone();

    macro_rules! fail {
        ($reason:expr) => {
            return Err(ScoutScenarioFailure {
                scenario_id: case.spec.id,
                seed,
                last_completed_milestone: last_completed,
                simulated_ms,
                reason: $reason,
                action_results,
                last_snapshot,
                restart_difference,
            })
        };
    }

    let observed = match client
        .send_action(&ClientAction::DispatchScout {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            mission: case.mission,
        })
        .await
    {
        Ok(result) => result,
        Err(error) => fail!(format!("dispatch transport failed: {error}")),
    };
    action_results.push(observed.clone());
    if !observed.result.ok {
        fail!(format!(
            "signed dispatch rejected: {:?}",
            observed.result.message
        ));
    }
    last_completed = Some("signed-action-accepted");

    let queued = match client.receive_snapshot().await {
        Ok(snapshot) => snapshot,
        Err(error) => fail!(format!(
            "accepted dispatch produced no projected snapshot: {error}"
        )),
    };
    last_snapshot = queued.clone();
    let queued_colony = selected_colony(&queued);
    let Some(job) = queued_colony.jobs.iter().find(|job| {
        job.kind == JobKind::Explore
            && !baseline_jobs.contains(job.id.as_str())
            && job.status == JobStatus::Queued
            && job.assigned_cat_name.is_some()
    }) else {
        fail!("accepted dispatch did not project a newly queued, assigned Explore job".to_owned());
    };
    let job_id = job.id.clone();
    let scout_name = job
        .assigned_cat_name
        .clone()
        .expect("matched an assigned scout");
    let Some(start_position) = baseline_colony
        .cats
        .iter()
        .find(|cat| cat.name == scout_name)
        .map(|cat| cat.position)
    else {
        fail!(format!(
            "assigned scout {scout_name:?} was absent from baseline"
        ));
    };
    last_completed = Some("queued");

    let mut saw_outbound = false;
    let mut saw_provisional = false;
    let mut restarted_in_flight = false;
    let mut saw_return = false;
    let mut delivered_snapshot = None;

    while simulated_ms < case.spec.horizon_ms {
        let snapshot = match harness.advance_by(&mut client, 1_000).await {
            Ok(snapshot) => snapshot,
            Err(error) => fail!(format!("deterministic tick failed: {error}")),
        };
        simulated_ms += 1_000;
        last_snapshot = snapshot.clone();
        let colony = selected_colony(&snapshot);
        let scout = colony.cats.iter().find(|cat| cat.name == scout_name);

        if !saw_outbound
            && scout.is_some_and(|cat| {
                cat.activity == CatActivity::Traveling && cat.position != start_position
            })
        {
            saw_outbound = true;
            last_completed = Some("physical-outbound");
        }

        if saw_outbound && !saw_provisional && !colony.provisional_tiles.is_empty() {
            let permanent = tile_set(&colony.revealed_tiles);
            if colony
                .provisional_tiles
                .iter()
                .any(|tile| permanent.contains(&(tile.x, tile.y)))
            {
                fail!("provisional fog leaked into the permanent reveal before return".to_owned());
            }
            saw_provisional = true;
            last_completed = Some("provisional-fog");
        }

        if saw_provisional && !restarted_in_flight {
            let provisional_before = tile_set(&colony.provisional_tiles);
            let restart_baseline = last_snapshot.clone();
            client = match harness.restart_and_reconnect(client, actor).await {
                Ok(client) => client,
                Err(error) => {
                    last_snapshot = restart_baseline;
                    fail!(format!("mid-search restart failed: {error}"));
                }
            };
            action_results.extend(client.action_results.clone());
            let restart = client.snapshot().clone();
            last_snapshot = restart.clone();
            let restarted = selected_colony(&restart);
            let job_survived = restarted.jobs.iter().any(|job| job.id == job_id);
            let notebook_survived = tile_set(&restarted.provisional_tiles) == provisional_before;
            if !job_survived || !notebook_survived {
                restart_difference = Some(serde_json::json!({
                    "checkpoint": "provisional-notebook",
                    "jobId": job_id,
                    "jobPresent": job_survived,
                    "expectedProvisional": provisional_before.len(),
                    "actualProvisional": restarted.provisional_tiles.len(),
                }));
                fail!("in-flight scout state changed across restart/reconnect".to_owned());
            }
            restarted_in_flight = true;
            last_completed = Some("mid-search-restart");
        }

        if restarted_in_flight
            && !saw_return
            && scout.is_some_and(|cat| cat.activity == CatActivity::Returning)
        {
            saw_return = true;
            last_completed = Some("physical-return");
        }

        if saw_return {
            // Events are newest-first and capped, so an index offset would discard
            // the newly prepended entries as the buffer rotates.
            let new_events = colony
                .events
                .iter()
                .filter(|event| event.timestamp > baseline_now);
            let allowed = new_events
                .filter(|event| event.kind == "discovery")
                .filter_map(|event| classify_outcome(&event.message))
                // A fresh village can have a concurrent Leader-dispatched wood
                // scout. Do not let that newer, unrelated discovery mask the
                // explicitly dispatched mission's allowed outcome.
                .find(|outcome| case.spec.allowed_outcomes.contains(outcome));
            let revealed = tile_set(&colony.revealed_tiles);
            if allowed.is_some()
                && colony.provisional_tiles.is_empty()
                && revealed.len() > baseline_revealed.len()
            {
                delivered_snapshot = Some(snapshot);
                last_completed = Some("permanent-reveal");
                break;
            }
        }
    }

    let Some(delivered) = delivered_snapshot else {
        fail!(format!(
            "bounded horizon elapsed (outbound={saw_outbound}, provisional={saw_provisional}, in-flight restart={restarted_in_flight}, return={saw_return})"
        ));
    };
    let delivered_colony = selected_colony(&delivered);
    let delivered_reveal = tile_set(&delivered_colony.revealed_tiles);
    let delivered_events = delivered_colony
        .events
        .iter()
        .map(|event| (&event.kind, &event.message, event.timestamp))
        .collect::<Vec<_>>();
    let restart_baseline = last_snapshot.clone();
    client = match harness.restart_and_reconnect(client, actor).await {
        Ok(client) => client,
        Err(error) => {
            last_snapshot = restart_baseline;
            fail!(format!("post-delivery restart failed: {error}"));
        }
    };
    action_results.extend(client.action_results.clone());
    let restart = client.snapshot().clone();
    last_snapshot = restart.clone();
    let restarted = selected_colony(&restart);
    let restarted_events = restarted
        .events
        .iter()
        .map(|event| (&event.kind, &event.message, event.timestamp))
        .collect::<Vec<_>>();
    if tile_set(&restarted.revealed_tiles) != delivered_reveal
        || !restarted.provisional_tiles.is_empty()
        || restarted_events != delivered_events
    {
        restart_difference = Some(serde_json::json!({
            "checkpoint": "committed-reveal",
            "expectedReveal": delivered_reveal.len(),
            "actualReveal": restarted.revealed_tiles.len(),
            "actualProvisional": restarted.provisional_tiles.len(),
            "eventsEqual": restarted_events == delivered_events,
        }));
        fail!("delivered knowledge changed across restart/reconnect".to_owned());
    }
    last_completed = Some("restart-persistence");
    let _ = last_completed;
    Ok(())
}

fn selected_colony(snapshot: &WorldSnapshot) -> &cat_protocol::ColonySnapshot {
    let selected = snapshot.selected_colony_id.as_deref();
    snapshot
        .colonies
        .iter()
        .find(|colony| selected == Some(colony.id.as_str()))
        .or_else(|| snapshot.colonies.first())
        .expect("playtest WebSocket snapshot must contain the selected village")
}

fn tile_set(tiles: &[TilePoint]) -> BTreeSet<(i32, i32)> {
    tiles.iter().map(|tile| (tile.x, tile.y)).collect()
}

fn classify_outcome(message: &str) -> Option<&'static str> {
    [
        ("no workable stone yet", "no_workable_stone_yet"),
        ("no woodland yet", "no_woodland_yet"),
        ("no forage yet", "no_forage_yet"),
        ("no water yet", "no_water_yet"),
        ("workable stone", "workable_stone"),
        ("woodland", "woodland"),
        ("forage", "forage"),
        ("water", "water"),
        ("new lands", "new_lands"),
    ]
    .into_iter()
    .find_map(|(needle, outcome)| message.contains(needle).then_some(outcome))
}

async fn run_requested_seed_tier(case: ScoutScenarioCase) {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        let mut harness = match WsGameHarness::start(seed).await {
            Ok(harness) => harness,
            Err(error) => {
                failures.push(format!(
                    "{} seed {seed}: harness start: {error}",
                    case.spec.id
                ));
                continue;
            }
        };
        let session = format!("{}-{seed}", case.spec.id);
        let (client, actor) = match harness
            .connect_authenticated(session, "Deterministic Scout")
            .await
        {
            Ok(connection) => connection,
            Err(error) => {
                failures.push(format!(
                    "{} seed {seed}: authentication: {error}",
                    case.spec.id
                ));
                continue;
            }
        };

        if let Err(failure) = run_scout_scenario(&mut harness, client, &actor, case, seed).await {
            let trace = failure.write_trace();
            failures.push(format!(
                "{} seed {seed}, after {:?} at {}ms: {}; trace={trace:?}",
                failure.scenario_id,
                failure.last_completed_milestone,
                failure.simulated_ms,
                failure.reason,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "scouting journey failures for {}:\n{}",
        case.spec.id,
        failures.join("\n")
    );
}

macro_rules! scouting_journey_tests {
    ($(($test_name:ident, $scenario_id:literal)),+ $(,)?) => {
        const GENERATED_SCOUTING_SCENARIO_IDS: &[&str] = &[$($scenario_id),+];

        $(
            #[tokio::test]
            async fn $test_name() {
                let case = CASES
                    .iter()
                    .copied()
                    .find(|case| case.spec.id == $scenario_id)
                    .expect("named scouting journey must have an executable case");
                run_requested_seed_tier(case).await;
            }
        )+
    };
}

scouting_journey_tests!(
    (
        real_websocket_scout_explore_shrine_return,
        "scout-explore-shrine-return"
    ),
    (
        real_websocket_scout_wood_shrine_return,
        "scout-wood-shrine-return"
    ),
    (
        real_websocket_scout_food_shrine_return,
        "scout-food-shrine-return"
    ),
    (
        real_websocket_scout_water_shrine_return,
        "scout-water-shrine-return"
    ),
    (
        real_websocket_scout_stone_shrine_return,
        "scout-stone-shrine-return"
    ),
);

#[test]
fn scouting_manifest_covers_every_mission_and_outcome_family() {
    assert_eq!(GENERATED_SCOUTING_SCENARIO_IDS, EXECUTABLE_SCENARIO_IDS);
    assert_eq!(
        SCENARIOS.len(),
        5,
        "Explore plus all four resource missions"
    );
    assert!(SCENARIOS.iter().all(|scenario| {
        scenario
            .milestones
            .iter()
            .map(|milestone| milestone.id)
            .eq(SCOUT_MILESTONES.iter().map(|milestone| milestone.id))
    }));
    assert_eq!(SCENARIOS[0].allowed_outcomes, EXPLORE_OUTCOMES);
    assert_eq!(CASES.len(), SCENARIOS.len());
    assert_eq!(CASES[0].mission, ScoutMission::Explore);
    assert_eq!(
        CASES[1..]
            .iter()
            .map(|case| case.mission)
            .collect::<Vec<_>>(),
        vec![
            ScoutMission::Resource(ScoutResource::Wood),
            ScoutMission::Resource(ScoutResource::Food),
            ScoutMission::Resource(ScoutResource::Water),
            ScoutMission::Resource(ScoutResource::Stone),
        ]
    );
    for scenario in &SCENARIOS[1..] {
        assert_eq!(scenario.allowed_outcomes.len(), 2);
        assert!(
            scenario
                .allowed_outcomes
                .iter()
                .any(|outcome| outcome.starts_with("no_")),
            "{} must explicitly allow a bounded exhausted result",
            scenario.id
        );
    }
    for outcome in [
        "new lands",
        "woodland",
        "no woodland yet",
        "forage",
        "no forage yet",
        "water",
        "no water yet",
        "workable stone",
        "no workable stone yet",
    ] {
        assert!(
            classify_outcome(&format!("A scout returned with news of {outcome}.")).is_some(),
            "unclassified discovery outcome: {outcome}"
        );
    }
}
