//! Focused LAI.3 acceptance tests for typed spatial objectives.

use cat_sim::{
    spatial_tasks::{
        OrderedTiles, Rect, ResourceSourceKind, SiteKind, SiteMetadata, SiteRef,
        SpatialBlockReason, SpatialInvariantError, SpatialObjective, SpatialRole, TaskFootprint,
        TilePoint, WorkSlot, canonical_building_footprint, footprint_for, footprint_tiles,
    },
    types::BuildingType,
};

fn point(x: i32, y: i32) -> TilePoint {
    TilePoint { x, y }
}

fn metadata(id: &str) -> SiteMetadata {
    SiteMetadata::revealed(id)
}

#[test]
fn ordered_tiles_are_canonical_row_major_and_deduplicated() {
    let tiles = OrderedTiles::canonical([
        point(8, 7),
        point(7, 8),
        point(7, 7),
        point(8, 7),
        point(8, 8),
    ]);

    assert_eq!(
        tiles.as_slice(),
        &[point(7, 7), point(8, 7), point(7, 8), point(8, 8)]
    );

    let restored: OrderedTiles = serde_json::from_value(serde_json::json!([
        { "x": 8, "y": 8 },
        { "x": 8, "y": 7 },
        { "x": 7, "y": 7 },
        { "x": 8, "y": 7 }
    ]))
    .unwrap();
    assert_eq!(
        restored.as_slice(),
        &[point(7, 7), point(8, 7), point(8, 8)]
    );
}

#[test]
fn malformed_rectangles_are_rejected_before_iteration_or_allocation() {
    assert!(Rect::new(point(0, 0), 0, 3).is_none());
    assert!(Rect::new(point(i32::MAX, 0), 2, 1).is_none());
    assert!(Rect::new(point(0, 0), 1025, 1025).is_none());

    let decoded = serde_json::from_value::<Rect>(serde_json::json!({
        "anchor": { "x": 0, "y": 0 },
        "width": 1025,
        "height": 1025
    }));
    assert!(decoded.is_err());
}

#[test]
fn workshop_is_exactly_three_by_three_and_nine_row_major_tiles() {
    let anchor = point(6, 6);
    let footprint = canonical_building_footprint(BuildingType::Workshop, anchor);

    assert_eq!(footprint_for(BuildingType::Workshop), (3, 3));
    assert_eq!((footprint.width, footprint.height), (3, 3));
    assert_eq!(
        footprint.tiles.as_slice(),
        &[
            point(6, 6),
            point(7, 6),
            point(8, 6),
            point(6, 7),
            point(7, 7),
            point(8, 7),
            point(6, 8),
            point(7, 8),
            point(8, 8),
        ]
    );
    assert_eq!(
        footprint_tiles(anchor, 3, 3),
        footprint.tiles.clone().into_vec()
    );
}

#[test]
fn resource_source_footprints_can_represent_a_two_by_three_tree() {
    let footprint = TaskFootprint::rectangular(
        Rect::new(point(20, 30), 2, 3).expect("tree footprint is non-empty"),
    );
    let tree = SiteRef::ResourceSource {
        metadata: metadata("tree:20:30"),
        source_id: "tree:20:30".to_owned(),
        resource_kind: ResourceSourceKind::Tree,
        footprint: footprint.clone(),
    };

    assert_eq!((footprint.width, footprint.height), (2, 3));
    assert_eq!(footprint.tiles.len(), 6);
    assert_eq!(
        footprint.tiles.as_slice(),
        &[
            point(20, 30),
            point(21, 30),
            point(20, 31),
            point(21, 31),
            point(20, 32),
            point(21, 32),
        ]
    );
    assert_eq!(tree.footprint(), Some(&footprint));
}

