//! Executable real-WebSocket journeys for the broad system scenario manifest.
//!
//! Fixture construction is deliberately confined to `WsGameHarness::start_with`.
//! Once the listener starts, every mutation and observation goes through signed
//! protocol actions, deterministic authoritative ticks, projected snapshots, and
//! restart/reconnect.

use std::collections::{BTreeMap, BTreeSet};

use cat_protocol::{
    BuildingType as ProtocolBuildingType, ClientAction, JobKind, OfferingResource,
    OfficerRole as ProtocolOfficerRole, ResourceKind as ProtoResourceKind, ScoutMission, TilePoint,
    TransportMode as ProtocolTransportMode, WorldSnapshot,
};
use cat_sim::{
    biomes::MaxResources,
    entities::{Position, Resources},
    items::{Item, ItemKind, Material},
    officers::{OfficerRole, prerequisite_for},
    stockpiles::{
        GatherSpot, GatherSpotPurpose, MAX_DESIGNATED_STOCKPILES, MAX_GATHER_SPOTS, ResourceKind,
        Stockpile,
    },
    trader::{self, TraderState},
    types::TileType,
    village_area::{from_tiles, gate_placement_default, side_delta},
    village_layout::GridPos,
    world_gen::TileResources,
    world_tick::{
        BuildingRuntime, ElectionKind, ElectionRuntime, RaiderRuntime, TilePos, TraderRuntime,
        VillageKind, VillageTradeOffer, WorldState, WorldTileRuntime, found_colony_at,
        reconcile_colony_stockpiles, register_colony_spatial,
    },
    zones::ZoneRect,
};
use serde_json::{Value, json};

use crate::playtest_harness::{
    FailureTrace, ObservedActionResult, SignedActor, WsClient, WsGameHarness, write_failure_trace,
};

pub(crate) const EXECUTABLE_SCENARIO_IDS: &[&str] = &[
    "fresh-world-survival-and-needs",
    "housing-breeding-migration-aging-extinction",
    "all-officers-vacant-and-assigned",
    "research-blessings-and-shrine-work",
    "elections-voting-and-vote-kick",
    "raids-training-and-defense",
    "stockpiles-gather-spots-and-hauling",
    "roads-bridges-rail-and-shipping",
    "traders-and-village-trade",
    "multi-village-selection-and-restart",
    "shrine-demand-ritual-lifecycle",
];

const START_MS: i64 = 1_700_000_000_000;

#[derive(Debug)]
struct JourneyFailure {
    scenario_id: &'static str,
    seed: u32,
    milestone: Option<&'static str>,
    simulated_ms: i64,
    reason: String,
    actions: Vec<ObservedActionResult>,
    snapshot: WorldSnapshot,
    restart_difference: Option<Value>,
}

impl JourneyFailure {
    fn write_trace(&self) -> Result<(), String> {
        write_failure_trace(&FailureTrace {
            scenario_id: self.scenario_id,
            seed: self.seed,
            last_completed_milestone: self.milestone,
            simulated_time_ms: self.simulated_ms,
            action_results: &self.actions,
            snapshot: &self.snapshot,
            restart_difference: self.restart_difference.as_ref(),
            failure: &self.reason,
        })
        .map(|_| ())
    }
}

struct Journey {
    id: &'static str,
    harness: WsGameHarness,
    client: Option<WsClient>,
    actor: SignedActor,
    seed: u32,
    milestone: Option<&'static str>,
    simulated_ms: i64,
    actions: Vec<ObservedActionResult>,
    last_snapshot: WorldSnapshot,
    restart_difference: Option<Value>,
}

impl Journey {
    async fn start(
        id: &'static str,
        seed: u32,
        setup: impl FnOnce(&mut WorldState),
    ) -> Result<Self, String> {
        let mut harness = WsGameHarness::start_with(seed, setup).await?;
        let (client, actor) = harness
            .connect_authenticated(format!("system-{id}-{seed}"), "System Playtester")
            .await?;
        let last_snapshot = client.snapshot().clone();
        let actions = client.action_results.clone();
        Ok(Self {
            id,
            harness,
            client: Some(client),
            actor,
            seed,
            milestone: None,
            simulated_ms: 0,
            actions,
            last_snapshot,
            restart_difference: None,
        })
    }

    fn signed(&self, make: impl FnOnce(&SignedActor) -> ClientAction) -> ClientAction {
        make(&self.actor)
    }

    async fn send(&mut self, action: ClientAction) -> Result<WorldSnapshot, JourneyFailure> {
        let observed = self
            .client
            .as_mut()
            .expect("journey client")
            .send_action(&action)
            .await
            .map_err(|error| {
                self.failure(format!(
                    "WebSocket action transport failed for {action:?}: {error}"
                ))
            })?;
        self.actions.push(observed.clone());
        if !observed.result.ok {
            return Err(self.failure(format!(
                "signed action rejected for {action:?}: {:?}",
                observed.result.message
            )));
        }
        self.milestone = Some("signed-control");
        let snapshot = self
            .client
            .as_mut()
            .expect("journey client")
            .receive_snapshot()
            .await
            .map_err(|error| {
                self.failure(format!(
                    "accepted action emitted no projected snapshot: {error}"
                ))
            })?;
        self.last_snapshot = snapshot.clone();
        Ok(snapshot)
    }

    async fn send_rejected(
        &mut self,
        action: ClientAction,
        expected_message: &str,
    ) -> Result<(), JourneyFailure> {
        let observed = self
            .client
            .as_mut()
            .expect("journey client")
            .send_action(&action)
            .await
            .map_err(|error| {
                self.failure(format!(
                    "WebSocket rejected-action transport failed for {action:?}: {error}"
                ))
            })?;
        self.actions.push(observed.clone());
        if observed.result.ok {
            return Err(self.failure(format!(
                "boundary action unexpectedly succeeded: {action:?}"
            )));
        }
        let message = observed.result.message.as_deref().unwrap_or_default();
        if !message.contains(expected_message) {
            return Err(self.failure(format!(
                "boundary action {action:?} rejected with {message:?}, expected fragment {expected_message:?}"
            )));
        }
        self.milestone = Some("rejected-boundary");
        Ok(())
    }

    async fn advance(&mut self, delta_ms: i64) -> Result<WorldSnapshot, JourneyFailure> {
        let snapshot = self
            .harness
            .advance_by(self.client.as_mut().expect("journey client"), delta_ms)
            .await
            .map_err(|error| self.failure(format!("authoritative tick failed: {error}")))?;
        self.simulated_ms += delta_ms;
        self.last_snapshot = snapshot.clone();
        Ok(snapshot)
    }

    async fn eventually(
        &mut self,
        horizon_ms: i64,
        cadence_ms: i64,
        mut predicate: impl FnMut(&WorldSnapshot) -> bool,
    ) -> Result<WorldSnapshot, JourneyFailure> {
        if predicate(&self.last_snapshot) {
            return Ok(self.last_snapshot.clone());
        }
        let started = self.simulated_ms;
        while self.simulated_ms - started < horizon_ms {
            let snapshot = self.advance(cadence_ms).await?;
            if predicate(&snapshot) {
                self.milestone = Some("physical-effect");
                return Ok(snapshot);
            }
        }
        Err(self.failure(format!(
            "physical milestone not reached within {horizon_ms}ms"
        )))
    }

    async fn restart_with_fingerprint(
        &mut self,
        fingerprint: impl Fn(&WorldSnapshot) -> Value,
    ) -> Result<(), JourneyFailure> {
        let before = fingerprint(&self.last_snapshot);
        let client = self.client.take().expect("journey client");
        self.client = Some(
            self.harness
                .restart_and_reconnect(client, &self.actor)
                .await
                .map_err(|error| self.failure(format!("restart/reconnect failed: {error}")))?,
        );
        let client = self.client.as_ref().expect("reconnected journey client");
        self.actions.extend(client.action_results.clone());
        self.last_snapshot = client.snapshot().clone();
        let after = fingerprint(&self.last_snapshot);
        if before != after {
            self.restart_difference = Some(json!({ "before": before, "after": after }));
            return Err(self.failure("projected checkpoint changed across restart".to_owned()));
        }
        self.milestone = Some("restart-equality");
        Ok(())
    }

    fn failure(&self, reason: String) -> JourneyFailure {
        JourneyFailure {
            scenario_id: self.id,
            seed: self.seed,
            milestone: self.milestone,
            simulated_ms: self.simulated_ms,
            reason,
            actions: self.actions.clone(),
            snapshot: self.last_snapshot.clone(),
            restart_difference: self.restart_difference.clone(),
        }
    }
}

#[allow(clippy::result_large_err)]
fn aggregate_results(
    family: &str,
    results: Vec<(&'static str, Result<(), JourneyFailure>)>,
) -> Result<(), JourneyFailure> {
    let mut failures = results
        .into_iter()
        .filter_map(|(name, result)| {
            result.err().map(|mut failure| {
                failure.reason = format!("{name}: {}", failure.reason);
                failure
            })
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return Ok(());
    }
    let reasons = failures
        .iter()
        .map(|failure| failure.reason.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    let mut failure = failures.remove(0);
    failure.reason = format!("independent {family} subjourneys: {reasons}");
    Err(failure)
}

fn common_setup(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.resources.food = 1_000.0;
    colony.resources.water = 1_000.0;
    colony.resources.materials = 1_000.0;
    colony.resources.refined = 1_000.0;
    colony.resources.metal = 1_000.0;
    colony.resources.lumber = 1_000.0;
    colony.resources.blocks = 1_000.0;
    colony.upgrade_tree.research_points = 10_000.0;
    colony.global_upgrade_points = 10_000.0;
    reconcile_colony_stockpiles(colony);
}

fn housing_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    let mate = colony.cats[1].id.clone();
    colony.cats[0].is_pregnant = true;
    colony.cats[0].pregnancy_due_age_hours = Some(colony.cats[0].age_hours + 0.01);
    colony.cats[0].pregnancy_due_time = Some(START_MS + 60_000);
    colony.cats[0].pregnancy_mate_id = Some(mate);
    colony.cats.last_mut().expect("elder fixture").age_hours = 10_000.0;
    let anchor = colony.anchor;
    colony.buildings.push(BuildingRuntime {
        id: "system-vacancy-den".to_owned(),
        building_type: cat_sim::types::BuildingType::Den,
        position: TilePos {
            x: anchor.x + 30,
            y: anchor.y + 30,
        },
        is_complete: true,
        construction_progress: 100,
        ..BuildingRuntime::default()
    });
}

fn conception_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    colony.run_started_at = START_MS - 36 * 60 * 60_000;
    colony.resources.blessings = 100.0;
    let anchor = colony.anchor;
    for index in 0..3 {
        colony.buildings.push(BuildingRuntime {
            id: format!("system-conception-den-{index}"),
            building_type: cat_sim::types::BuildingType::Den,
            position: TilePos {
                x: anchor.x + 24 + index * 4,
                y: anchor.y + 24,
            },
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        });
    }
    provision_lifecycle_reserves(colony);
}

fn migration_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    colony.run_started_at = START_MS - 30 * 60 * 60_000;
    colony.cats.truncate(15);
}

fn provision_lifecycle_reserves(colony: &mut cat_sim::world_tick::ColonyRuntime) {
    let anchor = colony.anchor;
    for index in 0..8 {
        for (kind, suffix) in [
            (cat_sim::types::BuildingType::FoodStorage, "food"),
            (cat_sim::types::BuildingType::WaterBowl, "water"),
        ] {
            colony.buildings.push(BuildingRuntime {
                id: format!("system-lifecycle-{suffix}-{index}"),
                building_type: kind,
                position: TilePos {
                    x: anchor.x + 80 + index * 4,
                    y: anchor.y + if suffix == "food" { 80 } else { 84 },
                },
                is_complete: true,
                construction_progress: 100,
                ..BuildingRuntime::default()
            });
        }
    }
    colony.resources.food = 1_000.0;
    colony.resources.water = 1_000.0;
    colony.resources.materials = 1_000.0;
    reconcile_colony_stockpiles(colony);
    if let Some(store) = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.id == "stockpile-storehouse")
    {
        store.contents.food = 500.0;
        store.contents.water = 500.0;
        store.contents.materials = 500.0;
    }
    colony.resources.food = 500.0;
    colony.resources.water = 500.0;
    colony.resources.materials = 500.0;
}

fn extinction_setup(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.resources = Resources::default();
    for pile in &mut colony.stockpiles {
        pile.contents = Resources::default();
    }
    for cat in &mut colony.cats {
        cat.needs.hunger = 0.0;
        cat.needs.thirst = 0.0;
        cat.needs.health = 100.0;
    }
    reconcile_colony_stockpiles(colony);
}

fn selected(snapshot: &WorldSnapshot) -> &cat_protocol::ColonySnapshot {
    let id = snapshot.selected_colony_id.as_deref();
    snapshot
        .colonies
        .iter()
        .find(|colony| Some(colony.id.as_str()) == id)
        .or_else(|| snapshot.colonies.first())
        .expect("system journey snapshot has a village")
}

