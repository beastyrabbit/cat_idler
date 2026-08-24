//! Whole-game weapon-production and Leader-progression journeys anchored in
//! `docs/GAME_VISION.md` and the corresponding full-playtest rows in
//! `docs/IMPLEMENTATION_AUDIT.md`.
//!
//! The prepared queue deliberately proves the physical production/equipment
//! mechanics independently of the Captain's strategic decision. The Captain
//! scenario starts from an empty Smithy queue: if demand-driven recipe selection
//! is absent, it remains an ordinary, accurately diagnosed red expectation.

use std::collections::BTreeSet;

use cat_protocol::{
    BuildingSnapshot, ColonySnapshot, ItemLocation, ItemStackSnapshot, ResourceKind, WorldSnapshot,
};
use serde::Serialize;

use super::{Milestone, ScenarioSpec, SeedTier};

pub(crate) const PREPARED_SMELTER_ID: &str = "playtest-prepared-smelter";
pub(crate) const PREPARED_SMITHY_ID: &str = "playtest-prepared-smithy";
pub(crate) const CAPTAIN_SMELTER_ID: &str = "playtest-captain-smelter";
pub(crate) const CAPTAIN_SMITHY_ID: &str = "playtest-captain-smithy";
pub(crate) const SMELTER_RECIPE_ID: &str = "ore_to_metal";
pub(crate) const WEAPON_RECIPE_ID: &str = "smithy_weapon";

const PREPARED_WEAPON_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "signed-queues-accepted",
        description: "the authenticated player authors the exact Smelter and Smithy queues",
    },
    Milestone {
        id: "signed-workers-accepted",
        description: "the authenticated player staffs both physical stations",
    },
    Milestone {
        id: "ore-inbound",
        description: "finite Ore leaves storage in cargo bound for the selected Smelter",
    },
    Milestone {
        id: "ore-at-smelter",
        description: "the Ore reaches the Smelter's station-local input compartment",
    },
    Milestone {
        id: "metal-at-smelter",
        description: "Smelter work consumes Ore and creates finite station-local Metal",
    },
    Milestone {
        id: "metal-outbound",
        description: "the Metal leaves the Smelter as physical outbound cargo",
    },
    Milestone {
        id: "metal-at-smithy",
        description: "the same conserved chain delivers Metal to the selected Smithy",
    },
    Milestone {
        id: "weapon-at-smithy",
        description: "the Smithy consumes Metal and creates one exact metal Weapon identity",
    },
    Milestone {
        id: "weapon-outbound",
        description: "the exact Weapon leaves the Smithy in outbound cargo",
    },
    Milestone {
        id: "weapon-stored",
        description: "the exact credited Weapon reaches an accepting village stockpile",
    },
    Milestone {
        id: "signed-equip-accepted",
        description: "the player targets the intended warrior and exact stored Weapon identity",
    },
    Milestone {
        id: "exact-warrior-equipped",
        description: "that warrior's loadout and the item location name the same Weapon identity",
    },
    Milestone {
        id: "restart-equality",
        description: "save, restart, reconnect preserves queues, finite identity, and loadout",
    },
];

const CAPTAIN_DEMAND_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "captain-observes-shortage",
        description: "a living appointed Captain can observe an unequipped warrior and no usable Weapon",
    },
    Milestone {
        id: "weapon-demand-recorded",
        description: "the authoritative simulation records unmet Weapon demand for that warrior",
    },
    Milestone {
        id: "weapon-recipe-selected",
        description: "the Captain selects smithy_weapon on the empty runnable Smithy queue",
    },
    Milestone {
        id: "captain-chain-runs",
        description: "Ore is hauled through Smelter and Metal through Smithy without player queue guidance",
    },
    Milestone {
        id: "weapon-stored",
        description: "one exact finite Weapon reaches village storage",
    },
    Milestone {
        id: "exact-warrior-equipped",
        description: "the demanded Weapon is physically issued to the originally identified warrior",
    },
    Milestone {
        id: "restart-equality",
        description: "the Captain decision, route, exact identity, and issue survive restart and reconnect",
    },
];

const LEADER_TENURE_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "below-35-control",
        description: "the living Leader begins below the existing 35 leadership policy boundary",
    },
    Milestone {
        id: "crosses-35",
        description: "real tenure raises the same Leader across 35 without an invented XP level",
    },
    Milestone {
        id: "normal-bucket-policy",
        description: "a subsequent controlled policy roll uses the 35..70 leadership weights",
    },
    Milestone {
        id: "crosses-70",
        description: "continued real tenure raises the same Leader across 70",
    },
    Milestone {
        id: "excellent-bucket-policy",
        description: "a subsequent controlled policy roll uses the existing >=70 leadership weights",
    },
    Milestone {
        id: "restart-equality",
        description: "the exact Leader identity and earned leadership persist across restart",
    },
];

const LEADER_RESEARCH_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "affordable-target-visible",
        description: "the projected ledger exposes an affordable deterministic Leader-priority study",
    },
    Milestone {
        id: "before-boundary-locked",
        description: "the study remains unowned immediately before the rolling 24-hour boundary",
    },
    Milestone {
        id: "daily-choice-unlocks",
        description: "the living Leader selects and purchases exactly one affordable study at the boundary",
    },
    Milestone {
        id: "capability-exposed",
        description: "the study's typed capability becomes available through the projected building/recipe contract",
    },
    Milestone {
        id: "no-same-day-second-choice",
        description: "additional research points do not mint another Leader choice inside the same rolling day",
    },
    Milestone {
        id: "restart-equality",
        description: "ownership, points, capability, and colony-wide daily clock survive restart and reconnect",
    },
];

