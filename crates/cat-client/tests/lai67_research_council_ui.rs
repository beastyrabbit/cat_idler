//! LAI.67 structural/report-safe checks. The coordinator owns test execution.

use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    lai54::bevy_shell::spawn_live_shell,
    lai67::{
        LAI67_STUDY_GRAPH_REGIONS, LAI67_VISUAL_DIRECTION, Lai67Pane, Lai67PaneKind,
        Lai67ReportsRoot, Lai67ResearchCouncilPlugin, Lai67SnapshotFeed, Lai67SurfaceState,
        Lai67ViewState, is_lai67_allowed_action, lai67_layout_contract, project_lai67_reports,
    },
};
use cat_protocol::{
    CANONICAL_SNAPSHOT_SCHEMA_VERSION, CanonicalGodAction, CanonicalSnapshotEnvelope,
    PROTOCOL_VERSION, StableId,
};
use serde_json::{Value, json};

fn hole_tiles() -> Vec<Value> {
    (0..25)
        .map(|value| json!({ "x": value % 5, "y": value / 5 }))
        .collect()
}

fn work_tiles() -> Vec<Value> {
    (1..=3)
        .flat_map(|y| (1..=3).map(move |x| json!({ "x": x, "y": y })))
        .collect()
}

fn report_snapshot() -> CanonicalSnapshotEnvelope {
    let value = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "snapshotSchemaVersion": CANONICAL_SNAPSHOT_SCHEMA_VERSION,
        "nowMs": 1_200_000,
        "selectedColonyId": "colony:home",
        "publicColonies": [{
            "colonyId": "colony:home",
            "displayName": "Home",
            "canView": true,
            "canControl": true
        }],
        "colonies": [{
            "colonyId": "colony:home",
            "stateVersion": 19,
            "plans": [{
                "planId": "plan:food",
                "topicId": "topic:food",
                "phase": "reviewing",
                "priorityBasisPoints": 9100,
                "confidence": "moderate",
                "rationale": "Food days are reported low",
                "dependencies": [{ "planId": "plan:water", "satisfied": false }],
                "responsibleOfficerId": "cat:lore"
            }],
            "officerRequests": [{
                "requestId": "request:research",
                "officerId": "cat:lore",
                "requestKind": "request_kind:research",
                "rationale": "A report-visible study is useful",
                "confidence": "high",
                "capabilityId": "capability:mill"
            }],
            "standingOrderCapabilities": [{
                "capabilityId": "capability:food",
                "officeId": "office:farmer",
                "orderKindId": "order_kind:conserve",
                "enabled": true,
                "reason": "Farmer duty is reported"
            }],
            "standingOrders": [{
                "orderId": "order:food",
                "capabilityId": "capability:food",
                "instruction": "Conserve food when reports worsen",
                "expiresAtMs": null
            }],
            "tasks": [{
                "taskId": "task:scholar",
                "taskKindId": "task_kind:research_prepare",
                "siteId": "building:school",
                "siteKindId": "building:school",
                "objective": "Prepare the reported mill study",
                "state": "in_progress",
                "footprint": { "orderedTiles": [
                    { "x": 4, "y": 4 }, { "x": 5, "y": 4 }, { "x": 6, "y": 4 },
                    { "x": 4, "y": 5 }, { "x": 5, "y": 5 }, { "x": 6, "y": 5 },
                    { "x": 4, "y": 6 }, { "x": 5, "y": 6 }, { "x": 6, "y": 6 }
                ] },
                "route": { "orderedTiles": [
                    { "x": 3, "y": 5 }, { "x": 4, "y": 5 }
                ] },
                "workerCatIds": ["cat:lore"],
                "cargo": [],
                "reservations": [],
                "refusals": [],
                "anatomyRequirements": [],
                "blockers": []
            }],
            "cats": [{
                "catId": "cat:lore",
                "displayName": "Lore",
                "attributes": [{
                    "attributeId": "attribute:intelligence",
                    "inheritedValue": 8,
                    "learnedValue": 3,
                    "totalValue": 11
                }],
                "skills": [{
                    "skillId": "skill:research",
                    "xp": 12100,
                    "level": 100,
                    "mastery": 2100
                }],
                "affinities": [{
                    "laborId": "labor:research",
                    "disposition": "Loved",
                    "refusing": false,
                    "refusalReason": null
                }],
                "anatomyEligibility": ["body:front_paws"],
                "family": {
                    "householdId": "household:scholars",
                    "parentIds": [],
                    "childIds": [],
                    "residenceId": "home:scholars",
                    "mentorId": null,
                    "traditionId": "tradition:scholar",
                    "surname": "Scholar",
                    "enterpriseId": "enterprise:school"
                },
                "officeId": "office:loremaster",
                "successionEligible": true
            }],
            "governance": {
                "electionId": "election:19",
                "candidates": [{
                    "catId": "cat:lore",
                    "reportReason": "Reported civic merit",
                    "backingBlocks": 0,
                    "eligible": true
                }],
                "officers": [{
                    "officeId": "office:loremaster",
                    "catId": "cat:lore",
                    "reportExpertiseLevel": 4,
                    "appointmentCandidateIds": ["cat:lore"]
                }],
                "successionSummary": "Lore is the reported successor"
            },
            "research": {
                "notesBalance": 44,
                "voidBalance": 7,
                "godQueue": [{
                    "studyId": "study:mill",
                    "lane": "god",
                    "position": 1,
                    "fundingState": "funded",
                    "progressBasisPoints": 4500,
                    "duplicateReason": null,
                    "refundReason": null
                }],
                "leaderDecisions": [{
                    "studyId": "study:water",
                    "lane": "leader",
                    "position": 1,
                    "fundingState": "instant unlock reported",
                    "progressBasisPoints": 10000,
                    "duplicateReason": "Free Leader lane avoided the funded mill study",
                    "refundReason": null
                }],
                "preparations": [{
                    "preparationId": "preparation:mill",
                    "studyId": "study:mill",
                    "physicalTaskId": "task:scholar",
                    "progressBasisPoints": 2500,
                    "playerDiscountBasisPoints": 2500
                }]
            },
            "construction": [{
                "projectId": "project:mill",
                "buildingId": "building:mill",
                "phase": "structure",
                "footprint": { "orderedTiles": [{ "x": 10, "y": 10 }] },
                "phaseProgressBasisPoints": 3000,
                "stageCargo": [],
                "artStateId": "art:mill_structure"
            }],
            "storageZones": [],
            "hole": {
                "holeId": "hole:home",
                "width": 2,
                "depth": 3,
                "darkness": 4,
                "footprint": { "orderedTiles": hole_tiles() },
                "workFootprint": { "orderedTiles": work_tiles() },
                "foodPermissionSummary": "Apples are reserved by the reported leader policy",
                "foodPermissions": [{
                    "contentId": "food:apple",
                    "permission": "reserve",
                    "reason": "Food days are low",
                    "confidence": "moderate"
                }],
                "officerReportLevel": 3,
                "regeneration": "unavailable",
                "officerReportedRegeneration": null,
                "contributionReceipts": ["contribution:001"]
            },
            "divine": {
                "inspirationExpiresAtMs": null,
                "activeBoostIds": ["boost:harvest"],
                "rescueAvailable": true,
                "rescueReason": "Reported hunger emergency"
            },
            "diplomacy": {
                "stances": [{
                    "otherColonyId": "colony:river",
                    "stance": "neutral",
                    "consented": true
                }],
                "contracts": [{
                    "contractId": "trade:apple-for-grain",
                    "partnerColonyId": "colony:river",
                    "stage": "en_route",
                    "route": { "orderedTiles": [{ "x": 1, "y": 1 }, { "x": 2, "y": 1 }] },
                    "escrow": [{
                        "cargoId": "cargo:apple",
                        "contentId": "food:apple",
                        "quantity": 5,
                        "qualityBand": 1
                    }],
                    "reportReason": "A close reported barter is possible now"
                }]
            },
            "contentManifest": {
                "manifestVersion": 1,
                "checksumId": "manifest:home",
                "entries": [{
                    "contentId": "study:mill",
                    "contentKindId": "research_study",
                    "displayName": "Mill study",
                    "artKey": "research:mill",
                    "accessibilityLabel": "Mill study",
                    "capabilityIds": ["capability:mill"]
                }]
            },
            "qualityLots": [],
            "exactItems": [],
            "foodStocks": [],
            "huntingSites": [],
            "rareMaterials": [],
            "augmentations": [],
            "fixtures": [],
            "cookhouseBatches": [],
            "fishingHuts": [],
            "visualStates": [],
            "diagnostics": []
        }]
    });
    CanonicalSnapshotEnvelope::decode_json(&value.to_string()).expect("valid report fixture")
}

