use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    DivineBoostKind, ProgressionAction, ProgressionActionBuildError,
    ProgressionAuthenticatedPlayerIdentity, ProgressionExpectedBoostVersion,
    ProgressionExpectedDiplomacyVersion, ProgressionExpectedPlannerVersion,
    ProgressionExpectedResearchVersion, ProgressionExpectedResourceVersion,
    ProgressionExpectedScholarVersion, ProgressionExpectedTradeVersion,
    ProgressionExpectedVersionBundle, ProgressionPanelInput, ProgressionPanelPlugin,
    ProgressionPanelProjectionResource, ProgressionPanelState, ProgressionRefreshState,
    ProgressionStableIdempotencyId, ProgressionStaleRefreshHandler, ProgressionTab, RoleColor,
    render_progression_panel,
};
use cat_protocol::{
    ActionConflict, ActionIdempotencyId, ActionProtocolVersion, AutomaticResearchQuotaSnapshot,
    BeliefReportSnapshot, BoundedAgeMs, BoundedBasisPoints, ConsentSnapshot, CurrentStateHint,
    CurrentVersionHint, DiplomacyRelationshipTarget, DiplomacySnapshot, DivineBoostSnapshot,
    FavorEventSnapshot, FavorLedgerSnapshot, InsightSnapshot, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, MANIFEST_STUDY_COUNT, NonEmptyStableId,
    OfferingPackageSnapshot, OfferingStageSnapshot, PlanQueueSnapshot, RegenerationReportSnapshot,
    RelationshipSnapshot, ReportEstimateSnapshot, ReportProvenanceSnapshot, ReportSafeString,
    ResearchFrontierSnapshot, ResearchStudySnapshot, ReservationSummarySnapshot,
    ScholarPreparationSnapshot, ShrineOfferingPipelineSnapshot, ShrineSnapshot,
    SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot, SiteVisibilitySnapshot,
    SnapshotProtocolVersion, SnapshotTilePoint as TilePoint, SnapshotVillageCapabilities,
    StaleClientRefresh, TradeCargoSnapshot, TradeContractSnapshot, TradeEscrowSnapshot,
    TradeRejectionReason, TradeRouteSnapshot, TradeStageSnapshot,
};

fn id(value: &str) -> NonEmptyStableId {
    NonEmptyStableId::new(value).expect("valid stable id")
}

fn text(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("valid report-safe text")
}

fn bp(value: u16) -> BoundedBasisPoints {
    BoundedBasisPoints::new(value).expect("valid basis points")
}

fn tile(x: i32, y: i32) -> TilePoint {
    TilePoint { x, y }
}

fn estimate(minimum: i64, maximum: i64, unit: &str) -> ReportEstimateSnapshot {
    ReportEstimateSnapshot {
        minimum,
        maximum,
        unit: text(unit),
    }
}

fn provenance(report_id: &str) -> ReportProvenanceSnapshot {
    ReportProvenanceSnapshot {
        source_report_ids: vec![id(report_id)],
        observer_id: Some(id("cat:accountant")),
        method: text("reported"),
    }
}

fn report(report_id: &str, level: u8) -> BeliefReportSnapshot {
    BeliefReportSnapshot {
        report_id: id(report_id),
        report_version: 1,
        subject_id: id("subject:progression"),
        domain: text("progression"),
        estimate: estimate(4, 7, "reported units"),
        confidence_basis_points: bp(7_000),
        age_ms: BoundedAgeMs::new(1_000),
        observed_at_ms: 10_000,
        expires_at_ms: 20_000,
        report_level: level,
        provenance: provenance(report_id),
        contradicts_report_ids: Vec::new(),
        replaces_report_id: None,
        unavailable_reason: None,
        regeneration: if level < 4 {
            RegenerationReportSnapshot::UnavailableBelowLevel4
        } else {
            RegenerationReportSnapshot::Estimated {
                level_4_or_higher: true,
                estimate: estimate(60_000, 90_000, "milliseconds"),
                provenance: provenance(report_id),
            }
        },
    }
}

