//! Worker-progression journeys and exhaustive typed-catalog/action contract guards.
//!
//! These scenarios are anchored in `docs/GAME_VISION.md` ("Cats do the work") and
//! the full playtest matrix in `docs/IMPLEMENTATION_AUDIT.md`. The catalog tests are
//! deliberately aggregate: every entry is inspected before a single diagnostic is
//! emitted, so one broken recipe or study cannot hide the rest of the drift.

use std::collections::{BTreeMap, BTreeSet};

use cat_protocol::{
    AccelerationPreset, BuildingType as ProtocolBuildingType, ClientAction, CropKind,
    JobKind as ProtocolJobKind, Labor as ProtocolLabor, OfferingResource,
    OfficerRole as ProtocolOfficerRole, ProductionQueueEdit, QueueMoveDirection,
    ResourceKind as ProtocolResource, ScoutMission, TilePoint, TransportMode, UpgradeKey, ZoneKind,
};
use cat_sim::{
    biomes::BiomeType,
    climate::{Biome, Mining, ResourceHint, biome_climate},
    entities::{CatActivity as SimCatActivity, MapType, Position, Resources},
    farming::{CropKind as SimCropKind, FarmPlot, FarmStage, FarmWorkPhase},
    items::{Item, ItemKind, MAX_QUALITY, Material, item_base_max_durability, item_weight_grams},
    officers::OfficerRole,
    research_catalog::{RESEARCH_NODE_COUNT, research_catalog, research_node_is_implemented},
    skills::Labor,
    station_recipes::{StationRecipeDescriptor, station_recipe, station_recipe_set},
    stockpiles::{self, ResourceKind},
    terrain_gen::{TREE_FOOTPRINT_HEIGHT, TREE_FOOTPRINT_WIDTH, tile_has_tree},
    trader::{self, TraderState},
    transport::{RoutePhase, TransportMode as SimTransportMode, TransportRoute},
    types::{BuildingType, CatSpecialization, JobKind, TileType},
    village_layout::village_ring_radius,
    world_gen::natural_deposits_for_biome,
    world_tick::{
        BuildingRuntime, ProductionQueueEntry, ProductionWorkSlot, RaiderRuntime, TraderRuntime,
        WorldState, has_replant_site,
    },
    zones::ZoneRect,
};

use super::{Milestone, ScenarioSpec, SeedTier};
use crate::playtest_harness::{
    FailureTrace, SignedActor, WsClient, WsGameHarness, write_failure_trace,
};

const WORKER_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "signed-control-accepted",
        description: "the server accepts the signed work, staffing, or priority control",
    },
    Milestone {
        id: "physical-work-started",
        description: "the projected cat and worksite expose travel or active work",
    },
    Milestone {
        id: "real-work-completed",
        description: "the authoritative physical job or station cycle completes",
    },
    Milestone {
        id: "skill-threshold-crossed",
        description: "the exact worker's labor XP crosses the scenario threshold",
    },
    Milestone {
        id: "productivity-effect-observed",
        description: "a subsequent equivalent work unit exposes the documented speed or yield effect",
    },
    Milestone {
        id: "restart-persistence",
        description: "save, shutdown, restart, reconnect, and snapshot preserve the exact worker XP",
    },
];

const SCOUT_WORKER_MILESTONES: &[Milestone] = &[
    Milestone {
        id: "signed-control-accepted",
        description: "the server accepts the signed DispatchScout action",
    },
    Milestone {
        id: "five-by-five-observed",
        description: "the worker at Scout XP 4 exposes a 5x5 provisional mission view",
    },
    Milestone {
        id: "real-work-completed",
        description: "the scout physically returns to the shrine and completes the mission",
    },
    Milestone {
        id: "skill-threshold-crossed",
        description: "the same scout crosses from XP 4 to XP 5",
    },
    Milestone {
        id: "six-by-six-observed",
        description: "the next mission by the same scout exposes a 6x6 provisional view",
    },
    Milestone {
        id: "restart-persistence",
        description: "save, restart, and reconnect preserve XP 5 and the 6x6 capability",
    },
];

const PRODUCTIVITY_OUTCOMES: &[&str] = &["speed_increased", "yield_increased"];
const SPEED_OUTCOME: &[&str] = &["speed_increased"];
const YIELD_OUTCOME: &[&str] = &["yield_increased"];
const SCOUT_OUTCOME: &[&str] = &["vision_5x5_then_6x6"];
const WORKER_PERSISTENCE: &[&str] = &["worker-skill-xp", "post-threshold-productivity"];
const SCOUT_PERSISTENCE: &[&str] = &["scout-xp-5", "mission-vision-6x6"];

macro_rules! worker_scenario {
    ($id:literal, $setup:literal, $trigger:literal, $outcomes:expr) => {
        ScenarioSpec {
            id: $id,
            design_anchor: "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix / worker progression",
            initial_setup: $setup,
            action_or_trigger: $trigger,
            milestones: WORKER_MILESTONES,
            horizon_ms: 3_600_000,
            allowed_outcomes: $outcomes,
            seed_tier: SeedTier::Primary,
            persistence_checkpoints: WORKER_PERSISTENCE,
        }
    };
}

/// One bounded real-work journey for every maintained worker skill.
pub(crate) const SCENARIOS: &[ScenarioSpec] = &[
    worker_scenario!(
        "worker-skill-hunt",
        "named hunter at Hunt XP 11 with reachable game and finite storage",
        "signed RequestJob(HuntExpedition), then repeat with the same worker",
        PRODUCTIVITY_OUTCOMES
    ),
    worker_scenario!(
        "worker-skill-fishing",
        "named fisher at Fishing XP 7 with a revealed stocked shoreline",
        "signed DesignateFishingSpot and RequestJob(Fish), then repeat",
        YIELD_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-build",
        "named builder at Build XP 24 with exact physical building inputs",
        "signed PlanBuilding, then construct a second equivalent footprint",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-ritual",
        "named ritualist at an observable Ritual productivity threshold with a completed shrine and offering",
        "signed OfferResource followed by PerformOffering, then repeat",
        PRODUCTIVITY_OUTCOMES
    ),
    worker_scenario!(
        "worker-skill-fight",
        "named warrior at Fight XP 23 with equipment and deterministic raid pressure",
        "signed DefendRaid through resolution, then resolve an equivalent raid",
        PRODUCTIVITY_OUTCOMES
    ),
    worker_scenario!(
        "worker-skill-train",
        "named adult at an observable Train productivity threshold with a staffed completed Barracks",
        "signed TrainWarrior for the exact cat, then train an equivalent cat",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-quarry",
        "named quarry worker at Quarry XP 5 with a revealed finite mountain site",
        "signed RequestJob(Quarry), then repeat before deposit exhaustion",
        YIELD_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-woodcut",
        "named forester at Woodcut XP 5 with a reachable mature tree",
        "signed RequestJob(GatherLogs), then fell another equivalent tree",
        YIELD_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-forage",
        "named forager at an observable Forage productivity threshold with output capacity",
        "signed RequestJob(ForageFibre), then repeat",
        PRODUCTIVITY_OUTCOMES
    ),
    worker_scenario!(
        "worker-skill-fetch-water",
        "named carrier at FetchWater XP 4 with reachable water and storage",
        "signed RequestJob(FetchWater), then repeat",
        PRODUCTIVITY_OUTCOMES
    ),
    worker_scenario!(
        "worker-skill-mill",
        "named miller at an observable Mill productivity threshold with a staffed Mill and finite batches",
        "signed EditProductionQueue and AssignWorker through two cycles",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-process",
        "named processor at an observable Process productivity threshold with a staffed Sawmill and finite Logs",
        "signed EditProductionQueue and AssignWorker through two cycles",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-craft",
        "named crafter at an observable Craft productivity threshold with a staffed Woodworking bench and finite inputs",
        "signed EditProductionQueue and AssignWorker through two cycles",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-textile",
        "named textile worker at an observable Textile productivity threshold with a staffed Clothier and finite Fibre",
        "signed EditProductionQueue and AssignWorker through two cycles",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-metalwork",
        "named smith at an observable Metalwork productivity threshold with a staffed Smithy and finite Metal",
        "signed EditProductionQueue and AssignWorker through two cycles",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-farm",
        "named farmer at Farm XP 24 with a staffed fertile plot",
        "signed DesignateFarm and AssignWorker through two harvest work-hours",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-haul",
        "named carrier at Haul XP 0.75 with repeated finite stockpile transfers",
        "signed HaulGatherSpot until the threshold, then an equivalent transfer",
        SPEED_OUTCOME
    ),
    worker_scenario!(
        "worker-skill-research",
        "named researcher at Research XP 24 with a staffed Research Hut",
        "signed AssignWorker through two research work-hours",
        PRODUCTIVITY_OUTCOMES
    ),
    ScenarioSpec {
        id: "worker-skill-scout-vision-4-to-5",
        design_anchor: "docs/IMPLEMENTATION_AUDIT.md#full-playtest-matrix / Scout skill 4 to 5",
        initial_setup: "named scout at exactly Scout XP 4 with hidden traversable frontier",
        action_or_trigger: "signed DispatchScout(Explore), shrine return, then dispatch the same cat again",
        milestones: SCOUT_WORKER_MILESTONES,
        horizon_ms: 600_000,
        allowed_outcomes: SCOUT_OUTCOME,
        seed_tier: SeedTier::HighRisk,
        persistence_checkpoints: SCOUT_PERSISTENCE,
    },
];

pub(crate) const EXECUTABLE_SCENARIO_IDS: &[&str] = &[
    "worker-skill-hunt",
    "worker-skill-fishing",
    "worker-skill-build",
    "worker-skill-ritual",
    "worker-skill-fight",
    "worker-skill-train",
    "worker-skill-quarry",
    "worker-skill-woodcut",
    "worker-skill-forage",
    "worker-skill-fetch-water",
    "worker-skill-mill",
    "worker-skill-process",
    "worker-skill-craft",
    "worker-skill-textile",
    "worker-skill-metalwork",
    "worker-skill-farm",
    "worker-skill-haul",
    "worker-skill-research",
    "worker-skill-scout-vision-4-to-5",
];

#[derive(Clone, Copy)]
struct WorkerCoverage {
    labor: Labor,
    scenario_id: &'static str,
}

const WORKER_COVERAGE: &[WorkerCoverage] = &[
    WorkerCoverage {
        labor: Labor::Hunt,
        scenario_id: "worker-skill-hunt",
    },
    WorkerCoverage {
        labor: Labor::Fishing,
        scenario_id: "worker-skill-fishing",
    },
    WorkerCoverage {
        labor: Labor::Build,
        scenario_id: "worker-skill-build",
    },
    WorkerCoverage {
        labor: Labor::Ritual,
        scenario_id: "worker-skill-ritual",
    },
    WorkerCoverage {
        labor: Labor::Fight,
        scenario_id: "worker-skill-fight",
    },
    WorkerCoverage {
        labor: Labor::Train,
        scenario_id: "worker-skill-train",
    },
    WorkerCoverage {
        labor: Labor::Quarry,
        scenario_id: "worker-skill-quarry",
    },
    WorkerCoverage {
        labor: Labor::Woodcut,
        scenario_id: "worker-skill-woodcut",
    },
    WorkerCoverage {
        labor: Labor::Forage,
        scenario_id: "worker-skill-forage",
    },
    WorkerCoverage {
        labor: Labor::FetchWater,
        scenario_id: "worker-skill-fetch-water",
    },
    WorkerCoverage {
        labor: Labor::Mill,
        scenario_id: "worker-skill-mill",
    },
    WorkerCoverage {
        labor: Labor::Process,
        scenario_id: "worker-skill-process",
    },
    WorkerCoverage {
        labor: Labor::Craft,
        scenario_id: "worker-skill-craft",
    },
    WorkerCoverage {
        labor: Labor::Textile,
        scenario_id: "worker-skill-textile",
    },
    WorkerCoverage {
        labor: Labor::Metalwork,
        scenario_id: "worker-skill-metalwork",
    },
    WorkerCoverage {
        labor: Labor::Farm,
        scenario_id: "worker-skill-farm",
    },
    WorkerCoverage {
        labor: Labor::Haul,
        scenario_id: "worker-skill-haul",
    },
    WorkerCoverage {
        labor: Labor::Research,
        scenario_id: "worker-skill-research",
    },
    WorkerCoverage {
        labor: Labor::Scout,
        scenario_id: "worker-skill-scout-vision-4-to-5",
    },
];

#[derive(Default)]
struct Failures(Vec<String>);

impl Failures {
    fn check(&mut self, condition: bool, message: impl FnOnce() -> String) {
        if !condition {
            self.0.push(message());
        }
    }

    fn finish(self, heading: &str) {
        assert!(
            self.0.is_empty(),
            "{heading} ({} failures):\n{}",
            self.0.len(),
            self.0.join("\n")
        );
    }
}

#[test]
fn worker_manifest_covers_all_nineteen_skills_and_scout_threshold() {
    let mut failures = Failures::default();
    failures.check(SCENARIOS.len() == 19, || {
        format!("expected 19 worker scenarios, got {}", SCENARIOS.len())
    });
    failures.check(WORKER_COVERAGE.len() == Labor::ALL.len(), || {
        format!(
            "coverage has {} entries for {} skills",
            WORKER_COVERAGE.len(),
            Labor::ALL.len()
        )
    });
    for labor in Labor::ALL {
        let matches = WORKER_COVERAGE
            .iter()
            .filter(|entry| entry.labor == *labor)
            .collect::<Vec<_>>();
        failures.check(matches.len() == 1, || {
            format!("{labor:?} has {} manifest mappings", matches.len())
        });
        if let Some(entry) = matches.first() {
            failures.check(
                SCENARIOS
                    .iter()
                    .any(|scenario| scenario.id == entry.scenario_id),
                || format!("{labor:?} points at missing scenario {}", entry.scenario_id),
            );
        }
    }
    let scout = SCENARIOS
        .iter()
        .find(|scenario| scenario.id == "worker-skill-scout-vision-4-to-5");
    failures.check(scout.is_some(), || {
        "Scout 4-to-5 scenario is missing".to_owned()
    });
    if let Some(scout) = scout {
        for milestone in [
            "five-by-five-observed",
            "skill-threshold-crossed",
            "six-by-six-observed",
            "restart-persistence",
        ] {
            failures.check(
                scout.milestones.iter().any(|entry| entry.id == milestone),
                || format!("Scout scenario is missing {milestone}"),
            );
        }
    }
    failures.finish("worker progression manifest drift");
}

fn json_label<T: serde::Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value).map(|json| json.trim_matches('"').to_owned())
}

