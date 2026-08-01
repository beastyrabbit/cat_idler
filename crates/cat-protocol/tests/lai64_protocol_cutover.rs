//! Fast, pure LAI.64 boundary checks.  Integration adapters are owned elsewhere.

use cat_protocol::{
    CANONICAL_ACTION_SCHEMA_VERSION, CANONICAL_SNAPSHOT_SCHEMA_VERSION, CanonicalActionEnvelope,
    CanonicalSnapshotEnvelope, CanonicalWireError, PROTOCOL_VERSION,
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

fn colony(id: &str) -> Value {
    json!({
        "colonyId": id,
        "stateVersion": 4,
        "governance": { "candidates": [], "officers": [] },
        "research": { "notesBalance": 0, "voidBalance": 0 },
        "hole": {
            "holeId": "hole_one", "width": 0, "depth": 0, "darkness": 0,
            "footprint": { "orderedTiles": hole_tiles() },
            "workFootprint": { "orderedTiles": hole_work_tiles() },
            "foodPermissionSummary": "reported policy", "officerReportLevel": 3,
            "regeneration": "unavailable"
        },
        "divine": { "rescueAvailable": false },
        "diplomacy": { "stances": [], "contracts": [] }
    })
}

fn snapshot(colonies: Vec<Value>, selected: &str) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "snapshotSchemaVersion": CANONICAL_SNAPSHOT_SCHEMA_VERSION,
        "nowMs": 1,
        "selectedColonyId": selected,
        "publicColonies": [],
        "colonies": colonies,
    })
}