const PRODUCTION_OUTCOMES: &[&str] = &["exact_metal_weapon_equipped"];
const POLICY_OUTCOMES: &[&str] = &["normal_bucket", "excellent_bucket"];
const RESEARCH_OUTCOMES: &[&str] = &["one_affordable_study_unlocked"];

pub(crate) const SCENARIOS: &[ScenarioSpec] = &[
    ScenarioSpec {
        id: "prepared-ore-smelter-smithy-weapon-equip",
        design_anchor: "docs/GAME_VISION.md#pillars / Production is physical",
        initial_setup: "comfortable authenticated village with Ore, unlocked completed Smelter and Smithy, two idle workers, one exact unequipped warrior, and deliberately paused authored queues",
        action_or_trigger: "signed queue edits and worker assignments, deterministic ticks, then signed EquipItem for the produced identity",
        milestones: PREPARED_WEAPON_MILESTONES,
        horizon_ms: 3_600_000,
        allowed_outcomes: PRODUCTION_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: &["paused-authored-queues", "stored-weapon", "equipped-weapon"],
    },
    ScenarioSpec {
        id: "captain-weapon-demand-to-exact-warrior",
        design_anchor: "docs/GAME_VISION.md#pillars / Officers automate categories",
        initial_setup: "comfortable village with appointed Captain, completed empty-queue Smelter and Smithy, Ore, no usable Weapons, and one stable unequipped warrior",
        action_or_trigger: "authoritative Captain tick observes the shortage and must select the required recipe without player guidance",
        milestones: CAPTAIN_DEMAND_MILESTONES,
        horizon_ms: 4 * 60 * 60 * 1_000,
        allowed_outcomes: PRODUCTION_OUTCOMES,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: &["captain-demand", "selected-recipe", "equipped-weapon"],
    },
    ScenarioSpec {
        id: "leader-tenure-crosses-policy-boundaries",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix / Leader progression",
        initial_setup: "young living Leader immediately below 35 leadership, protected survival stores, and deterministic policy rolls that distinguish all three existing weight buckets",
        action_or_trigger: "advance real simulation tenure across the existing 35 and 70 leadership boundaries",
        milestones: LEADER_TENURE_MILESTONES,
        horizon_ms: 102 * 60 * 60 * 1_000,
        allowed_outcomes: POLICY_OUTCOMES,
        seed_tier: SeedTier::Primary,
        persistence_checkpoints: &["normal-policy-boundary", "excellent-policy-boundary"],
    },
    ScenarioSpec {
        id: "leader-daily-research-exposes-capability",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix / research runtime",
        initial_setup: "living Leader with one exact affordable prerequisite-satisfied study and a consumed prior daily choice clock",
        action_or_trigger: "advance to just before and then exactly through the rolling 24-hour Leader choice boundary",
        milestones: LEADER_RESEARCH_MILESTONES,
        horizon_ms: 24 * 60 * 60 * 1_000 + 60_000,
        allowed_outcomes: RESEARCH_OUTCOMES,
        seed_tier: SeedTier::Primary,
        persistence_checkpoints: &["daily-choice-clock", "owned-study-capability"],
    },
];

pub(crate) const EXECUTABLE_SCENARIO_IDS: &[&str] = &[
    "prepared-ore-smelter-smithy-weapon-equip",
    "captain-weapon-demand-to-exact-warrior",
    "leader-tenure-crosses-policy-boundaries",
    "leader-daily-research-exposes-capability",
];

/// Ordered evidence collected from projected WebSocket snapshots. The runner keeps
/// every flag once observed, so a short-lived carrier/local-inventory phase is not
/// lost when later milestones are checked. A failing trace serializes this alongside
/// every action result; it never treats a rejected action as a completed milestone.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeaponRouteEvidence {
    pub(crate) ore_inbound: bool,
    pub(crate) ore_at_smelter: bool,
    pub(crate) metal_at_smelter: bool,
    pub(crate) metal_outbound: bool,
    pub(crate) metal_at_smithy: bool,
    pub(crate) weapon_at_smithy: bool,
    pub(crate) weapon_outbound: bool,
    pub(crate) stored_weapon_id: Option<String>,
    pub(crate) equipped_weapon_id: Option<String>,
}

impl WeaponRouteEvidence {
    pub(crate) fn observe(
        &mut self,
        snapshot: &WorldSnapshot,
        smelter_id: &str,
        smithy_id: &str,
        warrior_id: &str,
    ) {
        let Some(colony) = selected_colony(snapshot) else {
            return;
        };
        let smelter = building(colony, smelter_id);
        let smithy = building(colony, smithy_id);

        if let Some(smelter) = smelter {
            self.ore_inbound |= smelter
                .inbound_cargo
                .iter()
                .any(|stack| stack.kind == ResourceKind::Ore && stack.amount > f64::EPSILON);
            self.ore_at_smelter |= stack_has(&smelter.input_inventory, ResourceKind::Ore);
            self.metal_at_smelter |= stack_has(&smelter.output_inventory, ResourceKind::Metal);
            self.metal_outbound |= smelter
                .outbound_cargo
                .iter()
                .any(|stack| stack.kind == ResourceKind::Metal && stack.amount > f64::EPSILON);
        }
        if let Some(smithy) = smithy {
            self.metal_at_smithy |= smithy
                .inbound_cargo
                .iter()
                .any(|stack| stack.kind == ResourceKind::Metal && stack.amount > f64::EPSILON)
                || stack_has(&smithy.input_inventory, ResourceKind::Metal);
            self.weapon_at_smithy |= stack_has(&smithy.output_inventory, ResourceKind::Weapons);
            self.weapon_outbound |= smithy
                .outbound_cargo
                .iter()
                .any(|stack| stack.kind == ResourceKind::Weapons && stack.amount > f64::EPSILON);
        }

        let stored = exact_weapon_instances(colony).find_map(|instance| {
            matches!(instance.location, ItemLocation::Stockpile { .. }).then(|| instance.id.clone())
        });
        if stored.is_some() {
            self.stored_weapon_id = stored;
        }

        let equipped = colony
            .cats
            .iter()
            .find(|cat| cat.id == warrior_id)
            .and_then(|cat| cat.equipment.weapon_item_id.clone());
        if let Some(item_id) = equipped.filter(|item_id| {
            exact_weapon_instances(colony).any(|instance| {
                instance.id == *item_id
                    && instance.location
                        == ItemLocation::Equipped {
                            cat_id: warrior_id.to_owned(),
                        }
            })
        }) {
            self.equipped_weapon_id = Some(item_id);
        }
    }