fn site(site_id: &str) -> SiteSnapshot {
    SiteSnapshot {
        site_id: id(site_id),
        visibility: SiteVisibilitySnapshot::Visible,
        lifecycle_stage: SiteLifecycleStageSnapshot::Active,
        blocked_reason: None,
    }
}

fn shrine_endpoint() -> SiteRefSnapshot {
    SiteRefSnapshot::Shrine {
        site: site("site:shrine"),
        shrine_id: id("shrine:one"),
        endpoint: tile(0, 0),
    }
}

fn shrine() -> ShrineSnapshot {
    ShrineSnapshot {
        shrine_id: id("shrine:one"),
        endpoint: shrine_endpoint(),
        pipeline: Some(ShrineOfferingPipelineSnapshot {
            offering_id: id("offering:active"),
            package: OfferingPackageSnapshot {
                package_id: id("offering:5_herbs"),
                package_kind: text("5 Herbs"),
                cargo_ids: vec![id("cargo:herbs:one"), id("cargo:herbs:two")],
                favor_reward_micro_favor: 1_000_000,
            },
            stage: OfferingStageSnapshot::Hauling {
                carrier_cat_id: id("cat:hauler"),
            },
            source_report_ids: vec![id("report:good")],
            shrine_endpoint: shrine_endpoint(),
            cargo_disposition: text("delivered_to_shrine_pending"),
            rationale: text("lowest replacement cost from report"),
            blocked_reason: None,
        }),
    }
}

fn favor() -> FavorLedgerSnapshot {
    FavorLedgerSnapshot {
        ledger_version: 9,
        micro_favor: 3_000_000,
        favor_events: vec![
            FavorEventSnapshot {
                event_id: id("favor:credit:offering"),
                delta_micro_favor: 1_000_000,
                resulting_micro_favor: 4_000_000,
                occurred_at_ms: 11_000,
                reason: text("offering credited once"),
            },
            FavorEventSnapshot {
                event_id: id("favor:debit:research"),
                delta_micro_favor: -1_000_000,
                resulting_micro_favor: 3_000_000,
                occurred_at_ms: 12_000,
                reason: text("research purchase"),
            },
        ],
    }
}

fn research() -> ResearchFrontierSnapshot {
    ResearchFrontierSnapshot {
        research_version: 17,
        manifest_study_count: MANIFEST_STUDY_COUNT,
        owned_study_ids: vec![
            id("divine_duration_stage_03"),
            id("divine_economy_stage_02"),
        ],
        frontier: vec![ResearchStudySnapshot {
            study_id: id("research:rehabilitation:next"),
            display_name: text("Rehabilitation IV"),
            prerequisite_ids: vec![id("rehabilitation_stage_03")],
            price_micro_favor: 2_000_000,
            prepared_price_micro_favor: Some(1_500_000),
        }],
        automatic_quota: AutomaticResearchQuotaSnapshot {
            quota_used: 1,
            quota_limit: 3,
            quota_window_started_at_ms: 1_000,
        },
        insight: InsightSnapshot {
            insight_balance: 200,
            generated_this_week: 40,
            week_started_at_ms: Some(1_000),
        },
        preparations: vec![ScholarPreparationSnapshot {
            preparation_id: id("prep:rehab"),
            study_id: id("research:rehabilitation:next"),
            scholar_cat_id: Some(id("cat:scholar")),
            progress_basis_points: bp(7_500),
            committed_insight_cost: 2_000_000,
            player_discount_basis_points: bp(2_500),
            prepared: false,
        }],
    }
}

fn boosts() -> Vec<DivineBoostSnapshot> {
    vec![DivineBoostSnapshot {
        boost_id: id("boost:fleet:active"),
        boost_kind: text("fleet_paws"),
        effect_basis_points: bp(10_000),
        boost_price_micro_favor: 2_000_000,
        duration_ms: 7_200_000,
        boost_started_at_ms: 20_000,
        boost_expires_at_ms: 7_220_000,
        effect_stage: 2,
    }]
}

