//! Deterministic player-action campaign at the simulation's public action boundary.
//!
//! This deliberately supplies prerequisites (buildings, an election, a raid, a trader)
//! instead of waiting game-days for them. It is a coverage campaign, not a balance test:
//! every `ClientAction` variant reaches `apply_action`, every accepted mutation is
//! asserted, and the complete campaign repeats bit-for-bit.

use std::collections::{BTreeSet, HashSet};

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    entities::{CatActivity, MapType, Position},
    items::{Item, ItemKind, ItemStore, Material},
    officers::OfficerRole,
    terrain_gen::tile_climate_biome,
    trader::{self, TraderState},
    types::{BuildingType, JobKind, TileType},
    upgrade_tree::{self, UPGRADE_NODES},
    world_tick::{
        BuildingRuntime, ElectionKind, EventKind, RaidPhase, RaiderRuntime, TilePos, TraderRuntime,
        WorldState, building_is_road_connected_to_shrine, can_plan_building_at,
        default_production_queue, found_colony, new_world, publish_colony_spatial,
        road_path_attaches_to_shrine, road_placement_error, stockpile_placement_error,
        tile_is_occupied, world_tick,
    },
    zones::ZoneRect,
};

const EXPECTED_ACTIONS: [&str; 41] = [
    "advance_time",
    "assign_officer",
    "assign_worker",
    "boost",
    "boost_cat",
    "build_road",
    "buy_resource",
    "cast_vote",
    "clear_farm",
    "create_zone",
    "defend_raid",
    "designate_farm",
    "designate_gather_spot",
    "designate_fishing_spot",
    "designate_stockpile",
    "dispatch_scout",
    "edit_production_queue",
    "edit_production_work_slot",
    "ensure",
    "found_village",
    "haul_gather_spot",
    "join_village",
    "plan_building",
    "presence",
    "purchase_upgrade",
    "remove_gather_spot",
    "remove_stockpile",
    "remove_zone",
    "research_node",
    "request_job",
    "request_vote_kick",
    "repair_item",
    "sell_goods",
    "set_cat_labor_preference",
    "set_test_acceleration",
    "set_test_rng_seed",
    "train_warrior",
    "unassign_officer",
    "unlock_node",
    "offer_materials",
    "offer_tithe",
];