async fn fresh_survival_needs(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[0], seed, |_| {})
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[0], seed, error))?;
    let baseline_population = selected(&journey.last_snapshot).housing.population;
    let baseline_cat = selected(&journey.last_snapshot).cats[0].clone();
    let baseline_food = selected(&journey.last_snapshot).resources.food;
    let action = journey.signed(|actor| ClientAction::RequestJob {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        kind: JobKind::SupplyFood,
    });
    journey.send(action).await?;
    journey
        .eventually(6 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            colony.housing.population == baseline_population
                && colony.cats.iter().all(|cat| cat.death_time.is_none())
                && colony
                    .cats
                    .iter()
                    .find(|cat| cat.id == baseline_cat.id)
                    .is_some_and(|cat| {
                        cat.needs.hunger != baseline_cat.needs.hunger
                            || cat.needs.thirst != baseline_cat.needs.thirst
                            || cat.needs.rest != baseline_cat.needs.rest
                    })
                && (colony.resources.food != baseline_food
                    || colony.events.iter().any(|event| {
                        event.kind == "job_completed"
                            && event.message.to_ascii_lowercase().contains("food")
                    }))
                && colony.cats.iter().any(|cat| {
                    cat.activity != cat_protocol::CatActivity::Idle || cat.current_task.is_some()
                })
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "population": colony.housing.population,
                "cats": colony.cats.iter().map(|cat| (&cat.id, cat.death_time)).collect::<Vec<_>>(),
                "jobs": colony.jobs.iter().map(|job| (&job.id, job.kind, job.status)).collect::<Vec<_>>(),
            })
        })
        .await
}

fn fresh_water_setup(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.resources.water = 10.0;
    for pile in &mut colony.stockpiles {
        pile.contents.water = 0.0;
    }
    reconcile_colony_stockpiles(colony);
    if let Some(store) = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.id == "stockpile-storehouse")
    {
        store.contents.water = 10.0;
    }
    colony.resources.water = 10.0;
}

async fn fresh_water_emergency(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[0], seed, fresh_water_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[0], seed, error))?;
    let baseline_population = selected(&journey.last_snapshot).housing.population;
    let baseline_water = selected(&journey.last_snapshot).resources.water;
    let action = journey.signed(|actor| ClientAction::RequestJob {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        kind: JobKind::FetchWater,
    });
    journey.send(action).await?;
    let mut saw_fetch = false;
    let mut saw_carry = false;
    let mut saw_deposit = false;
    let mut previous_water = baseline_water;
    let result = journey
        .eventually(50 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            saw_fetch |= colony
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::FetchWater);
            saw_carry |= colony.cats.iter().any(|cat| {
                cat.carrying
                    .as_ref()
                    .is_some_and(|cargo| cargo.kind == cat_protocol::CarryingKind::Water)
            });
            // A vacant Accountant deliberately keeps the numeric stock report stale.
            // The authoritative public deposit proof is therefore the completed fetch
            // event after a visible Water cargo has disappeared at the village.
            saw_deposit |= saw_carry
                && colony.events.iter().any(|event| {
                    event.kind == "job_completed"
                        && event.message.to_ascii_lowercase().contains("water")
                })
                && colony.cats.iter().all(|cat| {
                    cat.carrying
                        .as_ref()
                        .is_none_or(|cargo| cargo.kind != cat_protocol::CarryingKind::Water)
                });
            previous_water = colony.resources.water;
            saw_fetch
                && saw_carry
                && saw_deposit
                && colony.housing.population == baseline_population
                && colony.cats.iter().all(|cat| cat.death_time.is_none())
        })
        .await;
    if let Err(mut failure) = result {
        failure.reason = format!(
            "emergency water lifecycle flags: fetch={saw_fetch}, carry={saw_carry}, deposit={saw_deposit}, baseline={baseline_water}, final={previous_water}; {}",
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "water": colony.resources.water,
                "population": colony.housing.population,
                "fetches": colony.jobs.iter().filter(|job| job.kind == JobKind::FetchWater).collect::<Vec<_>>(),
            })
        })
        .await
}

fn guidance_setup(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.test_time_scale = 100.0;
    colony.test_resource_decay_multiplier = 20.0;
    colony.test_resilience_hours_override = Some(0.0);
    colony.test_critical_ms_override = 10_000;
}

const MIN_VISIBLE_AVERAGE_NEED_DEFICIT: f64 = 0.5;

#[derive(Debug, Clone, Copy, Default)]
struct GuidanceOutcome {
    reported_stores: f64,
    report_age_ms: i64,
    population: u32,
    deaths: usize,
    survival_death_events: usize,
    migration_departures: u64,
    average_food_water_deficit: f64,
    lowest_hunger: f64,
    lowest_thirst: f64,
    lowest_health: f64,
    revealed: usize,
    resets: usize,
}

fn poor_guidance_has_visible_survival_cost(guided: GuidanceOutcome, poor: GuidanceOutcome) -> bool {
    let reports_are_current = guided.report_age_ms <= cat_sim::ledger::ACCOUNTING_ROUND_INTERVAL_MS
        && poor.report_age_ms <= cat_sim::ledger::ACCOUNTING_ROUND_INTERVAL_MS;
    poor.survival_death_events > guided.survival_death_events
        || poor.deaths > guided.deaths
        || poor.migration_departures > guided.migration_departures
        || poor.population < guided.population
        || poor.average_food_water_deficit
            > guided.average_food_water_deficit + MIN_VISIBLE_AVERAGE_NEED_DEFICIT
        || (reports_are_current && poor.reported_stores + 1.0 < guided.reported_stores)
}

#[test]
fn stale_accountant_totals_cannot_prove_a_poor_guidance_cost() {
    let guided = GuidanceOutcome {
        reported_stores: 300.0,
        report_age_ms: 600_000,
        population: 30,
        ..GuidanceOutcome::default()
    };
    let stale_poor = GuidanceOutcome {
        reported_stores: 0.0,
        ..guided
    };
    assert!(
        !poor_guidance_has_visible_survival_cost(guided, stale_poor),
        "an unstaffed Accountant's old totals cannot establish a current economic outcome"
    );

    let current_guided = GuidanceOutcome {
        report_age_ms: 30_000,
        ..guided
    };
    let current_poor = GuidanceOutcome {
        reported_stores: 250.0,
        ..current_guided
    };
    assert!(poor_guidance_has_visible_survival_cost(
        current_guided,
        current_poor
    ));

    let noisy_single_cat_minimum = GuidanceOutcome {
        lowest_hunger: current_guided.lowest_hunger - 2.0,
        lowest_thirst: current_guided.lowest_thirst - 2.0,
        lowest_health: current_guided.lowest_health - 2.0,
        ..current_guided
    };
    assert!(
        !poor_guidance_has_visible_survival_cost(current_guided, noisy_single_cat_minimum),
        "single-cat minima are diagnostics, not an independently sufficient outcome"
    );
}

async fn run_signed_guidance(
    seed: u32,
    poor: bool,
) -> Result<(Journey, GuidanceOutcome), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[0], seed, guidance_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[0], seed, error))?;
    for _ in 0..120 {
        let targets: &[(JobKind, usize)] = if poor {
            &[(JobKind::Explore, 1), (JobKind::Quarry, 8)]
        } else {
            &[
                (JobKind::HuntExpedition, 6),
                (JobKind::FetchWater, 3),
                (JobKind::Quarry, 1),
            ]
        };
        for &(kind, target) in targets {
            loop {
                let in_flight = selected(&journey.last_snapshot)
                    .jobs
                    .iter()
                    .filter(|job| {
                        job.kind == kind
                            && matches!(
                                job.status,
                                cat_protocol::JobStatus::Queued | cat_protocol::JobStatus::Active
                            )
                    })
                    .count();
                if in_flight >= target {
                    break;
                }
                let action = journey.signed(|actor| ClientAction::RequestJob {
                    session_id: actor.session_id.clone(),
                    nickname: actor.nickname.clone(),
                    sig: actor.sig.clone(),
                    kind,
                });
                journey.send(action).await?;
            }
        }
        journey.advance(5_000).await?;
    }
    let now = journey.last_snapshot.now;
    let colony = selected(&journey.last_snapshot);
    let living = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .collect::<Vec<_>>();
    let living_count = living.len().max(1) as f64;
    let outcome = GuidanceOutcome {
        reported_stores: colony.resources.food + colony.resources.water,
        report_age_ms: colony
            .stock_ledger
            .as_ref()
            .map_or(i64::MAX, |ledger| now.saturating_sub(ledger.last_counted)),
        population: colony.housing.population,
        deaths: colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_some())
            .count(),
        survival_death_events: colony
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind.as_str(),
                    "death_starvation" | "death_dehydration" | "death_starvation_and_dehydration"
                )
            })
            .count(),
        migration_departures: colony.housing.departures,
        average_food_water_deficit: living
            .iter()
            .map(|cat| 200.0 - cat.needs.hunger - cat.needs.thirst)
            .sum::<f64>()
            / living_count,
        lowest_hunger: living
            .iter()
            .map(|cat| cat.needs.hunger)
            .reduce(f64::min)
            .unwrap_or(0.0),
        lowest_thirst: living
            .iter()
            .map(|cat| cat.needs.thirst)
            .reduce(f64::min)
            .unwrap_or(0.0),
        lowest_health: living
            .iter()
            .map(|cat| cat.needs.health)
            .reduce(f64::min)
            .unwrap_or(0.0),
        revealed: colony.revealed_tiles.len(),
        resets: colony
            .events
            .iter()
            .filter(|event| event.kind == "reset")
            .count(),
    };
    Ok((journey, outcome))
}

async fn poor_decisions(seed: u32) -> Result<(), JourneyFailure> {
    let (_, guided) = run_signed_guidance(seed, false).await?;
    let (mut poor_journey, poor) = run_signed_guidance(seed, true).await?;
    if guided.resets > 1
        || poor.resets > 1
        || !poor_guidance_has_visible_survival_cost(guided, poor)
        || poor.revealed < guided.revealed
    {
        return Err(poor_journey.failure(format!(
            "deliberately poor signed guidance lacked bounded visible consequences: guided={guided:?}, poor={poor:?}"
        )));
    }
    poor_journey.milestone = Some("physical-effect");
    poor_journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "food": colony.resources.food,
                "water": colony.resources.water,
                "population": colony.housing.population,
                "revealed": colony.revealed_tiles.len(),
                "resets": colony.events.iter().filter(|event| event.kind == "reset").collect::<Vec<_>>(),
            })
        })
        .await
}

async fn fresh_survival(seed: u32) -> Result<(), JourneyFailure> {
    match std::env::var("CAT_SYSTEM_SUBJOURNEY_ID").as_deref() {
        Ok("needs-and-food") => return fresh_survival_needs(seed).await,
        Ok("emergency-water") => return fresh_water_emergency(seed).await,
        Ok("poor-decisions") => return poor_decisions(seed).await,
        _ => {}
    }
    aggregate_results(
        "fresh-world survival",
        vec![
            ("needs-and-food", fresh_survival_needs(seed).await),
            ("emergency-water", fresh_water_emergency(seed).await),
            ("poor-decisions", poor_decisions(seed).await),
        ],
    )
}

async fn housing_birth_aging(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[1], seed, housing_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[1], seed, error))?;
    let age_before = selected(&journey.last_snapshot).cats[0].age_hours;
    let buildings_before = selected(&journey.last_snapshot).buildings.len();
    let action = journey.signed(|actor| ClientAction::PlanBuilding {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        building_type: ProtocolBuildingType::Den,
        site: None,
    });
    journey.send(action).await?;
    let mut birth = false;
    let mut death = false;
    let mut aging = false;
    let result = journey
        .eventually(24 * 60 * 60_000, 60 * 60_000, |snapshot| {
            let colony = selected(snapshot);
            birth |= colony.events.iter().any(|event| event.kind == "birth");
            death |= colony
                .events
                .iter()
                .any(|event| event.kind.starts_with("death_"));
            aging |= colony.cats.iter().any(|cat| cat.age_hours > age_before);
            birth && death && aging && colony.buildings.len() > buildings_before
        })
        .await;
    if let Err(mut failure) = result {
        failure.reason = format!(
            "birth/aging/death/housing flags: birth={birth}, aging={aging}, death={death}, planned_housing={}; {}",
            selected(&journey.last_snapshot).buildings.len() > buildings_before,
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "housing": colony.housing, "cats": colony.cats, "buildings": colony.buildings })
        })
        .await
}

async fn housing_conception(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[1], seed, conception_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[1], seed, error))?;
    journey
        .eventually(3 * 24 * 60 * 60_000, 60 * 60_000, |snapshot| {
            selected(snapshot)
                .events
                .iter()
                .any(|event| event.kind == "conception")
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            json!(
                selected(snapshot)
                    .cats
                    .iter()
                    .filter(|cat| cat.pregnant)
                    .collect::<Vec<_>>()
            )
        })
        .await
}

