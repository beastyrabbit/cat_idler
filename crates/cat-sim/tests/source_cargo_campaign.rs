//! Deterministic unattended and signed-player campaigns for P19.C1 source cargo.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::CarryingKind,
    types::{BuildingType, JobKind, JobStatus},
    world_tick::{WorldState, found_colony, new_world, world_tick},
};

const START: i64 = 10_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "source-cargo-session".to_owned(),
        player_id: "source-cargo-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn run_passive_hunts(seed: u32) -> (WorldState, bool, bool) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let mut saw_hide_in_paws = false;
    let mut saw_bone_in_paws = false;
    for minute in 1..=24 * 60i64 {
        let now = START + minute * 60_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None, "passive minute {minute}");
        saw_hide_in_paws |= world.colonies[0].cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Hide)
        });
        saw_bone_in_paws |= world.colonies[0].cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Bone)
        });
    }
    (world, saw_hide_in_paws, saw_bone_in_paws)
}

#[test]
fn unattended_founder_hunts_physically_return_hide_deterministically() {
    let (left, left_saw_cargo, left_saw_bone) = run_passive_hunts(7);
    let (right, right_saw_cargo, right_saw_bone) = run_passive_hunts(7);
    assert_eq!(left, right);
    assert_eq!(left_saw_cargo, right_saw_cargo);
    assert_eq!(left_saw_bone, right_saw_bone);
    assert!(
        left_saw_cargo,
        "the passive run never showed Hide in a cat's paws"
    );
    assert!(left.colonies[0].resources.hide > 0.0);
    assert!(
        left_saw_bone,
        "the passive run never showed Bone in a cat's paws"
    );
    assert!(left.colonies[0].resources.bone > 0.0);
}

fn run_signed_quarry(seed: u32) -> (WorldState, bool, bool) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    assert_eq!(world.colonies[0].resources.stone, 0.0);
    // Fixture-assisted visibility only: this grants no resource, ownership, office,
    // worker, or job. The signed action still passes the real quarry-site gate.
    let loaded = world.colonies[0]
        .world_tiles
        .keys()
        .copied()
        .collect::<Vec<_>>();
    world.colonies[0].revealed_tiles.extend(loaded);

    let accelerated = apply_action(
        &mut world,
        &proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        &ctx(START),
    );
    assert!(accelerated.ok, "acceleration fixture failed");
    let stone_prep_id = world.colonies[0]
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::StonePrep && building.is_complete)
        .expect("fresh village has a completed Stone Prep")
        .id
        .clone();
    let worker_id = world.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none() && cat.activity == Default::default())
        .expect("fresh village has an available player-directed worker")
        .id
        .clone();
    let blocks_at_founding = world.colonies[0].resources.blocks;
    let assigned = apply_action(
        &mut world,
        &proto::ClientAction::AssignWorker {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            cat_id: worker_id,
            building_id: Some(stone_prep_id),
        },
        &ctx(START + 1),
    );
    assert!(
        assigned.ok,
        "signed Stone Prep assignment failed: {assigned:?}"
    );
    let ordered = apply_action(
        &mut world,
        &proto::ClientAction::RequestJob {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            kind: proto::JobKind::Quarry,
        },
        &ctx(START + 2),
    );
    assert!(ordered.ok, "signed quarry failed: {:?}", ordered.message);

    let mut saw_stone_in_paws = false;
    let mut dressed_quarried_stone = false;
    for second in 1..=2_400i64 {
        let reports = world_tick(&mut world, START + second * 1_000);
        assert_eq!(reports[0].reset_reason, None, "guided second {second}");
        saw_stone_in_paws |= world.colonies[0].cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Stone)
        });
        let quarry_done = world.colonies[0]
            .jobs
            .iter()
            .any(|job| job.kind == JobKind::Quarry && job.status == JobStatus::Completed);
        if quarry_done && world.colonies[0].resources.blocks > blocks_at_founding {
            dressed_quarried_stone = true;
            break;
        }
    }
    (world, saw_stone_in_paws, dressed_quarried_stone)
}

#[test]
fn signed_player_quarry_physically_returns_raw_stone_deterministically() {
    let (left, left_saw_cargo, left_dressed) = run_signed_quarry(0xCA7C_0100);
    let (right, right_saw_cargo, right_dressed) = run_signed_quarry(0xCA7C_0100);
    assert_eq!(left, right);
    assert_eq!(left_saw_cargo, right_saw_cargo);
    assert_eq!(left_dressed, right_dressed);
    assert!(
        left_saw_cargo,
        "the signed run never showed Stone in a cat's paws"
    );
    assert!(
        left_dressed,
        "the signed Stone Prep never dressed quarried Stone: stone={}, blocks={}, prep={:?}",
        left.colonies[0].resources.stone,
        left.colonies[0].resources.blocks,
        left.colonies[0]
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::StonePrep)
            .map(|building| (&building.assigned_cat, building.production_progress))
    );
}
