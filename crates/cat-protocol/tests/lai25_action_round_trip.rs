use cat_protocol::{
    ActionAcceptedResult, ActionAuthorityClass, ActionConflict, ActionDecodeError,
    ActionIdempotencyId, ActionProtocolVersion, ActionReplayResult, ActionValidationPipeline,
    ActionValidationStep, AuthenticatedPlayerId, AuthorityDenialReason, BoundedActionId,
    BoundedBasisPointNudge, BoundedBasisPoints, BoundedColonyId, BoundedEntityId,
    BoundedFavorAmount, BoundedPlayerId, BoundedStandingOrderText, BoundedTradeAmount,
    BuildingType, CropKind, CurrentStateHint, CurrentVersionHint, DiplomacyRelationshipTarget,
    DismissalReason, ExpectedStateVersions, LeaderAiActionEnvelope, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, OfficerAuthorityMode, OfficerRole,
    PhysicalPlacementActionPayload, PlayerOnlyAction, ReportSafeString, ResourceKind,
    SelectedColonyId, SiteRefActionTarget, SnapshotTilePoint, StaleClientRefresh,
    StandingOrderPatch, TradeRejectionReason, TransportMode,
};
use serde_json::{Value, json};

fn action_id(value: &str) -> BoundedActionId {
    BoundedActionId::new(value).expect("valid action id")
}

fn entity(value: &str) -> BoundedEntityId {
    BoundedEntityId::new(value).expect("valid entity id")
}

fn colony(value: &str) -> BoundedColonyId {
    BoundedColonyId::new(value).expect("valid colony id")
}

fn text(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("valid report-safe string")
}

fn instruction(value: &str) -> BoundedStandingOrderText {
    BoundedStandingOrderText::new(value).expect("valid standing order")
}

fn versions() -> ExpectedStateVersions {
    ExpectedStateVersions {
        expected_planner_version: 10,
        expected_domain_version: 11,
        expected_resource_version: 12,
        expected_spatial_version: Some(13),
        expected_reservation_version: Some(14),
        expected_research_version: Some(15),
        expected_scholar_version: Some(16),
        expected_boost_version: Some(17),
        expected_diplomacy_version: Some(18),
        expected_trade_version: Some(19),
        expected_prosthetic_version: Some(20),
        expected_care_version: Some(21),
        expected_officer_version: Some(22),
        expected_standing_order_version: Some(23),
    }
}

fn envelope(payload: LeaderAiActionPayload) -> LeaderAiActionEnvelope {
    LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:test:1").expect("valid idempotency id"),
        colony_id: SelectedColonyId::new("colony:home").expect("valid colony id"),
        player_id: AuthenticatedPlayerId::new("player:one").expect("valid player id"),
        expected_versions: versions(),
        payload,
    }
}

fn tile(x: i32, y: i32) -> SnapshotTilePoint {
    SnapshotTilePoint { x, y }
}

fn path() -> SiteRefActionTarget {
    SiteRefActionTarget::OrderedPath {
        ordered_tiles: vec![tile(1, 2), tile(2, 2)],
    }
}

