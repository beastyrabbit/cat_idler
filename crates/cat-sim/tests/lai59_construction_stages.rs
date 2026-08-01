//! LAI.59 contract for physical staged construction.

use cat_sim::{
    construction_stages::{
        ConstructionBills, ConstructionCargoLine, ConstructionInvariantError,
        ConstructionMutationError, ConstructionProject, ConstructionRecoveryCause,
        ConstructionStage, ConstructionStageBill, ConstructionTargetKind, GAME_HOUR_MS,
        ScaffoldTier, building_upgrade_duration_ms, stage_work_durations,
    },
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
    types::BuildingType,
};

fn bill(content_id: &str, units: u32) -> ConstructionStageBill {
    ConstructionStageBill::new([ConstructionCargoLine::new(content_id, units)])
}

fn footprint(width: i32, height: i32) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::try_new(TilePoint { x: 10, y: 20 }, width, height).unwrap())
}

fn workshop_project() -> ConstructionProject {
    ConstructionProject::new(
        "construction_workshop_1",
        ConstructionTargetKind::Building,
        Some(BuildingType::Workshop),
        1,
        ScaffoldTier::Basic,
        footprint(3, 3),
        ConstructionBills {
            scaffold: bill("resource_logs", 4),
            structure: bill("resource_stone", 6),
            fit_out: bill("resource_cloth", 2),
        },
        100_000,
        10,
    )
    .unwrap()
}

fn deliver(project: &mut ConstructionProject, content_id: &str, units: u32, tick: i64) {
    project.begin_transit(content_id, units, tick).unwrap();
    project
        .deliver_transit(content_id, units, tick + 1)
        .unwrap();
}

#[test]
fn lai59_upgrade_duration_is_a_fixed_deterministic_table_and_splits_20_60_20() {
    let expected = [
        (2, 28_800_000),
        (3, 68_498_330),
        (4, 113_708_795),
        (5, 162_917_403),
        (6, 215_330_225),
        (7, 270_446_616),
        (8, 327_917_835),
        (9, 387_485_069),
        (10, 448_947_570),
    ];
    assert_eq!(GAME_HOUR_MS, 3_600_000);
    assert_eq!(building_upgrade_duration_ms(1), None);
    assert_eq!(building_upgrade_duration_ms(11), None);
    for (level, duration) in expected {
        assert_eq!(building_upgrade_duration_ms(level), Some(duration));
        let (scaffold, structure, fit_out) = stage_work_durations(duration);
        assert_eq!(scaffold + structure + fit_out, duration);
        assert_eq!(scaffold, duration / 5);
        assert_eq!(structure, ((u128::from(duration) * 3) / 5) as u64);
    }
}

#[test]
fn lai59_wood_scaffold_precedes_structure_and_every_stage_requires_time() {
    let mut project = workshop_project();
    assert_eq!(project.stage, ConstructionStage::SiteReserved);
    assert_eq!(project.footprint.tiles.len(), 9);
    project.reserve_site(20).unwrap();
    assert_eq!(project.stage, ConstructionStage::DeliverScaffold);
    assert_eq!(
        project.begin_stage_work(21),
        Err(ConstructionMutationError::CargoIncomplete)
    );

    deliver(&mut project, "resource_logs", 4, 30);
    project.begin_stage_work(40).unwrap();
    assert_eq!(project.stage, ConstructionStage::BuildScaffold);
    assert_eq!(project.stage_work_remaining_ms, 20_000);
    assert!(!project.advance_work(19_999, 41).unwrap().completed_stage);
    assert_eq!(project.stage, ConstructionStage::BuildScaffold);
    project.advance_work(1, 42).unwrap();
    assert_eq!(project.stage, ConstructionStage::DeliverStructure);

    deliver(&mut project, "resource_stone", 6, 50);
    project.begin_stage_work(60).unwrap();
    assert_eq!(project.stage_work_remaining_ms, 60_000);
    project.advance_work(60_000, 61).unwrap();
    assert_eq!(project.stage, ConstructionStage::DeliverFitOut);

    deliver(&mut project, "resource_cloth", 2, 70);
    project.begin_stage_work(80).unwrap();
    assert_eq!(project.stage_work_remaining_ms, 20_000);
    project.advance_work(20_000, 81).unwrap();
    assert_eq!(project.stage, ConstructionStage::Operational);
    project.validate().unwrap();
}

