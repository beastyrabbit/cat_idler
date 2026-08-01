//! Pure/structural LAI.66 checks. The coordinator owns all test execution.

use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    lai54::{
        bevy_shell::spawn_live_shell,
        layout::{ClientPlatform, SUPPORTED_VIEWPORTS, UiScale},
        shell::PrimaryScreen,
    },
    lai66::{
        LAI66_VISUAL_DIRECTION, Lai66DetailPane, Lai66FocusRefresh, Lai66PrimaryPane,
        Lai66RefreshState, Lai66ReportsPlugin, Lai66ReportsRoot, Lai66ScreenRoot,
        Lai66SnapshotFeed, Lai66SurfaceState, Lai66ViewState, ReportAvailability,
        lai66_layout_contract, project_lai66_reports, retain_lai66_focus_after_refresh,
        stable_semantic_id,
    },
};
use cat_protocol::{
    CANONICAL_SNAPSHOT_SCHEMA_VERSION, CanonicalSnapshotEnvelope, PROTOCOL_VERSION,
};
use serde_json::{Value, json};

fn hole_tiles() -> Vec<Value> {
    (0..25)
        .map(|value| json!({ "x": value % 5, "y": value / 5 }))
        .collect()
}

fn hole_work_tiles() -> Vec<Value> {
    (1..=3)
        .flat_map(|y| (1..=3).map(move |x| json!({ "x": x, "y": y })))
        .collect()
}

