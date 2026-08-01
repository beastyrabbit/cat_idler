//! LAI.68 canonical-v3 world-render contract checks.
//!
//! The coordinator owns execution. These checks deliberately use only the
//! canonical report envelope: objective, work-site/bank, and delivery-site
//! geometry must be rendered exactly when present; every omitted role, crop,
//! or enterprise location remains unavailable rather than client-inferred.

use bevy::prelude::*;
use bevy::{asset::AssetMetaCheck, prelude::AssetPlugin};
use cat_client::leader_ai_ui::lai68::{
    Lai68FeedState, Lai68RenderEntity, Lai68RenderKey, Lai68RenderMarkerRole, Lai68SnapshotFeed,
    Lai68UnavailableField, Lai68Viewport, Lai68WorldProjectionResource, Lai68WorldRenderPlugin,
    project_lai68_world,
};
use cat_protocol::{
    CANONICAL_SNAPSHOT_SCHEMA_VERSION, PROTOCOL_VERSION, lai64::CanonicalSnapshotEnvelope,
};
use serde_json::{Value, json};

fn square(x: i32, y: i32, side: i32) -> Vec<Value> {
    (0..side)
        .flat_map(|offset_y| {
            (0..side).map(move |offset_x| {
                json!({
                    "x": x + offset_x,
                    "y": y + offset_y,
                })
            })
        })
        .collect()
}