    pub(crate) fn production_complete(&self) -> bool {
        self.ore_inbound
            && self.ore_at_smelter
            && self.metal_at_smelter
            && self.metal_outbound
            && self.metal_at_smithy
            && self.weapon_at_smithy
            && self.weapon_outbound
            && self.stored_weapon_id.is_some()
    }

    pub(crate) fn equipment_complete(&self) -> bool {
        self.production_complete() && self.equipped_weapon_id.is_some()
    }

    pub(crate) fn missing_milestones(&self) -> Vec<&'static str> {
        [
            (!self.ore_inbound).then_some("ore-inbound"),
            (!self.ore_at_smelter).then_some("ore-at-smelter"),
            (!self.metal_at_smelter).then_some("metal-at-smelter"),
            (!self.metal_outbound).then_some("metal-outbound"),
            (!self.metal_at_smithy).then_some("metal-at-smithy"),
            (!self.weapon_at_smithy).then_some("weapon-at-smithy"),
            (!self.weapon_outbound).then_some("weapon-outbound"),
            self.stored_weapon_id.is_none().then_some("weapon-stored"),
            self.equipped_weapon_id
                .is_none()
                .then_some("exact-warrior-equipped"),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

pub(crate) fn captain_selected_weapon_recipe(snapshot: &WorldSnapshot) -> bool {
    selected_colony(snapshot)
        .and_then(|colony| building(colony, CAPTAIN_SMITHY_ID))
        .and_then(|smithy| smithy.production_queue.first())
        .is_some_and(|entry| entry.recipe_id == WEAPON_RECIPE_ID)
}

pub(crate) fn affordable_target(snapshot: &WorldSnapshot) -> Option<&str> {
    let colony = selected_colony(snapshot)?;
    colony
        .research
        .next_target
        .as_ref()
        .filter(|target| target.cost <= colony.research.research_points + f64::EPSILON)
        .map(|target| target.id.as_str())
}

pub(crate) fn newly_owned_studies<'a>(
    before: &'a WorldSnapshot,
    after: &'a WorldSnapshot,
) -> Vec<&'a str> {
    let before_owned = selected_colony(before)
        .map(|colony| {
            colony
                .research
                .owned_node_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    selected_colony(after)
        .into_iter()
        .flat_map(|colony| colony.research.owned_node_ids.iter())
        .map(String::as_str)
        .filter(|id| !before_owned.contains(id))
        .collect()
}

fn selected_colony(snapshot: &WorldSnapshot) -> Option<&ColonySnapshot> {
    snapshot
        .selected_colony_id
        .as_deref()
        .and_then(|selected| {
            snapshot
                .colonies
                .iter()
                .find(|colony| colony.id == selected)
        })
        .or_else(|| snapshot.colonies.first())
}

fn building<'a>(colony: &'a ColonySnapshot, id: &str) -> Option<&'a BuildingSnapshot> {
    colony.buildings.iter().find(|building| building.id == id)
}

fn stack_has(stacks: &[cat_protocol::ResourceStackSnapshot], kind: ResourceKind) -> bool {
    stacks
        .iter()
        .any(|stack| stack.kind == kind && stack.amount > f64::EPSILON)
}

fn exact_weapon_instances(
    colony: &ColonySnapshot,
) -> impl Iterator<Item = &cat_protocol::ItemInstanceSnapshot> {
    colony
        .items
        .iter()
        .filter(is_metal_weapon_stack)
        .flat_map(|stack| stack.instances.iter())
}

fn is_metal_weapon_stack(stack: &&ItemStackSnapshot) -> bool {
    stack.kind == "weapon" && stack.material == "metal"
}

#[cfg(test)]
mod websocket_journeys {
    use std::sync::{Arc, Mutex};

    use cat_protocol::{ClientAction, ProductionQueueEdit};
    use cat_sim::{
        entities::CatNeeds,
        ledger::StockLedger,
        officers::OfficerRole,
        policy::{LeaderPolicyBucket, bucket_from_leadership, pick_policy_tier},
        research_catalog::research_catalog,
        station_recipes::{SMELTER_RECIPE_ID, SMITHY_WEAPON_RECIPE_ID},
        types::{BuildingType, CatSpecialization, PolicyTier},
        world_tick::{
            BuildingRuntime, ProductionQueueEntry, TilePos, WorldState, reconcile_colony_stockpiles,
        },
    };

    use crate::playtest_harness::{
        FailureTrace, SignedActor, WsClient, WsGameHarness, write_failure_trace,
    };

