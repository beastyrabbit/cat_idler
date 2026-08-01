//! Pure render and action models for the LAI.30 Cat Care UI.
//!
//! Cat care panels are a report-safe projection of LAI.24 snapshots. This file
//! constructs LAI.25 action envelopes but does not run simulation or infer
//! hidden capability, recovery, inventory, or regeneration facts.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use cat_protocol::{
    ActionConflict, ActionDecodeError, ActionIdempotencyId, ActionProtocolVersion,
    AuthenticatedPlayerId, BeliefReportSnapshot, BodyPartSnapshot, BoundedEntityId,
    BoundedPlayerId, CatCareSnapshot, ColonyAiSnapshot, CurrentStateHint, ExpectedStateVersions,
    InjurySnapshot, LeaderAiActionEnvelope, LeaderAiActionPayload, LeaderAiActionResponse,
    LeaderAiActionResult, NamedBasisPointSnapshot, ProstheticSnapshot, RegenerationReportSnapshot,
    ReportSafeString, SelectedColonyId, SiteRefActionTarget, SiteRefSnapshot, StaleClientRefresh,
    TreatmentSnapshot, VisibleTaskSnapshot,
};

use super::RoleColor;
use super::{
    AccessibleLabel, ControlKind, EntityKind, FeedbackState, StableIdempotencyId, StableUiId,
    TestIdBuilder, UiSection,
};

pub const ACCESSIBLE_CAT_CARE_PANEL_LABEL: &str = "Cat care panel";
pub const CAT_CARE_PANEL_TEST_ID_PREFIX: &str = "lai-ui:cats:cat:";
pub const CAT_CARE_BODY_PART_TEST_ID_PREFIX: &str = "lai-ui:cats:cat-anatomy:";
pub const CAT_CARE_CONTROL_TEST_ID_PREFIX: &str = "lai-ui:cats:control:";
pub const CAT_CARE_TASK_REF_TEST_ID_PREFIX: &str = "lai-ui:cats:task-ref:";
pub const PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST: &str = "lai30-cat-care-locators";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL: &str = "lai30-cat-panel";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC: &str =
    "lai30-treatment-prosthetic";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY: &str =
    "lai30-stale-refresh-privacy";

#[derive(Default)]
pub struct CatCarePanelPlugin;

