//! Executable real-WebSocket sweeps for constructible buildings and all maintained
//! physical station recipes.
//!
//! Every entry runs in an isolated deterministic world. Failures are collected only
//! after the complete typed catalog has run, and each failing entry writes its own
//! trace. Fixture mutation ends before the listener starts; planning, staffing,
//! queueing, ticks, save, restart, and observations all cross the real socket path.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, OnceLock},
};

use cat_protocol::{
    BuildingType as ProtoBuildingType, ClientAction, ItemLocation, ProductionQueueEdit,
    ResourceAmounts as ProtoResourceAmounts, ResourceKind as ProtoResourceKind, StationCompartment,
    TilePoint, WorldSnapshot,
};
use cat_sim::{
    actions::{ActionCtx, apply_action},
    biomes::MaxResources,
    climate::Biome,
    entities::{CatNeeds, MapType, Position},
    items::{Item, ItemKind, ItemLocation as SimItemLocation, MAX_QUALITY, Material},
    ledger::StockLedger,
    officers::OfficerRole,
    research_catalog::research_catalog,
    station_recipes::{StationRecipeDescriptor, station_recipe_set},
    stockpiles::{ResourceKind, Stockpile, set_resource},
    terrain_gen::{WORLD_TERRAIN_OPTIONS, generate_terrain_chunk, tile_climate_biome},
    types::{BuildingType, TileType},
    village_area::{from_tiles, gate_placement_default},
    village_layout::GridPos,
    world_gen::{TileResources, natural_deposits_for_biome},
    world_tick::{
        BuildingRuntime, TilePos, WorldState, WorldTileRuntime, publish_colony_spatial,
        reconcile_colony_stockpiles,
    },
    zones::normalize_rect,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::playtest_harness::{
    FailureTrace, SignedActor, WsClient, WsGameHarness, write_failure_trace,
};

pub(crate) const EXECUTABLE_SCENARIO_IDS: &[&str] = &[
    "every-building-plan-build-staff-operate-persist",
    "every-recipe-conserved-station-work-and-delivery",
    "every-crop-designate-grow-yield-persist",
    "every-finite-deposit-extract-carry-deplete-persist",
    "every-item-variant-persist-and-equipment-actions",
    "every-resource-storage-roundtrip-and-transport-families",
];

const CROP_MILESTONES: &[&str] = &[
    "authenticated",
    "signed-output-stockpile-accepted",
    "signed-field-staffing-accepted",
    "signed-designation-accepted",
    "plot-observed",
    "physical-work-observed",
    "growth-observed",
    "yield-in-transit",
    "signed-gather-haul-accepted",
    "yield-delivered",
    "restart-state-matched",
];
const ITEM_MILESTONES: &[&str] = &[
    "authenticated",
    "all-variants-visible",
    "tool-equipped",
    "weapon-equipped",
    "armor-equipped",
    "all-equipment-unequipped",
    "restart-state-matched",
];
const RESOURCE_MILESTONES: &[&str] = &[
    "authenticated",
    "all-resource-values-visible",
    "physical-stockpile-values-visible",
    "restart-state-matched",
];
const DEPOSIT_MILESTONES: &[&str] = &[
    "authenticated",
    "signed-extraction-accepted",
    "job-observed",
    "physical-carry-observed",
    "storage-delivery-observed",
    "finite-deposit-depleted",
    "restart-state-matched",
];

#[derive(Debug, Clone, Copy)]
struct DepositSpec {
    biome: Biome,
    resource: ResourceKind,
}

const FINITE_DEPOSITS: &[DepositSpec] = &[
    DepositSpec {
        biome: Biome::Mountains,
        resource: ResourceKind::Gem,
    },
    DepositSpec {
        biome: Biome::Badlands,
        resource: ResourceKind::Clay,
    },
    DepositSpec {
        biome: Biome::Swamp,
        resource: ResourceKind::Clay,
    },
    DepositSpec {
        biome: Biome::Marsh,
        resource: ResourceKind::Clay,
    },
    DepositSpec {
        biome: Biome::Beach,
        resource: ResourceKind::Sand,
    },
    DepositSpec {
        biome: Biome::Desert,
        resource: ResourceKind::Sand,
    },
];

const CONSTRUCTIBLE_BUILDINGS: &[BuildingType] = &[
    BuildingType::Den,
    BuildingType::FoodStorage,
    BuildingType::WaterBowl,
    BuildingType::Beds,
    BuildingType::HerbGarden,
    BuildingType::Nursery,
    BuildingType::ElderCorner,
    BuildingType::Walls,
    BuildingType::MouseFarm,
    BuildingType::Workshop,
    BuildingType::Field,
    BuildingType::Smithy,
    BuildingType::Barracks,
    BuildingType::AccountingTent,
    BuildingType::WoodCutter,
    BuildingType::StonePrep,
    BuildingType::Woodworking,
    BuildingType::Clothier,
    BuildingType::Tannery,
    BuildingType::ResearchHut,
    BuildingType::Smelter,
    BuildingType::Mill,
    BuildingType::Sawmill,
    BuildingType::School,
];
const FIELD_PARCEL_RADIUS: i32 = 24;
const BUILDING_STAFFED_MILESTONES: &[&str] = &[
    "authenticated",
    "signed-plan-accepted",
    "scaffold-seen",
    "physical-inputs-seen",
    "construction-complete",
    "signed-staffing-accepted",
    "staffing-observed",
    "operation-observed",
    "restart-state-matched",
];
const BUILDING_UNSTAFFED_MILESTONES: &[&str] = &[
    "authenticated",
    "signed-plan-accepted",
    "scaffold-seen",
    "physical-inputs-seen",
    "construction-complete",
    "restart-state-matched",
];
const RECIPE_MILESTONES: &[&str] = &[
    "authenticated",
    "signed-queue-accepted",
    "signed-staffing-accepted",
    "input-inbound",
    "input-station-local",
    "work-begun",
    "input-conserved",
    "output-station-local",
    "output-outbound",
    "output-delivered",
    "restart-state-matched",
];

#[derive(Debug)]
struct MilestoneTracker {
    ordered: &'static [&'static str],
    completed: usize,
}