    use super::*;

    const NICKNAME: &str = "Forge Playtester";
    const SESSION: &str = "forge-playtest-installation";
    const TICK_CADENCE_MS: i64 = 5_000;

    #[derive(Clone, Debug)]
    struct ForgeActors {
        smelter_worker_id: String,
        smithy_worker_id: String,
        warrior_id: String,
        expected_weapon_id: String,
    }

    async fn accepted(
        client: &mut WsClient,
        action: ClientAction,
        milestone: &str,
    ) -> Result<(), String> {
        let observed = client.send_action(&action).await?;
        if observed.result.ok {
            Ok(())
        } else {
            Err(format!(
                "{milestone}: signed action rejected: {:?}; raw={}",
                observed.result.message, observed.raw
            ))
        }
    }

    fn signed_queue_action(
        actor: &SignedActor,
        building_id: &str,
        edit: ProductionQueueEdit,
    ) -> ClientAction {
        ClientAction::EditProductionQueue {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            building_id: building_id.to_owned(),
            edit,
        }
    }

    fn signed_assignment(actor: &SignedActor, cat_id: &str, building_id: &str) -> ClientAction {
        ClientAction::AssignWorker {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            cat_id: cat_id.to_owned(),
            building_id: Some(building_id.to_owned()),
        }
    }

    fn signed_equip(actor: &SignedActor, cat_id: &str, item_id: &str) -> ClientAction {
        ClientAction::EquipItem {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            cat_id: cat_id.to_owned(),
            item_id: item_id.to_owned(),
        }
    }

    fn make_survival_inert(world: &mut WorldState) {
        let colony = &mut world.colonies[0];
        colony.test_resource_decay_multiplier = 1.0;
        colony.resources.food = 500.0;
        colony.resources.water = 500.0;
        colony.jobs.clear();
        for cat in &mut colony.cats {
            cat.age_hours = 72.0;
            cat.needs = CatNeeds {
                hunger: 100.0,
                thirst: 100.0,
                rest: 100.0,
                health: 100.0,
            };
            cat.death_time = None;
            cat.activity = Default::default();
            cat.carrying = None;
            cat.destination = None;
            cat.current_task = None;
        }
    }

    fn add_station(
        world: &mut WorldState,
        id: &str,
        building_type: BuildingType,
        offset: i32,
        queue: Vec<ProductionQueueEntry>,
        paused: bool,
    ) {
        let colony = &mut world.colonies[0];
        let anchor = colony.anchor;
        colony.buildings.push(BuildingRuntime {
            id: id.to_owned(),
            building_type,
            position: TilePos {
                x: anchor.x + offset,
                y: anchor.y + 6,
            },
            is_complete: true,
            construction_progress: 100,
            production_queue: queue,
            production_paused: paused,
            ..BuildingRuntime::default()
        });
    }

    fn prepare_forge_fixture(world: &mut WorldState, captain_driven: bool) -> ForgeActors {
        make_survival_inert(world);
        let colony = &mut world.colonies[0];
        colony.resources.ore = 10.0;
        colony.resources.metal = 0.0;
        colony.upgrade_tree.owned_node_ids.extend(
            [
                "basic_tools",
                "metallurgy_preparation",
                "weaponsmithing",
                // The Captain office only survives the officer-pruning gate
                // with its prerequisite study and completed Barracks.
                "barracks",
            ]
            .map(str::to_owned),
        );
        let occupied = colony
            .buildings
            .iter()
            .flat_map(|building| {
                building
                    .assigned_cat
                    .iter()
                    .chain(
                        building
                            .additional_work_slots
                            .iter()
                            .map(|slot| &slot.assigned_cat),
                    )
                    .map(String::as_str)
            })
            .collect::<BTreeSet<_>>();
        let free_ids = colony
            .cats
            .iter()
            .rev()
            .filter(|cat| !occupied.contains(cat.id.as_str()))
            .map(|cat| cat.id.clone())
            .take(5)
            .collect::<Vec<_>>();
        assert_eq!(free_ids.len(), 5, "forge fixture needs five idle cats");
        let actors = ForgeActors {
            smelter_worker_id: free_ids[0].clone(),
            smithy_worker_id: free_ids[1].clone(),
            warrior_id: free_ids[2].clone(),
            expected_weapon_id: "item-0000000000000001".to_owned(),
        };
        assert!(
            colony.items.instances().next().is_none(),
            "forge fixture predicts the first deterministic item identity"
        );
        colony
            .cats
            .iter_mut()
            .find(|cat| cat.id == actors.warrior_id)
            .expect("forge warrior remains in fixture")
            .specialization = Some(CatSpecialization::Warrior);
        // The founding blueprint can carry its own warriors; the demand journey
        // contracts on ONE exact recipient, so strip rival specializations or
        // auto-issue may legitimately arm a different unarmed warrior first.
        for cat in colony.cats.iter_mut() {
            if cat.id != actors.warrior_id && cat.specialization == Some(CatSpecialization::Warrior)
            {
                cat.specialization = None;
            }
        }

        if captain_driven {
            colony
                .officers
                .insert(OfficerRole::Captain, free_ids[3].clone());
        }

        let (smelter_id, smithy_id) = if captain_driven {
            (CAPTAIN_SMELTER_ID, CAPTAIN_SMITHY_ID)
        } else {
            (PREPARED_SMELTER_ID, PREPARED_SMITHY_ID)
        };
        add_station(
            world,
            smelter_id,
            BuildingType::Smelter,
            6,
            if captain_driven {
                vec![ProductionQueueEntry {
                    recipe_id: SMELTER_RECIPE_ID.to_owned(),
                    repeat: true,
                }]
            } else {
                Vec::new()
            },
            !captain_driven,
        );
        add_station(
            world,
            smithy_id,
            BuildingType::Smithy,
            10,
            Vec::new(),
            !captain_driven,
        );

        if !captain_driven {
            add_station(
                world,
                "playtest-accounting-tent",
                BuildingType::AccountingTent,
                0,
                Vec::new(),
                false,
            );
            let colony = &mut world.colonies[0];
            colony
                .officers
                .insert(OfficerRole::Accountant, free_ids[3].clone());
            let tent = colony
                .buildings
                .iter_mut()
                .find(|building| building.id == "playtest-accounting-tent")
                .expect("prepared route accounting fixture");
            tent.assigned_cat = Some(free_ids[3].clone());
            tent.automated_by = Some(OfficerRole::Accountant);
        }

        if captain_driven {
            // The Captain office needs its prerequisite Barracks to survive the
            // officer-pruning gate.
            add_station(
                world,
                "playtest-captain-barracks",
                BuildingType::Barracks,
                14,
                Vec::new(),
                false,
            );
            add_station(
                world,
                "playtest-captain-accounting",
                BuildingType::AccountingTent,
                18,
                Vec::new(),
                false,
            );
        }
        let colony = &mut world.colonies[0];
        if captain_driven {
            // The socket projection redacts exact equipment identities unless a
            // staffed Accounting Tent keeps the stock ledger exact — and this
            // journey is observed through that projection, so it needs a clerk.
            colony
                .officers
                .insert(OfficerRole::Accountant, free_ids[4].clone());
            let tent = colony
                .buildings
                .iter_mut()
                .find(|building| building.id == "playtest-captain-accounting")
                .expect("captain accounting fixture");
            tent.assigned_cat = Some(free_ids[4].clone());
            tent.automated_by = Some(OfficerRole::Accountant);
        }
        reconcile_colony_stockpiles(colony);
        colony.stock_ledger = StockLedger::counted_with_piles(
            &colony.resources,
            &colony.stockpiles,
            colony.last_tick,
        );
        actors
    }

