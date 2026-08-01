//! Focused contract for the pure LAI.41 Hole authority.

use cat_sim::{
    black_hole::{
        AXIS_MAX, AxisUpgradeProject, BLACK_HOLE_SCHEMA_VERSION, BlackHoleState,
        CatalogResolvedFeedPolicy, CommandApply, FeedEntry, FeedIdentity, FeedOrder,
        FeedPhysicalStage, FeedValueStage, HoleAxes, HoleAxis, MAX_COMMAND_RECEIPTS,
        MAX_OUTPUT_HISTORY, MAX_PENDING_UPGRADE_COMPLETIONS, MAX_RECOVERY_RECEIPTS,
        OPENING_GAME_MINUTES, RecoveryApply, RecoveryCause, RecoveryDisposition, UpgradeBill,
        UpgradeInput, UpgradePhysicalStage, UpgradeRecoveryRequest, credit_id, hole_footprint,
        opening_id, recovery_id, upgrade_bill,
    },
    content_manifest::{CapabilityId, ContentId, MaterialInstanceId, PhysicalLotId},
    quality_lots::{LotLocation, QualityBand},
    spatial_tasks::TilePoint,
};

fn content(value: &str) -> ContentId {
    ContentId::new(value).unwrap()
}
fn capability(value: &str) -> CapabilityId {
    CapabilityId::new(value).unwrap()
}
fn lot(value: &str) -> PhysicalLotId {
    PhysicalLotId::new(value).unwrap()
}

fn resolved(content_id: ContentId, quality: QualityBand) -> CatalogResolvedFeedPolicy {
    CatalogResolvedFeedPolicy {
        content_id,
        capability_id: capability("capability_hole_feed"),
        content_is_canonical: true,
        capability_is_owned: true,
        ownership_is_authorized: true,
        reservation_is_authorized: true,
        route_is_authorized: true,
        required_darkness: 0,
        maximum_quality: quality,
        base_value_milli: 200,
        stage: FeedValueStage::Prepared,
        installed_augmentation_value_milli: 50,
        current_condition: 3,
        maximum_condition: 4,
    }
}

fn entry(id: &str, units: u32) -> FeedEntry {
    let content_id = content("food_apple");
    FeedEntry {
        identity: FeedIdentity::BulkLot { lot_id: lot(id) },
        content_id: content_id.clone(),
        quality: QualityBand::Fine,
        provenance: "apple_tree".to_owned(),
        units,
        credited_units: 0,
        origin: LotLocation::Stockpile("storehouse_a".to_owned()),
        location: LotLocation::Stockpile("storehouse_a".to_owned()),
        reservation_id: "reservation_a".to_owned(),
        route_id: "route_a".to_owned(),
        stage: FeedPhysicalStage::Reserved,
        policy: CatalogResolvedFeedPolicy {
            installed_augmentation_value_milli: 0,
            current_condition: 1,
            maximum_condition: 1,
            ..resolved(content_id, QualityBand::Fine)
        },
    }
}

fn order(id: &str, command_id: &str, entries: Vec<FeedEntry>) -> FeedOrder {
    FeedOrder {
        id: id.to_owned(),
        command_id: command_id.to_owned(),
        entries,
        created_game_minute: 0,
    }
}

fn state(axes: HoleAxes) -> BlackHoleState {
    BlackHoleState::new("the_hole", TilePoint { x: 10, y: 20 }, axes, 40).unwrap()
}

fn upgrade_inputs(bill: &UpgradeBill) -> Vec<UpgradeInput> {
    let mut sequence = 0_u32;
    let mut inputs = Vec::new();
    for requirement in &bill.physical_inputs {
        let item_count = if requirement.content_id.as_str() == "item_generic_tool" {
            requirement.quantity
        } else {
            1
        };
        for _ in 0..item_count {
            let identity = if requirement.content_id.as_str() == "item_generic_tool" {
                FeedIdentity::ItemInstance {
                    item_id: MaterialInstanceId::new(format!("upgrade_item_{sequence}")).unwrap(),
                }
            } else {
                FeedIdentity::BulkLot {
                    lot_id: lot(&format!("upgrade_lot_{sequence}")),
                }
            };
            inputs.push(UpgradeInput {
                identity,
                content_id: requirement.content_id.clone(),
                quality: requirement.minimum_quality.unwrap_or(QualityBand::Crude),
                quantity: if item_count == 1 {
                    requirement.quantity
                } else {
                    1
                },
                provenance: format!("upgrade_source_{sequence}"),
                origin: LotLocation::Stockpile("upgrade_storehouse".to_owned()),
                location: LotLocation::Stockpile("upgrade_storehouse".to_owned()),
                reservation_id: format!("upgrade_reservation_{sequence}"),
                route_id: format!("upgrade_route_{sequence}"),
                stage: UpgradePhysicalStage::Reserved,
            });
            sequence += 1;
        }
    }
    inputs
}

fn upgrade_project(axes: HoleAxes, axis: HoleAxis, suffix: &str) -> AxisUpgradeProject {
    let bill = upgrade_bill(axes, axis).unwrap();
    let mut inputs = upgrade_inputs(&bill);
    for (index, input) in inputs.iter_mut().enumerate() {
        input.identity = match &input.identity {
            FeedIdentity::BulkLot { .. } => FeedIdentity::BulkLot {
                lot_id: lot(&format!("upgrade_lot_{suffix}_{index}")),
            },
            FeedIdentity::ItemInstance { .. } => FeedIdentity::ItemInstance {
                item_id: MaterialInstanceId::new(format!("upgrade_item_{suffix}_{index}")).unwrap(),
            },
        };
    }
    AxisUpgradeProject::new(
        format!("project_{suffix}"),
        format!("begin_upgrade_{suffix}"),
        axis,
        bill.target_level,
        inputs,
    )
}