async fn housing_migration(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[1], seed, migration_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[1], seed, error))?;
    let initial = selected(&journey.last_snapshot);
    let initial_population = initial.housing.population;
    let initial_food = initial.resources.food + initial.resources.fish;
    let initial_water = initial.resources.water;
    let initial_materials = initial.resources.materials;
    let arrival = journey
        .eventually(2 * 60 * 60_000, 1_000, |snapshot| {
            selected(snapshot)
                .cats
                .iter()
                .any(|cat| cat.migration_status == cat_protocol::CatMigrationStatus::Arriving)
        })
        .await;
    let arrival = match arrival {
        Ok(snapshot) => snapshot,
        Err(mut failure) => {
            failure.reason = format!(
                "prosperous migration fixture produced no physical arrival: population={initial_population}, food+fish={initial_food}, water={initial_water}, materials={initial_materials}; {}",
                failure.reason
            );
            return Err(failure);
        }
    };
    let migrant_id = selected(&arrival)
        .cats
        .iter()
        .find(|cat| cat.migration_status == cat_protocol::CatMigrationStatus::Arriving)
        .map(|cat| cat.id.clone())
        .expect("arrival milestone includes an arriving migrant");
    let mut last_status = cat_protocol::CatMigrationStatus::Arriving;
    let completion = journey
        .eventually(7 * 24 * 60 * 60_000, 15 * 60_000, |snapshot| {
            selected(snapshot)
                .cats
                .iter()
                .find(|cat| cat.id == migrant_id)
                .is_some_and(|cat| {
                    last_status = cat.migration_status;
                    cat.migration_status != cat_protocol::CatMigrationStatus::Arriving
                })
        })
        .await;
    if let Err(mut failure) = completion {
        let colony = selected(&journey.last_snapshot);
        let phases = colony
            .cats
            .iter()
            .filter(|cat| cat.migration_status != cat_protocol::CatMigrationStatus::Resident)
            .map(|cat| (&cat.id, cat.migration_status, cat.position, cat.destination))
            .collect::<Vec<_>>();
        failure.reason = format!(
            "physical migrant {migrant_id} never crossed the village gate within seven game-days; last_status={last_status:?}, phases={phases:?}; {}",
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| json!(selected(snapshot).housing))
        .await
}

async fn housing_extinction(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[1], seed, extinction_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[1], seed, error))?;
    journey
        .eventually(24 * 60 * 60_000, 60_000, |snapshot| {
            let colony = selected(snapshot);
            colony.events.iter().any(|event| {
                event.kind == "reset_reason" && event.message.contains("all-cats-dead")
            }) && colony.cats.iter().any(|cat| cat.death_time.is_none())
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "population": colony.housing.population, "events": colony.events })
        })
        .await
}

async fn housing_lifecycle(seed: u32) -> Result<(), JourneyFailure> {
    let mut failures = Vec::new();
    for (name, result) in [
        ("birth-aging-death-housing", housing_birth_aging(seed).await),
        ("conception", housing_conception(seed).await),
        ("migration-probation", housing_migration(seed).await),
        (
            "starvation-extinction-reset",
            housing_extinction(seed).await,
        ),
    ] {
        if let Err(mut failure) = result {
            failure.reason = format!("{name}: {}", failure.reason);
            failures.push(failure);
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    let reasons = failures
        .iter()
        .map(|failure| failure.reason.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    let mut failure = failures.remove(0);
    failure.reason = format!("independent housing subjourneys: {reasons}");
    Err(failure)
}

fn officer_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    colony.run_started_at = START_MS - 31 * 60 * 60_000;
    colony.leader_id = colony.cats.last().map(|cat| cat.id.clone());
    let anchor = colony.anchor;
    for (index, role) in OfficerRole::ALL.iter().copied().enumerate() {
        let prerequisite = prerequisite_for(role);
        if !colony
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|node| node == prerequisite.upgrade_node)
        {
            colony
                .upgrade_tree
                .owned_node_ids
                .push(prerequisite.upgrade_node.to_owned());
        }
        if !colony
            .buildings
            .iter()
            .any(|building| building.building_type == prerequisite.building && building.is_complete)
        {
            let mut building = BuildingRuntime {
                id: format!("system-officer-{role:?}"),
                building_type: prerequisite.building,
                position: TilePos {
                    x: anchor.x + 5 + index as i32 * 4,
                    y: anchor.y + 6,
                },
                is_complete: true,
                construction_progress: 100,
                ..BuildingRuntime::default()
            };
            if role == OfficerRole::ClothLeader {
                building.production_queue = vec![cat_sim::world_tick::ProductionQueueEntry {
                    recipe_id: "fibre_to_thread".to_owned(),
                    repeat: true,
                }];
                if !colony
                    .upgrade_tree
                    .owned_node_ids
                    .iter()
                    .any(|node| node == "textiles")
                {
                    colony
                        .upgrade_tree
                        .owned_node_ids
                        .push("textiles".to_owned());
                }
            }
            colony.buildings.push(building);
        }
    }
    if let Some(pile) = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.accepts.contains(&ResourceKind::Fibre))
    {
        pile.contents.fibre = 10.0;
    }
    reconcile_colony_stockpiles(colony);
    colony.resources.fibre = 10.0;
    for y in anchor.y..=(anchor.y + 10) {
        for x in anchor.x..=(anchor.x + 34) {
            let pos = TilePos { x, y };
            colony
                .world_tiles
                .entry(pos)
                .or_insert_with(|| WorldTileRuntime {
                    pos,
                    tile_type: TileType::Meadow,
                    resources: TileResources {
                        food: 0,
                        herbs: 0,
                        water: 0,
                        gem: 0,
                        clay: 0,
                        sand: 0,
                    },
                    max_resources: MaxResources { food: 0, herbs: 0 },
                    danger_level: 0.0,
                    path_wear: 0,
                    last_depleted: 0,
                    overlay_feature: None,
                });
        }
    }
    let authored = colony
        .world_tiles
        .iter()
        .filter(|(pos, _)| {
            pos.x >= anchor.x
                && pos.x <= anchor.x + 34
                && pos.y >= anchor.y
                && pos.y <= anchor.y + 10
        })
        .map(|(pos, tile)| (*pos, tile.clone()))
        .collect::<Vec<_>>();
    for (pos, tile) in authored {
        world.shared_spatial.tiles.insert(pos, tile);
    }
}

fn protocol_role(role: OfficerRole) -> ProtocolOfficerRole {
    match role {
        OfficerRole::Steward => ProtocolOfficerRole::Steward,
        OfficerRole::Accountant => ProtocolOfficerRole::Accountant,
        OfficerRole::Forester => ProtocolOfficerRole::Forester,
        OfficerRole::Farmer => ProtocolOfficerRole::Farmer,
        OfficerRole::Captain => ProtocolOfficerRole::Captain,
        OfficerRole::Loremaster => ProtocolOfficerRole::Loremaster,
        OfficerRole::ClothLeader => ProtocolOfficerRole::ClothLeader,
    }
}

async fn officers(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[2], seed, officer_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[2], seed, error))?;
    if !selected(&journey.last_snapshot).officers.is_empty() {
        return Err(journey.failure("offices were not initially vacant".to_owned()));
    }
    let cats = selected(&journey.last_snapshot)
        .cats
        .iter()
        .take(OfficerRole::ALL.len())
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();
    let research_event_baseline = selected(&journey.last_snapshot)
        .events
        .iter()
        .map(|event| event.timestamp)
        .max()
        .unwrap_or(i64::MIN);
    for (&role, cat_id) in OfficerRole::ALL.iter().zip(cats) {
        let action = journey.signed(|actor| ClientAction::AssignOfficer {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            role: protocol_role(role),
            cat_id,
        });
        journey.send(action).await?;
    }
    if selected(&journey.last_snapshot).officers.len() != OfficerRole::ALL.len() {
        return Err(journey.failure("not every vacant office accepted an exact holder".to_owned()));
    }
    let mut saw_research_unlock = false;
    let effects = journey
        .eventually(30 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            saw_research_unlock |= colony.events.iter().any(|event| {
                event.kind == "research_unlocked" && event.timestamp > research_event_baseline
            });
            OfficerRole::ALL.iter().copied().all(|role| {
                let protocol = protocol_role(role);
                if role == OfficerRole::Captain {
                    colony
                        .jobs
                        .iter()
                        .any(|job| job.kind == JobKind::TrainWarrior)
                        || colony.events.iter().any(|event| {
                            event.message.to_ascii_lowercase().contains("train warrior")
                        })
                } else {
                    colony.buildings.iter().any(|building| {
                        building
                            .work_slots
                            .iter()
                            .any(|slot| slot.automated_by == Some(protocol))
                    })
                }
            }) && saw_research_unlock
        })
        .await;
    if let Err(mut failure) = effects {
        let colony = selected(&journey.last_snapshot);
        let observed = OfficerRole::ALL
            .iter()
            .copied()
            .map(|role| {
                let protocol = protocol_role(role);
                let effect = if role == OfficerRole::Captain {
                    colony
                        .jobs
                        .iter()
                        .any(|job| job.kind == JobKind::TrainWarrior)
                } else {
                    colony.buildings.iter().any(|building| {
                        building
                            .work_slots
                            .iter()
                            .any(|slot| slot.automated_by == Some(protocol))
                    })
                };
                (role, effect)
            })
            .collect::<Vec<_>>();
        failure.reason = format!(
            "not every appointed role produced its owned automation effect: {observed:?}; saw_research_unlock={saw_research_unlock}; {}",
            failure.reason
        );
        return Err(failure);
    }
    for &role in OfficerRole::ALL {
        let action = journey.signed(|actor| ClientAction::UnassignOfficer {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            role: protocol_role(role),
        });
        journey.send(action).await?;
    }
    let colony = selected(&journey.last_snapshot);
    if !colony.officers.is_empty()
        || colony.buildings.iter().any(|building| {
            building
                .work_slots
                .iter()
                .any(|slot| slot.automated_by.is_some())
        })
        || colony.jobs.iter().any(|job| {
            job.kind == JobKind::TrainWarrior
                && matches!(
                    job.status,
                    cat_protocol::JobStatus::Queued | cat_protocol::JobStatus::Active
                )
        })
    {
        return Err(journey.failure(
            "unassigning all seven roles did not release every owned automation effect".to_owned(),
        ));
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "officers": colony.officers,
                "automation": colony.buildings.iter().flat_map(|building| &building.work_slots).map(|slot| (&slot.cat_id, slot.automated_by)).collect::<Vec<_>>(),
                "training": colony.jobs.iter().filter(|job| job.kind == JobKind::TrainWarrior && matches!(job.status, cat_protocol::JobStatus::Queued | cat_protocol::JobStatus::Active)).collect::<Vec<_>>(),
            })
        })
        .await
}

async fn research_unlock_chain(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[3], seed, common_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[3], seed, error))?;
    let blessings_before = selected(&journey.last_snapshot).research.blessings;
    let blocked = journey.signed(|actor| ClientAction::UnlockNode {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        node_id: "basic_tools".to_owned(),
    });
    journey.send_rejected(blocked, "PrerequisitesUnmet").await?;
    for action in [
        journey.signed(|actor| ClientAction::ResearchNode {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            node_id: "research_hut".to_owned(),
        }),
        journey.signed(|actor| ClientAction::ResearchNode {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            node_id: "research_hut_foundations".to_owned(),
        }),
        journey.signed(|actor| ClientAction::UnlockNode {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            node_id: "basic_tools".to_owned(),
        }),
    ] {
        journey.send(action).await?;
    }
    let colony = selected(&journey.last_snapshot);
    for node in ["research_hut", "research_hut_foundations", "basic_tools"] {
        if !colony
            .research
            .owned_node_ids
            .iter()
            .any(|owned| owned == node)
        {
            return Err(journey.failure(format!(
                "prerequisite/blessing research chain did not own {node}"
            )));
        }
    }
    if colony.research.blessings >= blessings_before
        || !colony.events.iter().any(|event| event.kind == "node_owned")
        || !colony
            .events
            .iter()
            .any(|event| event.kind == "research_unlocked")
    {
        return Err(journey.failure(
            "research and blessing purchase families were not independently projected".to_owned(),
        ));
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| json!(selected(snapshot).research))
        .await
}

async fn shrine_tithe(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[3], seed, common_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[3], seed, error))?;
    let blessings_before = selected(&journey.last_snapshot).research.blessings;
    let action = journey.signed(|actor| ClientAction::OfferTithe {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
    });
    journey.send(action).await?;
    let colony = selected(&journey.last_snapshot);
    if colony.research.blessings <= blessings_before
        || !colony.events.iter().any(|event| event.kind == "tithe")
    {
        return Err(journey.failure("signed shrine tithe did not mint blessings".to_owned()));
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "blessings": colony.research.blessings, "tithe": colony.events.iter().find(|event| event.kind == "tithe") })
        })
        .await
}

fn shrine_offering_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    colony.cats.truncate(10);
    colony.jobs.clear();
    colony.last_offering_at = None;
    colony.ritual_requested_at = None;
}