#[test]
fn lai59_developed_upgrades_require_lumber_or_planks_and_exact_duration() {
    let duration = building_upgrade_duration_ms(2).unwrap();
    let upgrade = ConstructionProject::new(
        "construction_workshop_upgrade_2",
        ConstructionTargetKind::BuildingUpgrade,
        Some(BuildingType::Workshop),
        2,
        ScaffoldTier::Developed,
        footprint(3, 3),
        ConstructionBills {
            scaffold: bill("resource_lumber", 4),
            structure: bill("resource_refined", 6),
            fit_out: bill("fixture_workshop", 2),
        },
        duration,
        10,
    );
    assert!(upgrade.is_ok());

    let wrong_tier = ConstructionProject::new(
        "construction_workshop_upgrade_wrong",
        ConstructionTargetKind::BuildingUpgrade,
        Some(BuildingType::Workshop),
        2,
        ScaffoldTier::Basic,
        footprint(3, 3),
        ConstructionBills {
            scaffold: bill("resource_logs", 4),
            structure: bill("resource_refined", 6),
            fit_out: bill("fixture_workshop", 2),
        },
        duration,
        10,
    );
    assert_eq!(
        wrong_tier,
        Err(ConstructionInvariantError::UpgradeRequiresDevelopedScaffold)
    );

    let wrong_duration = ConstructionProject::new(
        "construction_workshop_upgrade_fast",
        ConstructionTargetKind::BuildingUpgrade,
        Some(BuildingType::Workshop),
        2,
        ScaffoldTier::Developed,
        footprint(3, 3),
        ConstructionBills {
            scaffold: bill("resource_planks", 4),
            structure: bill("resource_refined", 6),
            fit_out: bill("fixture_workshop", 2),
        },
        duration - 1,
        10,
    );
    assert_eq!(
        wrong_duration,
        Err(ConstructionInvariantError::InvalidUpgradeDuration)
    );
}

#[test]
fn lai59_route_loss_and_death_return_explicit_physical_recovery_intents() {
    let mut project = workshop_project();
    project.reserve_site(20).unwrap();
    project.begin_transit("resource_logs", 4, 30).unwrap();
    let recovery = project
        .interrupt_transit("resource_logs", 2, ConstructionRecoveryCause::RouteLoss, 31)
        .unwrap();
    assert_eq!(recovery.content_id, "resource_logs");
    assert_eq!(recovery.units, 2);
    assert_eq!(recovery.cause, ConstructionRecoveryCause::RouteLoss);
    assert_eq!(project.bills.scaffold.lines[0].in_transit_units, 2);
    assert_eq!(project.bills.scaffold.lines[0].missing_units(), 2);

    let death = project
        .interrupt_transit(
            "resource_logs",
            2,
            ConstructionRecoveryCause::CarrierDeath,
            32,
        )
        .unwrap();
    assert_eq!(death.units, 2);
    assert_eq!(project.bills.scaffold.lines[0].in_transit_units, 0);
    assert_eq!(recovery.units + death.units, 4);
    assert_eq!(
        project.bills.scaffold.lines[0].accounted_units()
            + project.bills.scaffold.lines[0].missing_units(),
        4
    );
}

