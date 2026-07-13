//! Deterministic player-action campaign at the simulation's public action boundary.
//!
//! This deliberately supplies prerequisites (buildings, an election, a raid, a trader)
//! instead of waiting game-days for them. It is a coverage campaign, not a balance test:
//! every `ClientAction` variant reaches `apply_action`, every accepted mutation is
//! asserted, and the complete campaign repeats bit-for-bit.

use std::collections::{BTreeSet, HashSet};

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::{CatActivity, MapType, Position},
    items::{Item, ItemKind, Material},
    trader::{self, TraderState},
    types::{BuildingType, JobKind},
    upgrade_tree::{self, UPGRADE_NODES},
    world_tick::{
        BuildingRuntime, ElectionKind, RaiderRuntime, TilePos, TraderRuntime, WorldState,
        new_world, road_path_attaches_to_shrine, road_placement_error, stockpile_placement_error,
    },
    zones::ZoneRect,
};

const EXPECTED_ACTIONS: [&str; 29] = [
    "advance_time",
    "assign_officer",
    "assign_worker",
    "boost",
    "boost_cat",
    "build_road",
    "buy_resource",
    "cast_vote",
    "create_zone",
    "defend_raid",
    "designate_gather_spot",
    "designate_stockpile",
    "ensure",
    "found_village",
    "join_village",
    "plan_building",
    "presence",
    "purchase_upgrade",
    "remove_gather_spot",
    "remove_stockpile",
    "remove_zone",
    "request_job",
    "request_vote_kick",
    "sell_goods",
    "set_test_acceleration",
    "set_test_rng_seed",
    "train_warrior",
    "unassign_officer",
    "unlock_node",
];

fn action_name(action: &proto::ClientAction) -> &'static str {
    match action {
        proto::ClientAction::Ensure => "ensure",
        proto::ClientAction::Presence { .. } => "presence",
        proto::ClientAction::RequestJob { .. } => "request_job",
        proto::ClientAction::Boost { .. } => "boost",
        proto::ClientAction::PurchaseUpgrade { .. } => "purchase_upgrade",
        proto::ClientAction::CastVote { .. } => "cast_vote",
        proto::ClientAction::RequestVoteKick { .. } => "request_vote_kick",
        proto::ClientAction::CreateZone { .. } => "create_zone",
        proto::ClientAction::RemoveZone { .. } => "remove_zone",
        proto::ClientAction::PlanBuilding { .. } => "plan_building",
        proto::ClientAction::UnlockNode { .. } => "unlock_node",
        proto::ClientAction::AssignWorker { .. } => "assign_worker",
        proto::ClientAction::TrainWarrior { .. } => "train_warrior",
        proto::ClientAction::DefendRaid { .. } => "defend_raid",
        proto::ClientAction::BuildRoad { .. } => "build_road",
        proto::ClientAction::SetTestAcceleration { .. } => "set_test_acceleration",
        proto::ClientAction::AdvanceTime { .. } => "advance_time",
        proto::ClientAction::SetTestRngSeed { .. } => "set_test_rng_seed",
        proto::ClientAction::FoundVillage { .. } => "found_village",
        proto::ClientAction::JoinVillage { .. } => "join_village",
        proto::ClientAction::AssignOfficer { .. } => "assign_officer",
        proto::ClientAction::UnassignOfficer { .. } => "unassign_officer",
        proto::ClientAction::DesignateStockpile { .. } => "designate_stockpile",
        proto::ClientAction::RemoveStockpile { .. } => "remove_stockpile",
        proto::ClientAction::DesignateGatherSpot { .. } => "designate_gather_spot",
        proto::ClientAction::RemoveGatherSpot { .. } => "remove_gather_spot",
        proto::ClientAction::SellGoods { .. } => "sell_goods",
        proto::ClientAction::BuyResource { .. } => "buy_resource",
        proto::ClientAction::BoostCat { .. } => "boost_cat",
    }
}

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "campaign-session".to_owned(),
        player_id: "campaign-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn signed_job(kind: proto::JobKind) -> proto::ClientAction {
    proto::ClientAction::RequestJob {
        session_id: "campaign-session".to_owned(),
        nickname: "Playtester".to_owned(),
        sig: "ignored-by-pure-sim".to_owned(),
        kind,
    }
}