#[test]
fn typed_catalog_sweep_aggregates_every_current_option() {
    let mut failures = Failures::default();

    failures.check(BuildingType::ALL.len() == 25, || {
        format!(
            "building catalog count is {}, expected 25",
            BuildingType::ALL.len()
        )
    });
    let mut building_labels = BTreeSet::new();
    for building in BuildingType::ALL {
        failures.check(building_labels.insert(building.as_str()), || {
            format!("duplicate building label {}", building.as_str())
        });
        failures.check(
            building.as_str().parse::<BuildingType>() == Ok(*building),
            || format!("building {} does not round-trip", building.as_str()),
        );
    }

    failures.check(JobKind::ALL.len() == 20, || {
        format!("job catalog count is {}, expected 20", JobKind::ALL.len())
    });
    let mut job_labels = BTreeSet::new();
    for job in JobKind::ALL {
        failures.check(job_labels.insert(job.as_str()), || {
            format!("duplicate job label {}", job.as_str())
        });
        failures.check(job.as_str().parse::<JobKind>() == Ok(*job), || {
            format!("job {} does not round-trip", job.as_str())
        });
    }

    failures.check(ResourceKind::ALL.len() == 32, || {
        format!(
            "resource catalog count is {}, expected 32",
            ResourceKind::ALL.len()
        )
    });
    let mut resource_labels = BTreeSet::new();
    for resource in ResourceKind::ALL {
        match json_label(resource) {
            Ok(label) => {
                failures.check(resource_labels.insert(label.clone()), || {
                    format!("duplicate resource label {label}")
                });
                failures.check(
                    matches!(serde_json::from_str::<ResourceKind>(&format!("\"{label}\"")), Ok(value) if value == *resource),
                    || format!("resource {label} does not round-trip"),
                );
            }
            Err(error) => failures
                .0
                .push(format!("resource {resource:?} does not serialize: {error}")),
        }
    }

    let crops = [CropKind::Catnip, CropKind::Grain, CropKind::Herb];
    let mut crop_labels = BTreeSet::new();
    for crop in crops {
        match json_label(&crop) {
            Ok(label) => {
                failures.check(crop_labels.insert(label.clone()), || {
                    format!("duplicate crop label {label}")
                });
                failures.check(
                    matches!(serde_json::from_str::<CropKind>(&format!("\"{label}\"")), Ok(value) if value == crop),
                    || format!("crop {label} does not round-trip"),
                );
            }
            Err(error) => failures
                .0
                .push(format!("crop {crop:?} does not serialize: {error}")),
        }
    }
    failures.check(crop_labels.len() == 3, || {
        format!("crop catalog has {} entries", crop_labels.len())
    });

    failures.check(BiomeType::ALL.len() == 11, || {
        format!(
            "coarse biome catalog count is {}, expected 11",
            BiomeType::ALL.len()
        )
    });
    let mut coarse_biome_labels = BTreeSet::new();
    for biome in BiomeType::ALL {
        failures.check(coarse_biome_labels.insert(biome.as_str()), || {
            format!("duplicate coarse biome label {}", biome.as_str())
        });
        failures.check(biome.as_str().parse::<BiomeType>() == Ok(*biome), || {
            format!("coarse biome {} does not round-trip", biome.as_str())
        });
    }

    failures.check(Biome::ALL.len() == 26, || {
        format!(
            "fine biome catalog count is {}, expected 26",
            Biome::ALL.len()
        )
    });
    let mut fine_biome_labels = BTreeSet::new();
    let mut saw_ore = false;
    let mut saw_gem = false;
    let mut saw_clay = false;
    let mut saw_sand = false;
    for biome in Biome::ALL {
        let climate = biome_climate(*biome);
        failures.check(climate.biome == *biome, || {
            format!("{biome:?} resolves another climate row")
        });
        failures.check(fine_biome_labels.insert(climate.wire), || {
            format!("duplicate fine biome label {}", climate.wire)
        });
        failures.check(climate.wire.parse::<Biome>() == Ok(*biome), || {
            format!("fine biome {} does not round-trip", climate.wire)
        });
        saw_ore |= climate.resource == ResourceHint::Ore && climate.mining == Mining::Full;
        let (gem, clay, sand) = natural_deposits_for_biome(*biome);
        saw_gem |= gem > 0;
        saw_clay |= clay > 0;
        saw_sand |= sand > 0;
        failures.check(
            [gem, clay, sand]
                .iter()
                .filter(|amount| **amount > 0)
                .count()
                <= 1,
            || {
                format!(
                    "{biome:?} owns overlapping finite natural deposits ({gem}, {clay}, {sand})"
                )
            },
        );
    }
    for (name, seen) in [
        ("ore", saw_ore),
        ("gem", saw_gem),
        ("clay", saw_clay),
        ("sand", saw_sand),
    ] {
        failures.check(seen, || {
            format!("no biome supplies the {name} deposit family")
        });
    }

    failures.check(OfficerRole::ALL.len() == 7, || {
        format!(
            "officer catalog count is {}, expected 7",
            OfficerRole::ALL.len()
        )
    });
    let mut officer_labels = BTreeSet::new();
    for role in OfficerRole::ALL {
        match json_label(role) {
            Ok(label) => {
                failures.check(officer_labels.insert(label.clone()), || {
                    format!("duplicate officer label {label}")
                });
                failures.check(
                    matches!(serde_json::from_str::<OfficerRole>(&format!("\"{label}\"")), Ok(value) if value == *role),
                    || format!("officer {label} does not round-trip"),
                );
            }
            Err(error) => failures
                .0
                .push(format!("officer {role:?} does not serialize: {error}")),
        }
    }

    failures.check(Labor::ALL.len() == 19, || {
        format!(
            "worker skill catalog count is {}, expected 19",
            Labor::ALL.len()
        )
    });
    let mut labor_labels = BTreeSet::new();
    for labor in Labor::ALL {
        match json_label(labor) {
            Ok(label) => {
                failures.check(labor_labels.insert(label.clone()), || {
                    format!("duplicate worker skill label {label}")
                });
                failures.check(
                    matches!(serde_json::from_str::<Labor>(&format!("\"{label}\"")), Ok(value) if value == *labor),
                    || format!("worker skill {label} does not round-trip"),
                );
                failures.check(
                    serde_json::from_str::<ProtocolLabor>(&format!("\"{label}\"")).is_ok(),
                    || format!("worker skill {label} is missing from the protocol"),
                );
            }
            Err(error) => failures.0.push(format!(
                "worker skill {labor:?} does not serialize: {error}"
            )),
        }
    }

    failures.finish("typed catalog sweep");
}

#[test]
fn production_queue_equipment_recipe_and_research_sweep_aggregates_failures() {
    let mut failures = Failures::default();

    let queue_operations = [
        ProductionQueueEdit::Add {
            recipe_id: "recipe".to_owned(),
            repeat: false,
        },
        ProductionQueueEdit::Remove { index: 0 },
        ProductionQueueEdit::Move {
            index: 0,
            direction: QueueMoveDirection::Up,
        },
        ProductionQueueEdit::Move {
            index: 0,
            direction: QueueMoveDirection::Down,
        },
        ProductionQueueEdit::SetRepeat {
            index: 0,
            repeat: true,
        },
        ProductionQueueEdit::SetPaused { paused: true },
    ];
    for (index, operation) in queue_operations.iter().enumerate() {
        match serde_json::to_string(operation) {
            Ok(json) => failures.check(
                matches!(serde_json::from_str::<ProductionQueueEdit>(&json), Ok(value) if value == operation.clone()),
                || format!("queue operation {index} does not round-trip: {json}"),
            ),
            Err(error) => failures.0.push(format!(
                "queue operation {index} does not serialize: {error}"
            )),
        }
    }

    let mut item_wire_keys = BTreeSet::new();
    for kind in ItemKind::ALL {
        for material in Material::ALL {
            let mut previous_value = 0;
            for quality in 0..=MAX_QUALITY {
                let item = Item::new(*kind, *material, quality);
                match serde_json::to_string(&item) {
                    Ok(json) => {
                        failures.check(item_wire_keys.insert(json.clone()), || {
                            format!("duplicate item variant {json}")
                        });
                        failures.check(matches!(serde_json::from_str::<Item>(&json), Ok(value) if value == item), || {
                            format!("item {json} does not round-trip")
                        });
                    }
                    Err(error) => failures.0.push(format!(
                        "item {kind:?}/{material:?}/{quality} does not serialize: {error}"
                    )),
                }
                failures.check(item.value() >= previous_value, || {
                    format!("item value regressed for {kind:?}/{material:?} at quality {quality}")
                });
                failures.check(item_weight_grams(item) > 0, || {
                    format!("zero weight for {item:?}")
                });
                failures.check(item_base_max_durability(item) > 0, || {
                    format!("zero durability for {item:?}")
                });
                previous_value = item.value();
            }
        }
    }
    failures.check(
        item_wire_keys.len() == ItemKind::ALL.len() * Material::ALL.len() * 5,
        || {
            format!(
                "item variant sweep discovered {} unique keys",
                item_wire_keys.len()
            )
        },
    );

    let mut recipe_ids = BTreeSet::new();
    for building in BuildingType::ALL {
        if let Some(station) = station_recipe_set(*building) {
            failures.check(!station.input_resources.is_empty(), || {
                format!("{building:?} has no station input domain")
            });
            failures.check(!station.output_resources.is_empty(), || {
                format!("{building:?} has no station output domain")
            });
            for recipe in station.recipes {
                failures.check(recipe_ids.insert(recipe.id), || {
                    format!("duplicate recipe id {}", recipe.id)
                });
                failures.check(station_recipe(recipe.id) == Some(recipe), || {
                    format!("recipe {} does not resolve to itself", recipe.id)
                });
                failures.check(!recipe.input_resources.is_empty(), || {
                    format!("recipe {} has no finite input", recipe.id)
                });
                failures.check(
                    !recipe.output_resources.is_empty() || recipe.output_item.is_some(),
                    || format!("recipe {} has no finite output", recipe.id),
                );
                failures.check(recipe.building_type == *building, || {
                    format!(
                        "recipe {} belongs to {:?}, enumerated under {building:?}",
                        recipe.id, recipe.building_type
                    )
                });
            }
        }
    }
    failures.check(recipe_ids.len() == 108, || {
        format!("recipe sweep found {}, expected 108", recipe_ids.len())
    });

    let catalog = research_catalog();
    failures.check(catalog.nodes().len() == RESEARCH_NODE_COUNT, || {
        format!(
            "research catalog has {}, expected {RESEARCH_NODE_COUNT}",
            catalog.nodes().len()
        )
    });
    let mut research_ids = BTreeSet::new();
    for node in catalog.nodes() {
        failures.check(research_ids.insert(node.id.as_str()), || {
            format!("duplicate research study {}", node.id)
        });
        failures.check(catalog.get(&node.id) == Some(node), || {
            format!("research study {} does not resolve to itself", node.id)
        });
        failures.check(node.cost.is_finite() && node.cost >= 0.0, || {
            format!("research study {} has invalid cost {}", node.id, node.cost)
        });
        failures.check(!node.payloads.is_empty(), || {
            format!("research study {} has no typed payload", node.id)
        });
        for prerequisite in &node.prerequisites {
            failures.check(catalog.contains(prerequisite), || {
                format!(
                    "research study {} has missing prerequisite {prerequisite}",
                    node.id
                )
            });
        }
    }
    failures.check(research_ids.len() == RESEARCH_NODE_COUNT, || {
        format!("research sweep found {} unique studies", research_ids.len())
    });

    failures.finish("production/catalog breadth sweep");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthenticationClass {
    Presence,
    Signed,
    SessionBound,
    TestOnly,
}

fn authentication_class(action: &ClientAction) -> AuthenticationClass {
    match crate::action_authentication(action) {
        crate::ActionAuthentication::Presence { .. } => AuthenticationClass::Presence,
        crate::ActionAuthentication::Signed { .. } => AuthenticationClass::Signed,
        crate::ActionAuthentication::SessionBound { .. } => AuthenticationClass::SessionBound,
        crate::ActionAuthentication::TestOnly => AuthenticationClass::TestOnly,
    }
}

/// Exhaustive by construction: adding a protocol variant requires a stable wire-name
/// decision here before the server test suite compiles.
fn action_name(action: &ClientAction) -> &'static str {
    match action {
        ClientAction::Ensure => "ensure",
        ClientAction::Presence { .. } => "presence",
        ClientAction::RequestJob { .. } => "requestJob",
        ClientAction::DispatchScout { .. } => "dispatchScout",
        ClientAction::Boost { .. } => "boost",
        ClientAction::PurchaseUpgrade { .. } => "purchaseUpgrade",
        ClientAction::CastVote { .. } => "castVote",
        ClientAction::RequestVoteKick { .. } => "requestVoteKick",
        ClientAction::CreateZone { .. } => "createZone",
        ClientAction::RemoveZone { .. } => "removeZone",
        ClientAction::PlanBuilding { .. } => "planBuilding",
        ClientAction::UnlockNode { .. } => "unlockNode",
        ClientAction::ResearchNode { .. } => "researchNode",
        ClientAction::OfferTithe { .. } => "offerTithe",
        ClientAction::OfferMaterials { .. } => "offerMaterials",
        ClientAction::OfferResource { .. } => "offerResource",
        ClientAction::HaulGatherSpot { .. } => "haulGatherSpot",
        ClientAction::AssignWorker { .. } => "assignWorker",
        ClientAction::TrainWarrior { .. } => "trainWarrior",
        ClientAction::DefendRaid { .. } => "defendRaid",
        ClientAction::BuildRoad { .. } => "buildRoad",
        ClientAction::BuildBridge { .. } => "buildBridge",
        ClientAction::DesignateRail { .. } => "designateRail",
        ClientAction::BuildDock { .. } => "buildDock",
        ClientAction::BuildTransportVehicle { .. } => "buildTransportVehicle",
        ClientAction::CreateTransportRoute { .. } => "createTransportRoute",
        ClientAction::CancelTransportRoute { .. } => "cancelTransportRoute",
        ClientAction::SetTestAcceleration { .. } => "setTestAcceleration",
        ClientAction::AdvanceTime { .. } => "advanceTime",
        ClientAction::SetTestRngSeed { .. } => "setTestRngSeed",
        ClientAction::FoundVillage { .. } => "foundVillage",
        ClientAction::JoinVillage { .. } => "joinVillage",
        ClientAction::OfferVillageTrade { .. } => "offerVillageTrade",
        ClientAction::AcceptVillageTrade { .. } => "acceptVillageTrade",
        ClientAction::CancelVillageTrade { .. } => "cancelVillageTrade",
        ClientAction::AssignOfficer { .. } => "assignOfficer",
        ClientAction::UnassignOfficer { .. } => "unassignOfficer",
        ClientAction::DesignateFarm { .. } => "designateFarm",
        ClientAction::ClearFarm { .. } => "clearFarm",
        ClientAction::DesignateStockpile { .. } => "designateStockpile",
        ClientAction::RemoveStockpile { .. } => "removeStockpile",
        ClientAction::DesignateGatherSpot { .. } => "designateGatherSpot",
        ClientAction::DesignateFishingSpot { .. } => "designateFishingSpot",
        ClientAction::RemoveGatherSpot { .. } => "removeGatherSpot",
        ClientAction::SellGoods { .. } => "sellGoods",
        ClientAction::RepairItem { .. } => "repairItem",
        ClientAction::EquipItem { .. } => "equipItem",
        ClientAction::UnequipItem { .. } => "unequipItem",
        ClientAction::BuyResource { .. } => "buyResource",
        ClientAction::BoostCat { .. } => "boostCat",
        ClientAction::SetCatLaborPreference { .. } => "setCatLaborPreference",
        ClientAction::EditProductionQueue { .. } => "editProductionQueue",
        ClientAction::EditProductionWorkSlot { .. } => "editProductionWorkSlot",
    }
}

fn sid() -> String {
    "wrong-session".to_owned()
}
fn nickname() -> String {
    "Playtester".to_owned()
}
fn bad_sig() -> String {
    "invalid-signature".to_owned()
}
fn id() -> String {
    "missing-id".to_owned()
}
fn point() -> TilePoint {
    TilePoint { x: 6, y: 6 }
}

