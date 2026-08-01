use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    TaskFootprintProjectionError, TaskMarkerKind, TaskMarkerSpecialization, VisibleTaskMarkerInput,
    VisibleTaskMarkerPlugin, VisibleTaskMarkerWorld, VisibleTaskSnapshotMarkerSource,
    project_visible_task_footprint, project_visible_task_footprints,
};
use cat_protocol::{
    BoundedBasisPoints, NonEmptyStableId, ReportSafeString, ReservationSummarySnapshot,
    SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot, SiteVisibilitySnapshot,
    SnapshotTilePoint as TilePoint, TaskCargoSnapshot, VisibleTaskSnapshot, WorkSlotSnapshot,
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

fn site(site_id: &str) -> SiteSnapshot {
    SiteSnapshot {
        site_id: id(site_id),
        visibility: SiteVisibilitySnapshot::Visible,
        lifecycle_stage: SiteLifecycleStageSnapshot::Active,
        blocked_reason: None,
    }
}

fn blocked_site(site_id: &str) -> SiteSnapshot {
    SiteSnapshot {
        site_id: id(site_id),
        visibility: SiteVisibilitySnapshot::Visible,
        lifecycle_stage: SiteLifecycleStageSnapshot::Blocked,
        blocked_reason: Some(text("blocked by report")),
    }
}

fn work_slot(slot_id: &str, tile: TilePoint) -> WorkSlotSnapshot {
    WorkSlotSnapshot {
        slot_id: id(slot_id),
        tile,
        state: text("assigned"),
    }
}

fn task(
    task_id: &str,
    category: &str,
    objective: SiteRefSnapshot,
    work_slots: Vec<WorkSlotSnapshot>,
    endpoint: Option<SiteRefSnapshot>,
    footprint: Vec<TilePoint>,
) -> VisibleTaskSnapshot {
    VisibleTaskSnapshot {
        task_id: id(task_id),
        intent_id: id(&format!("intent:{task_id}")),
        category: text(category),
        stage: text("hauling"),
        assigned_cat_ids: vec![id("cat:mallow"), id("cat:sedge")],
        objective,
        work_slots,
        endpoint,
        footprint,
        progress_basis_points: bp(2_500),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:one")],
            reservation_version: 7,
        },
        blocked_reason: None,
        cargo: TaskCargoSnapshot {
            cargo_ids: Vec::new(),
            summary: text("none"),
        },
        last_updated_at_ms: 42_000,
    }
}

fn workshop_tiles(anchor: TilePoint) -> Vec<TilePoint> {
    (0..3)
        .flat_map(|dy| {
            (0..3).map(move |dx| TilePoint {
                x: anchor.x + dx,
                y: anchor.y + dy,
            })
        })
        .collect()
}

#[test]
fn hunt_projection_uses_authoritative_hunt_source_tile_and_identity() {
    let hunt = task(
        "task:hunt:1",
        "Hunt",
        SiteRefSnapshot::HuntSource {
            site: site("site:cave:revealed"),
            cave_id: id("cave:revealed"),
            source_tile: tile(31, -4),
        },
        Vec::new(),
        None,
        Vec::new(),
    );

    let projection = project_visible_task_footprint(&hunt)
        .expect("projection succeeds")
        .expect("visible hunt renders");

    assert_eq!(projection.task_id, "task:hunt:1");
    assert_eq!(projection.stage, "hauling");
    assert_eq!(projection.assigned_cat_ids, ["cat:mallow", "cat:sedge"]);
    assert_eq!(projection.markers.len(), 1);
    let marker = &projection.markers[0];
    assert_eq!(marker.kind, TaskMarkerKind::Objective);
    assert_eq!(
        marker.specialization,
        TaskMarkerSpecialization::HuntObjectiveCaveOrSource
    );
    assert_eq!(marker.tile, tile(31, -4));
    assert_eq!(marker.key.site_id, "cave:revealed");
    assert_eq!(
        marker.test_id.as_str(),
        "lai-ui:tasks:task:task:hunt:1:site:cave:revealed:objective"
    );
    assert_eq!(marker.label.as_str(), "Hunt objective, hunt source");
}