#[test]
fn canonical_snapshot_round_trips_and_keeps_sorted_multi_colony_partitions() {
    let mut encoded = snapshot(vec![colony("colony:home")], "colony:home");
    encoded["publicColonies"] = json!([
        {
            "colonyId": "colony:home", "displayName": "Home",
            "canView": true, "canControl": true
        },
        {
            "colonyId": "colony:other", "displayName": "Other",
            "canView": true, "canControl": false
        }
    ]);
    encoded["colonies"][0]["contentManifest"] = json!({
        "manifestVersion": 1,
        "checksumId": "manifest:abc123",
        "entries": [{
            "contentId": "food:apple", "contentKindId": "food",
            "displayName": "Apple", "artKey": "food:apple",
            "accessibilityLabel": "Apple", "capabilityIds": []
        }]
    });
    encoded["colonies"][0]["qualityLots"] = json!([{
        "lotId": "lot:apple", "contentId": "food:apple", "quantity": 12,
        "quality": "fine", "provenanceId": "source:tree", "ageMs": 60000,
        "locationSiteId": "stockpile:home"
    }]);
    encoded["colonies"][0]["exactItems"] = json!([{
        "itemId": "item:rod", "definitionId": "tool:fishing_rod",
        "materialId": "material:metal", "quality": "fine",
        "durabilityBasisPoints": 6100, "provenanceId": "craft:rod",
        "locationSiteId": "hut:fishing", "augmentationIds": ["augment:grip"]
    }]);
    encoded["colonies"][0]["foodStocks"] = json!([{
        "contentId": "food:apple", "lotId": "lot:apple", "quantity": 12,
        "quality": "fine", "nutritionBasisPoints": 12000,
        "spoilageBasisPoints": 100, "permission": "allowed",
        "locationSiteId": "stockpile:home"
    }]);
    encoded["colonies"][0]["huntingSites"] = json!([{
        "siteId": "lair:north", "siteKindId": "enemy_lair",
        "tile": { "x": 8, "y": 4 }, "levelBand": 1,
        "creatures": [{
            "creatureId": "creature:mouse", "levelBand": 1,
            "healthBasisPoints": 10000
        }],
        "reportConfidence": "moderate", "cacheLotIds": [],
        "cacheItemIds": [], "artKey": "lair:band1"
    }]);
    encoded["colonies"][0]["rareMaterials"] = json!([{
        "materialInstanceId": "material_instance:fang",
        "materialId": "material:warg_fang", "contentStateId": "material:warg_fang_raw",
        "processed": false, "quality": "fine", "provenanceId": "hunt:north",
        "locationSiteId": "stockpile:home"
    }]);
    encoded["colonies"][0]["augmentations"] = json!([{
        "augmentationInstanceId": "augment:grip", "augmentationId": "augment:warg_grip",
        "targetItemId": "item:rod", "materialInstanceId": "material_instance:fang",
        "installed": true, "effectSummary": "Improved catch rate"
    }]);
    encoded["colonies"][0]["fixtures"] = json!([{
        "fixtureInstanceId": "fixture:stove", "fixtureId": "fixture:cook_stove",
        "stationId": "cookhouse:home", "installed": true, "quality": "common",
        "effectSummary": "Reliable prepared meals"
    }]);
    encoded["colonies"][0]["cookhouseBatches"] = json!([{
        "batchId": "batch:stew", "stationId": "cookhouse:home",
        "recipeId": "recipe:stew", "stage": "working",
        "progressBasisPoints": 2500, "ingredientLotIds": ["lot:apple"],
        "outputLotIds": []
    }]);
    encoded["colonies"][0]["fishingHuts"] = json!([{
        "hutId": "hut:fishing", "footprint": { "orderedTiles": hole_work_tiles() },
        "dockLandTile": { "x": 2, "y": 3 }, "reservedWaterTile": { "x": 2, "y": 4 },
        "orientationId": "south", "modeId": "rod_and_staffed_hut",
        "stage": "working", "progressBasisPoints": 5000,
        "rodItemId": "item:rod", "workerCatId": "cat:fisher",
        "habitatReport": "Moderate fish activity", "reportConfidence": "moderate",
        "artKey": "fishing_hut:south_working"
    }]);
    encoded["colonies"][0]["visualStates"] = json!([{
        "subjectId": "hut:fishing", "artKey": "fishing_hut:south_working",
        "stateId": "working", "accessibilityLabel": "Fishing Hut working",
        "footprint": { "orderedTiles": hole_work_tiles() }
    }]);
    encoded["colonies"][0]["residences"] = json!([{
        "residenceId": "home:elder_lodge", "housingKindId": "elder_lodge",
        "footprint": { "orderedTiles": hole_work_tiles() }, "capacity": 8,
        "residentCatIds": [], "housingPressureBasisPoints": 2500
    }]);
    encoded["colonies"][0]["eventLog"] = json!([{
        "eventId": "event:hole_feed", "domainId": "hole",
        "eventKindId": "feed_completed", "message": "Hole feed completed",
        "occurredAtMs": 1, "repeatedCount": 1, "confidence": "officer_verified",
        "sourceIds": ["hole_one"]
    }]);
    let decoded = CanonicalSnapshotEnvelope::decode_json(&encoded.to_string()).unwrap();
    assert_eq!(decoded.selected_colony_id.as_str(), "colony:home");
    assert_eq!(decoded.colonies[0].hunting_sites.len(), 1);
    assert_eq!(decoded.colonies[0].fishing_huts.len(), 1);
    assert_eq!(decoded.colonies[0].event_log.len(), 1);
    let twin =
        CanonicalSnapshotEnvelope::decode_json(&serde_json::to_string(&decoded).unwrap()).unwrap();
    assert_eq!(decoded, twin);
}

#[test]
fn snapshot_rejects_foreign_detail_unordered_summaries_and_hidden_regeneration() {
    let wrong_partition = snapshot(vec![colony("colony_alpha")], "colony_beta");
    assert_eq!(
        CanonicalSnapshotEnvelope::decode_json(&wrong_partition.to_string()),
        Err(CanonicalWireError::WrongPartition)
    );
    let foreign_detail = snapshot(
        vec![colony("colony_alpha"), colony("colony_beta")],
        "colony_alpha",
    );
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(&foreign_detail.to_string()),
        Err(CanonicalWireError::InvalidBounds("colonies"))
    ));
    let mut unordered = snapshot(vec![colony("colony_alpha")], "colony_alpha");
    unordered["publicColonies"] = json!([
        {
            "colonyId": "colony_beta", "displayName": "Beta",
            "canView": true, "canControl": false
        },
        {
            "colonyId": "colony_alpha", "displayName": "Alpha",
            "canView": true, "canControl": true
        }
    ]);
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(&unordered.to_string()),
        Err(CanonicalWireError::DuplicateOrUnordered(
            "public_colony_ids"
        ))
    ));
    let mut secret = colony("colony_alpha");
    secret["hole"]["officerReportedRegeneration"] = json!("exact server truth");
    let hidden = snapshot(vec![secret], "colony_alpha");
    assert!(CanonicalSnapshotEnvelope::decode_json(&hidden.to_string()).is_err());
    let mut level_four = colony("colony_alpha");
    level_four["hole"]["officerReportLevel"] = json!(4);
    level_four["hole"]["regeneration"] = json!("officer_reported_estimate");
    level_four["hole"]["officerReportedRegeneration"] = json!({
        "lowerUnitsPerDay": 8,
        "upperUnitsPerDay": 12,
        "observedAtMs": 1,
        "confidence": "officer_verified"
    });
    assert!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![level_four], "colony_alpha").to_string()
        )
        .is_ok()
    );
}