fn action_samples() -> Vec<ClientAction> {
    vec![
        ClientAction::Ensure,
        ClientAction::Presence {
            session_id: sid(),
            nickname: nickname(),
            sig: Some(bad_sig()),
        },
        ClientAction::RequestJob {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            kind: ProtocolJobKind::HuntExpedition,
        },
        ClientAction::DispatchScout {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            mission: ScoutMission::Explore,
        },
        ClientAction::Boost {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            job_id: id(),
        },
        ClientAction::PurchaseUpgrade {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            key: UpgradeKey::ClickPower,
        },
        ClientAction::CastVote {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            election_id: id(),
            cat_id: id(),
        },
        ClientAction::RequestVoteKick {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
        },
        ClientAction::CreateZone {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            kind: ZoneKind::Avoid,
            a: point(),
            b: point(),
            duration_ms: 1,
        },
        ClientAction::RemoveZone {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            zone_id: id(),
        },
        ClientAction::PlanBuilding {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            building_type: ProtocolBuildingType::Den,
            site: Some(point()),
        },
        ClientAction::UnlockNode {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            node_id: id(),
        },
        ClientAction::ResearchNode {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            node_id: id(),
        },
        ClientAction::OfferTithe {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
        },
        ClientAction::OfferMaterials {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
        },
        ClientAction::OfferResource {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            resource: OfferingResource::Materials,
        },
        ClientAction::HaulGatherSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            stockpile_id: id(),
            cat_id: None,
        },
        ClientAction::AssignWorker {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: id(),
            building_id: Some(id()),
        },
        ClientAction::TrainWarrior {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: Some(id()),
        },
        ClientAction::DefendRaid {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
        },
        ClientAction::BuildRoad {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: point(),
            b: TilePoint { x: 7, y: 6 },
        },
        ClientAction::BuildBridge {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            at: point(),
        },
        ClientAction::DesignateRail {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: point(),
            b: TilePoint { x: 7, y: 6 },
            cat_id: id(),
        },
        ClientAction::BuildDock {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            land: point(),
            water: TilePoint { x: 6, y: 7 },
            cat_id: id(),
        },
        ClientAction::BuildTransportVehicle {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            mode: TransportMode::Rail,
            home: point(),
            cat_id: id(),
        },
        ClientAction::CreateTransportRoute {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            mode: TransportMode::Rail,
            source_stockpile_id: id(),
            destination_stockpile_id: id(),
            resource: ProtocolResource::Food,
            amount: 1.0,
            path: vec![point()],
            cat_id: id(),
            repeat: false,
        },
        ClientAction::CancelTransportRoute {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            route_id: id(),
        },
        ClientAction::SetTestAcceleration {
            preset: AccelerationPreset::Fast,
        },
        ClientAction::AdvanceTime { seconds: 1 },
        ClientAction::SetTestRngSeed { seed: Some(4_242) },
        ClientAction::FoundVillage {
            name: "Contract Village".to_owned(),
            session_id: sid(),
            sig: None,
        },
        ClientAction::JoinVillage {
            colony_id: id(),
            session_id: sid(),
            sig: Some(bad_sig()),
        },
        ClientAction::OfferVillageTrade {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            target_colony_id: id(),
            offered_kind: ProtocolResource::Food,
            offered_amount: 1.0,
            requested_kind: ProtocolResource::Water,
            requested_amount: 1.0,
        },
        ClientAction::AcceptVillageTrade {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            offer_id: id(),
        },
        ClientAction::CancelVillageTrade {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            offer_id: id(),
        },
        ClientAction::AssignOfficer {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            role: ProtocolOfficerRole::Steward,
            cat_id: id(),
        },
        ClientAction::UnassignOfficer {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            role: ProtocolOfficerRole::Steward,
        },
        ClientAction::DesignateFarm {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: point(),
            b: point(),
            crop: CropKind::Grain,
        },
        ClientAction::ClearFarm {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            plot_id: id(),
        },
        ClientAction::DesignateStockpile {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: point(),
            b: point(),
            accepts: vec![ProtocolResource::Food],
        },
        ClientAction::RemoveStockpile {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            stockpile_id: id(),
        },
        ClientAction::DesignateGatherSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: point(),
            b: point(),
            kind: ProtocolResource::Stone,
        },
        ClientAction::DesignateFishingSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            at: point(),
        },
        ClientAction::RemoveGatherSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            stockpile_id: id(),
        },
        ClientAction::SellGoods {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            kind: "mug".to_owned(),
            material: "clay".to_owned(),
            quality: 1,
            count: 1,
        },
        ClientAction::RepairItem {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            item_id: id(),
        },
        ClientAction::EquipItem {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: id(),
            item_id: id(),
        },
        ClientAction::UnequipItem {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: id(),
            item_id: id(),
        },
        ClientAction::BuyResource {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            resource: ProtocolResource::Food,
            amount: 1.0,
        },
        ClientAction::BoostCat {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: id(),
            boosted: true,
        },
        ClientAction::SetCatLaborPreference {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            cat_id: id(),
            labor: ProtocolLabor::Hunt,
            enabled: true,
        },
        ClientAction::EditProductionQueue {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            building_id: id(),
            edit: ProductionQueueEdit::SetPaused { paused: true },
        },
        ClientAction::EditProductionWorkSlot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            building_id: id(),
            cat_id: id(),
            edit: ProductionQueueEdit::SetPaused { paused: true },
        },
    ]
}

fn malformed_action_wire(action: &ClientAction) -> serde_json::Value {
    let tag = action_name(action);
    match action {
        ClientAction::Ensure => serde_json::json!({ "action": "Ensure" }),
        // `Option<u32>` intentionally treats an absent seed as `None`; exercise
        // its malformed field type instead of misclassifying the supported wire.
        ClientAction::SetTestRngSeed { .. } => {
            serde_json::json!({ "action": tag, "seed": "not-a-seed" })
        }
        _ => serde_json::json!({ "action": tag }),
    }
}

fn action_with_actor(
    action: &ClientAction,
    actor: &SignedActor,
) -> Result<ClientAction, serde_json::Error> {
    let mut value = serde_json::to_value(action)?;
    let object = value
        .as_object_mut()
        .expect("ClientAction must serialize as an object");
    if object.contains_key("sessionId") {
        object.insert(
            "sessionId".to_owned(),
            serde_json::Value::String(actor.session_id.clone()),
        );
    }
    if object.contains_key("nickname") {
        object.insert(
            "nickname".to_owned(),
            serde_json::Value::String(actor.nickname.clone()),
        );
    }
    if object.contains_key("sig") {
        object.insert(
            "sig".to_owned(),
            serde_json::Value::String(actor.sig.clone()),
        );
    }
    serde_json::from_value(value)
}

fn is_authentication_rejection(message: Option<&str>) -> bool {
    message.is_some_and(|message| {
        let lowercase = message.to_ascii_lowercase();
        lowercase.contains("signature")
            || lowercase.contains("session")
            || lowercase.contains("authenticate")
    })
}

fn prepare_public_action_fixture(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.recipe_entitlement_rules_version = 0;
    colony.global_upgrade_points = 10_000.0;
    colony.upgrade_tree.research_points = 100_000.0;
    for kind in ResourceKind::ALL {
        stockpiles::set_resource(&mut colony.resources, *kind, 10_000.0);
    }
    if let Some(store) = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.is_general_storehouse())
    {
        store.contents = colony.resources.clone();
    }
    for cat in &mut colony.cats {
        cat.age_hours = 30.0;
        cat.death_time = None;
        cat.activity = SimCatActivity::Idle;
        cat.current_task = None;
        cat.destination = None;
        cat.carrying = None;
        cat.needs.hunger = 100.0;
        cat.needs.thirst = 100.0;
        cat.needs.rest = 100.0;
        cat.needs.health = 100.0;
    }
}

#[derive(Clone, Copy)]
struct AcceptedActionEvidence {
    action: &'static str,
    websocket_test: &'static str,
}

const ACCEPTED_ACTION_EVIDENCE: &[AcceptedActionEvidence] = &[
    AcceptedActionEvidence {
        action: "presence",
        websocket_test: "playtest_harness::real_socket_auth_tick_save_restart_and_reconnect_is_deterministic",
    },
    AcceptedActionEvidence {
        action: "requestJob",
        websocket_test: "worker_catalog::every_job_kind_crosses_real_websocket_with_valid_behavior",
    },
    AcceptedActionEvidence {
        action: "dispatchScout",
        websocket_test: "scouting::every_scout_mission_completes_physical_lifecycle_and_persists",
    },
    AcceptedActionEvidence {
        action: "boost",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "purchaseUpgrade",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "castVote",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "requestVoteKick",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "createZone",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "removeZone",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "planBuilding",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "unlockNode",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "researchNode",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "offerTithe",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "offerMaterials",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "offerResource",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "haulGatherSpot",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "assignWorker",
        websocket_test: "catalog_journeys::every_building_type_completes_physical_lifecycle_and_restart",
    },
    AcceptedActionEvidence {
        action: "trainWarrior",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "defendRaid",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "buildRoad",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "buildBridge",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "designateRail",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "buildDock",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "buildTransportVehicle",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "createTransportRoute",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "cancelTransportRoute",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "foundVillage",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "joinVillage",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "offerVillageTrade",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "acceptVillageTrade",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "cancelVillageTrade",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "assignOfficer",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "unassignOfficer",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "designateFarm",
        websocket_test: "catalog_journeys::every_building_type_completes_physical_lifecycle_and_restart",
    },
    AcceptedActionEvidence {
        action: "clearFarm",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "designateStockpile",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "removeStockpile",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "designateGatherSpot",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "designateFishingSpot",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "removeGatherSpot",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "sellGoods",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "repairItem",
        websocket_test: "catalog_journeys::every_recipe_completes_physical_lifecycle_and_restart",
    },
    AcceptedActionEvidence {
        action: "equipItem",
        websocket_test: "weapons_leader::prepared_weapon_chain_reaches_exact_warrior",
    },
    AcceptedActionEvidence {
        action: "unequipItem",
        websocket_test: "weapons_leader::prepared_weapon_chain_reaches_exact_warrior",
    },
    AcceptedActionEvidence {
        action: "buyResource",
        websocket_test: "system_journeys::causal_system_journeys_run_requested_seed_cohort",
    },
    AcceptedActionEvidence {
        action: "boostCat",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "setCatLaborPreference",
        websocket_test: "worker_catalog::focused_public_actions_execute_accepted_real_websocket_behavior",
    },
    AcceptedActionEvidence {
        action: "editProductionQueue",
        websocket_test: "worker_catalog::production_queue_and_exact_work_slot_edits_project_every_operation",
    },
    AcceptedActionEvidence {
        action: "editProductionWorkSlot",
        websocket_test: "worker_catalog::production_queue_and_exact_work_slot_edits_project_every_operation",
    },
];

#[test]
fn every_non_test_public_action_has_accepted_websocket_evidence() {
    let expected = action_samples()
        .into_iter()
        .filter(|action| {
            !matches!(action, ClientAction::Ensure)
                && authentication_class(action) != AuthenticationClass::TestOnly
        })
        .map(|action| action_name(&action))
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    let mut failures = Failures::default();
    for evidence in ACCEPTED_ACTION_EVIDENCE {
        failures.check(actual.insert(evidence.action), || {
            format!("{} has duplicate accepted evidence", evidence.action)
        });
        failures.check(!evidence.websocket_test.is_empty(), || {
            format!("{} has empty WebSocket evidence", evidence.action)
        });
    }
    failures.check(actual == expected, || {
        format!(
            "accepted WebSocket evidence drift: missing={:?}, extra={:?}",
            expected.difference(&actual).collect::<Vec<_>>(),
            actual.difference(&expected).collect::<Vec<_>>()
        )
    });
    failures.finish("accepted public ClientAction evidence manifest");
}

#[test]
fn client_action_manifest_has_stable_names_valid_wires_and_malformed_cases() {
    let actions = action_samples();
    let mut failures = Failures::default();
    // The protocol currently has 52 public commands plus the internal Ensure test control.
    failures.check(actions.len() == 53, || {
        format!(
            "ClientAction manifest has {}, expected 53 total variants",
            actions.len()
        )
    });
    failures.check(
        actions
            .iter()
            .filter(|action| !matches!(action, ClientAction::Ensure))
            .count()
            == 52,
        || "ClientAction manifest no longer has 52 commands excluding internal Ensure".to_owned(),
    );
    let mut tags = BTreeSet::new();
    for action in &actions {
        let expected_tag = action_name(action);
        match serde_json::to_value(action) {
            Ok(value) => {
                let actual_tag = value.get("action").and_then(serde_json::Value::as_str);
                failures.check(actual_tag == Some(expected_tag), || {
                    format!("{expected_tag} serialized with tag {actual_tag:?}")
                });
                failures.check(tags.insert(expected_tag), || {
                    format!("duplicate action tag {expected_tag}")
                });
                match serde_json::from_value::<ClientAction>(value) {
                    Ok(round_trip) => failures.check(round_trip == *action, || {
                        format!("{expected_tag} valid wire changed on round-trip")
                    }),
                    Err(error) => failures
                        .0
                        .push(format!("{expected_tag} valid wire was rejected: {error}")),
                }
            }
            Err(error) => failures
                .0
                .push(format!("{expected_tag} did not serialize: {error}")),
        }

        let malformed = malformed_action_wire(action);
        failures.check(
            serde_json::from_value::<ClientAction>(malformed).is_err(),
            || format!("{expected_tag} accepted its malformed minimal wire"),
        );
    }
    failures.finish("ClientAction wire manifest drift");
}

#[test]
fn every_client_action_has_an_explicit_authentication_category() {
    let actions = action_samples();
    let mut failures = Failures::default();
    let counts = [
        (AuthenticationClass::Presence, 1),
        (AuthenticationClass::Signed, 47),
        (AuthenticationClass::SessionBound, 1),
        (AuthenticationClass::TestOnly, 4),
    ];
    for (class, expected) in counts {
        let actual = actions
            .iter()
            .filter(|action| authentication_class(action) == class)
            .count();
        failures.check(actual == expected, || {
            format!("{class:?} action count is {actual}, expected {expected}")
        });
    }
    failures.finish("ClientAction authentication manifest drift");
}

async fn execute_action(
    state: &crate::AppState,
    connection: &mut crate::ConnectionContext,
    action: &ClientAction,
) -> crate::ServerActionResult {
    let encoded = serde_json::to_string(action).expect("serialize catalog action sample");
    crate::handle_client_text(state, connection, &encoded).await
}

#[tokio::test]
async fn invalid_authentication_and_production_denied_test_controls_are_aggregated() {
    let state = crate::build_state(1_000_000);
    let mut failures = Failures::default();
    for (index, action) in action_samples().into_iter().enumerate() {
        let name = action_name(&action);
        // Each case receives a distinct already-bound socket identity. That keeps
        // this authentication sweep below both real sliding-window limiter keys
        // without weakening or bypassing the production limiter implementation.
        let bound_session = format!("bound-session-{index}");
        let mut connection = crate::ConnectionContext {
            limiter_fallback: format!("worker-catalog-contract-{index}"),
            peer_ip: None,
            identity: Some(crate::identity::SignedSession {
                session_id: bound_session.clone(),
                sig: "bound-signature".to_owned(),
                player_id: format!("bound-player-{index}"),
            }),
            nickname: Some(nickname()),
            colony_id: crate::STARTER_COLONY_ID.to_owned(),
        };
        match authentication_class(&action) {
            AuthenticationClass::Presence => {}
            AuthenticationClass::Signed | AuthenticationClass::SessionBound => {
                let result = execute_action(&state, &mut connection, &action).await;
                failures.check(!result.result.ok, || {
                    format!("{name} accepted invalid authentication")
                });
                failures.check(
                    result.result.message.as_deref().is_some_and(|message| {
                        message.contains("session")
                            || message.contains("signature")
                            || message.contains("Authenticate")
                    }),
                    || format!("{name} returned a non-auth rejection: {result:?}"),
                );
            }
            AuthenticationClass::TestOnly => {
                let result = execute_action(&state, &mut connection, &action).await;
                failures.check(!result.result.ok, || {
                    format!("production accepted test control {name}")
                });
                failures.check(
                    result.result.message.as_deref()
                        == Some("Test actions are disabled on this server."),
                    || format!("test control {name} returned an unexpected rejection: {result:?}"),
                );
            }
        }
    }
    failures.finish("ClientAction authentication execution sweep");
}

