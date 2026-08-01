use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    AdministrationSlotMeter, AuthenticatedPlayerIdentity, ExpectedDomainVersion,
    ExpectedPlannerVersion, ExpectedReservationVersion, ExpectedResourceVersion,
    ExpectedVersionBundle, LeaderAiPlanNudgeAction, LeaderAiStandingOrderAction, PlanUrgency,
    PlansPanelInput, PlansPanelPlugin, PlansPanelProjectionResource, PlansRefreshState,
    ReportSafeUnavailableState, RoleColor, StableIdempotencyId, StandingOrderDraft,
    StandingOrderDraftPatch, StandingOrdersPanel, VersionMismatchRefreshHandler,
    build_standing_order_action_envelope, render_authoritative_top_eight_plans,
    send_expected_version_action,
};
use cat_protocol::{
    ActionConflict, ActionIdempotencyId, ActionProtocolVersion, AnatomySnapshot,
    AutomaticResearchQuotaSnapshot, BeliefReportSnapshot, BoundedAgeMs, BoundedBasisPointNudge,
    BoundedBasisPoints, CareStatusSnapshot, CatCareSnapshot, CatPersonalitySnapshot,
    CatTraitsSnapshot, ColonyAiSnapshot, CurrentStateHint, CurrentVersionHint, DiplomacySnapshot,
    DismissalReason, FavorLedgerSnapshot, InsightSnapshot, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, MANIFEST_STUDY_COUNT, NonEmptyStableId,
    OfficerRequestSnapshot, PlanQueueSnapshot, PlanReasonSnapshot, PlanSnapshot,
    RegenerationReportSnapshot, ReportEstimateSnapshot, ReportProvenanceSnapshot, ReportSafeString,
    ResearchFrontierSnapshot, ReservationSummarySnapshot, ShrineSnapshot,
    SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot, SiteVisibilitySnapshot,
    SnapshotProtocolVersion, SnapshotTilePoint as TilePoint, SnapshotVillageCapabilities,
    StressSnapshot, TaskCargoSnapshot, VisibleTaskSnapshot, WillingnessSnapshot, WorkSlotSnapshot,
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
        observer_id: Some(id("cat:observer")),
        method: text("reported"),
    }
}

