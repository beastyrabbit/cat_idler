//! Dependency-free UI foundation for the leader AI client surfaces.
//!
//! This module provides product-normal theme, layout, focus, and accessibility
//! contracts for the LAI client work without owning snapshot DTOs or actions.

pub mod accessibility;
pub mod art_assets;
pub use accessibility::{
    LEADER_AI_BROWSER_VIEWPORT, LEADER_AI_CANVAS_ACTION_CHECKPOINTS, LEADER_AI_CANVAS_CHECKPOINTS,
    LeaderAiSemanticNode, report_safe_semantic_id, semantic_node, semantic_status_node,
};
pub mod cat_care;
pub mod interaction;
pub mod lai50_bridge;
pub mod lai50_food;
pub mod lai50_hole_hunting;
pub mod lai50_item_detail;
pub mod lai54;
pub mod lai66;
pub mod lai67;
pub mod lai68;
pub mod layout;
pub mod live_render;
pub mod plans;
pub mod progression;
pub mod recipe_art_assets;
pub mod task_footprints;
pub mod theme;

use bevy::prelude::*;

pub use accessibility::{
    AccessibleLabel, ControlKind, EntityKind, StableUiId, TaskMarkerRole, TestIdBuilder, UiSection,
};
pub use cat_care::{
    ACCESSIBLE_CAT_CARE_PANEL_LABEL, AcquiredTraitBadgeList, ActiveCareTaskReferenceList,
    AnatomySlot, BodyPartSlot, CAT_CARE_BODY_PART_TEST_ID_PREFIX, CAT_CARE_CONTROL_TEST_ID_PREFIX,
    CAT_CARE_PANEL_TEST_ID_PREFIX, CAT_CARE_TASK_REF_TEST_ID_PREFIX, CareConsentActionButton,
    CareItemCargoIdentityConservationGuard, CareRefusalAcknowledgeButton,
    CareTaskCargoReferenceLabel, CareTaskFitterOrMedicRef, CareTaskSiteRefLabel,
    CareTaskTreatmentPatientRef, CareTaskWorkshopRepairRef, CareTreatmentActionButton,
    CatAnatomyPanel, CatCareAction, CatCareActionBuildError, CatCareActionConflictRefresh,
    CatCareBoundedEligibilityReason, CatCareCardRenderModel, CatCareControlDisabledReason,
    CatCareDraft, CatCareMultiColonyPrivacyGuard, CatCareNoHiddenRegenerationProjection,
    CatCareNoHiddenTruthWillingnessRecompute, CatCarePanelChrome, CatCarePanelInput,
    CatCarePanelLayoutSpec, CatCarePanelPlugin, CatCarePanelProjection,
    CatCarePanelProjectionResource, CatCarePanelRoot, CatCarePanelState, CatCareRefreshState,
    CatCareRegenerationProjection, CatCareReportSafeProjectionOnly, CatCareSelectedColonyFilter,
    CatCareSelfPreservationOverrideBadge, CatCareStableCatId, CatCareTypedBlockReason,
    CatCareTypedFeedbackToast, CatCareVersionMismatchRefreshHandler, CatInjuryTreatmentState,
    CatProstheticPanel, CatRefusalStateBadge, CatStressRecoveryMeter, CatWillingnessReasonList,
    DuplicateCareReplayUsesOriginalResult, ExpectedCatCareVersion, ExpectedCatCareVersionBundle,
    ExpectedProstheticVersion, FittedProstheticDurabilityHours, FittedProstheticRestorationPercent,
    FittedProstheticSideLabel, FittedProstheticStableItemId, FittedProstheticTypeLabel,
    FittedProstheticWearProgress, FourPawTwoEyeTailAnatomyGrid,
    LearnedSkillAndOfficeExperienceBreakdown, LeftEyeStateLabel, LeftFrontPawStateLabel,
    LeftRearPawStateLabel, MigratedInnateAttributeBreakdown, PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST,
    PersonalityAxisBreakdown, PreserveCareDraftAfterRefresh, PreserveSelectedCatAfterRefresh,
    ProstheticAdaptationProgress, ProstheticFitActionButton, ProstheticRemoveActionButton,
    ProstheticRepairActionButton, ProstheticRestorationCapLabel, RemovedCatSelectionClearsSafely,
    RightEyeStateLabel, RightFrontPawStateLabel, RightRearPawStateLabel, TailStateLabel,
    TreatmentHoursRemainingLabel, VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL,
    VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY,
    VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC, build_cat_care_action_envelope,
    project_cat_care_regeneration_report, render_cat_care_panel, update_cat_care_panel_projection,
};
pub use interaction::{
    LeaderAiActionButton, LeaderAiInteractionPlugin, LeaderAiInteractionState, LeaderAiLocalAction,
    LeaderAiLocalButton, LeaderAiSelectionButton, LeaderAiSelectionKind,
};
pub use layout::{
    FocusKey, FocusMemory, FocusRetention, InputBlockerState, OverlayBand, OverlayLayer,
    ResponsiveClass, ResponsiveDecision, ResponsivePolicy, ViewportSize, WorldInputPolicy,
};
pub use live_render::{
    LeaderAiLiveRenderPlugin, LeaderAiPanelEntity, LeaderAiRowEntity, LeaderAiWorldMarkerEntity,
};
pub use plans::{
    ACCESSIBLE_PLANS_PANEL_LABEL, ACCESSIBLE_STANDING_ORDERS_PANEL_LABEL,
    AdministrationSlotLimitReached, AdministrationSlotMeter, AuthenticatedPlayerIdentity,
    BoundedPlanConflictToast, CurrentPlanningEpochOnly, DespawnsUnknownPlanRows,
    DeterministicPlanRowOrder, DismissPlanButton, DomainNudgeControl,
    DuplicateReplayUsesOriginalResult, EffectiveReportLevelGate, EqualNudgesDoNotStack,
    EstimateRange, ExpectedDomainVersion, ExpectedPlannerVersion, ExpectedReservationVersion,
    ExpectedResourceVersion, ExpectedVersionBundle, LeaderAiPlanNudgeAction,
    LeaderAiStandingOrderAction, LeaderResponsibleActorBadge, MovePlanDownButton, MovePlanUpButton,
    NoClientRegenerationFallback, NoStalePlanControlReuse, OFFICER_REPORT_TEST_ID_PREFIX,
    OfficerAuthorityBadge, OfficerReportPanel, OfficerReportTestId, OfficerRequestReasonList,
    OfficerVacancySlot, OppositeNudgeReplacesPrior, PLAN_CONTROL_TEST_ID_PREFIX,
    PLAN_NUDGE_DOWN_DELTA_BP_NEG_1500, PLAN_NUDGE_UP_DELTA_BP_1500, PLAN_ROW_TEST_ID_PREFIX,
    PLAYWRIGHT_NO_DOM_STATE_INJECTION, PlanActionBuildError, PlanActionConflictRefresh,
    PlanBlockReason, PlanBoundedRationale, PlanControlDisabledReason, PlanCostLabel,
    PlanDependencyList, PlanLifecycleStatusLabel, PlanReportAgeBadge, PlanReportProvenanceList,
    PlanResponsibleActorLabel, PlanRowRenderModel, PlanRowStableId, PlanScoreConfidenceRange,
    PlanUncertaintyCopy, PlanUrgency, PlansNoHiddenTruthGuard, PlansPanelChrome, PlansPanelInput,
    PlansPanelLayoutSpec, PlansPanelPlugin, PlansPanelProjection, PlansPanelProjectionResource,
    PlansPanelRoot, PlansRefreshState, PreservePlansPanelFocusAfterRefresh,
    PreserveStandingOrderDraftAfterRefresh, RegenerationUnavailableBelowReportLevel4,
    RemovedPlanControlsAreDisabled, ReportSafeUnavailableState, STANDING_ORDER_ROW_TEST_ID_PREFIX,
    StableIdempotencyId, StablePlanTieBreakKey, StandingOrderBoundedFeedback,
    StandingOrderCreateButton, StandingOrderDoesNotBypassKnowledgeOrPhysicalRules,
    StandingOrderDraft, StandingOrderDraftPatch, StandingOrderEditButton,
    StandingOrderPolicyDomainPicker, StandingOrderRemoveButton, StandingOrdersPanel,
    VISIBLE_BROWSER_CHECKPOINT_PLANS_TOP_EIGHT, VersionMismatchRefreshHandler,
    accessibility_label_dismiss_plan, accessibility_label_move_plan_down,
    accessibility_label_move_plan_up, build_leader_ai_action_envelope,
    build_standing_order_action_envelope, render_authoritative_top_eight_plans,
    send_expected_version_action, update_plans_panel_projection,
};
pub use progression::{
    ACCESSIBLE_DIPLOMACY_PANEL_LABEL, ACCESSIBLE_DIVINE_BOOST_PANEL_LABEL,
    ACCESSIBLE_FAVOR_LEDGER_PANEL_LABEL, ACCESSIBLE_RESEARCH_FRONTIER_PANEL_LABEL,
    ACCESSIBLE_SHRINE_OFFERING_PANEL_LABEL, ACCESSIBLE_TRADE_CONTRACTS_PANEL_LABEL,
    AllianceApprovalControl, AutomaticQuotaUsedLimitLabel, AutomaticSevenDayQuotaWindow,
    BoostBountifulLaborControl, BoostControlRenderModel, BoostCostMicroFavorLabel,
    BoostDurationPicker, BoostEffectStageLabel, BoostExpiryTickLabel, BoostFleetPawsControl,
    BoostInspiredWorkControl, BoostRestorativeGraceControl, ConsentRequiredTradeAcceptButton,
    ConsentRequiredTradeRejectButton, DiplomacyBoundedConflictFeedback,
    DiplomacyExpectedVersionAction, DiplomacyPanel, DivineBoostKind, DivineBoostPanel,
    EndlessOfferingStatusRow, ExactMicroFavorBalance, FavorConflictBoundedFeedback,
    FavorDebitCreditOnceMarker, FavorEventLedgerList,
    FavorIsNotInventoryCargoEscrowOrResearchPoints, FavorLedgerSummaryPanel,
    FavorLedgerVersionLabel, ForeignPrivateStateRedactionGuard, ImmediateBlockControl,
    InsightBalanceLabel, MultiColonyProgressionPrivacyGuard,
    NoHiddenStockOrRegenerationInOfferingUi, NoLeaderBoostActionGuard,
    NoMirroredFavorCurrencyGuard, OfferingBeliefRationale, OfferingCargoDispositionLabel,
    OfferingHaulStageLabel, OfferingOmissionBlockReason, OfferingPackageBadge,
    OfferingReportProvenanceList, OfferingRitualStageLabel, OfferingSourceStageLabel,
    PLAYWRIGHT_PROGRESSIONS_NO_DOM_STATE_INJECTION, PROGRESSION_ROW_TEST_ID_PREFIX,
    PinnedShrineEndpointLabel, PlayerPreparationDiscount25Percent, PrerequisiteReadyFrontierList,
    ProgressionAction, ProgressionActionBuildError, ProgressionAuthenticatedPlayerIdentity,
    ProgressionDuplicateReplayFeedback, ProgressionExpectedBoostVersion,
    ProgressionExpectedDiplomacyVersion, ProgressionExpectedPlannerVersion,
    ProgressionExpectedResearchVersion, ProgressionExpectedResourceVersion,
    ProgressionExpectedScholarVersion, ProgressionExpectedTradeVersion,
    ProgressionExpectedVersionBundle, ProgressionMalformedPayloadFeedback, ProgressionPanelChrome,
    ProgressionPanelInput, ProgressionPanelLayoutSpec, ProgressionPanelPlugin,
    ProgressionPanelProjection, ProgressionPanelProjectionResource, ProgressionPanelRoot,
    ProgressionPanelState, ProgressionRefreshState, ProgressionStableIdempotencyId,
    ProgressionStaleRefreshHandler, ProgressionTab, ProgressionTypedFeedback,
    RelationshipConsentStateLabel, ResearchFrontierPanel, ResearchPrerequisiteChip,
    ResearchPurchaseCommittedPriceLabel, SameTypeActiveBoostDisabledReason,
    ScholarPreparationPanel, ScholarReassignmentControl, ShrineOfferingPanel,
    StudyManifestCount531, TradeBeliefValuationConfidence, TradeCargoStageLabel,
    TradeContractsPanel, TradeEscrowSummary, TradeProposalValueReportRefs, TradeRecoveryStateLabel,
    TradeRouteBlockBoundedFeedback, TradeRouteEndpointLabel,
    VISIBLE_BROWSER_CHECKPOINT_LAI31_DIPLOMACY_TRADE,
    VISIBLE_BROWSER_CHECKPOINT_LAI31_OFFERING_RESTART,
    VISIBLE_BROWSER_CHECKPOINT_LAI31_RESEARCH_BOOST, build_progression_action_envelope,
    render_progression_panel, update_progression_panel_projection,
};
pub use task_footprints::{
    ACCESSIBLE_TASK_ENDPOINT_LABEL, ACCESSIBLE_TASK_OBJECTIVE_LABEL,
    ACCESSIBLE_TASK_WORK_SLOT_LABEL, BlockedOrUnreachableSiteSuppressesWorldMarker,
    BlockedSiteRefNoMarker, CanonicalFootprintCellIndex, DedupeVisibleTaskMarkerBySnapshotId,
    DespawnRemovedVisibleTaskMarkers, FetchWaterDryBankWorkMarker,
    FetchWaterPinnedDeliveryEndpointMarker, FetchWaterSourceMarker,
    ForeignColonyVisibleTaskNoMarker, HuntObjectiveCaveOrSourceMarker, MissingSiteRefNoMarker,
    MultiColonyTaskMarkerIsolation, NoCatDestinationAuthorityForTaskMarkers,
    NoClientSideSiteGuessing, NoDuplicateCoincidentTaskMarker, NoDuplicatedWorkshopSizeConstant,
    NoExactRegenerationBelowLevelFourTooltip, NoGenericTaskDestinationFallback,
    NoHiddenStockTooltipField, NoPrivateBeliefOrPlanTooltip, NoRadialTaskMarkerFallback,
    NoStaleTaskMarkerReuse, ObjectiveLessBlockedTaskNoMapEntity,
    PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST, RedactedVisibleTaskNoMarker,
    ReportSafeTaskMarkerVisibility, RouteContactMarkerIsNotDeliveryEndpoint,
    SemanticSiteStageDedupeKey, StrictSiteRefMarkerResolver, TASK_MARKER_CELL_TEST_ID,
    TASK_MARKER_ENDPOINT_TEST_ID, TASK_MARKER_OBJECTIVE_TEST_ID, TASK_MARKER_WORK_SLOT_TEST_ID,
    TaskFootprintProjection, TaskFootprintProjectionError, TaskMarkerEntity, TaskMarkerKind,
    TaskMarkerReportSafeTooltip, TaskMarkerScreenBoundsGuard, TaskMarkerSpecialization,
    TaskMarkerSupportedZoomRange, TaskMarkerTooltipRedactionGuard,
    TaskMarkerViewportCullingKeepsAuthoritativeIds, TaskSnapshotIdMarkerKey,
    TreeObjectiveSixCanonicalCells, VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE,
    VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER, VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION,
    VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT, VisibleTaskMarkerInput,
    VisibleTaskMarkerPlugin, VisibleTaskMarkerWorld, VisibleTaskRemovalEvent,
    VisibleTaskSnapshotMarkerSource, WaterSourceIsNotWalkableWorkPosition,
    WorkshopDistinctDeliveryEndpointMarker, WorkshopDistinctWorkSlotMarker,
    WorkshopObjectiveNineRowMajorCells, project_visible_task_footprint,
    project_visible_task_footprints, update_visible_task_marker_world,
};
pub use theme::{
    ColorToken, FeedbackState, ForbiddenPattern, GeometryScale, LeaderAiUiTheme, MotionDuration,
    RoleColor, SpacingScale, StateStyle, StyleValidationError, validate_product_normal_tokens,
};

/// Shared defaults registered by the client root for future LAI panels.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct LeaderAiUiFoundation {
    pub theme: LeaderAiUiTheme,
    pub responsive: ResponsivePolicy,
    pub focus: FocusMemory,
    pub input: WorldInputPolicy,
}

/// Registers pure LAI UI defaults; rendering systems are added by later slices.
#[derive(Default)]
pub struct LeaderAiUiFoundationPlugin;

impl Plugin for LeaderAiUiFoundationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LeaderAiUiFoundation>().add_plugins((
            lai50_hole_hunting::Lai50HoleHuntingPlugin,
            lai50_food::Lai50FoodCookhousePlugin,
            lai50_item_detail::Lai50ItemDetailPlugin,
            lai50_bridge::Lai50RouteBridgePlugin,
            lai66::Lai66ReportsPlugin,
            lai67::Lai67ResearchCouncilPlugin,
            lai68::Lai68WorldRenderPlugin,
        ));
    }
}