fn action_name(action: &proto::ClientAction) -> &'static str {
    match action {
        proto::ClientAction::Ensure => "ensure",
        proto::ClientAction::Presence { .. } => "presence",
        proto::ClientAction::RequestJob { .. } => "request_job",
        proto::ClientAction::DispatchScout { .. } => "dispatch_scout",
        proto::ClientAction::Boost { .. } => "boost",
        proto::ClientAction::PurchaseUpgrade { .. } => "purchase_upgrade",
        proto::ClientAction::CastVote { .. } => "cast_vote",
        proto::ClientAction::RequestVoteKick { .. } => "request_vote_kick",
        proto::ClientAction::CreateZone { .. } => "create_zone",
        proto::ClientAction::RemoveZone { .. } => "remove_zone",
        proto::ClientAction::PlanBuilding { .. } => "plan_building",
        proto::ClientAction::UnlockNode { .. } => "unlock_node",
        proto::ClientAction::ResearchNode { .. } => "research_node",
        proto::ClientAction::OfferTithe { .. } => "offer_tithe",
        proto::ClientAction::OfferMaterials { .. } => "offer_materials",
        proto::ClientAction::HaulGatherSpot { .. } => "haul_gather_spot",
        proto::ClientAction::AssignWorker { .. } => "assign_worker",
        proto::ClientAction::TrainWarrior { .. } => "train_warrior",
        proto::ClientAction::DefendRaid { .. } => "defend_raid",
        proto::ClientAction::BuildRoad { .. } => "build_road",
        proto::ClientAction::SetTestAcceleration { .. } => "set_test_acceleration",
        proto::ClientAction::AdvanceTime { .. } => "advance_time",
        proto::ClientAction::SetTestRngSeed { .. } => "set_test_rng_seed",
        proto::ClientAction::FoundVillage { .. } => "found_village",
        proto::ClientAction::JoinVillage { .. } => "join_village",
        proto::ClientAction::OfferVillageTrade { .. } => "offer_village_trade",
        proto::ClientAction::AcceptVillageTrade { .. } => "accept_village_trade",
        proto::ClientAction::CancelVillageTrade { .. } => "cancel_village_trade",
        proto::ClientAction::AssignOfficer { .. } => "assign_officer",
        proto::ClientAction::UnassignOfficer { .. } => "unassign_officer",
        proto::ClientAction::DesignateFarm { .. } => "designate_farm",
        proto::ClientAction::ClearFarm { .. } => "clear_farm",
        proto::ClientAction::DesignateStockpile { .. } => "designate_stockpile",
        proto::ClientAction::RemoveStockpile { .. } => "remove_stockpile",
        proto::ClientAction::DesignateGatherSpot { .. } => "designate_gather_spot",
        proto::ClientAction::DesignateFishingSpot { .. } => "designate_fishing_spot",
        proto::ClientAction::RemoveGatherSpot { .. } => "remove_gather_spot",
        proto::ClientAction::SellGoods { .. } => "sell_goods",
        proto::ClientAction::BuyResource { .. } => "buy_resource",
        proto::ClientAction::BoostCat { .. } => "boost_cat",
        proto::ClientAction::SetCatLaborPreference { .. } => "set_cat_labor_preference",
        proto::ClientAction::EditProductionQueue { .. } => "edit_production_queue",
        proto::ClientAction::EditProductionWorkSlot { .. } => "edit_production_work_slot",
        proto::ClientAction::RepairItem { .. } => "repair_item",
        proto::ClientAction::EquipItem { .. } => "equip_item",
        proto::ClientAction::UnequipItem { .. } => "unequip_item",
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
        building.automated_by = None;
        for slot in &mut building.additional_work_slots {
            slot.assigned_cat.clear();
            slot.automated_by = None;
        }
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
        automated_by: None,
        additional_work_slots: Vec::new(),
        production_queue: cat_sim::world_tick::default_production_queue(building_type),
        production_paused: false,
        construction_cargo: None,
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
            sig: None,
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
            sig: None,
        },
        &ctx(2_000),
    );
    assert_eq!(world.colonies.len(), 2);
    assert_eq!(world.colonies[1].name, "Second Grove");
    assert_ne!(world.colonies[0].anchor, world.colonies[1].anchor);

    // Logging is a researched outside-village job. Reveal the loaded campaign map
    // and grant its real research prerequisite so the accepted-path sweep reaches it.
    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("sawmill".to_owned());
    let loaded_tiles = world.colonies[0]
        .world_tiles
        .keys()
        .copied()
        .collect::<Vec<_>>();
    world.colonies[0].revealed_tiles.extend(loaded_tiles);
    let frontier = world.colonies[0]
        .world_tiles
        .values()
        .find(|tile| {
            !world.colonies[0].claimed_tiles.contains(&tile.pos)
                && matches!(tile.tile_type, TileType::Field | TileType::Meadow)
        })
        .expect("campaign map includes an ordinary frontier tile")
        .pos;
    world.colonies[0].revealed_tiles.remove(&frontier);

    // Every accepted manual job kind, including the ritual request's non-job state.
    for kind in [
        proto::JobKind::SupplyFood,
        proto::JobKind::SupplyWater,
        proto::JobKind::LeaderPlanHunt,
        proto::JobKind::HuntExpedition,
        proto::JobKind::LeaderPlanHouse,
        proto::JobKind::Ritual,
        proto::JobKind::Quarry,
        proto::JobKind::GatherLogs,
        proto::JobKind::ForageFibre,
        proto::JobKind::Explore,
        proto::JobKind::FetchWater,
        proto::JobKind::ExpandVillage,
        proto::JobKind::CarryOffering,
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
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DispatchScout {
            session_id,
            nickname,
            sig,
            mission: proto::ScoutMission::Explore,
        },
        &ctx(3_200),
    );
    assert_eq!(
        world.colonies[0].jobs.last().unwrap().kind,
        JobKind::Explore
    );
    reset_workers(&mut world);

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

    // The root node exercises the god-purchase path; its child separately exercises
    // spending scholar-earned research points without consuming blessings.
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
    world.colonies[0].upgrade_tree.research_points = 5.0;
    world.colonies[0].last_leader_research_choice_at = Some(4_000);
    let blessings_before_research = world.colonies[0].global_upgrade_points;
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::ResearchNode {
            session_id,
            nickname,
            sig,
            node_id: "basic_tools".to_owned(),
        },
        &ctx(4_150),
    );
    assert!(upgrade_tree::is_owned(
        &world.colonies[0].upgrade_tree,
        "basic_tools"
    ));
    assert_eq!(
        world.colonies[0].global_upgrade_points,
        blessings_before_research
    );
    assert_eq!(
        world.colonies[0].last_leader_research_choice_at,
        Some(4_000),
        "manual ResearchNode bypasses the Leader's autonomous daily clock"
    );

    world.colonies[0].resources.food = 200.0;
    world.colonies[0].resources.refined = 10.0;
    let blessings_before_tithe = world.colonies[0].global_upgrade_points;
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::OfferTithe {
            session_id,
            nickname,
            sig,
        },
        &ctx(4_175),
    );
    assert!(world.colonies[0].global_upgrade_points > blessings_before_tithe);

    reset_workers(&mut world);
    world.colonies[0].resources.materials = 100.0;
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::OfferMaterials {
            session_id,
            nickname,
            sig,
        },
        &ctx(4_190),
    );
    assert!(
        world.colonies[0]
            .jobs
            .iter()
            .any(|job| job.kind == JobKind::CarryOffering)
    );
    reset_workers(&mut world);

    // The one-second tick opened a scheduled election. Vote in it, then start a kick.
    let election_id = world.colonies[0]
        .elections
        .iter()
        .find(|election| election.kind == ElectionKind::Scheduled && election.resolved_at.is_none())
        .expect("scheduled election opened")
        .id
        .clone();
    let cat_id = build_snapshot(&world, 4_200, 1).colonies[0]
        .election
        .as_ref()
        .expect("scheduled election is exposed")
        .candidates[0]
        .id
        .clone();
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
    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("accounting_tent_foundations".to_owned());
    for building_type in [
        proto::BuildingType::Den,
        proto::BuildingType::FoodStorage,
        proto::BuildingType::WaterBowl,
        proto::BuildingType::Beds,
        proto::BuildingType::HerbGarden,
        proto::BuildingType::Nursery,
        proto::BuildingType::ElderCorner,
        proto::BuildingType::Walls,
        proto::BuildingType::MouseFarm,
        proto::BuildingType::Workshop,
        proto::BuildingType::Field,
        proto::BuildingType::Mill,
        proto::BuildingType::Sawmill,
        proto::BuildingType::ResearchHut,
        proto::BuildingType::School,
        proto::BuildingType::Smithy,
        proto::BuildingType::Barracks,
        proto::BuildingType::AccountingTent,
        proto::BuildingType::WoodCutter,
        proto::BuildingType::StonePrep,
        proto::BuildingType::Woodworking,
        proto::BuildingType::Clothier,
        proto::BuildingType::Tannery,
        proto::BuildingType::Smelter,
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
                site: None,
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
    for (id, building_type) in [
        ("campaign-steward-workshop", BuildingType::Workshop),
        ("campaign-accounting", BuildingType::AccountingTent),
        ("campaign-sawmill", BuildingType::Sawmill),
        ("campaign-field", BuildingType::Field),
        ("campaign-officer-barracks", BuildingType::Barracks),
        ("campaign-research-hut", BuildingType::ResearchHut),
        ("campaign-clothier", BuildingType::Clothier),
    ] {
        world.colonies[0]
            .buildings
            .push(complete_building(id, building_type));
    }

    // Player guidance can persist an exact labor preference and edit the physical
    // Sawmill queue without waiting for an officer automation tick.
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::SetCatLaborPreference {
            session_id,
            nickname,
            sig,
            cat_id: worker_id.clone(),
            labor: proto::Labor::Woodcut,
            enabled: true,
        },
        &ctx(6_250),
    );
    assert!(
        world.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == worker_id)
            .expect("campaign worker")
            .preferred_labors
            .contains(&cat_sim::skills::Labor::Woodcut)
    );

    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::EditProductionQueue {
            session_id,
            nickname,
            sig,
            building_id: "campaign-sawmill".to_owned(),
            edit: proto::ProductionQueueEdit::SetPaused { paused: true },
        },
        &ctx(6_275),
    );
    assert!(
        world.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "campaign-sawmill")
            .expect("campaign sawmill")
            .production_paused
    );

    // Crews research exposes a second independently controlled station through the
    // same signed player boundary used by the live client.
    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("sawmill_crews".to_owned());
    let second_worker_id = world.colonies[0].cats[1].id.clone();
    for cat_id in [&worker_id, &second_worker_id] {
        let (session_id, nickname, sig) = signed_fields();
        apply_ok(
            &mut world,
            &mut coverage,
            proto::ClientAction::AssignWorker {
                session_id,
                nickname,
                sig,
                cat_id: cat_id.clone(),
                building_id: Some("campaign-sawmill".to_owned()),
            },
            &ctx(6_280),
        );
    }
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::EditProductionWorkSlot {
            session_id,
            nickname,
            sig,
            building_id: "campaign-sawmill".to_owned(),
            cat_id: second_worker_id.clone(),
            edit: proto::ProductionQueueEdit::SetPaused { paused: true },
        },
        &ctx(6_285),
    );
    let sawmill = world.colonies[0]
        .buildings
        .iter()
        .find(|building| building.id == "campaign-sawmill")
        .expect("campaign sawmill");
    assert_eq!(sawmill.worker_count(), 2);
    assert!(sawmill.additional_work_slots[0].production_paused);
    reset_workers(&mut world);

    for role in [
        proto::OfficerRole::Steward,
        proto::OfficerRole::Accountant,
        proto::OfficerRole::Forester,
        proto::OfficerRole::Farmer,
        proto::OfficerRole::Captain,
        proto::OfficerRole::Loremaster,
        proto::OfficerRole::ClothLeader,
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

    // Farms occupy claimed expansion outside the founding wall and have a complete
    // player-controlled designate/clear lifecycle.
    let farm_pos = world.colonies[0]
        .world_tiles
        .keys()
        .copied()
        .find(|pos| {
            let anchor = world.colonies[0].anchor;
            (pos.x - anchor.x).abs().max((pos.y - anchor.y).abs()) > 6
                && tile_climate_biome(world.world_seed, pos.x, pos.y)
                    .properties()
                    .fertility
                    > 0.0
                && !tile_is_occupied(&world.colonies[0], *pos, world.world_seed)
        })
        .expect("campaign has fertile expansion ground");
    world.colonies[0].claimed_tiles.push(farm_pos);
    world.colonies[0].revealed_tiles.insert(farm_pos);
    let anchor = world.colonies[0].anchor;
    let (min_x, max_x) = if anchor.x <= farm_pos.x {
        (anchor.x, farm_pos.x)
    } else {
        (farm_pos.x, anchor.x)
    };
    let (min_y, max_y) = if anchor.y <= farm_pos.y {
        (anchor.y, farm_pos.y)
    } else {
        (farm_pos.y, anchor.y)
    };
    let route_claim = (min_x..=max_x)
        .map(|x| TilePos { x, y: anchor.y })
        .chain((min_y..=max_y).map(|y| TilePos { x: farm_pos.x, y }));
    for tile in route_claim {
        if !world.colonies[0].claimed_tiles.contains(&tile) {
            world.colonies[0].claimed_tiles.push(tile);
        }
        world.colonies[0].revealed_tiles.insert(tile);
    }
    let farm_neighbors = [
        TilePos {
            x: farm_pos.x - 1,
            y: farm_pos.y,
        },
        TilePos {
            x: farm_pos.x + 1,
            y: farm_pos.y,
        },
        TilePos {
            x: farm_pos.x,
            y: farm_pos.y - 1,
        },
        TilePos {
            x: farm_pos.x,
            y: farm_pos.y + 1,
        },
    ];
    for neighbor in farm_neighbors {
        if world.colonies[0].world_tiles.contains_key(&neighbor) {
            if !world.colonies[0].claimed_tiles.contains(&neighbor) {
                world.colonies[0].claimed_tiles.push(neighbor);
            }
            world.colonies[0].revealed_tiles.insert(neighbor);
        }
    }
    let handoff = farm_neighbors
        .into_iter()
        .find(|neighbor| world.colonies[0].world_tiles.contains_key(neighbor))
        .expect("campaign farm has one mapped adjacent handoff");
    let handoff_tile = world.colonies[0]
        .world_tiles
        .get_mut(&handoff)
        .expect("selected mapped handoff");
    handoff_tile.tile_type = TileType::Field;
    handoff_tile.resources.water = 0;
    handoff_tile.overlay_feature = Some("stump".to_owned());
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DesignateFarm {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint {
                x: farm_pos.x,
                y: farm_pos.y,
            },
            b: proto::TilePoint {
                x: farm_pos.x,
                y: farm_pos.y,
            },
            crop: proto::CropKind::Herb,
        },
        &ctx(6_750),
    );
    let plot_id = world.colonies[0].farms[0].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::ClearFarm {
            session_id,
            nickname,
            sig,
            plot_id,
        },
        &ctx(6_800),
    );
    assert!(world.colonies[0].farms.is_empty());

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
    world.colonies[0]
        .stockpiles
        .iter_mut()
        .find(|pile| pile.id == gather_id)
        .expect("gather spot stockpile")
        .contents
        .materials = 5.0;
    reset_workers(&mut world);
    let carrier_id = world.colonies[0].cats[0].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::HaulGatherSpot {
            session_id,
            nickname,
            sig,
            stockpile_id: gather_id.clone(),
            cat_id: Some(carrier_id.clone()),
        },
        &ctx(7_250),
    );
    assert!(world.colonies[0].jobs.iter().any(|job| {
        job.kind == JobKind::HaulGatherSpot
            && job.assigned_cat.as_deref() == Some(carrier_id.as_str())
    }));
    reset_workers(&mut world);
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

    // Fishing is a typed, durable shoreline designation rather than a generic
    // rectangular gather zone. Turn one mapped neighbor into water so this broad
    // action campaign remains independent of the generated seed's river layout.
    let fishing_bank = TilePos {
        x: gather_rect.x1,
        y: gather_rect.y1,
    };
    let water = [
        TilePos {
            x: fishing_bank.x,
            y: fishing_bank.y - 1,
        },
        TilePos {
            x: fishing_bank.x + 1,
            y: fishing_bank.y,
        },
        TilePos {
            x: fishing_bank.x,
            y: fishing_bank.y + 1,
        },
        TilePos {
            x: fishing_bank.x - 1,
            y: fishing_bank.y,
        },
    ]
    .into_iter()
    .find(|tile| world.colonies[0].world_tiles.contains_key(tile))
    .expect("mapped neighbor beside fishing bank");
    world.colonies[0].revealed_tiles.insert(water);
    world.colonies[0]
        .world_tiles
        .get_mut(&water)
        .unwrap()
        .tile_type = TileType::River;
    publish_colony_spatial(&mut world.shared_spatial, &world.colonies[0]);
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::DesignateFishingSpot {
            session_id,
            nickname,
            sig,
            at: proto::TilePoint {
                x: water.x,
                y: water.y,
            },
        },
        &ctx(7_350),
    );
    assert_eq!(
        world.colonies[0].gather_spots.last().unwrap().purpose,
        cat_sim::stockpiles::GatherSpotPurpose::Fishing
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
    reset_workers(&mut world);
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
        world.colonies[0].world_tiles[&road_pos].overlay_feature, None,
        "the signed action queues labor instead of painting terrain"
    );
    assert_eq!(world.colonies[0].resources.materials, materials_before);
    let tick_start = world.colonies[0].last_tick.max(8_000);
    for second in 1..=300 {
        let _ = cat_sim::world_tick::world_tick(&mut world, tick_start + second * 1_000);
        if world.colonies[0].world_tiles[&road_pos]
            .overlay_feature
            .as_deref()
            == Some("road_built")
        {
            break;
        }
    }
    assert_eq!(
        world.colonies[0].world_tiles[&road_pos]
            .overlay_feature
            .as_deref(),
        Some("road_built"),
        "road={road_pos:?} jobs={:?} cats={:?}",
        world.colonies[0]
            .jobs
            .iter()
            .filter(|job| job.kind == JobKind::BuildRoad)
            .collect::<Vec<_>>(),
        world.colonies[0]
            .cats
            .iter()
            .map(|cat| (
                &cat.id,
                cat.death_time,
                cat.activity,
                &cat.position,
                &cat.destination,
                &cat.carrying
            ))
            .collect::<Vec<_>>()
    );
    assert!(
        world.colonies[0].resources.materials < materials_before,
        "physical reconciliation and the finished tile debit the authored reserve"
    );
    assert!(
        world.colonies[0]
            .events
            .iter()
            .any(|event| event.message.contains("A builder paved road tile"))
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
    apply_ok(
        &mut world,
        &mut coverage,
        signed_job(proto::JobKind::TrainWarrior),
        &ctx(8_150),
    );
    assert!(
        world.colonies[0]
            .jobs
            .iter()
            .any(|job| job.kind == JobKind::TrainWarrior)
    );
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
        depart_at: Some(99_999),
        route_exterior: Some([0, 12]),
        visit_destination: Some([7, 8]),
        route_blocked: false,
        visit_number: 1,
        stock: trader::stock_for_visit(world.world_seed, "colony-1", 1),
        items: ItemStore::default(),
        coin: trader::coin_for_visit(world.world_seed, "colony-1", 1),
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

    // Repair is an exact signed action against a finite item id. A real, living
    // manually assigned worker and one visible plank are both required.
    reset_workers(&mut world);
    let repair_bench_id = "campaign-repair-bench".to_owned();
    world.colonies[0].buildings.push(complete_building(
        repair_bench_id.clone(),
        BuildingType::Woodworking,
    ));
    let repairer_id = world.colonies[0].cats[0].id.clone();
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: repairer_id,
            building_id: Some(repair_bench_id),
        },
        &ctx(8_450),
    );
    let tool = Item::new(ItemKind::Tool, Material::Wood, 1);
    world.colonies[0].add_crafted_item(tool, 1);
    let tool_id = world.colonies[0]
        .items
        .instances()
        .find(|instance| instance.item == tool)
        .expect("finite tool")
        .id
        .clone();
    world.colonies[0].items.wear(ItemKind::Tool, 1);
    let planks_before_repair = world.colonies[0].resources.planks;
    world.colonies[0].resources.planks += 1.0;
    world.colonies[0]
        .stockpiles
        .iter_mut()
        .find(|pile| !pile.is_station_local())
        .expect("founding village has visible storage")
        .contents
        .planks += 1.0;
    let (session_id, nickname, sig) = signed_fields();
    apply_ok(
        &mut world,
        &mut coverage,
        proto::ClientAction::RepairItem {
            session_id,
            nickname,
            sig,
            item_id: tool_id.clone(),
        },
        &ctx(8_500),
    );
    assert!(
        world.colonies[0]
            .items
            .instance(&tool_id)
            .unwrap()
            .is_pristine()
    );
    assert_eq!(world.colonies[0].resources.planks, planks_before_repair);

    let expected: BTreeSet<&str> = EXPECTED_ACTIONS.into_iter().collect();
    assert_eq!(coverage, expected, "the campaign missed an action variant");
    assert_eq!(coverage.len(), EXPECTED_ACTIONS.len());
    world
}