fn report(report_id: &str, level: u8, age_ms: u64) -> BeliefReportSnapshot {
    BeliefReportSnapshot {
        report_id: id(report_id),
        report_version: 1,
        subject_id: id("subject:wood"),
        domain: text("wood"),
        estimate: estimate(5, 8, "logs"),
        confidence_basis_points: bp(6_000),
        age_ms: BoundedAgeMs::new(age_ms),
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

fn plan(index: usize, score_bucket: i16, report_id: &str) -> PlanSnapshot {
    PlanSnapshot {
        plan_id: id(&format!("plan:p{index}")),
        intent_id: id(&format!("intent:p{index}")),
        lifecycle_state: text("active"),
        responsible_actor_id: id("cat:leader"),
        responsible_office: Some(text("steward")),
        dependency_intent_ids: vec![id("intent:dependency")],
        score_bucket,
        rationale: text(&format!("reported plan {index}")),
        expected_cost: estimate(10, 20, "work"),
        expected_benefit: estimate(30, 50, "benefit"),
        reasons: vec![PlanReasonSnapshot {
            code: id("reason:reported"),
            summary: text("report supported"),
            confidence_basis_points: bp(7_000),
            source_report_ids: vec![id(report_id)],
        }],
    }
}

fn site(site_id: &str) -> SiteSnapshot {
    SiteSnapshot {
        site_id: id(site_id),
        visibility: SiteVisibilitySnapshot::Visible,
        lifecycle_stage: SiteLifecycleStageSnapshot::Active,
        blocked_reason: Some(text("waiting for planks")),
    }
}

fn visible_task(intent: &str) -> VisibleTaskSnapshot {
    VisibleTaskSnapshot {
        task_id: id("task:workshop"),
        intent_id: id(intent),
        category: text("Build Workshop"),
        stage: text("hauling"),
        assigned_cat_ids: vec![id("cat:mallow")],
        objective: SiteRefSnapshot::Tile {
            site: site("site:workshop"),
            tile: tile(4, 9),
        },
        work_slots: vec![WorkSlotSnapshot {
            slot_id: id("slot:work"),
            tile: tile(4, 9),
            state: text("assigned"),
        }],
        endpoint: None,
        footprint: Vec::new(),
        progress_basis_points: bp(2_500),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:workshop")],
            reservation_version: 44,
        },
        blocked_reason: Some(text("waiting for planks")),
        cargo: TaskCargoSnapshot {
            cargo_ids: Vec::new(),
            summary: text("none"),
        },
        last_updated_at_ms: 15_000,
    }
}

fn cat(active_task_id: Option<&str>) -> CatCareSnapshot {
    CatCareSnapshot {
        cat_id: id("cat:mallow"),
        display_name: text("Mallow"),
        active_task_id: active_task_id.map(id),
        traits: CatTraitsSnapshot {
            innate_attributes: Vec::new(),
            learned_skills: Vec::new(),
            office_experience: Vec::new(),
            acquired_traits: Vec::new(),
        },
        personality: CatPersonalitySnapshot {
            sociability: bp(5_000),
            diligence: bp(5_000),
            courage: bp(5_000),
            empathy: bp(5_000),
            curiosity: bp(5_000),
        },
        stress: StressSnapshot {
            stress_basis_points: bp(0),
            recovery_basis_points: bp(0),
            refusing: false,
            refusal_reason: None,
        },
        willingness: WillingnessSnapshot {
            total_basis_points: bp(8_000),
            factors: Vec::new(),
            eligible: true,
            eligibility_reason: None,
        },
        anatomy: AnatomySnapshot {
            body_parts: Vec::new(),
        },
        prosthetics: Vec::new(),
        care: CareStatusSnapshot {
            care_site: None,
            treatment_task_id: None,
            fitting_task_id: None,
            repair_task_id: None,
            status: text("healthy"),
        },
    }
}

fn colony() -> ColonyAiSnapshot {
    ColonyAiSnapshot {
        colony_id: id("colony:one"),
        state_version: 12,
        action_versions: Default::default(),
        capabilities: SnapshotVillageCapabilities {
            can_view: true,
            can_control: true,
            is_owner: true,
        },
        reports: vec![
            report("report:low", 3, 5_000),
            report("report:high", 4, 1_000),
        ],
        plans: PlanQueueSnapshot {
            planner_version: 77,
            planning_epoch: 76,
            plans: (0..10)
                .map(|index| {
                    plan(
                        index,
                        if index == 0 {
                            22
                        } else {
                            18 - i16::try_from(index).unwrap()
                        },
                        if index == 0 {
                            "report:low"
                        } else {
                            "report:high"
                        },
                    )
                })
                .collect(),
        },
        officer_requests: vec![
            OfficerRequestSnapshot {
                request_id: id("request:steward"),
                request_version: 3,
                office: text("steward"),
                domain: text("construction"),
                requested_action: text("assign workshop crew"),
                budget: estimate(1, 2, "cats"),
                priority_basis_points: bp(7_500),
                source_report_ids: vec![id("report:high")],
                expires_at_ms: 30_000,
                merged_into_request_id: None,
                supersedes_request_ids: Vec::new(),
                blocked_reason: None,
            },
            OfficerRequestSnapshot {
                request_id: id("request:merged"),
                request_version: 4,
                office: text("scout"),
                domain: text("exploration"),
                requested_action: text("merged request"),
                budget: estimate(1, 1, "cats"),
                priority_basis_points: bp(5_000),
                source_report_ids: vec![id("report:high")],
                expires_at_ms: 30_000,
                merged_into_request_id: Some(id("request:steward")),
                supersedes_request_ids: Vec::new(),
                blocked_reason: None,
            },
        ],
        officer_institution: None,
        standing_orders: Vec::new(),
        refresh_hints: Vec::new(),
        visible_tasks: vec![visible_task("intent:p0")],
        cats: vec![cat(Some("task:workshop"))],
        shrine: ShrineSnapshot {
            shrine_id: id("shrine:one"),
            endpoint: SiteRefSnapshot::Tile {
                site: site("site:shrine"),
                tile: tile(0, 0),
            },
            pipeline: None,
        },
        favor: FavorLedgerSnapshot {
            ledger_version: 1,
            micro_favor: 0,
            favor_events: Vec::new(),
        },
        research: ResearchFrontierSnapshot {
            research_version: 1,
            manifest_study_count: MANIFEST_STUDY_COUNT,
            owned_study_ids: Vec::new(),
            frontier: Vec::new(),
            automatic_quota: AutomaticResearchQuotaSnapshot {
                quota_used: 0,
                quota_limit: 1,
                quota_window_started_at_ms: 0,
            },
            insight: InsightSnapshot {
                insight_balance: 0,
                generated_this_week: 0,
                week_started_at_ms: Some(0),
            },
            preparations: Vec::new(),
        },
        boosts: Vec::new(),
        diplomacy: DiplomacySnapshot {
            diplomacy_version: 1,
            relationships: Vec::new(),
        },
        trade: Vec::new(),
    }
}

fn identity() -> AuthenticatedPlayerIdentity {
    AuthenticatedPlayerIdentity {
        colony_id: "colony:one".to_string(),
        player_id: "player:one".to_string(),
    }
}

fn versions() -> ExpectedVersionBundle {
    ExpectedVersionBundle {
        planner: ExpectedPlannerVersion(77),
        domain: ExpectedDomainVersion(8),
        resource: ExpectedResourceVersion(9),
        reservation: ExpectedReservationVersion(Some(44)),
        standing_order: Some(6),
    }
}

fn current_versions() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(78),
        domain_version: Some(8),
        resource_version: Some(9),
        spatial_version: None,
        reservation_version: Some(44),
        research_version: None,
        scholar_version: None,
        boost_version: None,
        diplomacy_version: None,
        trade_version: None,
        prosthetic_version: None,
        care_version: None,
        officer_version: None,
        standing_order_version: Some(6),
    }
}

