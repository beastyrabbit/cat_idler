//! Pure render and action models for the LAI.31 progression UI.
//!
//! This leaf projects Shrine, Favor, research, boosts, diplomacy, and trade
//! from LAI.24 report-safe snapshots, and builds LAI.25 action envelopes. It
//! does not infer hidden stock, regeneration, route safety, or private colony
//! state.

use bevy::prelude::*;
use cat_protocol::{
    ActionConflict, ActionDecodeError, ActionIdempotencyId, ActionProtocolVersion,
    AuthenticatedPlayerId, BoundedColonyId, BoundedEntityId, BoundedFavorAmount, ColonyAiSnapshot,
    CurrentStateHint, DiplomacyRelationshipTarget, DivineBoostSnapshot, ExpectedStateVersions,
    FavorEventSnapshot, FavorLedgerSnapshot, LeaderAiActionEnvelope, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, OfferingStageSnapshot, RelationshipSnapshot,
    ReportSafeString, ResearchFrontierSnapshot, ResearchStudySnapshot, ScholarPreparationSnapshot,
    SelectedColonyId, ShrineOfferingPipelineSnapshot, ShrineSnapshot, SiteRefSnapshot,
    StaleClientRefresh, TradeContractSnapshot, TradeRejectionReason, TradeStageSnapshot,
};

use super::RoleColor;
use super::{
    AccessibleLabel, ControlKind, EntityKind, FeedbackState, StableIdempotencyId, StableUiId,
    TestIdBuilder, UiSection,
};

pub const ACCESSIBLE_SHRINE_OFFERING_PANEL_LABEL: &str = "Shrine offering panel";
pub const ACCESSIBLE_FAVOR_LEDGER_PANEL_LABEL: &str = "Favor ledger panel";
pub const ACCESSIBLE_RESEARCH_FRONTIER_PANEL_LABEL: &str = "Research frontier panel";
pub const ACCESSIBLE_DIVINE_BOOST_PANEL_LABEL: &str = "Divine boost panel";
pub const ACCESSIBLE_DIPLOMACY_PANEL_LABEL: &str = "Diplomacy panel";
pub const ACCESSIBLE_TRADE_CONTRACTS_PANEL_LABEL: &str = "Trade contracts panel";
pub const PROGRESSION_ROW_TEST_ID_PREFIX: &str = "lai-ui:progression:row:";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI31_OFFERING_RESTART: &str = "lai31-offering-restart";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI31_RESEARCH_BOOST: &str = "lai31-research-boost";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI31_DIPLOMACY_TRADE: &str = "lai31-diplomacy-trade";
pub const PLAYWRIGHT_PROGRESSIONS_NO_DOM_STATE_INJECTION: &str =
    "lai31-progressions-no-dom-state-injection";

pub const EXACT_ONE_FAVOR_MICRO_FAVOR: u64 = 1_000_000;
pub const STUDY_MANIFEST_COUNT_531: usize = 531;
pub const SCHOLAR_TRACK_COUNT: usize = 4;
pub const SCHOLAR_TRACK_STAGE_COUNT_11: usize = 11;
pub const PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS_25_PERCENT: u16 = 2_500;
pub const BOOST_DURATION_OPTIONS_HOURS: [u16; 12] = [1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24];

#[derive(Default)]
pub struct ProgressionPanelPlugin;