#[test]
fn defense_click_deals_exactly_one_hit_across_the_following_tick() {
    let mut world = new_world(0xCA7C_D3F3);
    let ensure = apply_action(&mut world, &proto::ClientAction::Ensure, &ctx(1_000));
    assert!(ensure.ok);

    let colony = &mut world.colonies[0];
    colony.active_raid = Some("click-parity-raid".to_owned());
    colony.raiders.push(RaiderRuntime {
        id: "click-parity-raider".to_owned(),
        raid_id: "click-parity-raid".to_owned(),
        // Keep the unit far from the gate so the following one-second tick cannot
        // resolve combat and obscure the click-damage assertion.
        position: Position {
            map: MapType::World,
            x: -100.0,
            y: -100.0,
        },
        destination: None,
        attack: 1.0,
        defense: 1.0,
        health: 20.0,
    });

    let (session_id, nickname, sig) = signed_fields();
    let result = apply_action(
        &mut world,
        &proto::ClientAction::DefendRaid {
            session_id,
            nickname,
            sig,
        },
        &ctx(1_500),
    );
    assert!(result.ok, "defense click failed: {:?}", result.message);
    assert_eq!(world.colonies[0].raiders[0].health, 14.0);
    assert_eq!(world.colonies[0].raid_clicks, 1.0);

    let _ = world_tick(&mut world, 2_000);

    assert_eq!(
        world.colonies[0].raiders[0].health, 14.0,
        "raidClicks is telemetry; the raid director must not replay immediate click damage",
    );
    assert_eq!(world.colonies[0].raid_clicks, 1.0);
}