#[test]
fn top_eight_plan_projection_preserves_server_order_and_report_safe_state() {
    let projection = render_authoritative_top_eight_plans(
        &colony(),
        StandingOrdersPanel::empty(3, 1),
        PlansRefreshState::Current,
    );

    assert_eq!(projection.planner_version, 77);
    assert_eq!(projection.rows.len(), 8);
    assert_eq!(projection.rows[0].stable_id.0, "plan:p0");
    assert_eq!(projection.rows[7].stable_id.0, "plan:p7");
    assert_eq!(projection.rows[0].index, 0);
    assert_eq!(projection.rows[0].urgency, PlanUrgency::Emergency);
    assert_eq!(projection.rows[1].urgency, PlanUrgency::High);
    assert_eq!(projection.rows[0].lifecycle.0, "active");
    assert_eq!(projection.rows[0].rationale.0, "reported plan 0");
    assert_eq!(projection.rows[0].cost.0.minimum, 10);
    assert_eq!(projection.rows[0].cost.0.maximum, 20);
    assert_eq!(
        projection.rows[0]
            .block_reason
            .as_ref()
            .map(|reason| reason.0.as_str()),
        Some("waiting for planks")
    );
    assert_eq!(
        projection.rows[0].uncertainty.0,
        "regeneration estimate unavailable until report level 4"
    );
    assert_eq!(
        projection.rows[0]
            .score_confidence_range
            .confidence_min_basis_points,
        Some(7_000)
    );
    assert_eq!(projection.rows[0].report_age.oldest_age_ms, Some(5_000));
    assert_eq!(
        projection.rows[0].unavailable_states,
        vec![ReportSafeUnavailableState::RegenerationUnavailableBelowReportLevel4]
    );
    let objective = projection.rows[0]
        .objective
        .as_ref()
        .expect("objective summary");
    assert_eq!(objective.site_id, "site:workshop");
    assert_eq!(projection.rows[0].assigned_cat_ids, ["cat:mallow"]);
    assert_eq!(projection.rows[0].stage.as_deref(), Some("hauling"));
    assert_eq!(projection.rows[0].progress_basis_points, Some(2_500));
    assert_eq!(projection.layout.rows_limit, 8);
    assert_eq!(projection.layout.panel_radius_px, 10);
    assert!(projection.layout.world_first);
    assert_eq!(projection.chrome.paper_role, RoleColor::Paper);
    assert_eq!(projection.chrome.action_role, RoleColor::Rust);
}

