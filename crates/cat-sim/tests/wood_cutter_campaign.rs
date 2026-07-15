//! Deterministic signed-player campaign for the physical Wood Cutter chain.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::CarryingKind,
    stockpiles::{ResourceKind, resource_amount, station_input_id, station_output_id},
    types::{BuildingType, JobKind, JobStatus},
    world_tick::{WorldState, found_colony, new_world, world_tick},
};

const START: i64 = 20_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "wood-cutter-session".to_owned(),
        player_id: "wood-cutter-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn signed_assign(cat_id: String, building_id: String) -> proto::ClientAction {
    proto::ClientAction::AssignWorker {
        session_id: "wood-cutter-session".to_owned(),
        nickname: "Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        cat_id,
        building_id: Some(building_id),
    }
}

fn signed_queue(building_id: String, edit: proto::ProductionQueueEdit) -> proto::ClientAction {
    proto::ClientAction::EditProductionQueue {
        session_id: "wood-cutter-session".to_owned(),
        nickname: "Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        building_id,
        edit,
    }
}

fn run_guided_wood_cutter(seed: u32) -> (WorldState, bool, bool, bool) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));

    // This fixture grants only the logging-job entitlement and map visibility
    // needed to exercise the actions. The founding Wood Cutter recipe itself
    // deliberately needs no study. The player still chooses the
    // worker, orders the logging job, and configures the real production queue.
    let colony = &mut world.colonies[0];
    colony
        .upgrade_tree
        .owned_node_ids
        .push("sawmill".to_owned());
    colony
        .revealed_tiles
        .extend(colony.world_tiles.keys().copied());
    let wood_cutter = colony
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::WoodCutter)
        .expect("founding village has a Wood Cutter")
        .id
        .clone();
    let worker = colony
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none() && cat.activity == Default::default())
        .expect("founding village has an idle worker")
        .id
        .clone();
    let planks_at_start = colony.resources.planks;

    let accelerated = apply_action(
        &mut world,
        &proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        &ctx(START),
    );
    assert!(accelerated.ok, "acceleration fixture failed");

    let assigned = apply_action(
        &mut world,
        &signed_assign(worker, wood_cutter.clone()),
        &ctx(START + 1),
    );
    assert!(assigned.ok, "signed assignment failed: {assigned:?}");

    // Replace the founding default through the same queue controls exposed by the
    // inspector, proving an explicitly ordered repeating batch drives the chain.
    let removed = apply_action(
        &mut world,
        &signed_queue(
            wood_cutter.clone(),
            proto::ProductionQueueEdit::Remove { index: 0 },
        ),
        &ctx(START + 2),
    );
    assert!(removed.ok, "signed queue removal failed: {removed:?}");
    let added = apply_action(
        &mut world,
        &signed_queue(
            wood_cutter.clone(),
            proto::ProductionQueueEdit::Add {
                recipe_id: "logs_to_planks".to_owned(),
                repeat: true,
            },
        ),
        &ctx(START + 3),
    );
    assert!(added.ok, "signed queue addition failed: {added:?}");

    let ordered = apply_action(
        &mut world,
        &proto::ClientAction::RequestJob {
            session_id: "wood-cutter-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            kind: proto::JobKind::GatherLogs,
        },
        &ctx(START + 4),
    );
    assert!(ordered.ok, "signed logging order failed: {ordered:?}");

    let input_id = station_input_id(&wood_cutter);
    let output_id = station_output_id(&wood_cutter);
    let mut saw_logs_in_paws = false;
    let mut saw_station_local_logs = false;
    let mut saw_station_local_planks = false;
    for second in 1..=2_400i64 {
        let reports = world_tick(&mut world, START + second * 1_000);
        assert_eq!(reports[0].reset_reason, None, "guided second {second}");
        let colony = &world.colonies[0];
        saw_logs_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Logs)
        });
        saw_station_local_logs |= colony.stockpiles.iter().any(|pile| {
            pile.id == input_id
                && resource_amount(&pile.contents, ResourceKind::Logs) > f64::EPSILON
        });
        saw_station_local_planks |= colony.stockpiles.iter().any(|pile| {
            pile.id == output_id
                && resource_amount(&pile.contents, ResourceKind::Planks) > f64::EPSILON
        });
        let logging_done = colony
            .jobs
            .iter()
            .any(|job| job.kind == JobKind::GatherLogs && job.status == JobStatus::Completed);
        if logging_done && colony.resources.planks > planks_at_start {
            break;
        }
    }

    let colony = &world.colonies[0];
    assert!(
        colony.resources.planks > planks_at_start,
        "the signed logging order never became banked planks: logs={}, planks={}",
        colony.resources.logs,
        colony.resources.planks
    );
    assert!(
        colony
            .world_tiles
            .values()
            .any(|tile| tile.overlay_feature.as_deref() == Some("stump")),
        "the guided harvest never left its physical stump"
    );
    (
        world,
        saw_logs_in_paws,
        saw_station_local_logs,
        saw_station_local_planks,
    )
}

#[test]
fn signed_player_guides_tree_to_repeating_wood_cutter_deterministically() {
    let left = run_guided_wood_cutter(42);
    let right = run_guided_wood_cutter(42);

    assert_eq!(
        left, right,
        "same signed campaign and seed must replay exactly"
    );
    assert!(
        left.1,
        "the run never showed harvested Logs in a cat's paws"
    );
    assert!(left.2, "the run never admitted Logs to Wood Cutter input");
    assert!(left.3, "the run never created Wood Cutter-local Planks");
}