    fn traced_error(
        harness: &WsGameHarness,
        client: &WsClient,
        scenario: &'static ScenarioSpec,
        last_completed: Option<&'static str>,
        failure: String,
    ) -> String {
        let trace = FailureTrace {
            scenario_id: scenario.id,
            seed: harness.seed,
            last_completed_milestone: last_completed,
            simulated_time_ms: harness.now_ms(),
            action_results: &client.action_results,
            snapshot: client.snapshot(),
            restart_difference: None,
            failure: &failure,
        };
        let path = write_failure_trace(&trace).ok();
        format!("{failure}; trace={path:?}")
    }

    async fn run_prepared_weapon(seed: u32) -> Result<(), String> {
        let actors_out = Arc::new(Mutex::new(None));
        let setup_actors = Arc::clone(&actors_out);
        let mut harness = WsGameHarness::start_with(seed, move |world| {
            *setup_actors.lock().expect("forge actor fixture lock") =
                Some(prepare_forge_fixture(world, false));
        })
        .await?;
        let actors = actors_out
            .lock()
            .expect("forge actor fixture lock")
            .clone()
            .expect("forge setup records actors");
        let (mut client, actor) = harness.connect_authenticated(SESSION, NICKNAME).await?;

        for (building_id, recipe_id, repeat) in [
            (PREPARED_SMELTER_ID, SMELTER_RECIPE_ID, true),
            (PREPARED_SMITHY_ID, SMITHY_WEAPON_RECIPE_ID, false),
        ] {
            accepted(
                &mut client,
                signed_queue_action(
                    &actor,
                    building_id,
                    ProductionQueueEdit::Add {
                        recipe_id: recipe_id.to_owned(),
                        repeat,
                    },
                ),
                "signed-queues-accepted",
            )
            .await
            .map_err(|error| traced_error(&harness, &client, &SCENARIOS[0], None, error))?;
        }
        for (cat_id, building_id) in [
            (actors.smelter_worker_id.as_str(), PREPARED_SMELTER_ID),
            (actors.smithy_worker_id.as_str(), PREPARED_SMITHY_ID),
        ] {
            accepted(
                &mut client,
                signed_assignment(&actor, cat_id, building_id),
                "signed-workers-accepted",
            )
            .await
            .map_err(|error| {
                traced_error(
                    &harness,
                    &client,
                    &SCENARIOS[0],
                    Some("signed-queues-accepted"),
                    error,
                )
            })?;
        }

        harness.save().await?;
        client = harness.restart_and_reconnect(client, &actor).await?;
        for building_id in [PREPARED_SMELTER_ID, PREPARED_SMITHY_ID] {
            accepted(
                &mut client,
                signed_queue_action(
                    &actor,
                    building_id,
                    ProductionQueueEdit::SetPaused { paused: false },
                ),
                "restart paused queue resume",
            )
            .await?;
        }

        let mut evidence = WeaponRouteEvidence::default();
        harness
            .eventually(
                &mut client,
                SCENARIOS[0].horizon_ms,
                TICK_CADENCE_MS,
                |snapshot| {
                    evidence.observe(
                        snapshot,
                        PREPARED_SMELTER_ID,
                        PREPARED_SMITHY_ID,
                        &actors.warrior_id,
                    );
                    if selected_colony(snapshot)
                        .is_some_and(|colony| colony.resources.weapons >= 1.0)
                    {
                        evidence.stored_weapon_id = Some(actors.expected_weapon_id.clone());
                    }
                    evidence.production_complete()
                },
            )
            .await
            .map_err(|error| {
                let failure = format!(
                    "{error}; missing={:?}; evidence={evidence:?}",
                    evidence.missing_milestones()
                );
                traced_error(
                    &harness,
                    &client,
                    &SCENARIOS[0],
                    Some("signed-workers-accepted"),
                    failure,
                )
            })?;

        let weapon_id = evidence
            .stored_weapon_id
            .clone()
            .ok_or_else(|| "weapon-stored milestone had no exact identity".to_owned())?;
        accepted(
            &mut client,
            signed_equip(&actor, &actors.warrior_id, &weapon_id),
            "signed-equip-accepted",
        )
        .await?;
        client = harness.restart_and_reconnect(client, &actor).await?;
        accepted(
            &mut client,
            ClientAction::UnequipItem {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                cat_id: actors.warrior_id.clone(),
                item_id: weapon_id.clone(),
            },
            "restart-preserved-exact-equipped-identity",
        )
        .await
        .map_err(|error| {
            traced_error(
                &harness,
                &client,
                &SCENARIOS[0],
                Some("signed-equip-accepted"),
                error,
            )
        })?;
        accepted(
            &mut client,
            signed_equip(&actor, &actors.warrior_id, &weapon_id),
            "exact-warrior-re-equipped",
        )
        .await?;
        Ok(())
    }