fn diplomacy() -> DiplomacySnapshot {
    DiplomacySnapshot {
        diplomacy_version: 22,
        relationships: vec![RelationshipSnapshot {
            relationship_id: id("relationship:pine"),
            other_colony_id: id("colony:pine"),
            relationship_version: 5,
            state: text("Friendly"),
            consent: ConsentSnapshot {
                local_approved: true,
                remote_approved: false,
                consent_version: 7,
            },
            updated_at_ms: 30_000,
        }],
    }
}

fn trade() -> Vec<TradeContractSnapshot> {
    vec![TradeContractSnapshot {
        contract_id: id("trade:pine:herbs"),
        contract_version: 31,
        partner_colony_id: id("colony:pine"),
        stage: TradeStageSnapshot::AwaitingConsent,
        actor_cat_ids: vec![id("cat:hauler")],
        valuation_report_ids: vec![id("report:good")],
        valuation_confidence_basis_points: bp(8_000),
        escrow: TradeEscrowSnapshot {
            escrow_id: id("escrow:pine:herbs"),
            cargo_ids: vec![id("cargo:herbs:one")],
            released: false,
        },
        route: TradeRouteSnapshot {
            route_id: id("route:pine"),
            ordered_tiles: vec![tile(0, 0), tile(1, 0), tile(2, 0)],
            endpoint: SiteRefSnapshot::TradeEndpoint {
                site: site("site:trade-pine"),
                contract_id: id("trade:pine:herbs"),
                colony_id: id("colony:pine"),
                endpoint: tile(2, 0),
            },
        },
        cargo: vec![TradeCargoSnapshot {
            cargo_id: id("cargo:herbs:one"),
            cargo_kind: text("herbs"),
            quantity: 5,
            state: text("escrowed"),
        }],
        next_event_at_ms: Some(40_000),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:trade")],
            reservation_version: 44,
        },
        bounded_failure: Some(text("route blocked")),
        recovery_state: Some(text("awaiting consent")),
    }]
}

fn colony() -> cat_protocol::ColonyAiSnapshot {
    cat_protocol::ColonyAiSnapshot {
        colony_id: id("colony:one"),
        state_version: 12,
        action_versions: Default::default(),
        capabilities: SnapshotVillageCapabilities {
            can_view: true,
            can_control: true,
            is_owner: true,
        },
        reports: vec![report("report:poor", 3), report("report:good", 4)],
        plans: PlanQueueSnapshot {
            planner_version: 77,
            planning_epoch: 76,
            plans: Vec::new(),
        },
        officer_requests: Vec::new(),
        officer_institution: None,
        standing_orders: Vec::new(),
        refresh_hints: Vec::new(),
        visible_tasks: Vec::new(),
        cats: Vec::new(),
        shrine: shrine(),
        favor: favor(),
        research: research(),
        boosts: boosts(),
        diplomacy: diplomacy(),
        trade: trade(),
    }
}

fn identity() -> ProgressionAuthenticatedPlayerIdentity {
    ProgressionAuthenticatedPlayerIdentity {
        colony_id: "colony:one".to_string(),
        player_id: "player:one".to_string(),
    }
}

fn versions() -> ProgressionExpectedVersionBundle {
    ProgressionExpectedVersionBundle {
        planner: ProgressionExpectedPlannerVersion(77),
        resource: ProgressionExpectedResourceVersion(9),
        research: Some(ProgressionExpectedResearchVersion(17)),
        scholar: Some(ProgressionExpectedScholarVersion(18)),
        boost: Some(ProgressionExpectedBoostVersion(19)),
        diplomacy: Some(ProgressionExpectedDiplomacyVersion(22)),
        trade: Some(ProgressionExpectedTradeVersion(31)),
        reservation: Some(44),
    }
}

fn current_versions() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(77),
        domain_version: Some(0),
        resource_version: Some(10),
        spatial_version: None,
        reservation_version: Some(44),
        research_version: Some(17),
        scholar_version: Some(18),
        boost_version: Some(20),
        diplomacy_version: Some(23),
        trade_version: Some(32),
        prosthetic_version: None,
        care_version: None,
        officer_version: None,
        standing_order_version: None,
    }
}