#[test]
fn killing_defense_click_finishes_once_with_no_stranded_raider() {
    let mut world = new_world(0xCA7C_D3F4);
    assert!(apply_action(&mut world, &proto::ClientAction::Ensure, &ctx(1_000)).ok);
    let colony = &mut world.colonies[0];
    colony.active_raid = Some("killing-click-raid".to_owned());
    colony.threat_pressure = 42.0;
    colony.raiders.push(RaiderRuntime {
        id: "killing-click-raider".to_owned(),
        raid_id: "killing-click-raid".to_owned(),
        position: Position {
            map: MapType::World,
            x: -100.0,
            y: -100.0,
        },
        destination: None,
        attack: 1.0,
        defense: 1.0,
        health: 6.0,
    });

    let (session_id, nickname, sig) = signed_fields();
    let result = apply_action(
        &mut world,
        &proto::ClientAction::DefendRaid {
            session_id,
            nickname,
            sig,
        },
        &ctx(1_500),
    );
    assert!(result.ok);
    let colony = &world.colonies[0];
    assert_eq!(colony.raiders[0].health, 0.0);
    assert_eq!(colony.active_raid.as_deref(), Some("killing-click-raid"));
    assert_eq!(colony.raid_clicks, 1.0);
    assert!(
        colony
            .events
            .iter()
            .all(|event| event.kind != EventKind::Raid(RaidPhase::Repelled)),
        "the action leaves terminal narration to the atomic raid phase cleanup"
    );

    let _ = world_tick(&mut world, 2_000);
    let colony = &world.colonies[0];
    assert_eq!(colony.active_raid, None);
    assert!(
        colony.raiders.is_empty(),
        "dead raid records must be removed"
    );
    assert_eq!(colony.raid_clicks, 0.0);
    assert_eq!(colony.threat_pressure, 0.0);
    assert_eq!(
        colony
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Raid(RaidPhase::Repelled))
            .count(),
        1
    );

    let _ = world_tick(&mut world, 3_000);
    assert_eq!(
        world.colonies[0]
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Raid(RaidPhase::Repelled))
            .count(),
        1,
        "the next tick must not emit a duplicate terminal event"
    );
    assert!(world.colonies[0].raiders.is_empty());
}