    async fn run_captain_demand(seed: u32) -> Result<(), String> {
        let actors_out = Arc::new(Mutex::new(None));
        let setup_actors = Arc::clone(&actors_out);
        let mut harness = WsGameHarness::start_with(seed, move |world| {
            *setup_actors.lock().expect("captain actor fixture lock") =
                Some(prepare_forge_fixture(world, true));
        })
        .await?;
        let actors = actors_out
            .lock()
            .expect("captain actor fixture lock")
            .clone()
            .expect("captain setup records actors");
        let (mut client, _actor) = harness
            .connect_authenticated("captain-demand-installation", "Demand Captain")
            .await?;

        let colony = selected_colony(client.snapshot())
            .ok_or_else(|| "Captain precondition has no selected colony".to_owned())?;
        let warrior = colony
            .cats
            .iter()
            .find(|cat| cat.id == actors.warrior_id)
            .ok_or_else(|| "Captain precondition lost the exact warrior".to_owned())?;
        if warrior.equipment.weapon_item_id.is_some()
            || exact_weapon_instances(colony).next().is_some()
        {
            return Err("Captain demand fixture unexpectedly starts with a Weapon".to_owned());
        }
        if captain_selected_weapon_recipe(client.snapshot()) {
            return Err(
                "Captain demand fixture unexpectedly starts with a selected recipe".to_owned(),
            );
        }

        harness
            .eventually(
                &mut client,
                SCENARIOS[1].horizon_ms,
                60_000,
                captain_selected_weapon_recipe,
            )
            .await
            .map_err(|error| {
                traced_error(
                    &harness,
                    &client,
                    &SCENARIOS[1],
                    Some("captain-observes-shortage"),
                    format!(
                        "{error}; Captain never selected {WEAPON_RECIPE_ID} for warrior {}",
                        actors.warrior_id
                    ),
                )
            })?;

        let mut evidence = WeaponRouteEvidence::default();
        harness
            .eventually(
                &mut client,
                SCENARIOS[1].horizon_ms,
                TICK_CADENCE_MS,
                |snapshot| {
                    evidence.observe(
                        snapshot,
                        CAPTAIN_SMELTER_ID,
                        CAPTAIN_SMITHY_ID,
                        &actors.warrior_id,
                    );
                    evidence.equipment_complete()
                },
            )
            .await
            .map_err(|error| {
                traced_error(
                    &harness,
                    &client,
                    &SCENARIOS[1],
                    Some("weapon-recipe-selected"),
                    format!("{error}; evidence={evidence:?}"),
                )
            })?;
        Ok(())
    }

    fn leader(snapshot: &WorldSnapshot) -> Result<(&str, f64), String> {
        let leader = selected_colony(snapshot)
            .and_then(|colony| colony.leader.as_ref())
            .ok_or_else(|| "selected colony has no living Leader".to_owned())?;
        Ok((leader.id.as_str(), leader.leadership))
    }