async fn shrine_offering(
    seed: u32,
    resource: OfferingResource,
    legacy_materials: bool,
) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[3], seed, shrine_offering_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[3], seed, error))?;
    let blessings_before = selected(&journey.last_snapshot).research.blessings;
    let action = if legacy_materials {
        journey.signed(|actor| ClientAction::OfferMaterials {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
        })
    } else {
        journey.signed(|actor| ClientAction::OfferResource {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            resource,
        })
    };
    journey.send(action).await?;
    let mut saw_carry = false;
    let mut saw_ritual = false;
    let result = journey
        .eventually(90 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            saw_carry |= colony
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::CarryOffering);
            saw_ritual |= colony
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::PerformOffering);
            saw_carry
                && saw_ritual
                && colony.research.blessings > blessings_before
                && colony
                    .events
                    .iter()
                    .any(|event| event.kind == "offering" || event.kind == "blessing_delivered")
        })
        .await;
    if let Err(mut failure) = result {
        failure.reason = format!(
            "offering {resource:?} legacy={legacy_materials} flags: carry={saw_carry}, ritual={saw_ritual}; {}",
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "blessings": colony.research.blessings,
                "jobs": colony.jobs.iter().filter(|job| matches!(job.kind, JobKind::CarryOffering | JobKind::PerformOffering)).collect::<Vec<_>>(),
            })
        })
        .await
}

async fn research_and_shrine(seed: u32) -> Result<(), JourneyFailure> {
    aggregate_results(
        "research and shrine",
        vec![
            (
                "research-prerequisite-and-blessing",
                research_unlock_chain(seed).await,
            ),
            ("immediate-tithe", shrine_tithe(seed).await),
            (
                "legacy-material-ritual",
                shrine_offering(seed, OfferingResource::Materials, true).await,
            ),
            (
                "food-resource-ritual",
                shrine_offering(seed, OfferingResource::Food, false).await,
            ),
            (
                "herb-resource-ritual",
                shrine_offering(seed, OfferingResource::Herbs, false).await,
            ),
            (
                "material-resource-ritual",
                shrine_offering(seed, OfferingResource::Materials, false).await,
            ),
        ],
    )
}

fn shrine_demand_setup(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.jobs.clear();
    colony.officers.clear();
    colony.ritual_requested_at = None;
    colony.test_time_scale = 60.0;

    for building in &mut colony.buildings {
        building.assigned_cat = None;
        building.automated_by = None;
        for slot in &mut building.additional_work_slots {
            slot.assigned_cat.clear();
            slot.automated_by = None;
        }
    }

    let away = Position {
        map: cat_sim::entities::MapType::World,
        x: f64::from(colony.anchor.x + 5),
        y: f64::from(colony.anchor.y + 5),
    };
    for cat in &mut colony.cats {
        cat.death_time = None;
        cat.current_task = None;
        cat.position = away;
        cat.destination = None;
        cat.carrying = None;
        cat.activity = cat_sim::entities::CatActivity::Idle;
        cat.needs.hunger = 100.0;
        cat.needs.thirst = 100.0;
        cat.needs.rest = 100.0;
        cat.needs.health = 100.0;
    }

    for pile in &mut colony.stockpiles {
        pile.contents.food = 0.0;
        pile.contents.water = 0.0;
    }
    let store = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.is_general_storehouse())
        .expect("founding fixture has a physical general storehouse");
    store.contents.food = 200.0;
    store.contents.water = 200.0;
    reconcile_colony_stockpiles(colony);
}

async fn shrine_demand_lifecycle(seed: u32) -> Result<(), JourneyFailure> {
    const SCENARIO_ID: &str = "shrine-demand-ritual-lifecycle";
    let mut journey = Journey::start(SCENARIO_ID, seed, shrine_demand_setup)
        .await
        .map_err(|error| bootstrap_failure(SCENARIO_ID, seed, error))?;
    let colony = selected(&journey.last_snapshot);
    if colony.jobs.iter().any(|job| job.kind == JobKind::Ritual) {
        return Err(journey.failure("shrine-demand fixture started with a Ritual job".to_owned()));
    }
    let blessings_before = colony.research.blessings;
    let initial_positions = colony
        .cats
        .iter()
        .map(|cat| (cat.name.clone(), cat.position))
        .collect::<BTreeMap<_, _>>();

    let demand = journey.signed(|actor| ClientAction::RequestJob {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        kind: JobKind::Ritual,
    });
    journey.send(demand).await?;
    if !selected(&journey.last_snapshot)
        .events
        .iter()
        .any(|event| event.kind == "ritual_ready")
    {
        return Err(
            journey.failure("accepted signed Ritual demand projected no request event".to_owned())
        );
    }
    let duplicate_demand = journey.signed(|actor| ClientAction::RequestJob {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        kind: JobKind::Ritual,
    });
    journey
        .send_rejected(
            duplicate_demand,
            "Ritual request already pending or active.",
        )
        .await?;

    let mut ritual_job_id = None;
    let mut assigned_cat_name = None;
    let mut saw_assigned_job = false;
    let mut saw_physical_shrine_work = false;
    let result = journey
        .eventually(15 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            if let Some(job) = colony.jobs.iter().find(|job| job.kind == JobKind::Ritual) {
                ritual_job_id.get_or_insert_with(|| job.id.clone());
                if let Some(name) = job.assigned_cat_name.as_ref() {
                    assigned_cat_name.get_or_insert_with(|| name.clone());
                    saw_assigned_job = true;
                    if let Some(cat) = colony.cats.iter().find(|cat| cat.name == *name) {
                        saw_physical_shrine_work |= initial_positions
                            .get(name)
                            .is_some_and(|initial| cat.position != *initial)
                            && matches!(
                                cat.activity,
                                cat_protocol::CatActivity::Traveling
                                    | cat_protocol::CatActivity::Working
                                    | cat_protocol::CatActivity::Returning
                            );
                    }
                }
            }
            // The public snapshot intentionally projects only queued/active work.
            // Completion is therefore the exact job disappearing after it was seen,
            // paired with its typed completion event and durable output below.
            let completed = ritual_job_id
                .as_ref()
                .is_some_and(|id| colony.jobs.iter().all(|job| job.id != *id));
            saw_assigned_job
                && saw_physical_shrine_work
                && completed
                && colony.research.blessings > blessings_before
                && colony.events.iter().any(|event| {
                    event.kind == "job_completed"
                        && event.message.to_ascii_lowercase().contains("ritual")
                })
                && colony
                    .events
                    .iter()
                    .any(|event| event.kind == "blessing_delivered")
        })
        .await;
    if let Err(mut failure) = result {
        failure.reason = format!(
            "shrine demand flags: job_id={ritual_job_id:?}, assigned={saw_assigned_job}, cat={assigned_cat_name:?}, physical_move_or_work={saw_physical_shrine_work}, blessings_before={blessings_before}, blessings_after={}; {}",
            selected(&journey.last_snapshot).research.blessings,
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "blessings": colony.research.blessings,
                "ritualJobs": colony.jobs.iter().filter(|job| job.kind == JobKind::Ritual).collect::<Vec<_>>(),
                "ritualEvents": colony.events.iter().filter(|event| matches!(event.kind.as_str(), "ritual_ready" | "job_completed" | "blessing_delivered")).collect::<Vec<_>>(),
                "ritualCats": colony.cats.iter().filter(|cat| cat.role_xp.ritualist > 0.0).collect::<Vec<_>>(),
            })
        })
        .await
}

fn election_setup(world: &mut WorldState) {
    common_setup(world);
    world.colonies[0].leader_id = Some(world.colonies[0].cats[0].id.clone());
    world.colonies[0].elections.push(ElectionRuntime {
        id: "system-election".to_owned(),
        opened_at: START_MS,
        closes_at: START_MS + 10 * 60_000,
        resolved_at: None,
        winner_cat_id: None,
        kind: ElectionKind::Scheduled,
    });
}

async fn elections(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[4], seed, election_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[4], seed, error))?;
    let election = selected(&journey.last_snapshot)
        .election
        .as_ref()
        .ok_or_else(|| journey.failure("fixture election was not projected".to_owned()))?;
    let election_id = election.id.clone();
    let cat_id = election
        .candidates
        .first()
        .ok_or_else(|| journey.failure("election had no eligible candidates".to_owned()))?
        .id
        .clone();
    let action = journey.signed(|actor| ClientAction::CastVote {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        election_id: election_id.clone(),
        cat_id: cat_id.clone(),
    });
    let snapshot = journey.send(action).await?;
    if snapshot
        .colonies
        .first()
        .and_then(|colony| colony.election.as_ref())
        .is_none_or(|election| election.total_ballots != 1)
    {
        return Err(journey.failure("signed ballot was not counted exactly once".to_owned()));
    }
    journey
        .eventually(15 * 60_000, 60_000, |snapshot| {
            let colony = selected(snapshot);
            colony.election.is_none()
                && colony
                    .leader
                    .as_ref()
                    .is_some_and(|leader| leader.id == cat_id)
                && colony
                    .events
                    .iter()
                    .any(|event| event.kind == "election_resolved")
        })
        .await?;
    let action = journey.signed(|actor| ClientAction::RequestVoteKick {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
    });
    journey.send(action).await?;
    if selected(&journey.last_snapshot).vote_kick.is_none() {
        return Err(journey.failure("signed vote-kick did not open or join a petition".to_owned()));
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "election": colony.election,
                "leader": colony.leader,
                "voteKick": colony.vote_kick,
                "resolved": colony.events.iter().filter(|event| event.kind == "election_resolved").collect::<Vec<_>>(),
            })
        })
        .await
}

fn raid_setup(world: &mut WorldState) {
    officer_setup(world);
    let colony = &mut world.colonies[0];
    colony.active_raid = Some("system-raid".to_owned());
    colony.threat_pressure = 100.0;
    colony.raiders.push(RaiderRuntime {
        id: "system-raider".to_owned(),
        raid_id: "system-raid".to_owned(),
        position: Position::default(),
        destination: None,
        attack: 1.0,
        defense: 1.0,
        health: 2.0,
    });
}

async fn raids(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[5], seed, raid_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[5], seed, error))?;
    let recruit = selected(&journey.last_snapshot)
        .cats
        .iter()
        .find(|cat| cat.specialization != Some(cat_protocol::Specialization::Warrior))
        .map(|cat| cat.id.clone())
        .ok_or_else(|| journey.failure("raid fixture had no trainable cat".to_owned()))?;
    let action = journey.signed(|actor| ClientAction::TrainWarrior {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        cat_id: Some(recruit),
    });
    journey.send(action).await?;
    let action = journey.signed(|actor| ClientAction::DefendRaid {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
    });
    journey.send(action).await?;
    journey
        .eventually(60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            !colony.threat.raid_active || colony.raiders.iter().all(|raider| raider.hp < 2.0)
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "threat": colony.threat, "raiders": colony.raiders })
        })
        .await
}

fn gather_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    let tile = colony.anchor;
    colony.stockpiles.push(Stockpile {
        id: "system-gather".to_owned(),
        rect: ZoneRect {
            x1: tile.x + 12,
            y1: tile.y + 12,
            x2: tile.x + 12,
            y2: tile.y + 12,
        },
        accepts: BTreeSet::from([ResourceKind::Food]),
        contents: Resources {
            food: 8.0,
            ..Resources::default()
        },
    });
    colony.gather_spots.push(GatherSpot {
        stockpile_id: "system-gather".to_owned(),
        kind: ResourceKind::Food,
        expires_at_ms: START_MS + 60 * 60_000,
        purpose: GatherSpotPurpose::General,
    });
    let designation = TilePos {
        x: tile.x + 18,
        y: tile.y + 18,
    };
    colony.world_tiles.insert(
        designation,
        WorldTileRuntime {
            pos: designation,
            tile_type: TileType::Meadow,
            resources: TileResources {
                food: 0,
                herbs: 0,
                water: 0,
                gem: 0,
                clay: 0,
                sand: 0,
            },
            max_resources: MaxResources { food: 0, herbs: 0 },
            danger_level: 0.0,
            path_wear: 0,
            last_depleted: 0,
            overlay_feature: None,
        },
    );
    colony.revealed_tiles.insert(designation);
    colony.claimed_tiles.push(designation);
    colony.agricultural_tiles.insert(designation);
    reconcile_colony_stockpiles(colony);
}

async fn stockpiles(seed: u32) -> Result<(), JourneyFailure> {
    aggregate_results(
        "stockpile and gather",
        vec![
            ("designation-lifecycle", stockpile_lifecycle(seed).await),
            ("physical-gather-haul", stockpile_haul(seed).await),
            ("blocked-and-collision", stockpile_boundaries(seed).await),
            ("full-capacity", stockpile_full_boundaries(seed).await),
        ],
    )
}