#[tokio::test]
async fn every_public_client_action_crosses_real_websocket_with_valid_identity() {
    let public_actions = action_samples()
        .into_iter()
        .filter(|action| !matches!(action, ClientAction::Ensure))
        .collect::<Vec<_>>();
    assert_eq!(public_actions.len(), 52);
    let mut failures = Failures::default();
    let mut observed_results = Vec::new();

    // Fresh bounded batches avoid conflating the command contract with either
    // per-session rate limiting or mutations performed by an earlier command.
    for (batch_index, batch) in public_actions.chunks(8).enumerate() {
        let mut harness =
            match WsGameHarness::start_with(4_242, prepare_public_action_fixture).await {
                Ok(harness) => harness,
                Err(error) => {
                    failures
                        .0
                        .push(format!("batch {batch_index} could not start: {error}"));
                    continue;
                }
            };
        let (mut client, actor) = match harness
            .connect_authenticated(
                format!("public-action-batch-{batch_index}"),
                format!("Action Cat {batch_index}"),
            )
            .await
        {
            Ok(connected) => connected,
            Err(error) => {
                failures.0.push(format!(
                    "batch {batch_index} could not authenticate: {error}"
                ));
                continue;
            }
        };
        for action in batch {
            let name = action_name(action);
            let signed = match action_with_actor(action, &actor) {
                Ok(action) => action,
                Err(error) => {
                    failures
                        .0
                        .push(format!("{name} could not bind valid identity: {error}"));
                    continue;
                }
            };
            match client.send_action(&signed).await {
                Ok(observed) => {
                    observed_results.push(serde_json::json!({
                        "action": name,
                        "result": observed.raw,
                    }));
                    if authentication_class(&signed) == AuthenticationClass::TestOnly {
                        failures.check(!observed.result.ok, || {
                            format!("production WebSocket accepted test control {name}")
                        });
                        failures.check(
                            observed.result.message.as_deref()
                                == Some("Test actions are disabled on this server."),
                            || format!("{name} returned unexpected production result {observed:?}"),
                        );
                    } else {
                        failures.check(
                            !is_authentication_rejection(observed.result.message.as_deref()),
                            || {
                                format!(
                                    "{name} rejected the harness-issued valid identity: {observed:?}"
                                )
                            },
                        );
                        failures.check(
                            observed.result.message.as_deref() != Some("Invalid action."),
                            || format!("{name} decoded but fell through the real action handler"),
                        );
                        failures.check(
                            observed.result.ok || observed.result.message.is_some(),
                            || format!("{name} rejected without a behavioral diagnostic"),
                        );
                    }
                }
                Err(error) => failures
                    .0
                    .push(format!("{name} did not return a WebSocket result: {error}")),
            }
        }
    }
    failures.check(observed_results.len() == 52, || {
        format!(
            "real WebSocket returned {} of 52 public action results; retained results: {}",
            observed_results.len(),
            serde_json::Value::Array(observed_results.clone())
        )
    });
    failures.finish("real WebSocket public ClientAction execution sweep");
}

#[tokio::test]
async fn every_public_malformed_wire_crosses_real_websocket_and_is_retained() {
    let public_actions = action_samples()
        .into_iter()
        .filter(|action| !matches!(action, ClientAction::Ensure))
        .collect::<Vec<_>>();
    let mut failures = Failures::default();
    let mut rejected_results = Vec::new();

    for (batch_index, batch) in public_actions.chunks(10).enumerate() {
        let mut harness = WsGameHarness::start(4_242)
            .await
            .unwrap_or_else(|error| panic!("start malformed batch {batch_index}: {error}"));
        let (mut client, _) = harness
            .connect_authenticated(
                format!("malformed-action-batch-{batch_index}"),
                format!("Malformed Cat {batch_index}"),
            )
            .await
            .unwrap_or_else(|error| panic!("authenticate malformed batch {batch_index}: {error}"));
        for action in batch {
            let name = action_name(action);
            let wire = malformed_action_wire(action).to_string();
            match client.send_raw(wire).await {
                Ok(observed) => {
                    rejected_results.push(serde_json::json!({
                        "action": name,
                        "result": observed.raw,
                    }));
                    failures.check(!observed.result.ok, || {
                        format!("malformed {name} was accepted over WebSocket")
                    });
                    failures.check(
                        observed.result.message.as_deref() == Some("Invalid action."),
                        || format!("malformed {name} returned {observed:?}"),
                    );
                }
                Err(error) => failures
                    .0
                    .push(format!("malformed {name} returned no result: {error}")),
            }
        }
    }
    failures.check(rejected_results.len() == 52, || {
        format!(
            "retained {} of 52 malformed WebSocket results",
            rejected_results.len()
        )
    });
    failures.finish("real WebSocket malformed ClientAction sweep");
}

#[tokio::test]
async fn every_invalid_authenticated_action_crosses_real_websocket_and_is_retained() {
    let actions = action_samples()
        .into_iter()
        .filter(|action| {
            matches!(
                authentication_class(action),
                AuthenticationClass::Signed | AuthenticationClass::SessionBound
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 48);
    let mut failures = Failures::default();
    let mut rejected_results = Vec::new();

    for (batch_index, batch) in actions.chunks(8).enumerate() {
        let mut harness = WsGameHarness::start(4_242)
            .await
            .unwrap_or_else(|error| panic!("start invalid-auth batch {batch_index}: {error}"));
        let (mut client, _) = harness
            .connect_authenticated(
                format!("invalid-auth-batch-{batch_index}"),
                format!("Invalid Auth Cat {batch_index}"),
            )
            .await
            .unwrap_or_else(|error| panic!("authenticate invalid-auth batch: {error}"));
        for action in batch {
            let name = action_name(action);
            match client.send_action(action).await {
                Ok(observed) => {
                    rejected_results.push(serde_json::json!({
                        "action": name,
                        "result": observed.raw,
                    }));
                    failures.check(!observed.result.ok, || {
                        format!("{name} accepted invalid identity over WebSocket")
                    });
                    failures.check(
                        is_authentication_rejection(observed.result.message.as_deref()),
                        || format!("{name} returned a non-auth result: {observed:?}"),
                    );
                }
                Err(error) => failures
                    .0
                    .push(format!("invalid-auth {name} returned no result: {error}")),
            }
        }
    }
    failures.check(rejected_results.len() == actions.len(), || {
        format!(
            "retained {} of {} invalid-auth WebSocket results",
            rejected_results.len(),
            actions.len()
        )
    });
    failures.finish("real WebSocket invalid-auth ClientAction sweep");
}

fn prepare_queue_action_fixture(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.recipe_entitlement_rules_version = 0;
    let worker_ids = colony
        .cats
        .iter()
        .take(2)
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(worker_ids.len(), 2, "queue fixture needs two cats");
    for cat in &mut colony.cats {
        cat.age_hours = 30.0;
        cat.death_time = None;
        cat.activity = SimCatActivity::Idle;
        cat.current_task = None;
        cat.destination = None;
        cat.carrying = None;
        cat.needs.hunger = 100.0;
        cat.needs.thirst = 100.0;
        cat.needs.rest = 100.0;
        cat.needs.health = 100.0;
    }
    prepare_station_fixture(colony, &worker_ids[0], BuildingType::Mill);
    let recipes = station_recipe_set(BuildingType::Mill)
        .expect("Mill recipes")
        .recipes;
    let initial_queue = vec![
        ProductionQueueEntry {
            recipe_id: recipes[0].id.to_owned(),
            repeat: true,
        },
        ProductionQueueEntry {
            recipe_id: recipes[1].id.to_owned(),
            repeat: false,
        },
    ];
    let building = colony
        .buildings
        .iter_mut()
        .find(|building| building.id == "worker-playtest-station")
        .expect("queue fixture Mill");
    building.production_queue = initial_queue.clone();
    building.production_paused = true;
    building.additional_work_slots = vec![ProductionWorkSlot {
        assigned_cat: worker_ids[1].clone(),
        automated_by: None,
        production_progress: 0.0,
        production_queue: initial_queue,
        production_paused: true,
    }];
}

fn prepare_focused_public_action_fixture(world: &mut WorldState) {
    prepare_public_action_fixture(world);
    let world_seed = world.world_seed;
    let colony = &mut world.colonies[0];
    let worker_id = colony.cats[0].id.clone();
    prepare_farm_fixture(colony, &worker_id);
    prepare_fishing_fixture(colony, &worker_id, world_seed);
    for cat in &mut colony.cats {
        cat.position = Position {
            map: MapType::World,
            x: f64::from(colony.anchor.x),
            y: f64::from(colony.anchor.y),
        };
    }
    colony.stockpiles.push(stockpiles::Stockpile {
        id: "action-removable-stockpile".to_owned(),
        rect: ZoneRect {
            x1: colony.anchor.x + 20,
            y1: colony.anchor.y + 20,
            x2: colony.anchor.x + 20,
            y2: colony.anchor.y + 20,
        },
        accepts: [ResourceKind::Food].into_iter().collect(),
        contents: Resources::default(),
    });
    colony.transport.routes.insert(
        "action-cancellable-route".to_owned(),
        TransportRoute {
            id: "action-cancellable-route".to_owned(),
            mode: SimTransportMode::Rail,
            source_stockpile_id: "source".to_owned(),
            destination_stockpile_id: "destination".to_owned(),
            resource: ResourceKind::Food,
            amount: 1.0,
            assigned_cat_id: worker_id,
            phase: RoutePhase::Boarding,
            path: vec![colony.anchor],
            path_index: 0,
            segment_progress: 0.0,
            cargo_loaded: 0.0,
            vehicle_id: "action-route-vehicle".to_owned(),
            position: colony.anchor,
            repeat: false,
        },
    );
    colony.add_item(Item::new(ItemKind::Mug, Material::Wood, 1), 1);
    colony.trader = Some(TraderRuntime {
        id: "action-trader".to_owned(),
        position: Position::default(),
        destination: None,
        state: TraderState::Trading,
        arrived_at: Some(colony.last_tick),
        depart_at: Some(i64::MAX),
        route_exterior: None,
        visit_destination: None,
        route_blocked: false,
        visit_number: 1,
        stock: BTreeMap::new(),
        items: Default::default(),
        coin: trader::TRADER_STARTING_COIN,
    });
}

async fn retain_accepted_action(
    client: &mut WsClient,
    action: ClientAction,
    label: &str,
    failures: &mut Failures,
) {
    match client.send_action(&action).await {
        Ok(observed) => failures.check(observed.result.ok, || {
            format!("{label} legal fixture was rejected: {observed:?}")
        }),
        Err(error) => failures
            .0
            .push(format!("{label} returned no WebSocket result: {error}")),
    }
}

#[tokio::test]
async fn focused_public_actions_execute_accepted_real_websocket_behavior() {
    let mut harness = WsGameHarness::start_with(4_242, prepare_focused_public_action_fixture)
        .await
        .expect("start focused action harness");
    let (mut client, actor) = harness
        .connect_authenticated("focused-public-actions", "Focused Action Cat")
        .await
        .expect("authenticate focused action harness");
    let colony = &client.snapshot().colonies[0];
    let cat_id = colony.cats[0].id.clone();
    let fishing = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id == "worker-playtest-fishing")
        .expect("fixture fishing spot");
    let fishing_at = TilePoint {
        x: fishing.x1,
        y: fishing.y1,
    };
    let anchor = colony.anchor;
    let signed = |action: ClientAction| {
        action_with_actor(&action, &actor).expect("bind focused action identity")
    };
    let mut failures = Failures::default();

    retain_accepted_action(
        &mut client,
        signed(ClientAction::RequestJob {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            kind: ProtocolJobKind::HuntExpedition,
        }),
        "requestJob prerequisite for Boost",
        &mut failures,
    )
    .await;
    let snapshot = harness
        .advance_by(&mut client, 1)
        .await
        .expect("project requested Boost job");
    if let Some(job_id) = snapshot.colonies[0]
        .jobs
        .iter()
        .find(|job| job.kind == ProtocolJobKind::HuntExpedition)
        .map(|job| job.id.clone())
    {
        retain_accepted_action(
            &mut client,
            signed(ClientAction::Boost {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                job_id,
            }),
            "boost",
            &mut failures,
        )
        .await;
    } else {
        failures
            .0
            .push("Boost fixture projected no Hunt job".to_owned());
    }
    retain_accepted_action(
        &mut client,
        signed(ClientAction::PurchaseUpgrade {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            key: UpgradeKey::ClickPower,
        }),
        "purchaseUpgrade",
        &mut failures,
    )
    .await;
    retain_accepted_action(
        &mut client,
        signed(ClientAction::CreateZone {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            kind: ZoneKind::Avoid,
            a: anchor,
            b: anchor,
            duration_ms: 600_000,
        }),
        "createZone",
        &mut failures,
    )
    .await;
    let snapshot = harness
        .advance_by(&mut client, 1)
        .await
        .expect("project created zone");
    if let Some(zone_id) = snapshot.colonies[0]
        .zones
        .first()
        .map(|zone| zone.id.clone())
    {
        retain_accepted_action(
            &mut client,
            signed(ClientAction::RemoveZone {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                zone_id,
            }),
            "removeZone",
            &mut failures,
        )
        .await;
    } else {
        failures.0.push("CreateZone projected no zone".to_owned());
    }
    for (label, action) in [
        (
            "cancelTransportRoute",
            ClientAction::CancelTransportRoute {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                route_id: "action-cancellable-route".to_owned(),
            },
        ),
        (
            "clearFarm",
            ClientAction::ClearFarm {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                plot_id: "worker-playtest-farm".to_owned(),
            },
        ),
        (
            "removeStockpile",
            ClientAction::RemoveStockpile {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                stockpile_id: "action-removable-stockpile".to_owned(),
            },
        ),
        (
            "boostCat",
            ClientAction::BoostCat {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                cat_id: cat_id.clone(),
                boosted: true,
            },
        ),
        (
            "setCatLaborPreference",
            ClientAction::SetCatLaborPreference {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                cat_id: cat_id.clone(),
                labor: ProtocolLabor::Hunt,
                enabled: true,
            },
        ),
        (
            "sellGoods",
            ClientAction::SellGoods {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                kind: "mug".to_owned(),
                material: "wood".to_owned(),
                quality: 1,
                count: 1,
            },
        ),
    ] {
        retain_accepted_action(&mut client, signed(action), label, &mut failures).await;
    }
    retain_accepted_action(
        &mut client,
        signed(ClientAction::RemoveGatherSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            stockpile_id: "worker-playtest-fishing".to_owned(),
        }),
        "removeGatherSpot",
        &mut failures,
    )
    .await;
    retain_accepted_action(
        &mut client,
        signed(ClientAction::DesignateFishingSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            at: fishing_at,
        }),
        "designateFishingSpot",
        &mut failures,
    )
    .await;
    let snapshot = harness
        .advance_by(&mut client, 1)
        .await
        .expect("project re-designated fishing spot");
    if let Some(stockpile_id) = snapshot.colonies[0]
        .stockpiles
        .iter()
        .find(|pile| {
            pile.gather_spot.is_some() && pile.x1 == fishing_at.x && pile.y1 == fishing_at.y
        })
        .map(|pile| pile.id.clone())
    {
        retain_accepted_action(
            &mut client,
            signed(ClientAction::RemoveGatherSpot {
                session_id: sid(),
                nickname: nickname(),
                sig: bad_sig(),
                stockpile_id,
            }),
            "remove re-designated fishing spot",
            &mut failures,
        )
        .await;
    } else {
        failures
            .0
            .push("DesignateFishingSpot projected no gather spot".to_owned());
    }
    retain_accepted_action(
        &mut client,
        signed(ClientAction::DesignateGatherSpot {
            session_id: sid(),
            nickname: nickname(),
            sig: bad_sig(),
            a: fishing_at,
            b: fishing_at,
            kind: ProtocolResource::Stone,
        }),
        "designateGatherSpot",
        &mut failures,
    )
    .await;
    let snapshot = harness
        .advance_by(&mut client, 1)
        .await
        .expect("project focused action outcomes");
    if let Some(cat) = snapshot.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.id == cat_id)
    {
        failures.check(cat.boosted, || "BoostCat did not project true".to_owned());
        failures.check(cat.preferred_labors.contains(&ProtocolLabor::Hunt), || {
            "SetCatLaborPreference did not project Hunt".to_owned()
        });
    }
    failures.finish("focused accepted public ClientAction WebSocket sweep");
}

