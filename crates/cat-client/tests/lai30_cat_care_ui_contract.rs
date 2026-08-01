//! LAI.30A red contract for the post-cutover cat care UI.
//!
//! These tests intentionally assert on missing future client symbols. They are a
//! TDD characterization for the LAI.30 production owner and must not be turned
//! green by local shims in this test target.

const CLIENT: &str = include_str!("../src/lib.rs");
const CATS_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/cats-and-care.md");
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
        CATS_DOC.contains(marker),
        "cats-and-care.md is missing LAI.30 contract marker {marker}"
    );
    assert!(
        TESTING_DOC.contains(marker),
        "testing-cutover.md is missing LAI.30 browser/test marker {marker}"
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
        "{test_name} found forbidden hidden-truth/recompute marker(s): {}",
        present.join(", ")
    );
}

#[test]
fn care_panel_renders_stable_report_safe_cat_identity_and_capability_breakdown() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "care_panel_renders_stable_report_safe_cat_identity_and_capability_breakdown",
        &[
            "CatCarePanelPlugin",
            "CatCarePanelRoot",
            "CatCareStableCatId",
            "CatCareSelectedColonyFilter",
            "MigratedInnateAttributeBreakdown",
            "LearnedSkillAndOfficeExperienceBreakdown",
            "PersonalityAxisBreakdown",
            "AcquiredTraitBadgeList",
            "CatCareReportSafeProjectionOnly",
        ],
    );
}

#[test]
fn stress_recovery_refusal_and_willingness_reasons_are_bounded() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "stress_recovery_refusal_and_willingness_reasons_are_bounded",
        &[
            "CatStressRecoveryMeter",
            "CatRefusalStateBadge",
            "CatWillingnessReasonList",
            "CatCareBoundedEligibilityReason",
            "CatCareTypedBlockReason",
            "CatCareNoHiddenTruthWillingnessRecompute",
            "CatCareNoHiddenRegenerationProjection",
            "CatCareSelfPreservationOverrideBadge",
        ],
    );
}

#[test]
fn anatomy_injury_and_treatment_state_cover_every_body_part() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "anatomy_injury_and_treatment_state_cover_every_body_part",
        &[
            "CatAnatomyPanel",
            "FourPawTwoEyeTailAnatomyGrid",
            "LeftFrontPawStateLabel",
            "RightFrontPawStateLabel",
            "LeftRearPawStateLabel",
            "RightRearPawStateLabel",
            "LeftEyeStateLabel",
            "RightEyeStateLabel",
            "TailStateLabel",
            "CatInjuryTreatmentState",
            "TreatmentHoursRemainingLabel",
        ],
    );
}

#[test]
fn prosthetic_state_reports_side_type_restoration_durability_and_wear() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "prosthetic_state_reports_side_type_restoration_durability_and_wear",
        &[
            "CatProstheticPanel",
            "FittedProstheticStableItemId",
            "FittedProstheticSideLabel",
            "FittedProstheticTypeLabel",
            "FittedProstheticRestorationPercent",
            "FittedProstheticDurabilityHours",
            "FittedProstheticWearProgress",
            "ProstheticAdaptationProgress",
            "ProstheticRestorationCapLabel",
        ],
    );
}

#[test]
fn active_care_tasks_sites_cargo_and_conservation_are_visible_without_leaks() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "active_care_tasks_sites_cargo_and_conservation_are_visible_without_leaks",
        &[
            "ActiveCareTaskReferenceList",
            "CareTaskSiteRefLabel",
            "CareTaskCargoReferenceLabel",
            "CareTaskTreatmentPatientRef",
            "CareTaskFitterOrMedicRef",
            "CareTaskWorkshopRepairRef",
            "CareItemCargoIdentityConservationGuard",
            "CatCareMultiColonyPrivacyGuard",
        ],
    );
}

#[test]
fn care_controls_send_authenticated_expected_version_idempotent_actions() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "care_controls_send_authenticated_expected_version_idempotent_actions",
        &[
            "CareTreatmentActionButton",
            "CareConsentActionButton",
            "CareRefusalAcknowledgeButton",
            "ProstheticFitActionButton",
            "ProstheticRemoveActionButton",
            "ProstheticRepairActionButton",
            "build_cat_care_action_envelope",
            "AuthenticatedPlayerIdentity",
            "ExpectedCatCareVersion",
            "StableIdempotencyId",
        ],
    );
}

#[test]
fn disabled_states_typed_feedback_and_stale_refresh_preserve_selected_cat() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "disabled_states_typed_feedback_and_stale_refresh_preserve_selected_cat",
        &[
            "CatCareControlDisabledReason",
            "CatCareTypedFeedbackToast",
            "CatCareActionConflictRefresh",
            "CatCareVersionMismatchRefreshHandler",
            "PreserveSelectedCatAfterRefresh",
            "PreserveCareDraftAfterRefresh",
            "DuplicateCareReplayUsesOriginalResult",
            "RemovedCatSelectionClearsSafely",
        ],
    );
}

#[test]
fn playwright_visible_browser_ids_and_hidden_truth_guards_are_defined() {
    assert_contract_docs("LAI.30_CAT_CARE_UI_CONTRACT");
    assert_client_has(
        "playwright_visible_browser_ids_and_hidden_truth_guards_are_defined",
        &[
            "ACCESSIBLE_CAT_CARE_PANEL_LABEL",
            "CAT_CARE_PANEL_TEST_ID_PREFIX",
            "CAT_CARE_BODY_PART_TEST_ID_PREFIX",
            "CAT_CARE_CONTROL_TEST_ID_PREFIX",
            "CAT_CARE_TASK_REF_TEST_ID_PREFIX",
            "PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST",
            "VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL",
            "VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC",
            "VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY",
        ],
    );
    assert_client_forbids(
        "playwright_visible_browser_ids_and_hidden_truth_guards_are_defined",
        &[
            "client recomputes cat capability from hidden truth",
            "hidden regeneration cat care",
            "private colony cat care leak",
            "prosthetic item id synthesized in client",
            "unbounded treatment error",
        ],
    );
}