#[test]
fn fetch_water_projection_separates_source_bank_and_delivery_endpoint() {
    let water = task(
        "task:water:1",
        "Fetch Water",
        SiteRefSnapshot::WaterSourceAndBank {
            site: site("site:river:north"),
            source_tile: tile(3, 9),
            bank_tile: tile(4, 9),
        },
        vec![work_slot("slot:river-bank", tile(4, 9))],
        Some(SiteRefSnapshot::VillageEndpoint {
            site: site("site:village:stores"),
            colony_id: id("colony:one"),
            endpoint: tile(12, 15),
        }),
        Vec::new(),
    );

    let projection = project_visible_task_footprint(&water)
        .expect("projection succeeds")
        .expect("visible water task renders");

    assert_eq!(projection.markers.len(), 3);
    let objective = projection
        .markers
        .iter()
        .find(|marker| marker.kind == TaskMarkerKind::Objective)
        .expect("water source marker");
    assert_eq!(objective.tile, tile(3, 9));
    assert_eq!(
        objective.specialization,
        TaskMarkerSpecialization::FetchWaterSource
    );

    let work = projection
        .markers
        .iter()
        .find(|marker| marker.kind == TaskMarkerKind::WorkSlot)
        .expect("dry bank work marker");
    assert_eq!(work.tile, tile(4, 9));
    assert_eq!(work.key.site_id, "slot:river-bank");
    assert_ne!(work.tile, objective.tile);
    assert_eq!(
        work.specialization,
        TaskMarkerSpecialization::FetchWaterDryBankWork
    );

    let endpoint = projection
        .markers
        .iter()
        .find(|marker| marker.kind == TaskMarkerKind::Endpoint)
        .expect("pinned delivery endpoint");
    assert_eq!(endpoint.tile, tile(12, 15));
    assert_eq!(
        endpoint.specialization,
        TaskMarkerSpecialization::FetchWaterPinnedDeliveryEndpoint
    );
    assert_eq!(
        endpoint.label.as_str(),
        "Fetch Water delivery endpoint, delivery endpoint"
    );
}