fn all_domain_payloads() -> Vec<LeaderAiActionPayload> {
    vec![
        LeaderAiActionPayload::NudgePlan {
            plan_id: entity("plan:one"),
            nudge: BoundedBasisPointNudge::new(1_500).expect("valid nudge"),
            reason_key: Some(entity("reason:priority")),
        },
        LeaderAiActionPayload::CreateStandingOrder {
            order_kind: entity("order:maintain"),
            domain: entity("domain:food"),
            target_id: Some(entity("stockpile:main")),
            instruction: instruction("Maintain the reported reserve"),
            priority_basis_points: BoundedBasisPoints::new(7_500).expect("valid priority"),
            expires_at_ms: None,
        },
        LeaderAiActionPayload::UpdateStandingOrder {
            standing_order_id: entity("standing-order:one"),
            patch: StandingOrderPatch {
                instruction: Some(instruction("Raise the reported reserve")),
                priority_basis_points: None,
                target_id: None,
                clear_target: false,
                expires_at_ms: None,
                clear_expiry: false,
            },
        },
        LeaderAiActionPayload::DeleteStandingOrder {
            standing_order_id: entity("standing-order:one"),
        },
        LeaderAiActionPayload::DismissIntent {
            intent_id: entity("intent:one"),
            planning_epoch: 4,
            reason: DismissalReason::NoLongerDesired,
        },
        LeaderAiActionPayload::AppointOfficer {
            role: OfficerRole::Steward,
            cat_id: entity("cat:steward"),
        },
        LeaderAiActionPayload::UnappointOfficer {
            role: OfficerRole::Steward,
        },
        LeaderAiActionPayload::OfficerAuthorityOverride {
            role: OfficerRole::Steward,
            domain: entity("domain:stockpile"),
            request_id: Some(entity("request:one")),
            mode: OfficerAuthorityMode::Grant,
        },
        LeaderAiActionPayload::RequestTreatment {
            cat_id: entity("cat:patient"),
            injury_id: entity("injury:one"),
            treatment_kind: entity("treatment:splint"),
        },
        LeaderAiActionPayload::FitProsthetic {
            cat_id: entity("cat:patient"),
            prosthetic_id: entity("prosthetic:one"),
            body_part_id: entity("body-part:left-forepaw"),
            fitting_site: SiteRefActionTarget::ExactTile { tile: tile(3, 4) },
            fitter_cat_id: Some(entity("cat:fitter")),
        },
        LeaderAiActionPayload::RepairProsthetic {
            prosthetic_id: entity("prosthetic:one"),
            workshop_id: entity("workshop:one"),
            input_reservation_id: entity("reservation:repair"),
        },
        LeaderAiActionPayload::PurchaseResearchWithFavor {
            study_id: entity("study:duration:2"),
            use_preparation: true,
            displayed_price_micro_favor: Some(
                BoundedFavorAmount::new(1_500_000).expect("valid Favor"),
            ),
        },
        LeaderAiActionPayload::PrepareScholarStudy {
            study_id: entity("study:duration:3"),
            scholar_cat_id: entity("cat:scholar"),
        },
        LeaderAiActionPayload::ActivateDivineBoost {
            boost_kind: entity("boost:harvest"),
            duration_hours: 24,
            displayed_price_micro_favor: Some(
                BoundedFavorAmount::new(2_000_000).expect("valid Favor"),
            ),
        },
        LeaderAiActionPayload::ChangeDiplomacy {
            target_colony_id: colony("colony:other"),
            relationship: DiplomacyRelationshipTarget::Friendly,
        },
        LeaderAiActionPayload::ApproveAlliance {
            target_colony_id: colony("colony:other"),
            proposal_id: entity("proposal:alliance"),
        },
        LeaderAiActionPayload::BlockColony {
            target_colony_id: colony("colony:other"),
            public_reason: Some(text("Contact blocked")),
        },
        LeaderAiActionPayload::AcceptTradeContract {
            contract_id: entity("trade:one"),
        },
        LeaderAiActionPayload::RejectTradeContract {
            contract_id: entity("trade:two"),
            reason: TradeRejectionReason::TermsDeclined,
        },
    ]
}

fn all_placement_payloads() -> Vec<PhysicalPlacementActionPayload> {
    let rectangle = || SiteRefActionTarget::AnchoredRect {
        anchor: tile(1, 2),
        width: 3,
        height: 3,
    };
    vec![
        PhysicalPlacementActionPayload::PlanBuilding {
            building_type: BuildingType::Workshop,
            site: rectangle(),
        },
        PhysicalPlacementActionPayload::DesignateFarm {
            site: rectangle(),
            crop: CropKind::Grain,
        },
        PhysicalPlacementActionPayload::DesignateStockpile {
            site: rectangle(),
            accepts: vec![ResourceKind::Food],
        },
        PhysicalPlacementActionPayload::DesignateGatherSpot {
            site: rectangle(),
            resource: ResourceKind::Logs,
        },
        PhysicalPlacementActionPayload::DesignateFishingSpot {
            site: SiteRefActionTarget::ExactTile { tile: tile(4, 5) },
        },
        PhysicalPlacementActionPayload::BuildRoad { route: path() },
        PhysicalPlacementActionPayload::BuildBridge {
            site: SiteRefActionTarget::ExactTile { tile: tile(5, 6) },
        },
        PhysicalPlacementActionPayload::DesignateRail {
            route: path(),
            worker_cat_id: entity("cat:builder"),
        },
        PhysicalPlacementActionPayload::BuildDock {
            endpoints: SiteRefActionTarget::EndpointPair {
                source: tile(1, 2),
                destination: tile(1, 3),
            },
            worker_cat_id: entity("cat:builder"),
        },
        PhysicalPlacementActionPayload::BuildTransportVehicle {
            mode: TransportMode::Rail,
            home: SiteRefActionTarget::ExactTile { tile: tile(1, 2) },
            worker_cat_id: entity("cat:builder"),
        },
        PhysicalPlacementActionPayload::CreateTransportRoute {
            mode: TransportMode::Rail,
            source_stockpile_id: entity("stockpile:source"),
            destination_stockpile_id: entity("stockpile:destination"),
            resource: ResourceKind::Food,
            amount: BoundedTradeAmount::new(10).expect("valid amount"),
            route: path(),
            worker_cat_id: entity("cat:driver"),
            repeat: true,
        },
    ]
}

