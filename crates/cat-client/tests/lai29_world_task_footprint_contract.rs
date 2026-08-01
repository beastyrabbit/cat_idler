//! LAI.29A red contract for authoritative world-task footprint rendering.
//!
//! These tests intentionally assert on missing future client symbols. They are a
//! TDD characterization for the LAI.29 production owner and must not be turned
//! green by local shims in this test target.

const CLIENT: &str = concat!(
    include_str!("../src/lib.rs"),
    include_str!("../src/leader_ai_ui/mod.rs"),
    include_str!("../src/leader_ai_ui/task_footprints.rs")
);
const SPATIAL_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/spatial-task-contract.md");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");

fn missing_markers(source: &str, markers: &[&str]) -> Vec<String> {
    markers
        .iter()
        .filter(|marker| !source.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect()
}

fn present_forbidden(source: &str, markers: &[&str]) -> Vec<String> {
    markers
        .iter()
        .filter(|marker| source.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect()
}

fn assert_contract_docs(marker: &str) {
    assert!(
        SPATIAL_DOC.contains(marker),
        "spatial-task-contract.md is missing LAI.29 contract marker {marker}"
    );
    assert!(
        TESTING_DOC.contains(marker),
        "testing-cutover.md is missing LAI.29 browser/test marker {marker}"
    );
}

fn assert_client_has(test_name: &str, markers: &[&str]) {
    let missing = missing_markers(CLIENT, markers);
    assert!(
        missing.is_empty(),
        "{test_name} is still red: cat-client production UI lacks {}",
        missing.join(", ")
    );
}

fn assert_client_forbids(test_name: &str, markers: &[&str]) {
    let present = present_forbidden(CLIENT, markers);
    assert!(
        present.is_empty(),
        "{test_name} found forbidden fallback/leak marker(s): {}",
        present.join(", ")
    );
}

#[test]
fn visible_task_markers_are_snapshot_only_and_strict_siterefs() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "visible_task_markers_are_snapshot_only_and_strict_siterefs",
        &[
            "VisibleTaskMarkerPlugin",
            "VisibleTaskSnapshotMarkerSource",
            "StrictSiteRefMarkerResolver",
            "TaskMarkerEntity",
            "TaskMarkerKind::Objective",
            "TaskMarkerKind::WorkSlot",
            "TaskMarkerKind::Endpoint",
            "TaskMarkerKind::FootprintCell",
            "NoCatDestinationAuthorityForTaskMarkers",
        ],
    );
}

#[test]
fn hunt_and_fetch_water_render_actual_objective_work_and_endpoint_sites() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "hunt_and_fetch_water_render_actual_objective_work_and_endpoint_sites",
        &[
            "render_hunt_objective_from_revealed_hunting_source",
            "HuntObjectiveCaveOrSourceMarker",
            "render_fetch_water_source_bank_endpoint",
            "FetchWaterSourceMarker",
            "FetchWaterDryBankWorkMarker",
            "FetchWaterPinnedDeliveryEndpointMarker",
            "WaterSourceIsNotWalkableWorkPosition",
            "BlockedOrUnreachableSiteSuppressesWorldMarker",
        ],
    );
}

#[test]
fn workshop_and_tree_footprints_render_all_canonical_cells() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "workshop_and_tree_footprints_render_all_canonical_cells",
        &[
            "render_workshop_three_by_three_objective_cells",
            "WorkshopObjectiveNineRowMajorCells",
            "WorkshopDistinctWorkSlotMarker",
            "WorkshopDistinctDeliveryEndpointMarker",
            "render_tree_six_canonical_footprint_cells",
            "TreeObjectiveSixCanonicalCells",
            "CanonicalFootprintCellIndex",
            "NoDuplicatedWorkshopSizeConstant",
        ],
    );
}

#[test]
fn snapshot_id_keyed_dedupe_update_and_despawn_are_authoritative() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "snapshot_id_keyed_dedupe_update_and_despawn_are_authoritative",
        &[
            "TaskSnapshotIdMarkerKey",
            "DedupeVisibleTaskMarkerBySnapshotId",
            "UpdateVisibleTaskMarkerFromSnapshotVersion",
            "DespawnRemovedVisibleTaskMarkers",
            "NoStaleTaskMarkerReuse",
            "NoDuplicateCoincidentTaskMarker",
            "SemanticSiteStageDedupeKey",
            "VisibleTaskRemovalEvent",
        ],
    );
}

#[test]
fn redacted_blocked_missing_or_foreign_tasks_emit_no_markers() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "redacted_blocked_missing_or_foreign_tasks_emit_no_markers",
        &[
            "RedactedVisibleTaskNoMarker",
            "ObjectiveLessBlockedTaskNoMapEntity",
            "MissingSiteRefNoMarker",
            "BlockedSiteRefNoMarker",
            "ForeignColonyVisibleTaskNoMarker",
            "SelectedColonyTaskMarkerFilter",
            "MultiColonyTaskMarkerIsolation",
            "ReportSafeTaskMarkerVisibility",
        ],
    );
}

#[test]
fn route_endpoint_and_work_marker_accessibility_ids_are_stable() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "route_endpoint_and_work_marker_accessibility_ids_are_stable",
        &[
            "TASK_MARKER_OBJECTIVE_TEST_ID",
            "TASK_MARKER_WORK_SLOT_TEST_ID",
            "TASK_MARKER_ENDPOINT_TEST_ID",
            "TASK_MARKER_CELL_TEST_ID",
            "ACCESSIBLE_TASK_OBJECTIVE_LABEL",
            "ACCESSIBLE_TASK_WORK_SLOT_LABEL",
            "ACCESSIBLE_TASK_ENDPOINT_LABEL",
            "RouteContactMarkerIsNotDeliveryEndpoint",
        ],
    );
}

#[test]
fn zoom_viewport_and_visible_browser_checkpoints_are_defined() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "zoom_viewport_and_visible_browser_checkpoints_are_defined",
        &[
            "TaskMarkerSupportedZoomRange",
            "TaskMarkerViewportCullingKeepsAuthoritativeIds",
            "TaskMarkerScreenBoundsGuard",
            "PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST",
            "VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT",
            "VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER",
            "VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE",
            "VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION",
        ],
    );
}

#[test]
fn tooltips_are_report_safe_and_fallbacks_are_absent() {
    assert_contract_docs("LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT");
    assert_client_has(
        "tooltips_are_report_safe_and_fallbacks_are_absent",
        &[
            "TaskMarkerReportSafeTooltip",
            "TaskMarkerTooltipRedactionGuard",
            "NoHiddenStockTooltipField",
            "NoExactRegenerationBelowLevelFourTooltip",
            "NoPrivateBeliefOrPlanTooltip",
            "NoRadialTaskMarkerFallback",
            "NoGenericTaskDestinationFallback",
            "NoClientSideSiteGuessing",
        ],
    );
    assert_client_forbids(
        "tooltips_are_report_safe_and_fallbacks_are_absent",
        &[
            "radial task marker fallback",
            "generic task destination fallback",
            "hidden stock tooltip",
            "exact regeneration tooltip",
            "private belief tooltip",
            "foreign colony private task marker",
        ],
    );
}