#[test]
fn workshop_projection_emits_exact_nine_row_major_cells_plus_work_and_endpoint() {
    let anchor = tile(-6, 20);
    let ordered_tiles = workshop_tiles(anchor);
    let workshop = task(
        "task:workshop:1",
        "Build Workshop",
        SiteRefSnapshot::BuildingFootprint {
            site: site("site:workshop"),
            building_id: id("building:workshop"),
            building_kind: text("workshop"),
            anchor,
            width: 3,
            height: 3,
            ordered_tiles: ordered_tiles.clone(),
        },
        vec![work_slot("slot:workshop:center", tile(-5, 21))],
        Some(SiteRefSnapshot::Tile {
            site: site("site:delivery:logs"),
            tile: tile(-7, 21),
        }),
        ordered_tiles.clone(),
    );

    let projection = project_visible_task_footprint(&workshop)
        .expect("projection succeeds")
        .expect("visible workshop renders");

    let cells = projection
        .markers
        .iter()
        .filter_map(|marker| match marker.kind {
            TaskMarkerKind::FootprintCell(index) => Some((index.get(), marker.tile)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 9);
    assert_eq!(
        cells,
        ordered_tiles
            .iter()
            .enumerate()
            .map(|(index, tile)| (u8::try_from(index).unwrap(), *tile))
            .collect::<Vec<_>>()
    );
    assert!(projection.markers.iter().any(|marker| {
        marker.kind == TaskMarkerKind::WorkSlot
            && marker.tile == tile(-5, 21)
            && marker.specialization == TaskMarkerSpecialization::WorkshopDistinctWorkSlot
    }));
    assert!(projection.markers.iter().any(|marker| {
        marker.kind == TaskMarkerKind::Endpoint
            && marker.tile == tile(-7, 21)
            && marker.specialization == TaskMarkerSpecialization::WorkshopDistinctDeliveryEndpoint
    }));
}

#[test]
fn strict_projection_never_falls_back_to_default_or_generic_coordinates() {
    let generic_hunt = task(
        "task:hunt:bad",
        "Hunt",
        SiteRefSnapshot::Tile {
            site: site("site:generic"),
            tile: tile(0, 0),
        },
        Vec::new(),
        None,
        Vec::new(),
    );
    assert!(matches!(
        project_visible_task_footprint(&generic_hunt),
        Err(TaskFootprintProjectionError::UnsupportedSiteRef {
            expected: "HuntSource",
            ..
        })
    ));

    let missing_bank_slot = task(
        "task:water:bad",
        "fetch_water",
        SiteRefSnapshot::WaterSourceAndBank {
            site: site("site:river"),
            source_tile: tile(1, 1),
            bank_tile: tile(2, 1),
        },
        Vec::new(),
        Some(SiteRefSnapshot::Tile {
            site: site("site:delivery"),
            tile: tile(5, 5),
        }),
        Vec::new(),
    );
    assert!(matches!(
        project_visible_task_footprint(&missing_bank_slot),
        Err(TaskFootprintProjectionError::MissingDryBankWorkSlot { .. })
    ));

    let blocked_hunt = task(
        "task:hunt:blocked",
        "Hunt",
        SiteRefSnapshot::HuntSource {
            site: blocked_site("site:cave:blocked"),
            cave_id: id("cave:blocked"),
            source_tile: tile(99, 99),
        },
        Vec::new(),
        None,
        Vec::new(),
    );
    assert_eq!(
        project_visible_task_footprint(&blocked_hunt).expect("suppression is not a leak"),
        None
    );
}

#[test]
fn batch_projection_preserves_task_stage_and_suppresses_redacted_markers() {
    let visible = task(
        "task:hunt:visible",
        "hunt",
        SiteRefSnapshot::HuntSource {
            site: site("site:cave:visible"),
            cave_id: id("cave:visible"),
            source_tile: tile(7, 8),
        },
        Vec::new(),
        None,
        Vec::new(),
    );
    let reported_only = task(
        "task:hunt:reported",
        "hunt",
        SiteRefSnapshot::HuntSource {
            site: SiteSnapshot {
                site_id: id("site:cave:reported"),
                visibility: SiteVisibilitySnapshot::Reported,
                lifecycle_stage: SiteLifecycleStageSnapshot::Active,
                blocked_reason: None,
            },
            cave_id: id("cave:reported"),
            source_tile: tile(70, 80),
        },
        Vec::new(),
        None,
        Vec::new(),
    );

    let projections = project_visible_task_footprints(VisibleTaskSnapshotMarkerSource {
        tasks: &[visible, reported_only],
    })
    .expect("batch projection succeeds");

    assert_eq!(projections.len(), 1);
    assert_eq!(projections[0].task_id, "task:hunt:visible");
    assert_eq!(projections[0].markers[0].stage, "hauling");
    assert_eq!(
        projections[0].markers[0].assigned_cat_ids,
        ["cat:mallow", "cat:sedge"]
    );
    assert_eq!(projections[0].markers[0].tile, tile(7, 8));
}

#[test]
fn tree_projection_emits_exact_six_canonical_cells_from_resource_source() {
    let ordered_tiles = vec![
        tile(20, 1),
        tile(21, 1),
        tile(20, 2),
        tile(21, 2),
        tile(20, 3),
        tile(21, 3),
    ];
    let tree = task(
        "task:tree:1",
        "Logging",
        SiteRefSnapshot::ResourceSource {
            site: site("site:tree:old-oak"),
            source_id: id("tree:old-oak"),
            resource_kind: text("tree"),
            ordered_tiles: ordered_tiles.clone(),
        },
        vec![work_slot("slot:tree:perimeter", tile(19, 2))],
        Some(SiteRefSnapshot::Tile {
            site: site("site:stockpile:logs"),
            tile: tile(12, 7),
        }),
        ordered_tiles.clone(),
    );

    let projection = project_visible_task_footprint(&tree)
        .expect("projection succeeds")
        .expect("tree task renders");

    let cells = projection
        .markers
        .iter()
        .filter_map(|marker| match marker.kind {
            TaskMarkerKind::FootprintCell(index) => Some((index.get(), marker.tile)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cells,
        ordered_tiles
            .iter()
            .enumerate()
            .map(|(index, tile)| (u8::try_from(index).unwrap(), *tile))
            .collect::<Vec<_>>()
    );
    assert!(projection.markers.iter().any(|marker| {
        marker.kind == TaskMarkerKind::WorkSlot
            && marker.specialization == TaskMarkerSpecialization::TreeObjectiveSixCanonicalCells
            && marker.tile == tile(19, 2)
    }));
    assert!(projection.markers.iter().any(|marker| {
        marker.kind == TaskMarkerKind::Endpoint
            && marker.specialization == TaskMarkerSpecialization::TreeObjectiveSixCanonicalCells
            && marker.tile == tile(12, 7)
    }));
}

#[test]
fn road_projection_preserves_authoritative_route_order_and_endpoint_distinction() {
    let route = vec![tile(-2, -2), tile(-1, -2), tile(0, -2), tile(0, -1)];
    let road = task(
        "task:road:1",
        "Road Construction",
        SiteRefSnapshot::OrderedRoute {
            site: site("site:road:north"),
            route_id: id("route:north"),
            ordered_tiles: route.clone(),
        },
        vec![work_slot("slot:road:next", tile(-1, -2))],
        Some(SiteRefSnapshot::VillageEndpoint {
            site: site("site:village:gate"),
            colony_id: id("colony:one"),
            endpoint: tile(0, 0),
        }),
        route.clone(),
    );

    let projection = project_visible_task_footprint(&road)
        .expect("projection succeeds")
        .expect("road task renders");

    let cells = projection
        .markers
        .iter()
        .filter_map(|marker| match marker.kind {
            TaskMarkerKind::FootprintCell(index) => Some((index.get(), marker.tile)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cells,
        route
            .iter()
            .enumerate()
            .map(|(index, tile)| (u8::try_from(index).unwrap(), *tile))
            .collect::<Vec<_>>()
    );
    let work = projection
        .markers
        .iter()
        .find(|marker| marker.kind == TaskMarkerKind::WorkSlot)
        .expect("road work marker");
    let endpoint = projection
        .markers
        .iter()
        .find(|marker| marker.kind == TaskMarkerKind::Endpoint)
        .expect("road endpoint marker");
    assert_eq!(work.tile, tile(-1, -2));
    assert_eq!(endpoint.tile, tile(0, 0));
    assert_ne!(work.kind, endpoint.kind);
}

#[test]
fn visible_task_marker_plugin_updates_dedupes_despawns_and_filters_foreign_colony() {
    let visible = task(
        "task:hunt:visible",
        "hunt",
        SiteRefSnapshot::HuntSource {
            site: site("site:cave:visible"),
            cave_id: id("cave:visible"),
            source_tile: tile(7, 8),
        },
        Vec::new(),
        None,
        Vec::new(),
    );

    let mut app = App::new();
    app.add_plugins(VisibleTaskMarkerPlugin);
    app.insert_resource(VisibleTaskMarkerInput {
        selected_colony_id: Some("colony:one".to_string()),
        colony_id: Some("colony:one".to_string()),
        tasks: vec![visible],
    });
    app.update();

    let world = app.world().resource::<VisibleTaskMarkerWorld>();
    assert_eq!(world.markers.len(), 1);
    assert!(world.last_error.is_none());
    let retained = world
        .retained_keys
        .iter()
        .next()
        .expect("retained key")
        .clone();

    app.insert_resource(VisibleTaskMarkerInput {
        selected_colony_id: Some("colony:one".to_string()),
        colony_id: Some("colony:one".to_string()),
        tasks: Vec::new(),
    });
    app.update();
    let world = app.world().resource::<VisibleTaskMarkerWorld>();
    assert!(world.markers.is_empty());
    assert_eq!(world.removed_keys, vec![retained]);

    app.insert_resource(VisibleTaskMarkerInput {
        selected_colony_id: Some("colony:two".to_string()),
        colony_id: Some("colony:one".to_string()),
        tasks: vec![task(
            "task:hunt:foreign",
            "hunt",
            SiteRefSnapshot::HuntSource {
                site: site("site:cave:foreign"),
                cave_id: id("cave:foreign"),
                source_tile: tile(70, 80),
            },
            Vec::new(),
            None,
            Vec::new(),
        )],
    });
    app.update();
    let world = app.world().resource::<VisibleTaskMarkerWorld>();
    assert!(world.markers.is_empty());
    assert!(world.retained_keys.is_empty());
}

#[test]
fn duplicate_snapshot_marker_keys_error_and_objective_less_blocked_tasks_emit_zero_markers() {
    let anchor = tile(1, 1);
    let ordered_tiles = workshop_tiles(anchor);
    let duplicate_work = task(
        "task:workshop:duplicate",
        "Build Workshop",
        SiteRefSnapshot::BuildingFootprint {
            site: site("site:workshop:duplicate"),
            building_id: id("building:workshop:duplicate"),
            building_kind: text("workshop"),
            anchor,
            width: 3,
            height: 3,
            ordered_tiles: ordered_tiles.clone(),
        },
        vec![
            work_slot("slot:workshop:duplicate", tile(2, 2)),
            work_slot("slot:workshop:duplicate", tile(2, 2)),
        ],
        None,
        ordered_tiles,
    );
    assert!(matches!(
        project_visible_task_footprint(&duplicate_work),
        Err(TaskFootprintProjectionError::DuplicateMarkerKey(_))
    ));

    let mut blocked = task(
        "task:blocked:objective-less",
        "Unknown blocked task",
        SiteRefSnapshot::Tile {
            site: blocked_site("site:redacted-objective"),
            tile: tile(0, 0),
        },
        Vec::new(),
        None,
        Vec::new(),
    );
    blocked.blocked_reason = Some(text("objective hidden by report"));
    assert_eq!(
        project_visible_task_footprint(&blocked).expect("blocked task suppresses markers"),
        None
    );
}