impl MilestoneTracker {
    const fn new(ordered: &'static [&'static str]) -> Self {
        Self {
            ordered,
            completed: 0,
        }
    }

    fn complete(&mut self, milestone: &'static str) {
        assert_eq!(
            self.ordered.get(self.completed).copied(),
            Some(milestone),
            "catalog milestones must complete in declared order"
        );
        self.completed += 1;
    }

    fn complete_if(&mut self, milestone: &'static str, condition: bool) {
        if condition && self.ordered.get(self.completed).copied() == Some(milestone) {
            self.completed += 1;
        }
    }

    fn last_completed(&self) -> Option<&'static str> {
        self.completed
            .checked_sub(1)
            .and_then(|index| self.ordered.get(index).copied())
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildingEvidence {
    signed_plan_accepted: bool,
    scaffold_seen: bool,
    physical_inputs_seen: bool,
    completed: bool,
    staffing_applicable: bool,
    signed_staffing_accepted: bool,
    staffing_observed: bool,
    operation_queue_seen: bool,
    operation_effect_seen: bool,
    restart_persisted: bool,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecipeEvidence {
    signed_queue_accepted: bool,
    signed_staffing_accepted: bool,
    initial_input_total: BTreeMap<String, f64>,
    input_inbound_peak: BTreeMap<String, f64>,
    input_local_peak: BTreeMap<String, f64>,
    input_local_after_work: BTreeMap<String, f64>,
    input_consumed: BTreeMap<String, f64>,
    conserved_input: bool,
    work_seen: bool,
    initial_output_total: BTreeMap<String, f64>,
    output_local_peak: BTreeMap<String, f64>,
    output_outbound_peak: BTreeMap<String, f64>,
    delivered_output: BTreeMap<String, f64>,
    output_item_id: Option<String>,
    item_local_seen: bool,
    item_outbound_seen: bool,
    signed_exact_id_verified: bool,
    delivered: bool,
    restart_persisted: bool,
}

#[derive(Debug)]
struct EntryFailure {
    id: String,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildingOperationFamily {
    None,
    Research,
    Accounting,
    Field,
    MouseFarm,
    ProductionStation,
}

fn building_operation_family(building_type: BuildingType) -> BuildingOperationFamily {
    match building_type {
        BuildingType::ResearchHut | BuildingType::School => BuildingOperationFamily::Research,
        BuildingType::AccountingTent => BuildingOperationFamily::Accounting,
        BuildingType::Field => BuildingOperationFamily::Field,
        BuildingType::MouseFarm => BuildingOperationFamily::MouseFarm,
        _ if station_recipe_set(building_type).is_some() => {
            BuildingOperationFamily::ProductionStation
        }
        _ => BuildingOperationFamily::None,
    }
}

fn sync_building_milestones(evidence: &BuildingEvidence, milestones: &mut MilestoneTracker) {
    milestones.complete_if("signed-plan-accepted", evidence.signed_plan_accepted);
    milestones.complete_if("scaffold-seen", evidence.scaffold_seen);
    milestones.complete_if("physical-inputs-seen", evidence.physical_inputs_seen);
    milestones.complete_if("construction-complete", evidence.completed);
    if evidence.staffing_applicable {
        milestones.complete_if(
            "signed-staffing-accepted",
            evidence.signed_staffing_accepted,
        );
        milestones.complete_if("staffing-observed", evidence.staffing_observed);
        milestones.complete_if("operation-observed", evidence.operation_effect_seen);
    }
    milestones.complete_if("restart-state-matched", evidence.restart_persisted);
}

fn all_recipe_resources_seen(peaks: &BTreeMap<String, f64>, resources: &[ResourceKind]) -> bool {
    resources.iter().copied().all(|kind| {
        peaks
            .get(&resource_key(proto_resource(kind)))
            .copied()
            .unwrap_or(0.0)
            > 0.0
    })
}

fn sync_recipe_milestones(
    recipe: &StationRecipeDescriptor,
    evidence: &RecipeEvidence,
    milestones: &mut MilestoneTracker,
) {
    milestones.complete_if("signed-queue-accepted", evidence.signed_queue_accepted);
    milestones.complete_if(
        "signed-staffing-accepted",
        evidence.signed_staffing_accepted,
    );
    milestones.complete_if(
        "input-inbound",
        all_recipe_resources_seen(&evidence.input_inbound_peak, recipe.input_resources),
    );
    milestones.complete_if(
        "input-station-local",
        all_recipe_resources_seen(&evidence.input_local_peak, recipe.input_resources),
    );
    milestones.complete_if("work-begun", evidence.work_seen);
    milestones.complete_if("input-conserved", evidence.conserved_input);
    let output_local =
        if recipe.output_item.is_some() || recipe_has_finite_functional_output(recipe) {
            evidence.item_local_seen
        } else {
            all_recipe_resources_seen(&evidence.output_local_peak, recipe.output_resources)
        };
    milestones.complete_if("output-station-local", output_local);
    let output_outbound =
        if recipe.output_item.is_some() || recipe_has_finite_functional_output(recipe) {
            evidence.item_outbound_seen
        } else {
            all_recipe_resources_seen(&evidence.output_outbound_peak, recipe.output_resources)
        };
    milestones.complete_if("output-outbound", output_outbound);
    milestones.complete_if("output-delivered", evidence.delivered);
    milestones.complete_if("restart-state-matched", evidence.restart_persisted);
}

fn restart_difference(before: &Value, after: &Value) -> Option<Value> {
    (before != after).then(|| json!({ "before": before, "after": after }))
}

fn prepare_common(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.jobs.clear();
    colony.test_time_scale = 100.0;
    colony.upgrade_tree.owned_node_ids = research_catalog()
        .nodes()
        .iter()
        .map(|node| node.id.clone())
        .collect();
    colony.upgrade_tree.research_points = 100_000.0;
    colony.resources.food = 500.0;
    colony.resources.water = 500.0;
    colony.resources.logs = 100.0;
    colony.resources.lumber = 100.0;
    colony.resources.planks = 100.0;
    colony.resources.blocks = 100.0;
    colony.resources.materials = 100.0;
    colony.resources.stone = 100.0;
    for kind in ResourceKind::ALL {
        if *kind != ResourceKind::Blessings {
            set_resource(&mut colony.resources, *kind, 100.0);
        }
    }
    for cat in &mut colony.cats {
        cat.age_hours = 72.0;
        cat.needs = CatNeeds {
            hunger: 100.0,
            thirst: 100.0,
            rest: 100.0,
            health: 100.0,
        };
        cat.activity = Default::default();
        cat.current_task = None;
        cat.destination = None;
        cat.carrying = None;
        cat.death_time = None;
    }
    for building in &mut colony.buildings {
        building.assigned_cat = None;
        building.automated_by = None;
        for slot in &mut building.additional_work_slots {
            slot.assigned_cat.clear();
            slot.automated_by = None;
        }
    }
    colony.officers.clear();
    reconcile_colony_stockpiles(colony);
}

fn prepare_building(world: &mut WorldState, building_type: BuildingType) {
    prepare_common(world);
    if building_type != BuildingType::Field {
        return;
    }
    let colony = &mut world.colonies[0];
    let steward_id = colony.cats[0].id.clone();
    colony.officers.insert(OfficerRole::Steward, steward_id);
    for dx in -FIELD_PARCEL_RADIUS..=FIELD_PARCEL_RADIUS {
        for dy in -FIELD_PARCEL_RADIUS..=FIELD_PARCEL_RADIUS {
            let tile = TilePos {
                x: colony.anchor.x + dx,
                y: colony.anchor.y + dy,
            };
            if !colony.claimed_tiles.contains(&tile) {
                colony.claimed_tiles.push(tile);
            }
            colony.revealed_tiles.insert(tile);
            // The parcel must be deterministic building ground: overwrite any
            // generated terrain so leftover trees, rocks, or water cannot
            // reject every probed field site.
            colony.world_tiles.insert(
                tile,
                WorldTileRuntime {
                    pos: tile,
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
                },
            );
        }
    }
    for index in 0..24 {
        colony.buildings.push(BuildingRuntime {
            id: format!("field-level-fixture-{index}"),
            building_type: BuildingType::Walls,
            position: TilePos {
                x: colony.anchor.x + 100 + index,
                y: colony.anchor.y + 100,
            },
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        });
    }
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
    let gate = gate_placement_default(&area).expect("field fixture claim has a gate");
    let mut road = TilePos {
        x: colony.anchor.x + 1,
        y: colony.anchor.y + 1,
    };
    while road.x != gate.x {
        road.x += (gate.x - road.x).signum();
        if let Some(tile) = colony.world_tiles.get_mut(&road) {
            tile.overlay_feature = Some("road_built".to_owned());
            tile.path_wear = 100;
        }
    }
    while road.y != gate.y {
        road.y += (gate.y - road.y).signum();
        if let Some(tile) = colony.world_tiles.get_mut(&road) {
            tile.overlay_feature = Some("road_built".to_owned());
            tile.path_wear = 100;
        }
    }
    publish_colony_spatial(&mut world.shared_spatial, &world.colonies[0]);
}

fn field_designation_candidates(anchor: TilePoint) -> Vec<TilePoint> {
    let offsets = [0, -1, 1, -2, 2, -3, 3, -6, 6, -9, 9, -12, 12, -15, 15, -18];
    offsets
        .into_iter()
        .flat_map(|offset| {
            [
                TilePoint {
                    x: anchor.x + offset,
                    y: anchor.y - FIELD_PARCEL_RADIUS,
                },
                TilePoint {
                    x: anchor.x + FIELD_PARCEL_RADIUS,
                    y: anchor.y + offset,
                },
                TilePoint {
                    x: anchor.x + offset,
                    y: anchor.y + FIELD_PARCEL_RADIUS,
                },
                TilePoint {
                    x: anchor.x - FIELD_PARCEL_RADIUS,
                    y: anchor.y + offset,
                },
            ]
        })
        .collect()
}

fn find_valid_farm_site(world: &WorldState) -> Result<TilePoint, String> {
    let colony = &world.colonies[0];
    let actor = SignedActor {
        session_id: "catalog-fixture-probe".to_owned(),
        nickname: "Catalog Fixture Probe".to_owned(),
        sig: "server-verified".to_owned(),
        player_id: "catalog-fixture-probe".to_owned(),
    };
    let ctx = ActionCtx {
        session_id: actor.session_id.clone(),
        player_id: "catalog-fixture-probe".to_owned(),
        colony_id: colony.id.clone(),
        now_ms: colony.last_tick,
    };
    let mut reasons = BTreeMap::<String, usize>::new();
    for candidate in field_designation_candidates(TilePoint {
        x: colony.anchor.x,
        y: colony.anchor.y,
    }) {
        let mut probe = world.clone();
        let result = apply_action(
            &mut probe,
            &signed_designate_farm(&actor, candidate, cat_protocol::CropKind::Grain),
            &ctx,
        );
        if result.ok {
            return Ok(candidate);
        }
        *reasons
            .entry(
                result
                    .message
                    .unwrap_or_else(|| "rejected without message".to_owned()),
            )
            .or_default() += 1;
    }
    Err(format!(
        "no action-legal farm designation site in field fixture; rejection histogram={reasons:?}"
    ))
}

fn find_valid_stockpile_site(
    world: &WorldState,
    accepts: Vec<ProtoResourceKind>,
    preferred_origin: Option<TilePoint>,
) -> Result<(TilePoint, TilePoint), String> {
    let colony = &world.colonies[0];
    let ctx = ActionCtx {
        session_id: "catalog-fixture-probe".to_owned(),
        player_id: "catalog-fixture-probe".to_owned(),
        colony_id: colony.id.clone(),
        now_ms: colony.last_tick,
    };
    let mut candidates = colony.claimed_tiles.clone();
    let origin = preferred_origin.unwrap_or(TilePoint {
        x: colony.anchor.x,
        y: colony.anchor.y,
    });
    candidates.sort_by_key(|site| (site.x - origin.x).abs().max((site.y - origin.y).abs()));
    let mut reasons = BTreeMap::<String, usize>::new();
    for site in candidates {
        let point = TilePoint {
            x: site.x,
            y: site.y,
        };
        let opposite = TilePoint {
            x: site.x + 1,
            y: site.y + 1,
        };
        if preferred_origin.is_some_and(|origin| {
            (site.x - 1..=opposite.x + 1).contains(&origin.x)
                && (site.y - 1..=opposite.y + 1).contains(&origin.y)
        }) {
            continue;
        }
        let mut probe = world.clone();
        let result = apply_action(
            &mut probe,
            &ClientAction::DesignateStockpile {
                session_id: ctx.session_id.clone(),
                nickname: "Catalog fixture probe".to_owned(),
                sig: "fixture".to_owned(),
                a: point,
                b: opposite,
                accepts: accepts.clone(),
            },
            &ctx,
        );
        if result.ok {
            return Ok((point, opposite));
        }
        *reasons
            .entry(
                result
                    .message
                    .unwrap_or_else(|| "unknown rejection".to_owned()),
            )
            .or_default() += 1;
    }
    Err(format!(
        "no signed-action-legal stockpile site; rejection histogram={reasons:?}"
    ))
}

fn find_valid_player_site(
    world: &WorldState,
    building_type: BuildingType,
) -> Result<TilePoint, String> {
    let colony = &world.colonies[0];
    let ctx = ActionCtx {
        session_id: "catalog-fixture-probe".to_owned(),
        player_id: "catalog-fixture-probe".to_owned(),
        colony_id: colony.id.clone(),
        now_ms: colony.last_tick,
    };
    let mut candidates = if building_type == BuildingType::Field {
        colony
            .world_tiles
            .iter()
            .filter(|(_, tile)| tile.overlay_feature.as_deref() == Some("road_built"))
            .filter(|(road, _)| {
                let distance = (road.x - colony.anchor.x)
                    .abs()
                    .max((road.y - colony.anchor.y).abs());
                (10..=20).contains(&distance)
            })
            .flat_map(|(road, _)| {
                [
                    TilePos {
                        x: road.x + 1,
                        y: road.y,
                    },
                    TilePos {
                        x: road.x - 2,
                        y: road.y,
                    },
                    TilePos {
                        x: road.x,
                        y: road.y + 1,
                    },
                    TilePos {
                        x: road.x,
                        y: road.y - 3,
                    },
                ]
            })
            .collect::<Vec<_>>()
    } else {
        colony.claimed_tiles.clone()
    };
    candidates.sort_by_key(|site| {
        (site.x - colony.anchor.x)
            .abs()
            .max((site.y - colony.anchor.y).abs())
    });
    candidates.truncate(64);
    let mut reasons = BTreeMap::<String, usize>::new();
    for site in candidates.into_iter().map(|site| TilePoint {
        x: site.x,
        y: site.y,
    }) {
        let mut probe = world.clone();
        let result = apply_action(
            &mut probe,
            &ClientAction::PlanBuilding {
                session_id: ctx.session_id.clone(),
                nickname: "Catalog fixture probe".to_owned(),
                sig: "fixture".to_owned(),
                building_type: sim_to_proto_building(building_type),
                site: Some(site),
            },
            &ctx,
        );
        if result.ok {
            return Ok(site);
        }
        *reasons
            .entry(
                result
                    .message
                    .unwrap_or_else(|| "unknown rejection".to_owned()),
            )
            .or_default() += 1;
    }
    Err(format!(
        "no valid site; action rejection histogram={reasons:?}"
    ))
}

fn sim_to_proto_building(kind: BuildingType) -> ProtoBuildingType {
    match kind {
        BuildingType::Den => ProtoBuildingType::Den,
        BuildingType::FoodStorage => ProtoBuildingType::FoodStorage,
        BuildingType::WaterBowl => ProtoBuildingType::WaterBowl,
        BuildingType::Beds => ProtoBuildingType::Beds,
        BuildingType::HerbGarden => ProtoBuildingType::HerbGarden,
        BuildingType::Nursery => ProtoBuildingType::Nursery,
        BuildingType::ElderCorner => ProtoBuildingType::ElderCorner,
        BuildingType::Walls => ProtoBuildingType::Walls,
        BuildingType::MouseFarm => ProtoBuildingType::MouseFarm,
        BuildingType::Shrine => ProtoBuildingType::Shrine,
        BuildingType::Workshop => ProtoBuildingType::Workshop,
        BuildingType::Field => ProtoBuildingType::Field,
        BuildingType::ResearchHut => ProtoBuildingType::ResearchHut,
        BuildingType::School => ProtoBuildingType::School,
        BuildingType::Smithy => ProtoBuildingType::Smithy,
        BuildingType::Barracks => ProtoBuildingType::Barracks,
        BuildingType::AccountingTent => ProtoBuildingType::AccountingTent,
        BuildingType::WoodCutter => ProtoBuildingType::WoodCutter,
        BuildingType::StonePrep => ProtoBuildingType::StonePrep,
        BuildingType::Woodworking => ProtoBuildingType::Woodworking,
        BuildingType::Clothier => ProtoBuildingType::Clothier,
        BuildingType::Tannery => ProtoBuildingType::Tannery,
        BuildingType::Smelter => ProtoBuildingType::Smelter,
        BuildingType::Mill => ProtoBuildingType::Mill,
        BuildingType::Sawmill => ProtoBuildingType::Sawmill,
    }
}

fn proto_resource(kind: ResourceKind) -> ProtoResourceKind {
    serde_json::from_value(serde_json::to_value(kind).expect("serialize simulation resource"))
        .expect("simulation and protocol resource wire names stay aligned")
}

fn resource_key(kind: ProtoResourceKind) -> String {
    serde_json::to_value(kind)
        .expect("serialize protocol resource")
        .as_str()
        .expect("resource serializes as a string")
        .to_owned()
}

fn resource_amount(resources: &ProtoResourceAmounts, kind: ProtoResourceKind) -> f64 {
    let key = resource_key(kind);
    serde_json::to_value(resources)
        .expect("serialize protocol resources")
        .get(&key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

fn stack_amount(stacks: &[cat_protocol::ResourceStackSnapshot], kind: ProtoResourceKind) -> f64 {
    stacks
        .iter()
        .filter(|stack| stack.kind == kind)
        .map(|stack| stack.amount)
        .sum()
}

fn record_peak(map: &mut BTreeMap<String, f64>, kind: ProtoResourceKind, amount: f64) {
    let peak = map.entry(resource_key(kind)).or_default();
    *peak = peak.max(amount);
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-6 * left.abs().max(right.abs()).max(1.0)
}

fn selected(snapshot: &WorldSnapshot) -> Option<&cat_protocol::ColonySnapshot> {
    snapshot
        .selected_colony_id
        .as_deref()
        .and_then(|id| snapshot.colonies.iter().find(|colony| colony.id == id))
        .or_else(|| snapshot.colonies.first())
}

fn signed_plan(
    actor: &SignedActor,
    building_type: BuildingType,
    site: Option<TilePoint>,
) -> ClientAction {
    ClientAction::PlanBuilding {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        building_type: sim_to_proto_building(building_type),
        site,
    }
}

fn signed_assign(actor: &SignedActor, cat_id: &str, building_id: &str) -> ClientAction {
    ClientAction::AssignWorker {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        cat_id: cat_id.to_owned(),
        building_id: Some(building_id.to_owned()),
    }
}

fn signed_queue(actor: &SignedActor, building_id: &str, recipe_id: &str) -> ClientAction {
    ClientAction::EditProductionQueue {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        building_id: building_id.to_owned(),
        edit: ProductionQueueEdit::Add {
            recipe_id: recipe_id.to_owned(),
            repeat: false,
        },
    }
}

fn signed_designate_farm(
    actor: &SignedActor,
    site: TilePoint,
    crop: cat_protocol::CropKind,
) -> ClientAction {
    ClientAction::DesignateFarm {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        a: site,
        b: site,
        crop,
    }
}

async fn require_action(
    client: &mut WsClient,
    action: ClientAction,
    label: &str,
) -> Result<(), String> {
    let observed = client.send_action(&action).await?;
    if observed.result.ok {
        Ok(())
    } else {
        Err(format!(
            "{label} rejected: {:?}; raw={}",
            observed.result.message, observed.raw
        ))
    }
}

fn available_worker(snapshot: &WorldSnapshot) -> Option<&str> {
    let colony = selected(snapshot)?;
    let assigned = colony
        .buildings
        .iter()
        .flat_map(|building| building.work_slots.iter().map(|slot| slot.cat_id.as_str()))
        .collect::<BTreeSet<_>>();
    colony
        .cats
        .iter()
        .find(|cat| {
            cat.death_time.is_none()
                && cat.activity == cat_protocol::CatActivity::Idle
                && cat.current_task.is_none()
                && cat.carrying.is_none()
                && cat.destination.is_none()
                && !assigned.contains(cat.id.as_str())
        })
        .map(|cat| cat.id.as_str())
}

#[derive(Debug)]
struct BuildingOperationBaseline {
    research_points: f64,
    last_counted: i64,
    food: f64,
    recipe_id: Option<&'static str>,
}

fn building_operation_seen(
    colony: &cat_protocol::ColonySnapshot,
    building: &cat_protocol::BuildingSnapshot,
    building_type: BuildingType,
    baseline: &BuildingOperationBaseline,
    operation_queue_seen: bool,
) -> bool {
    match building_operation_family(building_type) {
        BuildingOperationFamily::None => true,
        BuildingOperationFamily::Research => {
            colony.research.research_points > baseline.research_points
        }
        BuildingOperationFamily::Accounting => colony.stock_ledger.as_ref().is_some_and(|ledger| {
            ledger.active_round.is_some() || ledger.last_counted > baseline.last_counted
        }),
        BuildingOperationFamily::Field => colony.farms.iter().any(|farm| {
            farm.growth_hours > 0.0
                || farm.work_phase != cat_protocol::FarmWorkPhase::WaitingForWorker
        }),
        BuildingOperationFamily::MouseFarm => {
            !building.output_inventory.is_empty()
                || !building.outbound_cargo.is_empty()
                || building.outbound_haul > f64::EPSILON
                || colony.resources.food > baseline.food
        }
        BuildingOperationFamily::ProductionStation => {
            let queued_recipe_completed = operation_queue_seen
                && baseline.recipe_id.is_some_and(|recipe_id| {
                    building
                        .production_queue
                        .iter()
                        .all(|entry| entry.recipe_id != recipe_id)
                });
            building.production_progress > 0.0
                || !building.output_inventory.is_empty()
                || !building.outbound_cargo.is_empty()
                || building.outbound_haul > f64::EPSILON
                || queued_recipe_completed
        }
    }
}

fn output_resource_projection(
    colony: &cat_protocol::ColonySnapshot,
    recipe: Option<&StationRecipeDescriptor>,
) -> BTreeMap<String, f64> {
    recipe
        .into_iter()
        .flat_map(|recipe| recipe.output_resources.iter().copied())
        .map(proto_resource)
        .map(|kind| (resource_key(kind), resource_amount(&colony.resources, kind)))
        .collect()
}

fn building_restart_projection(
    snapshot: &WorldSnapshot,
    building_id: &str,
    building_type: BuildingType,
    recipe: Option<&StationRecipeDescriptor>,
) -> Value {
    let Some(colony) = selected(snapshot) else {
        return json!({ "missingColony": true });
    };
    let Some(building) = colony
        .buildings
        .iter()
        .find(|building| building.id == building_id)
    else {
        return json!({ "missingBuilding": building_id });
    };
    let operation = match building_operation_family(building_type) {
        BuildingOperationFamily::None => Value::Null,
        BuildingOperationFamily::Research => {
            json!({ "researchPoints": colony.research.research_points })
        }
        BuildingOperationFamily::Accounting => json!({ "stockLedger": colony.stock_ledger }),
        BuildingOperationFamily::Field => json!({ "farms": colony.farms }),
        BuildingOperationFamily::MouseFarm => json!({
            "food": colony.resources.food,
            "outputInventory": building.output_inventory,
            "outboundCargo": building.outbound_cargo,
            "outboundHaul": building.outbound_haul,
        }),
        BuildingOperationFamily::ProductionStation => json!({
            "queue": building.production_queue,
            "progress": building.production_progress,
            "inputInventory": building.input_inventory,
            "inboundCargo": building.inbound_cargo,
            "outputInventory": building.output_inventory,
            "outboundCargo": building.outbound_cargo,
            "inboundHaul": building.inbound_haul,
            "outboundHaul": building.outbound_haul,
            "outputResources": output_resource_projection(colony, recipe),
            "items": colony.items,
        }),
    };
    json!({
        "id": building.id,
        "type": building.building_type,
        "constructionProgress": building.construction_progress,
        "staffCount": building.staff_count,
        "workSlots": building.work_slots,
        "operation": operation,
    })
}

fn recipe_restart_projection(snapshot: &WorldSnapshot, recipe: &StationRecipeDescriptor) -> Value {
    building_restart_projection(
        snapshot,
        "catalog-recipe-station",
        recipe.building_type,
        Some(recipe),
    )
}

fn queues_are_cyclically_equivalent(
    before: &[cat_protocol::ProductionQueueEntrySnapshot],
    after: &[cat_protocol::ProductionQueueEntrySnapshot],
) -> bool {
    before.len() == after.len()
        && (before.is_empty()
            || (0..before.len()).any(|offset| {
                before
                    .iter()
                    .enumerate()
                    .all(|(index, entry)| entry == &after[(index + offset) % after.len()])
            }))
}

fn write_entry_failure(
    harness: &WsGameHarness,
    client: &WsClient,
    scenario_prefix: &str,
    entry_id: &str,
    milestones: &MilestoneTracker,
    restart_difference: Option<&Value>,
    message: &str,
) -> String {
    let scenario_id = format!("{scenario_prefix}-{entry_id}").replace('_', "-");
    let trace = FailureTrace {
        scenario_id: &scenario_id,
        seed: harness.seed,
        last_completed_milestone: milestones.last_completed(),
        simulated_time_ms: harness.now_ms(),
        action_results: &client.action_results,
        snapshot: client.snapshot(),
        restart_difference,
        failure: message,
    };
    let path = write_failure_trace(&trace).ok();
    format!("{message}; trace={path:?}")
}

async fn run_building_entry(
    seed: u32,
    building_type: BuildingType,
) -> Result<BuildingEvidence, EntryFailure> {
    let expects_staffing = cat_sim::production::building_staff_cap(building_type) > 0;
    let mut milestones = MilestoneTracker::new(if expects_staffing {
        BUILDING_STAFFED_MILESTONES
    } else {
        BUILDING_UNSTAFFED_MILESTONES
    });
    let mut restart_difference_value = None;
    let site_out = Arc::new(Mutex::new(None::<Result<TilePoint, String>>));
    let farm_site_out = Arc::new(Mutex::new(None::<Result<TilePoint, String>>));
    let setup_site = Arc::clone(&site_out);
    let setup_farm_site = Arc::clone(&farm_site_out);
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_building(world, building_type);
        *setup_site.lock().expect("building site fixture lock") =
            Some(find_valid_player_site(world, building_type));
        if building_type == BuildingType::Field {
            *setup_farm_site.lock().expect("farm site fixture lock") =
                Some(find_valid_farm_site(world));
        }
    })
    .await
    .map_err(|message| EntryFailure {
        id: building_type.as_str().to_owned(),
        message,
    })?;
    let (mut client, actor) = harness
        .connect_authenticated(
            format!("building-catalog-{}", building_type.as_str()),
            "Building Catalog",
        )
        .await
        .map_err(|message| EntryFailure {
            id: building_type.as_str().to_owned(),
            message,
        })?;
    milestones.complete("authenticated");
    let site = site_out
        .lock()
        .expect("building site fixture lock")
        .clone()
        .expect("building fixture records site result");
    let id = building_type.as_str().to_owned();
    let existing = selected(client.snapshot())
        .into_iter()
        .flat_map(|colony| colony.buildings.iter())
        .map(|building| building.id.clone())
        .collect::<BTreeSet<_>>();
    let mut evidence = BuildingEvidence::default();
    let site = match site {
        Ok(site) => site,
        Err(message) => {
            return Err(EntryFailure {
                id,
                message: write_entry_failure(
                    &harness,
                    &client,
                    "building",
                    building_type.as_str(),
                    &milestones,
                    restart_difference_value.as_ref(),
                    &message,
                ),
            });
        }
    };
    if let Err(message) = require_action(
        &mut client,
        signed_plan(&actor, building_type, Some(site)),
        "signed plan",
    )
    .await
    {
        return Err(EntryFailure {
            id,
            message: write_entry_failure(
                &harness,
                &client,
                "building",
                building_type.as_str(),
                &milestones,
                restart_difference_value.as_ref(),
                &message,
            ),
        });
    }
    evidence.signed_plan_accepted = true;
    sync_building_milestones(&evidence, &mut milestones);

    let completed = harness
        .eventually(&mut client, 15 * 60_000, 5_000, |snapshot| {
            let new = selected(snapshot)
                .into_iter()
                .flat_map(|colony| colony.buildings.iter())
                .find(|building| {
                    building.building_type == sim_to_proto_building(building_type)
                        && !existing.contains(&building.id)
                });
            if let Some(building) = new {
                evidence.scaffold_seen = true;
                evidence.physical_inputs_seen |= !building.construction_delivered.is_empty()
                    || !building.construction_in_transit.is_empty();
                evidence.completed = building.construction_progress >= 100.0;
            }
            sync_building_milestones(&evidence, &mut milestones);
            evidence.completed
        })
        .await
        .map_err(|message| EntryFailure {
            id: id.clone(),
            message: write_entry_failure(
                &harness,
                &client,
                "building",
                building_type.as_str(),
                &milestones,
                restart_difference_value.as_ref(),
                &format!("{message}; evidence={evidence:?}"),
            ),
        })?;
    let built = selected(&completed)
        .into_iter()
        .flat_map(|colony| colony.buildings.iter())
        .find(|building| {
            building.building_type == sim_to_proto_building(building_type)
                && !existing.contains(&building.id)
        })
        .cloned()
        .ok_or_else(|| EntryFailure {
            id: id.clone(),
            message: "completed building vanished".to_owned(),
        })?;

    evidence.staffing_applicable = built.staff_cap > 0;
    assert_eq!(
        evidence.staffing_applicable, expects_staffing,
        "wire staff applicability changed for {building_type:?}"
    );
    let operation_recipe = station_recipe_set(building_type).and_then(|set| set.recipes.first());
    let operation_snapshot;
    if evidence.staffing_applicable {
        let worker = available_worker(&completed)
            .ok_or_else(|| EntryFailure {
                id: id.clone(),
                message: "no idle worker after construction".to_owned(),
            })?
            .to_owned();
        require_action(
            &mut client,
            signed_assign(&actor, &worker, &built.id),
            "signed staffing",
        )
        .await
        .map_err(|message| EntryFailure {
            id: id.clone(),
            message,
        })?;
        evidence.signed_staffing_accepted = true;
        sync_building_milestones(&evidence, &mut milestones);
        let staffed = harness
            .eventually(&mut client, 120_000, 5_000, |snapshot| {
                selected(snapshot)
                    .into_iter()
                    .flat_map(|colony| colony.buildings.iter())
                    .find(|building| building.id == built.id)
                    .is_some_and(|building| building.staff_count > 0)
            })
            .await
            .map_err(|message| EntryFailure {
                id: id.clone(),
                message: write_entry_failure(
                    &harness,
                    &client,
                    "building",
                    building_type.as_str(),
                    &milestones,
                    restart_difference_value.as_ref(),
                    &message,
                ),
            })?;
        evidence.staffing_observed = true;
        sync_building_milestones(&evidence, &mut milestones);

        if building_type == BuildingType::Field {
            let farm_site = farm_site_out
                .lock()
                .expect("farm site fixture lock")
                .clone()
                .expect("field fixture records farm site")
                .map_err(|message| EntryFailure {
                    id: id.clone(),
                    message: write_entry_failure(
                        &harness,
                        &client,
                        "building",
                        building_type.as_str(),
                        &milestones,
                        restart_difference_value.as_ref(),
                        &message,
                    ),
                })?;
            require_action(
                &mut client,
                signed_designate_farm(&actor, farm_site, cat_protocol::CropKind::Grain),
                "signed farm designation",
            )
            .await
            .map_err(|message| EntryFailure {
                id: id.clone(),
                message: write_entry_failure(
                    &harness,
                    &client,
                    "building",
                    building_type.as_str(),
                    &milestones,
                    restart_difference_value.as_ref(),
                    &message,
                ),
            })?;
        }

        let colony = selected(&staffed).expect("staffed snapshot has selected colony");
        let baseline_research = colony.research.research_points;
        let baseline_counted = colony
            .stock_ledger
            .as_ref()
            .map_or(i64::MIN, |ledger| ledger.last_counted);
        let baseline_food = colony.resources.food;
        let baseline = BuildingOperationBaseline {
            research_points: baseline_research,
            last_counted: baseline_counted,
            food: baseline_food,
            recipe_id: operation_recipe.map(|recipe| recipe.id),
        };
        if let Some(recipe) = operation_recipe {
            let queue_is_empty = colony
                .buildings
                .iter()
                .find(|building| building.id == built.id)
                .is_some_and(|building| building.production_queue.is_empty());
            if queue_is_empty {
                require_action(
                    &mut client,
                    signed_queue(&actor, &built.id, recipe.id),
                    "signed operation queue",
                )
                .await
                .map_err(|message| EntryFailure {
                    id: id.clone(),
                    message,
                })?;
                evidence.operation_queue_seen = selected(client.snapshot())
                    .into_iter()
                    .flat_map(|colony| colony.buildings.iter())
                    .find(|building| building.id == built.id)
                    .is_some_and(|building| {
                        building
                            .production_queue
                            .iter()
                            .any(|entry| entry.recipe_id == recipe.id)
                    });
            }
        }
        operation_snapshot = harness
            .eventually(&mut client, 15 * 60_000, 1_000, |snapshot| {
                let Some(colony) = selected(snapshot) else {
                    return false;
                };
                let Some(building) = colony
                    .buildings
                    .iter()
                    .find(|building| building.id == built.id)
                else {
                    return false;
                };
                evidence.operation_queue_seen |= baseline.recipe_id.is_some_and(|recipe_id| {
                    building
                        .production_queue
                        .iter()
                        .any(|entry| entry.recipe_id == recipe_id)
                });
                evidence.operation_effect_seen = building_operation_seen(
                    colony,
                    building,
                    building_type,
                    &baseline,
                    evidence.operation_queue_seen,
                );
                sync_building_milestones(&evidence, &mut milestones);
                evidence.operation_effect_seen
            })
            .await
            .map_err(|message| EntryFailure {
                id: id.clone(),
                message: write_entry_failure(
                    &harness,
                    &client,
                    "building",
                    building_type.as_str(),
                    &milestones,
                    restart_difference_value.as_ref(),
                    &format!("{message}; no post-staff operation effect; evidence={evidence:?}"),
                ),
            })?;
    } else {
        operation_snapshot = completed;
    }

    let before_restart = building_restart_projection(
        &operation_snapshot,
        &built.id,
        building_type,
        operation_recipe,
    );
    client = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: id.clone(),
            message,
        })?;
    let after_restart = building_restart_projection(
        client.snapshot(),
        &built.id,
        building_type,
        operation_recipe,
    );
    evidence.restart_persisted = before_restart == after_restart;
    restart_difference_value = (!evidence.restart_persisted)
        .then(|| json!({ "before": before_restart.clone(), "after": after_restart.clone() }));
    sync_building_milestones(&evidence, &mut milestones);
    if !evidence.scaffold_seen
        || !evidence.physical_inputs_seen
        || !evidence.completed
        || (evidence.staffing_applicable && !evidence.operation_effect_seen)
        || !evidence.restart_persisted
    {
        let message = format!("building lifecycle incomplete: {evidence:?}");
        return Err(EntryFailure {
            id,
            message: write_entry_failure(
                &harness,
                &client,
                "building",
                building_type.as_str(),
                &milestones,
                restart_difference_value.as_ref(),
                &message,
            ),
        });
    }
    Ok(evidence)
}