fn report_snapshot() -> CanonicalSnapshotEnvelope {
    let value = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "snapshotSchemaVersion": CANONICAL_SNAPSHOT_SCHEMA_VERSION,
        "nowMs": 5_000,
        "selectedColonyId": "colony:home",
        "publicColonies": [{
            "colonyId": "colony:home",
            "displayName": "Home",
            "canView": true,
            "canControl": true
        }],
        "colonies": [{
            "colonyId": "colony:home",
            "stateVersion": 9,
            "tasks": [
                {
                    "taskId": "task:ahunt",
                    "taskKindId": "hunt",
                    "siteId": "site:lair_north",
                    "siteKindId": "resource_source:hunting",
                    "objective": "Hunt from the reported northern lair",
                    "state": "in_progress",
                    "footprint": { "orderedTiles": [{ "x": 21, "y": 3 }] },
                    "route": { "orderedTiles": [{ "x": 19, "y": 3 }, { "x": 20, "y": 3 }, { "x": 21, "y": 3 }] },
                    "workerCatIds": ["cat:birch"]
                },
                {
                    "taskId": "task:bhole",
                    "taskKindId": "hole_feed",
                    "siteId": "hole:home",
                    "siteKindId": "hole",
                    "objective": "Deliver approved cargo to the Hole",
                    "state": "reserved",
                    "footprint": { "orderedTiles": square(1, 1, 3) },
                    "route": { "orderedTiles": [{ "x": 0, "y": 2 }, { "x": 1, "y": 2 }] }
                },
                {
                    "taskId": "task:cwater",
                    "taskKindId": "fetch_water",
                    "siteId": "site:water_north",
                    "siteKindId": "resource_source:water",
                    "objective": "Collect reported water",
                    "state": "assigned",
                    "footprint": { "orderedTiles": [{ "x": 30, "y": 5 }] },
                    "route": { "orderedTiles": [{ "x": 28, "y": 5 }, { "x": 29, "y": 5 }, { "x": 30, "y": 5 }] },
                    "workSites": [{
                        "siteId": "site:water_north_bank",
                        "siteKindId": "tile",
                        "slotId": "slot:water_north_bank",
                        "footprint": { "orderedTiles": [{ "x": 30, "y": 6 }] }
                    }],
                    "deliverySite": {
                        "siteId": "zone:yard",
                        "siteKindId": "stockpile",
                        "footprint": { "orderedTiles": [{ "x": 7, "y": 8 }] }
                    }
                },
                {
                    "taskId": "task:dworkshop",
                    "taskKindId": "workshop_work",
                    "siteId": "workshop:main",
                    "siteKindId": "building:workshop",
                    "objective": "Build at the reported Workshop",
                    "state": "in_progress",
                    "footprint": { "orderedTiles": square(10, 10, 3) },
                    "route": { "orderedTiles": [{ "x": 8, "y": 11 }, { "x": 9, "y": 11 }, { "x": 10, "y": 11 }] }
                }
            ],
            "cats": [{
                "catId": "cat:birch",
                "displayName": "Birch",
                "lifeStage": "adult",
                "family": {
                    "householdId": "household:birches",
                    "residenceId": "residence:birch_home",
                    "traditionId": "tradition:woodwork",
                    "surname": "Birch",
                    "enterpriseId": "enterprise:birch_workshop"
                },
                "successionEligible": true
            }],
            "residences": [{
                "residenceId": "residence:birch_home",
                "housingKindId": "housing_kind:family_home",
                "footprint": { "orderedTiles": square(15, 10, 2) },
                "capacity": 6,
                "residentCatIds": ["cat:birch"],
                "housingPressureBasisPoints": 2500
            }],
            "governance": { "candidates": [], "officers": [] },
            "research": { "notesBalance": 0, "voidBalance": 0 },
            "construction": [{
                "projectId": "project:workshop",
                "buildingId": "workshop:main",
                "phase": "structure",
                "footprint": { "orderedTiles": square(10, 10, 3) },
                "phaseProgressBasisPoints": 4500,
                "artStateId": "art:workshop_structure"
            }],
            "storageZones": [{
                "zoneId": "zone:yard",
                "footprint": { "orderedTiles": [{ "x": 7, "y": 8 }] },
                "tiles": [{
                    "tile": { "x": 7, "y": 8 },
                    "slots": [{
                        "slotId": "slot:yard_1",
                        "containerId": "container:crate_1",
                        "fullnessBasisPoints": 7500
                    }, {
                        "slotId": "slot:yard_2",
                        "itemId": "item:oak_hammer",
                        "fullnessBasisPoints": 10000
                    }]
                }],
                "containers": [{
                    "containerId": "container:crate_1",
                    "containerKindId": "container_kind:crate",
                    "capacitySlots": 8,
                    "containedContentId": "material:planks",
                    "fullnessBasisPoints": 7500
                }],
                "lots": [{
                    "cargoId": "lot:oak_planks",
                    "contentId": "material:planks",
                    "quantity": 12,
                    "qualityBand": 3,
                    "provenanceId": "workshop:main",
                    "createdAtMs": 4_000,
                    "containerId": "container:crate_1",
                    "locationSiteId": "zone:yard",
                    "locationTile": { "x": 7, "y": 8 }
                }]
            }],
            "exactItems": [{
                "itemId": "item:oak_hammer",
                "definitionId": "tool:hammer",
                "materialId": "material:oak",
                "quality": "fine",
                "durabilityBasisPoints": 8200,
                "provenanceId": "workshop:main",
                "locationSiteId": "zone:yard"
            }],
            "hole": {
                "holeId": "hole:home",
                "width": 2,
                "depth": 3,
                "darkness": 1,
                "footprint": { "orderedTiles": square(0, 0, 5) },
                "workFootprint": { "orderedTiles": square(1, 1, 3) },
                "foodPermissionSummary": "Apples are reported reserved",
                "officerReportLevel": 3,
                "regeneration": "unavailable"
            },
            "divine": { "rescueAvailable": false },
            "diplomacy": { "stances": [], "contracts": [] },
            "huntingSites": [{
                "siteId": "site:lair_north",
                "siteKindId": "site_kind:cave_entrance",
                "tile": { "x": 21, "y": 3 },
                "levelBand": 2,
                "creatures": [],
                "reportConfidence": "moderate",
                "artKey": "art_lair_visual_11_20"
            }],
            "fishingHuts": [{
                "hutId": "hut:river",
                "footprint": { "orderedTiles": square(35, 4, 3) },
                "dockLandTile": { "x": 37, "y": 5 },
                "reservedWaterTile": { "x": 38, "y": 5 },
                "orientationId": "east",
                "modeId": "mode:fishing",
                "stage": "working",
                "progressBasisPoints": 3300,
                "habitatReport": "Reported shoreline habitat",
                "reportConfidence": "moderate",
                "artKey": "art:fishing_hut_east_working"
            }]
        }]
    });
    CanonicalSnapshotEnvelope::decode_json(&value.to_string()).expect("fixture is canonical")
}