#[test]
fn shrine_and_favor_projection_stays_endless_physical_and_exact() {
    let projection =
        render_progression_panel(&colony(), "colony:one", 4, ProgressionRefreshState::Current)
            .expect("selected colony");
    let row = projection.shrine.active_pipeline.as_ref().unwrap();

    assert_eq!(projection.shrine.package_catalog.len(), 4);
    assert!(
        projection
            .shrine
            .package_catalog
            .iter()
            .all(|package| package.favor_reward_micro_favor == 1_000_000)
    );
    assert_eq!(row.package.package_kind, "5 Herbs");
    assert_eq!(row.report_provenance.0, ["report:good"]);
    assert_eq!(row.rationale.0, "lowest replacement cost from report");
    assert_eq!(row.haul_stage.0, "hauling by cat:hauler");
    assert_eq!(row.ritual_stage.0, "not started");
    assert_eq!(row.pinned_endpoint.0, "site:shrine");
    assert_eq!(projection.visible_row_ids[0], "offering:active");

    assert_eq!(projection.favor.version.0, 9);
    assert_eq!(projection.favor.exact_balance.0, 3_000_000);
    assert_eq!(projection.favor.events.0.len(), 2);
    assert_eq!(projection.favor.events.0[1].delta_micro_favor, -1_000_000);
    assert_eq!(projection.layout.panel_width_px, 460);
    assert_eq!(projection.layout.compact_panel_width_px, 336);
    assert_eq!(projection.layout.panel_radius_px, 10);
    assert!(projection.layout.world_first);
    assert_eq!(projection.chrome.paper_role, RoleColor::Paper);
    assert_eq!(projection.chrome.border_role, RoleColor::Wood);
    assert_eq!(projection.chrome.favor_role, RoleColor::Rust);
    assert_eq!(projection.chrome.boost_role, RoleColor::Olive);
    assert_eq!(projection.chrome.danger_role, RoleColor::Danger);
    assert!(!format!("{projection:?}").contains("hidden stock"));
}

#[test]
fn research_scholar_and_boost_projection_uses_manifest_and_committed_snapshot_state() {
    let projection =
        render_progression_panel(&colony(), "colony:one", 4, ProgressionRefreshState::Current)
            .expect("selected colony");

    assert_eq!(projection.research.manifest_count.0, MANIFEST_STUDY_COUNT);
    assert_eq!(
        projection.research.frontier.0[0].study_id,
        "research:rehabilitation:next"
    );
    assert_eq!(
        projection.research.frontier.0[0]
            .committed_price
            .prepared_micro_favor,
        Some(1_500_000)
    );
    assert_eq!(projection.research.quota_used_limit.used, 1);
    assert_eq!(projection.research.quota_used_limit.limit, 3);
    assert_eq!(projection.research.insight.balance, 200);
    assert_eq!(
        projection.research.scholar_preparation.preparations[0].player_discount_basis_points,
        2_500
    );
    assert_eq!(projection.research.scholar_tracks.len(), 4);
    assert_eq!(projection.research.scholar_tracks[0].owned_stage, 3);
    assert_eq!(projection.research.scholar_tracks[1].owned_stage, 2);

    let boost_controls = &projection.boosts.controls;
    assert_eq!(boost_controls.len(), 4);
    let fleet = boost_controls
        .iter()
        .find(|control| control.kind == DivineBoostKind::FleetPaws)
        .unwrap();
    assert_eq!(fleet.duration_picker.unlocked_hours, [1, 2, 3, 4]);
    assert_eq!(fleet.cost.0, 3_760_000);
    assert!(fleet.same_type_disabled.is_some());
    assert_eq!(
        fleet.active.as_ref().unwrap().paid_cost_micro_favor,
        2_000_000
    );
    let labor = boost_controls
        .iter()
        .find(|control| control.kind == DivineBoostKind::BountifulLabor)
        .unwrap();
    assert_eq!(labor.cost.0, 7_520_000);
    assert!(labor.same_type_disabled.is_none());
}