fn all_recipes() -> Vec<&'static StationRecipeDescriptor> {
    BuildingType::ALL
        .iter()
        .copied()
        .filter_map(station_recipe_set)
        .flat_map(|set| set.recipes)
        .collect()
}

fn recipe_has_finite_functional_output(recipe: &StationRecipeDescriptor) -> bool {
    recipe.output_resources.iter().any(|kind| {
        matches!(
            kind,
            ResourceKind::Tools | ResourceKind::Weapons | ResourceKind::Armor
        )
    })
}

fn add_recipe_station(
    world: &mut WorldState,
    recipe: &StationRecipeDescriptor,
    worker_out: &Arc<Mutex<Option<String>>>,
) {
    prepare_common(world);
    let colony = &mut world.colonies[0];
    colony.test_time_scale = 1.0;
    for input in recipe.input_resources {
        set_resource(&mut colony.resources, *input, 100.0);
    }
    for output in recipe.output_resources {
        if !recipe.input_resources.contains(output) {
            set_resource(&mut colony.resources, *output, 0.0);
        }
    }
    let worker = colony
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none())
        .expect("recipe fixture has worker")
        .id
        .clone();
    assert!(
        colony.items.instances().next().is_none(),
        "recipe fixture predicts the first deterministic item identity"
    );
    *worker_out.lock().expect("recipe worker fixture lock") = Some(worker.clone());
    let accountant = colony
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none() && cat.id != worker)
        .expect("recipe fixture has a separate accountant")
        .id
        .clone();
    colony.buildings.push(BuildingRuntime {
        id: "catalog-recipe-station".to_owned(),
        building_type: recipe.building_type,
        position: TilePos {
            x: colony.anchor.x + 8,
            y: colony.anchor.y + 8,
        },
        is_complete: true,
        construction_progress: 100,
        production_queue: Vec::new(),
        ..BuildingRuntime::default()
    });
    colony.buildings.push(BuildingRuntime {
        id: "catalog-accounting-tent".to_owned(),
        building_type: BuildingType::AccountingTent,
        position: TilePos {
            x: colony.anchor.x,
            y: colony.anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        assigned_cat: Some(accountant.clone()),
        automated_by: Some(OfficerRole::Accountant),
        ..BuildingRuntime::default()
    });
    colony.officers.insert(OfficerRole::Accountant, accountant);
    reconcile_colony_stockpiles(colony);
    colony.stock_ledger =
        StockLedger::counted_with_piles(&colony.resources, &colony.stockpiles, colony.last_tick);
}