#[test]
fn projects_exact_authoritative_world_geometry_and_declares_protocol_gaps() {
    let projection = project_lai68_world(&Lai68SnapshotFeed {
        envelope: Some(report_snapshot()),
        state: Lai68FeedState::Ready,
    });

    assert!(projection.protocol_valid);
    assert!(!projection.reads_hidden_regeneration);
    assert!(!projection.uses_generic_marker_fallback);
    assert_eq!(
        projection
            .markers
            .iter()
            .filter(|marker| matches!(marker.role, Lai68RenderMarkerRole::HoleBoundary { .. }))
            .count(),
        25
    );
    assert_eq!(
        projection
            .markers
            .iter()
            .filter(|marker| matches!(marker.role, Lai68RenderMarkerRole::HoleWork { .. }))
            .count(),
        9
    );
    assert_eq!(
        projection
            .markers
            .iter()
            .filter(|marker| matches!(&marker.role, Lai68RenderMarkerRole::HoleArt { .. }))
            .count(),
        1
    );
    assert_eq!(projection.markers.iter().filter(|marker| matches!(marker.role, Lai68RenderMarkerRole::WorkshopFootprint { ref task_id, .. } if task_id == "task:dworkshop")).count(), 9);
    assert!(projection.markers.iter().any(|marker| {
        matches!(marker.role, Lai68RenderMarkerRole::HuntObjectiveFootprint { ref task_id, .. } if task_id == "task:ahunt")
            && marker.tile.x == 21
            && marker.tile.y == 3
    }));
    assert!(projection.markers.iter().any(|marker| {
        matches!(marker.role, Lai68RenderMarkerRole::WaterObjectiveFootprint { ref task_id, .. } if task_id == "task:cwater")
            && marker.tile.x == 30
            && marker.tile.y == 5
    }));
    assert!(projection.markers.iter().any(|marker| {
        matches!(marker.role, Lai68RenderMarkerRole::WaterBankWorkSite { ref task_id, ref site_id, ref slot_id, .. } if task_id == "task:cwater" && site_id == "site:water_north_bank" && slot_id.as_deref() == Some("slot:water_north_bank"))
            && marker.tile.x == 30
            && marker.tile.y == 6
    }));
    assert!(projection.markers.iter().any(|marker| {
        matches!(marker.role, Lai68RenderMarkerRole::TaskDeliverySite { ref task_id, ref site_id, .. } if task_id == "task:cwater" && site_id == "zone:yard")
            && marker.tile.x == 7
            && marker.tile.y == 8
    }));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::ConstructionFootprint { ref project_id, .. } if project_id == "project:workshop") && marker.reported_art_key.as_deref() == Some("art:workshop_structure")));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::StorageContainer { ref container_id } if container_id == "container:crate_1") && marker.tooltip.contains("75% full")));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::StorageLot { ref lot_id } if lot_id == "lot:oak_planks") && marker.tooltip.contains("quality band 3") && marker.tooltip.contains("workshop:main")));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::StorageItem { ref item_id } if item_id == "item:oak_hammer") && marker.tooltip.contains("fine quality") && marker.tooltip.contains("82% durability")));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::ResidenceFootprint { ref residence_id, .. } if residence_id == "residence:birch_home")));
    assert!(projection.markers.iter().any(|marker| matches!(marker.role, Lai68RenderMarkerRole::FamilyResidence { ref household_id, .. } if household_id == "household:birches")));
    assert!(
        projection
            .markers
            .windows(2)
            .all(|pair| pair[0].key < pair[1].key)
    );
    assert!(
        projection
            .markers
            .iter()
            .all(|marker| !marker.tooltip.to_ascii_lowercase().contains("regeneration"))
    );

    assert!(
        !projection
            .unavailable
            .contains(&Lai68UnavailableField::WaterBankWorkTile {
                task_id: "task:cwater".to_owned()
            })
    );
    assert!(
        projection
            .unavailable
            .contains(&Lai68UnavailableField::TaskWorkTile {
                task_id: "task:dworkshop".to_owned()
            })
    );
    assert!(
        projection
            .unavailable
            .contains(&Lai68UnavailableField::TaskDeliveryEndpoint {
                task_id: "task:bhole".to_owned()
            })
    );
    assert!(
        projection
            .unavailable
            .contains(&Lai68UnavailableField::CropWorldState)
    );
    assert!(
        projection
            .unavailable
            .contains(&Lai68UnavailableField::EnterpriseWorldLocation {
                enterprise_id: "enterprise:birch_workshop".to_owned()
            })
    );
}