fn deliver_upgrade(hole: &mut BlackHoleState, project_id: &str, suffix: &str) {
    let identities = hole
        .active_upgrade
        .as_ref()
        .unwrap()
        .inputs
        .iter()
        .map(|input| input.identity.clone())
        .collect::<Vec<_>>();
    for (index, identity) in identities.iter().enumerate() {
        assert_eq!(
            hole.mark_upgrade_carried(
                format!("carry_{suffix}_{index}"),
                project_id,
                identity,
                format!("cargo_{suffix}_{index}")
            ),
            Ok(CommandApply::Applied)
        );
        assert_eq!(
            hole.mark_upgrade_delivered(format!("deliver_{suffix}_{index}"), project_id, identity),
            Ok(CommandApply::Applied)
        );
    }
}

fn finish_single_feed(hole: &mut BlackHoleState, index: usize, drain_credit: bool) -> FeedOrder {
    let input = order(
        &format!("order_window_{index:04}"),
        &format!("command_window_{index:04}"),
        vec![entry(&format!("lot_window_{index:04}"), 1)],
    );
    let identity = input.entries[0].identity.clone();
    assert_eq!(hole.begin_feed(input.clone()), Ok(CommandApply::Applied));
    hole.mark_carried(&input.id, &identity, format!("cargo_window_{index:04}"))
        .unwrap();
    hole.mark_delivered(&input.id, &identity).unwrap();
    hole.advance_to(hole.next_opening_game_minute).unwrap();
    if drain_credit {
        assert_eq!(hole.take_credits(1).len(), 1);
    }
    input
}

#[test]
fn axes_geometry_and_opening_formula_are_exact() {
    for value in 0..=AXIS_MAX {
        let axes = HoleAxes::new(value, value, value).unwrap();
        assert_eq!(axes.intake_width(), 1 + u32::from(value));
        assert_eq!(axes.maximum_order_units(), 10 * (1 + u32::from(value)));
    }
    assert!(HoleAxes::new(11, 0, 0).is_err());
    assert!(HoleAxes::new(0, 11, 0).is_err());
    assert!(HoleAxes::new(0, 0, 11).is_err());

    let footprint = hole_footprint(TilePoint { x: 10, y: 20 }).unwrap();
    assert_eq!(footprint.landmark.tiles.as_slice().len(), 25);
    assert_eq!(footprint.work.tiles.as_slice().len(), 9);
    assert_eq!(footprint.ring.len(), 16);
    assert_eq!(footprint.ring[0], TilePoint { x: 10, y: 20 });
    assert_eq!(footprint.ring[15], TilePoint { x: 14, y: 24 });
    assert_eq!(
        footprint.work.tiles.as_slice()[0],
        TilePoint { x: 11, y: 21 }
    );
    assert_eq!(
        footprint.work.tiles.as_slice()[8],
        TilePoint { x: 13, y: 23 }
    );
    assert_eq!(footprint.pinned_delivery_edge, TilePoint { x: 12, y: 20 });
}

#[test]
fn capacity_counts_every_uncredited_physical_stage_and_feed_upgrade_are_independent() {
    let axes = HoleAxes::new(0, 0, 0).unwrap();
    let mut hole = state(axes);
    let mut queued = entry("lot_b", 2);
    queued.stage = FeedPhysicalStage::Queued;
    let mut carried = entry("lot_c", 3);
    carried.stage = FeedPhysicalStage::Carried;
    carried.location = LotLocation::Cargo("cargo_a".to_owned());
    let mut delivered = entry("lot_d", 4);
    delivered.stage = FeedPhysicalStage::Delivered;
    delivered.location = LotLocation::Hole("the_hole".to_owned());
    let too_large = order(
        "order_a",
        "command_a",
        vec![entry("lot_a", 2), queued, carried, delivered],
    );
    assert!(hole.begin_feed(too_large).is_err());

    let feed = order("order_a", "command_a", vec![entry("lot_a", 10)]);
    assert_eq!(hole.begin_feed(feed), Ok(CommandApply::Applied));
    let project = upgrade_project(axes, HoleAxis::Width, "independent");
    assert_eq!(
        hole.begin_upgrade(project),
        Ok(CommandApply::Applied),
        "one exact physical upgrade may coexist with one feed"
    );
    assert!(
        hole.begin_feed(order("order_b", "command_b", vec![entry("lot_b", 1)]))
            .is_err()
    );
}

#[test]
fn checked_micro_void_formula_has_one_final_floor_and_rejects_zero_maximum_condition() {
    assert_eq!(
        [
            FeedValueStage::Raw.value_percent(),
            FeedValueStage::Processed.value_percent(),
            FeedValueStage::Simple.value_percent(),
            FeedValueStage::Prepared.value_percent(),
            FeedValueStage::Complex.value_percent(),
            FeedValueStage::Feast.value_percent(),
        ],
        [100, 125, 125, 160, 210, 280],
    );
    assert_eq!(
        QualityBand::ALL.map(QualityBand::trade_hole_value_percent),
        [75, 100, 130, 170, 225],
    );
    let policy = resolved(content("food_apple"), QualityBand::Fine);
    // (200 + 50) * 1000 * 160 * 130 * 3 / (100 * 100 * 4)
    assert_eq!(policy.micro_void_for(QualityBand::Fine).unwrap(), 390_000);
    let mut invalid = policy;
    invalid.maximum_condition = 0;
    assert!(invalid.micro_void_for(QualityBand::Fine).is_err());
}