    async fn run_leader_tenure(seed: u32) -> Result<(), String> {
        let mut harness = WsGameHarness::start_with(seed, |world| {
            make_survival_inert(world);
            let colony = &mut world.colonies[0];
            // Tenure/aging use game time, while physical need decay remains on the
            // real elapsed clock. This keeps the 101-game-hour threshold journey
            // bounded without bypassing the authoritative tick path.
            colony.test_time_scale = 100.0;
            let leader_id = colony.cats[0].id.clone();
            for cat in &mut colony.cats[1..] {
                cat.stats.leadership = 1.0;
            }
            colony.cats[0].stats.leadership = 34.9;
            colony.leader_id = Some(leader_id);
            reconcile_colony_stockpiles(colony);
            colony.stock_ledger = StockLedger::counted_with_piles(
                &colony.resources,
                &colony.stockpiles,
                colony.last_tick,
            );
        })
        .await?;
        let (mut client, actor) = harness
            .connect_authenticated("leader-tenure-installation", "Tenured Leader")
            .await?;
        let (leader_id, initial) = leader(client.snapshot())?;
        let leader_id = leader_id.to_owned();
        if initial >= 35.0 || bucket_from_leadership(initial) != LeaderPolicyBucket::Bad {
            return Err(format!("below-35-control invalid: leadership={initial}"));
        }
        if pick_policy_tier(initial, 0.2) != PolicyTier::Simple {
            return Err("controlled bad-bucket policy roll did not select Simple".to_owned());
        }

        let crossed_35 = harness
            .eventually(
                &mut client,
                2 * 60 * 60 * 1_000,
                30 * 60 * 1_000,
                |snapshot| {
                    leader(snapshot)
                        .is_ok_and(|(id, leadership)| id == leader_id && leadership >= 35.0)
                },
            )
            .await?;
        let (_, normal_leadership) = leader(&crossed_35)?;
        if bucket_from_leadership(normal_leadership) != LeaderPolicyBucket::Normal
            || pick_policy_tier(normal_leadership, 0.2) != PolicyTier::Normal
        {
            return Err(format!(
                "normal-bucket-policy not exposed after tenure: leadership={normal_leadership}"
            ));
        }

        let crossed_70 = harness
            .eventually(
                &mut client,
                SCENARIOS[2].horizon_ms,
                5 * 60 * 1_000,
                |snapshot| {
                    leader(snapshot)
                        .is_ok_and(|(id, leadership)| id == leader_id && leadership >= 70.0)
                },
            )
            .await
            .map_err(|error| {
                format!(
                    "{error}; expected leader={leader_id}, observed={:?}",
                    leader(client.snapshot())
                )
            })?;
        let (_, excellent_leadership) = leader(&crossed_70)?;
        if bucket_from_leadership(excellent_leadership) != LeaderPolicyBucket::Excellent
            || pick_policy_tier(excellent_leadership, 0.8) != PolicyTier::Excellent
        {
            return Err(format!(
                "excellent-bucket-policy not exposed after tenure: leadership={excellent_leadership}"
            ));
        }

        let before_restart = selected_colony(client.snapshot())
            .and_then(|colony| colony.leader.clone())
            .ok_or_else(|| "Leader vanished before tenure restart".to_owned())?;
        client = harness.restart_and_reconnect(client, &actor).await?;
        let after_restart = selected_colony(client.snapshot())
            .and_then(|colony| colony.leader.as_ref())
            .ok_or_else(|| "Leader vanished after tenure restart".to_owned())?;
        if after_restart != &before_restart {
            return Err("Leader identity/leadership changed across restart".to_owned());
        }
        Ok(())
    }

    async fn run_leader_daily_research(seed: u32) -> Result<(), String> {
        let target = "weaponsmithing";
        let mut harness = WsGameHarness::start_with(seed, move |world| {
            make_survival_inert(world);
            let colony = &mut world.colonies[0];
            colony.leader_id = Some(colony.cats[0].id.clone());
            let node = research_catalog()
                .get(target)
                .expect("daily research target remains catalogued");
            colony.upgrade_tree.owned_node_ids = research_catalog()
                .nodes()
                .iter()
                .filter(|candidate| candidate.id != node.id)
                .map(|candidate| candidate.id.clone())
                .collect();
            colony.upgrade_tree.research_points = node.cost;
            colony.last_leader_research_choice_at = Some(colony.last_tick);
            add_station(
                world,
                "playtest-research-smithy",
                BuildingType::Smithy,
                8,
                vec![ProductionQueueEntry {
                    recipe_id: SMITHY_WEAPON_RECIPE_ID.to_owned(),
                    repeat: false,
                }],
                false,
            );
            let colony = &mut world.colonies[0];
            reconcile_colony_stockpiles(colony);
            colony.stock_ledger = StockLedger::counted_with_piles(
                &colony.resources,
                &colony.stockpiles,
                colony.last_tick,
            );
        })
        .await?;
        let (mut client, actor) = harness
            .connect_authenticated("leader-research-installation", "Research Leader")
            .await?;
        let before = client.snapshot().clone();
        if affordable_target(&before) != Some(target) {
            return Err(format!(
                "affordable-target-visible expected {target}, got {:?}",
                affordable_target(&before)
            ));
        }
        let smithy_before = selected_colony(&before)
            .and_then(|colony| building(colony, "playtest-research-smithy"))
            .ok_or_else(|| "daily research fixture lost Smithy".to_owned())?;
        if smithy_before.production_block_reason.as_deref() != Some("research_locked") {
            return Err(format!(
                "before-boundary capability was not locked: {:?}",
                smithy_before.production_block_reason
            ));
        }

        let boundary_interval = 24 * 60 * 60 * 1_000;
        let cadence = 5 * 60 * 1_000;
        let target_now = harness.now_ms() + boundary_interval - 1;
        while harness.now_ms() < target_now {
            let step = cadence.min(target_now - harness.now_ms());
            harness.advance_by(&mut client, step).await?;
        }
        let just_before = client.snapshot().clone();
        if selected_colony(&just_before)
            .is_some_and(|colony| colony.research.owned_node_ids.iter().any(|id| id == target))
        {
            return Err("Leader unlocked study before exact daily boundary".to_owned());
        }
        let at_boundary = harness.advance_by(&mut client, 1).await?;
        let new = newly_owned_studies(&before, &at_boundary);
        if new != [target] {
            return Err(format!(
                "daily Leader choice changed {:?}, expected [{target}]",
                new
            ));
        }
        let smithy_after = selected_colony(&at_boundary)
            .and_then(|colony| building(colony, "playtest-research-smithy"))
            .ok_or_else(|| "daily research result lost Smithy".to_owned())?;
        if !smithy_after
            .available_recipes
            .iter()
            .any(|recipe| recipe == SMITHY_WEAPON_RECIPE_ID)
            || smithy_after.production_block_reason.as_deref() == Some("research_locked")
        {
            return Err(format!(
                "capability-exposed failed: available={:?}, blocked={:?}",
                smithy_after.available_recipes, smithy_after.production_block_reason
            ));
        }

        let owned_count = selected_colony(&at_boundary)
            .map(|colony| colony.research.owned_node_ids.len())
            .unwrap_or_default();
        let same_day = harness.advance_by(&mut client, 60_000).await?;
        if selected_colony(&same_day).map(|colony| colony.research.owned_node_ids.len())
            != Some(owned_count)
        {
            return Err("same-day tick unlocked a second Leader study".to_owned());
        }

        let before_restart = selected_colony(&same_day)
            .cloned()
            .ok_or_else(|| "daily research selected colony missing".to_owned())?;
        client = harness.restart_and_reconnect(client, &actor).await?;
        let after_restart = selected_colony(client.snapshot())
            .ok_or_else(|| "daily research colony missing after restart".to_owned())?;
        if after_restart != &before_restart {
            return Err("daily research ownership/capability changed across restart".to_owned());
        }
        Ok(())
    }

