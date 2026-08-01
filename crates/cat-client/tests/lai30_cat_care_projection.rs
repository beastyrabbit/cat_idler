use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    AuthenticatedPlayerIdentity, BodyPartSlot, CatCareAction, CatCareActionBuildError,
    CatCareControlDisabledReason, CatCareDraft, CatCarePanelInput, CatCarePanelPlugin,
    CatCarePanelProjectionResource, CatCarePanelState, CatCareRefreshState,
    CatCareRegenerationProjection, CatCareVersionMismatchRefreshHandler, ExpectedCatCareVersion,
    ExpectedCatCareVersionBundle, ExpectedProstheticVersion, RoleColor, StableIdempotencyId,
    project_cat_care_regeneration_report, render_cat_care_panel,
};
use cat_protocol::{
    ActionConflict, ActionIdempotencyId, ActionProtocolVersion, AnatomySnapshot,
    AutomaticResearchQuotaSnapshot, BeliefReportSnapshot, BodyPartSnapshot, BoundedAgeMs,
    BoundedBasisPoints, CareStatusSnapshot, CatCareSnapshot, CatPersonalitySnapshot,
    CatTraitsSnapshot, ColonyAiSnapshot, CurrentStateHint, CurrentVersionHint, DiplomacySnapshot,
    FavorLedgerSnapshot, InjurySnapshot, InsightSnapshot, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, MANIFEST_STUDY_COUNT, NamedBasisPointSnapshot,
    NonEmptyStableId, PlanQueueSnapshot, ProstheticSnapshot, ProstheticWearSnapshot,
    RegenerationReportSnapshot, ReportEstimateSnapshot, ReportProvenanceSnapshot, ReportSafeString,
    ResearchFrontierSnapshot, ReservationSummarySnapshot, ShrineSnapshot,
    SiteLifecycleStageSnapshot, SiteRefActionTarget, SiteRefSnapshot, SiteSnapshot,
    SiteVisibilitySnapshot, SnapshotProtocolVersion, SnapshotTilePoint as TilePoint,
    SnapshotVillageCapabilities, StaleClientRefresh, StressSnapshot, TaskCargoSnapshot,
    TreatmentSnapshot, VisibleTaskSnapshot, WillingnessSnapshot, WorkSlotSnapshot,
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
        observer_id: Some(id("cat:medic")),
        method: text("reported"),
    }
}