#[test]
fn research_projects_two_lanes_physical_preparation_and_only_reported_graph_data() {
    let projection = project_lai67_reports(
        &Lai67SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: Default::default(),
        },
        &Lai67ViewState::default(),
    );

    assert_eq!(projection.research.god_queue.len(), 1);
    assert_eq!(projection.research.leader_lane.len(), 1);
    assert_eq!(
        projection.research.preparations[0]
            .physical_task_id
            .as_deref(),
        Some("task:scholar")
    );
    assert_eq!(
        projection
            .research
            .selected_study
            .as_ref()
            .unwrap()
            .study_id,
        "study:mill"
    );
    assert!(matches!(
        projection
            .research
            .selected_study
            .unwrap()
            .physical_scholar_work,
        cat_client::leader_ai_ui::lai67::Lai67Availability::Reported(_)
    ));
    assert!(matches!(
        projection.research.prerequisite_edges,
        cat_client::leader_ai_ui::lai67::Lai67Availability::Unavailable { .. }
    ));
    assert_eq!(projection.research.graph_regions.len(), 3);
    assert_eq!(
        projection.research.graph_regions[0].label,
        LAI67_STUDY_GRAPH_REGIONS[0]
    );
    assert!(!projection.reads_authoritative_world_truth);
    assert!(!projection.recomputes_hidden_rules);
    assert!(!projection.emits_disallowed_controls);
}