fn report_snapshot() -> CanonicalSnapshotEnvelope {
    let encoded = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "snapshotSchemaVersion": CANONICAL_SNAPSHOT_SCHEMA_VERSION,
        "nowMs": 2_000_000,
        "selectedColonyId": "colony:home",
        "publicColonies": [{
            "colonyId": "colony:home",
            "displayName": "Home",
            "canView": true,
            "canControl": true
        }],
        "colonies": [{
            "colonyId": "colony:home",
            "stateVersion": 17,
            "tasks": [{
                "taskId": "task:haul_apples",
                "taskKindId": "task_kind:hauling",
                "siteId": "zone:yard",
                "siteKindId": "stockpile",
                "objective": "Move reported apples into the yard",
                "state": "blocked",
                "footprint": { "orderedTiles": [{ "x": 8, "y": 9 }] },
                "route": { "orderedTiles": [
                    { "x": 6, "y": 9 }, { "x": 7, "y": 9 }, { "x": 8, "y": 9 }
                ] },
                "workerCatIds": ["cat:oak"],
                "blockers": [{
                    "blockerId": "blocker:gate",
                    "reason": "Gate route is blocked",
                    "recoverable": true
                }]
            }],
            "cats": [
                {
                    "catId": "cat:fern",
                    "displayName": "Fern",
                    "lifeStage": "adult",
                    "jobId": "job:steward",
                    "family": {
                        "householdId": "household:millers",
                        "partnershipId": "partnership:millers",
                        "childIds": ["cat:oak"],
                        "residenceId": "home:millers",
                        "traditionId": "tradition:milling",
                        "surname": "Miller",
                        "enterpriseId": "enterprise:mill"
                    },
                    "officeId": "office:steward",
                    "successionEligible": true
                },
                {
                    "catId": "cat:oak",
                    "displayName": "Oak",
                    "lifeStage": "adolescent",
                    "jobId": "job:hauler",
                    "family": {
                        "householdId": "household:millers",
                        "partnershipId": "partnership:millers",
                        "parentIds": ["cat:fern"],
                        "residenceId": "home:millers",
                        "mentorId": "cat:fern",
                        "traditionId": "tradition:milling",
                        "surname": "Miller",
                        "enterpriseId": "enterprise:mill"
                    },
                    "successionEligible": false
                }
            ],
            "jobAssignments": [
                {
                    "assignmentId": "assignment:fern",
                    "catId": "cat:fern",
                    "jobKindId": "job_kind:steward",
                    "stationId": "office:steward",
                    "active": true,
                    "reportReason": "Current appointed duty"
                },
                {
                    "assignmentId": "assignment:oak",
                    "catId": "cat:oak",
                    "jobKindId": "job_kind:hauler",
                    "stationId": "zone:yard",
                    "active": true,
                    "reportReason": "Assigned to reported hauling work"
                }
            ],
            "residences": [{
                "residenceId": "home:millers",
                "housingKindId": "housing_kind:family_home",
                "footprint": { "orderedTiles": [
                    { "x": 12, "y": 12 }, { "x": 13, "y": 12 },
                    { "x": 12, "y": 13 }, { "x": 13, "y": 13 }
                ] },
                "capacity": 6,
                "residentCatIds": ["cat:fern", "cat:oak"],
                "housingPressureBasisPoints": 3300
            }],
            "governance": {
                "electionId": "election:17",
                "candidates": [{
                    "catId": "cat:fern",
                    "reportReason": "Strong reported service record",
                    "backingBlocks": 1,
                    "eligible": true
                }],
                "officers": [{
                    "officeId": "office:steward",
                    "catId": "cat:fern",
                    "reportExpertiseLevel": 4,
                    "appointmentCandidateIds": ["cat:fern", "cat:oak"]
                }],
                "successionSummary": "Fern is the leading reported successor"
            },
            "research": { "notesBalance": 12, "voidBalance": 3 },
            "storageZones": [{
                "zoneId": "zone:yard",
                "linkedWorkshopId": "workshop:main",
                "footprint": { "orderedTiles": [{ "x": 8, "y": 9 }] },
                "tiles": [{
                    "tile": { "x": 8, "y": 9 },
                    "slots": [
                        {
                            "slotId": "slot:01",
                            "containerId": "container:basket",
                            "fullnessBasisPoints": 5000
                        },
                        {
                            "slotId": "slot:02",
                            "lotId": "lot:grain",
                            "fullnessBasisPoints": 2500
                        }
                    ]
                }],
                "containers": [{
                    "containerId": "container:basket",
                    "containerKindId": "container_kind:basket",
                    "capacitySlots": 4,
                    "containedContentId": "food:apple",
                    "fullnessBasisPoints": 5000
                }],
                "lots": [{
                    "cargoId": "lot:apple",
                    "contentId": "food:apple",
                    "quantity": 8,
                    "qualityBand": 2,
                    "provenanceId": "tree:north",
                    "createdAtMs": 1_900_000,
                    "containerId": "container:basket",
                    "locationSiteId": "zone:yard"
                }]
            }],
            "hole": {
                "holeId": "hole:home",
                "width": 2,
                "depth": 1,
                "darkness": 3,
                "footprint": { "orderedTiles": hole_tiles() },
                "workFootprint": { "orderedTiles": hole_work_tiles() },
                "foodPermissionSummary": "Apples are held in reserve",
                "foodPermissions": [{
                    "contentId": "food:apple",
                    "permission": "reserve",
                    "reason": "Low reported food days",
                    "confidence": "moderate"
                }],
                "officerReportLevel": 3,
                "regeneration": "unavailable"
            },
            "divine": { "rescueAvailable": false },
            "diplomacy": { "stances": [], "contracts": [] },
            "contentManifest": {
                "manifestVersion": 1,
                "checksumId": "manifest:home",
                "entries": [{
                    "contentId": "food:apple",
                    "contentKindId": "food",
                    "displayName": "Apple",
                    "artKey": "food:apple",
                    "accessibilityLabel": "Apple",
                    "capabilityIds": []
                }]
            },
            "qualityLots": [{
                "lotId": "lot:grain",
                "contentId": "material:grain",
                "quantity": 4,
                "quality": "common",
                "provenanceId": "farm:east",
                "ageMs": 120000,
                "locationSiteId": "zone:yard"
            }],
            "eventLog": [
                {
                    "eventId": "event:001",
                    "domainId": "storage",
                    "eventKindId": "event_kind:route_blocked",
                    "message": "Gate route is blocked",
                    "occurredAtMs": 1_100_000,
                    "repeatedCount": 2,
                    "confidence": "moderate",
                    "sourceIds": ["event_source:001", "event_source:002"]
                },
                {
                    "eventId": "event:003",
                    "domainId": "village",
                    "eventKindId": "event_kind:teaching_completed",
                    "message": "Teaching obligation completed",
                    "occurredAtMs": 1_200_000,
                    "repeatedCount": 1,
                    "confidence": "officer_verified",
                    "sourceIds": ["task:teaching"]
                }
            ]
        }]
    });
    CanonicalSnapshotEnvelope::decode_json(&encoded.to_string()).expect("valid canonical report")
}