fn observe_recipe(
    snapshot: &WorldSnapshot,
    recipe: &StationRecipeDescriptor,
    evidence: &mut RecipeEvidence,
) {
    let Some(colony) = selected(snapshot) else {
        return;
    };
    let Some(station) = colony
        .buildings
        .iter()
        .find(|building| building.id == "catalog-recipe-station")
    else {
        return;
    };
    for input in recipe.input_resources.iter().copied().map(proto_resource) {
        record_peak(
            &mut evidence.input_inbound_peak,
            input,
            stack_amount(&station.inbound_cargo, input),
        );
        record_peak(
            &mut evidence.input_local_peak,
            input,
            stack_amount(&station.input_inventory, input),
        );
    }
    evidence.work_seen |= station.production_progress > 0.0;
    for output in recipe.output_resources.iter().copied().map(proto_resource) {
        record_peak(
            &mut evidence.output_local_peak,
            output,
            stack_amount(&station.output_inventory, output),
        );
        record_peak(
            &mut evidence.output_outbound_peak,
            output,
            stack_amount(&station.outbound_cargo, output),
        );
        let key = resource_key(output);
        let initial = evidence
            .initial_output_total
            .get(&key)
            .copied()
            .unwrap_or(0.0);
        record_peak(
            &mut evidence.delivered_output,
            output,
            (resource_amount(&colony.resources, output) - initial).max(0.0),
        );
    }

    if let Some(output) = recipe.output_item {
        evidence.item_local_seen |= colony.events.iter().any(|event| {
            event.kind == "production"
                && event.message.contains(output.kind.as_str())
                && event.message.contains(output.material.as_str())
                && event.message.contains("await haulage")
        });
        evidence.item_outbound_seen |= evidence.item_local_seen
            && (station.outbound_haul > 0.0
                || !station.outbound_cargo.is_empty()
                || colony.cats.iter().any(|cat| {
                    cat.carrying.as_ref().is_some_and(|cargo| {
                        cargo
                            .item_ids
                            .iter()
                            .any(|id| Some(id.as_str()) == evidence.output_item_id.as_deref())
                    })
                }));
        for stack in &colony.items {
            if stack.kind != output.kind.as_str()
                || stack.material != output.material.as_str()
                || stack.quality != output.quality
            {
                continue;
            }
            for instance in &stack.instances {
                if evidence.output_item_id.is_none() {
                    evidence.output_item_id = Some(instance.id.clone());
                }
                if evidence.output_item_id.as_deref() != Some(instance.id.as_str()) {
                    continue;
                }
                match &instance.location {
                    ItemLocation::Station {
                        building_id,
                        compartment: StationCompartment::LocalOutput,
                    } if building_id == "catalog-recipe-station" => {
                        evidence.item_local_seen = true;
                    }
                    ItemLocation::Carrier { .. } => evidence.item_outbound_seen = true,
                    ItemLocation::Stockpile { .. } if instance.credited => {
                        evidence.delivered = true;
                    }
                    _ => {}
                }
            }
        }
    }

    if recipe_has_finite_functional_output(recipe) {
        evidence.item_local_seen |= recipe
            .output_resources
            .iter()
            .copied()
            .map(proto_resource)
            .any(|kind| {
                evidence
                    .output_local_peak
                    .get(&resource_key(kind))
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
            });
        evidence.item_outbound_seen |= recipe
            .output_resources
            .iter()
            .copied()
            .map(proto_resource)
            .any(|kind| {
                evidence
                    .output_outbound_peak
                    .get(&resource_key(kind))
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
            });
    }

    let completed_work = evidence.work_seen && station.production_queue.is_empty();
    if completed_work {
        let mut all_inputs_conserved = true;
        for input in recipe.input_resources.iter().copied().map(proto_resource) {
            let key = resource_key(input);
            let local_after = stack_amount(&station.input_inventory, input);
            evidence
                .input_local_after_work
                .insert(key.clone(), local_after);
            let local_peak = evidence.input_local_peak.get(&key).copied().unwrap_or(0.0);
            let consumed = (local_peak - local_after).max(0.0);
            evidence.input_consumed.insert(key.clone(), consumed);
            let initial = evidence
                .initial_input_total
                .get(&key)
                .copied()
                .unwrap_or(0.0);
            let aggregate_consumed = (initial - resource_amount(&colony.resources, input)).max(0.0);
            all_inputs_conserved &= consumed > 0.0 && aggregate_consumed + 1.0e-6 >= consumed;
        }
        evidence.conserved_input |= all_inputs_conserved;
    }

    if recipe.output_item.is_none() && completed_work && evidence.conserved_input {
        let exact_output_route = recipe
            .output_resources
            .iter()
            .copied()
            .map(proto_resource)
            .all(|output| {
                let key = resource_key(output);
                let local = evidence.output_local_peak.get(&key).copied().unwrap_or(0.0);
                let outbound = evidence
                    .output_outbound_peak
                    .get(&key)
                    .copied()
                    .unwrap_or(0.0);
                local > 0.0 && outbound > 0.0 && approximately_equal(local, outbound)
            });
        let route_cleared = recipe
            .output_resources
            .iter()
            .copied()
            .map(proto_resource)
            .all(|output| {
                stack_amount(&station.output_inventory, output) <= f64::EPSILON
                    && stack_amount(&station.outbound_cargo, output) <= f64::EPSILON
            })
            && station.outbound_haul <= f64::EPSILON;
        if exact_output_route && route_cleared {
            for output in recipe.output_resources.iter().copied().map(proto_resource) {
                let key = resource_key(output);
                let amount = evidence
                    .output_outbound_peak
                    .get(&key)
                    .copied()
                    .unwrap_or(0.0);
                evidence.delivered_output.insert(key, amount);
            }
            evidence.delivered = true;
        }
    }
    if recipe.output_item.is_some()
        && completed_work
        && evidence.conserved_input
        && evidence.item_local_seen
        && evidence.item_outbound_seen
        && station.output_inventory.is_empty()
        && station.outbound_cargo.is_empty()
        && station.outbound_haul <= f64::EPSILON
    {
        evidence.delivered = true;
    }
}