#[test]
fn lai59_cancellation_is_one_shot_and_cannot_duplicate_salvage() {
    let mut project = workshop_project();
    project.reserve_site(20).unwrap();
    project.begin_transit("resource_logs", 3, 30).unwrap();
    project.deliver_transit("resource_logs", 2, 31).unwrap();
    let salvage = project.cancel(40).unwrap();
    assert_eq!(salvage.scaffold[0].delivered_units, 2);
    assert_eq!(salvage.scaffold[0].in_transit_units, 1);
    assert_eq!(project.stage, ConstructionStage::Cancelled);
    assert_eq!(project.bills.scaffold.lines[0].delivered_units, 0);
    assert_eq!(project.bills.scaffold.lines[0].in_transit_units, 0);
    assert_eq!(
        project.cancel(41),
        Err(ConstructionMutationError::TerminalProject)
    );
}

#[test]
fn lai59_strict_restart_rejects_unknown_future_and_impossible_stage_state() {
    let project = workshop_project();
    let encoded = project.to_canonical_json();
    assert_eq!(
        ConstructionProject::decode_strict(&encoded).unwrap(),
        project
    );

    let mut unknown: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(ConstructionProject::decode_strict(&unknown.to_string()).is_err());

    let mut nested_unknown: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    nested_unknown["footprint"]["unexpected"] = serde_json::json!(true);
    assert!(
        ConstructionProject::decode_strict(&nested_unknown.to_string()).is_err(),
        "nested construction state must be strict too"
    );
    let mut tile_unknown: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    tile_unknown["footprint"]["tiles"][0]["unexpected"] = serde_json::json!(true);
    assert!(
        ConstructionProject::decode_strict(&tile_unknown.to_string()).is_err(),
        "nested tile coordinates must reject unknown fields"
    );

    let mut unknown_content: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    unknown_content["bills"]["structure"]["lines"][0]["contentId"] =
        serde_json::json!("compatibility_material");
    assert!(
        ConstructionProject::decode_strict(&unknown_content.to_string()).is_err(),
        "construction bills must use the unified manifest instead of aliases"
    );

    let mut future: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    future["version"] = serde_json::json!(2);
    assert!(ConstructionProject::decode_strict(&future.to_string()).is_err());

    let mut impossible: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    impossible["stage"] = serde_json::json!("operational");
    assert!(ConstructionProject::decode_strict(&impossible.to_string()).is_err());

    let mut overflowed_cargo: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    overflowed_cargo["bills"]["scaffold"]["lines"][0]["requiredUnits"] =
        serde_json::json!(u32::MAX);
    overflowed_cargo["bills"]["scaffold"]["lines"][0]["deliveredUnits"] =
        serde_json::json!(u32::MAX);
    overflowed_cargo["bills"]["scaffold"]["lines"][0]["inTransitUnits"] =
        serde_json::json!(u32::MAX);
    assert!(
        ConstructionProject::decode_strict(&overflowed_cargo.to_string()).is_err(),
        "saturating u32 arithmetic must not make overfilled cargo look valid"
    );
}

#[test]
fn lai59_hole_upgrade_work_uses_the_complete_central_three_by_three() {
    let hole = ConstructionProject::new(
        "construction_hole_width_1",
        ConstructionTargetKind::HoleUpgrade,
        None,
        1,
        ScaffoldTier::Basic,
        footprint(3, 3),
        ConstructionBills {
            scaffold: bill("resource_logs", 2),
            structure: bill("resource_refined", 5),
            fit_out: bill("item_generic_tool", 1),
        },
        50_000,
        10,
    )
    .unwrap();
    assert_eq!(hole.footprint.width, 3);
    assert_eq!(hole.footprint.height, 3);
    assert_eq!(hole.footprint.tiles.len(), 9);

    assert_eq!(
        ConstructionProject::new(
            "construction_hole_bad_footprint",
            ConstructionTargetKind::HoleUpgrade,
            None,
            1,
            ScaffoldTier::Basic,
            footprint(1, 1),
            ConstructionBills {
                scaffold: bill("resource_logs", 2),
                structure: bill("resource_refined", 5),
                fit_out: bill("item_generic_tool", 1),
            },
            50_000,
            10,
        ),
        Err(ConstructionInvariantError::InvalidHoleWorkFootprint)
    );
}