fn apply_ok(
    world: &mut WorldState,
    coverage: &mut BTreeSet<&'static str>,
    action: proto::ClientAction,
    action_ctx: &ActionCtx,
) {
    coverage.insert(action_name(&action));
    let result = apply_action(world, &action, action_ctx);
    assert!(result.ok, "{action:?} failed: {:?}", result.message);
}

fn reset_workers(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.jobs.clear();
    for cat in &mut colony.cats {
        cat.current_task = None;
        cat.activity = CatActivity::Idle;
        cat.destination = None;
        cat.carrying = None;
    }
    for building in &mut colony.buildings {
        building.assigned_cat = None;
    }
}

fn complete_building(id: impl Into<String>, building_type: BuildingType) -> BuildingRuntime {
    BuildingRuntime {
        id: id.into(),
        building_type,
        level: 1,
        position: TilePos { x: 20, y: 20 },
        is_complete: true,
        construction_progress: 100,
        production_progress: 0.0,
        assigned_cat: None,
    }
}

fn signed_fields() -> (String, String, String) {
    (
        "campaign-session".to_owned(),
        "Playtester".to_owned(),
        "ignored-by-pure-sim".to_owned(),
    )
}

fn open_stockpile_rect(world: &WorldState, require_claimed: bool, edge: i32) -> ZoneRect {
    let colony = &world.colonies[0];
    let mut anchors: Vec<TilePos> = if require_claimed {
        colony.claimed_tiles.clone()
    } else {
        colony.world_tiles.keys().copied().collect()
    };
    anchors.sort_by_key(|tile| (tile.y, tile.x));
    anchors
        .into_iter()
        .map(|anchor| ZoneRect {
            x1: anchor.x,
            y1: anchor.y,
            x2: anchor.x + edge - 1,
            y2: anchor.y + edge - 1,
        })
        .find(|rect| {
            stockpile_placement_error(colony, *rect, world.world_seed, require_claimed).is_none()
        })
        .expect("campaign map has a valid stockpile footprint")
}

