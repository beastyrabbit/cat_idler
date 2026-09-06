//! Synthetic fixture generator. Refuses all existing paths and has no live-save input.
#[allow(dead_code)]
#[path = "../../../legacy/persistence.rs"]
mod persistence;

fn station_outputs(colony: &mut cat_sim::world_tick::ColonyRuntime) {
    use cat_sim::{
        entities::{Carrying, CarryingKind},
        items::{Item, ItemKind, ItemLocation, Material, StationCompartment},
        stockpiles::{self, GENERAL_STOREHOUSE_ID, ResourceKind},
        types::BuildingType,
        world_tick::{BuildingRuntime, ProductionQueueEntry},
    };

    let destination = colony
        .stockpiles
        .iter_mut()
        .find(|pile| pile.id == GENERAL_STOREHOUSE_ID)
        .expect("starter destination");
    destination.contents.planks = 10.0;
    destination.contents.refined = 7.0;
    destination.contents.tools = 1.0;
    destination.contents.weapons = 1.0;
    destination.contents.armor = 1.0;
    colony.resources.planks = 10.0;
    colony.resources.refined = 7.0;
    colony.resources.tools = 1.0;
    colony.resources.weapons = 1.0;
    colony.resources.armor = 1.0;

    let cases = [
        (
            "planks",
            BuildingType::WoodCutter,
            "logs_to_planks",
            CarryingKind::Planks,
            None,
        ),
        (
            "tools",
            BuildingType::Woodworking,
            "planks_and_blocks_to_tools",
            CarryingKind::Tools,
            Some((ItemKind::Tool, Material::Wood)),
        ),
        (
            "weapons",
            BuildingType::Smithy,
            "smithy_weapon",
            CarryingKind::Weapons,
            Some((ItemKind::Weapon, Material::Metal)),
        ),
        (
            "armor",
            BuildingType::Smithy,
            "smithy_armor",
            CarryingKind::Armor,
            Some((ItemKind::Armor, Material::Metal)),
        ),
        (
            "trinket",
            BuildingType::Workshop,
            "gem_jewelry",
            CarryingKind::Refined,
            Some((ItemKind::Trinket, Material::Gem)),
        ),
    ];
    for (index, (label, building_type, recipe, carrying_kind, exact)) in
        cases.into_iter().enumerate()
    {
        let cat_index = index + 2;
        let cat_id = colony.cats[cat_index].id.clone();
        colony.cats[cat_index].name = format!("Fixture output {label}");
        let building_id = format!("fixture-output-{label}");
        colony.buildings.push(BuildingRuntime {
            id: building_id.clone(),
            building_type,
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(cat_id.clone()),
            production_queue: vec![ProductionQueueEntry {
                recipe_id: recipe.into(),
                repeat: false,
            }],
            ..BuildingRuntime::default()
        });
        // Frozen world_tick.rs emits a destination here, never a transit mirror.
        let mut marker = format!("station-out|{building_id}|{GENERAL_STOREHOUSE_ID}");
        let amount = if let Some((kind, material)) = exact {
            let count = if kind == ItemKind::Trinket { 1 } else { 2 };
            let ids = colony.items.add_at(
                Item::new(kind, material, 2),
                count,
                1.0,
                ItemLocation::Carrier {
                    cat_id: cat_id.clone(),
                },
                false,
            );
            for (offset, id) in ids.iter().enumerate() {
                let item = colony.items.instance_mut(id).expect("new carrier item");
                item.durability = 17 + offset as u32;
                item.max_durability = 42 + offset as u32;
            }
            colony.items.add_at(
                Item::new(kind, material, 1),
                1,
                1.0,
                ItemLocation::Station {
                    building_id: building_id.clone(),
                    compartment: StationCompartment::LocalOutput,
                },
                false,
            );
            if kind == ItemKind::Trinket {
                // Nonfunctional exact output uses Refined as compatibility cargo.
                marker.push_str(&format!("|item:{}", ids[0]));
            } else {
                // Functional output keeps an unsuffixed marker for all carried IDs.
                // The remaining station identity also has a legacy scalar mirror.
                let resource = match kind {
                    ItemKind::Tool => ResourceKind::Tools,
                    ItemKind::Weapon => ResourceKind::Weapons,
                    ItemKind::Armor => ResourceKind::Armor,
                    _ => unreachable!(),
                };
                let mut output = stockpiles::make_station_store(
                    stockpiles::station_output_id(&building_id),
                    cat_sim::zones::ZoneRect {
                        x1: 0,
                        y1: 0,
                        x2: 0,
                        y2: 0,
                    },
                    [resource],
                );
                stockpiles::add_resource(&mut output.contents, resource, 1.0);
                colony.stockpiles.push(output);
                colony.items.add_at(
                    Item::new(kind, material, 3),
                    1,
                    1.0,
                    ItemLocation::Stockpile {
                        stockpile_id: GENERAL_STOREHOUSE_ID.into(),
                    },
                    true,
                );
                colony.items.add_at(
                    Item::new(kind, material, 4),
                    1,
                    1.0,
                    ItemLocation::Equipped {
                        cat_id: cat_id.clone(),
                    },
                    true,
                );
            }
            f64::from(count)
        } else {
            3.0
        };
        colony.cats[cat_index].carrying = Some(Carrying {
            kind: carrying_kind,
            amount,
            job_ended_at: 1_000_000,
            source_gather_spot: Some(marker),
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cat_sim::{
        entities::{Carrying, CarryingKind},
        world_tick::{found_global_colony, new_world, register_colony_spatial},
    };
    let path = std::env::args()
        .nth(1)
        .ok_or("new synthetic destination required")?;
    let scenario = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "station-input".into());
    if scenario != "station-input" && scenario != "station-output" {
        return Err("unknown synthetic scenario".into());
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(&path)?;
    let conn = rusqlite::Connection::open(&path)?;
    persistence::init_schema(&conn)?;
    let mut world = new_world(42);
    let mut colony = found_global_colony(42, "synthetic-commons", 1_000_000, 4242);
    colony.cats[0].name = "Fixture Moss".into();
    colony.cats[0].age_hours = 47.25;
    colony.cats[0].needs.hunger = 71.0;
    colony.cats[0].carrying = Some(Carrying {
        kind: CarryingKind::Logs,
        amount: 2.5,
        job_ended_at: 1_000_000,
        source_gather_spot: None,
    });
    colony.cats[0].parent_ids = vec![Some("synthetic-parent".into()), None];
    let worker_id = colony.cats[1].id.clone();
    let bench = colony
        .buildings
        .iter_mut()
        .find(|b| b.building_type == cat_sim::types::BuildingType::WoodCutter)
        .expect("starter bench");
    bench.assigned_cat = Some(worker_id);
    bench.production_progress = 599.0;
    let bench_id = bench.id.clone();
    let at = bench.position;
    let rect = cat_sim::zones::ZoneRect {
        x1: at.x,
        y1: at.y,
        x2: at.x,
        y2: at.y,
    };
    let transit_id = cat_sim::stockpiles::station_transit_id(&bench_id);
    let mut input = cat_sim::stockpiles::make_station_store(
        cat_sim::stockpiles::station_input_id(&bench_id),
        rect,
        [cat_sim::stockpiles::ResourceKind::Logs],
    );
    input.contents.logs = 2.0;
    let mut transit = cat_sim::stockpiles::make_station_store(
        transit_id.clone(),
        rect,
        [cat_sim::stockpiles::ResourceKind::Logs],
    );
    transit.contents.logs = 3.0;
    colony.stockpiles.extend([input, transit]);
    colony.resources.logs += 5.0;
    colony.cats[1].carrying = Some(Carrying {
        kind: CarryingKind::Logs,
        amount: 3.0,
        job_ended_at: 1_000_000,
        source_gather_spot: Some(format!("station-in|{bench_id}|{transit_id}")),
    });
    if scenario == "station-output" {
        station_outputs(&mut colony);
    }
    world.colonies.push(colony);
    register_colony_spatial(&mut world, 0);
    persistence::save_world(&conn, &world)?;
    println!(
        "Created one synthetic communal world with {} cats.",
        world.colonies[0].cats.len()
    );
    Ok(())
}