#[test]
fn bevy_projection_entities_dedupe_cull_and_despawn_across_restart() {
    let snapshot = report_snapshot();
    let expected = project_lai68_world(&Lai68SnapshotFeed {
        envelope: Some(snapshot.clone()),
        state: Lai68FeedState::Ready,
    });
    let mut app = App::new();
    app.add_plugins(AssetPlugin {
        file_path: ".".to_owned(),
        meta_check: AssetMetaCheck::Never,
        ..default()
    });
    app.add_plugins(Lai68WorldRenderPlugin);
    app.world_mut().resource_mut::<Lai68SnapshotFeed>().envelope = Some(snapshot);
    app.world_mut().resource_mut::<Lai68SnapshotFeed>().state = Lai68FeedState::Ready;
    app.update();

    let mut keys = {
        let world = app.world_mut();
        let mut query = world.query::<&Lai68RenderEntity>();
        query
            .iter(world)
            .map(|entity| entity.key.clone())
            .collect::<Vec<_>>()
    };
    keys.sort();
    assert_eq!(keys.len(), expected.markers.len());
    assert_eq!(
        keys,
        expected
            .markers
            .iter()
            .map(|marker| marker.key.clone())
            .collect::<Vec<Lai68RenderKey>>()
    );

    *app.world_mut().resource_mut::<Lai68Viewport>() = Lai68Viewport {
        center_x: 0,
        center_y: 0,
        half_width_tiles: 2,
        half_height_tiles: 2,
        orthographic_scale_basis_points: 10_000,
    };
    app.update();
    let has_culled_lair = {
        let world = app.world_mut();
        let mut query = world.query::<(&Lai68RenderEntity, &Visibility)>();
        query
            .iter(world)
            .any(|(entity, visibility)| entity.tile.x == 21 && *visibility == Visibility::Hidden)
    };
    assert!(has_culled_lair);

    app.world_mut().resource_mut::<Lai68SnapshotFeed>().envelope = None;
    app.update();
    let remaining_entities = {
        let world = app.world_mut();
        let mut query = world.query::<&Lai68RenderEntity>();
        query.iter(world).count()
    };
    assert_eq!(remaining_entities, 0);
    assert_eq!(
        app.world()
            .resource::<Lai68WorldProjectionResource>()
            .0
            .removed_keys
            .len(),
        expected.markers.len()
    );

    app.world_mut().resource_mut::<Lai68SnapshotFeed>().envelope = Some(report_snapshot());
    app.world_mut().resource_mut::<Lai68SnapshotFeed>().state = Lai68FeedState::Ready;
    app.update();
    let restarted = {
        let world = app.world_mut();
        let mut query = world.query::<&Lai68RenderEntity>();
        query
            .iter(world)
            .map(|entity| entity.key.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        restarted,
        expected
            .markers
            .iter()
            .map(|marker| marker.key.clone())
            .collect()
    );
}