fn prepare_job_action_fixture(world: &mut WorldState, kind: ProtocolJobKind) {
    if kind == ProtocolJobKind::ReplantTree {
        let logging = EXECUTABLE_CASES
            .iter()
            .copied()
            .find(|case| case.labor == Labor::Woodcut)
            .expect("Woodcut worker case");
        prepare_worker_fixture(world, logging, 0.0);
        let world_seed = world.world_seed;
        let colony = &mut world.colonies[0];
        let anchor = colony.anchor;
        for runtime in colony.world_tiles.values_mut() {
            runtime.tile_type = TileType::Meadow;
            runtime.resources.water = 0;
            runtime.overlay_feature = None;
            colony.revealed_tiles.insert(runtime.pos);
        }
        let mut candidates = colony
            .world_tiles
            .keys()
            .copied()
            .filter(|site| {
                site.x.abs_diff(anchor.x).max(site.y.abs_diff(anchor.y)) > 8
                    && tile_has_tree(world_seed, site.x, site.y)
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|site| {
            (
                site.x.abs_diff(anchor.x).max(site.y.abs_diff(anchor.y)),
                site.y,
                site.x,
            )
        });
        let target = candidates
            .into_iter()
            .find(|site| {
                let old_overlay = colony
                    .world_tiles
                    .get_mut(site)
                    .and_then(|tile| tile.overlay_feature.replace("stump".to_owned()));
                let eligible = has_replant_site(colony, world_seed);
                if let Some(tile) = colony.world_tiles.get_mut(site) {
                    tile.overlay_feature = old_overlay;
                }
                eligible
            })
            .expect("mapped terrain contains a causal logging/replant site");
        let footprint = (0..TREE_FOOTPRINT_HEIGHT)
            .flat_map(|dy| {
                (0..TREE_FOOTPRINT_WIDTH).map(move |dx| cat_sim::world_tick::TilePos {
                    x: target.x + dx,
                    y: target.y + dy,
                })
            })
            .collect::<BTreeSet<_>>();
        colony.revealed_tiles = footprint;
        colony
            .world_tiles
            .get_mut(&target)
            .expect("selected logging tile")
            .overlay_feature = Some("stump".to_owned());
        assert!(
            has_replant_site(colony, world_seed),
            "selected logging site remains replantable under final fog fixture"
        );
        colony
            .world_tiles
            .get_mut(&target)
            .expect("selected logging tile")
            .overlay_feature = None;
        return;
    }
    if let Some(case) = EXECUTABLE_CASES.iter().copied().find(|case| {
        matches!(case.driver, WorkerDriver::Job(job_kind) if job_kind == kind)
            || kind == ProtocolJobKind::Explore && matches!(case.driver, WorkerDriver::Scout)
    }) {
        prepare_worker_fixture(world, case, 0.0);
        if kind == ProtocolJobKind::Explore {
            let colony = &mut world.colonies[0];
            colony.revealed_tiles = colony.claimed_tiles.iter().copied().collect();
            colony.provisional_tiles.clear();
        }
        return;
    }
    prepare_public_action_fixture(world);
}

#[tokio::test]
async fn every_job_kind_crosses_real_websocket_with_valid_behavior() {
    let kinds = JobKind::ALL
        .iter()
        .map(|kind| {
            serde_json::from_value::<ProtocolJobKind>(
                serde_json::to_value(kind).expect("serialize sim JobKind"),
            )
            .expect("sim and protocol JobKind catalogs stay aligned")
        })
        .collect::<Vec<_>>();
    assert_eq!(kinds.len(), 20);
    let mut failures = Failures::default();
    let mut observed_results = Vec::new();
    for (index, kind) in kinds.into_iter().enumerate() {
        let mut harness = match WsGameHarness::start_with(4_242, move |world| {
            prepare_job_action_fixture(world, kind);
        })
        .await
        {
            Ok(harness) => harness,
            Err(error) => {
                failures
                    .0
                    .push(format!("{kind:?} fixture failed to start: {error}"));
                continue;
            }
        };
        let (mut client, actor) = match harness
            .connect_authenticated(format!("job-kind-{index}"), format!("{kind:?} Cat"))
            .await
        {
            Ok(connected) => connected,
            Err(error) => {
                failures
                    .0
                    .push(format!("{kind:?} fixture failed to authenticate: {error}"));
                continue;
            }
        };
        let intended_replant_site = (kind == ProtocolJobKind::ReplantTree).then(|| {
            let sites = client.snapshot().colonies[0]
                .revealed_tiles
                .iter()
                .filter(|site| tile_has_tree(crate::WORLD_SEED, site.x, site.y))
                .copied()
                .collect::<Vec<_>>();
            failures.check(sites.len() == 1, || {
                format!(
                    "ReplantTree fixture exposed {} generated trees: {sites:?}",
                    sites.len()
                )
            });
            sites.first().copied()
        });
        if kind == ProtocolJobKind::ReplantTree {
            let logging = ClientAction::RequestJob {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                kind: ProtocolJobKind::GatherLogs,
            };
            match client.send_action(&logging).await {
                Ok(observed) => failures.check(observed.result.ok, || {
                    format!("ReplantTree logging prerequisite rejected: {observed:?}")
                }),
                Err(error) => failures
                    .0
                    .push(format!("ReplantTree logging prerequisite failed: {error}")),
            }
            if let Err(error) = harness
                .eventually(&mut client, 900_000, 10_000, |snapshot| {
                    !snapshot.colonies[0].stump_tiles.is_empty()
                        && !snapshot.colonies[0]
                            .jobs
                            .iter()
                            .any(|job| job.kind == ProtocolJobKind::GatherLogs)
                        && snapshot.colonies[0].cats.iter().any(|cat| {
                            cat.activity == cat_protocol::CatActivity::Idle && cat.age_hours >= 12.0
                        })
                })
                .await
            {
                failures.0.push(format!(
                    "ReplantTree produced no available logged stump: {error}"
                ));
            }
            if let Some(Some(intended)) = intended_replant_site {
                failures.check(
                    client.snapshot().colonies[0].stump_tiles == [intended],
                    || {
                        format!(
                            "GatherLogs did not create the intended reachable stump {intended:?}: {:?}",
                            client.snapshot().colonies[0].stump_tiles
                        )
                    },
                );
            }
        }
        let action = ClientAction::RequestJob {
            session_id: actor.session_id,
            nickname: actor.nickname,
            sig: actor.sig,
            kind,
        };
        match client.send_action(&action).await {
            Ok(observed) => {
                observed_results.push(serde_json::json!({
                    "kind": format!("{kind:?}"),
                    "result": observed.raw,
                }));
                let intentional_rejection = match kind {
                    ProtocolJobKind::BuildRoad => Some("Unknown job kind."),
                    ProtocolJobKind::PerformOffering => {
                        Some("An offering ritual begins only after physical shrine delivery.")
                    }
                    ProtocolJobKind::HaulGatherSpot => Some("Choose a gather spot to haul."),
                    ProtocolJobKind::BuildHouse => Some("Choose a building type to construct."),
                    _ => None,
                };
                if let Some(message) = intentional_rejection {
                    failures.check(!observed.result.ok, || {
                        format!("{kind:?} bypassed its required dedicated action")
                    });
                    failures.check(observed.result.message.as_deref() == Some(message), || {
                        format!("{kind:?} returned unexpected rejection {observed:?}")
                    });
                } else {
                    failures.check(observed.result.ok, || {
                        if kind == ProtocolJobKind::ReplantTree {
                            format!(
                                "legal ReplantTree RequestJob rejected after exact causal stump \
                                 proof {intended_replant_site:?}: {observed:?}"
                            )
                        } else {
                            format!("legal {kind:?} RequestJob rejected: {observed:?}")
                        }
                    });
                }
            }
            Err(error) => failures
                .0
                .push(format!("{kind:?} returned no WebSocket result: {error}")),
        }
    }
    failures.check(observed_results.len() == JobKind::ALL.len(), || {
        format!(
            "retained {} of {} JobKind WebSocket results",
            observed_results.len(),
            JobKind::ALL.len()
        )
    });
    failures.finish("real WebSocket JobKind catalog behavior sweep");
}

fn research_dependency_order() -> Result<Vec<String>, String> {
    let catalog = research_catalog();
    let mut remaining = catalog
        .nodes()
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let mut owned = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let ready = catalog
            .nodes()
            .iter()
            .filter(|node| remaining.contains(&node.id))
            .filter(|node| {
                node.prerequisites
                    .iter()
                    .all(|prerequisite| owned.contains(prerequisite))
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(format!(
                "research dependency graph stalled with {} nodes: {:?}",
                remaining.len(),
                remaining.iter().take(12).collect::<Vec<_>>()
            ));
        }
        for id in ready {
            remaining.remove(&id);
            owned.insert(id.clone());
            ordered.push(id);
        }
    }
    Ok(ordered)
}

#[tokio::test]
async fn every_research_study_is_purchased_in_dependency_order_and_persists() {
    let order = research_dependency_order().expect("acyclic research catalog");
    assert_eq!(order.len(), RESEARCH_NODE_COUNT);
    let mut harness = WsGameHarness::start_with(4_242, |world| {
        let colony = &mut world.colonies[0];
        colony.leader_id = None;
        colony.last_leader_research_choice_at = Some(colony.last_tick);
        colony.upgrade_tree.owned_node_ids.clear();
        colony.upgrade_tree.research_points = 1_000_000_000.0;
    })
    .await
    .expect("start research catalog harness");
    let (mut client, actor) = harness
        .connect_authenticated("research-catalog-install", "Catalog Scholar")
        .await
        .expect("authenticate research catalog harness");
    let mut failures = Failures::default();
    let mut observed_results = Vec::with_capacity(order.len());

    for batch in order.chunks(8) {
        for node_id in batch {
            let node = research_catalog()
                .get(node_id)
                .expect("ordered research node remains in catalog");
            failures.check(research_node_is_implemented(node), || {
                format!("{} has no implemented typed capability", node.id)
            });
            failures.check(!node.payloads.is_empty(), || {
                format!("{} exposes no typed payload", node.id)
            });
            let action = ClientAction::ResearchNode {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                node_id: node_id.clone(),
            };
            match client.send_action(&action).await {
                Ok(observed) => {
                    observed_results.push(serde_json::json!({
                        "nodeId": node_id,
                        "payloads": node.payloads,
                        "result": observed.raw,
                    }));
                    failures.check(observed.result.ok, || {
                        format!(
                            "{} legal dependency-ordered purchase rejected: {observed:?}",
                            node.id
                        )
                    });
                }
                Err(error) => failures
                    .0
                    .push(format!("{} returned no WebSocket result: {error}", node.id)),
            }
        }
        match harness.advance_by(&mut client, 60_001).await {
            Ok(snapshot) => {
                let owned = snapshot.colonies[0]
                    .research
                    .owned_node_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                for node_id in batch {
                    failures.check(owned.contains(node_id.as_str()), || {
                        format!("{node_id} purchase was not projected as authoritative ownership")
                    });
                }
            }
            Err(error) => failures
                .0
                .push(format!("research batch projection failed: {error}")),
        }
    }
    failures.check(observed_results.len() == RESEARCH_NODE_COUNT, || {
        format!(
            "retained {} of {RESEARCH_NODE_COUNT} research action results",
            observed_results.len()
        )
    });
    let before_restart = client.snapshot().colonies[0]
        .research
        .owned_node_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    match harness.restart_and_reconnect(client, &actor).await {
        Ok(reconnected) => {
            let restored = reconnected.snapshot().colonies[0]
                .research
                .owned_node_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            failures.check(restored == before_restart, || {
                format!(
                    "research ownership changed across restart: missing={:?}, extra={:?}",
                    before_restart
                        .difference(&restored)
                        .take(12)
                        .collect::<Vec<_>>(),
                    restored
                        .difference(&before_restart)
                        .take(12)
                        .collect::<Vec<_>>()
                )
            });
            failures.check(restored.len() == RESEARCH_NODE_COUNT, || {
                format!(
                    "restart restored {} of {RESEARCH_NODE_COUNT} studies",
                    restored.len()
                )
            });
        }
        Err(error) => failures
            .0
            .push(format!("research restart/reconnect failed: {error}")),
    }
    failures.finish("real WebSocket research dependency/persistence sweep");
}

fn projected_queue_state(
    snapshot: &cat_protocol::WorldSnapshot,
    slot_cat_id: Option<&str>,
) -> Option<(Vec<(String, bool)>, bool)> {
    let building = snapshot.colonies[0]
        .buildings
        .iter()
        .find(|building| building.id == "worker-playtest-station")?;
    let (queue, paused) = if let Some(cat_id) = slot_cat_id {
        let slot = building
            .work_slots
            .iter()
            .find(|slot| slot.cat_id == cat_id)?;
        (&slot.production_queue, slot.production_paused)
    } else {
        (&building.production_queue, building.production_paused)
    };
    Some((
        queue
            .iter()
            .map(|entry| (entry.recipe_id.clone(), entry.repeat))
            .collect(),
        paused,
    ))
}

async fn send_queue_edit_and_project(
    harness: &mut WsGameHarness,
    client: &mut WsClient,
    actor: &SignedActor,
    slot_cat_id: Option<&str>,
    edit: ProductionQueueEdit,
) -> Result<
    (
        crate::playtest_harness::ObservedActionResult,
        cat_protocol::WorldSnapshot,
    ),
    String,
> {
    let action = if let Some(cat_id) = slot_cat_id {
        ClientAction::EditProductionWorkSlot {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            building_id: "worker-playtest-station".to_owned(),
            cat_id: cat_id.to_owned(),
            edit,
        }
    } else {
        ClientAction::EditProductionQueue {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            building_id: "worker-playtest-station".to_owned(),
            edit,
        }
    };
    let observed = client.send_action(&action).await?;
    let snapshot = harness.advance_by(client, 1).await?;
    Ok((observed, snapshot))
}

#[tokio::test]
async fn production_queue_and_exact_work_slot_edits_project_every_operation() {
    let mut harness = WsGameHarness::start_with(4_242, prepare_queue_action_fixture)
        .await
        .expect("start production queue harness");
    let (mut client, actor) = harness
        .connect_authenticated("queue-operation-install", "Queue Cat")
        .await
        .expect("authenticate production queue harness");
    let colony = &client.snapshot().colonies[0];
    let secondary_cat_id = colony
        .buildings
        .iter()
        .find(|building| building.id == "worker-playtest-station")
        .and_then(|building| building.work_slots.get(1))
        .map(|slot| slot.cat_id.clone())
        .expect("fixture projects second work slot");
    let recipes = colony
        .buildings
        .iter()
        .find(|building| building.id == "worker-playtest-station")
        .expect("fixture projects Mill")
        .available_recipes
        .clone();
    assert!(recipes.len() >= 3, "Mill must expose three queue recipes");
    let initial = vec![(recipes[0].clone(), true), (recipes[1].clone(), false)];
    let after_add = vec![
        (recipes[0].clone(), true),
        (recipes[1].clone(), false),
        (recipes[2].clone(), false),
    ];
    let after_move = vec![
        (recipes[0].clone(), true),
        (recipes[2].clone(), false),
        (recipes[1].clone(), false),
    ];
    let after_repeat = vec![
        (recipes[0].clone(), true),
        (recipes[2].clone(), true),
        (recipes[1].clone(), false),
    ];
    let after_remove = vec![(recipes[2].clone(), true), (recipes[1].clone(), false)];
    let operations = vec![
        (
            "add",
            ProductionQueueEdit::Add {
                recipe_id: recipes[2].clone(),
                repeat: false,
            },
            after_add,
            true,
        ),
        (
            "move",
            ProductionQueueEdit::Move {
                index: 2,
                direction: QueueMoveDirection::Up,
            },
            after_move,
            true,
        ),
        (
            "set_repeat",
            ProductionQueueEdit::SetRepeat {
                index: 1,
                repeat: true,
            },
            after_repeat,
            true,
        ),
        (
            "remove",
            ProductionQueueEdit::Remove { index: 0 },
            after_remove.clone(),
            true,
        ),
        (
            "set_paused",
            ProductionQueueEdit::SetPaused { paused: false },
            after_remove,
            false,
        ),
    ];
    let mut failures = Failures::default();
    failures.check(
        projected_queue_state(client.snapshot(), None) == Some((initial.clone(), true)),
        || "building queue fixture did not project its initial state".to_owned(),
    );
    failures.check(
        projected_queue_state(client.snapshot(), Some(&secondary_cat_id)) == Some((initial, true)),
        || "work-slot queue fixture did not project its initial state".to_owned(),
    );

    for slot_cat_id in [None, Some(secondary_cat_id.as_str())] {
        let target = slot_cat_id.unwrap_or("building");
        for (operation, edit, expected_queue, expected_paused) in operations.clone() {
            match send_queue_edit_and_project(&mut harness, &mut client, &actor, slot_cat_id, edit)
                .await
            {
                Ok((observed, snapshot)) => {
                    failures.check(observed.result.ok, || {
                        format!("{target} {operation} rejected: {observed:?}")
                    });
                    failures.check(
                        projected_queue_state(&snapshot, slot_cat_id)
                            == Some((expected_queue.clone(), expected_paused)),
                        || {
                            format!(
                                "{target} {operation} projected {:?}, expected {:?}",
                                projected_queue_state(&snapshot, slot_cat_id),
                                (expected_queue, expected_paused)
                            )
                        },
                    );
                }
                Err(error) => failures.0.push(format!(
                    "{target} {operation} returned no projection: {error}"
                )),
            }
        }
    }
    failures.finish("real WebSocket production queue/work-slot operation sweep");
}

#[derive(Clone, Copy, Debug)]
enum WorkerDriver {
    Job(ProtocolJobKind),
    Build,
    Station(BuildingType),
    Farm,
    Research,
    Offering,
    Haul,
    Fight,
    Scout,
}

#[derive(Clone, Copy, Debug)]
struct ExecutableWorkerCase {
    scenario_id: &'static str,
    labor: Labor,
    driver: WorkerDriver,
}

const EXECUTABLE_CASES: &[ExecutableWorkerCase] = &[
    ExecutableWorkerCase {
        scenario_id: "worker-skill-hunt",
        labor: Labor::Hunt,
        driver: WorkerDriver::Job(ProtocolJobKind::HuntExpedition),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-fishing",
        labor: Labor::Fishing,
        driver: WorkerDriver::Job(ProtocolJobKind::Fish),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-build",
        labor: Labor::Build,
        driver: WorkerDriver::Build,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-ritual",
        labor: Labor::Ritual,
        driver: WorkerDriver::Offering,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-fight",
        labor: Labor::Fight,
        driver: WorkerDriver::Fight,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-train",
        labor: Labor::Train,
        driver: WorkerDriver::Job(ProtocolJobKind::TrainWarrior),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-quarry",
        labor: Labor::Quarry,
        driver: WorkerDriver::Job(ProtocolJobKind::Quarry),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-woodcut",
        labor: Labor::Woodcut,
        driver: WorkerDriver::Job(ProtocolJobKind::GatherLogs),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-forage",
        labor: Labor::Forage,
        driver: WorkerDriver::Job(ProtocolJobKind::ForageFibre),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-fetch-water",
        labor: Labor::FetchWater,
        driver: WorkerDriver::Job(ProtocolJobKind::FetchWater),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-mill",
        labor: Labor::Mill,
        driver: WorkerDriver::Station(BuildingType::Mill),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-process",
        labor: Labor::Process,
        driver: WorkerDriver::Station(BuildingType::Sawmill),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-craft",
        labor: Labor::Craft,
        driver: WorkerDriver::Station(BuildingType::Woodworking),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-textile",
        labor: Labor::Textile,
        driver: WorkerDriver::Station(BuildingType::Clothier),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-metalwork",
        labor: Labor::Metalwork,
        driver: WorkerDriver::Station(BuildingType::Smelter),
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-farm",
        labor: Labor::Farm,
        driver: WorkerDriver::Farm,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-haul",
        labor: Labor::Haul,
        driver: WorkerDriver::Haul,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-research",
        labor: Labor::Research,
        driver: WorkerDriver::Research,
    },
    ExecutableWorkerCase {
        scenario_id: "worker-skill-scout-vision-4-to-5",
        labor: Labor::Scout,
        driver: WorkerDriver::Scout,
    },
];

fn protocol_labor(labor: Labor) -> ProtocolLabor {
    match labor {
        Labor::Hunt => ProtocolLabor::Hunt,
        Labor::Fishing => ProtocolLabor::Fishing,
        Labor::Build => ProtocolLabor::Build,
        Labor::Ritual => ProtocolLabor::Ritual,
        Labor::Fight => ProtocolLabor::Fight,
        Labor::Train => ProtocolLabor::Train,
        Labor::Quarry => ProtocolLabor::Quarry,
        Labor::Woodcut => ProtocolLabor::Woodcut,
        Labor::Forage => ProtocolLabor::Forage,
        Labor::FetchWater => ProtocolLabor::FetchWater,
        Labor::Mill => ProtocolLabor::Mill,
        Labor::Process => ProtocolLabor::Process,
        Labor::Craft => ProtocolLabor::Craft,
        Labor::Textile => ProtocolLabor::Textile,
        Labor::Metalwork => ProtocolLabor::Metalwork,
        Labor::Farm => ProtocolLabor::Farm,
        Labor::Haul => ProtocolLabor::Haul,
        Labor::Research => ProtocolLabor::Research,
        Labor::Scout => ProtocolLabor::Scout,
    }
}

fn worker_skill(
    snapshot: &cat_protocol::WorldSnapshot,
    worker_id: &str,
    labor: ProtocolLabor,
) -> f64 {
    snapshot.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.id == worker_id)
        .and_then(|cat| cat.skills.get(&labor))
        .copied()
        .unwrap_or(0.0)
}