fn designation_setup(world: &mut WorldState) {
    common_setup(world);
    let world_seed = world.world_seed;
    let colony = &mut world.colonies[0];
    let anchor = colony.anchor;
    for y in anchor.y..=anchor.y + 40 {
        for x in anchor.x..=anchor.x + 40 {
            let pos = TilePos { x, y };
            let tile = WorldTileRuntime {
                pos,
                tile_type: TileType::Meadow,
                resources: TileResources {
                    food: 0,
                    herbs: 0,
                    water: 0,
                    gem: 0,
                    clay: 0,
                    sand: 0,
                },
                max_resources: MaxResources { food: 0, herbs: 0 },
                danger_level: 0.0,
                path_wear: 0,
                last_depleted: 1,
                overlay_feature: None,
            };
            colony.world_tiles.insert(pos, tile.clone());
            colony.revealed_tiles.insert(pos);
            world.shared_spatial.tiles.insert(pos, tile);
        }
    }
    for (dx, dy) in [(30, 30), (31, 30), (32, 30), (33, 30)] {
        let pos = TilePos {
            x: anchor.x + dx,
            y: anchor.y + dy,
        };
        if !colony.claimed_tiles.contains(&pos) {
            colony.claimed_tiles.push(pos);
        }
    }
    let water = TilePos {
        x: anchor.x + 36,
        y: anchor.y + 29,
    };
    let water_tile = colony.world_tiles.get_mut(&water).expect("authored water");
    water_tile.tile_type = TileType::River;
    water_tile.resources.water = 100;
    world.shared_spatial.tiles.insert(water, water_tile.clone());
    register_colony_spatial(world, 0);
    let colony = &world.colonies[0];
    for point in [
        TilePos {
            x: anchor.x + 30,
            y: anchor.y + 30,
        },
        TilePos {
            x: anchor.x + 31,
            y: anchor.y + 30,
        },
        TilePos {
            x: anchor.x + 32,
            y: anchor.y + 30,
        },
        TilePos {
            x: anchor.x + 33,
            y: anchor.y + 30,
        },
    ] {
        assert!(
            cat_sim::world_tick::stockpile_placement_error(
                colony,
                ZoneRect {
                    x1: point.x,
                    y1: point.y,
                    x2: point.x,
                    y2: point.y,
                },
                world_seed,
                true,
            )
            .is_none(),
            "authored designation point {point:?} must be clear"
        );
    }
}

async fn stockpile_lifecycle(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[6], seed, designation_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[6], seed, error))?;
    let anchor = selected(&journey.last_snapshot).anchor;
    let point = |dx: i32| TilePoint {
        x: anchor.x + dx,
        y: anchor.y + 30,
    };
    let before = selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .map(|pile| pile.id.clone())
        .collect::<BTreeSet<_>>();
    let action = journey.signed(|actor| ClientAction::DesignateStockpile {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: point(30),
        b: point(30),
        accepts: vec![ProtoResourceKind::Food],
    });
    journey.send(action).await?;
    let stockpile_id = selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .find(|pile| !before.contains(&pile.id))
        .map(|pile| pile.id.clone())
        .ok_or_else(|| journey.failure("designated stockpile was not projected".to_owned()))?;
    let action = journey.signed(|actor| ClientAction::RemoveStockpile {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        stockpile_id: stockpile_id.clone(),
    });
    journey.send(action).await?;
    if selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .any(|pile| pile.id == stockpile_id)
    {
        return Err(journey.failure("removed stockpile remained projected".to_owned()));
    }

    let action = journey.signed(|actor| ClientAction::DesignateGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: point(32),
        b: point(32),
        kind: ProtoResourceKind::Herbs,
    });
    journey.send(action).await?;
    let gather_id = selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .find(|pile| {
            pile.gather_spot.is_some_and(|spot| {
                spot.purpose == cat_protocol::GatherSpotPurpose::General
                    && spot.kind == ProtoResourceKind::Herbs
            })
        })
        .map(|pile| pile.id.clone())
        .ok_or_else(|| journey.failure("general gather spot was not projected".to_owned()))?;
    let action = journey.signed(|actor| ClientAction::RemoveGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        stockpile_id: gather_id.clone(),
    });
    journey.send(action).await?;

    let water = TilePoint {
        x: anchor.x + 36,
        y: anchor.y + 29,
    };
    let action = journey.signed(|actor| ClientAction::DesignateFishingSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        at: water,
    });
    journey.send(action).await?;
    let fishing_id = selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .find(|pile| {
            pile.gather_spot
                .is_some_and(|spot| spot.purpose == cat_protocol::GatherSpotPurpose::Fishing)
        })
        .map(|pile| pile.id.clone())
        .ok_or_else(|| journey.failure("fishing designation was not projected".to_owned()))?;
    let action = journey.signed(|actor| ClientAction::RemoveGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        stockpile_id: fishing_id.clone(),
    });
    journey.send(action).await?;
    if selected(&journey.last_snapshot)
        .stockpiles
        .iter()
        .any(|pile| pile.id == gather_id || pile.id == fishing_id)
    {
        return Err(journey.failure("removed gather/fishing spot remained projected".to_owned()));
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "designations": colony.stockpiles.iter().filter(|pile| {
                    pile.id == stockpile_id || pile.id == gather_id || pile.id == fishing_id
                }).collect::<Vec<_>>()
            })
        })
        .await
}

async fn stockpile_boundaries(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[6], seed, designation_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[6], seed, error))?;
    let colony = selected(&journey.last_snapshot);
    let anchor = colony.anchor;
    let open = TilePoint {
        x: anchor.x + 30,
        y: anchor.y + 30,
    };
    let building = colony
        .buildings
        .iter()
        .find(|building| building.building_type != ProtocolBuildingType::Shrine)
        .map(|building| building.world_position)
        .ok_or_else(|| journey.failure("fixture had no blocking building".to_owned()))?;
    let designate = |journey: &Journey, at: TilePoint| {
        journey.signed(|actor| ClientAction::DesignateStockpile {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            a: at,
            b: at,
            accepts: vec![ProtoResourceKind::Food],
        })
    };
    journey.send(designate(&journey, open)).await?;
    journey
        .send_rejected(designate(&journey, open), "another stockpile")
        .await?;
    journey
        .send_rejected(designate(&journey, building), "building footprint")
        .await?;
    let water = TilePoint {
        x: anchor.x + 36,
        y: anchor.y + 29,
    };
    let blocked_gather = journey.signed(|actor| ClientAction::DesignateGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: water,
        b: water,
        kind: ProtoResourceKind::Food,
    });
    journey.send_rejected(blocked_gather, "water").await?;
    journey.milestone = Some("physical-effect");
    Ok(())
}

fn full_designation_setup(world: &mut WorldState) {
    designation_setup(world);
    let colony = &mut world.colonies[0];
    let anchor = colony.anchor;
    for index in 0..MAX_DESIGNATED_STOCKPILES {
        colony.stockpiles.push(Stockpile {
            id: format!("system-full-stockpile-{index}"),
            rect: ZoneRect {
                x1: anchor.x + 100 + i32::try_from(index).expect("small limit"),
                y1: anchor.y + 100,
                x2: anchor.x + 100 + i32::try_from(index).expect("small limit"),
                y2: anchor.y + 100,
            },
            accepts: BTreeSet::from([ResourceKind::Food]),
            contents: Resources::default(),
        });
    }
    for index in 0..MAX_GATHER_SPOTS {
        let id = format!("system-full-gather-{index}");
        colony.stockpiles.push(Stockpile {
            id: id.clone(),
            rect: ZoneRect {
                x1: anchor.x + 100 + i32::try_from(index).expect("small limit"),
                y1: anchor.y + 102,
                x2: anchor.x + 100 + i32::try_from(index).expect("small limit"),
                y2: anchor.y + 102,
            },
            accepts: BTreeSet::from([ResourceKind::Food]),
            contents: Resources::default(),
        });
        colony.gather_spots.push(GatherSpot {
            stockpile_id: id,
            kind: ResourceKind::Food,
            expires_at_ms: START_MS + 60 * 60_000,
            purpose: GatherSpotPurpose::General,
        });
    }
    reconcile_colony_stockpiles(colony);
}

async fn stockpile_full_boundaries(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[6], seed, full_designation_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[6], seed, error))?;
    let anchor = selected(&journey.last_snapshot).anchor;
    let point = TilePoint {
        x: anchor.x + 30,
        y: anchor.y + 30,
    };
    let action = journey.signed(|actor| ClientAction::DesignateStockpile {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: point,
        b: point,
        accepts: vec![ProtoResourceKind::Food],
    });
    journey
        .send_rejected(
            action,
            &format!("already have {MAX_DESIGNATED_STOCKPILES} stockpiles"),
        )
        .await?;
    let action = journey.signed(|actor| ClientAction::DesignateGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: point,
        b: point,
        kind: ProtoResourceKind::Food,
    });
    journey
        .send_rejected(
            action,
            &format!("already have {MAX_GATHER_SPOTS} gather spots"),
        )
        .await?;
    journey.milestone = Some("physical-effect");
    Ok(())
}

async fn stockpile_haul(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[6], seed, gather_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[6], seed, error))?;
    let anchor = selected(&journey.last_snapshot).anchor;
    let designate = journey.signed(|actor| ClientAction::DesignateStockpile {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: TilePoint {
            x: anchor.x + 18,
            y: anchor.y + 18,
        },
        b: TilePoint {
            x: anchor.x + 18,
            y: anchor.y + 18,
        },
        accepts: vec![ProtoResourceKind::Food],
    });
    journey.send(designate).await?;
    let action = journey.signed(|actor| ClientAction::HaulGatherSpot {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        stockpile_id: "system-gather".to_owned(),
        cat_id: None,
    });
    journey.send(action).await?;
    journey
        .eventually(10 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            colony
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::HaulGatherSpot)
                || colony
                    .stockpiles
                    .iter()
                    .find(|pile| pile.id == "system-gather")
                    .is_some_and(|pile| pile.contents.food == 0.0)
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "gather": colony.stockpiles.iter().find(|pile| pile.id == "system-gather"),
                "haul": colony.active_stockpile_haul,
                "jobs": colony.jobs.iter().filter(|job| job.kind == JobKind::HaulGatherSpot).collect::<Vec<_>>(),
            })
        })
        .await
}

fn transport_setup(world: &mut WorldState) {
    common_setup(world);
    let colony = &mut world.colonies[0];
    for node in ["rail", "shipping"] {
        if !colony
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|owned| owned == node)
        {
            colony.upgrade_tree.owned_node_ids.push(node.to_owned());
        }
    }
    let anchor = colony.anchor;
    for y in (anchor.y - 1)..=(anchor.y + 23) {
        for x in (anchor.x - 1)..=(anchor.x + 40) {
            let pos = TilePos { x, y };
            colony
                .world_tiles
                .entry(pos)
                .or_insert_with(|| WorldTileRuntime {
                    pos,
                    tile_type: TileType::Meadow,
                    resources: TileResources {
                        food: 0,
                        herbs: 0,
                        water: 0,
                        gem: 0,
                        clay: 0,
                        sand: 0,
                    },
                    max_resources: MaxResources { food: 0, herbs: 0 },
                    danger_level: 0.0,
                    path_wear: 0,
                    last_depleted: 0,
                    overlay_feature: None,
                });
            colony.revealed_tiles.insert(pos);
        }
    }
    for x in (anchor.x + 31)..=(anchor.x + 34) {
        let tile = colony
            .world_tiles
            .get_mut(&TilePos {
                x,
                y: anchor.y + 21,
            })
            .expect("shipping channel tile");
        tile.tile_type = TileType::River;
        tile.overlay_feature = None;
    }
    for dx in [10, 13, 16, 19] {
        colony
            .world_tiles
            .get_mut(&TilePos {
                x: anchor.x + dx,
                y: anchor.y + 20,
            })
            .unwrap()
            .tile_type = TileType::River;
    }
    let rail_start = (anchor.y + 21..=anchor.y + 23)
        .find_map(|y| {
            (anchor.x + 20..=anchor.x + 27).find_map(|x| {
                (0..3)
                    .all(|dx| {
                        let pos = TilePos { x: x + dx, y };
                        colony.revealed_tiles.contains(&pos)
                            && colony.world_tiles.get(&pos).is_some_and(|tile| {
                                tile.tile_type != TileType::River
                                    && tile.overlay_feature.as_deref() != Some("river")
                                    && tile.resources.water == 0
                            })
                            && cat_sim::world_tick::road_placement_error(
                                colony,
                                pos,
                                world.world_seed,
                            )
                            .is_none()
                    })
                    .then_some(TilePos { x, y })
            })
        })
        .expect("authored transport fixture has a valid dry rail line");
    for (id, x, y, gem) in [
        (
            format!("system-rail-source:{}:{}", rail_start.x, rail_start.y),
            rail_start.x - 1,
            rail_start.y,
            20.0,
        ),
        (
            format!("system-rail-destination:{}:{}", rail_start.x, rail_start.y),
            rail_start.x + 3,
            rail_start.y,
            0.0,
        ),
        (
            "system-ship-source".to_owned(),
            anchor.x + 30,
            anchor.y + 21,
            20.0,
        ),
        (
            "system-ship-destination".to_owned(),
            anchor.x + 35,
            anchor.y + 21,
            0.0,
        ),
    ] {
        colony.stockpiles.push(Stockpile {
            id,
            rect: ZoneRect {
                x1: x,
                y1: y,
                x2: x,
                y2: y,
            },
            accepts: BTreeSet::from([ResourceKind::Gem]),
            contents: Resources {
                gem,
                ..Resources::default()
            },
        });
    }
    colony.resources.gem += 40.0;
    reconcile_colony_stockpiles(colony);
    let authored = colony
        .world_tiles
        .iter()
        .filter(|(pos, _)| {
            pos.x >= anchor.x - 1
                && pos.x <= anchor.x + 40
                && pos.y >= anchor.y - 1
                && pos.y <= anchor.y + 23
        })
        .map(|(pos, tile)| (*pos, tile.clone()))
        .collect::<Vec<_>>();
    for (pos, tile) in authored {
        world.shared_spatial.tiles.insert(pos, tile);
    }
}