#[test]
fn every_action_domain_round_trips_through_the_versioned_decoder() {
    for payload in all_domain_payloads() {
        let expected = envelope(payload);
        let encoded = serde_json::to_string(&expected).expect("serialize action");
        let decoded = LeaderAiActionEnvelope::decode_json(&encoded).expect("decode action");
        assert_eq!(decoded, expected);
    }
}

#[test]
fn every_physical_placement_domain_round_trips_under_the_same_envelope() {
    for placement in all_placement_payloads() {
        let expected = envelope(LeaderAiActionPayload::PhysicalPlacement { placement });
        let encoded = serde_json::to_string(&expected).expect("serialize placement");
        let decoded = LeaderAiActionEnvelope::decode_json(&encoded).expect("decode placement");
        assert_eq!(decoded, expected);
    }
}

#[test]
fn incompatible_version_wins_before_nested_action_decode() {
    let mut value = serde_json::to_value(envelope(LeaderAiActionPayload::NudgePlan {
        plan_id: entity("plan:one"),
        nudge: BoundedBasisPointNudge::new(1_500).expect("valid nudge"),
        reason_key: None,
    }))
    .expect("serialize action");
    value["protocolVersion"] = Value::from(999);
    value["payload"]["action"] = Value::String("unknown_future_action".into());

    let error = LeaderAiActionEnvelope::decode_json(
        &serde_json::to_string(&value).expect("serialize malformed action"),
    )
    .expect_err("old client must be rejected before payload");
    assert_eq!(error, ActionDecodeError::UnsupportedProtocolVersion(999));
}

#[test]
fn unknown_variants_fields_ids_and_numeric_bounds_fail_closed() {
    let mut unknown = serde_json::to_value(envelope(LeaderAiActionPayload::NudgePlan {
        plan_id: entity("plan:one"),
        nudge: BoundedBasisPointNudge::new(1_500).expect("valid nudge"),
        reason_key: None,
    }))
    .expect("serialize action");
    unknown["payload"]["action"] = Value::String("unknown_future_action".into());
    let error = LeaderAiActionEnvelope::decode_json(
        &serde_json::to_string(&unknown).expect("serialize unknown action"),
    )
    .expect_err("unknown action must fail");
    assert_eq!(error, ActionDecodeError::UnknownActionVariant);

    let mut extra = serde_json::to_value(envelope(LeaderAiActionPayload::NudgePlan {
        plan_id: entity("plan:one"),
        nudge: BoundedBasisPointNudge::new(-1_500).expect("valid nudge"),
        reason_key: None,
    }))
    .expect("serialize action");
    extra["payload"]
        .as_object_mut()
        .expect("payload object")
        .insert("unboundedNote".into(), Value::String("no".into()));
    assert!(
        LeaderAiActionEnvelope::decode_json(
            &serde_json::to_string(&extra).expect("serialize extra field")
        )
        .is_err()
    );

    assert!(BoundedActionId::new("bad action id").is_err());
    assert!(BoundedPlayerId::new(" player").is_err());
    assert!(BoundedBasisPointNudge::new(1_499).is_err());
    assert!(serde_json::from_value::<BoundedTradeAmount>(json!(0)).is_err());
    assert!(serde_json::from_value::<BoundedFavorAmount>(json!(0)).is_err());
}

#[test]
fn action_specific_expected_versions_and_placement_bounds_are_required() {
    let mut placement = envelope(LeaderAiActionPayload::PhysicalPlacement {
        placement: PhysicalPlacementActionPayload::BuildRoad { route: path() },
    });
    placement.expected_versions.expected_reservation_version = None;
    assert!(cat_protocol::validate_lai25_action_bounds(&placement).is_err());

    let oversized = envelope(LeaderAiActionPayload::PhysicalPlacement {
        placement: PhysicalPlacementActionPayload::PlanBuilding {
            building_type: BuildingType::Workshop,
            site: SiteRefActionTarget::AnchoredRect {
                anchor: tile(1, 2),
                width: 65,
                height: 1,
            },
        },
    });
    assert!(cat_protocol::validate_lai25_action_bounds(&oversized).is_err());

    let mut research = envelope(LeaderAiActionPayload::PurchaseResearchWithFavor {
        study_id: entity("study:one"),
        use_preparation: false,
        displayed_price_micro_favor: None,
    });
    research.expected_versions.expected_research_version = None;
    assert!(cat_protocol::validate_lai25_action_bounds(&research).is_err());
}