fn fixture_worker_id(
    snapshot: &cat_protocol::WorldSnapshot,
    labor: ProtocolLabor,
    expected: f64,
) -> Option<String> {
    snapshot.colonies[0]
        .cats
        .iter()
        .find(|cat| {
            cat.skills
                .get(&labor)
                .is_some_and(|skill| skill.to_bits() == expected.to_bits())
        })
        .map(|cat| cat.id.clone())
}

fn contains_full_provisional_square(snapshot: &cat_protocol::WorldSnapshot, side: i32) -> bool {
    let tiles = snapshot.colonies[0]
        .provisional_tiles
        .iter()
        .map(|tile| (tile.x, tile.y))
        .collect::<BTreeSet<_>>();
    tiles.iter().any(|(origin_x, origin_y)| {
        (0..side).all(|dx| (0..side).all(|dy| tiles.contains(&(origin_x + dx, origin_y + dy))))
    })
}

fn provisional_span_reaches(snapshot: &cat_protocol::WorldSnapshot, side: i32) -> bool {
    let tiles = &snapshot.colonies[0].provisional_tiles;
    let Some(min_x) = tiles.iter().map(|tile| tile.x).min() else {
        return false;
    };
    let max_x = tiles.iter().map(|tile| tile.x).max().unwrap_or(min_x);
    let min_y = tiles.iter().map(|tile| tile.y).min().unwrap_or_default();
    let max_y = tiles.iter().map(|tile| tile.y).max().unwrap_or(min_y);
    max_x - min_x + 1 >= side && max_y - min_y + 1 >= side
}

fn prepare_worker_fixture(world: &mut WorldState, case: ExecutableWorkerCase, initial_skill: f64) {
    let world_seed = world.world_seed;
    let colony = &mut world.colonies[0];
    colony.recipe_entitlement_rules_version = 0;
    colony.test_time_scale = match case.labor {
        Labor::Fishing
        | Labor::Quarry
        | Labor::Woodcut
        | Labor::Forage
        | Labor::FetchWater
        | Labor::Scout => 10.0,
        Labor::Haul => 1.0,
        _ => 60.0,
    };
    colony.status = cat_sim::entities::ColonyStatus::Thriving;
    for kind in ResourceKind::ALL {
        stockpiles::set_resource(&mut colony.resources, *kind, 500.0);
    }
    if let Some(store) = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.is_general_storehouse())
    {
        store.contents = colony.resources.clone();
    }

    let retained_leader = colony.cats[0].id.clone();
    let worker_id = colony
        .cats
        .iter()
        .filter(|cat| cat.id != retained_leader)
        .map(|cat| cat.id.clone())
        .next()
        .expect("fresh colony must contain a non-leader worker");
    colony.leader_id = Some(retained_leader.clone());
    for cat in &mut colony.cats {
        if cat.id == worker_id || cat.id == retained_leader {
            cat.death_time = None;
            cat.current_task = None;
            cat.activity = SimCatActivity::Idle;
            cat.destination = None;
            cat.carrying = None;
        } else {
            cat.death_time = Some(colony.created_at);
        }
    }
    let worker = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == worker_id)
        .expect("fixture worker");
    worker.skills.clear();
    worker.skills.insert(case.labor, initial_skill);
    worker.position = Position {
        map: MapType::World,
        x: f64::from(colony.anchor.x + 1),
        y: f64::from(colony.anchor.y + 1),
    };
    worker.needs.hunger = 100.0;
    worker.needs.thirst = 100.0;
    worker.needs.rest = 100.0;
    worker.needs.health = 100.0;

    let leader = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == retained_leader)
        .expect("fixture leader");
    // Keep a living leader for ordinary policy/session semantics, but make it a
    // kitten so it cannot steal the explicitly preferred worker's job or join the
    // deterministic combat muster.
    leader.age_hours = 0.0;
    leader.birth_time = colony.last_tick;
    leader.needs.hunger = 100.0;
    leader.needs.thirst = 100.0;
    leader.needs.rest = 100.0;
    leader.needs.health = 100.0;

    if case.labor == Labor::Scout {
        // The footprint assertion needs a genuinely hidden frontier. Keep the
        // claimed village geometry, but start with no previously revealed halo so
        // the first physical scout step exposes the complete skill-sized square.
        colony.revealed_tiles.clear();
        colony.provisional_tiles.clear();
        if let Some(worker) = colony.cats.iter_mut().find(|cat| cat.id == worker_id) {
            worker.position = Position {
                map: MapType::World,
                x: f64::from(colony.anchor.x + 10),
                y: f64::from(colony.anchor.y + 10),
            };
        }
    }

    let finite_yield_resource = match case.labor {
        Labor::Hunt => Some(ResourceKind::Food),
        Labor::Fishing => Some(ResourceKind::Fish),
        Labor::Quarry => Some(ResourceKind::Stone),
        Labor::Woodcut => Some(ResourceKind::Logs),
        Labor::Forage => Some(ResourceKind::Fibre),
        Labor::FetchWater => Some(ResourceKind::Water),
        _ => None,
    };
    if let Some(kind) = finite_yield_resource {
        stockpiles::set_resource(&mut colony.resources, kind, 0.0);
        for pile in &mut colony.stockpiles {
            stockpiles::set_resource(&mut pile.contents, kind, 0.0);
        }
    }
    if case.labor == Labor::Hunt
        && let Some(store) = colony
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
    {
        let x = colony.anchor.x - 100;
        let y = colony.anchor.y;
        store.rect = ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        };
    }

    match case.driver {
        WorkerDriver::Station(building_type) => {
            prepare_station_fixture(colony, &worker_id, building_type);
        }
        WorkerDriver::Farm => prepare_farm_fixture(colony, &worker_id),
        WorkerDriver::Research => prepare_research_fixture(colony, &worker_id),
        WorkerDriver::Fight => prepare_fight_fixture(colony, &worker_id),
        WorkerDriver::Haul => prepare_haul_fixture(colony),
        WorkerDriver::Job(ProtocolJobKind::Fish) => {
            prepare_fishing_fixture(colony, &worker_id, world_seed);
        }
        WorkerDriver::Job(ProtocolJobKind::GatherLogs) => {
            if !colony
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|id| id == "sawmill")
            {
                colony
                    .upgrade_tree
                    .owned_node_ids
                    .push("sawmill".to_owned());
            }
            let forests = colony
                .world_tiles
                .iter()
                .filter_map(|(pos, tile)| {
                    matches!(
                        tile.tile_type,
                        TileType::Forest
                            | TileType::DenseWoods
                            | TileType::OakForest
                            | TileType::PineForest
                            | TileType::Jungle
                    )
                    .then_some(*pos)
                })
                .collect::<Vec<_>>();
            colony.revealed_tiles.extend(forests);
        }
        WorkerDriver::Job(_)
        | WorkerDriver::Build
        | WorkerDriver::Offering
        | WorkerDriver::Scout => {}
    }
}

fn fixture_building(
    id: &str,
    building_type: BuildingType,
    colony: &cat_sim::world_tick::ColonyRuntime,
) -> BuildingRuntime {
    BuildingRuntime {
        id: id.to_owned(),
        building_type,
        level: 1,
        position: cat_sim::world_tick::TilePos {
            x: colony.anchor.x + 1,
            y: colony.anchor.y + 1,
        },
        is_complete: true,
        construction_progress: 100,
        ..BuildingRuntime::default()
    }
}

fn prepare_station_fixture(
    colony: &mut cat_sim::world_tick::ColonyRuntime,
    worker_id: &str,
    building_type: BuildingType,
) {
    let set = station_recipe_set(building_type).expect("worker station must own recipes");
    let recipe: &StationRecipeDescriptor = &set.recipes[0];
    let mut building = fixture_building("worker-playtest-station", building_type, colony);
    building.assigned_cat = Some(worker_id.to_owned());
    building.production_progress = 0.0;
    building.production_queue = vec![ProductionQueueEntry {
        recipe_id: recipe.id.to_owned(),
        repeat: true,
    }];
    let rect = ZoneRect {
        x1: building.position.x,
        y1: building.position.y,
        x2: building.position.x,
        y2: building.position.y,
    };
    let mut input = stockpiles::make_station_store(
        stockpiles::station_input_id(&building.id),
        rect,
        set.input_resources.iter().copied(),
    );
    for kind in recipe.input_resources {
        stockpiles::add_resource(&mut input.contents, *kind, 100.0);
        if let Some(store) = colony
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
        {
            stockpiles::add_resource(&mut store.contents, *kind, -100.0);
        }
    }
    let output = stockpiles::make_station_store(
        stockpiles::station_output_id(&building.id),
        rect,
        set.output_resources.iter().copied(),
    );
    colony.stockpiles.push(input);
    colony.stockpiles.push(output);
    colony.buildings.push(building);
}

fn prepare_farm_fixture(colony: &mut cat_sim::world_tick::ColonyRuntime, _worker_id: &str) {
    let mut building = fixture_building("worker-playtest-field", BuildingType::Field, colony);
    building.assigned_cat = Some(_worker_id.to_owned());
    let x = colony.anchor.x + 7;
    let y = colony.anchor.y + 7;
    for tile_x in (x - 1)..=(x + 1) {
        for tile_y in (y - 1)..=(y + 1) {
            let tile = cat_sim::world_tick::TilePos {
                x: tile_x,
                y: tile_y,
            };
            if !colony.claimed_tiles.contains(&tile) {
                colony.claimed_tiles.push(tile);
            }
            colony.revealed_tiles.insert(tile);
        }
    }
    colony
        .agricultural_tiles
        .insert(cat_sim::world_tick::TilePos { x, y });
    if let Some(worker) = colony.cats.iter_mut().find(|cat| cat.id == _worker_id) {
        worker.position = Position {
            map: MapType::World,
            x: f64::from(x),
            y: f64::from(y),
        };
    }
    colony.buildings.push(building);
    colony.farms.push(FarmPlot {
        id: "worker-playtest-farm".to_owned(),
        rect: ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        },
        crop: SimCropKind::Grain,
        planted_at: colony.last_tick,
        stage: FarmStage::Soil,
        growth_hours: 0.0,
        fertility: 1.0,
        worker_id: None,
        work_phase: FarmWorkPhase::Planting,
        pending_output: 0.0,
    });
    if let Some(plot) = colony
        .farms
        .iter_mut()
        .find(|plot| plot.id == "worker-playtest-farm")
    {
        plot.worker_id = Some(_worker_id.to_owned());
    }
}