#[test]
fn council_keeps_task_geometry_cat_detail_hole_gate_and_barter_report_safe() {
    let projection = project_lai67_reports(
        &Lai67SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: Default::default(),
        },
        &Lai67ViewState::default(),
    );
    let task = &projection.council.tasks.rows[0];
    assert_eq!(task.site_id, "building:school");
    assert_eq!(task.ordered_footprint.len(), 9);
    assert_eq!(task.ordered_route, [(3, 5), (4, 5)]);

    let cat = projection.council.cats.selected.unwrap();
    assert_eq!(cat.skills[0].mastery, 2100);
    assert_eq!(cat.tradition_id.as_deref(), Some("tradition:scholar"));
    assert!(matches!(
        projection.council.hole.regeneration,
        cat_client::leader_ai_ui::lai67::Lai67Availability::Unavailable { .. }
    ));
    assert_eq!(
        projection.council.hole.landmark_footprint,
        cat_client::leader_ai_ui::lai67::Lai67Availability::Reported(
            (0..25)
                .map(|value| ((value % 5) as i32, (value / 5) as i32))
                .collect()
        )
    );
    assert_eq!(
        projection.council.trade.rows[0].escrow[0].content_id,
        "food:apple"
    );
    assert!(matches!(
        projection.council.trade.direct_trade_controls,
        cat_client::leader_ai_ui::lai67::Lai67Availability::Unavailable { .. }
    ));
}

#[test]
fn controls_are_limited_to_the_canonical_god_action_family() {
    assert!(is_lai67_allowed_action(
        &CanonicalGodAction::ResearchQueue {
            study_id: StableId::new("study:mill").unwrap(),
        }
    ));
    assert!(is_lai67_allowed_action(&CanonicalGodAction::Inspiration));
    assert!(is_lai67_allowed_action(
        &CanonicalGodAction::PersonalStance {
            other_colony_id: StableId::new("colony:river").unwrap(),
            stance: cat_protocol::PersonalStance::Neutral,
        }
    ));
    let source = include_str!("../src/leader_ai_ui/lai67.rs");
    for retired in [
        "Favor",
        "Shrine",
        "OfferTithe",
        "AssignOfficer",
        "TradeConsent",
    ] {
        assert!(
            !source.contains(retired),
            "retired control leaked: {retired}"
        );
    }
}

