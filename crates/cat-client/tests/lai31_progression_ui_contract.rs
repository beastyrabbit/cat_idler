//! LAI.31A red contract for Shrine, Favor, research, boosts, diplomacy, and trade UI.
//!
//! These tests intentionally assert on missing future client symbols. They are a
//! TDD characterization for the LAI.31 production owner and must not be turned
//! green by local shims in this test target.

const CLIENT: &str = include_str!("../src/lib.rs");
const SHRINE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/shrine-favor-research.md");
const DIPLOMACY_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/diplomacy-trade.md");
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
        SHRINE_DOC.contains(marker),
        "shrine-favor-research.md is missing LAI.31 contract marker {marker}"
    );
    assert!(
        DIPLOMACY_DOC.contains(marker),
        "diplomacy-trade.md is missing LAI.31 contract marker {marker}"
    );
    assert!(
        TESTING_DOC.contains(marker),
        "testing-cutover.md is missing LAI.31 browser/test marker {marker}"
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
fn shrine_offering_status_is_report_safe_endless_and_physical() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "shrine_offering_status_is_report_safe_endless_and_physical",
        &[
            "ProgressionPanelPlugin",
            "ShrineOfferingPanel",
            "EndlessOfferingStatusRow",
            "OfferingPackageBadge",
            "OfferingBeliefRationale",
            "OfferingReportProvenanceList",
            "OfferingSourceStageLabel",
            "OfferingHaulStageLabel",
            "OfferingRitualStageLabel",
            "OfferingCargoDispositionLabel",
            "PinnedShrineEndpointLabel",
            "OfferingOmissionBlockReason",
            "NoHiddenStockOrRegenerationInOfferingUi",
        ],
    );
}

#[test]
fn exact_micro_favor_ledger_is_single_source_without_mirrored_currency() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "exact_micro_favor_ledger_is_single_source_without_mirrored_currency",
        &[
            "FavorLedgerSummaryPanel",
            "ExactMicroFavorBalance",
            "FavorEventLedgerList",
            "FavorLedgerVersionLabel",
            "FavorDebitCreditOnceMarker",
            "NoMirroredFavorCurrencyGuard",
            "FavorIsNotInventoryCargoEscrowOrResearchPoints",
            "FavorConflictBoundedFeedback",
        ],
    );
}

#[test]
fn research_frontier_quota_insight_scholars_and_preparation_are_visible() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "research_frontier_quota_insight_scholars_and_preparation_are_visible",
        &[
            "ResearchFrontierPanel",
            "StudyManifestCount531",
            "PrerequisiteReadyFrontierList",
            "ResearchPrerequisiteChip",
            "AutomaticSevenDayQuotaWindow",
            "AutomaticQuotaUsedLimitLabel",
            "InsightBalanceLabel",
            "ScholarPreparationPanel",
            "ScholarReassignmentControl",
            "PlayerPreparationDiscount25Percent",
            "ResearchPurchaseCommittedPriceLabel",
        ],
    );
}

#[test]
fn player_only_boost_controls_show_cost_duration_effect_expiry_and_same_type_disable() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "player_only_boost_controls_show_cost_duration_effect_expiry_and_same_type_disable",
        &[
            "DivineBoostPanel",
            "BoostBountifulLaborControl",
            "BoostFleetPawsControl",
            "BoostInspiredWorkControl",
            "BoostRestorativeGraceControl",
            "BoostCostMicroFavorLabel",
            "BoostDurationPicker",
            "BoostEffectStageLabel",
            "BoostExpiryTickLabel",
            "SameTypeActiveBoostDisabledReason",
            "NoLeaderBoostActionGuard",
        ],
    );
}

#[test]
fn diplomacy_consent_state_and_private_colony_boundaries_are_visible() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "diplomacy_consent_state_and_private_colony_boundaries_are_visible",
        &[
            "DiplomacyPanel",
            "RelationshipConsentStateLabel",
            "AllianceApprovalControl",
            "ImmediateBlockControl",
            "DiplomacyExpectedVersionAction",
            "DiplomacyBoundedConflictFeedback",
            "ForeignPrivateStateRedactionGuard",
            "MultiColonyProgressionPrivacyGuard",
        ],
    );
}

#[test]
fn trade_proposal_valuation_escrow_route_cargo_stage_and_recovery_are_visible() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "trade_proposal_valuation_escrow_route_cargo_stage_and_recovery_are_visible",
        &[
            "TradeContractsPanel",
            "TradeProposalValueReportRefs",
            "TradeBeliefValuationConfidence",
            "TradeEscrowSummary",
            "TradeRouteEndpointLabel",
            "TradeCargoStageLabel",
            "TradeRecoveryStateLabel",
            "ConsentRequiredTradeAcceptButton",
            "ConsentRequiredTradeRejectButton",
            "TradeRouteBlockBoundedFeedback",
        ],
    );
}

#[test]
fn progression_actions_are_authenticated_expected_versioned_idempotent_and_bounded() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "progression_actions_are_authenticated_expected_versioned_idempotent_and_bounded",
        &[
            "build_progression_action_envelope",
            "ProgressionStableIdempotencyId",
            "ProgressionAuthenticatedPlayerIdentity",
            "ProgressionExpectedPlannerVersion",
            "ProgressionExpectedResourceVersion",
            "ProgressionExpectedDiplomacyVersion",
            "ProgressionExpectedTradeVersion",
            "ProgressionStaleRefreshHandler",
            "ProgressionDuplicateReplayFeedback",
            "ProgressionMalformedPayloadFeedback",
        ],
    );
}

#[test]
fn playwright_and_visible_browser_checkpoints_are_stable_for_progression_surface() {
    assert_contract_docs("LAI.31_PROGRESSION_UI_CONTRACT");
    assert_client_has(
        "playwright_and_visible_browser_checkpoints_are_stable_for_progression_surface",
        &[
            "ACCESSIBLE_SHRINE_OFFERING_PANEL_LABEL",
            "ACCESSIBLE_FAVOR_LEDGER_PANEL_LABEL",
            "ACCESSIBLE_RESEARCH_FRONTIER_PANEL_LABEL",
            "ACCESSIBLE_DIVINE_BOOST_PANEL_LABEL",
            "ACCESSIBLE_DIPLOMACY_PANEL_LABEL",
            "ACCESSIBLE_TRADE_CONTRACTS_PANEL_LABEL",
            "PROGRESSION_ROW_TEST_ID_PREFIX",
            "VISIBLE_BROWSER_CHECKPOINT_LAI31_OFFERING_RESTART",
            "VISIBLE_BROWSER_CHECKPOINT_LAI31_RESEARCH_BOOST",
            "VISIBLE_BROWSER_CHECKPOINT_LAI31_DIPLOMACY_TRADE",
            "PLAYWRIGHT_PROGRESSIONS_NO_DOM_STATE_INJECTION",
        ],
    );
}