fn version_hint() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(1),
        domain_version: Some(2),
        resource_version: Some(3),
        spatial_version: None,
        reservation_version: None,
        research_version: None,
        scholar_version: None,
        boost_version: None,
        diplomacy_version: None,
        trade_version: None,
        prosthetic_version: None,
        care_version: None,
        officer_version: None,
        standing_order_version: None,
    }
}

fn state_hint() -> CurrentStateHint {
    CurrentStateHint {
        state_code: text("refresh_required"),
        visible_entity_id: Some(entity("plan:one")),
        visible_stage: Some(text("visible")),
    }
}

#[test]
fn typed_conflicts_and_replays_round_trip_without_private_facts() {
    let replay = ActionReplayResult {
        original_accepted: true,
        result_code: text("accepted"),
        committed_versions: Some(version_hint()),
        current_state_hint: Some(state_hint()),
    };
    let conflicts = vec![
        ActionConflict::update_required(),
        ActionConflict::Unauthorized,
        ActionConflict::OwnershipDenied,
        ActionConflict::AuthorityDenied {
            reason_class: AuthorityDenialReason::PlayerOnly,
        },
        ActionConflict::VersionMismatch {
            current_version_hint: version_hint(),
            current_state_hint: state_hint(),
        },
        ActionConflict::DuplicateReplay { replay },
        ActionConflict::PreconditionFailed {
            reason: text("reported precondition changed"),
        },
        ActionConflict::InsufficientFavor {
            current_state_hint: state_hint(),
        },
        ActionConflict::ReservationConflict {
            current_state_hint: state_hint(),
        },
        ActionConflict::MalformedActionId,
        ActionConflict::UnknownActionVariant,
        ActionConflict::MalformedPayload,
        ActionConflict::RateLimited {
            retry_after_ms: 1_000,
        },
        ActionConflict::LeaderCannotActivateBoost,
        ActionConflict::OfficerCannotActivateBoost,
    ];

    for conflict in conflicts {
        let encoded = serde_json::to_string(&conflict).expect("serialize conflict");
        let decoded: ActionConflict = serde_json::from_str(&encoded).expect("decode conflict");
        assert_eq!(decoded, conflict);
        for forbidden in [
            "hidden",
            "exactStock",
            "regeneration",
            "reservationLoser",
            "rejectedAmount",
        ] {
            assert!(!encoded.contains(forbidden));
        }
    }

    let responses = vec![
        LeaderAiActionResponse {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: action_id("action:accepted"),
            colony_id: colony("colony:home"),
            result: LeaderAiActionResult::Accepted {
                accepted: ActionAcceptedResult {
                    result_code: text("accepted"),
                    changed_ids: vec![entity("plan:one")],
                    committed_versions: version_hint(),
                    current_state_hint: Some(state_hint()),
                },
            },
            refresh: None,
        },
        LeaderAiActionResponse {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: action_id("action:rejected"),
            colony_id: colony("colony:home"),
            result: LeaderAiActionResult::Rejected {
                conflict: ActionConflict::VersionMismatch {
                    current_version_hint: version_hint(),
                    current_state_hint: state_hint(),
                },
            },
            refresh: Some(StaleClientRefresh {
                current_versions: version_hint(),
                current_state_hint: state_hint(),
            }),
        },
        LeaderAiActionResponse {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: action_id("action:replayed"),
            colony_id: colony("colony:home"),
            result: LeaderAiActionResult::DuplicateReplay {
                replay: ActionReplayResult {
                    original_accepted: true,
                    result_code: text("accepted"),
                    committed_versions: Some(version_hint()),
                    current_state_hint: Some(state_hint()),
                },
            },
            refresh: None,
        },
    ];
    for response in responses {
        let encoded = serde_json::to_string(&response).expect("serialize action response");
        let decoded: LeaderAiActionResponse =
            serde_json::from_str(&encoded).expect("decode action response");
        assert_eq!(decoded, response);
    }
}

#[test]
fn validation_order_and_player_only_boost_boundary_are_explicit() {
    assert_eq!(
        ActionValidationPipeline::ordered_steps(),
        [
            ActionValidationStep::ProtocolCompatibility,
            ActionValidationStep::Authentication,
            ActionValidationStep::ColonyOwnership,
            ActionValidationStep::ActionAuthority,
            ActionValidationStep::ExpectedVersions,
            ActionValidationStep::DuplicateReplay,
            ActionValidationStep::CurrentPreconditions,
            ActionValidationStep::FavorOrReservationCommit,
        ]
    );

    let boost = LeaderAiActionPayload::ActivateDivineBoost {
        boost_kind: entity("boost:harvest"),
        duration_hours: 24,
        displayed_price_micro_favor: None,
    };
    assert_eq!(
        boost.authority_class(),
        ActionAuthorityClass::PlayerOnly(PlayerOnlyAction::ActivateDivineBoost)
    );
}
