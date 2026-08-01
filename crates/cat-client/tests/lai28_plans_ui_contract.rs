//! LAI.28A red contract for the post-cutover Plans and standing-orders UI.
//!
//! These tests intentionally assert on missing future client symbols. They are a
//! TDD characterization for the LAI.28 production owner and must not be turned
//! green by local shims in this test target.

const CLIENT: &str = include_str!("../src/lib.rs");
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");

fn missing_markers(source: &str, markers: &[&str]) -> Vec<String> {
    markers
        .iter()
        .filter(|marker| !source.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect()
}

fn assert_contract_docs(marker: &str) {
    assert!(
        WIRE_DOC.contains(marker),
        "wire-persistence-ui.md is missing LAI.28 contract marker {marker}"
    );
    assert!(
        TESTING_DOC.contains(marker),
        "testing-cutover.md is missing LAI.28 browser/test marker {marker}"
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

#[test]
fn plans_panel_renders_authoritative_top_eight_report_safe_rows() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "plans_panel_renders_authoritative_top_eight_report_safe_rows",
        &[
            "PlansPanelPlugin",
            "PlansPanelRoot",
            "render_authoritative_top_eight_plans",
            "PlanRowStableId",
            "PlanLifecycleStatusLabel",
            "PlanResponsibleActorLabel",
            "PlanDependencyList",
            "PlanBoundedRationale",
            "PlanScoreConfidenceRange",
            "PlanReportAgeBadge",
            "PlanReportProvenanceList",
            "PlansNoHiddenTruthGuard",
        ],
    );
}

#[test]
fn plans_panel_exposes_accessible_nudge_dismiss_and_domain_controls() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "plans_panel_exposes_accessible_nudge_dismiss_and_domain_controls",
        &[
            "MovePlanUpButton",
            "MovePlanDownButton",
            "DismissPlanButton",
            "DomainNudgeControl",
            "PLAN_NUDGE_UP_DELTA_BP_1500",
            "PLAN_NUDGE_DOWN_DELTA_BP_NEG_1500",
            "accessibility_label_move_plan_up",
            "accessibility_label_move_plan_down",
            "accessibility_label_dismiss_plan",
            "PlanControlDisabledReason",
        ],
    );
}

#[test]
fn standing_orders_enforce_administration_slots_and_bounded_feedback() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "standing_orders_enforce_administration_slots_and_bounded_feedback",
        &[
            "StandingOrdersPanel",
            "StandingOrderCreateButton",
            "StandingOrderEditButton",
            "StandingOrderRemoveButton",
            "AdministrationSlotMeter",
            "AdministrationSlotLimitReached",
            "StandingOrderBoundedFeedback",
            "StandingOrderPolicyDomainPicker",
            "StandingOrderDoesNotBypassKnowledgeOrPhysicalRules",
        ],
    );
}

#[test]
fn plan_actions_send_authenticated_expected_version_idempotency_payloads() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "plan_actions_send_authenticated_expected_version_idempotency_payloads",
        &[
            "build_leader_ai_action_envelope",
            "LeaderAiPlanNudgeAction",
            "LeaderAiStandingOrderAction",
            "StableIdempotencyId",
            "AuthenticatedPlayerIdentity",
            "ExpectedPlannerVersion",
            "ExpectedDomainVersion",
            "ExpectedResourceVersion",
            "ExpectedReservationVersion",
            "send_expected_version_action",
        ],
    );
}

#[test]
fn stale_actions_refresh_and_preserve_context_while_removed_plans_despawn() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "stale_actions_refresh_and_preserve_context_while_removed_plans_despawn",
        &[
            "PlanActionConflictRefresh",
            "VersionMismatchRefreshHandler",
            "PreservePlansPanelFocusAfterRefresh",
            "PreserveStandingOrderDraftAfterRefresh",
            "DespawnsUnknownPlanRows",
            "RemovedPlanControlsAreDisabled",
            "DuplicateReplayUsesOriginalResult",
            "BoundedPlanConflictToast",
        ],
    );
}

#[test]
fn equal_nudges_and_stale_plan_ordering_are_deterministic() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "equal_nudges_and_stale_plan_ordering_are_deterministic",
        &[
            "DeterministicPlanRowOrder",
            "EqualNudgesDoNotStack",
            "OppositeNudgeReplacesPrior",
            "StablePlanTieBreakKey",
            "CurrentPlanningEpochOnly",
            "NoStalePlanControlReuse",
        ],
    );
}

#[test]
fn officer_reports_vacancies_authority_and_regeneration_gate_are_visible() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "officer_reports_vacancies_authority_and_regeneration_gate_are_visible",
        &[
            "OfficerReportPanel",
            "OfficerVacancySlot",
            "OfficerAuthorityBadge",
            "OfficerRequestReasonList",
            "LeaderResponsibleActorBadge",
            "RegenerationUnavailableBelowReportLevel4",
            "EffectiveReportLevelGate",
            "NoClientRegenerationFallback",
        ],
    );
}

#[test]
fn playwright_and_visible_browser_contracts_have_stable_accessibility_targets() {
    assert_contract_docs("LAI.28_PLANS_UI_CONTRACT");
    assert_client_has(
        "playwright_and_visible_browser_contracts_have_stable_accessibility_targets",
        &[
            "ACCESSIBLE_PLANS_PANEL_LABEL",
            "ACCESSIBLE_STANDING_ORDERS_PANEL_LABEL",
            "PLAN_ROW_TEST_ID_PREFIX",
            "STANDING_ORDER_ROW_TEST_ID_PREFIX",
            "PLAN_CONTROL_TEST_ID_PREFIX",
            "OFFICER_REPORT_TEST_ID_PREFIX",
            "VISIBLE_BROWSER_CHECKPOINT_PLANS_TOP_EIGHT",
            "PLAYWRIGHT_NO_DOM_STATE_INJECTION",
        ],
    );
}