#[test]
fn duplicate_manual_expansion_requests_do_not_reserve_two_cats_for_one_frontier() {
    let mut world = new_world(0xCA7C_EA5E);
    assert!(apply_action(&mut world, &proto::ClientAction::Ensure, &ctx(1_000)).ok);

    let first = apply_action(
        &mut world,
        &signed_job(proto::JobKind::ExpandVillage),
        &ctx(1_100),
    );
    let duplicate = apply_action(
        &mut world,
        &signed_job(proto::JobKind::ExpandVillage),
        &ctx(1_200),
    );

    assert!(first.ok, "first expansion failed: {:?}", first.message);
    assert!(!duplicate.ok, "duplicate expansion unexpectedly queued");
    assert_eq!(
        world.colonies[0]
            .jobs
            .iter()
            .filter(|job| job.kind == JobKind::ExpandVillage)
            .count(),
        1,
    );
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
    assert_eq!(first.colonies[0].items.values().sum::<u32>(), 2);
    assert_eq!(first.colonies[0].coin, 97.0);
}

fn migration_guidance_campaign() -> (WorldState, WorldState, Vec<String>, i64) {
    const STARTED_AT: i64 = 10_000;
    const STEP_MS: i64 = 15 * 60_000;
    const MAX_ARRIVAL_HOUR: i64 = 60;

    let seed = 42;
    let mut base = new_world(seed);
    base.colonies
        .push(found_colony(seed, "colony-1", STARTED_AT, seed));
    // Migration is a prosperity mechanic, so begin from the smallest genuinely
    // automated settlement. Reuse two road-connected founding yards with identical
    // footprints rather than fabricating buildings in the packed founding claim;
    // the fourth den itself still has to travel through the signed production action.
    let holders = base.colonies[0]
        .cats
        .iter()
        .take(3)
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();
    base.colonies[0].resources.materials = 1_000.0;
    base.colonies[0].resources.lumber = 100.0;
    base.colonies[0].resources.blocks = 100.0;
    base.colonies[0].resources.food = 300.0;
    base.colonies[0].resources.water = 300.0;
    for (from, building_type, node_id) in [
        (
            BuildingType::Woodworking,
            BuildingType::Workshop,
            "basic_tools",
        ),
        (BuildingType::WoodCutter, BuildingType::Sawmill, "sawmill"),
        (BuildingType::StonePrep, BuildingType::Barracks, "barracks"),
    ] {
        if !base.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|owned| owned == node_id)
        {
            base.colonies[0]
                .upgrade_tree
                .owned_node_ids
                .push(node_id.to_owned());
        }
        let office = base.colonies[0]
            .buildings
            .iter_mut()
            .find(|building| building.building_type == from)
            .expect("founding blueprint contains the compatible office yard");
        office.building_type = building_type;
        office.production_queue = default_production_queue(building_type);
    }
    for office in base.colonies[0].buildings.iter().filter(|building| {
        matches!(
            building.building_type,
            BuildingType::Workshop | BuildingType::Sawmill | BuildingType::Barracks
        )
    }) {
        assert!(building_is_road_connected_to_shrine(
            &base.colonies[0],
            office,
            seed,
        ));
    }
    for (role, holder) in [
        OfficerRole::Steward,
        OfficerRole::Forester,
        OfficerRole::Captain,
    ]
    .into_iter()
    .zip(holders)
    {
        base.colonies[0].officers.insert(role, holder);
    }
    let mut now = STARTED_AT;
    while now < STARTED_AT + MAX_ARRIVAL_HOUR * 3_600_000
        && base.colonies[0]
            .migration_state
            .probationary_migrants
            .is_empty()
    {
        now += STEP_MS;
        let report = world_tick(&mut base, now);
        assert_eq!(
            report[0].reset_reason,
            None,
            "pre-arrival collapse at game hour {}: pop={} food={:.2} water={:.2} jobs={:?} events={:?}",
            (now - STARTED_AT) / 3_600_000,
            base.colonies[0]
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .count(),
            base.colonies[0].resources.food,
            base.colonies[0].resources.water,
            base.colonies[0]
                .jobs
                .iter()
                .filter(|job| matches!(
                    job.status,
                    cat_sim::types::JobStatus::Active | cat_sim::types::JobStatus::Queued
                ))
                .map(|job| (job.kind, job.requested_by))
                .collect::<Vec<_>>(),
            base.colonies[0]
                .events
                .iter()
                .rev()
                .take(8)
                .collect::<Vec<_>>(),
        );
    }
    let first_cohort = base.colonies[0]
        .migration_state
        .probationary_migrants
        .iter()
        .map(|migrant| migrant.id.clone())
        .collect::<Vec<_>>();
    assert!(
        !first_cohort.is_empty(),
        "organic prosperity produced no visitors by game hour {MAX_ARRIVAL_HOUR}: pop={} food={:.2} water={:.2} construction=({:.2},{:.2},{:.2}) status={:?} critical={:?} raid={} migration={:?} migration_events={:?} events={:?}",
        base.colonies[0]
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count(),
        base.colonies[0].resources.food,
        base.colonies[0].resources.water,
        base.colonies[0].resources.materials,
        base.colonies[0].resources.blocks,
        base.colonies[0].resources.lumber,
        base.colonies[0].status,
        base.colonies[0].critical_since,
        base.colonies[0].active_raid.is_some(),
        base.colonies[0].migration_state,
        base.colonies[0]
            .events
            .iter()
            .filter(|event| matches!(
                event.kind,
                EventKind::MigrationArrived
                    | EventKind::MigrationRetained
                    | EventKind::MigrationDeparted
            ))
            .collect::<Vec<_>>(),
        base.colonies[0]
            .events
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>(),
    );
    assert_eq!(completed_beds(&base.colonies[0]), 15);

    // The Steward was setup scaffolding used only to reach a real prosperity cohort.
    // Vacate that office before the behavioral branch so the unchanged twin cannot
    // silently answer the housing problem itself. Its road-connected 3x3 yard leaves
    // an authored explicit site for the player's 2x3 den without inventing terrain.
    let den_site = base.colonies[0]
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::Workshop)
        .expect("migration setup has a Steward workshop")
        .position;
    base.colonies[0].officers.remove(&OfficerRole::Steward);
    base.colonies[0]
        .buildings
        .retain(|building| building.building_type != BuildingType::Workshop);
    let released_planners = base.colonies[0]
        .jobs
        .iter()
        .filter(|job| job.kind == JobKind::LeaderPlanHouse)
        .filter_map(|job| job.assigned_cat.clone())
        .collect::<HashSet<_>>();
    base.colonies[0]
        .jobs
        .retain(|job| job.kind != JobKind::LeaderPlanHouse);
    for cat in &mut base.colonies[0].cats {
        if released_planners.contains(&cat.id) {
            cat.current_task = None;
            cat.activity = CatActivity::Idle;
            cat.destination = None;
        }
    }
    assert!(
        can_plan_building_at(&base.colonies[0], den_site, seed, BuildingType::Den),
        "the retired setup yard must expose a legal explicit den footprint"
    );

    let mut guided = base.clone();
    let mut unguided = base;
    let mut action_accepted_at = None;
    let mut last_plan_error = None;
    for _ in 0..=48 {
        let result = apply_action(
            &mut guided,
            &proto::ClientAction::PlanBuilding {
                session_id: "guided-session".to_owned(),
                nickname: "Playtester".to_owned(),
                sig: "validated-at-server-boundary".to_owned(),
                building_type: proto::BuildingType::Den,
                site: Some(proto::TilePoint {
                    x: den_site.x,
                    y: den_site.y,
                }),
            },
            &ActionCtx {
                session_id: "guided-session".to_owned(),
                player_id: "guided-player".to_owned(),
                colony_id: "colony-1".to_owned(),
                now_ms: now,
            },
        );
        let accepted = result.ok;
        if !accepted {
            last_plan_error = result.message;
        }
        if accepted {
            action_accepted_at = Some(now);
            break;
        }
        now += STEP_MS;
        assert_eq!(world_tick(&mut guided, now)[0].reset_reason, None);
        assert_eq!(world_tick(&mut unguided, now)[0].reset_reason, None);
    }
    let accepted_at = action_accepted_at.unwrap_or_else(|| {
        panic!(
            "the explicit fourth den never found a legal claimed site: last={last_plan_error:?} claim={} jobs={:?} events={:?}",
            guided.colonies[0].claimed_tiles.len(),
            guided.colonies[0].jobs.iter().map(|job| (job.kind, job.status, job.assigned_cat.as_deref())).collect::<Vec<_>>(),
            guided.colonies[0].events.iter().rev().take(12).collect::<Vec<_>>(),
        )
    });

    let end_at = now + 45 * 3_600_000;
    while now < end_at {
        now += STEP_MS;
        assert_eq!(world_tick(&mut guided, now)[0].reset_reason, None);
        assert_eq!(world_tick(&mut unguided, now)[0].reset_reason, None);
    }
    (guided, unguided, first_cohort, accepted_at)
}