#[test]
fn delivered_intake_catches_up_by_absolute_openings_and_matches_partition_restart() {
    let mut single = state(HoleAxes::new(1, 0, 0).unwrap());
    let input = order("order_a", "command_a", vec![entry("lot_a", 5)]);
    single.begin_feed(input.clone()).unwrap();
    let identity = input.entries[0].identity.clone();
    single
        .mark_carried("order_a", &identity, "cargo_a".to_owned())
        .unwrap();
    single.mark_delivered("order_a", &identity).unwrap();
    let one_go = single.advance_to(120).unwrap();
    assert_eq!(one_go.len(), 3);
    assert_eq!(
        one_go
            .iter()
            .map(|opening| opening
                .credits
                .iter()
                .map(|credit| credit.quantity)
                .sum::<u32>())
            .collect::<Vec<_>>(),
        vec![2, 2, 1]
    );
    assert!(single.active_feed.is_none());
    assert_eq!(single.next_opening_game_minute, 160);

    let mut partitioned = state(HoleAxes::new(1, 0, 0).unwrap());
    partitioned.begin_feed(input).unwrap();
    partitioned
        .mark_carried("order_a", &identity, "cargo_a".to_owned())
        .unwrap();
    partitioned.mark_delivered("order_a", &identity).unwrap();
    partitioned.advance_to(40).unwrap();
    let restart_json = serde_json::to_string(&partitioned).unwrap();
    partitioned = serde_json::from_str(&restart_json).unwrap();
    partitioned.advance_to(80).unwrap();
    partitioned.advance_to(120).unwrap();
    assert_eq!(partitioned, single);
}

#[test]
fn policy_failure_is_atomic_and_does_not_apply_a_hidden_survival_veto() {
    let mut hole = state(HoleAxes::default());
    let before = hole.clone();
    let mut bad = entry("lot_a", 1);
    bad.policy.capability_is_owned = false;
    assert!(
        hole.begin_feed(order("order_a", "command_a", vec![bad]))
            .is_err()
    );
    assert_eq!(hole, before);

    let accepted = order("order_a", "command_a", vec![entry("lot_a", 1)]);
    assert_eq!(hole.begin_feed(accepted), Ok(CommandApply::Applied));
}

#[test]
fn policy_gate_failures_are_atomic_for_catalog_route_reservation_darkness_and_quality() {
    let mutations: &[fn(&mut FeedEntry)] = &[
        |entry| entry.policy.content_is_canonical = false,
        |entry| entry.policy.ownership_is_authorized = false,
        |entry| entry.policy.reservation_is_authorized = false,
        |entry| entry.policy.route_is_authorized = false,
        |entry| entry.policy.required_darkness = 1,
        |entry| entry.policy.maximum_quality = QualityBand::Crude,
    ];
    for mutation in mutations {
        let mut hole = state(HoleAxes::default());
        let before = hole.clone();
        let mut candidate = entry("lot_a", 1);
        mutation(&mut candidate);
        assert!(
            hole.begin_feed(order("order_a", "command_a", vec![candidate]))
                .is_err()
        );
        assert_eq!(hole, before);
    }
}

#[test]
fn zero_quantity_empty_location_and_value_overflow_fail_without_reserving_or_crediting() {
    let invalid_entries: &[fn() -> FeedEntry] = &[
        || entry("lot_a", 0),
        || {
            let mut candidate = entry("lot_a", 1);
            candidate.origin = LotLocation::Source(String::new());
            candidate.location = candidate.origin.clone();
            candidate
        },
        || {
            let mut candidate = entry("lot_a", 1);
            candidate.policy.base_value_milli = u64::MAX;
            candidate
        },
    ];
    for make_entry in invalid_entries {
        let mut hole = state(HoleAxes::default());
        let before = hole.clone();
        assert!(
            hole.begin_feed(order("order_a", "command_a", vec![make_entry()]))
                .is_err()
        );
        assert_eq!(hole, before);
        assert!(hole.credits().is_empty());
    }
}

#[test]
fn command_replay_conflict_and_recovery_preserve_identity_quality_provenance_and_quantity() {
    let mut hole = state(HoleAxes::default());
    let input = order("order_a", "command_a", vec![entry("lot_a", 1)]);
    assert_eq!(hole.begin_feed(input.clone()), Ok(CommandApply::Applied));
    assert_eq!(hole.begin_feed(input), Ok(CommandApply::AlreadyApplied));
    assert!(
        hole.begin_feed(order("order_a", "command_a", vec![entry("lot_b", 1)]))
            .is_err()
    );

    let identity = FeedIdentity::BulkLot {
        lot_id: lot("lot_a"),
    };
    let recovery = recovery_id("order_a", &identity, 0);
    assert_eq!(
        hole.recover_entry(
            recovery.clone(),
            "order_a",
            &identity,
            RecoveryCause::Cancelled,
            RecoveryDisposition::ReleasedAtOrigin
        ),
        Ok(RecoveryApply::Applied)
    );
    assert_eq!(
        hole.recover_entry(
            recovery,
            "order_a",
            &identity,
            RecoveryCause::Cancelled,
            RecoveryDisposition::ReleasedAtOrigin
        ),
        Ok(RecoveryApply::AlreadyApplied)
    );
    let recovered = &hole.terminal_entries()[0];
    assert_eq!(recovered.identity, identity);
    assert_eq!(recovered.quality, QualityBand::Fine);
    assert_eq!(recovered.provenance, "apple_tree");
    assert_eq!(recovered.units, 1);
    assert!(recovered.reservation_id.is_empty());
}