fn prepare_research_fixture(colony: &mut cat_sim::world_tick::ColonyRuntime, _worker_id: &str) {
    let mut building = fixture_building(
        "worker-playtest-research",
        BuildingType::ResearchHut,
        colony,
    );
    building.assigned_cat = Some(_worker_id.to_owned());
    colony.buildings.push(building);
}

fn prepare_haul_fixture(colony: &mut cat_sim::world_tick::ColonyRuntime) {
    // Keep pickup close, then place every eligible destination on the opposite
    // reachable edge. This makes the skill-governed carrying leg long without
    // asking pathfinding to leave the generated world component.
    let x = colony.anchor.x + 100;
    let y = colony.anchor.y + 6;
    let mut pile = stockpiles::Stockpile {
        id: "worker-playtest-gather".to_owned(),
        rect: ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        },
        accepts: [ResourceKind::Materials].into_iter().collect(),
        contents: Resources::default(),
    };
    pile.contents.materials = 4.0;
    stockpiles::set_resource(&mut colony.resources, ResourceKind::Materials, 4.0);
    for existing in &mut colony.stockpiles {
        stockpiles::set_resource(&mut existing.contents, ResourceKind::Materials, 0.0);
        existing.accepts.remove(&ResourceKind::Materials);
    }
    colony.stockpiles.push(stockpiles::Stockpile {
        id: "worker-playtest-destination".to_owned(),
        rect: ZoneRect {
            x1: colony.anchor.x - 100,
            y1: y,
            x2: colony.anchor.x - 100,
            y2: y,
        },
        accepts: [ResourceKind::Materials].into_iter().collect(),
        contents: Resources::default(),
    });
    colony.stockpiles.push(pile);
    colony.gather_spots.push(stockpiles::GatherSpot {
        stockpile_id: "worker-playtest-gather".to_owned(),
        kind: ResourceKind::Materials,
        expires_at_ms: colony.last_tick + 86_400_000,
        purpose: stockpiles::GatherSpotPurpose::General,
    });
}

fn prepare_fishing_fixture(
    colony: &mut cat_sim::world_tick::ColonyRuntime,
    worker_id: &str,
    world_seed: u32,
) {
    let (bank, water) = colony
        .world_tiles
        .iter()
        .filter(|(_, tile)| tile.tile_type == TileType::River)
        .find_map(|(water, _)| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .map(|(dx, dy)| cat_sim::world_tick::TilePos {
                    x: water.x + dx,
                    y: water.y + dy,
                })
                .find(|bank| {
                    colony
                        .world_tiles
                        .get(bank)
                        .is_some_and(|tile| tile.tile_type != TileType::River)
                        && cat_sim::world_tick::stockpile_placement_error(
                            colony,
                            ZoneRect {
                                x1: bank.x,
                                y1: bank.y,
                                x2: bank.x,
                                y2: bank.y,
                            },
                            world_seed,
                            false,
                        )
                        .is_none()
                })
                .map(|bank| (bank, *water))
        })
        .unwrap_or_else(|| {
            let bank = cat_sim::world_tick::TilePos {
                x: colony.anchor.x + 2,
                y: colony.anchor.y + 2,
            };
            (
                bank,
                cat_sim::world_tick::TilePos {
                    x: bank.x,
                    y: bank.y + 1,
                },
            )
        });
    colony.revealed_tiles.insert(bank);
    colony.revealed_tiles.insert(water);
    if let Some(worker) = colony.cats.iter_mut().find(|cat| cat.id == worker_id) {
        worker.position = Position {
            map: MapType::World,
            x: f64::from(bank.x),
            y: f64::from(bank.y),
        };
    }
    let x = bank.x;
    let y = bank.y;
    colony.stockpiles.push(stockpiles::Stockpile {
        id: "worker-playtest-fishing".to_owned(),
        rect: ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        },
        accepts: [ResourceKind::Fish].into_iter().collect(),
        contents: Resources::default(),
    });
    colony.gather_spots.push(stockpiles::GatherSpot {
        stockpile_id: "worker-playtest-fishing".to_owned(),
        kind: ResourceKind::Fish,
        expires_at_ms: colony.last_tick + 86_400_000,
        purpose: stockpiles::GatherSpotPurpose::Fishing,
    });
    colony.fish_habitats.insert(
        water,
        stockpiles::FishPopulation {
            stock: 24.0,
            capacity: 24.0,
            last_replenished_at_ms: colony.last_tick,
        },
    );
}

fn prepare_fight_fixture(colony: &mut cat_sim::world_tick::ColonyRuntime, worker_id: &str) {
    let worker = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == worker_id)
        .expect("fixture worker");
    worker.specialization = Some(CatSpecialization::Warrior);
    worker.stats.attack = 5.0;
    worker.stats.defense = 5.0;
    if !colony
        .upgrade_tree
        .owned_node_ids
        .iter()
        .any(|id| id == "barracks")
    {
        colony
            .upgrade_tree
            .owned_node_ids
            .push("barracks".to_owned());
    }
    colony
        .officers
        .insert(OfficerRole::Captain, worker_id.to_owned());
    let gate = Position {
        map: MapType::World,
        x: f64::from(colony.anchor.x),
        y: f64::from(colony.anchor.y + village_ring_radius(colony.buildings.len() as i32)),
    };
    colony.active_raid = Some("worker-playtest-raid".to_owned());
    colony.raiders = vec![RaiderRuntime {
        id: "worker-playtest-raider".to_owned(),
        raid_id: "worker-playtest-raid".to_owned(),
        position: gate,
        destination: None,
        attack: 1.0,
        defense: 1.0,
        health: 21.0,
    }];
}

fn signed_worker_action(
    case: ExecutableWorkerCase,
    actor: &SignedActor,
    worker_id: &str,
) -> ClientAction {
    let credentials = || {
        (
            actor.session_id.clone(),
            actor.nickname.clone(),
            actor.sig.clone(),
        )
    };
    match case.driver {
        WorkerDriver::Job(kind) => {
            let (session_id, nickname, sig) = credentials();
            if kind == ProtocolJobKind::TrainWarrior {
                ClientAction::TrainWarrior {
                    session_id,
                    nickname,
                    sig,
                    cat_id: Some(worker_id.to_owned()),
                }
            } else {
                ClientAction::RequestJob {
                    session_id,
                    nickname,
                    sig,
                    kind,
                }
            }
        }
        WorkerDriver::Build => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::PlanBuilding {
                session_id,
                nickname,
                sig,
                building_type: ProtocolBuildingType::Den,
                site: None,
            }
        }
        WorkerDriver::Station(_) => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::EditProductionQueue {
                session_id,
                nickname,
                sig,
                building_id: "worker-playtest-station".to_owned(),
                edit: ProductionQueueEdit::SetPaused { paused: false },
            }
        }
        WorkerDriver::Farm => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::AssignWorker {
                session_id,
                nickname,
                sig,
                cat_id: worker_id.to_owned(),
                building_id: Some("worker-playtest-field".to_owned()),
            }
        }
        WorkerDriver::Research => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::AssignWorker {
                session_id,
                nickname,
                sig,
                cat_id: worker_id.to_owned(),
                building_id: Some("worker-playtest-research".to_owned()),
            }
        }
        WorkerDriver::Offering => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::OfferMaterials {
                session_id,
                nickname,
                sig,
            }
        }
        WorkerDriver::Haul => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::HaulGatherSpot {
                session_id,
                nickname,
                sig,
                stockpile_id: "worker-playtest-gather".to_owned(),
                cat_id: Some(worker_id.to_owned()),
            }
        }
        WorkerDriver::Fight => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::DefendRaid {
                session_id,
                nickname,
                sig,
            }
        }
        WorkerDriver::Scout => {
            let (session_id, nickname, sig) = credentials();
            ClientAction::DispatchScout {
                session_id,
                nickname,
                sig,
                mission: ScoutMission::Explore,
            }
        }
    }
}

async fn trace_worker_failure(
    case: ExecutableWorkerCase,
    harness: &WsGameHarness,
    client: &WsClient,
    last_completed_milestone: Option<&'static str>,
    failure: &str,
    restart_difference: Option<&serde_json::Value>,
) -> String {
    let trace = FailureTrace {
        scenario_id: case.scenario_id,
        seed: harness.seed,
        last_completed_milestone,
        simulated_time_ms: harness.now_ms(),
        action_results: &client.action_results,
        snapshot: client.snapshot(),
        restart_difference,
        failure,
    };
    match write_failure_trace(&trace) {
        Ok(path) => format!("{failure} (trace: {})", path.display()),
        Err(error) => format!("{failure} (trace write failed: {error})"),
    }
}

#[allow(dead_code)]
async fn run_executable_worker_case_in_one_world_legacy(
    case: ExecutableWorkerCase,
    seed: u32,
) -> Result<(), String> {
    let scenario = SCENARIOS
        .iter()
        .find(|scenario| scenario.id == case.scenario_id)
        .ok_or_else(|| format!("{} is absent from manifest", case.scenario_id))?;
    let fixture_skill = if case.labor == Labor::Scout {
        4.0
    } else {
        24.0
    };
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_worker_fixture(world, case, fixture_skill)
    })
    .await?;
    let (mut client, actor) = harness
        .connect_authenticated(format!("{}-install", case.scenario_id), "Worker Playtester")
        .await?;
    let protocol_labor = protocol_labor(case.labor);
    let expected_before: f64 = if case.labor == Labor::Scout {
        4.0
    } else {
        24.0
    };
    let worker_id = fixture_worker_id(client.snapshot(), protocol_labor, expected_before)
        .ok_or_else(|| "fixture exposed no exact worker".to_owned())?;
    let before = worker_skill(client.snapshot(), &worker_id, protocol_labor);
    if before.to_bits() != expected_before.to_bits() {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            None,
            &format!("fixture skill was {before}, expected {expected_before}"),
            None,
        )
        .await);
    }

    let preference = ClientAction::SetCatLaborPreference {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        cat_id: worker_id.clone(),
        labor: protocol_labor,
        enabled: true,
    };
    let preference_result = client.send_action(&preference).await?;
    if !preference_result.result.ok {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            None,
            &format!(
                "signed labor preference rejected: {:?}",
                preference_result.result.message
            ),
            None,
        )
        .await);
    }
    let action = signed_worker_action(case, &actor, &worker_id);
    let first_started_at = harness.now_ms();
    let action_result = client.send_action(&action).await?;
    if !action_result.result.ok {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("signed-control-accepted"),
            &format!(
                "signed work action rejected: {:?}",
                action_result.result.message
            ),
            None,
        )
        .await);
    }

    let first_scout_footprint = if case.labor == Labor::Scout {
        let _observed = match harness
            .eventually(&mut client, scenario.horizon_ms, 1_000, |snapshot| {
                contains_full_provisional_square(snapshot, 5)
            })
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(trace_worker_failure(
                    case,
                    &harness,
                    &client,
                    Some("physical-work-started"),
                    &format!("first Scout footprint was not observed: {error}"),
                    None,
                )
                .await);
            }
        };
        Some(5)
    } else {
        None
    };

    let reached = harness
        .eventually(&mut client, scenario.horizon_ms, 30_000, |snapshot| {
            worker_skill(snapshot, &worker_id, protocol_labor) > before
        })
        .await;
    let snapshot = match reached {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("physical-work-started"),
                &error,
                None,
            )
            .await);
        }
    };
    let first_elapsed = harness.now_ms() - first_started_at;

    if matches!(
        case.driver,
        WorkerDriver::Job(_)
            | WorkerDriver::Build
            | WorkerDriver::Offering
            | WorkerDriver::Haul
            | WorkerDriver::Scout
    ) {
        let ready = harness
            .eventually(&mut client, scenario.horizon_ms, 30_000, |snapshot| {
                snapshot.colonies[0]
                    .cats
                    .iter()
                    .find(|cat| cat.id == worker_id)
                    .is_some_and(|cat| {
                        cat.current_task.is_none()
                            && cat.carrying.is_none()
                            && cat.activity == cat_protocol::CatActivity::Idle
                    })
            })
            .await;
        if let Err(error) = ready {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("skill-threshold-crossed"),
                &format!("worker never became available for the second work unit: {error}"),
                None,
            )
            .await);
        }
    }

    let second_worker_id = if case.labor == Labor::Train {
        fixture_worker_id(&snapshot, protocol_labor, 25.0)
            .filter(|candidate| candidate != &worker_id)
            .ok_or_else(|| "fixture exposed no second valid Train recruit".to_owned())?
    } else {
        worker_id.clone()
    };
    if matches!(case.driver, WorkerDriver::Station(_)) {
        let assign = ClientAction::AssignWorker {
            session_id: actor.session_id.clone(),
            nickname: actor.nickname.clone(),
            sig: actor.sig.clone(),
            cat_id: worker_id.clone(),
            building_id: Some("worker-playtest-station".to_owned()),
        };
        let result = client.send_action(&assign).await?;
        if !result.result.ok {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("skill-threshold-crossed"),
                &format!(
                    "second station staffing action was rejected: {:?}",
                    result.result.message
                ),
                None,
            )
            .await);
        }
    }
    let second_before = worker_skill(&snapshot, &second_worker_id, protocol_labor);
    let second_action = signed_worker_action(case, &actor, &second_worker_id);
    let second_started_at = harness.now_ms();
    let second_result = client.send_action(&second_action).await?;
    if !second_result.result.ok {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("skill-threshold-crossed"),
            &format!(
                "second equivalent signed work unit was rejected: {:?}",
                second_result.result.message
            ),
            None,
        )
        .await);
    }
    let second_scout_footprint = if case.labor == Labor::Scout {
        let _observed = match harness
            .eventually(&mut client, scenario.horizon_ms, 1_000, |snapshot| {
                contains_full_provisional_square(snapshot, 6)
            })
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(trace_worker_failure(
                    case,
                    &harness,
                    &client,
                    Some("skill-threshold-crossed"),
                    &format!("second Scout footprint was not observed: {error}"),
                    None,
                )
                .await);
            }
        };
        Some(6)
    } else {
        None
    };
    let second_snapshot = match harness
        .eventually(&mut client, scenario.horizon_ms, 30_000, |snapshot| {
            worker_skill(snapshot, &second_worker_id, protocol_labor) > second_before
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("skill-threshold-crossed"),
                &format!("second equivalent work unit did not complete: {error}"),
                None,
            )
            .await);
        }
    };
    let second_elapsed = harness.now_ms() - second_started_at;
    if second_elapsed >= first_elapsed {
        return Err(trace_worker_failure(case, &harness, &client, Some("skill-threshold-crossed"), &format!("second equivalent real work unit was not faster: {first_elapsed}ms -> {second_elapsed}ms"), None).await);
    }
    if case.labor == Labor::Scout
        && (first_scout_footprint != Some(25) || second_scout_footprint != Some(36))
    {
        return Err(trace_worker_failure(case, &harness, &client, Some("skill-threshold-crossed"), &format!("measured Scout provisional footprints were {:?} then {:?}, expected 25 (5x5) then 36 (6x6)", first_scout_footprint, second_scout_footprint), None).await);
    }

    let persisted = worker_skill(&second_snapshot, &worker_id, protocol_labor);
    client = harness.restart_and_reconnect(client, &actor).await?;
    let restored = worker_skill(client.snapshot(), &worker_id, protocol_labor);
    if restored.to_bits() != persisted.to_bits() {
        let difference =
            serde_json::json!({ "beforeRestart": persisted, "afterRestart": restored });
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("productivity-effect-observed"),
            "worker skill changed across restart",
            Some(&difference),
        )
        .await);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct WorkObservation {
    elapsed_ms: i64,
    scout_footprint_side: Option<i32>,
    effect_metric: f64,
    initial_skill: f64,
}