#[test]
fn log_groups_repetition_without_losing_source_history_or_inventing_confidence() {
    let feed = Lai66SnapshotFeed {
        envelope: Some(report_snapshot()),
        refresh: Lai66RefreshState::Ready,
    };
    let projection = project_lai66_reports(&feed, &Lai66ViewState::default());

    assert_eq!(projection.log.total_reported_events, 3);
    assert_eq!(projection.log.total_grouped_rows, 2);
    let repeated = projection
        .log
        .visible_groups
        .iter()
        .find(|group| group.domain_id == "storage")
        .expect("storage group");
    assert_eq!(repeated.repeat_count, 2);
    assert_eq!(repeated.ledger_event_ids, ["event:001"]);
    assert_eq!(
        repeated.source_event_ids,
        ["event_source:001", "event_source:002"]
    );
    assert!(matches!(
        &repeated.confidence,
        ReportAvailability::Reported(cat_protocol::ReportConfidence::Moderate)
    ));
    assert!(matches!(
        projection.log.authoritative_history_coverage,
        ReportAvailability::Reported(_)
    ));
    assert!(!projection.reads_authoritative_world_truth);
    assert!(!projection.recomputes_hidden_rules);
    assert!(!projection.exposes_mutation_controls);
}

#[test]
fn stores_preserve_exact_lots_containers_permissions_routes_and_blockers() {
    let feed = Lai66SnapshotFeed {
        envelope: Some(report_snapshot()),
        refresh: Lai66RefreshState::Ready,
    };
    let mut view = Lai66ViewState::default();
    view.selected_zone_id = Some("zone:yard".to_owned());
    let stores = project_lai66_reports(&feed, &view).stores;
    let zone = stores.selected_zone.expect("selected zone");

    assert_eq!(stores.visible_loose_slots, 2);
    assert_eq!(stores.occupied_loose_slots, 2);
    assert_eq!(zone.containers[0].internal_lot_ids, ["lot:apple"]);
    assert_eq!(zone.lots.len(), 2);
    assert_eq!(zone.lots[0].provenance_id, "tree:north");
    assert_eq!(zone.linked_hauling[0].ordered_route.len(), 3);
    assert_eq!(zone.blockers[0].blocker_id, "blocker:gate");
    assert_eq!(
        stores.food_permissions[0].permission,
        cat_protocol::FoodPermission::Reserve
    );
    assert!(matches!(
        stores.explicit_workshop_zone_links,
        ReportAvailability::Reported(ref links)
            if links[0].workshop_id == "workshop:main"
    ));
}

#[test]
fn village_uses_reported_households_jobs_governance_and_marks_missing_relations() {
    let feed = Lai66SnapshotFeed {
        envelope: Some(report_snapshot()),
        refresh: Lai66RefreshState::Ready,
    };
    let mut view = Lai66ViewState::default();
    view.selected_household_id = Some("household:millers".to_owned());
    let village = project_lai66_reports(&feed, &view).village;

    assert_eq!(village.demographics.reported_resident_count, 2);
    assert_eq!(village.demographics.assigned_office_count, 1);
    assert_eq!(village.employment[1].active_task_ids, ["task:haul_apples"]);
    let household = village.selected_household.expect("household selection");
    assert_eq!(
        household.parent_child_edges,
        [("cat:fern".to_owned(), "cat:oak".to_owned())]
    );
    assert_eq!(village.traditions[0].tradition_id, "tradition:milling");
    assert_eq!(village.enterprises[0].enterprise_id, "enterprise:mill");
    assert_eq!(village.election.candidates[0].backing_blocks, 1);
    assert_eq!(village.officers[0].report_expertise_level, 4);
    assert!(matches!(
        village.partnerships,
        ReportAvailability::Reported(ref rows)
            if rows[0].cat_ids == ["cat:fern", "cat:oak"]
    ));
    assert!(matches!(
        village.durable_job_assignments,
        ReportAvailability::Reported(ref rows) if rows.len() == 2
    ));
    assert!(matches!(
        village.housing[0].housing_pressure,
        ReportAvailability::Reported(ref pressure) if pressure == "3300/10000"
    ));
    assert!(matches!(
        village.housing[0].reported_capacity,
        ReportAvailability::Reported(6)
    ));
    assert!(matches!(
        village.demographics.life_stage_counts,
        ReportAvailability::Reported(ref rows)
            if rows.iter().find(|row| row.id == "adult").unwrap().count == 1
    ));
}

#[test]
fn loading_error_stale_and_empty_are_explicit_and_stale_keeps_the_report() {
    let loading = project_lai66_reports(&Lai66SnapshotFeed::default(), &Lai66ViewState::default());
    assert_eq!(loading.log.state, Lai66SurfaceState::Loading);

    let stale = project_lai66_reports(
        &Lai66SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: Lai66RefreshState::Stale {
                stale_since_ms: 1_500_000,
            },
        },
        &Lai66ViewState::default(),
    );
    assert!(stale.log.state.keeps_last_report_visible());
    assert_eq!(stale.log.visible_groups.len(), 2);

    let error = project_lai66_reports(
        &Lai66SnapshotFeed {
            envelope: None,
            refresh: Lai66RefreshState::Error {
                message: "connection closed".to_owned(),
            },
        },
        &Lai66ViewState::default(),
    );
    assert!(matches!(
        error.stores.state,
        Lai66SurfaceState::Error { .. }
    ));

    let mut filtered = Lai66ViewState::default();
    filtered.log_filters.query = "not present".to_owned();
    let empty = project_lai66_reports(
        &Lai66SnapshotFeed {
            envelope: Some(report_snapshot()),
            refresh: Lai66RefreshState::Ready,
        },
        &filtered,
    );
    assert_eq!(empty.log.state, Lai66SurfaceState::Empty);
}