#[test]
fn every_site_ref_family_has_a_stable_id_and_kind() {
    let rect = Rect::new(point(2, 3), 2, 2).unwrap();
    let footprint = TaskFootprint::rectangular(rect);
    let ordered = OrderedTiles::canonical([point(3, 3), point(2, 3)]);
    let sites = vec![
        SiteRef::Tile {
            metadata: metadata("tile:2:3"),
            tile: point(2, 3),
        },
        SiteRef::Rect {
            metadata: metadata("rect:2:3:2:2"),
            rect,
            footprint: footprint.clone(),
        },
        SiteRef::OrderedTiles {
            metadata: metadata("tiles:accounting-round-7"),
            tiles: ordered.clone(),
        },
        SiteRef::building("building:workshop-1", BuildingType::Workshop, point(6, 6)),
        SiteRef::Stockpile {
            metadata: metadata("stockpile:food-1"),
            stockpile_id: "food-1".to_owned(),
            footprint: footprint.clone(),
        },
        SiteRef::ResourceSource {
            metadata: metadata("source:tree-1"),
            source_id: "tree-1".to_owned(),
            resource_kind: ResourceSourceKind::Tree,
            footprint: footprint.clone(),
        },
        SiteRef::OrderedRoute {
            metadata: metadata("route:scout-1"),
            route: vec![point(2, 3), point(3, 3)],
        },
        SiteRef::Shrine {
            metadata: metadata("shrine:colony-1"),
            building_id: "shrine-1".to_owned(),
            anchor: point(6, 6),
            footprint: canonical_building_footprint(BuildingType::Shrine, point(6, 6)),
        },
        SiteRef::VillageTradeEndpoint {
            metadata: metadata("village:colony-2"),
            colony_id: "colony-2".to_owned(),
            footprint,
        },
    ];

    assert_eq!(
        sites.iter().map(SiteRef::kind).collect::<Vec<_>>(),
        vec![
            SiteKind::Tile,
            SiteKind::Rect,
            SiteKind::OrderedTiles,
            SiteKind::Building,
            SiteKind::Stockpile,
            SiteKind::ResourceSource,
            SiteKind::OrderedRoute,
            SiteKind::Shrine,
            SiteKind::VillageTradeEndpoint,
        ]
    );
    assert!(sites.iter().all(|site| !site.stable_id().is_empty()));
    assert!(sites.iter().all(|site| site.validate().is_ok()));
}

#[test]
fn spatial_objective_preserves_objective_work_and_delivery_roles() {
    let objective = SiteRef::building("building:workshop-1", BuildingType::Workshop, point(6, 6));
    let work_site = SiteRef::Tile {
        metadata: metadata("work:workshop-1:slot-0"),
        tile: point(6, 7),
    };
    let endpoint = SiteRef::Stockpile {
        metadata: metadata("stockpile:workshop-output-1"),
        stockpile_id: "workshop-output-1".to_owned(),
        footprint: TaskFootprint::rectangular(Rect::new(point(9, 6), 1, 1).unwrap()),
    };
    let spatial = SpatialObjective::resolved(
        objective,
        vec![WorkSlot::exclusive("workshop-1:slot-0", work_site)],
        Some(endpoint),
    );

    assert_eq!(
        spatial
            .site_for_role(SpatialRole::Objective, 0)
            .unwrap()
            .kind(),
        SiteKind::Building
    );
    assert_eq!(
        spatial
            .site_for_role(SpatialRole::WorkPosition, 0)
            .unwrap()
            .kind(),
        SiteKind::Tile
    );
    assert_eq!(
        spatial
            .site_for_role(SpatialRole::DeliveryEndpoint, 0)
            .unwrap()
            .kind(),
        SiteKind::Stockpile
    );
    assert_eq!(spatial.footprint().unwrap().tiles.len(), 9);
    assert_eq!(spatial.blocked_reason, None);

    let blocked = SpatialObjective::blocked(SpatialBlockReason::SourceUnavailable);
    assert!(blocked.objective.is_none());
    assert!(blocked.work_positions.is_empty());
    assert_eq!(
        blocked.blocked_reason,
        Some(SpatialBlockReason::SourceUnavailable)
    );
}

#[test]
fn work_slots_and_redundant_site_payloads_fail_closed_validation() {
    let work_site = SiteRef::Tile {
        metadata: metadata("work:tree-1"),
        tile: point(4, 5),
    };
    assert_eq!(
        WorkSlot::capacity("tree-1:capacity", work_site, 0),
        Err(SpatialInvariantError::ZeroWorkSlotCapacity)
    );

    let rect = Rect::new(point(2, 3), 2, 2).unwrap();
    let malformed_rect = SiteRef::Rect {
        metadata: metadata("rect:mismatch"),
        rect,
        footprint: TaskFootprint::rectangular(Rect::new(point(2, 3), 1, 1).unwrap()),
    };
    let restored: SiteRef =
        serde_json::from_value(serde_json::to_value(malformed_rect).unwrap()).unwrap();
    assert_eq!(
        restored.validate(),
        Err(SpatialInvariantError::RectFootprintMismatch)
    );

    let malformed_building = SiteRef::Building {
        metadata: metadata("building:bad-workshop"),
        building_id: "bad-workshop".to_owned(),
        building_type: BuildingType::Workshop,
        anchor: point(6, 6),
        footprint: TaskFootprint::rectangular(Rect::new(point(6, 6), 2, 3).unwrap()),
    };
    assert_eq!(
        malformed_building.validate(),
        Err(SpatialInvariantError::BuildingFootprintMismatch)
    );
}