#[test]
fn carried_and_delivered_recovery_use_origin_stockpile_or_last_land_cache_without_laundering() {
    for (position, disposition) in [
        RecoveryDisposition::ReturnedToOrigin,
        RecoveryDisposition::NearestStockpile {
            stockpile_id: "storehouse_b".to_owned(),
        },
        RecoveryDisposition::LastLandCache {
            cache_id: "cache_a".to_owned(),
            tile: TilePoint { x: 4, y: 9 },
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut hole = state(HoleAxes::default());
        let input = order("order_a", "command_a", vec![entry("lot_a", 1)]);
        let identity = input.entries[0].identity.clone();
        hole.begin_feed(input).unwrap();
        hole.mark_carried("order_a", &identity, "cargo_a".to_owned())
            .unwrap();
        if position == 0 {
            hole.mark_delivered("order_a", &identity).unwrap();
        }
        let id = recovery_id("order_a", &identity, 0);
        let cause = [
            RecoveryCause::CarrierDeath,
            RecoveryCause::RouteLost,
            RecoveryCause::Interrupted,
        ][position];
        assert_eq!(
            hole.recover_entry(id, "order_a", &identity, cause, disposition.clone()),
            Ok(RecoveryApply::Applied)
        );
        let recovered = &hole.terminal_entries()[0];
        assert_eq!(recovered.identity, identity);
        assert_eq!(recovered.quality, QualityBand::Fine);
        assert_eq!(recovered.provenance, "apple_tree");
        assert_eq!(recovered.units, 1);
        assert!(recovered.reservation_id.is_empty());
    }
}

#[test]
fn upgrade_bills_are_exact_physical_only_progression() {
    let width = upgrade_bill(HoleAxes::new(3, 0, 0).unwrap(), HoleAxis::Width).unwrap();
    assert_eq!(width.target_level, 4);
    assert_eq!(
        width
            .physical_inputs
            .iter()
            .map(|value| (
                value.content_id.as_str(),
                value.quantity,
                value.minimum_quality
            ))
            .collect::<Vec<_>>(),
        vec![
            ("resource_refined", 20, None),
            ("resource_logs", 8, None),
            ("resource_planks", 2, None),
            ("item_generic_tool", 1, Some(QualityBand::Crude)),
        ]
    );
    let darkness = upgrade_bill(HoleAxes::new(0, 0, 9).unwrap(), HoleAxis::Darkness).unwrap();
    assert!(
        darkness
            .physical_inputs
            .iter()
            .any(|value| value.content_id.as_str() == "resource_gem" && value.quantity == 4)
    );
    assert!(
        darkness
            .physical_inputs
            .iter()
            .any(|value| value.content_id.as_str() == "resource_metal" && value.quantity == 8)
    );
    assert!(
        darkness
            .physical_inputs
            .iter()
            .any(
                |value| value.minimum_quality == Some(QualityBand::Masterwork)
                    && value.quantity == 3
            )
    );

    let expected_tools = [
        None,
        Some((QualityBand::Crude, 1)),
        Some((QualityBand::Crude, 1)),
        Some((QualityBand::Crude, 1)),
        Some((QualityBand::Common, 1)),
        Some((QualityBand::Common, 1)),
        Some((QualityBand::Fine, 2)),
        Some((QualityBand::Fine, 2)),
        Some((QualityBand::Superior, 2)),
        Some((QualityBand::Masterwork, 3)),
    ];
    for (offset, expected_tool) in expected_tools.into_iter().enumerate() {
        let current = u8::try_from(offset).unwrap();
        let bill = upgrade_bill(HoleAxes::new(current, 0, 0).unwrap(), HoleAxis::Width).unwrap();
        let level = current + 1;
        assert_eq!(bill.physical_inputs[0].quantity, 5 * u32::from(level));
        assert_eq!(bill.physical_inputs[1].quantity, 2 * u32::from(level));
        assert_eq!(
            bill.physical_inputs
                .iter()
                .find(|input| input.content_id.as_str() == "item_generic_tool")
                .map(|input| (input.minimum_quality.unwrap(), input.quantity)),
            expected_tool
        );
    }

    let axes = HoleAxes::new(0, 0, 3).unwrap();
    let project = upgrade_project(axes, HoleAxis::Darkness, "darkness_duplicate");
    assert_eq!(
        project
            .inputs
            .iter()
            .filter(|input| input.content_id.as_str() == "resource_refined")
            .map(|input| input.quantity)
            .sum::<u32>(),
        22
    );
    let project_id = project.id.clone();
    let mut hole = state(axes);
    assert_eq!(hole.begin_upgrade(project), Ok(CommandApply::Applied));
    deliver_upgrade(&mut hole, &project_id, "darkness_duplicate");
    assert_eq!(
        hole.complete_upgrade("complete_darkness_duplicate".to_owned(), &project_id),
        Ok(CommandApply::Applied)
    );
    assert_eq!(hole.axes.darkness, 4);
}

#[test]
fn upgrade_levels_one_four_seven_and_ten_require_the_bound_physical_bill() {
    for target_level in [1_u8, 4, 7, 10] {
        let axes = HoleAxes::new(target_level - 1, 0, 0).unwrap();
        let suffix = format!("level_{target_level}");
        let project_id = format!("project_{suffix}");
        let mut hole = state(axes);
        let project = upgrade_project(axes, HoleAxis::Width, &suffix);
        let exact_bill = upgrade_bill(axes, HoleAxis::Width).unwrap();
        assert_eq!(hole.begin_upgrade(project), Ok(CommandApply::Applied));
        assert_eq!(
            hole.active_upgrade.as_ref().unwrap().bound_bill(),
            Some(&exact_bill)
        );
        assert!(
            hole.active_upgrade
                .as_ref()
                .unwrap()
                .inputs
                .iter()
                .all(|input| input.stage == UpgradePhysicalStage::Reserved
                    && !input.reservation_id.is_empty())
        );
        let original_inputs = hole.active_upgrade.as_ref().unwrap().inputs.clone();

        let reserved = hole.clone();
        assert!(
            hole.complete_upgrade(format!("complete_early_{target_level}"), &project_id)
                .is_err(),
            "reserved cargo cannot complete before physical delivery"
        );
        assert_eq!(hole, reserved);

        deliver_upgrade(&mut hole, &project_id, &suffix);
        let restart = serde_json::to_string(&hole).unwrap();
        hole = serde_json::from_str(&restart).unwrap();
        assert_eq!(
            hole.complete_upgrade(format!("complete_{target_level}"), &project_id),
            Ok(CommandApply::Applied)
        );
        assert_eq!(
            hole.complete_upgrade(format!("complete_{target_level}"), &project_id),
            Ok(CommandApply::AlreadyApplied)
        );
        assert_eq!(hole.axes.width, target_level);
        assert!(hole.active_upgrade.is_none());
        assert_eq!(hole.micro_void_balance, 0);
        let completed = hole.take_completed_upgrades(1);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].bound_bill, exact_bill);
        assert_eq!(
            completed[0]
                .consumed_inputs
                .iter()
                .map(|input| (
                    &input.identity,
                    &input.content_id,
                    input.quality,
                    input.quantity,
                    &input.provenance,
                    &input.route_id,
                ))
                .collect::<Vec<_>>(),
            original_inputs
                .iter()
                .map(|input| (
                    &input.identity,
                    &input.content_id,
                    input.quality,
                    input.quantity,
                    &input.provenance,
                    &input.route_id,
                ))
                .collect::<Vec<_>>()
        );
        assert!(completed[0].consumed_inputs.iter().all(|input| {
            input.stage == UpgradePhysicalStage::Consumed
                && input.reservation_id.is_empty()
                && input.location == LotLocation::Hole("the_hole".to_owned())
        }));
    }
}