#[test]
fn responsive_contract_and_plugin_create_scrollable_research_and_council_panes() {
    for platform in [
        cat_client::leader_ai_ui::lai54::layout::ClientPlatform::Native,
        cat_client::leader_ai_ui::lai54::layout::ClientPlatform::Wasm,
    ] {
        for viewport in cat_client::leader_ai_ui::lai54::layout::SUPPORTED_VIEWPORTS {
            for scale in cat_client::leader_ai_ui::lai54::layout::UiScale::ALL {
                let layout = lai67_layout_contract(platform, viewport, scale).unwrap();
                assert!(layout.catalog_width_percent > 0.0);
                assert!(layout.graph_width_percent > 0.0);
                assert!(layout.inspector_width_percent > 0.0);
                assert!(layout.minimum_pane_height_px >= 220);
            }
        }
    }
    assert!(LAI67_VISUAL_DIRECTION.product_normal);
    assert!(LAI67_VISUAL_DIRECTION.parchment_content);
    assert!(LAI67_VISUAL_DIRECTION.wood_rules);
    assert!(LAI67_VISUAL_DIRECTION.dark_forest_worktable);
    assert!(!LAI67_VISUAL_DIRECTION.uses_glass);
    assert!(!LAI67_VISUAL_DIRECTION.uses_glow);
    assert!(!LAI67_VISUAL_DIRECTION.uses_kpi_grid);
    assert!(!LAI67_VISUAL_DIRECTION.uses_excessive_pills);

    let mut app = App::new();
    app.add_systems(Startup, spawn_live_shell)
        .add_plugins(Lai67ResearchCouncilPlugin);
    app.update();
    app.update();
    let world = app.world_mut();
    let mut roots = world.query::<&Lai67ReportsRoot>();
    assert_eq!(roots.iter(world).count(), 1);
    let mut panes = world.query_filtered::<Entity, (With<Lai67Pane>, With<ScrollPosition>)>();
    assert_eq!(panes.iter(world).count(), 5);
    let mut research_panes = world.query::<&Lai67Pane>();
    assert_eq!(
        research_panes
            .iter(world)
            .filter(|pane| pane.kind == Lai67PaneKind::ResearchGraph)
            .count(),
        1
    );
}

#[test]
fn loading_stale_error_and_empty_states_are_explicit() {
    let loading = project_lai67_reports(&Lai67SnapshotFeed::default(), &Lai67ViewState::default());
    assert_eq!(loading.research.state, Lai67SurfaceState::Loading);
    let stale = project_lai67_reports(
        &Lai67SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: cat_client::leader_ai_ui::lai67::Lai67RefreshState::Stale {
                stale_since_ms: 900_000,
            },
        },
        &Lai67ViewState::default(),
    );
    assert!(stale.research.state.keeps_last_report_visible());
    let conflict = project_lai67_reports(
        &Lai67SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: cat_client::leader_ai_ui::lai67::Lai67RefreshState::Conflict {
                reason: "Research version changed".to_owned(),
            },
        },
        &Lai67ViewState::default(),
    );
    assert!(matches!(
        conflict.research.state,
        Lai67SurfaceState::Conflict { .. }
    ));
    assert!(conflict.research.state.blocks_remote_actions());
    let error = project_lai67_reports(
        &Lai67SnapshotFeed {
            envelope: None,
            refresh: cat_client::leader_ai_ui::lai67::Lai67RefreshState::Error {
                message: "connection closed".to_owned(),
            },
        },
        &Lai67ViewState::default(),
    );
    assert!(matches!(
        error.council.state,
        Lai67SurfaceState::Error { .. }
    ));
}