/// Player-guided counterpart to the no-input soak: the same organic migration
/// cohort is retained only when the player uses the public action boundary to
/// order a den before its 36-hour deadline. The unchanged twin demonstrates the
/// visible departure path, and repeating the whole campaign proves determinism.
#[test]
fn player_planned_den_retains_migrants_while_no_input_loses_them() {
    let (guided, unguided, cohort, accepted_at) = migration_guidance_campaign();
    let (guided_again, unguided_again, cohort_again, accepted_again) =
        migration_guidance_campaign();
    assert_eq!(guided, guided_again, "guided campaign diverged");
    assert_eq!(unguided, unguided_again, "no-input campaign diverged");
    assert_eq!(cohort, cohort_again);
    assert_eq!(accepted_at, accepted_again);

    let guided_colony = &guided.colonies[0];
    assert!(
        completed_beds(guided_colony) >= 20,
        "the requested fourth den never completed"
    );
    assert!(cohort.iter().all(|id| {
        guided_colony
            .cats
            .iter()
            .any(|cat| cat.id == *id && cat.death_time.is_none())
            && !guided_colony
                .migration_state
                .probationary_migrants
                .iter()
                .any(|migrant| migrant.id == *id)
    }));
    assert!(
        guided_colony
            .events
            .iter()
            .any(|event| event.kind == EventKind::MigrationRetained),
        "the completed den never retained the waiting cohort"
    );

    let unguided_colony = &unguided.colonies[0];
    assert!(cohort.iter().all(|id| {
        !unguided_colony.cats.iter().any(|cat| cat.id == *id)
            && !unguided_colony
                .migration_state
                .probationary_migrants
                .iter()
                .any(|migrant| migrant.id == *id)
    }));
    assert!(unguided_colony.migration_departures >= cohort.len() as u64);
}

fn completed_beds(colony: &cat_sim::world_tick::ColonyRuntime) -> usize {
    colony
        .buildings
        .iter()
        .filter(|building| building.is_complete && building.building_type == BuildingType::Den)
        .count()
        * 5
}