impl Plugin for CatCarePanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CatCarePanelState>()
            .init_resource::<CatCarePanelInput>()
            .init_resource::<CatCarePanelProjectionResource>()
            .add_systems(Update, update_cat_care_panel_projection);
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct CatCarePanelRoot;

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct CatCarePanelState {
    pub selected_cat_id: Option<String>,
    pub draft: Option<CatCareDraft>,
    pub refresh_state: CatCareRefreshState,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct CatCarePanelInput {
    pub selected_colony_id: Option<String>,
    pub colony: Option<ColonyAiSnapshot>,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct CatCarePanelProjectionResource {
    pub projection: Option<CatCarePanelProjection>,
}

pub fn update_cat_care_panel_projection(
    input: Res<'_, CatCarePanelInput>,
    state: Res<'_, CatCarePanelState>,
    mut output: ResMut<'_, CatCarePanelProjectionResource>,
) {
    let Some(colony) = input.colony.as_ref() else {
        output.projection = None;
        return;
    };
    let selected_colony_id = input
        .selected_colony_id
        .as_deref()
        .unwrap_or_else(|| colony.colony_id.as_str());
    let projection = render_cat_care_panel(
        colony,
        selected_colony_id,
        state.selected_cat_id.as_deref(),
        state.refresh_state,
    );
    output.projection = (!projection.cards.is_empty()).then_some(projection);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CatCareRefreshState {
    #[default]
    Current,
    Loading,
    Stale,
    UpdateRequired,
    Error,
}

impl CatCareRefreshState {
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
pub struct CatCarePanelProjection {
    pub colony_id: String,
    pub cards: Vec<CatCareCardRenderModel>,
    pub selected_cat_id: Option<CatCareStableCatId>,
    pub refresh_state: CatCareRefreshState,
    pub layout: CatCarePanelLayoutSpec,
    pub chrome: CatCarePanelChrome,
    pub privacy_guard: CatCareMultiColonyPrivacyGuard,
}

pub fn render_cat_care_panel(
    colony: &ColonyAiSnapshot,
    selected_colony_id: &str,
    selected_cat_id: Option<&str>,
    refresh_state: CatCareRefreshState,
) -> CatCarePanelProjection {
    if !CatCareSelectedColonyFilter::is_selected(colony, selected_colony_id) {
        return CatCarePanelProjection {
            colony_id: colony.colony_id.as_str().to_string(),
            cards: Vec::new(),
            selected_cat_id: None,
            refresh_state,
            layout: CatCarePanelLayoutSpec::default(),
            chrome: CatCarePanelChrome::default(),
            privacy_guard: CatCareMultiColonyPrivacyGuard,
        };
    }
    let tasks_by_id = colony
        .visible_tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    let mut cards = colony
        .cats
        .iter()
        .map(|cat| CatCareCardRenderModel::from_snapshot(cat, &tasks_by_id))
        .collect::<Vec<_>>();
    cards.sort_by(|left, right| left.stable_id.0.cmp(&right.stable_id.0));
    let selected_cat_id = selected_cat_id.and_then(|cat_id| {
        cards
            .iter()
            .any(|card| card.stable_id.0 == cat_id)
            .then(|| CatCareStableCatId(cat_id.to_string()))
    });
    CatCarePanelProjection {
        colony_id: colony.colony_id.as_str().to_string(),
        cards,
        selected_cat_id,
        refresh_state,
        layout: CatCarePanelLayoutSpec::default(),
        chrome: CatCarePanelChrome::default(),
        privacy_guard: CatCareMultiColonyPrivacyGuard,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCarePanelLayoutSpec {
    pub panel_width_px: u16,
    pub compact_panel_width_px: u16,
    pub card_min_height_px: u16,
    pub anatomy_slot_px: u16,
    pub panel_radius_px: u16,
    pub world_first: bool,
}

impl Default for CatCarePanelLayoutSpec {
    fn default() -> Self {
        Self {
            panel_width_px: 420,
            compact_panel_width_px: 320,
            card_min_height_px: 96,
            anatomy_slot_px: 36,
            panel_radius_px: 10,
            world_first: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCarePanelChrome {
    pub paper_role: RoleColor,
    pub border_role: RoleColor,
    pub care_action_role: RoleColor,
    pub injury_role: RoleColor,
    pub prosthetic_role: RoleColor,
}

impl Default for CatCarePanelChrome {
    fn default() -> Self {
        Self {
            paper_role: RoleColor::Paper,
            border_role: RoleColor::Wood,
            care_action_role: RoleColor::Olive,
            injury_role: RoleColor::Danger,
            prosthetic_role: RoleColor::Rust,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareCardRenderModel {
    pub stable_id: CatCareStableCatId,
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub display_name: String,
    pub innate_attributes: MigratedInnateAttributeBreakdown,
    pub learned_skills: LearnedSkillAndOfficeExperienceBreakdown,
    pub personality: PersonalityAxisBreakdown,
    pub acquired_traits: AcquiredTraitBadgeList,
    pub stress: CatStressRecoveryMeter,
    pub refusal: CatRefusalStateBadge,
    pub willingness: CatWillingnessReasonList,
    pub eligibility_reason: Option<CatCareBoundedEligibilityReason>,
    pub typed_block_reason: Option<CatCareTypedBlockReason>,
    pub self_preservation: CatCareSelfPreservationOverrideBadge,
    pub anatomy: CatAnatomyPanel,
    pub prosthetics: CatProstheticPanel,
    pub active_tasks: ActiveCareTaskReferenceList,
    pub controls: CatCareControls,
    pub projection_guard: CatCareReportSafeProjectionOnly,
}

impl CatCareCardRenderModel {
    fn from_snapshot(
        cat: &CatCareSnapshot,
        tasks_by_id: &BTreeMap<&str, &VisibleTaskSnapshot>,
    ) -> Self {
        let stable_id = CatCareStableCatId(cat.cat_id.as_str().to_string());
        let active_tasks = ActiveCareTaskReferenceList::from_snapshot(cat, tasks_by_id);
        let prosthetics =
            CatProstheticPanel::from_snapshots(&cat.prosthetics, &cat.anatomy.body_parts);
        Self {
            test_id: TestIdBuilder::row(UiSection::Cats, EntityKind::Cat, cat.cat_id.as_str()),
            label: AccessibleLabel::panel(UiSection::Cats),
            display_name: cat.display_name.as_str().to_string(),
            innate_attributes: MigratedInnateAttributeBreakdown::from_named(
                &cat.traits.innate_attributes,
            ),
            learned_skills: LearnedSkillAndOfficeExperienceBreakdown {
                learned_skills: named_basis_points(&cat.traits.learned_skills),
                office_experience: named_basis_points(&cat.traits.office_experience),
            },
            personality: PersonalityAxisBreakdown {
                axes: vec![
                    ("sociability".to_string(), cat.personality.sociability.get()),
                    ("diligence".to_string(), cat.personality.diligence.get()),
                    ("courage".to_string(), cat.personality.courage.get()),
                    ("empathy".to_string(), cat.personality.empathy.get()),
                    ("curiosity".to_string(), cat.personality.curiosity.get()),
                ],
            },
            acquired_traits: AcquiredTraitBadgeList(
                cat.traits
                    .acquired_traits
                    .iter()
                    .map(|trait_id| trait_id.as_str().to_string())
                    .collect(),
            ),
            stress: CatStressRecoveryMeter {
                stress_basis_points: cat.stress.stress_basis_points.get(),
                recovery_basis_points: cat.stress.recovery_basis_points.get(),
            },
            refusal: CatRefusalStateBadge {
                refusing: cat.stress.refusing,
                reason: cat
                    .stress
                    .refusal_reason
                    .as_ref()
                    .map(|reason| reason.as_str().to_string()),
            },
            willingness: CatWillingnessReasonList {
                total_basis_points: cat.willingness.total_basis_points.get(),
                factors: named_basis_points(&cat.willingness.factors),
                eligible: cat.willingness.eligible,
            },
            eligibility_reason: cat
                .willingness
                .eligibility_reason
                .as_ref()
                .map(|reason| CatCareBoundedEligibilityReason(reason.as_str().to_string())),
            typed_block_reason: cat
                .willingness
                .eligibility_reason
                .as_ref()
                .map(|reason| CatCareTypedBlockReason(reason.as_str().to_string())),
            self_preservation: CatCareSelfPreservationOverrideBadge {
                visible: cat.stress.refusing && cat.willingness.eligible,
            },
            anatomy: CatAnatomyPanel::from_parts(&cat.anatomy.body_parts),
            prosthetics,
            active_tasks,
            controls: CatCareControls::for_cat(cat),
            stable_id,
            projection_guard: CatCareReportSafeProjectionOnly,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareStableCatId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareSelectedColonyFilter;

impl CatCareSelectedColonyFilter {
    pub fn is_selected(colony: &ColonyAiSnapshot, selected_colony_id: &str) -> bool {
        colony.colony_id.as_str() == selected_colony_id && colony.capabilities.can_view
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareReportSafeProjectionOnly;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigratedInnateAttributeBreakdown(pub Vec<NamedBasisPointRenderModel>);

impl MigratedInnateAttributeBreakdown {
    fn from_named(values: &[NamedBasisPointSnapshot]) -> Self {
        Self(named_basis_points(values))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedSkillAndOfficeExperienceBreakdown {
    pub learned_skills: Vec<NamedBasisPointRenderModel>,
    pub office_experience: Vec<NamedBasisPointRenderModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalityAxisBreakdown {
    pub axes: Vec<(String, u16)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquiredTraitBadgeList(pub Vec<String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedBasisPointRenderModel {
    pub name: String,
    pub value_basis_points: u16,
}

fn named_basis_points(values: &[NamedBasisPointSnapshot]) -> Vec<NamedBasisPointRenderModel> {
    values
        .iter()
        .map(|value| NamedBasisPointRenderModel {
            name: value.name.as_str().to_string(),
            value_basis_points: value.value_basis_points.get(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatStressRecoveryMeter {
    pub stress_basis_points: u16,
    pub recovery_basis_points: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatRefusalStateBadge {
    pub refusing: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatWillingnessReasonList {
    pub total_basis_points: u16,
    pub factors: Vec<NamedBasisPointRenderModel>,
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareBoundedEligibilityReason(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareTypedBlockReason(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareNoHiddenTruthWillingnessRecompute;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareNoHiddenRegenerationProjection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareSelfPreservationOverrideBadge {
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatAnatomyPanel {
    pub grid: FourPawTwoEyeTailAnatomyGrid,
}

impl CatAnatomyPanel {
    fn from_parts(parts: &[BodyPartSnapshot]) -> Self {
        let by_id = parts
            .iter()
            .filter_map(|part| {
                canonical_part_key(part.body_part_id.as_str()).map(|slot| (slot, part))
            })
            .collect::<BTreeMap<_, _>>();
        let slots = BodyPartSlot::ORDER
            .iter()
            .copied()
            .map(|slot| AnatomySlot::from_part(slot, by_id.get(&slot).copied()))
            .collect();
        Self {
            grid: FourPawTwoEyeTailAnatomyGrid { slots },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FourPawTwoEyeTailAnatomyGrid {
    pub slots: Vec<AnatomySlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnatomySlot {
    pub slot: BodyPartSlot,
    pub body_part_id: Option<String>,
    pub label: String,
    pub side: Option<String>,
    pub functional_basis_points: Option<u16>,
    pub injury: Option<CatInjuryTreatmentState>,
    pub prosthetic_id: Option<String>,
    pub test_id: String,
}

impl AnatomySlot {
    fn from_part(slot: BodyPartSlot, part: Option<&BodyPartSnapshot>) -> Self {
        Self {
            slot,
            body_part_id: part.map(|part| part.body_part_id.as_str().to_string()),
            label: slot.label().to_string(),
            side: part.and_then(|part| part.side.as_ref().map(|side| side.as_str().to_string())),
            functional_basis_points: part.map(|part| part.functional_basis_points.get()),
            injury: part.and_then(|part| {
                part.injury
                    .as_ref()
                    .map(CatInjuryTreatmentState::from_snapshot)
            }),
            prosthetic_id: part
                .and_then(|part| part.prosthetic_id.as_ref())
                .map(|id| id.as_str().to_string()),
            test_id: format!("{CAT_CARE_BODY_PART_TEST_ID_PREFIX}{}", slot.key()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BodyPartSlot {
    FrontLeftPaw,
    FrontRightPaw,
    HindLeftPaw,
    HindRightPaw,
    LeftEye,
    RightEye,
    Tail,
}

impl BodyPartSlot {
    const ORDER: [Self; 7] = [
        Self::FrontLeftPaw,
        Self::FrontRightPaw,
        Self::HindLeftPaw,
        Self::HindRightPaw,
        Self::LeftEye,
        Self::RightEye,
        Self::Tail,
    ];

    const fn key(self) -> &'static str {
        match self {
            Self::FrontLeftPaw => "front_left_paw",
            Self::FrontRightPaw => "front_right_paw",
            Self::HindLeftPaw => "hind_left_paw",
            Self::HindRightPaw => "hind_right_paw",
            Self::LeftEye => "left_eye",
            Self::RightEye => "right_eye",
            Self::Tail => "tail",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::FrontLeftPaw => "left front paw",
            Self::FrontRightPaw => "right front paw",
            Self::HindLeftPaw => "left rear paw",
            Self::HindRightPaw => "right rear paw",
            Self::LeftEye => "left eye",
            Self::RightEye => "right eye",
            Self::Tail => "tail",
        }
    }
}

fn canonical_part_key(raw: &str) -> Option<BodyPartSlot> {
    match raw {
        "front_left_paw" | "left_front_paw" => Some(BodyPartSlot::FrontLeftPaw),
        "front_right_paw" | "right_front_paw" => Some(BodyPartSlot::FrontRightPaw),
        "hind_left_paw" | "rear_left_paw" | "left_rear_paw" => Some(BodyPartSlot::HindLeftPaw),
        "hind_right_paw" | "rear_right_paw" | "right_rear_paw" => Some(BodyPartSlot::HindRightPaw),
        "left_eye" => Some(BodyPartSlot::LeftEye),
        "right_eye" => Some(BodyPartSlot::RightEye),
        "tail" => Some(BodyPartSlot::Tail),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftFrontPawStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RightFrontPawStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftRearPawStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RightRearPawStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftEyeStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RightEyeStateLabel;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TailStateLabel;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatInjuryTreatmentState {
    pub injury_id: String,
    pub injury_kind: String,
    pub severity_basis_points: u16,
    pub sustained_at_ms: i64,
    pub treatment: Option<TreatmentHoursRemainingLabel>,
}

impl CatInjuryTreatmentState {
    fn from_snapshot(injury: &InjurySnapshot) -> Self {
        Self {
            injury_id: injury.injury_id.as_str().to_string(),
            injury_kind: injury.injury_kind.as_str().to_string(),
            severity_basis_points: injury.severity_basis_points.get(),
            sustained_at_ms: injury.sustained_at_ms,
            treatment: injury
                .treatment
                .as_ref()
                .map(TreatmentHoursRemainingLabel::from_snapshot),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreatmentHoursRemainingLabel {
    pub treatment_id: String,
    pub stage: String,
    pub medic_cat_id: Option<String>,
    pub care_site: Option<CareTaskSiteRefLabel>,
    pub task_id: Option<String>,
}

impl TreatmentHoursRemainingLabel {
    fn from_snapshot(treatment: &TreatmentSnapshot) -> Self {
        Self {
            treatment_id: treatment.treatment_id.as_str().to_string(),
            stage: treatment.stage.as_str().to_string(),
            medic_cat_id: treatment
                .medic_cat_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            care_site: treatment
                .care_site
                .as_ref()
                .map(CareTaskSiteRefLabel::from_site_ref),
            task_id: treatment.task_id.as_ref().map(|id| id.as_str().to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatProstheticPanel {
    pub prosthetics: Vec<FittedProstheticRenderModel>,
}

impl CatProstheticPanel {
    fn from_snapshots(prosthetics: &[ProstheticSnapshot], parts: &[BodyPartSnapshot]) -> Self {
        let side_by_part = parts
            .iter()
            .filter_map(|part| {
                part.side
                    .as_ref()
                    .map(|side| (part.body_part_id.as_str(), side.as_str()))
            })
            .collect::<BTreeMap<_, _>>();
        Self {
            prosthetics: prosthetics
                .iter()
                .map(|prosthetic| FittedProstheticRenderModel {
                    stable_item_id: FittedProstheticStableItemId(
                        prosthetic.prosthetic_id.as_str().to_string(),
                    ),
                    side: FittedProstheticSideLabel(
                        side_by_part
                            .get(prosthetic.body_part_id.as_str())
                            .copied()
                            .unwrap_or("unspecified")
                            .to_string(),
                    ),
                    prosthetic_type: FittedProstheticTypeLabel(
                        prosthetic.prosthetic_kind.as_str().to_string(),
                    ),
                    restoration_percent: FittedProstheticRestorationPercent(
                        prosthetic.restoration_basis_points.get() / 100,
                    ),
                    durability_hours: FittedProstheticDurabilityHours(
                        prosthetic.wear.durability_basis_points.get() / 100,
                    ),
                    wear_progress: FittedProstheticWearProgress(
                        prosthetic.wear.wear_basis_points.get(),
                    ),
                    adaptation_progress: ProstheticAdaptationProgress(
                        if prosthetic.restoration_basis_points.get() >= 9_000 {
                            10_000
                        } else {
                            prosthetic.restoration_basis_points.get()
                        },
                    ),
                    restoration_cap: ProstheticRestorationCapLabel(90),
                    fitting_task_id: prosthetic
                        .fitting_task_id
                        .as_ref()
                        .map(|id| id.as_str().to_string()),
                    repair_task_id: prosthetic
                        .repair_task_id
                        .as_ref()
                        .map(|id| id.as_str().to_string()),
                    repair_eligible: prosthetic.wear.repair_eligible,
                    repair_reason: prosthetic
                        .wear
                        .repair_reason
                        .as_ref()
                        .map(|reason| reason.as_str().to_string()),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FittedProstheticRenderModel {
    pub stable_item_id: FittedProstheticStableItemId,
    pub side: FittedProstheticSideLabel,
    pub prosthetic_type: FittedProstheticTypeLabel,
    pub restoration_percent: FittedProstheticRestorationPercent,
    pub durability_hours: FittedProstheticDurabilityHours,
    pub wear_progress: FittedProstheticWearProgress,
    pub adaptation_progress: ProstheticAdaptationProgress,
    pub restoration_cap: ProstheticRestorationCapLabel,
    pub fitting_task_id: Option<String>,
    pub repair_task_id: Option<String>,
    pub repair_eligible: bool,
    pub repair_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FittedProstheticStableItemId(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FittedProstheticSideLabel(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FittedProstheticTypeLabel(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FittedProstheticRestorationPercent(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FittedProstheticDurabilityHours(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FittedProstheticWearProgress(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProstheticAdaptationProgress(pub u16);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProstheticRestorationCapLabel(pub u16);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveCareTaskReferenceList(pub Vec<CareTaskReference>);

impl ActiveCareTaskReferenceList {
    fn from_snapshot(
        cat: &CatCareSnapshot,
        tasks_by_id: &BTreeMap<&str, &VisibleTaskSnapshot>,
    ) -> Self {
        let mut ids = BTreeSet::new();
        if let Some(task_id) = &cat.active_task_id {
            ids.insert(task_id.as_str());
        }
        if let Some(task_id) = &cat.care.treatment_task_id {
            ids.insert(task_id.as_str());
        }
        if let Some(task_id) = &cat.care.fitting_task_id {
            ids.insert(task_id.as_str());
        }
        if let Some(task_id) = &cat.care.repair_task_id {
            ids.insert(task_id.as_str());
        }
        Self(
            ids.into_iter()
                .filter_map(|task_id| {
                    tasks_by_id
                        .get(task_id)
                        .map(|task| CareTaskReference::from_task(cat.cat_id.as_str(), task))
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskReference {
    pub task_id: String,
    pub site: CareTaskSiteRefLabel,
    pub cargo: CareTaskCargoReferenceLabel,
    pub patient: CareTaskTreatmentPatientRef,
    pub fitter_or_medic: CareTaskFitterOrMedicRef,
    pub workshop: CareTaskWorkshopRepairRef,
    pub test_id: String,
}

impl CareTaskReference {
    fn from_task(cat_id: &str, task: &VisibleTaskSnapshot) -> Self {
        Self {
            task_id: task.task_id.as_str().to_string(),
            site: CareTaskSiteRefLabel::from_site_ref(&task.objective),
            cargo: CareTaskCargoReferenceLabel(
                task.cargo
                    .cargo_ids
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            ),
            patient: CareTaskTreatmentPatientRef(cat_id.to_string()),
            fitter_or_medic: CareTaskFitterOrMedicRef(
                task.assigned_cat_ids
                    .first()
                    .map(|id| id.as_str().to_string()),
            ),
            workshop: CareTaskWorkshopRepairRef(task.endpoint.as_ref().map(site_ref_id)),
            test_id: format!(
                "{CAT_CARE_TASK_REF_TEST_ID_PREFIX}{}:{}",
                cat_id,
                task.task_id.as_str()
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskSiteRefLabel {
    pub site_id: String,
    pub site_kind: String,
}

impl CareTaskSiteRefLabel {
    fn from_site_ref(site_ref: &SiteRefSnapshot) -> Self {
        Self {
            site_id: site_ref_id(site_ref),
            site_kind: site_ref_kind(site_ref).to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskCargoReferenceLabel(pub Vec<String>);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskTreatmentPatientRef(pub String);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskFitterOrMedicRef(pub Option<String>);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTaskWorkshopRepairRef(pub Option<String>);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CareItemCargoIdentityConservationGuard;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatCareMultiColonyPrivacyGuard;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareControls {
    pub treatment: CareTreatmentActionButton,
    pub consent: CareConsentActionButton,
    pub refusal_acknowledge: CareRefusalAcknowledgeButton,
    pub prosthetic_fit: ProstheticFitActionButton,
    pub prosthetic_remove: ProstheticRemoveActionButton,
    pub prosthetic_repair: ProstheticRepairActionButton,
}

impl CatCareControls {
    fn for_cat(cat: &CatCareSnapshot) -> Self {
        let disabled_reason = (!cat.willingness.eligible).then(|| {
            CatCareControlDisabledReason::Blocked(
                cat.willingness
                    .eligibility_reason
                    .as_ref()
                    .map(|reason| reason.as_str().to_string())
                    .unwrap_or_else(|| "not eligible".to_string()),
            )
        });
        Self {
            treatment: CareTreatmentActionButton::new(cat.cat_id.as_str(), disabled_reason.clone()),
            consent: CareConsentActionButton::new(cat.cat_id.as_str(), disabled_reason.clone()),
            refusal_acknowledge: CareRefusalAcknowledgeButton::new(cat.cat_id.as_str()),
            prosthetic_fit: ProstheticFitActionButton::new(
                cat.cat_id.as_str(),
                disabled_reason.clone(),
            ),
            prosthetic_remove: ProstheticRemoveActionButton::new(
                cat.cat_id.as_str(),
                Some(CatCareControlDisabledReason::NoProtocolAction),
            ),
            prosthetic_repair: ProstheticRepairActionButton::new(
                cat.cat_id.as_str(),
                disabled_reason,
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareTreatmentActionButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<CatCareControlDisabledReason>,
}

impl CareTreatmentActionButton {
    fn new(cat_id: &str, disabled_reason: Option<CatCareControlDisabledReason>) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Treat, cat_id),
            label: AccessibleLabel::control(ControlKind::Treat, cat_id),
            disabled_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareConsentActionButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<CatCareControlDisabledReason>,
}

impl CareConsentActionButton {
    fn new(cat_id: &str, disabled_reason: Option<CatCareControlDisabledReason>) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Consent, cat_id),
            label: AccessibleLabel::control(ControlKind::Consent, cat_id),
            disabled_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareRefusalAcknowledgeButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
}

impl CareRefusalAcknowledgeButton {
    fn new(cat_id: &str) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Dismiss, cat_id),
            label: AccessibleLabel::control(ControlKind::Dismiss, cat_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProstheticFitActionButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<CatCareControlDisabledReason>,
}

impl ProstheticFitActionButton {
    fn new(cat_id: &str, disabled_reason: Option<CatCareControlDisabledReason>) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Fit, cat_id),
            label: AccessibleLabel::control(ControlKind::Fit, cat_id),
            disabled_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProstheticRemoveActionButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<CatCareControlDisabledReason>,
}

impl ProstheticRemoveActionButton {
    fn new(cat_id: &str, disabled_reason: Option<CatCareControlDisabledReason>) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Remove, cat_id),
            label: AccessibleLabel::control(ControlKind::Remove, cat_id),
            disabled_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProstheticRepairActionButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<CatCareControlDisabledReason>,
}

impl ProstheticRepairActionButton {
    fn new(cat_id: &str, disabled_reason: Option<CatCareControlDisabledReason>) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Cats, ControlKind::Repair, cat_id),
            label: AccessibleLabel::control(ControlKind::Repair, cat_id),
            disabled_reason,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatCareControlDisabledReason {
    Stale,
    UpdateRequired,
    Blocked(String),
    MissingVersion(&'static str),
    MalformedInput,
    NoProtocolAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareTypedFeedbackToast {
    pub state: FeedbackState,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatCareAction {
    RequestTreatment {
        cat_id: String,
        injury_id: String,
        treatment_kind: String,
    },
    FitProsthetic {
        cat_id: String,
        prosthetic_id: String,
        body_part_id: String,
        fitting_site: SiteRefActionTarget,
        fitter_cat_id: Option<String>,
    },
    RepairProsthetic {
        prosthetic_id: String,
        workshop_id: String,
        input_reservation_id: String,
    },
}

impl CatCareAction {
    fn into_payload(self) -> Result<LeaderAiActionPayload, ActionDecodeError> {
        match self {
            Self::RequestTreatment {
                cat_id,
                injury_id,
                treatment_kind,
            } => Ok(LeaderAiActionPayload::RequestTreatment {
                cat_id: entity_id(&cat_id)?,
                injury_id: entity_id(&injury_id)?,
                treatment_kind: entity_id(&treatment_kind)?,
            }),
            Self::FitProsthetic {
                cat_id,
                prosthetic_id,
                body_part_id,
                fitting_site,
                fitter_cat_id,
            } => Ok(LeaderAiActionPayload::FitProsthetic {
                cat_id: entity_id(&cat_id)?,
                prosthetic_id: entity_id(&prosthetic_id)?,
                body_part_id: entity_id(&body_part_id)?,
                fitting_site,
                fitter_cat_id: fitter_cat_id.map(|id| entity_id(&id)).transpose()?,
            }),
            Self::RepairProsthetic {
                prosthetic_id,
                workshop_id,
                input_reservation_id,
            } => Ok(LeaderAiActionPayload::RepairProsthetic {
                prosthetic_id: entity_id(&prosthetic_id)?,
                workshop_id: entity_id(&workshop_id)?,
                input_reservation_id: entity_id(&input_reservation_id)?,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedCatCareVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedProstheticVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedCatCareVersionBundle {
    pub planner_version: u64,
    pub domain_version: u64,
    pub resource_version: u64,
    pub care: ExpectedCatCareVersion,
    pub prosthetic: Option<ExpectedProstheticVersion>,
    pub spatial_version: Option<u64>,
    pub reservation_version: Option<u64>,
}

impl ExpectedCatCareVersionBundle {
    fn into_protocol(self) -> ExpectedStateVersions {
        ExpectedStateVersions {
            expected_planner_version: self.planner_version,
            expected_domain_version: self.domain_version,
            expected_resource_version: self.resource_version,
            expected_spatial_version: self.spatial_version,
            expected_reservation_version: self.reservation_version,
            expected_research_version: None,
            expected_scholar_version: None,
            expected_boost_version: None,
            expected_diplomacy_version: None,
            expected_trade_version: None,
            expected_prosthetic_version: self.prosthetic.map(|version| version.0),
            expected_care_version: Some(self.care.0),
            expected_officer_version: None,
            expected_standing_order_version: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatCareActionBuildError {
    Action(ActionDecodeError),
    MissingVersion(&'static str),
}

impl From<ActionDecodeError> for CatCareActionBuildError {
    fn from(value: ActionDecodeError) -> Self {
        Self::Action(value)
    }
}

pub fn build_cat_care_action_envelope(
    identity: CatCareAuthenticatedPlayerIdentity,
    idempotency: StableIdempotencyId,
    expected_versions: ExpectedCatCareVersionBundle,
    action: CatCareAction,
) -> Result<LeaderAiActionEnvelope, CatCareActionBuildError> {
    if matches!(
        action,
        CatCareAction::FitProsthetic { .. } | CatCareAction::RepairProsthetic { .. }
    ) && (expected_versions.prosthetic.is_none()
        || expected_versions.spatial_version.is_none()
        || expected_versions.reservation_version.is_none())
    {
        return Err(CatCareActionBuildError::MissingVersion(
            "prosthetic_spatial_reservation",
        ));
    }
    Ok(LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new(idempotency.0)?,
        colony_id: SelectedColonyId::new(identity.colony_id)?,
        player_id: AuthenticatedPlayerId::new(identity.player_id)?,
        expected_versions: expected_versions.into_protocol(),
        payload: action.into_payload()?,
    })
}

pub type CatCareAuthenticatedPlayerIdentity = super::AuthenticatedPlayerIdentity;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareActionConflictRefresh {
    pub refresh_state: CatCareRefreshState,
    pub selected_cat: PreserveSelectedCatAfterRefresh,
    pub draft: PreserveCareDraftAfterRefresh,
    pub feedback: CatCareTypedFeedbackToast,
}

pub struct CatCareVersionMismatchRefreshHandler;

impl CatCareVersionMismatchRefreshHandler {
    pub fn handle(
        response: &LeaderAiActionResponse,
        selected_cat_id: Option<&str>,
        draft: Option<CatCareDraft>,
        visible_cat_ids: &[String],
    ) -> Option<CatCareActionConflictRefresh> {
        match &response.result {
            LeaderAiActionResult::Accepted { .. } => None,
            LeaderAiActionResult::DuplicateReplay { replay } => {
                Some(CatCareActionConflictRefresh {
                    refresh_state: CatCareRefreshState::Stale,
                    selected_cat: PreserveSelectedCatAfterRefresh::preserve(
                        selected_cat_id,
                        visible_cat_ids,
                    ),
                    draft: PreserveCareDraftAfterRefresh(draft),
                    feedback: CatCareTypedFeedbackToast {
                        state: FeedbackState::Stale,
                        message: truncate_report_safe(replay.result_code.as_str()),
                    },
                })
            }
            LeaderAiActionResult::Rejected { conflict } => {
                let refresh_state = match conflict {
                    ActionConflict::UpdateRequired { .. } => CatCareRefreshState::UpdateRequired,
                    ActionConflict::VersionMismatch { .. } => CatCareRefreshState::Stale,
                    _ => CatCareRefreshState::Error,
                };
                Some(CatCareActionConflictRefresh {
                    refresh_state,
                    selected_cat: PreserveSelectedCatAfterRefresh::preserve(
                        selected_cat_id,
                        visible_cat_ids,
                    ),
                    draft: PreserveCareDraftAfterRefresh(draft),
                    feedback: CatCareTypedFeedbackToast {
                        state: refresh_state.feedback(),
                        message: bounded_conflict_message(conflict, response.refresh.as_ref()),
                    },
                })
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreserveSelectedCatAfterRefresh(pub Option<String>);

impl PreserveSelectedCatAfterRefresh {
    fn preserve(selected_cat_id: Option<&str>, visible_cat_ids: &[String]) -> Self {
        Self(selected_cat_id.and_then(|cat_id| {
            visible_cat_ids
                .iter()
                .any(|visible| visible == cat_id)
                .then(|| cat_id.to_string())
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreserveCareDraftAfterRefresh(pub Option<CatCareDraft>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedCatSelectionClearsSafely(pub bool);

impl RemovedCatSelectionClearsSafely {
    pub fn from_refresh(selected: &PreserveSelectedCatAfterRefresh) -> Self {
        Self(selected.0.is_none())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuplicateCareReplayUsesOriginalResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatCareDraft {
    pub cat_id: String,
    pub target_id: String,
    pub action_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatCareRegenerationProjection {
    UnavailableBelowLevel4,
    EstimatedRange {
        minimum: i64,
        maximum: i64,
        unit: String,
        provenance_count: usize,
    },
}

pub fn project_cat_care_regeneration_report(
    report: &BeliefReportSnapshot,
) -> CatCareRegenerationProjection {
    match &report.regeneration {
        RegenerationReportSnapshot::Estimated {
            estimate,
            provenance,
            ..
        } if report.report_level >= 4 => CatCareRegenerationProjection::EstimatedRange {
            minimum: estimate.minimum,
            maximum: estimate.maximum,
            unit: estimate.unit.as_str().to_string(),
            provenance_count: provenance.source_report_ids.len(),
        },
        _ => CatCareRegenerationProjection::UnavailableBelowLevel4,
    }
}

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

fn site_ref_kind(site_ref: &SiteRefSnapshot) -> &'static str {
    match site_ref {
        SiteRefSnapshot::Tile { .. } => "tile",
        SiteRefSnapshot::AnchoredRect { .. } => "anchored rect",
        SiteRefSnapshot::OrderedTileSet { .. } => "ordered tile set",
        SiteRefSnapshot::BuildingFootprint { .. } => "building footprint",
        SiteRefSnapshot::StockpileFootprint { .. } => "stockpile footprint",
        SiteRefSnapshot::ResourceSource { .. } => "resource source",
        SiteRefSnapshot::HuntSource { .. } => "hunt source",
        SiteRefSnapshot::WaterSourceAndBank { .. } => "water source and bank",
        SiteRefSnapshot::OrderedRoute { .. } => "ordered route",
        SiteRefSnapshot::Shrine { .. } => "shrine",
        SiteRefSnapshot::VillageEndpoint { .. } => "village endpoint",
        SiteRefSnapshot::TradeEndpoint { .. } => "trade endpoint",
    }
}

#[allow(dead_code)]
fn _bounded_identity(_: BoundedPlayerId, _: ReportSafeString, _: CurrentStateHint) {}