#[test]
fn level_four_may_be_unavailable_without_a_report_and_hole_footprint_is_exactly_five_by_five() {
    let mut no_report = colony("colony_alpha");
    no_report["hole"]["officerReportLevel"] = json!(4);
    assert!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![no_report], "colony_alpha").to_string()
        )
        .is_ok()
    );

    let mut marker_without_estimate = colony("colony_alpha");
    marker_without_estimate["hole"]["officerReportLevel"] = json!(4);
    marker_without_estimate["hole"]["regeneration"] = json!("officer_reported_estimate");
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![marker_without_estimate], "colony_alpha").to_string()
        ),
        Err(CanonicalWireError::InvalidBounds("regeneration"))
    ));

    let mut estimate_without_marker = colony("colony_alpha");
    estimate_without_marker["hole"]["officerReportLevel"] = json!(4);
    estimate_without_marker["hole"]["officerReportedRegeneration"] = json!({
        "lowerUnitsPerDay": 8,
        "upperUnitsPerDay": 12,
        "observedAtMs": 1,
        "confidence": "officer_verified"
    });
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![estimate_without_marker], "colony_alpha").to_string()
        ),
        Err(CanonicalWireError::InvalidBounds("regeneration"))
    ));

    let mut estimate_below_level_four = colony("colony_alpha");
    estimate_below_level_four["hole"]["regeneration"] = json!("officer_reported_estimate");
    estimate_below_level_four["hole"]["officerReportedRegeneration"] = json!({
        "lowerUnitsPerDay": 8,
        "upperUnitsPerDay": 12,
        "observedAtMs": 1,
        "confidence": "officer_verified"
    });
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![estimate_below_level_four], "colony_alpha").to_string()
        ),
        Err(CanonicalWireError::InvalidBounds("regeneration"))
    ));

    let mut duplicate = colony("colony_alpha");
    duplicate["hole"]["footprint"]["orderedTiles"][24] =
        duplicate["hole"]["footprint"]["orderedTiles"][0].clone();
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![duplicate], "colony_alpha").to_string()
        ),
        Err(CanonicalWireError::DuplicateOrUnordered("hole_footprint"))
    ));

    let mut wrong_shape = colony("colony_alpha");
    wrong_shape["hole"]["footprint"]["orderedTiles"][24] = json!({ "x": 9, "y": 9 });
    assert!(matches!(
        CanonicalSnapshotEnvelope::decode_json(
            &snapshot(vec![wrong_shape], "colony_alpha").to_string()
        ),
        Err(CanonicalWireError::InvalidBounds("hole_footprint"))
    ));
}

#[test]
fn action_header_is_rejected_before_unknown_payload_decode() {
    let incompatible = json!({
        "protocolVersion": 999,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_one", "payload": { "action": "definitely_bad" }
    });
    assert_eq!(
        CanonicalActionEnvelope::decode_json(&incompatible.to_string()),
        Err(CanonicalWireError::UnsupportedProtocolVersion(999))
    );
    let obsolete = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_one", "payload": { "action": "build_road" }
    });
    assert_eq!(
        CanonicalActionEnvelope::decode_json(&obsolete.to_string()),
        Err(CanonicalWireError::UnsupportedAction)
    );
    let micromanagement = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_one",
        "payload": { "action": "assign_worker", "catId": "cat_one", "taskId": "task_one" }
    });
    assert_eq!(
        CanonicalActionEnvelope::decode_json(&micromanagement.to_string()),
        Err(CanonicalWireError::UnsupportedAction)
    );
}