#[test]
fn completed_upgrade_payload_backpressure_preserves_the_delivered_project_until_drain() {
    let mut hole = state(HoleAxes::default());
    for sequence in 0..MAX_PENDING_UPGRADE_COMPLETIONS {
        let axis = if sequence < 10 {
            HoleAxis::Width
        } else {
            HoleAxis::Depth
        };
        let suffix = format!("completion_window_{sequence}");
        let project = upgrade_project(hole.axes, axis, &suffix);
        let project_id = project.id.clone();
        hole.begin_upgrade(project).unwrap();
        deliver_upgrade(&mut hole, &project_id, &suffix);
        hole.complete_upgrade(format!("complete_window_{sequence}"), &project_id)
            .unwrap();
    }
    assert_eq!(
        hole.completed_upgrades().len(),
        MAX_PENDING_UPGRADE_COMPLETIONS
    );

    let suffix = "completion_backpressure";
    let project = upgrade_project(hole.axes, HoleAxis::Depth, suffix);
    let project_id = project.id.clone();
    hole.begin_upgrade(project).unwrap();
    deliver_upgrade(&mut hole, &project_id, suffix);
    let before = hole.clone();
    assert!(
        hole.complete_upgrade("complete_backpressure".to_owned(), &project_id)
            .is_err()
    );
    assert_eq!(hole, before);
    assert!(hole.active_upgrade.is_some());

    assert_eq!(hole.take_completed_upgrades(1).len(), 1);
    assert_eq!(
        hole.complete_upgrade("complete_backpressure".to_owned(), &project_id),
        Ok(CommandApply::Applied)
    );
    assert_eq!(
        hole.completed_upgrades().len(),
        MAX_PENDING_UPGRADE_COMPLETIONS
    );
    assert!(hole.active_upgrade.is_none());
}

#[test]
fn upgrade_rejects_missing_wrong_duplicate_quality_and_amount_without_mutation() {
    let axes = HoleAxes::new(3, 0, 0).unwrap();
    let exact = upgrade_project(axes, HoleAxis::Width, "exact");
    let mut invalid = Vec::new();

    let mut missing = exact.clone();
    missing.inputs.pop();
    invalid.push(missing);

    let mut wrong_content = exact.clone();
    wrong_content.inputs[0].content_id = content("resource_stone");
    invalid.push(wrong_content);

    let mut wrong_amount = exact.clone();
    wrong_amount.inputs[0].quantity += 1;
    invalid.push(wrong_amount);

    let mut duplicate = exact.clone();
    duplicate.inputs[1].identity = duplicate.inputs[0].identity.clone();
    invalid.push(duplicate);

    let mut extra = exact.clone();
    let mut extra_input = extra.inputs[0].clone();
    extra_input.identity = FeedIdentity::BulkLot {
        lot_id: lot("upgrade_extra_lot"),
    };
    extra_input.quantity = 1;
    extra.inputs.push(extra_input);
    invalid.push(extra);

    let mut wrong_stage = exact.clone();
    wrong_stage.inputs[0].stage = UpgradePhysicalStage::Queued;
    wrong_stage.inputs[0].reservation_id.clear();
    invalid.push(wrong_stage);

    let mut wrong_location = exact.clone();
    wrong_location.inputs[0].stage = UpgradePhysicalStage::Carried;
    invalid.push(wrong_location);

    let mut wrong_route = exact.clone();
    wrong_route.inputs[0].route_id.clear();
    invalid.push(wrong_route);

    let mut missing_provenance = exact.clone();
    missing_provenance.inputs[0].provenance.clear();
    invalid.push(missing_provenance);

    for project in invalid {
        let mut hole = state(axes);
        let before = hole.clone();
        assert!(hole.begin_upgrade(project).is_err());
        assert_eq!(hole, before);
    }

    let quality_axes = HoleAxes::new(4, 0, 0).unwrap();
    let mut wrong_quality = upgrade_project(quality_axes, HoleAxis::Width, "quality");
    wrong_quality
        .inputs
        .iter_mut()
        .find(|input| input.content_id.as_str() == "item_generic_tool")
        .unwrap()
        .quality = QualityBand::Crude;
    let mut hole = state(quality_axes);
    let before = hole.clone();
    assert!(hole.begin_upgrade(wrong_quality).is_err());
    assert_eq!(hole, before);
}