#[test]
fn plans_panel_plugin_updates_projection_resource_without_client_root_growth() {
    let mut app = App::new();
    app.add_plugins(PlansPanelPlugin);
    app.insert_resource(PlansPanelInput {
        selected_colony_id: Some("colony:one".to_string()),
        colony: Some(colony()),
        standing_orders: StandingOrdersPanel::empty(4, 1),
    });

    app.update();

    let projection = app
        .world()
        .resource::<PlansPanelProjectionResource>()
        .projection
        .as_ref()
        .expect("plans projection");
    assert_eq!(projection.rows.len(), 8);
    assert_eq!(projection.standing_orders.slot_meter.slot_limit, 4);
    assert_eq!(
        projection.rows[0].controls.move_up.delta_basis_points,
        1_500
    );
    assert_eq!(
        projection.rows[0].controls.move_down.delta_basis_points,
        -1_500
    );
    assert!(
        projection.rows[0]
            .controls
            .dismiss
            .label
            .as_str()
            .contains("reported plan 0")
    );

    app.insert_resource(PlansPanelInput {
        selected_colony_id: Some("colony:other".to_string()),
        colony: Some(colony()),
        standing_orders: StandingOrdersPanel::empty(4, 1),
    });
    app.update();

    assert!(
        app.world()
            .resource::<PlansPanelProjectionResource>()
            .projection
            .is_none()
    );
}

#[test]
fn visible_panel_states_officer_requests_and_standing_order_slots_are_bounded() {
    let panel = StandingOrdersPanel {
        slot_meter: AdministrationSlotMeter::new(2, 2),
        draft: Some(StandingOrderDraft {
            order_kind: "reserve".to_string(),
            domain: "wood".to_string(),
            target_id: Some("stockpile:main".to_string()),
            instruction: "keep firewood dry".to_string(),
            priority_basis_points: 6_000,
            expires_at_ms: None,
        }),
        feedback: None,
    };
    let projection =
        render_authoritative_top_eight_plans(&colony(), panel, PlansRefreshState::Stale);

    assert_eq!(
        projection.refresh_state.feedback(),
        cat_client::leader_ai_ui::FeedbackState::Stale
    );
    assert!(projection.standing_orders.slot_meter.limit_reached);
    assert_eq!(projection.standing_orders.slot_meter.vacant, 0);
    assert_eq!(projection.officer_reports.len(), 1);
    assert_eq!(projection.officer_reports[0].request_id, "request:steward");
    assert_eq!(projection.officer_reports[0].domain, "construction");
    assert_eq!(projection.officer_reports[0].priority_basis_points, 7_500);
}

#[test]
fn plan_and_standing_order_actions_use_real_lai25_bounds_and_versions() {
    let up = send_expected_version_action(
        identity(),
        StableIdempotencyId("idem:nudge:up".to_string()),
        versions(),
        LeaderAiPlanNudgeAction::MoveUp {
            plan_id: "plan:p0".to_string(),
            reason_key: Some("reason:reported".to_string()),
        },
    )
    .expect("valid nudge action");
    assert_eq!(
        up.protocol_version.get(),
        SnapshotProtocolVersion::current().get()
    );
    assert_eq!(up.expected_versions.expected_planner_version, 77);
    assert_eq!(up.expected_versions.expected_reservation_version, Some(44));
    match up.payload {
        LeaderAiActionPayload::NudgePlan { nudge, .. } => {
            assert_eq!(nudge.get(), 1_500);
        }
        other => panic!("unexpected payload {other:?}"),
    }

    let down = send_expected_version_action(
        identity(),
        StableIdempotencyId("idem:nudge:down".to_string()),
        versions(),
        LeaderAiPlanNudgeAction::MoveDown {
            plan_id: "plan:p0".to_string(),
            reason_key: None,
        },
    )
    .expect("valid down nudge");
    match down.payload {
        LeaderAiActionPayload::NudgePlan { nudge, .. } => assert_eq!(nudge.get(), -1_500),
        other => panic!("unexpected payload {other:?}"),
    }
    assert!(BoundedBasisPointNudge::new(1_499).is_err());

    let dismiss = send_expected_version_action(
        identity(),
        StableIdempotencyId("idem:dismiss:p0".to_string()),
        versions(),
        LeaderAiPlanNudgeAction::Dismiss {
            intent_id: "intent:p0".to_string(),
            planning_epoch: 11,
            reason: DismissalReason::NoLongerDesired,
        },
    )
    .expect("valid dismiss");
    assert!(matches!(
        dismiss.payload,
        LeaderAiActionPayload::DismissIntent {
            planning_epoch: 11,
            reason: DismissalReason::NoLongerDesired,
            ..
        }
    ));

    let create = build_standing_order_action_envelope(
        identity(),
        StableIdempotencyId("idem:standing:create".to_string()),
        versions(),
        LeaderAiStandingOrderAction::Create(StandingOrderDraft {
            order_kind: "reserve".to_string(),
            domain: "wood".to_string(),
            target_id: Some("stockpile:main".to_string()),
            instruction: "keep firewood dry".to_string(),
            priority_basis_points: 6_500,
            expires_at_ms: Some(50_000),
        }),
    )
    .expect("valid standing order create");
    assert_eq!(
        create.expected_versions.expected_standing_order_version,
        Some(6)
    );
    assert!(matches!(
        create.payload,
        LeaderAiActionPayload::CreateStandingOrder { .. }
    ));

    let update = build_standing_order_action_envelope(
        identity(),
        StableIdempotencyId("idem:standing:update".to_string()),
        versions(),
        LeaderAiStandingOrderAction::Update {
            standing_order_id: "standing:one".to_string(),
            patch: StandingOrderDraftPatch {
                instruction: Some("prioritize seasoned logs".to_string()),
                priority_basis_points: Some(7_000),
                ..StandingOrderDraftPatch::default()
            },
        },
    )
    .expect("valid standing order update");
    assert!(matches!(
        update.payload,
        LeaderAiActionPayload::UpdateStandingOrder { .. }
    ));

    let delete = build_standing_order_action_envelope(
        identity(),
        StableIdempotencyId("idem:standing:delete".to_string()),
        versions(),
        LeaderAiStandingOrderAction::Delete {
            standing_order_id: "standing:one".to_string(),
        },
    )
    .expect("valid standing order delete");
    assert!(matches!(
        delete.payload,
        LeaderAiActionPayload::DeleteStandingOrder { .. }
    ));
}