async fn transport_network(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[7], seed, transport_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[7], seed, error))?;
    let colony = selected(&journey.last_snapshot);
    let anchor = colony.anchor;
    let transport_y = anchor.y + 21;
    let cats = colony
        .cats
        .iter()
        .take(8)
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();
    let rail_source_id = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id.starts_with("system-rail-source:"))
        .map(|pile| pile.id.clone())
        .expect("projected rail source marker");
    let rail_destination_id = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id.starts_with("system-rail-destination:"))
        .map(|pile| pile.id.clone())
        .expect("projected rail destination marker");
    let mut rail_parts = rail_source_id.rsplit(':');
    let rail_y = rail_parts
        .next()
        .and_then(|value| value.parse::<i32>().ok())
        .expect("rail marker y");
    let rail_x = rail_parts
        .next()
        .and_then(|value| value.parse::<i32>().ok())
        .expect("rail marker x");
    let road_a = TilePoint {
        x: anchor.x + 3,
        y: anchor.y + 1,
    };
    let road_b = TilePoint {
        x: anchor.x + 5,
        y: anchor.y + 1,
    };
    let action = journey.signed(|actor| ClientAction::BuildRoad {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: road_a,
        b: road_b,
    });
    journey.send(action).await?;
    journey
        .eventually(30 * 60_000, 1_000, |snapshot| {
            selected(snapshot).road_tiles.contains(&road_b)
        })
        .await?;

    let mut bridge = None;
    for dx in [10, 13, 16, 19] {
        let candidate = TilePoint {
            x: anchor.x + dx,
            y: anchor.y + 20,
        };
        let action = journey.signed(|actor| ClientAction::BuildBridge {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            at: candidate,
        });
        if journey.send(action).await.is_ok() {
            bridge = Some(candidate);
            break;
        }
    }
    let bridge = bridge.ok_or_else(|| {
        journey.failure("no authored bridge candidate had two unobstructed banks".to_owned())
    })?;
    journey
        .eventually(30 * 60_000, 1_000, |snapshot| {
            selected(snapshot)
                .bridge_tiles
                .iter()
                .any(|built| built.tile == bridge && built.completed)
        })
        .await?;

    let rail_path = (0..3)
        .map(|dx| TilePoint {
            x: rail_x + dx,
            y: rail_y,
        })
        .collect::<Vec<_>>();
    let action = journey.signed(|actor| ClientAction::DesignateRail {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: rail_path[0],
        b: rail_path[2],
        cat_id: cats[1].clone(),
    });
    journey.send(action).await?;
    journey
        .eventually(4 * 60 * 60_000, 10_000, |snapshot| {
            selected(snapshot).transport.track_tiles.len() >= 3
        })
        .await?;

    for (index, (land, water, cat_id)) in [
        (
            TilePoint {
                x: anchor.x + 30,
                y: transport_y,
            },
            TilePoint {
                x: anchor.x + 31,
                y: transport_y,
            },
            cats[2].clone(),
        ),
        (
            TilePoint {
                x: anchor.x + 35,
                y: transport_y,
            },
            TilePoint {
                x: anchor.x + 34,
                y: transport_y,
            },
            cats[3].clone(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let action = journey.signed(|actor| ClientAction::BuildDock {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            land,
            water,
            cat_id,
        });
        journey.send(action).await?;
        journey
            .eventually(4 * 60 * 60_000, 10_000, |snapshot| {
                selected(snapshot).transport.docks.len() > index
            })
            .await?;
    }

    for (mode, home, cat_id) in [
        (ProtocolTransportMode::Rail, rail_path[0], cats[4].clone()),
        (
            ProtocolTransportMode::Shipping,
            TilePoint {
                x: anchor.x + 31,
                y: transport_y,
            },
            cats[5].clone(),
        ),
    ] {
        let action = journey.signed(|actor| ClientAction::BuildTransportVehicle {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            mode,
            home,
            cat_id,
        });
        journey.send(action).await?;
        journey
            .eventually(4 * 60 * 60_000, 10_000, |snapshot| {
                selected(snapshot)
                    .transport
                    .vehicles
                    .iter()
                    .any(|vehicle| vehicle.mode == mode)
            })
            .await?;
    }

    let mut route_cats = BTreeSet::new();
    for (mode, source, destination, path) in [
        (
            ProtocolTransportMode::Rail,
            rail_source_id.clone(),
            rail_destination_id.clone(),
            rail_path,
        ),
        (
            ProtocolTransportMode::Shipping,
            "system-ship-source".to_owned(),
            "system-ship-destination".to_owned(),
            (31..=34)
                .map(|dx| TilePoint {
                    x: anchor.x + dx,
                    y: transport_y,
                })
                .collect(),
        ),
    ] {
        let cat_id = selected(&journey.last_snapshot)
            .cats
            .iter()
            .find(|cat| {
                cat.death_time.is_none()
                    && cat.current_task.is_none()
                    && cat.assigned_building_id.is_none()
                    && cat.carrying.is_none()
                    && cat.activity == cat_protocol::CatActivity::Idle
                    && !route_cats.contains(&cat.id)
            })
            .map(|cat| cat.id.clone())
            .ok_or_else(|| journey.failure("no idle route crew remained".to_owned()))?;
        route_cats.insert(cat_id.clone());
        let action = journey.signed(|actor| ClientAction::CreateTransportRoute {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            mode,
            source_stockpile_id: source,
            destination_stockpile_id: destination,
            resource: ProtoResourceKind::Gem,
            amount: 2.0,
            path,
            cat_id,
            repeat: false,
        });
        journey.send(action).await?;
    }
    let scoped_total =
        |snapshot: &WorldSnapshot, source: &str, destination: &str, mode: ProtocolTransportMode| {
            let colony = selected(snapshot);
            colony
                .stockpiles
                .iter()
                .filter(|pile| pile.id == source || pile.id == destination)
                .map(|pile| pile.contents.gem)
                .sum::<f64>()
                + colony
                    .transport
                    .vehicles
                    .iter()
                    .filter(|vehicle| vehicle.mode == mode)
                    .map(|vehicle| vehicle.cargo)
                    .sum::<f64>()
        };
    let destination_amount = |snapshot: &WorldSnapshot, id: &str| {
        selected(snapshot)
            .stockpiles
            .iter()
            .find(|pile| pile.id == id)
            .map_or(0.0, |pile| pile.contents.gem)
    };
    // Each authored source starts with 20 finite units, and route creation rejects
    // unless the live physical source still holds the requested cargo.
    let rail_baseline = 20.0;
    let ship_baseline = 20.0;
    let mut rail_destination_peak = 0.0_f64;
    let mut ship_destination_peak = 0.0_f64;
    let mut rail_conservation_observed = false;
    let mut ship_conservation_observed = false;
    let mut rail_conserved = true;
    let mut ship_conserved = true;
    let delivery = journey
        .eventually(4 * 60 * 60_000, 10_000, |snapshot| {
            let colony = selected(snapshot);
            rail_destination_peak =
                rail_destination_peak.max(destination_amount(snapshot, &rail_destination_id));
            ship_destination_peak =
                ship_destination_peak.max(destination_amount(snapshot, "system-ship-destination"));
            // Reported conservation becomes observable only after physical loading:
            // route planning intentionally does not disclose source or destination stock.
            let route_is_in_flight = |mode| {
                colony.transport.routes.iter().any(|route| {
                    route.mode == mode
                        && !matches!(
                            route.phase.as_str(),
                            "boarding" | "loading" | "complete" | "cancelled"
                        )
                })
            };
            if route_is_in_flight(ProtocolTransportMode::Rail) {
                rail_conservation_observed = true;
                rail_conserved &= (scoped_total(
                    snapshot,
                    &rail_source_id,
                    &rail_destination_id,
                    ProtocolTransportMode::Rail,
                ) - rail_baseline)
                    .abs()
                    <= 1.0e-9;
            }
            if route_is_in_flight(ProtocolTransportMode::Shipping) {
                ship_conservation_observed = true;
                ship_conserved &= (scoped_total(
                    snapshot,
                    "system-ship-source",
                    "system-ship-destination",
                    ProtocolTransportMode::Shipping,
                ) - ship_baseline)
                    .abs()
                    <= 1.0e-9;
            }
            colony.transport.routes.len() == 2
                && colony
                    .transport
                    .routes
                    .iter()
                    .all(|route| route.phase == "complete")
                && rail_destination_peak >= 2.0
                && ship_destination_peak >= 2.0
                && rail_conservation_observed
                && ship_conservation_observed
                && rail_conserved
                && ship_conserved
        })
        .await;
    if let Err(mut failure) = delivery {
        let routes = selected(&journey.last_snapshot)
            .transport
            .routes
            .iter()
            .map(|route| (&route.id, route.mode, route.phase.as_str()))
            .collect::<Vec<_>>();
        failure.reason = format!(
            "transport lifecycle routes={routes:?}, rail_destination_peak={rail_destination_peak}, ship_destination_peak={ship_destination_peak}, rail_conserved={rail_conserved} after_observation={rail_conservation_observed} (baseline={rail_baseline}), ship_conserved={ship_conserved} after_observation={ship_conservation_observed} (baseline={ship_baseline}); {}",
            failure.reason
        );
        return Err(failure);
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "roads": colony.road_tiles, "bridges": colony.bridge_tiles, "transport": colony.transport })
        })
        .await
}

async fn transport(seed: u32) -> Result<(), JourneyFailure> {
    if std::env::var("CAT_SYSTEM_SUBJOURNEY_ID").as_deref() == Ok("wall-gate-crossing") {
        return wall_gate_access(seed).await;
    }
    aggregate_results(
        "transport and palisade access",
        vec![
            ("road-bridge-rail-shipping", transport_network(seed).await),
            ("wall-gate-crossing", wall_gate_access(seed).await),
        ],
    )
}

fn trader_setup(world: &mut WorldState) {
    common_setup(world);
    let world_seed = world.world_seed;
    let colony = &mut world.colonies[0];
    // Leave exact headroom in the player-visible Food pile for the signed buy.
    colony.resources.food = 0.0;
    colony.resources.water = 0.0;
    colony.resources.materials = 0.0;
    for pile in &mut colony.stockpiles {
        pile.contents.food = 0.0;
        pile.contents.water = 0.0;
        pile.contents.materials = 0.0;
    }
    reconcile_colony_stockpiles(colony);
    colony.resources.food = 0.0;
    colony.resources.water = 0.0;
    colony.resources.materials = 0.0;
    colony.coin = 1_000.0;
    colony.add_item(Item::new(ItemKind::Mug, Material::Wood, 1), 2);
    colony.trader = Some(TraderRuntime {
        id: "system-trader".to_owned(),
        position: Position::default(),
        destination: None,
        state: TraderState::Trading,
        arrived_at: Some(START_MS),
        depart_at: Some(START_MS + 60 * 60_000),
        route_exterior: None,
        visit_destination: None,
        route_blocked: false,
        visit_number: 1,
        stock: BTreeMap::from([(ResourceKind::Food, 10.0)]),
        items: Default::default(),
        coin: trader::TRADER_STARTING_COIN,
    });

    let mut peer = found_colony_at(
        world_seed,
        "system-trade-peer",
        START_MS,
        991,
        TilePos { x: 160, y: 160 },
    );
    peer.name = "Trade Peer".to_owned();
    peer.kind = VillageKind::Personal;
    peer.owner_player_id = None;
    peer.jobs.clear();
    peer.transport = Default::default();
    for building in &mut peer.buildings {
        building.construction_cargo = None;
    }
    peer.resources = Resources::default();
    for pile in &mut peer.stockpiles {
        pile.contents = Resources::default();
    }
    peer.stockpiles.push(Stockpile {
        id: "system-peer-trade-source".to_owned(),
        rect: ZoneRect {
            x1: peer.anchor.x + 10,
            y1: peer.anchor.y + 10,
            x2: peer.anchor.x + 10,
            y2: peer.anchor.y + 10,
        },
        accepts: BTreeSet::from([ResourceKind::Materials, ResourceKind::Food]),
        contents: Resources {
            materials: 10.0,
            ..Resources::default()
        },
    });
    peer.resources.materials = 10.0;
    peer.known_village_ids.insert(colony.id.clone());
    colony.known_village_ids.insert(peer.id.clone());
    peer.village_trade_offers.insert(
        "system-peer-offer".to_owned(),
        VillageTradeOffer {
            id: "system-peer-offer".to_owned(),
            from_colony_id: peer.id.clone(),
            to_colony_id: colony.id.clone(),
            offered_kind: ResourceKind::Materials,
            offered_amount: 1.0,
            requested_kind: ResourceKind::Food,
            requested_amount: 1.0,
            created_at: START_MS,
        },
    );
    world.colonies.push(peer);
    register_colony_spatial(world, 1);
    let (source, target) = (world.colonies[0].anchor, world.colonies[1].anchor);
    for y in (source.y - 10)..=(target.y + 10) {
        for x in (source.x - 10)..=(target.x + 10) {
            let pos = TilePos { x, y };
            world.shared_spatial.tiles.insert(
                pos,
                WorldTileRuntime {
                    pos,
                    tile_type: TileType::Meadow,
                    resources: TileResources {
                        food: 0,
                        herbs: 0,
                        water: 0,
                        gem: 0,
                        clay: 0,
                        sand: 0,
                    },
                    max_resources: MaxResources { food: 0, herbs: 0 },
                    danger_level: 0.0,
                    path_wear: 0,
                    last_depleted: 0,
                    overlay_feature: None,
                },
            );
        }
    }
}