fn report(report_id: &str, level: u8) -> BeliefReportSnapshot {
    BeliefReportSnapshot {
        report_id: id(report_id),
        report_version: 1,
        subject_id: id("cat:mallow"),
        domain: text("health"),
        estimate: estimate(2, 4, "care"),
        confidence_basis_points: bp(7_000),
        age_ms: BoundedAgeMs::new(500),
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

fn care_site() -> SiteRefSnapshot {
    SiteRefSnapshot::Tile {
        site: site("site:care-den"),
        tile: tile(7, 3),
    }
}

fn body_part(
    body_part_id: &str,
    side: Option<&str>,
    functional_basis_points: u16,
    injury: Option<InjurySnapshot>,
    prosthetic_id: Option<&str>,
) -> BodyPartSnapshot {
    BodyPartSnapshot {
        body_part_id: id(body_part_id),
        side: side.map(text),
        functional_basis_points: bp(functional_basis_points),
        injury,
        prosthetic_id: prosthetic_id.map(id),
    }
}

fn injury() -> InjurySnapshot {
    InjurySnapshot {
        injury_id: id("injury:sprain-left-front"),
        injury_kind: text("sprain"),
        severity_basis_points: bp(6_500),
        sustained_at_ms: 9_000,
        treatment: Some(TreatmentSnapshot {
            treatment_id: id("treatment:sprain-left-front"),
            stage: text("bandaged"),
            medic_cat_id: Some(id("cat:medic")),
            care_site: Some(care_site()),
            task_id: Some(id("task:care")),
        }),
    }
}

fn cat() -> CatCareSnapshot {
    CatCareSnapshot {
        cat_id: id("cat:mallow"),
        display_name: text("Mallow"),
        active_task_id: Some(id("task:care")),
        traits: CatTraitsSnapshot {
            innate_attributes: vec![
                NamedBasisPointSnapshot {
                    name: id("attribute:medicine"),
                    value_basis_points: bp(8_000),
                },
                NamedBasisPointSnapshot {
                    name: id("attribute:coordination"),
                    value_basis_points: bp(6_000),
                },
            ],
            learned_skills: vec![NamedBasisPointSnapshot {
                name: id("skill:treatment"),
                value_basis_points: bp(7_500),
            }],
            office_experience: vec![NamedBasisPointSnapshot {
                name: id("office:healer"),
                value_basis_points: bp(2_500),
            }],
            acquired_traits: vec![id("trait:careful"), id("trait:prosthetic-adapted")],
        },
        personality: CatPersonalitySnapshot {
            sociability: bp(4_000),
            diligence: bp(8_000),
            courage: bp(5_500),
            empathy: bp(9_000),
            curiosity: bp(6_500),
        },
        stress: StressSnapshot {
            stress_basis_points: bp(8_500),
            recovery_basis_points: bp(2_000),
            refusing: true,
            refusal_reason: Some(text("pain")),
        },
        willingness: WillingnessSnapshot {
            total_basis_points: bp(3_500),
            factors: vec![
                NamedBasisPointSnapshot {
                    name: id("factor:injury"),
                    value_basis_points: bp(2_000),
                },
                NamedBasisPointSnapshot {
                    name: id("factor:stress"),
                    value_basis_points: bp(1_500),
                },
            ],
            eligible: false,
            eligibility_reason: Some(text("injury risk")),
        },
        anatomy: AnatomySnapshot {
            body_parts: vec![
                body_part(
                    "front_left_paw",
                    Some("left"),
                    4_500,
                    Some(injury()),
                    Some("prosthetic:willow-left-front"),
                ),
                body_part("front_right_paw", Some("right"), 10_000, None, None),
                body_part("hind_left_paw", Some("left"), 9_500, None, None),
                body_part("hind_right_paw", Some("right"), 10_000, None, None),
                body_part("left_eye", Some("left"), 10_000, None, None),
                body_part("right_eye", Some("right"), 10_000, None, None),
                body_part("tail", None, 10_000, None, None),
            ],
        },
        prosthetics: vec![ProstheticSnapshot {
            prosthetic_id: id("prosthetic:willow-left-front"),
            body_part_id: id("front_left_paw"),
            prosthetic_kind: text("willow splint"),
            restoration_basis_points: bp(8_500),
            wear: ProstheticWearSnapshot {
                durability_basis_points: bp(9_000),
                wear_basis_points: bp(2_500),
                repair_eligible: true,
                repair_reason: Some(text("worn binding")),
            },
            fitting_task_id: Some(id("task:care")),
            repair_task_id: Some(id("task:care")),
        }],
        care: CareStatusSnapshot {
            care_site: Some(care_site()),
            treatment_task_id: Some(id("task:care")),
            fitting_task_id: Some(id("task:care")),
            repair_task_id: Some(id("task:care")),
            status: text("needs care"),
        },
    }
}

fn visible_task() -> VisibleTaskSnapshot {
    VisibleTaskSnapshot {
        task_id: id("task:care"),
        intent_id: id("intent:care"),
        category: text("Treatment"),
        stage: text("reserved"),
        assigned_cat_ids: vec![id("cat:medic")],
        objective: care_site(),
        work_slots: vec![WorkSlotSnapshot {
            slot_id: id("slot:care"),
            tile: tile(7, 3),
            state: text("assigned"),
        }],
        endpoint: Some(SiteRefSnapshot::Tile {
            site: site("site:workshop"),
            tile: tile(8, 3),
        }),
        footprint: Vec::new(),
        progress_basis_points: bp(2_500),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:care")],
            reservation_version: 44,
        },
        blocked_reason: None,
        cargo: TaskCargoSnapshot {
            cargo_ids: vec![id("cargo:willow-splint")],
            summary: text("willow splint"),
        },
        last_updated_at_ms: 15_000,
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
        reports: vec![report("report:low", 3), report("report:high", 4)],
        plans: PlanQueueSnapshot {
            planner_version: 77,
            planning_epoch: 76,
            plans: Vec::new(),
        },
        officer_requests: Vec::new(),
        officer_institution: None,
        standing_orders: Vec::new(),
        refresh_hints: Vec::new(),
        visible_tasks: vec![visible_task()],
        cats: vec![cat()],
        shrine: ShrineSnapshot {
            shrine_id: id("shrine:one"),
            endpoint: care_site(),
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

fn versions() -> ExpectedCatCareVersionBundle {
    ExpectedCatCareVersionBundle {
        planner_version: 77,
        domain_version: 8,
        resource_version: 9,
        care: ExpectedCatCareVersion(12),
        prosthetic: Some(ExpectedProstheticVersion(5)),
        spatial_version: Some(21),
        reservation_version: Some(44),
    }
}

fn current_versions() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(78),
        domain_version: Some(8),
        resource_version: Some(9),
        spatial_version: Some(21),
        reservation_version: Some(44),
        research_version: None,
        scholar_version: None,
        boost_version: None,
        diplomacy_version: None,
        trade_version: None,
        prosthetic_version: Some(5),
        care_version: Some(13),
        officer_version: None,
        standing_order_version: None,
    }
}

#[test]
fn cat_care_projection_uses_authoritative_report_safe_snapshot_fields() {
    let projection = render_cat_care_panel(
        &colony(),
        "colony:one",
        Some("cat:mallow"),
        CatCareRefreshState::Current,
    );

    assert_eq!(projection.cards.len(), 1);
    assert_eq!(projection.selected_cat_id.as_ref().unwrap().0, "cat:mallow");
    assert_eq!(projection.layout.panel_width_px, 420);
    assert_eq!(projection.layout.compact_panel_width_px, 320);
    assert_eq!(projection.layout.panel_radius_px, 10);
    assert!(projection.layout.world_first);
    assert_eq!(projection.chrome.paper_role, RoleColor::Paper);
    assert_eq!(projection.chrome.care_action_role, RoleColor::Olive);
    assert_eq!(projection.chrome.injury_role, RoleColor::Danger);
    assert_eq!(projection.chrome.prosthetic_role, RoleColor::Rust);
    let card = &projection.cards[0];
    assert_eq!(card.display_name, "Mallow");
    assert_eq!(card.innate_attributes.0[0].name, "attribute:medicine");
    assert_eq!(
        card.learned_skills.learned_skills[0].name,
        "skill:treatment"
    );
    assert_eq!(
        card.learned_skills.office_experience[0].name,
        "office:healer"
    );
    assert_eq!(card.personality.axes.len(), 5);
    assert_eq!(
        card.acquired_traits.0,
        ["trait:careful", "trait:prosthetic-adapted"]
    );
    assert_eq!(card.stress.stress_basis_points, 8_500);
    assert!(card.refusal.refusing);
    assert_eq!(card.refusal.reason.as_deref(), Some("pain"));
    assert!(!card.willingness.eligible);
    assert_eq!(
        card.typed_block_reason
            .as_ref()
            .map(|reason| reason.0.as_str()),
        Some("injury risk")
    );
    assert_eq!(
        card.controls.treatment.disabled_reason,
        Some(CatCareControlDisabledReason::Blocked(
            "injury risk".to_string()
        ))
    );

    assert_eq!(card.anatomy.grid.slots.len(), 7);
    assert_eq!(card.anatomy.grid.slots[0].slot, BodyPartSlot::FrontLeftPaw);
    assert_eq!(card.anatomy.grid.slots[0].label, "left front paw");
    assert_eq!(
        card.anatomy.grid.slots[0].functional_basis_points,
        Some(4_500)
    );
    let treatment = card.anatomy.grid.slots[0]
        .injury
        .as_ref()
        .and_then(|injury| injury.treatment.as_ref())
        .expect("treatment state");
    assert_eq!(treatment.stage, "bandaged");
    assert_eq!(treatment.task_id.as_deref(), Some("task:care"));

    let prosthetic = &card.prosthetics.prosthetics[0];
    assert_eq!(prosthetic.stable_item_id.0, "prosthetic:willow-left-front");
    assert_eq!(prosthetic.side.0, "left");
    assert_eq!(prosthetic.prosthetic_type.0, "willow splint");
    assert_eq!(prosthetic.restoration_percent.0, 85);
    assert_eq!(prosthetic.wear_progress.0, 2_500);
    assert_eq!(prosthetic.repair_reason.as_deref(), Some("worn binding"));

    let task = &card.active_tasks.0[0];
    assert_eq!(task.task_id, "task:care");
    assert_eq!(task.site.site_id, "site:care-den");
    assert_eq!(task.cargo.0, ["cargo:willow-splint"]);
    assert_eq!(task.patient.0, "cat:mallow");
    assert_eq!(task.fitter_or_medic.0.as_deref(), Some("cat:medic"));
    assert_eq!(task.workshop.0.as_deref(), Some("site:workshop"));
    assert!(card.label.as_str().contains("Cat care"));
    assert!(
        card.controls
            .prosthetic_fit
            .label
            .as_str()
            .contains("cat:mallow")
    );

    let debug = format!("{projection:?}");
    assert!(!debug.contains("exact regeneration"));
    assert!(!debug.contains("private"));
}

#[test]
fn selected_colony_filter_prevents_cross_colony_cat_cards() {
    let projection = render_cat_care_panel(
        &colony(),
        "colony:other",
        Some("cat:mallow"),
        CatCareRefreshState::Current,
    );

    assert!(projection.cards.is_empty());
    assert_eq!(projection.selected_cat_id, None);
}

#[test]
fn cat_care_panel_plugin_updates_projection_resource_and_filters_hidden_colonies() {
    let mut app = App::new();
    app.add_plugins(CatCarePanelPlugin);
    app.insert_resource(CatCarePanelState {
        selected_cat_id: Some("cat:mallow".to_string()),
        draft: None,
        refresh_state: CatCareRefreshState::Stale,
    });
    app.insert_resource(CatCarePanelInput {
        selected_colony_id: Some("colony:one".to_string()),
        colony: Some(colony()),
    });

    app.update();

    let projection = app
        .world()
        .resource::<CatCarePanelProjectionResource>()
        .projection
        .as_ref()
        .expect("cat care projection");
    assert_eq!(projection.cards.len(), 1);
    assert_eq!(projection.refresh_state, CatCareRefreshState::Stale);
    assert_eq!(projection.selected_cat_id.as_ref().unwrap().0, "cat:mallow");
    assert_eq!(projection.cards[0].anatomy.grid.slots.len(), 7);
    assert_eq!(
        projection.cards[0].prosthetics.prosthetics[0]
            .stable_item_id
            .0,
        "prosthetic:willow-left-front"
    );

    app.insert_resource(CatCarePanelInput {
        selected_colony_id: Some("colony:other".to_string()),
        colony: Some(colony()),
    });
    app.update();

    assert!(
        app.world()
            .resource::<CatCarePanelProjectionResource>()
            .projection
            .is_none()
    );
}

#[test]
fn regeneration_projection_is_unavailable_below_l4_and_bounded_above_l4() {
    let low = report("report:low", 3);
    let high = report("report:high", 4);

    assert_eq!(
        project_cat_care_regeneration_report(&low),
        CatCareRegenerationProjection::UnavailableBelowLevel4
    );
    assert_eq!(
        project_cat_care_regeneration_report(&high),
        CatCareRegenerationProjection::EstimatedRange {
            minimum: 60_000,
            maximum: 90_000,
            unit: "milliseconds".to_string(),
            provenance_count: 1,
        }
    );
}

#[test]
fn treatment_and_prosthetic_actions_use_real_lai25_envelopes_and_versions() {
    let treatment = cat_client::leader_ai_ui::build_cat_care_action_envelope(
        identity(),
        StableIdempotencyId("idem:care:treat".to_string()),
        versions(),
        CatCareAction::RequestTreatment {
            cat_id: "cat:mallow".to_string(),
            injury_id: "injury:sprain-left-front".to_string(),
            treatment_kind: "treatment:bandage".to_string(),
        },
    )
    .expect("valid treatment action");
    assert_eq!(
        treatment.protocol_version.get(),
        SnapshotProtocolVersion::current().get()
    );
    assert_eq!(treatment.expected_versions.expected_care_version, Some(12));
    assert_eq!(
        treatment.expected_versions.expected_reservation_version,
        Some(44)
    );
    assert!(matches!(
        treatment.payload,
        LeaderAiActionPayload::RequestTreatment { .. }
    ));

    let missing_prosthetic = cat_client::leader_ai_ui::build_cat_care_action_envelope(
        identity(),
        StableIdempotencyId("idem:care:fit-missing".to_string()),
        ExpectedCatCareVersionBundle {
            prosthetic: None,
            spatial_version: None,
            reservation_version: None,
            ..versions()
        },
        CatCareAction::FitProsthetic {
            cat_id: "cat:mallow".to_string(),
            prosthetic_id: "prosthetic:willow-left-front".to_string(),
            body_part_id: "front_left_paw".to_string(),
            fitting_site: SiteRefActionTarget::ExactTile { tile: tile(7, 3) },
            fitter_cat_id: Some("cat:medic".to_string()),
        },
    );
    assert_eq!(
        missing_prosthetic,
        Err(CatCareActionBuildError::MissingVersion(
            "prosthetic_spatial_reservation"
        ))
    );

    let fit = cat_client::leader_ai_ui::build_cat_care_action_envelope(
        identity(),
        StableIdempotencyId("idem:care:fit".to_string()),
        versions(),
        CatCareAction::FitProsthetic {
            cat_id: "cat:mallow".to_string(),
            prosthetic_id: "prosthetic:willow-left-front".to_string(),
            body_part_id: "front_left_paw".to_string(),
            fitting_site: SiteRefActionTarget::ExactTile { tile: tile(7, 3) },
            fitter_cat_id: Some("cat:medic".to_string()),
        },
    )
    .expect("valid fit action");
    assert_eq!(fit.expected_versions.expected_prosthetic_version, Some(5));
    assert_eq!(fit.expected_versions.expected_spatial_version, Some(21));
    assert!(matches!(
        fit.payload,
        LeaderAiActionPayload::FitProsthetic { .. }
    ));

    let repair = cat_client::leader_ai_ui::build_cat_care_action_envelope(
        identity(),
        StableIdempotencyId("idem:care:repair".to_string()),
        versions(),
        CatCareAction::RepairProsthetic {
            prosthetic_id: "prosthetic:willow-left-front".to_string(),
            workshop_id: "workshop:care".to_string(),
            input_reservation_id: "reservation:care".to_string(),
        },
    )
    .expect("valid repair action");
    assert!(matches!(
        repair.payload,
        LeaderAiActionPayload::RepairProsthetic { .. }
    ));
}

#[test]
fn stale_refresh_preserves_visible_cat_selection_and_draft_only_when_safe() {
    let draft = CatCareDraft {
        cat_id: "cat:mallow".to_string(),
        target_id: "injury:sprain-left-front".to_string(),
        action_kind: "treatment".to_string(),
    };
    let response = LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("idem:stale:care").unwrap(),
        colony_id: cat_protocol::SelectedColonyId::new("colony:one").unwrap(),
        result: LeaderAiActionResult::Rejected {
            conflict: ActionConflict::VersionMismatch {
                current_version_hint: current_versions(),
                current_state_hint: CurrentStateHint {
                    state_code: text("care state changed"),
                    visible_entity_id: None,
                    visible_stage: Some(text("refresh")),
                },
            },
        },
        refresh: Some(StaleClientRefresh {
            current_versions: current_versions(),
            current_state_hint: CurrentStateHint {
                state_code: text("care state changed"),
                visible_entity_id: Some(cat_protocol::BoundedEntityId::new("cat:mallow").unwrap()),
                visible_stage: Some(text("refresh")),
            },
        }),
    };

    let refresh = CatCareVersionMismatchRefreshHandler::handle(
        &response,
        Some("cat:mallow"),
        Some(draft.clone()),
        &["cat:mallow".to_string()],
    )
    .expect("stale refresh");
    assert_eq!(refresh.refresh_state, CatCareRefreshState::Stale);
    assert_eq!(refresh.selected_cat.0.as_deref(), Some("cat:mallow"));
    assert_eq!(refresh.draft.0, Some(draft));
    assert_eq!(refresh.feedback.message, "care state changed");

    let removed = CatCareVersionMismatchRefreshHandler::handle(
        &response,
        Some("cat:removed"),
        None,
        &["cat:mallow".to_string()],
    )
    .expect("removed cat refresh");
    assert_eq!(removed.selected_cat.0, None);
}