#[test]
fn stale_refresh_preserves_focus_and_draft_only_for_visible_plan_context() {
    let draft = StandingOrderDraft {
        order_kind: "reserve".to_string(),
        domain: "wood".to_string(),
        target_id: None,
        instruction: "keep a dry reserve".to_string(),
        priority_basis_points: 5_500,
        expires_at_ms: None,
    };
    let response = LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("idem:stale:one").unwrap(),
        colony_id: cat_protocol::SelectedColonyId::new("colony:one").unwrap(),
        result: LeaderAiActionResult::Rejected {
            conflict: ActionConflict::VersionMismatch {
                current_version_hint: current_versions(),
                current_state_hint: CurrentStateHint {
                    state_code: text("plan reordered"),
                    visible_entity_id: None,
                    visible_stage: Some(text("refresh")),
                },
            },
        },
        refresh: None,
    };

    let refresh = VersionMismatchRefreshHandler::handle(
        &response,
        Some("plan:p0"),
        Some(draft.clone()),
        &["plan:p0".to_string(), "plan:p1".to_string()],
    )
    .expect("stale refresh");
    assert_eq!(refresh.refresh_state, PlansRefreshState::Stale);
    assert_eq!(refresh.focus.0.as_deref(), Some("plan:p0"));
    assert_eq!(refresh.draft.0, Some(draft));
    assert_eq!(refresh.feedback.message, "plan reordered");

    let removed = VersionMismatchRefreshHandler::handle(
        &response,
        Some("plan:removed"),
        None,
        &["plan:p0".to_string()],
    )
    .expect("removed plan refresh");
    assert_eq!(removed.focus.0, None);
}

#[test]
fn projection_and_actions_do_not_depend_on_hidden_truth_or_private_client_state() {
    let projection = render_authoritative_top_eight_plans(
        &colony(),
        StandingOrdersPanel::empty(1, 0),
        PlansRefreshState::UpdateRequired,
    );
    let rendered_text = format!("{projection:?}");

    assert!(rendered_text.contains("RegenerationUnavailableBelowReportLevel4"));
    assert!(!rendered_text.contains("hidden"));
    assert!(!rendered_text.contains("exact regeneration"));
    assert!(!rendered_text.contains("private belief"));
    assert_eq!(
        projection.refresh_state.feedback(),
        cat_client::leader_ai_ui::FeedbackState::UpdateRequired
    );
}