    #[tokio::test]
    async fn prepared_queue_runs_physical_weapon_chain_and_equips_exact_warrior() {
        let mut failures = Vec::new();
        for &seed in super::super::requested_seed_tier().seeds() {
            if let Err(error) = run_prepared_weapon(seed).await {
                failures.push(format!("seed {seed}: {error}"));
            }
        }
        assert!(
            failures.is_empty(),
            "prepared physical Weapon journey failures:\n{}",
            failures.join("\n")
        );
    }

    /// Ordinary red expectation: do not ignore or weaken this when the Captain has
    /// not yet learned to turn defender shortage into a selected Smithy recipe.
    #[tokio::test]
    async fn captain_demand_selects_and_delivers_weapon_to_exact_warrior() {
        let mut failures = Vec::new();
        for &seed in super::super::requested_seed_tier().seeds() {
            if let Err(error) = run_captain_demand(seed).await {
                failures.push(format!("seed {seed}: {error}"));
            }
        }
        assert!(
            failures.is_empty(),
            "Captain demand-driven Weapon journey failures:\n{}",
            failures.join("\n")
        );
    }

    #[tokio::test]
    async fn real_leader_tenure_crosses_existing_policy_thresholds() {
        run_leader_tenure(super::super::PRIMARY_SEED)
            .await
            .expect("Leader tenure/policy journey");
    }

    #[tokio::test]
    async fn daily_leader_choice_unlocks_one_affordable_capability_and_persists() {
        run_leader_daily_research(super::super::PRIMARY_SEED)
            .await
            .expect("Leader daily research journey");
    }
}

#[test]
fn manifest_keeps_mechanical_captain_and_leader_expectations_separate() {
    assert_eq!(SCENARIOS.len(), 4);
    let ids = SCENARIOS
        .iter()
        .map(|scenario| scenario.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), SCENARIOS.len());
    assert!(ids.contains("prepared-ore-smelter-smithy-weapon-equip"));
    assert!(ids.contains("captain-weapon-demand-to-exact-warrior"));
    assert!(ids.contains("leader-tenure-crosses-policy-boundaries"));
    assert!(ids.contains("leader-daily-research-exposes-capability"));
    assert_eq!(SCENARIOS[0].seed_tier, SeedTier::HighRisk);
    assert_eq!(SCENARIOS[1].seed_tier, SeedTier::HighRisk);
    assert_eq!(SCENARIOS[2].seed_tier, SeedTier::Primary);
    assert_eq!(SCENARIOS[3].seed_tier, SeedTier::Primary);
}

#[test]
fn milestone_contract_pins_decision_before_mechanics_and_no_leader_xp() {
    let captain = &SCENARIOS[1];
    let captain_ids = captain
        .milestones
        .iter()
        .map(|milestone| milestone.id)
        .collect::<Vec<_>>();
    let decision = captain_ids
        .iter()
        .position(|id| *id == "weapon-recipe-selected")
        .unwrap();
    let mechanics = captain_ids
        .iter()
        .position(|id| *id == "captain-chain-runs")
        .unwrap();
    assert!(decision < mechanics);

    let tenure = &SCENARIOS[2];
    assert!(
        tenure
            .milestones
            .iter()
            .any(|milestone| milestone.id == "crosses-35")
    );
    assert!(
        tenure
            .milestones
            .iter()
            .any(|milestone| milestone.id == "crosses-70")
    );
    assert!(
        !tenure
            .milestones
            .iter()
            .any(|milestone| milestone.id.contains("xp"))
    );
}

#[test]
fn evidence_reports_every_unobserved_physical_boundary() {
    let missing = WeaponRouteEvidence::default().missing_milestones();
    assert_eq!(
        missing,
        [
            "ore-inbound",
            "ore-at-smelter",
            "metal-at-smelter",
            "metal-outbound",
            "metal-at-smithy",
            "weapon-at-smithy",
            "weapon-outbound",
            "weapon-stored",
            "exact-warrior-equipped",
        ]
    );
}

#[test]
fn recipe_constants_match_the_typed_runtime_descriptors() {
    assert_eq!(
        SMELTER_RECIPE_ID,
        cat_sim::station_recipes::SMELTER_RECIPE_ID
    );
    assert_eq!(
        WEAPON_RECIPE_ID,
        cat_sim::station_recipes::SMITHY_WEAPON_RECIPE_ID
    );
}