#[test]
fn upgrade_stage_commands_are_idempotent_and_restart_partition_stable() {
    let axes = HoleAxes::new(6, 0, 0).unwrap();
    let project = upgrade_project(axes, HoleAxis::Width, "partition");
    let project_id = project.id.clone();
    let mut uninterrupted = state(axes);
    uninterrupted.begin_upgrade(project).unwrap();
    let mut restarted = uninterrupted.clone();

    deliver_upgrade(&mut uninterrupted, &project_id, "partition");
    deliver_upgrade(&mut restarted, &project_id, "partition");
    let identity = restarted.active_upgrade.as_ref().unwrap().inputs[0]
        .identity
        .clone();
    assert_eq!(
        restarted.mark_upgrade_delivered("deliver_partition_0".to_owned(), &project_id, &identity),
        Ok(CommandApply::AlreadyApplied)
    );
    let other_identity = restarted.active_upgrade.as_ref().unwrap().inputs[1]
        .identity
        .clone();
    assert!(
        restarted
            .mark_upgrade_delivered(
                "deliver_partition_0".to_owned(),
                &project_id,
                &other_identity
            )
            .is_err(),
        "a retained command ID must conflict with different physical content"
    );

    restarted = serde_json::from_str(&serde_json::to_string(&restarted).unwrap()).unwrap();
    uninterrupted
        .complete_upgrade("complete_partition".to_owned(), &project_id)
        .unwrap();
    restarted
        .complete_upgrade("complete_partition".to_owned(), &project_id)
        .unwrap();
    assert_eq!(restarted, uninterrupted);
}

#[test]
fn upgrade_interruption_recovers_every_unconsumed_identity_without_void() {
    let axes = HoleAxes::new(3, 0, 0).unwrap();
    let project = upgrade_project(axes, HoleAxis::Width, "recovery");
    let project_id = project.id.clone();
    let mut hole = state(axes);
    hole.begin_upgrade(project).unwrap();
    let identities = hole
        .active_upgrade
        .as_ref()
        .unwrap()
        .inputs
        .iter()
        .map(|input| input.identity.clone())
        .collect::<Vec<_>>();

    hole.mark_upgrade_carried(
        "recover_carry_1".to_owned(),
        &project_id,
        &identities[1],
        "recover_cargo_1".to_owned(),
    )
    .unwrap();
    hole.mark_upgrade_carried(
        "recover_carry_2".to_owned(),
        &project_id,
        &identities[2],
        "recover_cargo_2".to_owned(),
    )
    .unwrap();
    hole.mark_upgrade_delivered("recover_deliver_2".to_owned(), &project_id, &identities[2])
        .unwrap();

    let requests = hole
        .active_upgrade
        .as_ref()
        .unwrap()
        .inputs
        .iter()
        .map(|input| UpgradeRecoveryRequest {
            identity: input.identity.clone(),
            disposition: match input.stage {
                UpgradePhysicalStage::Reserved => RecoveryDisposition::ReleasedAtOrigin,
                UpgradePhysicalStage::Carried => RecoveryDisposition::NearestStockpile {
                    stockpile_id: "recovery_storehouse".to_owned(),
                },
                UpgradePhysicalStage::Delivered => RecoveryDisposition::LastLandCache {
                    cache_id: "upgrade_cache".to_owned(),
                    tile: TilePoint { x: 8, y: 12 },
                },
                _ => panic!("test project contains no terminal stage"),
            },
        })
        .collect::<Vec<_>>();
    let before_restart = serde_json::to_string(&hole).unwrap();
    hole = serde_json::from_str(&before_restart).unwrap();
    assert_eq!(
        hole.recover_upgrade(
            "upgrade_recovery_receipt".to_owned(),
            &project_id,
            RecoveryCause::RouteLost,
            requests.clone()
        ),
        Ok(RecoveryApply::Applied)
    );
    assert_eq!(
        hole.recover_upgrade(
            "upgrade_recovery_receipt".to_owned(),
            &project_id,
            RecoveryCause::RouteLost,
            requests.clone()
        ),
        Ok(RecoveryApply::AlreadyApplied)
    );
    assert!(
        hole.recover_upgrade(
            "upgrade_recovery_receipt".to_owned(),
            &project_id,
            RecoveryCause::Cancelled,
            requests
        )
        .is_err()
    );
    assert!(hole.active_upgrade.is_none());
    assert_eq!(hole.micro_void_balance, 0);
    assert_eq!(hole.terminal_upgrade_recoveries().len(), 1);
    let recovered = &hole.terminal_upgrade_recoveries()[0];
    assert_eq!(
        recovered.bound_bill,
        upgrade_bill(axes, HoleAxis::Width).unwrap()
    );
    assert_eq!(recovered.recovered_inputs.len(), identities.len());
    assert!(recovered.recovered_inputs.iter().all(|recovery| {
        recovery.input.reservation_id.is_empty()
            && recovery.input.location != LotLocation::Hole("the_hole".to_owned())
            && identities.contains(&recovery.input.identity)
            && recovery.input.provenance.starts_with("upgrade_source_")
            && recovery.input.route_id.starts_with("upgrade_route_")
    }));
    let pending_restart: BlackHoleState =
        serde_json::from_str(&serde_json::to_string(&hole).unwrap()).unwrap();
    assert_eq!(pending_restart, hole);
    let mut unknown_recovery = serde_json::to_value(&hole).unwrap();
    unknown_recovery["terminalUpgradeRecoveries"][0]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(unknown_recovery).is_err());
    let drained = hole.take_terminal_upgrade_recoveries(usize::MAX);
    assert_eq!(drained.len(), 1);
    assert!(hole.terminal_upgrade_recoveries().is_empty());
    let restarted: BlackHoleState =
        serde_json::from_str(&serde_json::to_string(&hole).unwrap()).unwrap();
    assert_eq!(restarted, hole);
}