async fn run_recipe_entry(
    seed: u32,
    recipe: &'static StationRecipeDescriptor,
) -> Result<RecipeEvidence, EntryFailure> {
    let mut milestones = MilestoneTracker::new(RECIPE_MILESTONES);
    let worker_out = Arc::new(Mutex::new(None));
    let setup_worker = Arc::clone(&worker_out);
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        add_recipe_station(world, recipe, &setup_worker);
    })
    .await
    .map_err(|message| EntryFailure {
        id: recipe.id.to_owned(),
        message,
    })?;
    let worker_id = worker_out
        .lock()
        .expect("recipe worker fixture lock")
        .clone()
        .expect("recipe fixture records worker");
    let (mut client, actor) = harness
        .connect_authenticated(format!("recipe-catalog-{}", recipe.id), "Recipe Catalog")
        .await
        .map_err(|message| EntryFailure {
            id: recipe.id.to_owned(),
            message,
        })?;
    milestones.complete("authenticated");
    let mut evidence = RecipeEvidence::default();
    let initial_colony = selected(client.snapshot()).expect("recipe snapshot has selected colony");
    for input in recipe.input_resources.iter().copied().map(proto_resource) {
        evidence.initial_input_total.insert(
            resource_key(input),
            resource_amount(&initial_colony.resources, input),
        );
    }
    for output in recipe.output_resources.iter().copied().map(proto_resource) {
        evidence.initial_output_total.insert(
            resource_key(output),
            resource_amount(&initial_colony.resources, output),
        );
    }
    if recipe.output_item.is_some() || recipe_has_finite_functional_output(recipe) {
        evidence.output_item_id = Some("item-0000000000000001".to_owned());
    }
    require_action(
        &mut client,
        signed_queue(&actor, "catalog-recipe-station", recipe.id),
        "signed queue",
    )
    .await
    .map_err(|message| EntryFailure {
        id: recipe.id.to_owned(),
        message,
    })?;
    evidence.signed_queue_accepted = true;
    sync_recipe_milestones(recipe, &evidence, &mut milestones);
    require_action(
        &mut client,
        signed_assign(&actor, &worker_id, "catalog-recipe-station"),
        "signed staffing",
    )
    .await
    .map_err(|message| EntryFailure {
        id: recipe.id.to_owned(),
        message,
    })?;
    evidence.signed_staffing_accepted = true;
    sync_recipe_milestones(recipe, &evidence, &mut milestones);

    let completion_error = harness
        .eventually(&mut client, 15 * 60_000, 5_000, |snapshot| {
            observe_recipe(snapshot, recipe, &mut evidence);
            sync_recipe_milestones(recipe, &evidence, &mut milestones);
            evidence.delivered
        })
        .await
        .err();
    // A missing delivery milestone must not skip the persistence checkpoint.
    // Restart the exact partial station/cargo state, retain the timeout as the
    // primary failure, and report any restart difference alongside it.
    let before_restart = recipe_restart_projection(client.snapshot(), recipe);
    let expected_item_id = evidence.output_item_id.clone();
    client = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: recipe.id.to_owned(),
            message,
        })?;
    let after_restart = recipe_restart_projection(client.snapshot(), recipe);
    let restart_state_valid = before_restart == after_restart;
    let mut restart_difference_value = (!restart_state_valid)
        .then(|| json!({ "before": before_restart.clone(), "after": after_restart.clone() }));
    if let Some(item_id) = expected_item_id.as_deref() {
        let observed = client
            .send_action(&ClientAction::RepairItem {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                item_id: item_id.to_owned(),
            })
            .await
            .map_err(|message| EntryFailure {
                id: recipe.id.to_owned(),
                message,
            })?;
        evidence.signed_exact_id_verified = !observed.result.ok
            && observed.result.message.as_deref() == Some("That item does not need repair.");
    }
    if expected_item_id.is_some() && !evidence.signed_exact_id_verified {
        restart_difference_value = Some(json!({
            "before": before_restart.clone(),
            "after": after_restart.clone(),
            "exactItemVerification": false,
            "expectedItemId": expected_item_id.clone(),
        }));
    }
    evidence.restart_persisted = restart_difference_value.is_none()
        && (expected_item_id.is_none() || evidence.signed_exact_id_verified);
    sync_recipe_milestones(recipe, &evidence, &mut milestones);
    let all_input_inbound =
        all_recipe_resources_seen(&evidence.input_inbound_peak, recipe.input_resources);
    let all_input_local =
        all_recipe_resources_seen(&evidence.input_local_peak, recipe.input_resources);
    let exact_item_lifecycle = evidence.output_item_id.is_none()
        || (evidence.output_item_id.is_some()
            && evidence.item_local_seen
            && evidence.item_outbound_seen
            && evidence.signed_exact_id_verified);
    if completion_error.is_some()
        || !all_input_inbound
        || !all_input_local
        || !evidence.conserved_input
        || !evidence.work_seen
        || !evidence.delivered
        || !exact_item_lifecycle
        || !evidence.restart_persisted
    {
        let message = format!(
            "{}; recipe physical lifecycle incomplete: {evidence:?}",
            completion_error
                .as_deref()
                .unwrap_or("all bounded delivery milestones completed")
        );
        return Err(EntryFailure {
            id: recipe.id.to_owned(),
            message: write_entry_failure(
                &harness,
                &client,
                "recipe",
                recipe.id,
                &milestones,
                restart_difference_value.as_ref(),
                &message,
            ),
        });
    }
    Ok(evidence)
}

