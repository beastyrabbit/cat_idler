//! Focused LAI.60 farm/road/barrier contracts. Execution is reserved for the
//! serialized feature verification lane.

use cat_sim::{
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
    village_infrastructure::{
        BarrierKind, BarrierTileProject, FarmPlotProject, FarmStage, FreeLaborDestination,
        InfrastructureActor, PhysicalInputProgress, PhysicalTileWork, RoadProject, TileWorkStage,
        VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION, VillageInfrastructureState, VillagePriorityReport,
        authorize_exact_infrastructure_action, free_labor_destination,
    },
};

fn input(id: &str, units: u64) -> PhysicalInputProgress {
    PhysicalInputProgress {
        definition_id: id.to_owned(),
        required_units: units,
        delivered_units: 0,
        in_transit_units: 0,
        consumed_units: 0,
    }
}

fn tile_work(tile: TilePoint) -> PhysicalTileWork {
    PhysicalTileWork {
        tile,
        stage: TileWorkStage::Preview,
        material: input("stone", 1),
        labor_required_minutes: 10,
        labor_completed_minutes: 0,
    }
}

#[test]
fn lai60_farm_is_a_visible_crop_assigned_physical_world_sequence() {
    let mut farm = FarmPlotProject {
        schema_version: VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION,
        plot_id: "farm-1".to_owned(),
        footprint: TaskFootprint::rectangular(Rect::new(TilePoint { x: 4, y: 4 }, 2, 2).unwrap()),
        crop_id: "apple_rootstock".to_owned(),
        stage: FarmStage::Reserved,
        seed: input("apple_seed", 1),
        clear_required_minutes: 10,
        clear_completed_minutes: 0,
        sow_required_minutes: 5,
        sow_completed_minutes: 0,
        grow_required_minutes: 30,
        grow_elapsed_minutes: 0,
        harvest_required_minutes: 8,
        harvest_completed_minutes: 0,
    };
    farm.begin_clearing().unwrap();
    farm.record_clearing(10).unwrap();
    farm.seed.reserve_in_transit(1).unwrap();
    farm.seed.deliver(1).unwrap();
    farm.begin_sowing().unwrap();
    farm.record_sowing(5).unwrap();
    farm.record_growth(30).unwrap();
    farm.begin_harvest().unwrap();
    farm.record_harvest(8).unwrap();
    assert_eq!(farm.stage, FarmStage::Fallow);
    farm.validate().unwrap();
}

#[test]
fn lai60_road_keeps_authored_preview_and_completes_physical_tiles() {
    let route = vec![
        TilePoint { x: 0, y: 0 },
        TilePoint { x: 1, y: 0 },
        TilePoint { x: 1, y: 1 },
    ];
    let mut road = RoadProject {
        schema_version: VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION,
        road_id: "road-1".to_owned(),
        route_preview: route.clone(),
        tiles: route.iter().copied().map(tile_work).collect(),
    };
    let first = &mut road.tiles[0];
    first.reserve_material(1).unwrap();
    first.deliver_material(1).unwrap();
    first.begin_labor().unwrap();
    first.record_labor(10).unwrap();
    assert_eq!(road.completed_tiles(), [TilePoint { x: 0, y: 0 }]);
    road.validate().unwrap();
}

#[test]
fn lai60_walls_block_and_operational_gates_are_the_only_crossing() {
    let tile = TilePoint { x: 8, y: 8 };
    let mut wall_work = tile_work(tile);
    wall_work.reserve_material(1).unwrap();
    wall_work.deliver_material(1).unwrap();
    wall_work.begin_labor().unwrap();
    wall_work.record_labor(10).unwrap();
    let wall = BarrierTileProject {
        barrier_id: "wall-1".to_owned(),
        kind: BarrierKind::Wall,
        work: wall_work,
        gate_open: false,
    };
    assert!(wall.blocks_crossing());

    let gate_tile = TilePoint { x: 9, y: 8 };
    let mut gate_work = tile_work(gate_tile);
    gate_work.reserve_material(1).unwrap();
    gate_work.deliver_material(1).unwrap();
    gate_work.begin_labor().unwrap();
    gate_work.record_labor(10).unwrap();
    let mut gate = BarrierTileProject {
        barrier_id: "gate-1".to_owned(),
        kind: BarrierKind::Gate,
        work: gate_work,
        gate_open: false,
    };
    assert!(gate.blocks_crossing());
    gate.set_gate_open(true).unwrap();
    assert!(!gate.blocks_crossing());

    let state = VillageInfrastructureState {
        schema_version: VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION,
        farms: Default::default(),
        roads: Default::default(),
        barriers: [
            (wall.barrier_id.clone(), wall),
            (gate.barrier_id.clone(), gate),
        ]
        .into_iter()
        .collect(),
    };
    state.validate().unwrap();
    assert!(state.tile_blocks_crossing(tile));
    assert!(!state.tile_blocks_crossing(gate_tile));
}

#[test]
fn lai60_ai_owns_exact_actions_and_village_demand_precedes_hole_dependencies() {
    authorize_exact_infrastructure_action(InfrastructureActor::Leader).unwrap();
    authorize_exact_infrastructure_action(InfrastructureActor::Steward).unwrap();
    authorize_exact_infrastructure_action(InfrastructureActor::Farmer).unwrap();
    assert!(authorize_exact_infrastructure_action(InfrastructureActor::God).is_err());
    assert_eq!(
        free_labor_destination(VillagePriorityReport {
            survival_adequately_staffed: true,
            defense_adequately_staffed: true,
            active_village_plans_adequately_staffed: false,
        }),
        FreeLaborDestination::VillageDemand
    );
    assert_eq!(
        free_labor_destination(VillagePriorityReport {
            survival_adequately_staffed: true,
            defense_adequately_staffed: true,
            active_village_plans_adequately_staffed: true,
        }),
        FreeLaborDestination::UsefulHoleDependency
    );
}