#[test]
fn diplomacy_and_trade_projection_preserves_consent_escrow_route_cargo_and_recovery() {
    let projection =
        render_progression_panel(&colony(), "colony:one", 4, ProgressionRefreshState::Current)
            .expect("selected colony");

    let relationship = &projection.diplomacy.relationships[0];
    assert_eq!(projection.diplomacy.expected_version.0, 22);
    assert_eq!(relationship.other_colony_id, "colony:pine");
    assert!(relationship.consent.local_approved);
    assert!(!relationship.consent.remote_approved);
    assert!(relationship.alliance_approval.requires_remote_consent);

    let trade = &projection.trade.rows[0];
    assert_eq!(trade.contract_id, "trade:pine:herbs");
    assert_eq!(trade.value_reports.0, ["report:good"]);
    assert_eq!(trade.valuation_confidence.0, 8_000);
    assert_eq!(trade.escrow.cargo_ids, ["cargo:herbs:one"]);
    assert_eq!(trade.route_endpoint.endpoint_site_id, "site:trade-pine");
    assert_eq!(trade.route_endpoint.route_tiles, 3);
    assert_eq!(trade.cargo[0].state, "escrowed");
    assert_eq!(trade.recovery.stage, "awaiting consent");
    assert_eq!(
        trade.route_block_feedback.as_ref().unwrap().0,
        "route blocked"
    );
    assert!(!trade.accept.disabled);
    assert!(!trade.reject.disabled);
}

#[test]
fn progression_actions_use_real_lai25_payload_bounds_and_expected_versions() {
    let purchase = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:research".to_string()),
        versions(),
        ProgressionAction::PurchaseResearch {
            study_id: "research:rehabilitation:next".to_string(),
            use_preparation: true,
            displayed_price_micro_favor: Some(1_500_000),
        },
    )
    .expect("valid research action");
    assert_eq!(
        purchase.protocol_version.get(),
        SnapshotProtocolVersion::current().get()
    );
    assert_eq!(purchase.expected_versions.expected_resource_version, 9);
    assert_eq!(
        purchase.expected_versions.expected_research_version,
        Some(17)
    );
    assert!(matches!(
        purchase.payload,
        LeaderAiActionPayload::PurchaseResearchWithFavor { .. }
    ));

    let boost = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:boost".to_string()),
        versions(),
        ProgressionAction::ActivateDivineBoost {
            boost_kind: DivineBoostKind::BountifulLabor,
            duration_hours: 4,
            displayed_price_micro_favor: Some(7_520_000),
        },
    )
    .expect("valid boost action");
    assert_eq!(boost.expected_versions.expected_boost_version, Some(19));
    assert!(matches!(
        boost.payload,
        LeaderAiActionPayload::ActivateDivineBoost { .. }
    ));

    let missing_boost = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:boost-missing".to_string()),
        ProgressionExpectedVersionBundle {
            boost: None,
            ..versions()
        },
        ProgressionAction::ActivateDivineBoost {
            boost_kind: DivineBoostKind::FleetPaws,
            duration_hours: 1,
            displayed_price_micro_favor: Some(1_000_000),
        },
    );
    assert_eq!(
        missing_boost,
        Err(ProgressionActionBuildError::MissingVersion(
            "research_boost"
        ))
    );

    let prepare = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:prepare".to_string()),
        versions(),
        ProgressionAction::PrepareScholarStudy {
            study_id: "research:rehabilitation:next".to_string(),
            scholar_cat_id: "cat:scholar".to_string(),
        },
    )
    .expect("valid scholar action");
    assert_eq!(prepare.expected_versions.expected_scholar_version, Some(18));

    let diplomacy = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:diplomacy".to_string()),
        versions(),
        ProgressionAction::ChangeDiplomacy {
            target_colony_id: "colony:pine".to_string(),
            relationship: DiplomacyRelationshipTarget::Friendly,
        },
    )
    .expect("valid diplomacy action");
    assert_eq!(
        diplomacy.expected_versions.expected_diplomacy_version,
        Some(22)
    );

    let trade = cat_client::leader_ai_ui::build_progression_action_envelope(
        identity(),
        ProgressionStableIdempotencyId("idem:progression:trade".to_string()),
        versions(),
        ProgressionAction::RejectTradeContract {
            contract_id: "trade:pine:herbs".to_string(),
            reason: TradeRejectionReason::TermsDeclined,
        },
    )
    .expect("valid trade action");
    assert_eq!(trade.expected_versions.expected_trade_version, Some(31));
    assert!(matches!(
        trade.payload,
        LeaderAiActionPayload::RejectTradeContract { .. }
    ));
}