fn aggregate_failures(label: &str, failures: Vec<EntryFailure>) {
    assert!(
        failures.is_empty(),
        "{label} ({} failures):\n{}",
        failures.len(),
        failures
            .into_iter()
            .map(|failure| format!("{}: {}", failure.id, failure.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn catalog_entry_selected(id: &str) -> bool {
    std::env::var("CAT_CATALOG_ENTRY").map_or(true, |filter| {
        filter.eq_ignore_ascii_case(id)
            || id
                .to_ascii_lowercase()
                .contains(&filter.to_ascii_lowercase())
    })
}

fn crop_resource(crop: cat_protocol::CropKind) -> ProtoResourceKind {
    match crop {
        cat_protocol::CropKind::Catnip => ProtoResourceKind::Catnip,
        cat_protocol::CropKind::Grain => ProtoResourceKind::Grain,
        cat_protocol::CropKind::Herb => ProtoResourceKind::Herbs,
    }
}

async fn run_crop_entry(seed: u32, crop: cat_protocol::CropKind) -> Result<(), EntryFailure> {
    let site_result = Arc::new(Mutex::new(None));
    let setup_site = Arc::clone(&site_result);
    let stockpile_result = Arc::new(Mutex::new(None));
    let setup_stockpile = Arc::clone(&stockpile_result);
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_building(world, BuildingType::Field);
        let site = find_valid_farm_site(world);
        *setup_stockpile.lock().expect("stockpile site lock") = Some(site.as_ref().map_or_else(
            |message| Err(message.clone()),
            |site| find_valid_stockpile_site(world, vec![crop_resource(crop)], Some(*site)),
        ));
        *setup_site.lock().expect("farm site lock") = Some(site);
        let colony = &mut world.colonies[0];
        colony.cats.truncate(4);
        colony.leader_id = Some(colony.cats[0].id.clone());
        for cat in &mut colony.cats {
            cat.age_hours = 24.0;
        }
        colony.buildings.push(BuildingRuntime {
            id: "catalog-crop-field".to_owned(),
            building_type: BuildingType::Field,
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        });
        let accountant_id = colony.cats[2].id.clone();
        colony
            .officers
            .insert(OfficerRole::Accountant, accountant_id.clone());
        colony.buildings.push(BuildingRuntime {
            id: "catalog-crop-accounting".to_owned(),
            building_type: BuildingType::AccountingTent,
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(accountant_id),
            ..BuildingRuntime::default()
        });
        let steward_id = colony.cats[3].id.clone();
        colony.officers.insert(OfficerRole::Steward, steward_id);
        for pile in &mut colony.stockpiles {
            for &kind in ResourceKind::ALL {
                set_resource(&mut pile.contents, kind, 0.0);
            }
        }
        for &kind in ResourceKind::ALL {
            set_resource(&mut colony.resources, kind, 0.0);
        }
        let general = colony
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
            .expect("crop fixture general stockpile");
        set_resource(&mut general.contents, ResourceKind::Food, 20.0);
        set_resource(&mut general.contents, ResourceKind::Water, 20.0);
        reconcile_colony_stockpiles(colony);
        colony.stock_ledger = StockLedger::counted_with_piles(
            &colony.resources,
            &colony.stockpiles,
            colony.last_tick,
        );
    })
    .await
    .map_err(|message| EntryFailure {
        id: format!("{crop:?}"),
        message,
    })?;
    let site = site_result
        .lock()
        .expect("farm site lock")
        .take()
        .expect("farm site setup ran")
        .map_err(|message| EntryFailure {
            id: format!("{crop:?}"),
            message,
        })?;
    let (stockpile_a, stockpile_b) = stockpile_result
        .lock()
        .expect("stockpile site lock")
        .take()
        .expect("stockpile site setup ran")
        .map_err(|message| EntryFailure {
            id: format!("{crop:?}"),
            message,
        })?;
    let (mut client, actor) = harness
        .connect_authenticated(format!("crop-{crop:?}"), "Crop Catalog")
        .await
        .map_err(|message| EntryFailure {
            id: format!("{crop:?}"),
            message,
        })?;
    let mut milestones = MilestoneTracker::new(CROP_MILESTONES);
    milestones.complete("authenticated");
    let kind = crop_resource(crop);
    require_action(
        &mut client,
        ClientAction::DesignateStockpile {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            a: stockpile_a,
            b: stockpile_b,
            accepts: vec![kind],
        },
        "designate crop output stockpile",
    )
    .await
    .map_err(|message| EntryFailure {
        id: format!("{crop:?}"),
        message,
    })?;
    milestones.complete("signed-output-stockpile-accepted");
    let worker_id = selected(client.snapshot())
        .and_then(|colony| colony.cats.get(1))
        .map(|cat| cat.id.clone())
        .ok_or_else(|| EntryFailure {
            id: format!("{crop:?}"),
            message: "no idle crop worker available".to_owned(),
        })?;
    require_action(
        &mut client,
        signed_assign(&actor, &worker_id, "catalog-crop-field"),
        "staff crop field",
    )
    .await
    .map_err(|message| EntryFailure {
        id: format!("{crop:?}"),
        message,
    })?;
    milestones.complete("signed-field-staffing-accepted");
    require_action(
        &mut client,
        signed_designate_farm(&actor, site, crop),
        "designate crop",
    )
    .await
    .map_err(|message| EntryFailure {
        id: format!("{crop:?}"),
        message,
    })?;
    milestones.complete("signed-designation-accepted");

    let mut plot_seen = false;
    let mut work_seen = false;
    let mut growth_seen = false;
    let mut transit_seen = false;
    let mut gather_spot_id = None;
    let gathered = harness
        .eventually(&mut client, 2 * 60 * 60 * 1_000, 5_000, |snapshot| {
            let Some(colony) = selected(snapshot) else {
                return false;
            };
            if let Some(plot) = colony.farms.iter().find(|plot| plot.crop == crop) {
                plot_seen = true;
                work_seen |= plot.worker_id.is_some()
                    && plot.work_phase != cat_protocol::FarmWorkPhase::WaitingForWorker;
                growth_seen |= plot.growth_hours > 0.0;
                transit_seen |= !plot.output_inventory.is_empty()
                    || plot.work_phase == cat_protocol::FarmWorkPhase::Hauling
                    || colony.cats.iter().any(|cat| cat.carrying.is_some());
                if plot.work_phase == cat_protocol::FarmWorkPhase::OutputBlocked {
                    gather_spot_id = colony
                        .stockpiles
                        .iter()
                        .find(|pile| pile.id.starts_with("farm-gather:"))
                        .map(|pile| pile.id.clone());
                }
            }
            gather_spot_id.is_some()
        })
        .await;
    milestones.complete_if("plot-observed", plot_seen);
    milestones.complete_if("physical-work-observed", work_seen);
    milestones.complete_if("growth-observed", growth_seen);
    milestones.complete_if("yield-in-transit", transit_seen && gathered.is_ok());
    if gathered.is_ok() {
        require_action(
            &mut client,
            ClientAction::HaulGatherSpot {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                stockpile_id: gather_spot_id.expect("gathered crop has a handoff"),
                cat_id: None,
            },
            "haul harvested crop from local handoff",
        )
        .await
        .map_err(|message| EntryFailure {
            id: format!("{crop:?}"),
            message,
        })?;
        milestones.complete("signed-gather-haul-accepted");
    }
    let completed = if gathered.is_ok() {
        harness
            .eventually(&mut client, 60 * 60 * 1_000, 5_000, |snapshot| {
                selected(snapshot).is_some_and(|colony| {
                    colony
                        .stockpiles
                        .iter()
                        .filter(|pile| !pile.id.starts_with("farm-gather:"))
                        .map(|pile| resource_amount(&pile.contents, kind))
                        .sum::<f64>()
                        > 0.0
                })
            })
            .await
    } else {
        gathered
    };
    milestones.complete_if("yield-delivered", completed.is_ok());

    let before = client.snapshot().clone();
    let restarted = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: format!("{crop:?}"),
            message,
        })?;
    let after = restarted.snapshot().clone();
    let projection = |snapshot: &WorldSnapshot| {
        let colony = selected(snapshot).expect("crop snapshot colony");
        json!({
            "farm": colony.farms.iter().find(|plot| plot.crop == crop),
            "physicalPiles": colony.stockpiles.iter().map(|pile| json!({
                "id": pile.id,
                "amount": resource_amount(&pile.contents, kind),
            })).collect::<Vec<_>>(),
        })
    };
    let before_projection = projection(&before);
    let after_projection = projection(&after);
    let difference = restart_difference(&before_projection, &after_projection);
    milestones.complete_if("restart-state-matched", difference.is_none());
    if completed.is_err() || milestones.last_completed() != Some("restart-state-matched") {
        let reason = completed
            .err()
            .unwrap_or_else(|| "crop lifecycle milestone missing".to_owned());
        return Err(EntryFailure {
            id: format!("{crop:?}"),
            message: write_entry_failure(
                &harness,
                &restarted,
                "crop",
                &format!("{crop:?}"),
                &milestones,
                difference.as_ref(),
                &reason,
            ),
        });
    }
    Ok(())
}

fn item_variant_key(kind: &str, material: &str, quality: u8) -> String {
    format!("{kind}:{material}:{quality}")
}

fn item_variant_projection(snapshot: &WorldSnapshot) -> BTreeMap<String, Vec<Value>> {
    let Some(colony) = selected(snapshot) else {
        return BTreeMap::new();
    };
    colony
        .items
        .iter()
        .map(|stack| {
            (
                item_variant_key(&stack.kind, &stack.material, stack.quality),
                stack
                    .instances
                    .iter()
                    .map(|instance| {
                        json!({
                            "id": instance.id,
                            "durability": instance.durability,
                            "maxDurability": instance.max_durability,
                            "broken": instance.broken,
                            "credited": instance.credited,
                            "location": instance.location,
                        })
                    })
                    .collect(),
            )
        })
        .collect()
}

async fn run_item_variant_catalog(seed: u32) -> Result<(), EntryFailure> {
    let damaged_ids = Arc::new(Mutex::new(BTreeMap::<ItemKind, String>::new()));
    let setup_ids = Arc::clone(&damaged_ids);
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_common(world);
        let colony = &mut world.colonies[0];
        let pile_id = colony
            .stockpiles
            .iter()
            .find(|pile| pile.is_general_storehouse())
            .expect("starter general stockpile")
            .id
            .clone();
        for &kind in ItemKind::ALL {
            for &material in Material::ALL {
                for quality in 0..=MAX_QUALITY {
                    colony.items.add_at(
                        Item::new(kind, material, quality),
                        1,
                        1.0,
                        SimItemLocation::Stockpile {
                            stockpile_id: pile_id.clone(),
                        },
                        true,
                    );
                }
            }
        }
        for kind in [ItemKind::Tool, ItemKind::Weapon, ItemKind::Armor] {
            colony.items.wear(kind, 1);
            let id = colony
                .items
                .instances()
                .find(|instance| instance.item.kind == kind && !instance.is_pristine())
                .expect("worn equipment identity")
                .id
                .clone();
            setup_ids.lock().expect("damaged id lock").insert(kind, id);
        }
        let worker_id = colony.cats[0].id.clone();
        for (index, building_type) in [
            BuildingType::Woodworking,
            BuildingType::Smithy,
            BuildingType::StonePrep,
            BuildingType::Workshop,
            BuildingType::Clothier,
            BuildingType::Tannery,
        ]
        .into_iter()
        .enumerate()
        {
            colony.buildings.push(BuildingRuntime {
                id: format!("catalog-repair-{index}"),
                building_type,
                is_complete: true,
                construction_progress: 100,
                assigned_cat: Some(worker_id.clone()),
                ..BuildingRuntime::default()
            });
        }
        reconcile_colony_stockpiles(colony);
        colony.stock_ledger = StockLedger::counted_with_piles(
            &colony.resources,
            &colony.stockpiles,
            colony.last_tick,
        );
    })
    .await
    .map_err(|message| EntryFailure {
        id: "item-variants".to_owned(),
        message,
    })?;
    let ids = damaged_ids.lock().expect("damaged id lock").clone();
    let (mut client, actor) = harness
        .connect_authenticated("item-catalog", "Item Catalog")
        .await
        .map_err(|message| EntryFailure {
            id: "item-variants".to_owned(),
            message,
        })?;
    let mut milestones = MilestoneTracker::new(ITEM_MILESTONES);
    milestones.complete("authenticated");
    let variants = item_variant_projection(client.snapshot());
    let expected_count = ItemKind::ALL.len() * Material::ALL.len() * (usize::from(MAX_QUALITY) + 1);
    let mut variant_failures = Vec::new();
    for &kind in ItemKind::ALL {
        for &material in Material::ALL {
            for quality in 0..=MAX_QUALITY {
                let key = item_variant_key(kind.as_str(), material.as_str(), quality);
                if variants.get(&key).map_or(0, Vec::len) != 1 {
                    variant_failures.push(key);
                }
            }
        }
    }
    milestones.complete_if(
        "all-variants-visible",
        variants.len() == expected_count && variant_failures.is_empty(),
    );
    let cat_id = selected(client.snapshot())
        .and_then(|colony| colony.cats.first())
        .map(|cat| cat.id.clone())
        .ok_or_else(|| EntryFailure {
            id: "item-variants".to_owned(),
            message: "missing equipment bearer".to_owned(),
        })?;
    for (kind, milestone) in [
        (ItemKind::Tool, "tool-equipped"),
        (ItemKind::Weapon, "weapon-equipped"),
        (ItemKind::Armor, "armor-equipped"),
    ] {
        let item_id = ids.get(&kind).expect("damaged functional id");
        require_action(
            &mut client,
            ClientAction::RepairItem {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                item_id: item_id.clone(),
            },
            "repair exact equipment",
        )
        .await
        .map_err(|message| EntryFailure {
            id: format!("repair-{kind:?}"),
            message,
        })?;
        require_action(
            &mut client,
            ClientAction::EquipItem {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                cat_id: cat_id.clone(),
                item_id: item_id.clone(),
            },
            "equip exact equipment",
        )
        .await
        .map_err(|message| EntryFailure {
            id: format!("equip-{kind:?}"),
            message,
        })?;
        milestones.complete(milestone);
    }
    for kind in [ItemKind::Tool, ItemKind::Weapon, ItemKind::Armor] {
        let item_id = ids.get(&kind).expect("functional id");
        require_action(
            &mut client,
            ClientAction::UnequipItem {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                cat_id: cat_id.clone(),
                item_id: item_id.clone(),
            },
            "unequip exact equipment",
        )
        .await
        .map_err(|message| EntryFailure {
            id: format!("unequip-{kind:?}"),
            message,
        })?;
    }
    milestones.complete("all-equipment-unequipped");
    harness
        .advance_by(&mut client, 1)
        .await
        .map_err(|message| EntryFailure {
            id: "item-variants".to_owned(),
            message: format!("project final equipment actions: {message}"),
        })?;
    let before = item_variant_projection(client.snapshot());
    let restarted = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: "item-variants".to_owned(),
            message,
        })?;
    let after = item_variant_projection(restarted.snapshot());
    let difference = restart_difference(&json!(before), &json!(after));
    milestones.complete_if("restart-state-matched", difference.is_none());
    if !variant_failures.is_empty() || difference.is_some() {
        return Err(EntryFailure {
            id: "item-variants".to_owned(),
            message: write_entry_failure(
                &harness,
                &restarted,
                "item-variants",
                "all-450",
                &milestones,
                difference.as_ref(),
                &format!("missing or duplicate variants: {variant_failures:?}"),
            ),
        });
    }
    Ok(())
}