impl Plugin for ProgressionPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProgressionPanelState>()
            .init_resource::<ProgressionPanelInput>()
            .init_resource::<ProgressionPanelProjectionResource>()
            .add_systems(Update, update_progression_panel_projection);
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ProgressionPanelRoot;

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProgressionPanelState {
    pub selected_row_id: Option<String>,
    pub selected_tab: ProgressionTab,
    pub refresh_state: ProgressionRefreshState,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProgressionPanelInput {
    pub selected_colony_id: Option<String>,
    pub selected_duration_hours: u16,
    pub colony: Option<ColonyAiSnapshot>,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct ProgressionPanelProjectionResource {
    pub projection: Option<ProgressionPanelProjection>,
}

pub fn update_progression_panel_projection(
    input: Res<'_, ProgressionPanelInput>,
    state: Res<'_, ProgressionPanelState>,
    mut output: ResMut<'_, ProgressionPanelProjectionResource>,
) {
    let Some(colony) = input.colony.as_ref() else {
        output.projection = None;
        return;
    };
    let selected_colony_id = input
        .selected_colony_id
        .as_deref()
        .unwrap_or_else(|| colony.colony_id.as_str());
    let Some(mut projection) = render_progression_panel(
        colony,
        selected_colony_id,
        input.selected_duration_hours,
        state.refresh_state,
    ) else {
        output.projection = None;
        return;
    };
    projection.selected_tab = state.selected_tab;
    projection.selected_row_id = state.selected_row_id.as_ref().and_then(|row_id| {
        projection
            .visible_row_ids
            .iter()
            .any(|visible| visible == row_id)
            .then(|| row_id.clone())
    });
    output.projection = Some(projection);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressionTab {
    Shrine,
    #[default]
    Research,
    Boosts,
    Diplomacy,
    Trade,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressionRefreshState {
    #[default]
    Current,
    Loading,
    Stale,
    UpdateRequired,
    Error,
}

impl ProgressionRefreshState {
    pub const fn feedback(self) -> FeedbackState {
        match self {
            Self::Current => FeedbackState::Empty,
            Self::Loading => FeedbackState::Loading,
            Self::Stale => FeedbackState::Stale,
            Self::UpdateRequired => FeedbackState::UpdateRequired,
            Self::Error => FeedbackState::Error,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionPanelProjection {
    pub colony_id: String,
    pub shrine: ShrineOfferingPanel,
    pub favor: FavorLedgerSummaryPanel,
    pub research: ResearchFrontierPanel,
    pub boosts: DivineBoostPanel,
    pub diplomacy: DiplomacyPanel,
    pub trade: TradeContractsPanel,
    pub selected_tab: ProgressionTab,
    pub selected_row_id: Option<String>,
    pub visible_row_ids: Vec<String>,
    pub refresh_state: ProgressionRefreshState,
    pub layout: ProgressionPanelLayoutSpec,
    pub chrome: ProgressionPanelChrome,
    pub privacy_guard: MultiColonyProgressionPrivacyGuard,
}

pub fn render_progression_panel(
    colony: &ColonyAiSnapshot,
    selected_colony_id: &str,
    selected_duration_hours: u16,
    refresh_state: ProgressionRefreshState,
) -> Option<ProgressionPanelProjection> {
    if colony.colony_id.as_str() != selected_colony_id || !colony.capabilities.can_view {
        return None;
    }
    let shrine = ShrineOfferingPanel::from_snapshot(&colony.shrine);
    let favor = FavorLedgerSummaryPanel::from_snapshot(&colony.favor);
    let research = ResearchFrontierPanel::from_snapshot(&colony.research);
    let boosts =
        DivineBoostPanel::from_snapshot(&colony.research, &colony.boosts, selected_duration_hours);
    let diplomacy = DiplomacyPanel::from_snapshot(&colony.diplomacy);
    let trade = TradeContractsPanel::from_snapshot(&colony.trade);
    let visible_row_ids =
        visible_progression_row_ids(&shrine, &favor, &research, &boosts, &diplomacy, &trade);
    Some(ProgressionPanelProjection {
        colony_id: colony.colony_id.as_str().to_string(),
        shrine,
        favor,
        research,
        boosts,
        diplomacy,
        trade,
        selected_tab: ProgressionTab::default(),
        selected_row_id: None,
        visible_row_ids,
        refresh_state,
        layout: ProgressionPanelLayoutSpec::default(),
        chrome: ProgressionPanelChrome::default(),
        privacy_guard: MultiColonyProgressionPrivacyGuard,
    })
}

fn visible_progression_row_ids(
    shrine: &ShrineOfferingPanel,
    favor: &FavorLedgerSummaryPanel,
    research: &ResearchFrontierPanel,
    boosts: &DivineBoostPanel,
    diplomacy: &DiplomacyPanel,
    trade: &TradeContractsPanel,
) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(pipeline) = shrine.active_pipeline.as_ref() {
        ids.push(pipeline.offering_id.clone());
    }
    ids.extend(favor.events.0.iter().map(|event| event.event_id.clone()));
    ids.extend(
        research
            .frontier
            .0
            .iter()
            .map(|study| study.study_id.clone()),
    );
    ids.extend(
        research
            .scholar_preparation
            .preparations
            .iter()
            .map(|preparation| preparation.preparation_id.clone()),
    );
    ids.extend(
        boosts
            .controls
            .iter()
            .map(|boost| boost.kind.protocol_id().to_string()),
    );
    ids.extend(
        diplomacy
            .relationships
            .iter()
            .map(|relationship| relationship.relationship_id.clone()),
    );
    ids.extend(
        trade
            .rows
            .iter()
            .map(|contract| contract.contract_id.clone()),
    );
    ids
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionPanelLayoutSpec {
    pub panel_width_px: u16,
    pub compact_panel_width_px: u16,
    pub row_min_height_px: u16,
    pub ledger_event_height_px: u16,
    pub panel_radius_px: u16,
    pub world_first: bool,
}

impl Default for ProgressionPanelLayoutSpec {
    fn default() -> Self {
        Self {
            panel_width_px: 460,
            compact_panel_width_px: 336,
            row_min_height_px: 44,
            ledger_event_height_px: 36,
            panel_radius_px: 10,
            world_first: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionPanelChrome {
    pub paper_role: RoleColor,
    pub border_role: RoleColor,
    pub shrine_role: RoleColor,
    pub favor_role: RoleColor,
    pub boost_role: RoleColor,
    pub diplomacy_role: RoleColor,
    pub trade_role: RoleColor,
    pub danger_role: RoleColor,
}

impl Default for ProgressionPanelChrome {
    fn default() -> Self {
        Self {
            paper_role: RoleColor::Paper,
            border_role: RoleColor::Wood,
            shrine_role: RoleColor::Stone,
            favor_role: RoleColor::Rust,
            boost_role: RoleColor::Olive,
            diplomacy_role: RoleColor::Olive,
            trade_role: RoleColor::Wood,
            danger_role: RoleColor::Danger,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShrineOfferingPanel {
    pub shrine_id: String,
    pub label: AccessibleLabel,
    pub pinned_endpoint: PinnedShrineEndpointLabel,
    pub active_pipeline: Option<EndlessOfferingStatusRow>,
    pub package_catalog: Vec<OfferingPackageBadge>,
    pub no_hidden_guard: NoHiddenStockOrRegenerationInOfferingUi,
}

impl ShrineOfferingPanel {
    fn from_snapshot(shrine: &ShrineSnapshot) -> Self {
        Self {
            shrine_id: shrine.shrine_id.as_str().to_string(),
            label: AccessibleLabel::panel(UiSection::Progression),
            pinned_endpoint: PinnedShrineEndpointLabel(site_ref_id(&shrine.endpoint)),
            active_pipeline: shrine
                .pipeline
                .as_ref()
                .map(EndlessOfferingStatusRow::from_snapshot),
            package_catalog: one_favor_package_catalog(),
            no_hidden_guard: NoHiddenStockOrRegenerationInOfferingUi,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndlessOfferingStatusRow {
    pub offering_id: String,
    pub test_id: StableUiId,
    pub package: OfferingPackageBadge,
    pub rationale: OfferingBeliefRationale,
    pub report_provenance: OfferingReportProvenanceList,
    pub source_stage: OfferingSourceStageLabel,
    pub haul_stage: OfferingHaulStageLabel,
    pub ritual_stage: OfferingRitualStageLabel,
    pub cargo_disposition: OfferingCargoDispositionLabel,
    pub pinned_endpoint: PinnedShrineEndpointLabel,
    pub omission_or_block: Option<OfferingOmissionBlockReason>,
}

impl EndlessOfferingStatusRow {
    fn from_snapshot(pipeline: &ShrineOfferingPipelineSnapshot) -> Self {
        let (source_stage, haul_stage, ritual_stage, blocked) =
            offering_stage_labels(&pipeline.stage);
        Self {
            offering_id: pipeline.offering_id.as_str().to_string(),
            test_id: TestIdBuilder::row(
                UiSection::Progression,
                EntityKind::ShrineOffering,
                pipeline.offering_id.as_str(),
            ),
            package: OfferingPackageBadge {
                package_id: pipeline.package.package_id.as_str().to_string(),
                package_kind: pipeline.package.package_kind.as_str().to_string(),
                cargo_ids: pipeline
                    .package
                    .cargo_ids
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
                favor_reward_micro_favor: pipeline.package.favor_reward_micro_favor,
            },
            rationale: OfferingBeliefRationale(pipeline.rationale.as_str().to_string()),
            report_provenance: OfferingReportProvenanceList(
                pipeline
                    .source_report_ids
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            ),
            source_stage,
            haul_stage,
            ritual_stage,
            cargo_disposition: OfferingCargoDispositionLabel(
                pipeline.cargo_disposition.as_str().to_string(),
            ),
            pinned_endpoint: PinnedShrineEndpointLabel(site_ref_id(&pipeline.shrine_endpoint)),
            omission_or_block: pipeline
                .blocked_reason
                .as_ref()
                .map(|reason| OfferingOmissionBlockReason(reason.as_str().to_string()))
                .or(blocked),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingPackageBadge {
    pub package_id: String,
    pub package_kind: String,
    pub cargo_ids: Vec<String>,
    pub favor_reward_micro_favor: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingBeliefRationale(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingReportProvenanceList(pub Vec<String>);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingSourceStageLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingHaulStageLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingRitualStageLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingCargoDispositionLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedShrineEndpointLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferingOmissionBlockReason(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoHiddenStockOrRegenerationInOfferingUi;

fn one_favor_package_catalog() -> Vec<OfferingPackageBadge> {
    [
        ("offering:20_food", "20 Food"),
        ("offering:5_herbs", "5 Herbs"),
        ("offering:10_materials", "10 Materials"),
        ("offering:5_refined_resources", "5 Refined resources"),
    ]
    .into_iter()
    .map(|(package_id, package_kind)| OfferingPackageBadge {
        package_id: package_id.to_string(),
        package_kind: package_kind.to_string(),
        cargo_ids: Vec::new(),
        favor_reward_micro_favor: EXACT_ONE_FAVOR_MICRO_FAVOR,
    })
    .collect()
}

fn offering_stage_labels(
    stage: &OfferingStageSnapshot,
) -> (
    OfferingSourceStageLabel,
    OfferingHaulStageLabel,
    OfferingRitualStageLabel,
    Option<OfferingOmissionBlockReason>,
) {
    match stage {
        OfferingStageSnapshot::Proposed => (
            OfferingSourceStageLabel("belief review".to_string()),
            OfferingHaulStageLabel("not hauling".to_string()),
            OfferingRitualStageLabel("not started".to_string()),
            None,
        ),
        OfferingStageSnapshot::Reserved => (
            OfferingSourceStageLabel("reserved".to_string()),
            OfferingHaulStageLabel("awaiting pickup".to_string()),
            OfferingRitualStageLabel("not started".to_string()),
            None,
        ),
        OfferingStageSnapshot::Hauling { carrier_cat_id } => (
            OfferingSourceStageLabel("source committed".to_string()),
            OfferingHaulStageLabel(format!("hauling by {}", carrier_cat_id.as_str())),
            OfferingRitualStageLabel("not started".to_string()),
            None,
        ),
        OfferingStageSnapshot::Ritual { ritualist_cat_id } => (
            OfferingSourceStageLabel("delivered".to_string()),
            OfferingHaulStageLabel("delivered to Shrine".to_string()),
            OfferingRitualStageLabel(format!("ritual by {}", ritualist_cat_id.as_str())),
            None,
        ),
        OfferingStageSnapshot::Complete { completed_at_ms } => (
            OfferingSourceStageLabel("consumed".to_string()),
            OfferingHaulStageLabel("delivered to Shrine".to_string()),
            OfferingRitualStageLabel(format!("credited at {completed_at_ms}")),
            None,
        ),
        OfferingStageSnapshot::Blocked { reason } => (
            OfferingSourceStageLabel("blocked".to_string()),
            OfferingHaulStageLabel("paused".to_string()),
            OfferingRitualStageLabel("not credited".to_string()),
            Some(OfferingOmissionBlockReason(reason.as_str().to_string())),
        ),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavorLedgerSummaryPanel {
    pub version: FavorLedgerVersionLabel,
    pub exact_balance: ExactMicroFavorBalance,
    pub events: FavorEventLedgerList,
    pub single_source_guard: NoMirroredFavorCurrencyGuard,
    pub not_inventory_guard: FavorIsNotInventoryCargoEscrowOrResearchPoints,
}

impl FavorLedgerSummaryPanel {
    fn from_snapshot(favor: &FavorLedgerSnapshot) -> Self {
        Self {
            version: FavorLedgerVersionLabel(favor.ledger_version),
            exact_balance: ExactMicroFavorBalance(favor.micro_favor),
            events: FavorEventLedgerList(
                favor
                    .favor_events
                    .iter()
                    .map(FavorLedgerEventRow::from_snapshot)
                    .collect(),
            ),
            single_source_guard: NoMirroredFavorCurrencyGuard,
            not_inventory_guard: FavorIsNotInventoryCargoEscrowOrResearchPoints,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactMicroFavorBalance(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavorEventLedgerList(pub Vec<FavorLedgerEventRow>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FavorLedgerVersionLabel(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FavorDebitCreditOnceMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoMirroredFavorCurrencyGuard;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FavorIsNotInventoryCargoEscrowOrResearchPoints;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavorConflictBoundedFeedback(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavorLedgerEventRow {
    pub event_id: String,
    pub delta_micro_favor: i64,
    pub resulting_micro_favor: u64,
    pub occurred_at_ms: i64,
    pub reason: String,
    pub once_marker: FavorDebitCreditOnceMarker,
}

impl FavorLedgerEventRow {
    fn from_snapshot(event: &FavorEventSnapshot) -> Self {
        Self {
            event_id: event.event_id.as_str().to_string(),
            delta_micro_favor: event.delta_micro_favor,
            resulting_micro_favor: event.resulting_micro_favor,
            occurred_at_ms: event.occurred_at_ms,
            reason: event.reason.as_str().to_string(),
            once_marker: FavorDebitCreditOnceMarker,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchFrontierPanel {
    pub manifest_count: StudyManifestCount531,
    pub owned_study_ids: Vec<String>,
    pub frontier: PrerequisiteReadyFrontierList,
    pub quota_window: AutomaticSevenDayQuotaWindow,
    pub quota_used_limit: AutomaticQuotaUsedLimitLabel,
    pub insight: InsightBalanceLabel,
    pub scholar_preparation: ScholarPreparationPanel,
    pub scholar_tracks: Vec<ScholarTrackStageNavigator>,
}

impl ResearchFrontierPanel {
    fn from_snapshot(research: &ResearchFrontierSnapshot) -> Self {
        Self {
            manifest_count: StudyManifestCount531(research.manifest_study_count),
            owned_study_ids: research
                .owned_study_ids
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            frontier: PrerequisiteReadyFrontierList(
                research
                    .frontier
                    .iter()
                    .map(ResearchStudyRow::from_snapshot)
                    .collect(),
            ),
            quota_window: AutomaticSevenDayQuotaWindow {
                started_at_ms: research.automatic_quota.quota_window_started_at_ms,
                ends_at_ms: research
                    .automatic_quota
                    .quota_window_started_at_ms
                    .saturating_add(7 * 24 * 60 * 60 * 1_000),
            },
            quota_used_limit: AutomaticQuotaUsedLimitLabel {
                used: research.automatic_quota.quota_used,
                limit: research.automatic_quota.quota_limit,
            },
            insight: InsightBalanceLabel {
                balance: research.insight.insight_balance,
                generated_this_week: research.insight.generated_this_week,
                week_started_at_ms: research.insight.week_started_at_ms,
            },
            scholar_preparation: ScholarPreparationPanel::from_snapshot(&research.preparations),
            scholar_tracks: ScholarTrackStageNavigator::from_owned_studies(
                &research.owned_study_ids,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StudyManifestCount531(pub usize);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrerequisiteReadyFrontierList(pub Vec<ResearchStudyRow>);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchPrerequisiteChip(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutomaticSevenDayQuotaWindow {
    pub started_at_ms: i64,
    pub ends_at_ms: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutomaticQuotaUsedLimitLabel {
    pub used: u8,
    pub limit: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsightBalanceLabel {
    pub balance: u64,
    pub generated_this_week: u64,
    pub week_started_at_ms: Option<i64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerPreparationDiscount25Percent(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchPurchaseCommittedPriceLabel {
    pub undiscounted_micro_favor: u64,
    pub prepared_micro_favor: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchStudyRow {
    pub study_id: String,
    pub display_name: String,
    pub prerequisites: Vec<ResearchPrerequisiteChip>,
    pub committed_price: ResearchPurchaseCommittedPriceLabel,
    pub test_id: StableUiId,
}

impl ResearchStudyRow {
    fn from_snapshot(study: &ResearchStudySnapshot) -> Self {
        Self {
            study_id: study.study_id.as_str().to_string(),
            display_name: study.display_name.as_str().to_string(),
            prerequisites: study
                .prerequisite_ids
                .iter()
                .map(|id| ResearchPrerequisiteChip(id.as_str().to_string()))
                .collect(),
            committed_price: ResearchPurchaseCommittedPriceLabel {
                undiscounted_micro_favor: study.price_micro_favor,
                prepared_micro_favor: study.prepared_price_micro_favor,
            },
            test_id: TestIdBuilder::row(
                UiSection::Progression,
                EntityKind::Research,
                study.study_id.as_str(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScholarPreparationPanel {
    pub preparations: Vec<ScholarPreparationRow>,
    pub reassignment: ScholarReassignmentControl,
    pub player_discount: PlayerPreparationDiscount25Percent,
}

impl ScholarPreparationPanel {
    fn from_snapshot(preparations: &[ScholarPreparationSnapshot]) -> Self {
        Self {
            preparations: preparations
                .iter()
                .map(ScholarPreparationRow::from_snapshot)
                .collect(),
            reassignment: ScholarReassignmentControl {
                enabled: !preparations.is_empty(),
            },
            player_discount: PlayerPreparationDiscount25Percent(
                PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS_25_PERCENT,
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScholarPreparationRow {
    pub preparation_id: String,
    pub study_id: String,
    pub scholar_cat_id: Option<String>,
    pub progress_basis_points: u16,
    pub committed_insight_cost: u64,
    pub player_discount_basis_points: u16,
    pub prepared: bool,
}

impl ScholarPreparationRow {
    fn from_snapshot(preparation: &ScholarPreparationSnapshot) -> Self {
        Self {
            preparation_id: preparation.preparation_id.as_str().to_string(),
            study_id: preparation.study_id.as_str().to_string(),
            scholar_cat_id: preparation
                .scholar_cat_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            progress_basis_points: preparation.progress_basis_points.get(),
            committed_insight_cost: preparation.committed_insight_cost,
            player_discount_basis_points: preparation.player_discount_basis_points.get(),
            prepared: preparation.prepared,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScholarReassignmentControl {
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScholarTrackStageNavigator {
    pub track_id: String,
    pub stage_count: usize,
    pub owned_stage: u8,
}

impl ScholarTrackStageNavigator {
    fn from_owned_studies(owned_study_ids: &[cat_protocol::NonEmptyStableId]) -> Vec<Self> {
        [
            "divine_duration",
            "divine_economy",
            "rehabilitation",
            "administration",
        ]
        .into_iter()
        .map(|track_id| Self {
            track_id: track_id.to_string(),
            stage_count: SCHOLAR_TRACK_STAGE_COUNT_11,
            owned_stage: owned_track_stage(owned_study_ids, track_id),
        })
        .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivineBoostPanel {
    pub controls: Vec<BoostControlRenderModel>,
    pub no_leader_action: NoLeaderBoostActionGuard,
}

impl DivineBoostPanel {
    fn from_snapshot(
        research: &ResearchFrontierSnapshot,
        boosts: &[DivineBoostSnapshot],
        selected_duration_hours: u16,
    ) -> Self {
        let duration_stage = owned_track_stage(&research.owned_study_ids, "divine_duration");
        let economy_stage = owned_track_stage(&research.owned_study_ids, "divine_economy");
        Self {
            controls: DivineBoostKind::ALL
                .iter()
                .copied()
                .map(|kind| {
                    BoostControlRenderModel::from_snapshot(
                        kind,
                        boosts,
                        duration_stage,
                        economy_stage,
                        selected_duration_hours,
                    )
                })
                .collect(),
            no_leader_action: NoLeaderBoostActionGuard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoostControlRenderModel {
    pub kind: DivineBoostKind,
    pub test_id: StableUiId,
    pub cost: BoostCostMicroFavorLabel,
    pub duration_picker: BoostDurationPicker,
    pub effect_stage: BoostEffectStageLabel,
    pub active_expiry: Option<BoostExpiryTickLabel>,
    pub same_type_disabled: Option<SameTypeActiveBoostDisabledReason>,
    pub active: Option<ActiveBoostRenderModel>,
}

impl BoostControlRenderModel {
    fn from_snapshot(
        kind: DivineBoostKind,
        boosts: &[DivineBoostSnapshot],
        duration_stage: u8,
        economy_stage: u8,
        selected_duration_hours: u16,
    ) -> Self {
        let active = boosts
            .iter()
            .find(|boost| boost.boost_kind.as_str() == kind.protocol_id())
            .map(ActiveBoostRenderModel::from_snapshot);
        Self {
            kind,
            test_id: TestIdBuilder::control(
                UiSection::Progression,
                ControlKind::Activate,
                kind.protocol_id(),
            ),
            cost: BoostCostMicroFavorLabel(boost_cost_micro_favor(
                kind,
                selected_duration_hours,
                economy_stage,
            )),
            duration_picker: BoostDurationPicker {
                selected_hours: selected_duration_hours,
                unlocked_hours: unlocked_duration_options(duration_stage),
            },
            effect_stage: BoostEffectStageLabel {
                duration_stage,
                economy_stage,
                effect_basis_points: 15_000,
            },
            active_expiry: active.as_ref().map(|active| BoostExpiryTickLabel {
                started_at_ms: active.started_at_ms,
                expires_at_ms: active.expires_at_ms,
            }),
            same_type_disabled: active
                .is_some()
                .then_some(SameTypeActiveBoostDisabledReason),
            active,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivineBoostKind {
    BountifulLabor,
    FleetPaws,
    InspiredWork,
    RestorativeGrace,
}

impl DivineBoostKind {
    pub const ALL: [Self; 4] = [
        Self::BountifulLabor,
        Self::FleetPaws,
        Self::InspiredWork,
        Self::RestorativeGrace,
    ];

    pub const fn protocol_id(self) -> &'static str {
        match self {
            Self::BountifulLabor => "bountiful_labor",
            Self::FleetPaws => "fleet_paws",
            Self::InspiredWork => "inspired_work",
            Self::RestorativeGrace => "restorative_grace",
        }
    }

    const fn base_micro_favor_per_hour(self) -> u64 {
        match self {
            Self::FleetPaws => EXACT_ONE_FAVOR_MICRO_FAVOR,
            Self::BountifulLabor | Self::InspiredWork | Self::RestorativeGrace => {
                EXACT_ONE_FAVOR_MICRO_FAVOR * 2
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostBountifulLaborControl;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostFleetPawsControl;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostInspiredWorkControl;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostRestorativeGraceControl;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostCostMicroFavorLabel(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoostDurationPicker {
    pub selected_hours: u16,
    pub unlocked_hours: Vec<u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostEffectStageLabel {
    pub duration_stage: u8,
    pub economy_stage: u8,
    pub effect_basis_points: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoostExpiryTickLabel {
    pub started_at_ms: i64,
    pub expires_at_ms: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SameTypeActiveBoostDisabledReason;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoLeaderBoostActionGuard;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveBoostRenderModel {
    pub boost_id: String,
    pub boost_kind: String,
    pub paid_cost_micro_favor: u64,
    pub duration_ms: u64,
    pub started_at_ms: i64,
    pub expires_at_ms: i64,
    pub effect_stage: u8,
}

impl ActiveBoostRenderModel {
    fn from_snapshot(boost: &DivineBoostSnapshot) -> Self {
        Self {
            boost_id: boost.boost_id.as_str().to_string(),
            boost_kind: boost.boost_kind.as_str().to_string(),
            paid_cost_micro_favor: boost.boost_price_micro_favor,
            duration_ms: boost.duration_ms,
            started_at_ms: boost.boost_started_at_ms,
            expires_at_ms: boost.boost_expires_at_ms,
            effect_stage: boost.effect_stage,
        }
    }
}

fn unlocked_duration_options(stage: u8) -> Vec<u16> {
    let max_index = usize::from(stage).min(BOOST_DURATION_OPTIONS_HOURS.len() - 1);
    BOOST_DURATION_OPTIONS_HOURS[..=max_index].to_vec()
}

fn boost_cost_micro_favor(kind: DivineBoostKind, duration_hours: u16, economy_stage: u8) -> u64 {
    let reduction_percent = u64::from(economy_stage.saturating_mul(3).min(33));
    let numerator = kind
        .base_micro_favor_per_hour()
        .saturating_mul(u64::from(duration_hours))
        .saturating_mul(100 - reduction_percent);
    numerator.div_ceil(100)
}

fn owned_track_stage(owned_study_ids: &[cat_protocol::NonEmptyStableId], track_prefix: &str) -> u8 {
    owned_study_ids
        .iter()
        .filter_map(|id| {
            id.as_str()
                .strip_prefix(track_prefix)
                .and_then(|suffix| suffix.strip_prefix("_stage_"))
                .and_then(|digits| digits.parse::<u8>().ok())
        })
        .max()
        .unwrap_or(0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiplomacyPanel {
    pub relationships: Vec<RelationshipRow>,
    pub expected_version: DiplomacyExpectedVersionAction,
    pub privacy_guard: ForeignPrivateStateRedactionGuard,
}

impl DiplomacyPanel {
    fn from_snapshot(diplomacy: &cat_protocol::DiplomacySnapshot) -> Self {
        Self {
            relationships: diplomacy
                .relationships
                .iter()
                .map(RelationshipRow::from_snapshot)
                .collect(),
            expected_version: DiplomacyExpectedVersionAction(diplomacy.diplomacy_version),
            privacy_guard: ForeignPrivateStateRedactionGuard,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipRow {
    pub relationship_id: String,
    pub other_colony_id: String,
    pub state: String,
    pub consent: RelationshipConsentStateLabel,
    pub alliance_approval: AllianceApprovalControl,
    pub immediate_block: ImmediateBlockControl,
    pub updated_at_ms: i64,
}

impl RelationshipRow {
    fn from_snapshot(relationship: &RelationshipSnapshot) -> Self {
        Self {
            relationship_id: relationship.relationship_id.as_str().to_string(),
            other_colony_id: relationship.other_colony_id.as_str().to_string(),
            state: relationship.state.as_str().to_string(),
            consent: RelationshipConsentStateLabel {
                local_approved: relationship.consent.local_approved,
                remote_approved: relationship.consent.remote_approved,
                consent_version: relationship.consent.consent_version,
            },
            alliance_approval: AllianceApprovalControl {
                test_id: TestIdBuilder::control(
                    UiSection::Progression,
                    ControlKind::Accept,
                    relationship.other_colony_id.as_str(),
                ),
                requires_remote_consent: !relationship.consent.remote_approved,
            },
            immediate_block: ImmediateBlockControl {
                test_id: TestIdBuilder::control(
                    UiSection::Progression,
                    ControlKind::Reject,
                    relationship.other_colony_id.as_str(),
                ),
            },
            updated_at_ms: relationship.updated_at_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationshipConsentStateLabel {
    pub local_approved: bool,
    pub remote_approved: bool,
    pub consent_version: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllianceApprovalControl {
    pub test_id: StableUiId,
    pub requires_remote_consent: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImmediateBlockControl {
    pub test_id: StableUiId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiplomacyExpectedVersionAction(pub u64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiplomacyBoundedConflictFeedback(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignPrivateStateRedactionGuard;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiColonyProgressionPrivacyGuard;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeContractsPanel {
    pub rows: Vec<TradeContractRow>,
}

impl TradeContractsPanel {
    fn from_snapshot(contracts: &[TradeContractSnapshot]) -> Self {
        Self {
            rows: contracts
                .iter()
                .map(TradeContractRow::from_snapshot)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeContractRow {
    pub contract_id: String,
    pub partner_colony_id: String,
    pub value_reports: TradeProposalValueReportRefs,
    pub valuation_confidence: TradeBeliefValuationConfidence,
    pub escrow: TradeEscrowSummary,
    pub route_endpoint: TradeRouteEndpointLabel,
    pub cargo: Vec<TradeCargoStageLabel>,
    pub recovery: TradeRecoveryStateLabel,
    pub accept: ConsentRequiredTradeAcceptButton,
    pub reject: ConsentRequiredTradeRejectButton,
    pub route_block_feedback: Option<TradeRouteBlockBoundedFeedback>,
}

impl TradeContractRow {
    fn from_snapshot(contract: &TradeContractSnapshot) -> Self {
        let route_endpoint = TradeRouteEndpointLabel {
            route_id: contract.route.route_id.as_str().to_string(),
            endpoint_site_id: site_ref_id(&contract.route.endpoint),
            route_tiles: contract.route.ordered_tiles.len(),
        };
        Self {
            contract_id: contract.contract_id.as_str().to_string(),
            partner_colony_id: contract.partner_colony_id.as_str().to_string(),
            value_reports: TradeProposalValueReportRefs(
                contract
                    .valuation_report_ids
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            ),
            valuation_confidence: TradeBeliefValuationConfidence(
                contract.valuation_confidence_basis_points.get(),
            ),
            escrow: TradeEscrowSummary {
                escrow_id: contract.escrow.escrow_id.as_str().to_string(),
                cargo_ids: contract
                    .escrow
                    .cargo_ids
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
                released: contract.escrow.released,
            },
            route_endpoint,
            cargo: contract
                .cargo
                .iter()
                .map(|cargo| TradeCargoStageLabel {
                    cargo_id: cargo.cargo_id.as_str().to_string(),
                    cargo_kind: cargo.cargo_kind.as_str().to_string(),
                    quantity: cargo.quantity,
                    state: cargo.state.as_str().to_string(),
                })
                .collect(),
            recovery: TradeRecoveryStateLabel {
                stage: trade_stage_label(&contract.stage),
                next_event_at_ms: contract.next_event_at_ms,
                bounded_failure: contract
                    .bounded_failure
                    .as_ref()
                    .map(|reason| reason.as_str().to_string()),
                recovery_state: contract
                    .recovery_state
                    .as_ref()
                    .map(|state| state.as_str().to_string()),
            },
            accept: ConsentRequiredTradeAcceptButton {
                test_id: TestIdBuilder::control(
                    UiSection::Progression,
                    ControlKind::Accept,
                    contract.contract_id.as_str(),
                ),
                disabled: !matches!(
                    contract.stage,
                    TradeStageSnapshot::Proposed | TradeStageSnapshot::AwaitingConsent
                ),
            },
            reject: ConsentRequiredTradeRejectButton {
                test_id: TestIdBuilder::control(
                    UiSection::Progression,
                    ControlKind::Reject,
                    contract.contract_id.as_str(),
                ),
                disabled: !matches!(
                    contract.stage,
                    TradeStageSnapshot::Proposed | TradeStageSnapshot::AwaitingConsent
                ),
            },
            route_block_feedback: contract
                .bounded_failure
                .as_ref()
                .map(|reason| TradeRouteBlockBoundedFeedback(reason.as_str().to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeProposalValueReportRefs(pub Vec<String>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeBeliefValuationConfidence(pub u16);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeEscrowSummary {
    pub escrow_id: String,
    pub cargo_ids: Vec<String>,
    pub released: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRouteEndpointLabel {
    pub route_id: String,
    pub endpoint_site_id: String,
    pub route_tiles: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCargoStageLabel {
    pub cargo_id: String,
    pub cargo_kind: String,
    pub quantity: u64,
    pub state: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRecoveryStateLabel {
    pub stage: String,
    pub next_event_at_ms: Option<i64>,
    pub bounded_failure: Option<String>,
    pub recovery_state: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsentRequiredTradeAcceptButton {
    pub test_id: StableUiId,
    pub disabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsentRequiredTradeRejectButton {
    pub test_id: StableUiId,
    pub disabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRouteBlockBoundedFeedback(pub String);

fn trade_stage_label(stage: &TradeStageSnapshot) -> String {
    match stage {
        TradeStageSnapshot::Proposed => "proposed".to_string(),
        TradeStageSnapshot::AwaitingConsent => "awaiting consent".to_string(),
        TradeStageSnapshot::Escrowed => "escrowed".to_string(),
        TradeStageSnapshot::Outbound => "outbound".to_string(),
        TradeStageSnapshot::Returning => "returning".to_string(),
        TradeStageSnapshot::Complete => "complete".to_string(),
        TradeStageSnapshot::Stranded { recovery_task_id } => {
            format!("stranded recovery {}", recovery_task_id.as_str())
        }
        TradeStageSnapshot::Failed { bounded_failure } => {
            format!("failed {}", bounded_failure.as_str())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgressionAction {
    PurchaseResearch {
        study_id: String,
        use_preparation: bool,
        displayed_price_micro_favor: Option<u64>,
    },
    PrepareScholarStudy {
        study_id: String,
        scholar_cat_id: String,
    },
    ActivateDivineBoost {
        boost_kind: DivineBoostKind,
        duration_hours: u16,
        displayed_price_micro_favor: Option<u64>,
    },
    ChangeDiplomacy {
        target_colony_id: String,
        relationship: DiplomacyRelationshipTarget,
    },
    ApproveAlliance {
        target_colony_id: String,
        proposal_id: String,
    },
    BlockColony {
        target_colony_id: String,
        public_reason: Option<String>,
    },
    AcceptTradeContract {
        contract_id: String,
    },
    RejectTradeContract {
        contract_id: String,
        reason: TradeRejectionReason,
    },
}

impl ProgressionAction {
    fn into_payload(self) -> Result<LeaderAiActionPayload, ActionDecodeError> {
        match self {
            Self::PurchaseResearch {
                study_id,
                use_preparation,
                displayed_price_micro_favor,
            } => Ok(LeaderAiActionPayload::PurchaseResearchWithFavor {
                study_id: entity_id(&study_id)?,
                use_preparation,
                displayed_price_micro_favor: displayed_price_micro_favor
                    .map(BoundedFavorAmount::new)
                    .transpose()?,
            }),
            Self::PrepareScholarStudy {
                study_id,
                scholar_cat_id,
            } => Ok(LeaderAiActionPayload::PrepareScholarStudy {
                study_id: entity_id(&study_id)?,
                scholar_cat_id: entity_id(&scholar_cat_id)?,
            }),
            Self::ActivateDivineBoost {
                boost_kind,
                duration_hours,
                displayed_price_micro_favor,
            } => Ok(LeaderAiActionPayload::ActivateDivineBoost {
                boost_kind: entity_id(boost_kind.protocol_id())?,
                duration_hours,
                displayed_price_micro_favor: displayed_price_micro_favor
                    .map(BoundedFavorAmount::new)
                    .transpose()?,
            }),
            Self::ChangeDiplomacy {
                target_colony_id,
                relationship,
            } => Ok(LeaderAiActionPayload::ChangeDiplomacy {
                target_colony_id: colony_id(&target_colony_id)?,
                relationship,
            }),
            Self::ApproveAlliance {
                target_colony_id,
                proposal_id,
            } => Ok(LeaderAiActionPayload::ApproveAlliance {
                target_colony_id: colony_id(&target_colony_id)?,
                proposal_id: entity_id(&proposal_id)?,
            }),
            Self::BlockColony {
                target_colony_id,
                public_reason,
            } => Ok(LeaderAiActionPayload::BlockColony {
                target_colony_id: colony_id(&target_colony_id)?,
                public_reason: public_reason
                    .map(|reason| {
                        ReportSafeString::new(reason)
                            .map_err(|_| ActionDecodeError::MalformedPayload)
                    })
                    .transpose()?,
            }),
            Self::AcceptTradeContract { contract_id } => {
                Ok(LeaderAiActionPayload::AcceptTradeContract {
                    contract_id: entity_id(&contract_id)?,
                })
            }
            Self::RejectTradeContract {
                contract_id,
                reason,
            } => Ok(LeaderAiActionPayload::RejectTradeContract {
                contract_id: entity_id(&contract_id)?,
                reason,
            }),
        }
    }

    const fn version_class(&self) -> ProgressionVersionClass {
        match self {
            Self::PurchaseResearch { .. } => ProgressionVersionClass::ResearchPurchase,
            Self::PrepareScholarStudy { .. } => ProgressionVersionClass::ScholarPreparation,
            Self::ActivateDivineBoost { .. } => ProgressionVersionClass::BoostActivation,
            Self::ChangeDiplomacy { .. }
            | Self::ApproveAlliance { .. }
            | Self::BlockColony { .. } => ProgressionVersionClass::Diplomacy,
            Self::AcceptTradeContract { .. } | Self::RejectTradeContract { .. } => {
                ProgressionVersionClass::Trade
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgressionVersionClass {
    ResearchPurchase,
    ScholarPreparation,
    BoostActivation,
    Diplomacy,
    Trade,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionStableIdempotencyId(pub String);

impl From<StableIdempotencyId> for ProgressionStableIdempotencyId {
    fn from(value: StableIdempotencyId) -> Self {
        Self(value.0)
    }
}

pub type ProgressionAuthenticatedPlayerIdentity = super::AuthenticatedPlayerIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedPlannerVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedResourceVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedResearchVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedScholarVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedBoostVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedDiplomacyVersion(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedTradeVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionExpectedVersionBundle {
    pub planner: ProgressionExpectedPlannerVersion,
    pub resource: ProgressionExpectedResourceVersion,
    pub research: Option<ProgressionExpectedResearchVersion>,
    pub scholar: Option<ProgressionExpectedScholarVersion>,
    pub boost: Option<ProgressionExpectedBoostVersion>,
    pub diplomacy: Option<ProgressionExpectedDiplomacyVersion>,
    pub trade: Option<ProgressionExpectedTradeVersion>,
    pub reservation: Option<u64>,
}

impl ProgressionExpectedVersionBundle {
    fn into_protocol(self) -> ExpectedStateVersions {
        ExpectedStateVersions {
            expected_planner_version: self.planner.0,
            expected_domain_version: 0,
            expected_resource_version: self.resource.0,
            expected_spatial_version: None,
            expected_reservation_version: self.reservation,
            expected_research_version: self.research.map(|version| version.0),
            expected_scholar_version: self.scholar.map(|version| version.0),
            expected_boost_version: self.boost.map(|version| version.0),
            expected_diplomacy_version: self.diplomacy.map(|version| version.0),
            expected_trade_version: self.trade.map(|version| version.0),
            expected_prosthetic_version: None,
            expected_care_version: None,
            expected_officer_version: None,
            expected_standing_order_version: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgressionActionBuildError {
    Action(ActionDecodeError),
    MissingVersion(&'static str),
    LeaderOrOfficerCannotActivateBoost,
}

impl From<ActionDecodeError> for ProgressionActionBuildError {
    fn from(value: ActionDecodeError) -> Self {
        Self::Action(value)
    }
}

pub fn build_progression_action_envelope(
    identity: ProgressionAuthenticatedPlayerIdentity,
    idempotency: ProgressionStableIdempotencyId,
    expected_versions: ProgressionExpectedVersionBundle,
    action: ProgressionAction,
) -> Result<LeaderAiActionEnvelope, ProgressionActionBuildError> {
    require_versions(action.version_class(), expected_versions)?;
    Ok(LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new(idempotency.0)?,
        colony_id: SelectedColonyId::new(identity.colony_id)?,
        player_id: AuthenticatedPlayerId::new(identity.player_id)?,
        expected_versions: expected_versions.into_protocol(),
        payload: action.into_payload()?,
    })
}

fn require_versions(
    class: ProgressionVersionClass,
    versions: ProgressionExpectedVersionBundle,
) -> Result<(), ProgressionActionBuildError> {
    match class {
        ProgressionVersionClass::ResearchPurchase if versions.research.is_none() => {
            Err(ProgressionActionBuildError::MissingVersion("research"))
        }
        ProgressionVersionClass::ScholarPreparation
            if versions.research.is_none() || versions.scholar.is_none() =>
        {
            Err(ProgressionActionBuildError::MissingVersion(
                "research_scholar",
            ))
        }
        ProgressionVersionClass::BoostActivation
            if versions.research.is_none() || versions.boost.is_none() =>
        {
            Err(ProgressionActionBuildError::MissingVersion(
                "research_boost",
            ))
        }
        ProgressionVersionClass::Diplomacy if versions.diplomacy.is_none() => {
            Err(ProgressionActionBuildError::MissingVersion("diplomacy"))
        }
        ProgressionVersionClass::Trade if versions.trade.is_none() => {
            Err(ProgressionActionBuildError::MissingVersion("trade"))
        }
        _ => Ok(()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionStaleRefresh {
    pub refresh_state: ProgressionRefreshState,
    pub selected_row: PreserveProgressionRowAfterRefresh,
    pub feedback: ProgressionTypedFeedback,
}

pub struct ProgressionStaleRefreshHandler;

impl ProgressionStaleRefreshHandler {
    pub fn handle(
        response: &LeaderAiActionResponse,
        selected_row_id: Option<&str>,
        visible_row_ids: &[String],
    ) -> Option<ProgressionStaleRefresh> {
        match &response.result {
            LeaderAiActionResult::Accepted { .. } => None,
            LeaderAiActionResult::DuplicateReplay { replay } => Some(ProgressionStaleRefresh {
                refresh_state: ProgressionRefreshState::Stale,
                selected_row: PreserveProgressionRowAfterRefresh::preserve(
                    selected_row_id,
                    visible_row_ids,
                ),
                feedback: ProgressionTypedFeedback {
                    state: FeedbackState::Stale,
                    message: truncate_report_safe(replay.result_code.as_str()),
                },
            }),
            LeaderAiActionResult::Rejected { conflict } => {
                let refresh_state = match conflict {
                    ActionConflict::UpdateRequired { .. } => {
                        ProgressionRefreshState::UpdateRequired
                    }
                    ActionConflict::VersionMismatch { .. } => ProgressionRefreshState::Stale,
                    _ => ProgressionRefreshState::Error,
                };
                Some(ProgressionStaleRefresh {
                    refresh_state,
                    selected_row: PreserveProgressionRowAfterRefresh::preserve(
                        selected_row_id,
                        visible_row_ids,
                    ),
                    feedback: ProgressionTypedFeedback {
                        state: refresh_state.feedback(),
                        message: bounded_conflict_message(conflict, response.refresh.as_ref()),
                    },
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreserveProgressionRowAfterRefresh(pub Option<String>);

impl PreserveProgressionRowAfterRefresh {
    fn preserve(selected_row_id: Option<&str>, visible_row_ids: &[String]) -> Self {
        Self(selected_row_id.and_then(|row_id| {
            visible_row_ids
                .iter()
                .any(|visible| visible == row_id)
                .then(|| row_id.to_string())
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionTypedFeedback {
    pub state: FeedbackState,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionDuplicateReplayFeedback;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgressionMalformedPayloadFeedback;

fn bounded_conflict_message(
    conflict: &ActionConflict,
    refresh: Option<&StaleClientRefresh>,
) -> String {
    let raw = refresh
        .map(|refresh| refresh.current_state_hint.state_code.as_str())
        .unwrap_or_else(|| match conflict {
            ActionConflict::UpdateRequired { .. } => "UPDATE_REQUIRED",
            ActionConflict::VersionMismatch {
                current_state_hint, ..
            }
            | ActionConflict::InsufficientFavor { current_state_hint }
            | ActionConflict::ReservationConflict { current_state_hint } => {
                current_state_hint.state_code.as_str()
            }
            ActionConflict::PreconditionFailed { reason } => reason.as_str(),
            ActionConflict::Unauthorized => "unauthorized",
            ActionConflict::OwnershipDenied => "ownership denied",
            ActionConflict::AuthorityDenied { .. } => "authority denied",
            ActionConflict::DuplicateReplay { replay } => replay.result_code.as_str(),
            ActionConflict::MalformedActionId
            | ActionConflict::UnknownActionVariant
            | ActionConflict::MalformedPayload => "malformed action",
            ActionConflict::RateLimited { .. } => "rate limited",
            ActionConflict::LeaderCannotActivateBoost => "leader cannot activate boost",
            ActionConflict::OfficerCannotActivateBoost => "officer cannot activate boost",
        });
    truncate_report_safe(raw)
}

fn truncate_report_safe(value: &str) -> String {
    const MAX: usize = 120;
    if value.len() <= MAX {
        value.to_string()
    } else {
        value.chars().take(MAX).collect()
    }
}

fn entity_id(value: &str) -> Result<BoundedEntityId, ActionDecodeError> {
    BoundedEntityId::new(value)
}

fn colony_id(value: &str) -> Result<BoundedColonyId, ActionDecodeError> {
    BoundedColonyId::new(value)
}

fn site_ref_id(site_ref: &SiteRefSnapshot) -> String {
    match site_ref {
        SiteRefSnapshot::Tile { site, .. }
        | SiteRefSnapshot::AnchoredRect { site, .. }
        | SiteRefSnapshot::OrderedTileSet { site, .. }
        | SiteRefSnapshot::BuildingFootprint { site, .. }
        | SiteRefSnapshot::StockpileFootprint { site, .. }
        | SiteRefSnapshot::ResourceSource { site, .. }
        | SiteRefSnapshot::HuntSource { site, .. }
        | SiteRefSnapshot::WaterSourceAndBank { site, .. }
        | SiteRefSnapshot::OrderedRoute { site, .. }
        | SiteRefSnapshot::Shrine { site, .. }
        | SiteRefSnapshot::VillageEndpoint { site, .. }
        | SiteRefSnapshot::TradeEndpoint { site, .. } => site.site_id.as_str().to_string(),
    }
}

#[allow(dead_code)]
fn _progression_identity(_: CurrentStateHint) {}