fn scheduled_effect_metric(
    snapshot: &cat_protocol::WorldSnapshot,
    case: ExecutableWorkerCase,
) -> Option<f64> {
    let expected_kind = match case.driver {
        WorkerDriver::Job(kind) => kind,
        WorkerDriver::Build => ProtocolJobKind::BuildHouse,
        WorkerDriver::Offering => ProtocolJobKind::PerformOffering,
        WorkerDriver::Haul => ProtocolJobKind::HaulGatherSpot,
        WorkerDriver::Scout => ProtocolJobKind::Explore,
        WorkerDriver::Station(_)
        | WorkerDriver::Farm
        | WorkerDriver::Research
        | WorkerDriver::Fight => return None,
    };
    snapshot.colonies[0]
        .jobs
        .iter()
        .filter(|job| job.kind == expected_kind && job.ends_at > job.started_at)
        .max_by_key(|job| job.started_at)
        .map(|job| -(job.ends_at - job.started_at) as f64)
}

fn fixed_window_effect_metric(
    snapshot: &cat_protocol::WorldSnapshot,
    case: ExecutableWorkerCase,
) -> Option<f64> {
    match case.driver {
        WorkerDriver::Station(_) => snapshot.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "worker-playtest-station")
            .map(|building| building.production_progress),
        WorkerDriver::Farm => snapshot.colonies[0]
            .farms
            .iter()
            .find(|farm| farm.id == "worker-playtest-farm")
            .map(|farm| farm.growth_hours),
        WorkerDriver::Research => Some(snapshot.colonies[0].research.research_points),
        _ => None,
    }
}

fn continuous_work_units(
    snapshot: &cat_protocol::WorldSnapshot,
    case: ExecutableWorkerCase,
    worker_id: &str,
    labor: ProtocolLabor,
    initial_skill: f64,
) -> Option<f64> {
    let base = fixed_window_effect_metric(snapshot, case)?;
    Some(match case.driver {
        WorkerDriver::Station(_) => base + worker_skill(snapshot, worker_id, labor) - initial_skill,
        WorkerDriver::Farm | WorkerDriver::Research => base,
        _ => return None,
    })
}

fn finite_yield_metric(
    snapshot: &cat_protocol::WorldSnapshot,
    worker_id: &str,
    labor: Labor,
) -> Option<f64> {
    if !matches!(
        labor,
        Labor::Hunt
            | Labor::Fishing
            | Labor::Quarry
            | Labor::Woodcut
            | Labor::Forage
            | Labor::FetchWater
    ) {
        return None;
    }
    let carried = snapshot.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.id == worker_id)
        .and_then(|cat| cat.carrying.as_ref())
        .map_or(0.0, |cargo| cargo.amount);
    let fishing_depletion = if labor == Labor::Fishing {
        snapshot.colonies[0]
            .stockpiles
            .iter()
            .filter_map(|pile| pile.gather_spot.as_ref()?.fish_population)
            .map(|population| (population.capacity - population.stock).max(0.0))
            .sum()
    } else {
        0.0
    };
    Some(carried + fishing_depletion)
}

fn carried_water_trip(
    snapshot: &cat_protocol::WorldSnapshot,
    worker_id: &str,
) -> Option<(i64, f64)> {
    let cargo = snapshot.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.id == worker_id)?
        .carrying
        .as_ref()?;
    (cargo.kind == cat_protocol::CarryingKind::Water).then_some((cargo.job_ended_at, cargo.amount))
}

async fn run_isolated_worker_unit(
    case: ExecutableWorkerCase,
    seed: u32,
    initial_skill: f64,
    required_skill: f64,
    expected_scout_side: Option<i32>,
    persist: bool,
    baseline: Option<WorkObservation>,
) -> Result<WorkObservation, String> {
    let scenario = SCENARIOS
        .iter()
        .find(|scenario| scenario.id == case.scenario_id)
        .ok_or_else(|| format!("{} is absent from manifest", case.scenario_id))?;
    let mut harness = WsGameHarness::start_with(seed, move |world| {
        prepare_worker_fixture(world, case, initial_skill)
    })
    .await?;
    let install = format!("{}-{initial_skill}-install", case.scenario_id);
    let (mut client, actor) = harness
        .connect_authenticated(install, "Worker Playtester")
        .await?;
    let labor = protocol_labor(case.labor);
    let Some(worker_id) = fixture_worker_id(client.snapshot(), labor, initial_skill) else {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            None,
            &format!("fixture exposed no worker at {initial_skill} XP"),
            None,
        )
        .await);
    };

    let preference = ClientAction::SetCatLaborPreference {
        session_id: actor.session_id.clone(),
        nickname: actor.nickname.clone(),
        sig: actor.sig.clone(),
        cat_id: worker_id.clone(),
        labor,
        enabled: true,
    };
    let preference_result = client.send_action(&preference).await?;
    if !preference_result.result.ok {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            None,
            &format!(
                "signed labor preference rejected: {:?}",
                preference_result.result.message
            ),
            None,
        )
        .await);
    }

    let action = signed_worker_action(case, &actor, &worker_id);
    let started_at = harness.now_ms();
    let action_result = client.send_action(&action).await?;
    if !action_result.result.ok {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("signed-control-accepted"),
            &format!(
                "signed work action rejected: {:?}",
                action_result.result.message
            ),
            None,
        )
        .await);
    }

    let continuous = matches!(
        case.driver,
        WorkerDriver::Station(_) | WorkerDriver::Farm | WorkerDriver::Research
    );
    let yield_case = matches!(
        case.labor,
        Labor::Hunt
            | Labor::Fishing
            | Labor::Quarry
            | Labor::Woodcut
            | Labor::Forage
            | Labor::FetchWater
    );
    let mut observed_water_trips = BTreeSet::new();
    let mut accumulated_water = 0.0;
    let mut peak_yield =
        finite_yield_metric(client.snapshot(), &worker_id, case.labor).unwrap_or_default();
    let mut effect_metric = scheduled_effect_metric(client.snapshot(), case);
    if continuous {
        let active = match harness
            .eventually(&mut client, 60_000, 1_000, |snapshot| {
                continuous_work_units(snapshot, case, &worker_id, labor, initial_skill)
                    .is_some_and(|units| units > 0.0)
                    || snapshot.colonies[0]
                        .cats
                        .iter()
                        .find(|cat| cat.id == worker_id)
                        .is_some_and(|cat| cat.activity == cat_protocol::CatActivity::Working)
            })
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return Err(trace_worker_failure(
                    case,
                    &harness,
                    &client,
                    Some("signed-control-accepted"),
                    &format!("continuous worker never became physically active: {error}"),
                    None,
                )
                .await);
            }
        };
        let start_units = continuous_work_units(&active, case, &worker_id, labor, initial_skill)
            .unwrap_or_default();
        let end = harness.advance_by(&mut client, 1_000).await?;
        effect_metric = continuous_work_units(&end, case, &worker_id, labor, initial_skill)
            .map(|end_units| end_units - start_units);
    } else if matches!(case.driver, WorkerDriver::Haul) {
        let carrying = harness
            .eventually(&mut client, 600_000, 30_000, |snapshot| {
                snapshot.colonies[0]
                    .cats
                    .iter()
                    .find(|cat| cat.id == worker_id)
                    .is_some_and(|cat| cat.carrying.is_some())
            })
            .await?;
        debug_assert!(
            carrying.colonies[0]
                .cats
                .iter()
                .find(|cat| cat.id == worker_id)
                .is_some_and(|cat| cat.carrying.is_some())
        );
        let return_started_at = harness.now_ms();
        harness
            .eventually(&mut client, 1_000_000, 100, |snapshot| {
                snapshot.colonies[0]
                    .cats
                    .iter()
                    .find(|cat| cat.id == worker_id)
                    .is_some_and(|cat| cat.carrying.is_none())
            })
            .await?;
        effect_metric = Some(-((harness.now_ms() - return_started_at) as f64));
    } else {
        let probe = harness.advance_by(&mut client, 1_000).await?;
        if let Some((trip, amount)) = carried_water_trip(&probe, &worker_id)
            && observed_water_trips.insert(trip)
        {
            accumulated_water += amount;
        }
        if yield_case {
            peak_yield = peak_yield
                .max(finite_yield_metric(&probe, &worker_id, case.labor).unwrap_or_default());
        }
        if effect_metric.is_none() {
            effect_metric = scheduled_effect_metric(&probe, case);
        }
    }

    if let Some(side) = expected_scout_side
        && let Err(error) = harness
            .eventually(&mut client, scenario.horizon_ms, 1_000, |snapshot| {
                provisional_span_reaches(snapshot, side)
            })
            .await
    {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("physical-work-started"),
            &format!("Scout {side}x{side} footprint was not physically observed: {error}"),
            None,
        )
        .await);
    }

    let completion_cadence_ms = 1_000;
    let completion_horizon_ms = match case.labor {
        Labor::Hunt | Labor::Haul => 900_000,
        _ if yield_case => 900_000,
        _ => 600_000,
    };
    let completed = match harness
        .eventually(
            &mut client,
            scenario.horizon_ms.min(completion_horizon_ms),
            completion_cadence_ms,
            |snapshot| {
                if yield_case {
                    if let Some((trip, amount)) = carried_water_trip(snapshot, &worker_id)
                        && observed_water_trips.insert(trip)
                    {
                        accumulated_water += amount;
                    }
                    peak_yield = peak_yield.max(
                        finite_yield_metric(snapshot, &worker_id, case.labor).unwrap_or_default(),
                    );
                }
                if effect_metric.is_none() {
                    effect_metric = scheduled_effect_metric(snapshot, case);
                }
                worker_skill(snapshot, &worker_id, labor) + f64::EPSILON >= required_skill
            },
        )
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("physical-work-started"),
                &format!("accepted real work did not increase the exact worker's XP: {error}"),
                None,
            )
            .await);
        }
    };
    if matches!(case.driver, WorkerDriver::Fight) {
        effect_metric = Some(f64::from(completed.colonies[0].events.iter().any(
            |event| event.kind == "raid_won" || event.message.contains("drove the raiders off"),
        )));
    }
    if matches!(
        case.labor,
        Labor::Hunt | Labor::Fishing | Labor::Quarry | Labor::Woodcut | Labor::Forage
    ) {
        effect_metric = Some(
            peak_yield
                .max(finite_yield_metric(&completed, &worker_id, case.labor).unwrap_or_default()),
        );
    }
    if case.labor == Labor::FetchWater {
        effect_metric = Some(accumulated_water);
    }
    if matches!(case.labor, Labor::Hunt | Labor::Forage) {
        // These finite jobs have floor-quantized yields. Adjacent XP values can
        // retain the same yield while their skill-scaled real duration improves.
        effect_metric = Some(-((harness.now_ms() - started_at) as f64));
    }
    if let Some(side) = expected_scout_side {
        effect_metric = Some(f64::from(side));
    }
    let Some(effect_metric) = effect_metric else {
        return Err(trace_worker_failure(
            case,
            &harness,
            &client,
            Some("real-work-completed"),
            "authoritative productivity effect was not exposed by the projected work state",
            None,
        )
        .await);
    };
    let observation = WorkObservation {
        elapsed_ms: harness.now_ms() - started_at,
        scout_footprint_side: expected_scout_side,
        effect_metric,
        initial_skill,
    };

    if let Some(baseline) = baseline {
        if observation.effect_metric <= baseline.effect_metric {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("skill-threshold-crossed"),
                &format!(
                    "isolated equivalent real work did not expose a productivity increase: effect {} at XP {} versus {} at XP {} (completion observations {}ms and {}ms)",
                    baseline.effect_metric,
                    baseline.initial_skill,
                    observation.effect_metric,
                    initial_skill,
                    baseline.elapsed_ms,
                    observation.elapsed_ms,
                ),
                None,
            )
            .await);
        }
        if case.labor == Labor::Scout
            && (baseline.scout_footprint_side != Some(5)
                || observation.scout_footprint_side != Some(6))
        {
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("skill-threshold-crossed"),
                &format!(
                    "actual Scout footprints were {:?} then {:?}, expected 5x5 then 6x6",
                    baseline.scout_footprint_side, observation.scout_footprint_side
                ),
                None,
            )
            .await);
        }
    }

    if persist {
        if let WorkerDriver::Station(_) = case.driver {
            let pause = ClientAction::EditProductionQueue {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                building_id: "worker-playtest-station".to_owned(),
                edit: ProductionQueueEdit::SetPaused { paused: true },
            };
            let result = client.send_action(&pause).await?;
            if !result.result.ok {
                return Err(trace_worker_failure(
                    case,
                    &harness,
                    &client,
                    Some("skill-threshold-crossed"),
                    &format!(
                        "could not pause continuous work before restart: {:?}",
                        result.result.message
                    ),
                    None,
                )
                .await);
            }
        }
        if matches!(
            case.driver,
            WorkerDriver::Station(_) | WorkerDriver::Farm | WorkerDriver::Research
        ) {
            let unassign = ClientAction::AssignWorker {
                session_id: actor.session_id.clone(),
                nickname: actor.nickname.clone(),
                sig: actor.sig.clone(),
                cat_id: worker_id.clone(),
                building_id: None,
            };
            let result = client.send_action(&unassign).await?;
            if !result.result.ok {
                return Err(trace_worker_failure(
                    case,
                    &harness,
                    &client,
                    Some("skill-threshold-crossed"),
                    &format!(
                        "could not stop continuous work before restart: {:?}",
                        result.result.message
                    ),
                    None,
                )
                .await);
            }
        }
        let stopped = harness.advance_by(&mut client, 1).await?;
        let stopped_skill = worker_skill(&stopped, &worker_id, labor);
        client = harness.restart_and_reconnect(client, &actor).await?;
        let restored = worker_skill(client.snapshot(), &worker_id, labor);
        if restored.to_bits() != stopped_skill.to_bits() {
            let difference = serde_json::json!({
                "beforeRestart": stopped_skill,
                "afterRestart": restored,
            });
            return Err(trace_worker_failure(
                case,
                &harness,
                &client,
                Some("real-work-completed"),
                "worker XP changed across stopped-work restart/reconnect",
                Some(&difference),
            )
            .await);
        }
    }

    Ok(observation)
}

async fn run_executable_worker_case(case: ExecutableWorkerCase, seed: u32) -> Result<(), String> {
    let initial_skill = match case.labor {
        Labor::Hunt => 11.0,
        Labor::Fishing => 7.0,
        Labor::Quarry | Labor::Woodcut => 5.0,
        Labor::FetchWater | Labor::Scout => 4.0,
        Labor::Build | Labor::Farm | Labor::Research => 24.0,
        Labor::Fight => 23.0,
        Labor::Haul => 0.75,
        _ => 0.0,
    };
    let comparison_skill = if case.labor == Labor::Haul {
        1.0
    } else {
        initial_skill + 1.0
    };
    let gain = match case.labor {
        Labor::Fight => 4.0,
        Labor::Haul => 0.25,
        _ => 1.0,
    };
    let baseline_side = (case.labor == Labor::Scout).then_some(5);
    let comparison_side = (case.labor == Labor::Scout).then_some(6);
    let baseline = run_isolated_worker_unit(
        case,
        seed,
        initial_skill,
        initial_skill + gain,
        baseline_side,
        true,
        None,
    )
    .await?;
    run_isolated_worker_unit(
        case,
        seed,
        comparison_skill,
        comparison_skill + gain,
        comparison_side,
        false,
        Some(baseline),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn real_websocket_worker_progression_runs_every_manifest_case_and_aggregates_reds() {
    assert_eq!(EXECUTABLE_CASES.len(), 19);
    assert_eq!(EXECUTABLE_SCENARIO_IDS.len(), 19);
    let mut failures = Vec::new();
    let requested_tier = super::requested_seed_tier();
    for case in EXECUTABLE_CASES {
        SCENARIOS
            .iter()
            .find(|scenario| scenario.id == case.scenario_id)
            .expect("executable case must have a manifest entry");
        let seeds = requested_tier.seeds();
        for seed in seeds {
            if let Err(error) = run_executable_worker_case(*case, *seed).await {
                failures.push(format!("{} seed {}: {error}", case.scenario_id, seed));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "worker WebSocket scenario failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