#[test]
fn stale_refresh_keeps_visible_selection_and_uses_bounded_feedback() {
    let response = LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("idem:progression:stale").unwrap(),
        colony_id: cat_protocol::SelectedColonyId::new("colony:one").unwrap(),
        result: LeaderAiActionResult::Rejected {
            conflict: ActionConflict::VersionMismatch {
                current_version_hint: current_versions(),
                current_state_hint: CurrentStateHint {
                    state_code: text("Favor ledger changed"),
                    visible_entity_id: None,
                    visible_stage: Some(text("refresh")),
                },
            },
        },
        refresh: Some(StaleClientRefresh {
            current_versions: current_versions(),
            current_state_hint: CurrentStateHint {
                state_code: text("Favor ledger changed"),
                visible_entity_id: Some(
                    cat_protocol::BoundedEntityId::new("research:rehabilitation:next").unwrap(),
                ),
                visible_stage: Some(text("refresh")),
            },
        }),
    };

    let refresh = ProgressionStaleRefreshHandler::handle(
        &response,
        Some("research:rehabilitation:next"),
        &["research:rehabilitation:next".to_string()],
    )
    .expect("stale refresh");
    assert_eq!(refresh.refresh_state, ProgressionRefreshState::Stale);
    assert_eq!(
        refresh.selected_row.0.as_deref(),
        Some("research:rehabilitation:next")
    );
    assert_eq!(refresh.feedback.message, "Favor ledger changed");

    let removed =
        ProgressionStaleRefreshHandler::handle(&response, Some("research:gone"), &[]).unwrap();
    assert_eq!(removed.selected_row.0, None);
}

#[test]
fn selected_colony_privacy_blocks_projection_and_debug_output_has_no_hidden_truth() {
    assert!(
        render_progression_panel(
            &colony(),
            "colony:other",
            4,
            ProgressionRefreshState::Current
        )
        .is_none()
    );

    let projection =
        render_progression_panel(&colony(), "colony:one", 4, ProgressionRefreshState::Current)
            .unwrap();
    let debug = format!("{projection:?}");
    assert!(!debug.contains("hidden stock"));
    assert!(!debug.contains("hidden regeneration"));
    assert!(!debug.contains("private colony"));
}

#[test]
fn progression_panel_plugin_updates_projection_resource_and_preserves_visible_row_selection() {
    let mut app = App::new();
    app.add_plugins(ProgressionPanelPlugin);
    app.insert_resource(ProgressionPanelState {
        selected_row_id: Some("trade:pine:herbs".to_string()),
        selected_tab: ProgressionTab::Trade,
        refresh_state: ProgressionRefreshState::Stale,
    });
    app.insert_resource(ProgressionPanelInput {
        selected_colony_id: Some("colony:one".to_string()),
        selected_duration_hours: 4,
        colony: Some(colony()),
    });

    app.update();

    let projection = app
        .world()
        .resource::<ProgressionPanelProjectionResource>()
        .projection
        .as_ref()
        .expect("progression projection");
    assert_eq!(projection.colony_id, "colony:one");
    assert_eq!(projection.selected_tab, ProgressionTab::Trade);
    assert_eq!(
        projection.selected_row_id.as_deref(),
        Some("trade:pine:herbs")
    );
    assert_eq!(projection.refresh_state, ProgressionRefreshState::Stale);
    assert!(
        projection
            .visible_row_ids
            .iter()
            .any(|row_id| row_id == "research:rehabilitation:next")
    );
    assert!(
        projection
            .visible_row_ids
            .iter()
            .any(|row_id| row_id == "fleet_paws")
    );

    app.insert_resource(ProgressionPanelInput {
        selected_colony_id: Some("colony:other".to_string()),
        selected_duration_hours: 4,
        colony: Some(colony()),
    });
    app.update();

    assert!(
        app.world()
            .resource::<ProgressionPanelProjectionResource>()
            .projection
            .is_none()
    );
}