fn wall_gate_crossing(colony: &cat_sim::world_tick::ColonyRuntime) -> (TilePos, TilePos) {
    let area = from_tiles(
        &colony
            .claimed_tiles
            .iter()
            .map(|tile| GridPos {
                x: tile.x,
                y: tile.y,
            })
            .collect::<Vec<_>>(),
    );
    let gate = gate_placement_default(&area).expect("founded village gate");
    let delta = side_delta(gate.side);
    let exterior = TilePos {
        x: gate.x + delta.x,
        y: gate.y + delta.y,
    };
    let crossing = TilePos {
        x: exterior.x + 2 * delta.x,
        y: exterior.y + 2 * delta.y,
    };
    (exterior, crossing)
}

fn wall_gate_setup(world: &mut WorldState) {
    trader_setup(world);
    if let Some(store) = world.colonies[0]
        .stockpiles
        .iter_mut()
        .find(|pile| pile.is_general_storehouse())
    {
        store.contents.food = 10.0;
        store.contents.materials = 10.0;
    }
    world.colonies[0].resources.food = 10.0;
    world.colonies[0].resources.materials = 10.0;
    let (exterior, crossing) = wall_gate_crossing(&world.colonies[0]);
    let source = world.colonies[0].anchor;
    let target = world.colonies[1].anchor;
    let outward = (
        (crossing.x - exterior.x).signum(),
        (crossing.y - exterior.y).signum(),
    );
    assert!(
        (target.x - source.x) * outward.0 + (target.y - source.y) * outward.1 > 0,
        "trade peer must be beyond the selected village gate"
    );

    let clear_tile = |pos: TilePos| WorldTileRuntime {
        pos,
        tile_type: TileType::Meadow,
        resources: TileResources {
            food: 0,
            herbs: 0,
            water: 0,
            gem: 0,
            clay: 0,
            sand: 0,
        },
        max_resources: MaxResources { food: 0, herbs: 0 },
        danger_level: 0.0,
        path_wear: 0,
        last_depleted: 1,
        overlay_feature: None,
    };
    let river_positions = if outward.1 != 0 {
        (-1_000..=1_000)
            .map(|x| TilePos { x, y: crossing.y })
            .collect::<Vec<_>>()
    } else {
        (-1_000..=1_000)
            .map(|y| TilePos { x: crossing.x, y })
            .collect::<Vec<_>>()
    };
    for pos in river_positions {
        let mut tile = clear_tile(pos);
        tile.tile_type = TileType::River;
        tile.resources.water = 100;
        world.shared_spatial.tiles.insert(pos, tile);
    }

    let far_bank = TilePos {
        x: crossing.x + outward.0,
        y: crossing.y + outward.1,
    };
    let near_bank = TilePos {
        x: crossing.x - outward.0,
        y: crossing.y - outward.1,
    };
    let colony = &mut world.colonies[0];
    for pos in [exterior, near_bank, crossing, far_bank] {
        let mut tile = clear_tile(pos);
        if pos == crossing {
            tile.tile_type = TileType::River;
            tile.resources.water = 100;
        }
        colony.world_tiles.insert(pos, tile.clone());
        colony.revealed_tiles.insert(pos);
        world.shared_spatial.tiles.insert(pos, tile);
    }
}

async fn wall_gate_access(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[7], seed, wall_gate_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[7], seed, error))?;
    let colony = selected(&journey.last_snapshot);
    if colony.village_gate.is_none() || colony.wall_segments.is_empty() {
        return Err(journey.failure("founded palisade/gate was not projected".to_owned()));
    }
    let (_, crossing) = {
        let gate = colony.village_gate.expect("checked gate");
        let outward = match gate.side {
            cat_protocol::GateSide::N => (0, -1),
            cat_protocol::GateSide::E => (1, 0),
            cat_protocol::GateSide::S => (0, 1),
            cat_protocol::GateSide::W => (-1, 0),
        };
        let exterior = TilePos {
            x: gate.x + outward.0,
            y: gate.y + outward.1,
        };
        (
            exterior,
            TilePos {
                x: exterior.x + 2 * outward.0,
                y: exterior.y + 2 * outward.1,
            },
        )
    };
    let accept = |journey: &Journey| {
        journey.signed(|actor| ClientAction::AcceptVillageTrade {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            offer_id: "system-peer-offer".to_owned(),
        })
    };
    journey
        .send_rejected(
            accept(&journey),
            "No passable land caravan route connects the village shrines",
        )
        .await?;
    let bridge = journey.signed(|actor| ClientAction::BuildBridge {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        at: TilePoint {
            x: crossing.x,
            y: crossing.y,
        },
    });
    journey.send(bridge).await?;
    journey
        .eventually(30 * 60_000, 1_000, |snapshot| {
            selected(snapshot).bridge_tiles.iter().any(|bridge| {
                bridge.tile.x == crossing.x && bridge.tile.y == crossing.y && bridge.completed
            })
        })
        .await?;
    journey.send(accept(&journey)).await?;
    if !journey
        .last_snapshot
        .village_trade_caravans
        .iter()
        .any(|caravan| caravan.id == "system-peer-offer")
    {
        return Err(journey
            .failure("gate crossing did not make the accepted caravan route visible".to_owned()));
    }
    journey
        .eventually(2 * 60 * 60_000, 5_000, |snapshot| {
            snapshot.village_trade_caravans.is_empty()
                && selected(snapshot).resources.materials >= 1.0
        })
        .await?;
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({
                "gate": colony.village_gate,
                "walls": colony.wall_segments,
                "bridge": colony.bridge_tiles.iter().find(|bridge| bridge.tile.x == crossing.x && bridge.tile.y == crossing.y),
                "materials": colony.resources.materials,
            })
        })
        .await
}

async fn trader_commerce_and_village_trade(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[8], seed, trader_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[8], seed, error))?;
    let coin_before = selected(&journey.last_snapshot).coin;
    let action = journey.signed(|actor| ClientAction::SellGoods {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        kind: "mug".to_owned(),
        material: "wood".to_owned(),
        quality: 1,
        count: 1,
    });
    journey.send(action).await?;
    let colony = selected(&journey.last_snapshot);
    if colony.coin <= coin_before
        || colony
            .trader
            .as_ref()
            .and_then(|trader| {
                trader
                    .buy_offers
                    .iter()
                    .find(|offer| offer.kind == "mug" && offer.material == "wood")
            })
            .is_none_or(|offer| offer.available != 1)
        || !colony.events.iter().any(|event| event.kind == "trade_sell")
    {
        return Err(
            journey.failure("signed sale did not debit the exact item and credit coin".to_owned())
        );
    }
    let action = journey.signed(|actor| ClientAction::BuyResource {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        resource: ProtoResourceKind::Food,
        amount: 1.0,
    });
    journey.send(action).await?;
    let offer_action = |journey: &Journey| {
        journey.signed(|actor| ClientAction::OfferVillageTrade {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            target_colony_id: "system-trade-peer".to_owned(),
            offered_kind: ProtoResourceKind::Food,
            offered_amount: 1.0,
            requested_kind: ProtoResourceKind::Water,
            requested_amount: 1.0,
        })
    };
    journey.send(offer_action(&journey)).await?;
    let signed_offer = journey
        .last_snapshot
        .village_trade_offers
        .iter()
        .find(|offer| offer.id != "system-peer-offer")
        .map(|offer| offer.id.clone())
        .ok_or_else(|| journey.failure("signed village offer was not projected".to_owned()))?;
    let cancel = journey.signed(|actor| ClientAction::CancelVillageTrade {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        offer_id: signed_offer,
    });
    journey.send(cancel).await?;
    let accept = journey.signed(|actor| ClientAction::AcceptVillageTrade {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        offer_id: "system-peer-offer".to_owned(),
    });
    journey.send(accept).await?;
    if !journey
        .last_snapshot
        .village_trade_caravans
        .iter()
        .any(|caravan| caravan.id == "system-peer-offer")
    {
        return Err(journey.failure("accepted trade did not create a physical caravan".to_owned()));
    }
    journey
        .eventually(2 * 60 * 60_000, 5_000, |snapshot| {
            snapshot.village_trade_caravans.is_empty()
                && selected(snapshot).resources.materials >= 1.0
        })
        .await?;
    let colony = selected(&journey.last_snapshot);
    if colony
        .trader
        .as_ref()
        .is_none_or(|trader| trader.id != "system-trader")
    {
        return Err(
            journey.failure("trader purchase or village offer was not projected".to_owned())
        );
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "coin": colony.coin, "trader": colony.trader, "offers": snapshot.village_trade_offers })
        })
        .await
}

fn trader_restock_setup(world: &mut WorldState) {
    designation_setup(world);
    let colony = &mut world.colonies[0];
    colony.trader = None;
    colony.trader_visit_count = 1;
    colony.test_time_scale = 1_000.0;
    let scaled_interval_ms =
        (trader::TRADER_VISIT_INTERVAL_GAME_HOURS * 3_600_000.0 / colony.test_time_scale) as i64;
    colony.last_trader_departed_at = Some(START_MS - scaled_interval_ms + 5_000);
}

async fn trader_restock(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[8], seed, trader_restock_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[8], seed, error))?;
    let manifest_seed = journey.last_snapshot.world_seed as u32;
    let first_manifest =
        trader::stock_for_visit(manifest_seed, &selected(&journey.last_snapshot).id, 1);
    let second_manifest =
        trader::stock_for_visit(manifest_seed, &selected(&journey.last_snapshot).id, 2);
    let mut saw_arrival = false;
    let mut observed_visit = None;
    let mut observed_stock = Vec::new();
    let result = journey
        .eventually(2 * 60_000, 1_000, |snapshot| {
            let colony = selected(snapshot);
            saw_arrival |= colony
                .events
                .iter()
                .any(|event| event.kind == "trader_arrived");
            colony.trader.as_ref().is_some_and(|visiting| {
                observed_visit = Some((visiting.visit_number, visiting.state));
                observed_stock = visiting
                    .stock
                    .iter()
                    .map(|entry| (entry.resource, entry.available.to_bits()))
                    .collect();
                visiting.visit_number == 2
                    && visiting.state == cat_protocol::TraderVisitState::Trading
                    && {
                        let visible = visiting
                            .stock
                            .iter()
                            .map(|entry| entry.available.to_bits())
                            .collect::<Vec<_>>();
                        visible
                            == second_manifest
                                .values()
                                .map(|available| available.to_bits())
                                .collect::<Vec<_>>()
                    }
            })
        })
        .await;
    if let Err(mut failure) = result {
        failure.reason = format!(
            "second trader visit/restock flags: arrival={saw_arrival}, observed_visit={observed_visit:?}, observed_stock={observed_stock:?}, expected_second={:?}; {}",
            second_manifest, failure.reason
        );
        return Err(failure);
    }
    if first_manifest == second_manifest {
        return Err(journey.failure(format!(
            "scheduled visit 2 arrived and projected its exact deterministic stock, but it was identical to visit 1 instead of a fresh restock: {second_manifest:?}"
        )));
    }
    journey
        .restart_with_fingerprint(|snapshot| {
            let colony = selected(snapshot);
            json!({ "trader": colony.trader, "arrival": colony.events.iter().filter(|event| event.kind == "trader_arrived").collect::<Vec<_>>() })
        })
        .await
}