fn resource_projection(snapshot: &WorldSnapshot) -> BTreeMap<String, f64> {
    let Some(colony) = selected(snapshot) else {
        return BTreeMap::new();
    };
    ProtoResourceKind::ALL
        .iter()
        .copied()
        .map(|kind| (resource_key(kind), resource_amount(&colony.resources, kind)))
        .collect()
}

fn physical_resource_projection(
    snapshot: &WorldSnapshot,
    kind: ProtoResourceKind,
) -> BTreeMap<String, f64> {
    selected(snapshot)
        .into_iter()
        .flat_map(|colony| colony.stockpiles.iter())
        .map(|pile| (pile.id.clone(), resource_amount(&pile.contents, kind)))
        .collect()
}

async fn run_resource_catalog(seed: u32) -> Result<(), EntryFailure> {
    let mut harness = WsGameHarness::start_with(seed, |world| {
        prepare_common(world);
        let colony = &mut world.colonies[0];
        for pile in &mut colony.stockpiles {
            for &kind in ResourceKind::ALL {
                set_resource(&mut pile.contents, kind, 0.0);
            }
        }
        let pile_index = colony
            .stockpiles
            .iter()
            .position(|pile| pile.is_general_storehouse())
            .expect("starter general stockpile");
        let pile_id = colony.stockpiles[pile_index].id.clone();
        for (index, &kind) in ResourceKind::ALL.iter().enumerate() {
            let amount = (index + 1) as f64;
            set_resource(&mut colony.resources, kind, amount);
            if kind == ResourceKind::Blessings {
                continue;
            } else {
                set_resource(&mut colony.stockpiles[pile_index].contents, kind, amount);
            }
        }
        colony.items = Default::default();
        for (kind, count) in [
            (ItemKind::Weapon, 14),
            (ItemKind::Armor, 15),
            (ItemKind::Tool, 20),
        ] {
            colony.items.add_at(
                Item::new(kind, Material::Wood, 0),
                count,
                1.0,
                SimItemLocation::Stockpile {
                    stockpile_id: pile_id.clone(),
                },
                true,
            );
        }
        reconcile_colony_stockpiles(colony);
        set_resource(&mut colony.resources, ResourceKind::Blessings, 32.0);
        colony.global_upgrade_points = 32.0;
        colony.stock_ledger = StockLedger::counted_with_piles(
            &colony.resources,
            &colony.stockpiles,
            colony.last_tick,
        );
    })
    .await
    .map_err(|message| EntryFailure {
        id: "resources".to_owned(),
        message,
    })?;
    let (client, actor) = harness
        .connect_authenticated("resource-catalog", "Resource Catalog")
        .await
        .map_err(|message| EntryFailure {
            id: "resources".to_owned(),
            message,
        })?;
    let mut milestones = MilestoneTracker::new(RESOURCE_MILESTONES);
    milestones.complete("authenticated");
    let before = resource_projection(client.snapshot());
    let failures = ProtoResourceKind::ALL
        .iter()
        .enumerate()
        .filter_map(|(index, kind)| {
            let expected = (index + 1) as f64;
            let actual = before
                .get(&resource_key(*kind))
                .copied()
                .unwrap_or_default();
            (!approximately_equal(expected, actual))
                .then_some(format!("{kind:?}: expected {expected}, observed {actual}"))
        })
        .collect::<Vec<_>>();
    milestones.complete_if("all-resource-values-visible", failures.is_empty());
    let physical_visible = ProtoResourceKind::physical_stockpile_goods().all(|kind| {
        before
            .get(&resource_key(kind))
            .copied()
            .is_some_and(|amount| amount > 0.0)
    });
    milestones.complete_if("physical-stockpile-values-visible", physical_visible);
    let restarted = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: "resources".to_owned(),
            message,
        })?;
    let after = resource_projection(restarted.snapshot());
    let difference = restart_difference(&json!(before), &json!(after));
    milestones.complete_if("restart-state-matched", difference.is_none());
    if !failures.is_empty() || !physical_visible || difference.is_some() {
        return Err(EntryFailure {
            id: "resources".to_owned(),
            message: write_entry_failure(
                &harness,
                &restarted,
                "resources",
                "all-32",
                &milestones,
                difference.as_ref(),
                &format!("resource failures: {failures:?}"),
            ),
        });
    }
    Ok(())
}

fn find_fine_biome_site(seed: u32, anchor: TilePos, biome: Biome) -> Option<TilePos> {
    type SiteCache = BTreeMap<(u32, i32, i32), BTreeMap<&'static str, TilePos>>;
    static CACHE: OnceLock<Mutex<SiteCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let cache_key = (seed, anchor.x, anchor.y);
    if let Some(site) = cache
        .lock()
        .expect("fine-biome site cache")
        .get(&cache_key)
        .and_then(|sites| sites.get(biome.as_str()))
        .copied()
    {
        return Some(site);
    }
    let wanted = if std::env::var_os("CAT_CATALOG_ENTRY").is_some() {
        BTreeSet::from([biome.as_str()])
    } else {
        FINITE_DEPOSITS
            .iter()
            .map(|spec| spec.biome.as_str())
            .collect::<BTreeSet<_>>()
    };
    let mut found = BTreeMap::new();
    // Generate every 12x12 chunk once. `tile_climate_biome` regenerates its
    // owning chunk, so calling it per coordinate is 144x duplicate work.
    for radius in 0_i32..=128 {
        for offset in -radius..=radius {
            for (chunk_x, chunk_y) in [
                (offset, -radius),
                (radius, offset),
                (offset, radius),
                (-radius, offset),
            ] {
                for tile in
                    generate_terrain_chunk(chunk_x, chunk_y, i64::from(seed), WORLD_TERRAIN_OPTIONS)
                {
                    if wanted.contains(tile.climate_biome.as_str()) {
                        found.entry(tile.climate_biome.as_str()).or_insert(TilePos {
                            x: tile.x,
                            y: tile.y,
                        });
                    }
                }
            }
        }
        if found.len() == wanted.len() {
            break;
        }
    }
    let answer = found.get(biome.as_str()).copied();
    cache
        .lock()
        .expect("fine-biome site cache")
        .insert(cache_key, found);
    answer
}

fn deposit_carrying_kind(resource: ResourceKind) -> cat_protocol::CarryingKind {
    match resource {
        ResourceKind::Gem => cat_protocol::CarryingKind::Gem,
        ResourceKind::Clay => cat_protocol::CarryingKind::Clay,
        ResourceKind::Sand => cat_protocol::CarryingKind::Sand,
        _ => unreachable!("finite deposit catalog only contains gem/clay/sand"),
    }
}

async fn run_deposit_entry(seed: u32, spec: DepositSpec) -> Result<(), EntryFailure> {
    let site_result = Arc::new(Mutex::new(None));
    let setup_site = Arc::clone(&site_result);
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_common(world);
        let colony = &mut world.colonies[0];
        colony.test_time_scale = 1.0;
        colony.cats.truncate(4);
        colony.leader_id = Some(colony.cats[3].id.clone());
        for cat in &mut colony.cats {
            cat.age_hours = 24.0;
        }
        let accountant_id = colony.cats[0].id.clone();
        colony
            .officers
            .insert(OfficerRole::Accountant, accountant_id.clone());
        colony.buildings.push(BuildingRuntime {
            id: "catalog-deposit-accounting".to_owned(),
            building_type: BuildingType::AccountingTent,
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(accountant_id),
            ..BuildingRuntime::default()
        });
        let site = find_fine_biome_site(seed, colony.anchor, spec.biome);
        *setup_site.lock().expect("deposit site lock") = site;
        for tile in colony.world_tiles.values_mut() {
            tile.tile_type = TileType::Meadow;
            tile.resources.gem = 0;
            tile.resources.clay = 0;
            tile.resources.sand = 0;
        }
        if let Some(site) = site {
            colony.revealed_tiles.clear();
            colony.revealed_tiles.insert(colony.anchor);
            colony.revealed_tiles.insert(site);
            for dx in -8..=-1 {
                let path = TilePos {
                    x: site.x + dx,
                    y: site.y,
                };
                colony.revealed_tiles.insert(path);
                colony.world_tiles.insert(
                    path,
                    WorldTileRuntime {
                        pos: path,
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
                    },
                );
            }
            let worker_start = TilePos {
                x: site.x - 3,
                y: site.y,
            };
            colony.cats[1].position = Position {
                map: MapType::World,
                x: f64::from(worker_start.x),
                y: f64::from(worker_start.y),
            };
            colony.cats[0].position = Position {
                map: MapType::World,
                x: f64::from(site.x - 7),
                y: f64::from(site.y),
            };
            if let Some(tent) = colony
                .buildings
                .iter_mut()
                .find(|building| building.id == "catalog-deposit-accounting")
            {
                tent.position = TilePos {
                    x: site.x - 7,
                    y: site.y,
                };
            }
            colony.stockpiles.push(Stockpile {
                id: "catalog-deposit-output".to_owned(),
                rect: normalize_rect(
                    f64::from(site.x - 6),
                    f64::from(site.y),
                    f64::from(site.x - 6),
                    f64::from(site.y),
                ),
                accepts: if spec.resource == ResourceKind::Gem {
                    BTreeSet::from([
                        ResourceKind::Stone,
                        ResourceKind::Materials,
                        ResourceKind::Ore,
                        ResourceKind::Gem,
                    ])
                } else {
                    BTreeSet::from([spec.resource])
                },
                contents: Default::default(),
            });
            let mut resources = TileResources {
                food: 0,
                herbs: 0,
                water: 0,
                gem: 0,
                clay: 0,
                sand: 0,
            };
            match spec.resource {
                ResourceKind::Gem => resources.gem = 1,
                ResourceKind::Clay => resources.clay = 1,
                ResourceKind::Sand => resources.sand = 1,
                _ => unreachable!(),
            }
            colony.world_tiles.insert(
                site,
                WorldTileRuntime {
                    pos: site,
                    tile_type: if spec.resource == ResourceKind::Gem {
                        TileType::Mountains
                    } else {
                        TileType::Meadow
                    },
                    resources,
                    max_resources: MaxResources { food: 0, herbs: 0 },
                    danger_level: 0.0,
                    path_wear: 0,
                    last_depleted: 1,
                    overlay_feature: None,
                },
            );
        }
        for pile in &mut colony.stockpiles {
            set_resource(&mut pile.contents, spec.resource, 0.0);
        }
        set_resource(&mut colony.resources, spec.resource, 0.0);
        reconcile_colony_stockpiles(colony);
        colony.stock_ledger = StockLedger::counted_with_piles(
            &colony.resources,
            &colony.stockpiles,
            colony.last_tick,
        );
        publish_colony_spatial(&mut world.shared_spatial, &world.colonies[0]);
    })
    .await
    .map_err(|message| EntryFailure {
        id: format!("{:?}-{:?}", spec.biome, spec.resource),
        message,
    })?;
    let site = site_result
        .lock()
        .expect("deposit site lock")
        .ok_or_else(|| EntryFailure {
            id: format!("{:?}-{:?}", spec.biome, spec.resource),
            message: "no matching fine-biome coordinate within radius 768".to_owned(),
        })?;
    debug_assert_eq!(tile_climate_biome(seed, site.x, site.y), spec.biome);
    debug_assert_ne!(natural_deposits_for_biome(spec.biome), (0, 0, 0));
    let (mut client, actor) = harness
        .connect_authenticated(
            format!("deposit-{:?}-{:?}", spec.biome, spec.resource),
            "Deposit Catalog",
        )
        .await
        .map_err(|message| EntryFailure {
            id: format!("{:?}-{:?}", spec.biome, spec.resource),
            message,
        })?;
    let mut milestones = MilestoneTracker::new(DEPOSIT_MILESTONES);
    milestones.complete("authenticated");
    require_action(
        &mut client,
        ClientAction::RequestJob {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            kind: cat_protocol::JobKind::Quarry,
        },
        "request finite-deposit quarry",
    )
    .await
    .map_err(|message| EntryFailure {
        id: format!("{:?}-{:?}", spec.biome, spec.resource),
        message,
    })?;
    milestones.complete("signed-extraction-accepted");
    let kind = proto_resource(spec.resource);
    let carrying_kind = deposit_carrying_kind(spec.resource);
    let mut job_seen = false;
    let mut requested_job_id = None::<String>;
    let mut carry_seen = false;
    let mut completed_event_seen = false;
    let mut lifecycle_complete = false;
    let mut terminal_failure = false;
    // The fixture keeps a genuine seed-derived deposit but places worker, tent,
    // and accepting pile within eight tiles. The authored Quarry work duration
    // is two hours; three hours covers that plus all local loads/accounting.
    let horizon_ms = 3 * 60 * 60 * 1_000;
    let completion_wait = harness
        .eventually(&mut client, horizon_ms, 5 * 60_000, |snapshot| {
            let Some(colony) = selected(snapshot) else {
                return false;
            };
            if requested_job_id.is_none() {
                requested_job_id = colony
                    .jobs
                    .iter()
                    .find(|job| job.kind == cat_protocol::JobKind::Quarry)
                    .map(|job| job.id.clone());
            }
            job_seen |= requested_job_id.is_some();
            let requested_job_active = requested_job_id.as_deref().is_some_and(|id| {
                colony.jobs.iter().any(|job| {
                    job.id == id
                        && matches!(
                            job.status,
                            cat_protocol::JobStatus::Queued | cat_protocol::JobStatus::Active
                        )
                })
            });
            carry_seen |= colony.cats.iter().any(|cat| {
                cat.carrying
                    .as_ref()
                    .is_some_and(|cargo| cargo.kind == carrying_kind && cargo.amount > 0.0)
            });
            completed_event_seen |= colony.events.iter().any(|event| {
                event.kind == "job_completed"
                    && event.message.eq_ignore_ascii_case("Completed quarry.")
            });
            let physical_amount = colony
                .stockpiles
                .iter()
                .map(|pile| resource_amount(&pile.contents, kind))
                .sum::<f64>();
            lifecycle_complete = job_seen
                && !requested_job_active
                && completed_event_seen
                && !colony.cats.iter().any(|cat| {
                    cat.carrying
                        .as_ref()
                        .is_some_and(|cargo| cargo.kind == carrying_kind)
                })
                && approximately_equal(physical_amount, 1.0);
            terminal_failure = job_seen
                && carry_seen
                && !requested_job_active
                && !colony.cats.iter().any(|cat| {
                    cat.carrying
                        .as_ref()
                        .is_some_and(|cargo| cargo.kind == carrying_kind)
                })
                && !completed_event_seen
                && !lifecycle_complete;
            lifecycle_complete || terminal_failure
        })
        .await;
    let completed = if lifecycle_complete {
        completion_wait
    } else if terminal_failure {
        Err("quarry terminated after finite cargo pickup without authoritative completion and exact physical delivery".to_owned())
    } else {
        completion_wait
    };
    milestones.complete_if("job-observed", job_seen);
    milestones.complete_if("physical-carry-observed", carry_seen);
    milestones.complete_if("storage-delivery-observed", completed.is_ok());
    milestones.complete_if("finite-deposit-depleted", completed.is_ok());
    let terminal_diagnostic = selected(client.snapshot()).map(|colony| {
        json!({
            "requestedJobId": requested_job_id,
            "requestedJob": colony.jobs.iter().find(|job| Some(job.id.as_str()) == requested_job_id.as_deref()),
            "site": { "x": site.x, "y": site.y, "fineBiome": spec.biome.as_str() },
            "cats": colony.cats.iter().filter(|cat| {
                cat.current_task.is_some() || cat.carrying.is_some()
            }).map(|cat| json!({
                "id": cat.id,
                "activity": cat.activity,
                "task": cat.current_task,
                "position": cat.position,
                "destination": cat.destination,
                "carrying": cat.carrying,
            })).collect::<Vec<_>>(),
            "physicalPiles": colony.stockpiles.iter().map(|pile| json!({
                "id": pile.id,
                "amount": resource_amount(&pile.contents, kind),
            })).collect::<Vec<_>>(),
            "recentEvents": colony.events.iter().rev().take(12).collect::<Vec<_>>(),
        })
    });
    let before = physical_resource_projection(client.snapshot(), kind);
    let restarted = harness
        .restart_and_reconnect(client, &actor)
        .await
        .map_err(|message| EntryFailure {
            id: format!("{:?}-{:?}", spec.biome, spec.resource),
            message,
        })?;
    let after = physical_resource_projection(restarted.snapshot(), kind);
    let difference = restart_difference(&json!(before), &json!(after));
    milestones.complete_if("restart-state-matched", difference.is_none());
    if completed.is_err() || difference.is_some() {
        return Err(EntryFailure {
            id: format!("{:?}-{:?}", spec.biome, spec.resource),
            message: write_entry_failure(
                &harness,
                &restarted,
                "finite-deposit",
                &format!("{:?}-{:?}", spec.biome, spec.resource),
                &milestones,
                difference.as_ref(),
                &format!(
                    "{}; diagnostic={}",
                    completed
                        .err()
                        .unwrap_or_else(|| "restart mismatch".to_owned()),
                    terminal_diagnostic.unwrap_or(Value::Null)
                ),
            ),
        });
    }
    Ok(())
}