#[test]
fn upgrade_recovery_backpressure_preserves_the_whole_reserved_bill_until_drain() {
    let mut hole = state(HoleAxes::default());
    for sequence in 0..MAX_OUTPUT_HISTORY {
        let suffix = format!("upgrade_recovery_window_{sequence}");
        let project = upgrade_project(hole.axes, HoleAxis::Width, &suffix);
        let project_id = project.id.clone();
        let requests = project
            .inputs
            .iter()
            .map(|input| UpgradeRecoveryRequest {
                identity: input.identity.clone(),
                disposition: RecoveryDisposition::ReleasedAtOrigin,
            })
            .collect::<Vec<_>>();
        hole.begin_upgrade(project).unwrap();
        hole.recover_upgrade(
            format!("upgrade_recovery_receipt_{sequence}"),
            &project_id,
            [
                RecoveryCause::Cancelled,
                RecoveryCause::CarrierDeath,
                RecoveryCause::RouteLost,
                RecoveryCause::Interrupted,
            ][sequence % 4],
            requests,
        )
        .unwrap();
    }
    assert_eq!(hole.terminal_upgrade_recoveries().len(), MAX_OUTPUT_HISTORY);

    let suffix = "upgrade_recovery_backpressure";
    let project = upgrade_project(hole.axes, HoleAxis::Width, suffix);
    let project_id = project.id.clone();
    let requests = project
        .inputs
        .iter()
        .map(|input| UpgradeRecoveryRequest {
            identity: input.identity.clone(),
            disposition: RecoveryDisposition::ReleasedAtOrigin,
        })
        .collect::<Vec<_>>();
    hole.begin_upgrade(project).unwrap();
    let before = hole.clone();
    assert!(
        hole.recover_upgrade(
            "upgrade_recovery_backpressure_receipt".to_owned(),
            &project_id,
            RecoveryCause::Interrupted,
            requests.clone(),
        )
        .is_err()
    );
    assert_eq!(hole, before);
    assert_eq!(hole.take_terminal_upgrade_recoveries(1).len(), 1);
    assert_eq!(
        hole.recover_upgrade(
            "upgrade_recovery_backpressure_receipt".to_owned(),
            &project_id,
            RecoveryCause::Interrupted,
            requests,
        ),
        Ok(RecoveryApply::Applied)
    );
    assert_eq!(hole.terminal_upgrade_recoveries().len(), MAX_OUTPUT_HISTORY);
    assert!(hole.active_upgrade.is_none());
}

#[test]
fn receipt_window_prunes_by_persisted_sequence_and_feeds_continue_indefinitely() {
    let mut hole = state(HoleAxes::default());
    let feed_count = MAX_COMMAND_RECEIPTS + 32;
    let mut last_order = None;
    for index in 0..feed_count {
        last_order = Some(finish_single_feed(&mut hole, index, true));
    }
    assert!(hole.active_feed.is_none());
    assert!(hole.credits().is_empty());

    let wire = serde_json::to_value(&hole).unwrap();
    let receipts = wire["commandReceipts"].as_array().unwrap();
    assert_eq!(receipts.len(), MAX_COMMAND_RECEIPTS);
    assert_eq!(
        wire["nextCommandSequence"],
        serde_json::json!(feed_count as u64)
    );
    assert!(
        !receipts
            .iter()
            .any(|receipt| receipt["id"] == serde_json::json!("command_window_0000"))
    );
    assert!(receipts.iter().any(|receipt| {
        receipt["id"] == serde_json::json!(format!("command_window_{:04}", feed_count - 1))
    }));

    let last_order = last_order.unwrap();
    assert_eq!(
        hole.begin_feed(last_order.clone()),
        Ok(CommandApply::AlreadyApplied)
    );
    let mut conflict = last_order;
    conflict.entries[0].identity = FeedIdentity::BulkLot {
        lot_id: lot("conflicting_retry_lot"),
    };
    assert!(hole.begin_feed(conflict).is_err());

    let mut restarted: BlackHoleState =
        serde_json::from_str(&serde_json::to_string(&hole).unwrap()).unwrap();
    let mut uninterrupted = hole;
    for index in feed_count..feed_count + 32 {
        finish_single_feed(&mut uninterrupted, index, true);
        finish_single_feed(&mut restarted, index, true);
    }
    assert_eq!(restarted, uninterrupted);
}

#[test]
fn credit_backpressure_is_atomic_and_drain_allows_the_same_identity_to_continue() {
    let mut hole = state(HoleAxes::default());
    for index in 0..MAX_OUTPUT_HISTORY {
        finish_single_feed(&mut hole, index, false);
    }
    assert_eq!(hole.credits().len(), MAX_OUTPUT_HISTORY);

    let index = MAX_OUTPUT_HISTORY;
    let pending = order(
        &format!("order_window_{index:04}"),
        &format!("command_window_{index:04}"),
        vec![entry(&format!("lot_window_{index:04}"), 1)],
    );
    let identity = pending.entries[0].identity.clone();
    hole.begin_feed(pending.clone()).unwrap();
    hole.mark_carried(&pending.id, &identity, "backpressure_cargo".to_owned())
        .unwrap();
    hole.mark_delivered(&pending.id, &identity).unwrap();
    let before = hole.clone();
    assert!(hole.advance_to(hole.next_opening_game_minute).is_err());
    assert_eq!(hole, before, "backpressure must not consume physical input");

    assert_eq!(
        hole.take_credits(MAX_OUTPUT_HISTORY).len(),
        MAX_OUTPUT_HISTORY
    );
    assert_eq!(
        hole.advance_to(hole.next_opening_game_minute)
            .unwrap()
            .iter()
            .flat_map(|opening| &opening.credits)
            .map(|credit| credit.identity.clone())
            .collect::<Vec<_>>(),
        vec![identity]
    );
}