#[test]
fn actions_accept_real_ids_and_require_ordered_exact_version_lanes() {
    let valid = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony:home",
        "idempotencyId": "receipt_one",
        "expectedVersions": [{ "lane": "planner", "expectedVersion": 8 }],
        "payload": {
            "action": "broad_domain_nudge",
            "domain": "food",
            "basisPoints": 250
        }
    });
    let decoded = CanonicalActionEnvelope::decode_json(&valid.to_string()).unwrap();
    assert_eq!(
        CanonicalActionEnvelope::decode_json(&serde_json::to_string(&decoded).unwrap()).unwrap(),
        decoded
    );
    let duplicate_lanes = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_two",
        "expectedVersions": [{ "lane": "planner", "expectedVersion": 8 }, { "lane": "planner", "expectedVersion": 9 }],
        "payload": {
            "action": "broad_domain_nudge",
            "domain": "food", "basisPoints": 250
        }
    });
    assert!(CanonicalActionEnvelope::decode_json(&duplicate_lanes.to_string()).is_err());

    let planner_id_action = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony:home",
        "idempotencyId": "planner:v1|4:food",
        "expectedVersions": [{ "lane": "research", "expectedVersion": 2 }],
        "payload": { "action": "research_queue", "studyId": "study:food" }
    });
    CanonicalActionEnvelope::decode_json(&planner_id_action.to_string()).unwrap();
}

#[test]
fn broad_god_actions_have_typed_targets_and_bound_click_batches() {
    let invalid_clicks = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_three",
        "expectedVersions": [
            { "lane": "hole", "expectedVersion": 1 }, { "lane": "divine", "expectedVersion": 1 }, { "lane": "reservations", "expectedVersion": 1 }
        ],
        "payload": {
            "action": "hole_click_batch", "targetId": "hole_one",
            "requestedClicks": 0, "clientBatchWindowMs": 100
        }
    });
    assert!(CanonicalActionEnvelope::decode_json(&invalid_clicks.to_string()).is_err());

    let valid_clicks = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_four",
        "expectedVersions": [
            { "lane": "hole", "expectedVersion": 1 }, { "lane": "divine", "expectedVersion": 1 }, { "lane": "reservations", "expectedVersion": 1 }
        ],
        "payload": {
            "action": "hole_click_batch", "targetId": "hole_one",
            "requestedClicks": 64, "clientBatchWindowMs": 100
        }
    });
    CanonicalActionEnvelope::decode_json(&valid_clicks.to_string()).unwrap();

    let food_nudge = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_five",
        "expectedVersions": [
            { "lane": "planner", "expectedVersion": 1 },
            { "lane": "food_policy", "expectedVersion": 1 }
        ],
        "payload": { "action": "food_conservation", "nudgeBasisPoints": 500 }
    });
    CanonicalActionEnvelope::decode_json(&food_nudge.to_string()).unwrap();

    let rescue = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_six",
        "expectedVersions": [
            { "lane": "storage", "expectedVersion": 1 },
            { "lane": "divine", "expectedVersion": 1 },
            { "lane": "reservations", "expectedVersion": 1 }
        ],
        "payload": { "action": "emergency_rescue", "supply": "divine_water" }
    });
    CanonicalActionEnvelope::decode_json(&rescue.to_string()).unwrap();

    let backing = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "actionSchemaVersion": CANONICAL_ACTION_SCHEMA_VERSION,
        "authenticatedPlayerId": "player_one", "selectedColonyId": "colony_alpha",
        "idempotencyId": "receipt_seven",
        "expectedVersions": [{ "lane": "governance", "expectedVersion": 1 }],
        "payload": {
            "action": "candidate_backing",
            "electionId": "election_one", "candidateId": "cat_one"
        }
    });
    CanonicalActionEnvelope::decode_json(&backing.to_string()).unwrap();
}