fn run_action_campaign() -> WorldState {
    let mut coverage = BTreeSet::new();
    let mut world = new_world(0xCA7C_0100);

    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::Ensure,
        &ctx(1_000),
    );
    assert_eq!(world.colonies.len(), 1, "ensure founded the first village");

    let before_presence = world.clone();
    let (session_id, nickname, _) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::Presence {
            session_id,
            nickname,
            sig: None,
        },
        &ctx(1_000),
    );
    assert_eq!(
        world, before_presence,
        "presence is intentionally a pure ack"
    );

    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Fast,
        },
        &ctx(1_000),
    );
    assert!(world.colonies[0].test_time_scale > 1.0);
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::SetTestRngSeed { seed: Some(4242) },
        &ctx(1_000),
    );
    assert_eq!(world.colonies[0].test_rng_seed, Some(4242));
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::AdvanceTime { seconds: 1 },
        &ctx(1_000),
    );
    assert_eq!(world.colonies[0].last_tick, 2_000);
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Off,
        },
        &ctx(2_000),
    );
    assert_eq!(world.colonies[0].test_time_scale, 1.0);

    let before_join = world.clone();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::JoinVillage {
            colony_id: "colony-1".to_owned(),
            session_id: "campaign-session".to_owned(),
        },
        &ctx(2_000),
    );
    assert_eq!(world, before_join, "join is intentionally a membership ack");
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::FoundVillage {
            name: "Second Grove".to_owned(),
            session_id: "campaign-session".to_owned(),
        },
        &ctx(2_000),
    );
    assert_eq!(world.colonies.len(), 2);
    assert_eq!(world.colonies[1].name, "Second Grove");
    assert_ne!(world.colonies[0].anchor, world.colonies[1].anchor);

    // Every accepted manual job kind, including the ritual request's non-job state.
    for kind in [
        proto::JobKind::SupplyFood,
        proto::JobKind::SupplyWater,
        proto::JobKind::LeaderPlanHunt,
        proto::JobKind::LeaderPlanHouse,
        proto::JobKind::Ritual,
    ] {
        let jobs_before = world.colonies[0].jobs.len();
        apply_ok(&mut world, &mut coverage, signed_job(kind), &ctx(3_000));
        if kind == proto::JobKind::Ritual {
            assert_eq!(world.colonies[0].ritual_requested_at, Some(3_000));
        } else {
            assert_eq!(world.colonies[0].jobs.len(), jobs_before + 1);
        }

        if kind == proto::JobKind::SupplyFood {
            let job_id = world.colonies[0].jobs.last().unwrap().id.clone();
            let old_end = world.colonies[0].jobs.last().unwrap().ends_at.unwrap();
            let (session_id, nickname, sig) = signed_fields();
            apply_ok(
                &mut world,
                &mut coverage,
                proto::ClientAction::Boost {
                    session_id,
                    nickname,
                    sig,
                    job_id,
                },
                &ctx(3_100),
            );
            let boosted = world.colonies[0].jobs.last().unwrap();
            assert_eq!(boosted.click_count, 1);
            assert!(boosted.ends_at.unwrap() < old_end);
        }
        reset_workers(&mut world);
    }

    // Every idle upgrade key is buyable through the same player action.
    world.colonies[0].global_upgrade_points = 10_000.0;
    for key in [
        proto::UpgradeKey::ClickPower,
        proto::UpgradeKey::SupplySpeed,
        proto::UpgradeKey::HuntMastery,
        proto::UpgradeKey::BuildMastery,
        proto::UpgradeKey::RitualMastery,
        proto::UpgradeKey::Resilience,
    ] {
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::PurchaseUpgrade {
                session_id,
                nickname,
                sig,
                key,
            },
            &ctx(4_000),
        );
    }
    let levels = &world.colonies[0].upgrade_levels;
    assert_eq!(
        [
            levels.click_power,
            levels.supply_speed,
            levels.hunt_mastery,
            levels.build_mastery,
            levels.ritual_mastery,
            levels.resilience,
        ],
        [1; 6]
    );

    // The root node has no prerequisites, so it exercises the god-purchase path cleanly.
    world.colonies[0].upgrade_tree = upgrade_tree::create_upgrade_tree_state();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::UnlockNode {
            session_id,
            nickname,
            sig,
            node_id: "research_hut".to_owned(),
        },
        &ctx(4_100),
    );
    assert!(upgrade_tree::is_owned(
        &world.colonies[0].upgrade_tree,
        "research_hut"
    ));

    // The one-second tick opened a scheduled election. Vote in it, then start a kick.
    let election_id = world.colonies[0]
        .elections
        .iter()
        .find(|election| election.kind == ElectionKind::Scheduled && election.resolved_at.is_none())
        .expect("scheduled election opened")
        .id
        .clone();
    let cat_id = world.colonies[0].cats[0].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::CastVote {
            session_id,
            nickname,
            sig,
            election_id: election_id.clone(),
            cat_id: cat_id.clone(),
        },
        &ctx(4_200),
    );
    assert!(
        world.colonies[0]
            .votes
            .iter()
            .any(|vote| { vote.election_id == election_id && vote.cat_id == cat_id })
    );
    assert!(world.colonies[0].leader_id.is_some());
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::RequestVoteKick {
            session_id,
            nickname,
            sig,
        },
        &ctx(4_300),
    );
    assert!(world.colonies[0].elections.iter().any(|election| {
        election.kind == ElectionKind::VoteKick && election.resolved_at.is_none()
    }));

    // Both paint-zone modes and their removal path.
    for kind in [proto::ZoneKind::Avoid, proto::ZoneKind::Gather] {
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::CreateZone {
                session_id,
                nickname,
                sig,
                kind,
                a: proto::TilePoint { x: -2, y: -2 },
                b: proto::TilePoint { x: 1, y: 1 },
                duration_ms: 600_000,
            },
            &ctx(5_000),
        );
        assert_eq!(world.colonies[0].zones.len(), 1);
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::RemoveZone {
                session_id,
                nickname,
                sig,
                zone_id: "zone-0".to_owned(),
            },
            &ctx(5_100),
        );
        assert!(world.colonies[0].zones.is_empty());
    }

    // Raise the synthetic village to level 4 and grant prerequisites so every building
    // type currently accepted by PlanBuilding is covered without waiting for progression.
    for index in 0..14 {
        world.colonies[0].buildings.push(complete_building(
            format!("campaign-den-{index}"),
            BuildingType::Den,
        ));
    }
    world.colonies[0].upgrade_tree.owned_node_ids = UPGRADE_NODES
        .iter()
        .map(|node| node.id.to_owned())
        .collect();
    for building_type in [
        proto::BuildingType::Workshop,
        proto::BuildingType::Field,
        proto::BuildingType::Smithy,
        proto::BuildingType::Barracks,
        proto::BuildingType::FoodStorage,
        proto::BuildingType::Den,
        proto::BuildingType::Smelter,
        proto::BuildingType::School,
    ] {
        let jobs_before = world.colonies[0].jobs.len();
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::PlanBuilding {
                session_id,
                nickname,
                sig,
                building_type,
            },
            &ctx(6_000),
        );
        assert_eq!(world.colonies[0].jobs.len(), jobs_before + 1);
        assert_eq!(
            world.colonies[0].jobs.last().unwrap().kind,
            JobKind::BuildHouse
        );
        reset_workers(&mut world);
    }

    let workshop_id = "campaign-workshop".to_owned();
    world.colonies[0].buildings.push(complete_building(
        workshop_id.clone(),
        BuildingType::Workshop,
    ));
    let worker_id = world.colonies[0].cats[0].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: worker_id.clone(),
            building_id: Some(workshop_id.clone()),
        },
        &ctx(6_100),
    );
    assert_eq!(
        world.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == workshop_id)
            .unwrap()
            .assigned_cat
            .as_deref(),
        Some(worker_id.as_str())
    );
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: worker_id.clone(),
            building_id: None,
        },
        &ctx(6_200),
    );
    assert!(
        world.colonies[0]
            .buildings
            .iter()
            .all(|building| building.assigned_cat.as_deref() != Some(worker_id.as_str()))
    );

    // Every officer role is independently appointable and vacatable.
    for role in [
        proto::OfficerRole::Steward,
        proto::OfficerRole::Forester,
        proto::OfficerRole::Farmer,
        proto::OfficerRole::Captain,
        proto::OfficerRole::Loremaster,
    ] {
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::AssignOfficer {
                session_id,
                nickname,
                sig,
                role,
                cat_id: worker_id.clone(),
            },
            &ctx(6_300),
        );
        assert_eq!(world.colonies[0].officers.len(), 1);
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::UnassignOfficer {
                session_id,
                nickname,
                sig,
                role,
            },
            &ctx(6_400),
        );
        assert!(world.colonies[0].officers.is_empty());
    }

    let boosted_id = world.colonies[0].cats[1].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::BoostCat {
            session_id,
            nickname,
            sig,
            cat_id: boosted_id.clone(),
            boosted: true,
        },
        &ctx(6_500),
    );
    assert!(
        world.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == boosted_id)
            .unwrap()
            .boosted
    );

    // Designated piles and P16 gather spots have separate create/remove lifecycles.
    let stockpiles_before = world.colonies[0].stockpiles.len();
    let stockpile_rect = open_stockpile_rect(&world, true, 2);
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DesignateStockpile {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint {
                x: stockpile_rect.x1,
                y: stockpile_rect.y1,
            },
            b: proto::TilePoint {
                x: stockpile_rect.x2,
                y: stockpile_rect.y2,
            },
            accepts: vec![proto::ResourceKind::Food, proto::ResourceKind::Materials],
        },
        &ctx(7_000),
    );
    assert_eq!(world.colonies[0].stockpiles.len(), stockpiles_before + 1);
    let stockpile_id = world.colonies[0].stockpiles.last().unwrap().id.clone();
    let gather_rect = open_stockpile_rect(&world, false, 2);
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::RemoveStockpile {
            session_id,
            nickname,
            sig,
            stockpile_id: stockpile_id.clone(),
        },
        &ctx(7_100),
    );
    assert!(
        world.colonies[0]
            .stockpiles
            .iter()
            .all(|pile| pile.id != stockpile_id)
    );

    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DesignateGatherSpot {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint {
                x: gather_rect.x1,
                y: gather_rect.y1,
            },
            b: proto::TilePoint {
                x: gather_rect.x2,
                y: gather_rect.y2,
            },
            kind: proto::ResourceKind::Materials,
        },
        &ctx(7_200),
    );
    let gather_id = world.colonies[0].gather_spots[0].stockpile_id.clone();
    assert!(
        world.colonies[0]
            .stockpiles
            .iter()
            .any(|pile| pile.id == gather_id)
    );
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::RemoveGatherSpot {
            session_id,
            nickname,
            sig,
            stockpile_id: gather_id.clone(),
        },
        &ctx(7_300),
    );
    assert!(world.colonies[0].gather_spots.is_empty());
    assert!(
        world.colonies[0]
            .stockpiles
            .iter()
            .all(|pile| pile.id != gather_id)
    );

    // Lay one observable road tile on a known non-water, unpaved world tile.
    let road_pos = world.colonies[0]
        .world_tiles
        .iter()
        .find(|(pos, tile)| {
            tile.overlay_feature.as_deref() != Some("road_built")
                && tile.overlay_feature.as_deref() != Some("river")
                && road_placement_error(&world.colonies[0], **pos, world.world_seed).is_none()
                && road_path_attaches_to_shrine(&world.colonies[0], &[**pos])
        })
        .map(|(pos, _)| *pos)
        .expect("found an unpaved solid tile");
    world.colonies[0].resources.materials = 1_000.0;
    let materials_before = world.colonies[0].resources.materials;
    let endpoint = proto::TilePoint {
        x: road_pos.x,
        y: road_pos.y,
    };
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::BuildRoad {
            session_id,
            nickname,
            sig,
            a: endpoint,
            b: endpoint,
        },
        &ctx(8_000),
    );
    assert_eq!(
        world.colonies[0].world_tiles[&road_pos]
            .overlay_feature
            .as_deref(),
        Some("road_built")
    );
    assert_eq!(
        world.colonies[0].resources.materials,
        materials_before - 1.0
    );

    // Barracks action and direct raid-defense click.
    reset_workers(&mut world);
    world.colonies[0].buildings.push(complete_building(
        "campaign-barracks",
        BuildingType::Barracks,
    ));
    let recruit_id = world.colonies[0].cats[2].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::TrainWarrior {
            session_id,
            nickname,
            sig,
            cat_id: Some(recruit_id.clone()),
        },
        &ctx(8_100),
    );
    assert!(world.colonies[0].jobs.iter().any(|job| {
        job.kind == JobKind::TrainWarrior && job.assigned_cat.as_deref() == Some(&recruit_id)
    }));
    reset_workers(&mut world);

    world.colonies[0].active_raid = Some("campaign-raid".to_owned());
    world.colonies[0].raiders.push(RaiderRuntime {
        id: "campaign-raider".to_owned(),
        raid_id: "campaign-raid".to_owned(),
        position: Position {
            map: MapType::World,
            x: 0.0,
            y: 0.0,
        },
        destination: None,
        attack: 1.0,
        defense: 1.0,
        health: 12.0,
    });
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DefendRaid {
            session_id,
            nickname,
            sig,
        },
        &ctx(8_200),
    );
    assert_eq!(world.colonies[0].raiders.last().unwrap().health, 6.0);
    assert_eq!(world.colonies[0].raid_clicks, 1.0);

    // A scripted trading window exercises both directions and their concrete stores.
    let trade_item = Item::new(ItemKind::Mug, Material::Wood, 1);
    world.colonies[0].add_item(trade_item, 2);
    world.colonies[0].trader = Some(TraderRuntime {
        id: "campaign-trader".to_owned(),
        position: Position {
            map: MapType::World,
            x: 0.0,
            y: 0.0,
        },
        destination: None,
        state: TraderState::Trading,
        arrived_at: Some(8_300),
    });
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::SellGoods {
            session_id,
            nickname,
            sig,
            kind: "mug".to_owned(),
            material: "wood".to_owned(),
            quality: 1,
            count: 1,
        },
        &ctx(8_300),
    );
    assert_eq!(world.colonies[0].items.get(&trade_item), Some(&1));
    assert_eq!(
        world.colonies[0].coin,
        trader::trader_buy_price(trade_item, 1)
    );
    world.colonies[0].coin = 100.0;
    let food_before = world.colonies[0].resources.food;
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::BuyResource {
            session_id,
            nickname,
            sig,
            resource: proto::ResourceKind::Food,
            amount: 2.0,
        },
        &ctx(8_400),
    );
    assert_eq!(world.colonies[0].resources.food, food_before + 2.0);
    assert_eq!(world.colonies[0].coin, 97.0);

    let expected: BTreeSet<&str> = EXPECTED_ACTIONS.into_iter().collect();
    assert_eq!(coverage, expected, "the campaign missed an action variant");
    assert_eq!(coverage.len(), 29);
    world
}

#[test]
fn every_player_action_mutates_its_feature_and_the_campaign_is_deterministic() {
    let first = run_action_campaign();
    let repeated = run_action_campaign();
    assert_eq!(first, repeated, "scripted action campaign diverged");

    let colony_ids: HashSet<&str> = first
        .colonies
        .iter()
        .map(|colony| colony.id.as_str())
        .collect();
    assert_eq!(colony_ids.len(), 2, "village ids are distinct");
    assert_eq!(first.colonies[0].items.values().sum::<u32>(), 1);
    assert_eq!(first.colonies[0].coin, 97.0);
}