#[test]
fn recovery_output_backpressure_never_discards_identity_and_can_be_drained() {
    let mut hole = state(HoleAxes::default());
    for index in 0..MAX_OUTPUT_HISTORY {
        let input = order(
            &format!("recovery_order_{index:04}"),
            &format!("recovery_command_{index:04}"),
            vec![entry(&format!("recovery_lot_{index:04}"), 1)],
        );
        let identity = input.entries[0].identity.clone();
        hole.begin_feed(input.clone()).unwrap();
        hole.recover_entry(
            format!("recovery_receipt_{index:04}"),
            &input.id,
            &identity,
            RecoveryCause::Cancelled,
            RecoveryDisposition::ReleasedAtOrigin,
        )
        .unwrap();
    }
    assert_eq!(hole.terminal_entries().len(), MAX_OUTPUT_HISTORY);

    let index = MAX_OUTPUT_HISTORY;
    let pending = order(
        &format!("recovery_order_{index:04}"),
        &format!("recovery_command_{index:04}"),
        vec![entry(&format!("recovery_lot_{index:04}"), 1)],
    );
    let identity = pending.entries[0].identity.clone();
    hole.begin_feed(pending.clone()).unwrap();
    let before = hole.clone();
    let receipt_id = format!("recovery_receipt_{index:04}");
    assert!(
        hole.recover_entry(
            receipt_id.clone(),
            &pending.id,
            &identity,
            RecoveryCause::Cancelled,
            RecoveryDisposition::ReleasedAtOrigin,
        )
        .is_err()
    );
    assert_eq!(hole, before);
    assert_eq!(hole.take_terminal_entries(1).len(), 1);
    assert_eq!(
        hole.recover_entry(
            receipt_id,
            &pending.id,
            &identity,
            RecoveryCause::Cancelled,
            RecoveryDisposition::ReleasedAtOrigin,
        ),
        Ok(RecoveryApply::Applied)
    );
    assert!(
        hole.terminal_entries()
            .iter()
            .any(|entry| entry.identity == identity)
    );
    assert_eq!(
        hole.take_terminal_entries(usize::MAX).len(),
        MAX_OUTPUT_HISTORY
    );
    assert!(hole.terminal_entries().is_empty());
    let wire = serde_json::to_value(&hole).unwrap();
    assert_eq!(
        wire["recoveryReceipts"].as_array().unwrap().len(),
        MAX_RECOVERY_RECEIPTS
    );
    assert_eq!(
        wire["nextRecoverySequence"],
        serde_json::json!((MAX_OUTPUT_HISTORY + 1) as u64)
    );
    let restarted: BlackHoleState =
        serde_json::from_str(&serde_json::to_string(&hole).unwrap()).unwrap();
    assert_eq!(restarted, hole);
}

#[test]
fn strict_state_decode_and_stable_ids_reject_future_unknown_and_malformed_input() {
    let hole = state(HoleAxes::default());
    let json = serde_json::to_value(&hole).unwrap();
    assert_eq!(json["schemaVersion"], BLACK_HOLE_SCHEMA_VERSION);
    let mut future = json.clone();
    future["schemaVersion"] = serde_json::json!(BLACK_HOLE_SCHEMA_VERSION + 1);
    assert!(serde_json::from_value::<BlackHoleState>(future).is_err());
    let mut unknown = json.clone();
    unknown["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(unknown).is_err());
    let mut missing_window = json.clone();
    missing_window
        .as_object_mut()
        .unwrap()
        .remove("nextCommandSequence");
    assert!(serde_json::from_value::<BlackHoleState>(missing_window).is_err());
    let mut nested_unknown = json.clone();
    nested_unknown["anchor"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(nested_unknown).is_err());
    let mut active = state(HoleAxes::default());
    active
        .begin_feed(order("order_a", "command_a", vec![entry("lot_a", 1)]))
        .unwrap();
    let mut nested_location = serde_json::to_value(&active).unwrap();
    nested_location["activeFeed"]["entries"][0]["origin"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(nested_location).is_err());
    let mut malformed = json.clone();
    malformed["axes"]["width"] = serde_json::json!(11);
    assert!(serde_json::from_value::<BlackHoleState>(malformed).is_err());

    let axes = HoleAxes::new(0, 0, 0).unwrap();
    let mut upgrading = state(axes);
    upgrading
        .begin_upgrade(upgrade_project(axes, HoleAxis::Width, "strict"))
        .unwrap();
    let mut unknown_upgrade_input = serde_json::to_value(&upgrading).unwrap();
    unknown_upgrade_input["activeUpgrade"]["inputs"][0]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(unknown_upgrade_input).is_err());
    let mut forged_bill = serde_json::to_value(&upgrading).unwrap();
    forged_bill["activeUpgrade"]["boundBill"]["physicalInputs"][0]["quantity"] =
        serde_json::json!(999);
    assert!(serde_json::from_value::<BlackHoleState>(forged_bill).is_err());

    let project_id = upgrading.active_upgrade.as_ref().unwrap().id.clone();
    deliver_upgrade(&mut upgrading, &project_id, "strict");
    upgrading
        .complete_upgrade("complete_strict".to_owned(), &project_id)
        .unwrap();
    let mut unknown_completion = serde_json::to_value(&upgrading).unwrap();
    unknown_completion["completedUpgrades"][0]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BlackHoleState>(unknown_completion).is_err());
    let mut forged_consumed_reservation = serde_json::to_value(&upgrading).unwrap();
    forged_consumed_reservation["completedUpgrades"][0]["consumedInputs"][0]["reservationId"] =
        serde_json::json!("forged_reservation");
    assert!(serde_json::from_value::<BlackHoleState>(forged_consumed_reservation).is_err());
    assert_eq!(opening_id("order_a", 0), opening_id("order_a", 0));
    assert_eq!(credit_id("order_a", 0, 0), credit_id("order_a", 0, 0));
    assert!(MaterialInstanceId::new("item_a").is_ok());
    assert!(PhysicalLotId::new("lot_a").is_ok());
    assert_eq!(OPENING_GAME_MINUTES, 40);
}

#[test]
fn source_contains_no_replaced_domain_type_or_saturating_ledger_adapter() {
    let source = include_str!("../src/black_hole.rs");
    for replaced in [
        concat!("Resource", "Kind"),
        concat!("Child", "Load"),
        concat!("Shr", "ine"),
        concat!("Fav", "or"),
        concat!("Bles", "sing"),
        concat!("Ins", "ight"),
        "saturating_",
        "items::",
    ] {
        assert!(
            !source.contains(replaced),
            "replaced authority leaked: {replaced}"
        );
    }
}