#[tokio::test]
async fn every_constructible_building_runs_signed_physical_lifecycle() {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        for building_type in CONSTRUCTIBLE_BUILDINGS {
            if !catalog_entry_selected(building_type.as_str()) {
                continue;
            }
            if let Err(mut failure) = run_building_entry(seed, *building_type).await {
                failure.id = format!("{}@{seed}", failure.id);
                failures.push(failure);
            }
        }
    }
    aggregate_failures("constructible building catalog journey", failures);
}

#[tokio::test]
async fn all_108_recipes_run_conserved_station_local_delivery_lifecycle() {
    let recipes = all_recipes();
    assert_eq!(recipes.len(), 108, "typed runtime recipe contract changed");
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        for recipe in &recipes {
            if !catalog_entry_selected(recipe.id) {
                continue;
            }
            if let Err(mut failure) = run_recipe_entry(seed, recipe).await {
                failure.id = format!("{}@{seed}", failure.id);
                failures.push(failure);
            }
        }
    }
    aggregate_failures("physical recipe catalog journey", failures);
}

#[tokio::test]
async fn all_three_crops_run_signed_physical_yield_and_restart_lifecycle() {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        for crop in [
            cat_protocol::CropKind::Catnip,
            cat_protocol::CropKind::Grain,
            cat_protocol::CropKind::Herb,
        ] {
            if !catalog_entry_selected(&format!("{crop:?}")) {
                continue;
            }
            if let Err(mut failure) = run_crop_entry(seed, crop).await {
                failure.id = format!("{}@{seed}", failure.id);
                failures.push(failure);
            }
        }
    }
    aggregate_failures("crop catalog journey", failures);
}

#[tokio::test]
async fn all_450_item_variants_persist_and_functional_families_use_signed_actions() {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        if let Err(mut failure) = run_item_variant_catalog(seed).await {
            failure.id = format!("{}@{seed}", failure.id);
            failures.push(failure);
        }
    }
    aggregate_failures("finite item variant catalog journey", failures);
}

#[tokio::test]
async fn all_32_resources_roundtrip_through_snapshot_and_sqlite() {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        if let Err(mut failure) = run_resource_catalog(seed).await {
            failure.id = format!("{}@{seed}", failure.id);
            failures.push(failure);
        }
    }
    aggregate_failures("resource catalog journey", failures);
}

#[tokio::test]
async fn every_finite_deposit_family_in_each_applicable_fine_biome_is_depleted_physically() {
    let mut failures = Vec::new();
    for &seed in super::requested_seed_tier().seeds() {
        for &spec in FINITE_DEPOSITS {
            let entry_id = format!("{:?}-{:?}", spec.biome, spec.resource);
            if !catalog_entry_selected(&entry_id) {
                continue;
            }
            if let Err(mut failure) = run_deposit_entry(seed, spec).await {
                failure.id = format!("{}@{seed}", failure.id);
                failures.push(failure);
            }
        }
    }
    aggregate_failures("finite deposit catalog journey", failures);
}

#[test]
fn executable_catalog_inventory_is_exhaustive_and_stable() {
    assert_eq!(EXECUTABLE_SCENARIO_IDS.len(), 6);
    assert_eq!(CONSTRUCTIBLE_BUILDINGS.len(), BuildingType::ALL.len() - 1);
    assert!(!CONSTRUCTIBLE_BUILDINGS.contains(&BuildingType::Shrine));
    assert_eq!(all_recipes().len(), 108);
    let unique = all_recipes()
        .into_iter()
        .map(|recipe| recipe.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), 108);
    assert_eq!(cat_protocol::ResourceKind::ALL.len(), 32);
    assert_eq!(ItemKind::ALL.len(), 10);
    assert_eq!(Material::ALL.len(), 9);
    assert_eq!(MAX_QUALITY, 4);
    assert_eq!(ItemKind::ALL.len() * Material::ALL.len() * 5, 450);
    assert_eq!(FINITE_DEPOSITS.len(), 6);
    for spec in FINITE_DEPOSITS {
        let (gem, clay, sand) = natural_deposits_for_biome(spec.biome);
        assert_eq!(gem > 0, spec.resource == ResourceKind::Gem);
        assert_eq!(clay > 0, spec.resource == ResourceKind::Clay);
        assert_eq!(sand > 0, spec.resource == ResourceKind::Sand);
    }
}

#[test]
fn milestone_tracker_never_skips_an_ordered_boundary() {
    static ORDER: &[&str] = &["first", "second", "third"];
    let mut tracker = MilestoneTracker::new(ORDER);
    tracker.complete_if("second", true);
    assert_eq!(tracker.last_completed(), None);
    tracker.complete("first");
    tracker.complete_if("third", true);
    assert_eq!(tracker.last_completed(), Some("first"));
    tracker.complete_if("second", true);
    tracker.complete_if("third", true);
    assert_eq!(tracker.last_completed(), Some("third"));
}

#[test]
fn every_staffed_building_has_family_specific_operation_evidence() {
    for &building_type in BuildingType::ALL {
        let staffed = cat_sim::production::building_staff_cap(building_type) > 0;
        let family = building_operation_family(building_type);
        assert_eq!(
            staffed,
            family != BuildingOperationFamily::None,
            "{building_type:?} operation family drifted from its staffing contract"
        );
    }
}

#[test]
fn restart_difference_is_absent_for_equality_and_reports_both_sides() {
    let before = json!({ "queue": ["recipe"], "staffCount": 1 });
    assert_eq!(restart_difference(&before, &before), None);
    let after = json!({ "queue": [], "staffCount": 0 });
    assert_eq!(
        restart_difference(&before, &after),
        Some(json!({ "before": before, "after": after }))
    );
}

#[test]
fn repeating_queue_rotation_is_restart_continuity_not_loss() {
    let entry = |recipe_id: &str| cat_protocol::ProductionQueueEntrySnapshot {
        recipe_id: recipe_id.to_owned(),
        repeat: true,
    };
    let before = vec![entry("first"), entry("second")];
    let after = vec![entry("second"), entry("first")];
    assert!(queues_are_cyclically_equivalent(&before, &after));
    assert!(!queues_are_cyclically_equivalent(
        &before,
        &[entry("second")]
    ));
}

#[tokio::test]
async fn field_fixture_has_a_signed_action_legal_site() {
    let result = Arc::new(Mutex::new(None));
    let setup_result = Arc::clone(&result);
    let _harness = WsGameHarness::start_with(super::PRIMARY_SEED, move |world| {
        prepare_building(world, BuildingType::Field);
        *setup_result.lock().expect("field fixture result lock") =
            Some(find_valid_player_site(world, BuildingType::Field));
    })
    .await
    .expect("field fixture harness starts");
    let result = result
        .lock()
        .expect("field fixture result lock")
        .clone()
        .expect("field fixture records site result");
    assert!(result.is_ok(), "{result:?}");
}