#[test]
fn responsive_contract_and_material_direction_cover_the_full_desktop_matrix() {
    for platform in [ClientPlatform::Native, ClientPlatform::Wasm] {
        for viewport in SUPPORTED_VIEWPORTS {
            for scale in UiScale::ALL {
                let layout =
                    lai66_layout_contract(platform, viewport, scale).expect("desktop layout");
                assert!(layout.primary_width_percent > 0.0);
                assert!(layout.detail_width_percent > 0.0);
                assert!(layout.minimum_pane_height_px >= 220);
                assert!(layout.row_minimum_height_px >= 32);
            }
        }
    }
    assert!(LAI66_VISUAL_DIRECTION.product_normal);
    assert!(LAI66_VISUAL_DIRECTION.parchment_content);
    assert!(LAI66_VISUAL_DIRECTION.wood_rules);
    assert!(LAI66_VISUAL_DIRECTION.dark_forest_worktable);
    assert!(!LAI66_VISUAL_DIRECTION.uses_glass);
    assert!(!LAI66_VISUAL_DIRECTION.uses_glow);
    assert!(!LAI66_VISUAL_DIRECTION.uses_kpi_grid);
    assert!(!LAI66_VISUAL_DIRECTION.uses_excessive_pills);
}

#[test]
fn plugin_spawns_one_routed_surface_with_scrollable_master_detail_panes() {
    let mut app = App::new();
    app.add_systems(Startup, spawn_live_shell)
        .add_plugins(Lai66ReportsPlugin);
    app.update();
    app.update();

    let world = app.world_mut();
    let mut roots = world.query::<&Lai66ReportsRoot>();
    assert_eq!(roots.iter(world).count(), 1);
    let mut screens = world.query::<&Lai66ScreenRoot>();
    let screens = screens
        .iter(world)
        .map(|screen| screen.0)
        .collect::<Vec<_>>();
    assert_eq!(screens.len(), 3);
    assert!(screens.contains(&PrimaryScreen::Log));
    assert!(screens.contains(&PrimaryScreen::Stores));
    assert!(screens.contains(&PrimaryScreen::Village));
    let mut primary =
        world.query_filtered::<Entity, (With<Lai66PrimaryPane>, With<ScrollPosition>)>();
    let mut detail =
        world.query_filtered::<Entity, (With<Lai66DetailPane>, With<ScrollPosition>)>();
    assert_eq!(primary.iter(world).count(), 3);
    assert_eq!(detail.iter(world).count(), 3);
}

#[test]
fn semantic_ids_are_stable_bounded_and_collision_resistant_for_real_authority_ids() {
    let first = stable_semantic_id("stores", &format!("planner:v1|{}:lot", "a".repeat(128)));
    let second = stable_semantic_id(
        "stores",
        &format!("planner:v1|{}:lot", "a".repeat(127) + "b"),
    );
    assert!(first.len() < 160);
    assert!(second.len() < 160);
    assert_ne!(first, second);
    assert_eq!(
        stable_semantic_id("village", "household:millers"),
        "lai66:village:household:millers"
    );
}

#[test]
fn focus_survives_stale_refresh_or_moves_to_the_screen_refresh_control() {
    let mut focus = Some("lai66:stores:zone:yard".to_owned());
    assert_eq!(
        retain_lai66_focus_after_refresh(
            &mut focus,
            ["lai66:stores:refresh", "lai66:stores:zone:yard"],
            "lai66:stores:refresh"
        ),
        Lai66FocusRefresh::Preserved
    );
    assert_eq!(focus.as_deref(), Some("lai66:stores:zone:yard"));

    assert_eq!(
        retain_lai66_focus_after_refresh(
            &mut focus,
            ["lai66:stores:refresh"],
            "lai66:stores:refresh"
        ),
        Lai66FocusRefresh::MovedToScreenRefresh
    );
    assert_eq!(focus.as_deref(), Some("lai66:stores:refresh"));
}