async fn traders(seed: u32) -> Result<(), JourneyFailure> {
    if std::env::var("CAT_SYSTEM_SUBJOURNEY_ID").as_deref() == Ok("scheduled-restock") {
        return trader_restock(seed).await;
    }
    aggregate_results(
        "trader commerce",
        vec![
            (
                "buy-sell-and-village-trade",
                trader_commerce_and_village_trade(seed).await,
            ),
            ("scheduled-restock", trader_restock(seed).await),
        ],
    )
}

fn multi_village_setup(world: &mut WorldState) {
    common_setup(world);
    let world_seed = world.world_seed;
    let global = &world.colonies[0];
    let area = from_tiles(
        &global
            .claimed_tiles
            .iter()
            .map(|tile| GridPos {
                x: tile.x,
                y: tile.y,
            })
            .collect::<Vec<_>>(),
    );
    let gate = gate_placement_default(&area).expect("global contact fixture has a gate");
    let delta = side_delta(gate.side);
    let perpendicular = GridPos {
        x: -delta.y,
        y: delta.x,
    };
    for (index, side) in [-1, 1].into_iter().enumerate() {
        let peer_shrine = TilePos {
            x: gate.x + delta.x * 7 + perpendicular.x * side * 5,
            y: gate.y + delta.y * 7 + perpendicular.y * side * 5,
        };
        let mut peer = found_colony_at(
            world_seed,
            format!("system-contact-peer-{index}"),
            START_MS,
            8_000 + index as u32,
            TilePos {
                x: peer_shrine.x - 1,
                y: peer_shrine.y - 1,
            },
        );
        peer.kind = VillageKind::Personal;
        peer.owner_player_id = None;
        world.colonies.push(peer);
        register_colony_spatial(world, world.colonies.len() - 1);
    }

    // Give both directional peers one shared outward road with separate branches.
    // They remain a meaningful scout trip away while every contact is physically reachable.
    for side in [-1, 1] {
        for outward in 1..=7 {
            let pos = TilePos {
                x: gate.x + delta.x * outward,
                y: gate.y + delta.y * outward,
            };
            if let Some(tile) = world.shared_spatial.tiles.get_mut(&pos) {
                tile.tile_type = cat_sim::types::TileType::Meadow;
                tile.resources.water = 0;
                tile.overlay_feature = Some("road_built".to_owned());
            }
        }
        for branch in 1..=5 {
            let pos = TilePos {
                x: gate.x + delta.x * 7 + perpendicular.x * side * branch,
                y: gate.y + delta.y * 7 + perpendicular.y * side * branch,
            };
            if let Some(tile) = world.shared_spatial.tiles.get_mut(&pos) {
                tile.tile_type = cat_sim::types::TileType::Meadow;
                tile.resources.water = 0;
                tile.overlay_feature = Some("road_built".to_owned());
            }
        }
    }
    cat_sim::world_tick::sync_all_colonies_from_shared(world);
}

async fn multi_village(seed: u32) -> Result<(), JourneyFailure> {
    let mut journey = Journey::start(EXECUTABLE_SCENARIO_IDS[9], seed, multi_village_setup)
        .await
        .map_err(|error| bootstrap_failure(EXECUTABLE_SCENARIO_IDS[9], seed, error))?;
    let global_id = selected(&journey.last_snapshot).id.clone();
    let (mut second_client, second_actor) = journey
        .harness
        .connect_authenticated(format!("system-second-owner-{seed}"), "Second Founder")
        .await
        .map_err(|error| journey.failure(format!("second owner authentication failed: {error}")))?;
    let second_found = second_client
        .send_action(&ClientAction::FoundVillage {
            name: "Second Hamlet".to_owned(),
            session_id: second_actor.session_id.clone(),
            sig: Some(second_actor.sig.clone()),
        })
        .await
        .map_err(|error| journey.failure(format!("second owner founding transport: {error}")))?;
    journey.actions.push(second_found.clone());
    if !second_found.result.ok {
        return Err(journey.failure(format!(
            "second personal village founding rejected: {:?}",
            second_found.result.message
        )));
    }
    let second_snapshot = second_client
        .receive_snapshot()
        .await
        .map_err(|error| journey.failure(format!("second founding snapshot: {error}")))?;
    let second_id = second_snapshot
        .selected_colony_id
        .clone()
        .filter(|id| id != &global_id)
        .ok_or_else(|| journey.failure("second owner village was not selected".to_owned()))?;
    let scout = journey.signed(|actor| ClientAction::DispatchScout {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        mission: ScoutMission::Explore,
    });
    journey.send(scout).await?;
    let action = journey.signed(|actor| ClientAction::FoundVillage {
        name: "Socket Hamlet".to_owned(),
        session_id: actor.session_id.clone(),
        sig: Some(actor.sig.clone()),
    });
    journey.send(action).await?;
    let second_scout = second_client
        .send_action(&ClientAction::DispatchScout {
            session_id: second_actor.session_id.clone(),
            nickname: second_actor.nickname.clone(),
            sig: second_actor.sig.clone(),
            mission: ScoutMission::Explore,
        })
        .await
        .map_err(|error| journey.failure(format!("second scout transport: {error}")))?;
    journey.actions.push(second_scout.clone());
    if !second_scout.result.ok {
        return Err(journey.failure(format!(
            "second-village scout rejected: {:?}",
            second_scout.result.message
        )));
    }
    let personal_id = journey
        .last_snapshot
        .selected_colony_id
        .clone()
        .filter(|id| id != &global_id)
        .ok_or_else(|| {
            journey.failure("founding did not select the new personal village".to_owned())
        })?;
    let action = journey.signed(|actor| ClientAction::JoinVillage {
        colony_id: global_id.clone(),
        session_id: actor.session_id.clone(),
        sig: Some(actor.sig.clone()),
    });
    journey.send(action).await?;
    let action = journey.signed(|actor| ClientAction::JoinVillage {
        colony_id: personal_id.clone(),
        session_id: actor.session_id.clone(),
        sig: Some(actor.sig.clone()),
    });
    journey.send(action).await?;
    if journey.last_snapshot.selected_colony_id.as_deref() != Some(personal_id.as_str())
        || journey.last_snapshot.colonies.len() < 2
    {
        return Err(journey
            .failure("global/personal selection did not round-trip over the socket".to_owned()));
    }
    let action = journey.signed(|actor| ClientAction::JoinVillage {
        colony_id: global_id.clone(),
        session_id: actor.session_id.clone(),
        sig: Some(actor.sig.clone()),
    });
    journey.send(action).await?;
    let global = journey
        .last_snapshot
        .colonies
        .iter()
        .find(|colony| colony.id == global_id)
        .expect("global village remains projected");
    let gate = global
        .village_gate
        .expect("global village projects its gate");
    let (dx, dy) = match gate.side {
        cat_protocol::GateSide::N => (0, -1),
        cat_protocol::GateSide::E => (1, 0),
        cat_protocol::GateSide::S => (0, 1),
        cat_protocol::GateSide::W => (-1, 0),
    };
    let (px, py) = (-dy, dx);
    let contact_centers = [-1, 1].map(|side| TilePoint {
        x: gate.x + dx * 7 + px * side * 5,
        y: gate.y + dy * 7 + py * side * 5,
    });
    let mut saw_return = false;
    let mut saw_peer_shrine_reveal = false;
    let contact = journey
        .eventually(45 * 60_000, 5_000, |snapshot| {
            let global = snapshot
                .colonies
                .iter()
                .find(|colony| colony.id == global_id)
                .expect("global village remains projected");
            saw_return |= global.events.iter().any(|event| event.kind == "discovery");
            saw_peer_shrine_reveal |= contact_centers
                .iter()
                .any(|center| global.revealed_tiles.contains(center));
            saw_return
                && saw_peer_shrine_reveal
                && snapshot.known_villages.iter().any(|village| {
                    village.id == second_id || village.id.starts_with("system-contact-peer-")
                })
        })
        .await;
    if let Err(mut failure) = contact {
        failure.reason = format!(
            "contact flags after bounded signed scout lifecycle: returned={saw_return}, peer_shrine_revealed={saw_peer_shrine_reveal}, contact=false; {}",
            failure.reason
        );
        return Err(failure);
    }
    journey.milestone = Some("physical-effect");
    journey
        .restart_with_fingerprint(|snapshot| {
            json!({
                "selected": snapshot.selected_colony_id,
                "villages": snapshot.colonies.iter().map(|colony| (&colony.id, colony.kind, colony.capabilities)).collect::<Vec<_>>(),
            })
        })
        .await
}

fn bootstrap_failure(id: &'static str, seed: u32, reason: String) -> JourneyFailure {
    JourneyFailure {
        scenario_id: id,
        seed,
        milestone: None,
        simulated_ms: 0,
        reason,
        actions: Vec::new(),
        snapshot: serde_json::from_value(json!({
            "now": START_MS,
            "worldSeed": 0,
            "colonies": [],
            "onlineCount": 0,
            "knownVillages": [],
            "villageTradeOffers": [],
            "villageTradeCaravans": []
        }))
        .expect("minimal failure snapshot"),
        restart_difference: None,
    }
}

async fn run_case(id: &'static str, seed: u32) -> Result<(), JourneyFailure> {
    match id {
        "fresh-world-survival-and-needs" => fresh_survival(seed).await,
        "housing-breeding-migration-aging-extinction" => housing_lifecycle(seed).await,
        "all-officers-vacant-and-assigned" => officers(seed).await,
        "research-blessings-and-shrine-work" => research_and_shrine(seed).await,
        "elections-voting-and-vote-kick" => elections(seed).await,
        "raids-training-and-defense" => raids(seed).await,
        "stockpiles-gather-spots-and-hauling" => stockpiles(seed).await,
        "roads-bridges-rail-and-shipping" => transport(seed).await,
        "traders-and-village-trade" => traders(seed).await,
        "multi-village-selection-and-restart" => multi_village(seed).await,
        "shrine-demand-ritual-lifecycle" => shrine_demand_lifecycle(seed).await,
        _ => Err(bootstrap_failure(
            id,
            seed,
            "unknown executable system scenario".to_owned(),
        )),
    }
}

async fn run_requested_seed_tier(id: &'static str) {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        if let Err(failure) = run_case(id, seed).await {
            let trace = failure.write_trace();
            failures.push(format!(
                "{} seed {seed} after {:?} at {}ms: {}; trace={trace:?}",
                failure.scenario_id, failure.milestone, failure.simulated_ms, failure.reason
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "system journey failures for {id}:\n{}",
        failures.join("\n")
    );
}

macro_rules! system_journey_tests {
    ($(($test_name:ident, $scenario_id:literal)),+ $(,)?) => {
        const GENERATED_SYSTEM_SCENARIO_IDS: &[&str] = &[$($scenario_id),+];

        $(
            #[tokio::test]
            async fn $test_name() {
                run_requested_seed_tier($scenario_id).await;
            }
        )+
    };
}

system_journey_tests!(
    (
        real_websocket_fresh_world_survival_and_needs,
        "fresh-world-survival-and-needs"
    ),
    (
        real_websocket_housing_breeding_migration_aging_extinction,
        "housing-breeding-migration-aging-extinction"
    ),
    (
        real_websocket_all_officers_vacant_and_assigned,
        "all-officers-vacant-and-assigned"
    ),
    (
        real_websocket_research_blessings_and_shrine_work,
        "research-blessings-and-shrine-work"
    ),
    (
        real_websocket_elections_voting_and_vote_kick,
        "elections-voting-and-vote-kick"
    ),
    (
        real_websocket_raids_training_and_defense,
        "raids-training-and-defense"
    ),
    (
        real_websocket_stockpiles_gather_spots_and_hauling,
        "stockpiles-gather-spots-and-hauling"
    ),
    (
        real_websocket_roads_bridges_rail_and_shipping,
        "roads-bridges-rail-and-shipping"
    ),
    (
        real_websocket_traders_and_village_trade,
        "traders-and-village-trade"
    ),
    (
        real_websocket_multi_village_selection_and_restart,
        "multi-village-selection-and-restart"
    ),
    (
        real_websocket_shrine_demand_ritual_lifecycle,
        "shrine-demand-ritual-lifecycle"
    ),
);

#[tokio::test]
async fn signed_shrine_demand_runs_one_physical_ritual_and_persists() {
    if let Err(failure) = shrine_demand_lifecycle(super::PRIMARY_SEED).await {
        let trace = failure.write_trace();
        panic!(
            "{} seed {} after {:?} at {}ms: {}; trace={trace:?}",
            failure.scenario_id,
            failure.seed,
            failure.milestone,
            failure.simulated_ms,
            failure.reason
        );
    }
}

#[test]
fn executable_system_ids_are_unique_and_exclude_catalog_sweeps() {
    assert_eq!(GENERATED_SYSTEM_SCENARIO_IDS, EXECUTABLE_SCENARIO_IDS);
    let ids = EXECUTABLE_SCENARIO_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 11);
    assert!(!ids.contains("every-building-plan-build-staff-operate-persist"));
    assert!(!ids.contains("every-recipe-conserved-station-work-and-delivery"));
}
