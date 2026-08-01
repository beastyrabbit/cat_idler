//! Strict action DTOs and version preflight for the leader-AI cutover.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::{
    BoundedBasisPoints, BuildingType, CropKind, OfficerRole, PROTOCOL_VERSION, ReportSafeString,
    ResourceKind, SnapshotTilePoint, TransportMode,
};

const MAX_ACTION_ID_BYTES: usize = 128;
const MAX_PRINCIPAL_ID_BYTES: usize = 128;
const MAX_ENTITY_ID_BYTES: usize = 128;
const MAX_STANDING_ORDER_BYTES: usize = 512;
const MAX_VILLAGE_NAME_CHARS: usize = 48;
const MAX_PLACEMENT_DIMENSION: u16 = 64;
const MAX_PLACEMENT_TILES: usize = 4_096;
const MAX_TRADE_AMOUNT: u64 = 1_000_000_000;
const MAX_FAVOR_AMOUNT: u64 = 1_000_000_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDecodeError {
    UnsupportedProtocolVersion(u32),
    UnknownActionVariant,
    MalformedActionId,
    MalformedPayload,
    InvalidBounds(&'static str),
}

impl fmt::Display for ActionDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported action protocol version {version}")
            }
            Self::UnknownActionVariant => formatter.write_str("unknown action variant"),
            Self::MalformedActionId => formatter.write_str("malformed action id"),
            Self::MalformedPayload => formatter.write_str("malformed action payload"),
            Self::InvalidBounds(field) => write!(formatter, "invalid action bounds: {field}"),
        }
    }
}

impl std::error::Error for ActionDecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionProtocolVersion(u32);

impl ActionProtocolVersion {
    #[must_use]
    pub const fn current() -> Self {
        Self(PROTOCOL_VERSION)
    }

    pub fn new(version: u32) -> Result<Self, ActionDecodeError> {
        reject_unknown_action_version(version)?;
        Ok(Self(version))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Serialize for ActionProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for ActionProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u32::deserialize(deserializer)?;
        Self::new(version).map_err(D::Error::custom)
    }
}

pub fn reject_unknown_action_version(version: u32) -> Result<(), ActionDecodeError> {
    if version == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(ActionDecodeError::UnsupportedProtocolVersion(version))
    }
}

macro_rules! bounded_stable_id {
    ($name:ident, $maximum:expr, $error:expr, $allow_component_delimiter:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ActionDecodeError> {
                let value = value.into();
                let valid = !value.is_empty()
                    && value.len() <= $maximum
                    && value.split(':').all(|segment| {
                        !segment.is_empty()
                            && segment.chars().all(|character| {
                                character.is_ascii_alphanumeric()
                                    || character == '_'
                                    || character == '-'
                                    // Planner-owned entity IDs use a
                                    // length-prefixed `|<bytes>:<component>`
                                    // encoding. The delimiter is part of the
                                    // canonical persisted identity and must
                                    // round-trip in action targets. It remains
                                    // forbidden for action/principal IDs.
                                    || ($allow_component_delimiter && character == '|')
                            })
                    });
                if valid { Ok(Self(value)) } else { Err($error) }
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(D::Error::custom)
            }
        }
    };
}

bounded_stable_id!(
    BoundedActionId,
    MAX_ACTION_ID_BYTES,
    ActionDecodeError::MalformedActionId,
    false
);
bounded_stable_id!(
    BoundedPlayerId,
    MAX_PRINCIPAL_ID_BYTES,
    ActionDecodeError::InvalidBounds("player_id"),
    false
);
bounded_stable_id!(
    BoundedColonyId,
    MAX_PRINCIPAL_ID_BYTES,
    ActionDecodeError::InvalidBounds("colony_id"),
    false
);
bounded_stable_id!(
    BoundedEntityId,
    MAX_ENTITY_ID_BYTES,
    ActionDecodeError::InvalidBounds("entity_id"),
    true
);

pub type ActionIdempotencyId = BoundedActionId;
pub type SelectedColonyId = BoundedColonyId;
pub type AuthenticatedPlayerId = BoundedPlayerId;

pub fn reject_malformed_idempotency_id(value: &str) -> Result<(), ActionDecodeError> {
    BoundedActionId::new(value).map(|_| ())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BoundedStandingOrderText(String);

impl BoundedStandingOrderText {
    pub fn new(value: impl Into<String>) -> Result<Self, ActionDecodeError> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > MAX_STANDING_ORDER_BYTES
            || value.chars().any(char::is_control)
        {
            Err(ActionDecodeError::InvalidBounds("standing_order_text"))
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BoundedVillageName(String);

impl BoundedVillageName {
    pub fn new(value: impl Into<String>) -> Result<Self, ActionDecodeError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.chars().count() > MAX_VILLAGE_NAME_CHARS
            || trimmed.chars().any(char::is_control)
        {
            Err(ActionDecodeError::InvalidBounds("village_name"))
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for BoundedVillageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl<'de> Deserialize<'de> for BoundedStandingOrderText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BoundedBasisPointNudge(i16);

impl BoundedBasisPointNudge {
    pub fn new(value: i16) -> Result<Self, ActionDecodeError> {
        if matches!(value, -1_500 | 1_500) {
            Ok(Self(value))
        } else {
            Err(ActionDecodeError::InvalidBounds("plan_nudge"))
        }
    }

    #[must_use]
    pub const fn get(self) -> i16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for BoundedBasisPointNudge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i16::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

macro_rules! bounded_positive_amount {
    ($name:ident, $maximum:expr, $label:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub fn new(value: u64) -> Result<Self, ActionDecodeError> {
                if (1..=$maximum).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(ActionDecodeError::InvalidBounds($label))
                }
            }

            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = u64::deserialize(deserializer)?;
                Self::new(value).map_err(D::Error::custom)
            }
        }
    };
}

bounded_positive_amount!(BoundedFavorAmount, MAX_FAVOR_AMOUNT, "favor_amount");
bounded_positive_amount!(BoundedTradeAmount, MAX_TRADE_AMOUNT, "trade_amount");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedStateVersions {
    pub expected_planner_version: u64,
    pub expected_domain_version: u64,
    pub expected_resource_version: u64,
    pub expected_spatial_version: Option<u64>,
    pub expected_reservation_version: Option<u64>,
    pub expected_research_version: Option<u64>,
    pub expected_scholar_version: Option<u64>,
    pub expected_boost_version: Option<u64>,
    pub expected_diplomacy_version: Option<u64>,
    pub expected_trade_version: Option<u64>,
    pub expected_prosthetic_version: Option<u64>,
    pub expected_care_version: Option<u64>,
    pub expected_officer_version: Option<u64>,
    pub expected_standing_order_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LeaderAiActionEnvelope {
    pub protocol_version: ActionProtocolVersion,
    pub idempotency_id: ActionIdempotencyId,
    pub colony_id: SelectedColonyId,
    pub player_id: AuthenticatedPlayerId,
    pub expected_versions: ExpectedStateVersions,
    pub payload: LeaderAiActionPayload,
}

impl LeaderAiActionEnvelope {
    /// Preflight protocol and action tag before decoding identity or domain data.
    pub fn decode_json(encoded: &str) -> Result<Self, ActionDecodeError> {
        let value: serde_json::Value =
            serde_json::from_str(encoded).map_err(|_| ActionDecodeError::MalformedPayload)?;
        let object = value
            .as_object()
            .ok_or(ActionDecodeError::MalformedPayload)?;
        let version = object
            .get("protocolVersion")
            .and_then(serde_json::Value::as_u64)
            .and_then(|version| u32::try_from(version).ok())
            .ok_or(ActionDecodeError::MalformedPayload)?;
        reject_unknown_action_version(version)?;
        let action_id = object
            .get("idempotencyId")
            .and_then(serde_json::Value::as_str)
            .ok_or(ActionDecodeError::MalformedActionId)?;
        reject_malformed_idempotency_id(action_id)?;
        let action = object
            .get("payload")
            .and_then(serde_json::Value::as_object)
            .and_then(|payload| payload.get("action"))
            .and_then(serde_json::Value::as_str)
            .ok_or(ActionDecodeError::MalformedPayload)?;
        reject_unknown_action_variant(action)?;
        let envelope: Self =
            serde_json::from_value(value).map_err(|_| ActionDecodeError::MalformedPayload)?;
        validate_lai25_action_bounds(&envelope)?;
        Ok(envelope)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LeaderAiActionPayload {
    /// Change the authoritative selected colony for this authenticated socket.
    ///
    /// `envelope.colony_id` remains the current selection used for optimistic
    /// version checks; the target receives an independent ownership check.
    SelectColony {
        target_colony_id: BoundedColonyId,
    },
    /// Found (or resume) the authenticated player's one personal village.
    ///
    /// This is a typed LAI.25 lifecycle mutation so the production client never
    /// falls back to the retired legacy `FoundVillage` wire shape.
    FoundVillage {
        display_name: BoundedVillageName,
    },
    NudgePlan {
        plan_id: BoundedEntityId,
        nudge: BoundedBasisPointNudge,
        reason_key: Option<BoundedEntityId>,
    },
    CreateStandingOrder {
        order_kind: BoundedEntityId,
        domain: BoundedEntityId,
        target_id: Option<BoundedEntityId>,
        instruction: BoundedStandingOrderText,
        priority_basis_points: BoundedBasisPoints,
        expires_at_ms: Option<i64>,
    },
    UpdateStandingOrder {
        standing_order_id: BoundedEntityId,
        patch: StandingOrderPatch,
    },
    DeleteStandingOrder {
        standing_order_id: BoundedEntityId,
    },
    DismissIntent {
        intent_id: BoundedEntityId,
        planning_epoch: u64,
        reason: DismissalReason,
    },
    AppointOfficer {
        role: OfficerRole,
        cat_id: BoundedEntityId,
    },
    UnappointOfficer {
        role: OfficerRole,
    },
    OfficerAuthorityOverride {
        role: OfficerRole,
        domain: BoundedEntityId,
        request_id: Option<BoundedEntityId>,
        mode: OfficerAuthorityMode,
    },
    RequestTreatment {
        cat_id: BoundedEntityId,
        injury_id: BoundedEntityId,
        treatment_kind: BoundedEntityId,
    },
    FitProsthetic {
        cat_id: BoundedEntityId,
        prosthetic_id: BoundedEntityId,
        body_part_id: BoundedEntityId,
        fitting_site: SiteRefActionTarget,
        fitter_cat_id: Option<BoundedEntityId>,
    },
    RepairProsthetic {
        prosthetic_id: BoundedEntityId,
        workshop_id: BoundedEntityId,
        input_reservation_id: BoundedEntityId,
    },
    PurchaseResearchWithFavor {
        study_id: BoundedEntityId,
        use_preparation: bool,
        displayed_price_micro_favor: Option<BoundedFavorAmount>,
    },
    PrepareScholarStudy {
        study_id: BoundedEntityId,
        scholar_cat_id: BoundedEntityId,
    },
    ActivateDivineBoost {
        boost_kind: BoundedEntityId,
        duration_hours: u16,
        displayed_price_micro_favor: Option<BoundedFavorAmount>,
    },
    ChangeDiplomacy {
        target_colony_id: BoundedColonyId,
        relationship: DiplomacyRelationshipTarget,
    },
    ApproveAlliance {
        target_colony_id: BoundedColonyId,
        proposal_id: BoundedEntityId,
    },
    BlockColony {
        target_colony_id: BoundedColonyId,
        public_reason: Option<ReportSafeString>,
    },
    AcceptTradeContract {
        contract_id: BoundedEntityId,
    },
    RejectTradeContract {
        contract_id: BoundedEntityId,
        reason: TradeRejectionReason,
    },
    PhysicalPlacement {
        placement: PhysicalPlacementActionPayload,
    },
}

impl LeaderAiActionPayload {
    fn tag_is_supported(tag: &str) -> bool {
        matches!(
            tag,
            "select_colony"
                | "found_village"
                | "nudge_plan"
                | "create_standing_order"
                | "update_standing_order"
                | "delete_standing_order"
                | "dismiss_intent"
                | "appoint_officer"
                | "unappoint_officer"
                | "officer_authority_override"
                | "request_treatment"
                | "fit_prosthetic"
                | "repair_prosthetic"
                | "purchase_research_with_favor"
                | "prepare_scholar_study"
                | "activate_divine_boost"
                | "change_diplomacy"
                | "approve_alliance"
                | "block_colony"
                | "accept_trade_contract"
                | "reject_trade_contract"
                | "physical_placement"
        )
    }

    #[must_use]
    pub const fn authority_class(&self) -> ActionAuthorityClass {
        match self {
            Self::ActivateDivineBoost { .. } => {
                ActionAuthorityClass::PlayerOnly(PlayerOnlyAction::ActivateDivineBoost)
            }
            _ => ActionAuthorityClass::AuthenticatedPlayer,
        }
    }
}

pub fn reject_unknown_action_variant(tag: &str) -> Result<(), ActionDecodeError> {
    if LeaderAiActionPayload::tag_is_supported(tag) {
        Ok(())
    } else {
        Err(ActionDecodeError::UnknownActionVariant)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct StandingOrderPatch {
    pub instruction: Option<BoundedStandingOrderText>,
    pub priority_basis_points: Option<BoundedBasisPoints>,
    pub target_id: Option<BoundedEntityId>,
    pub clear_target: bool,
    pub expires_at_ms: Option<i64>,
    pub clear_expiry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DismissalReason {
    PlayerPriority,
    Superseded,
    NoLongerDesired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerAuthorityMode {
    Grant,
    Revoke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiplomacyRelationshipTarget {
    Friendly,
    Allied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeRejectionReason {
    TermsDeclined,
    RelationshipChanged,
    NoLongerNeeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "target",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SiteRefActionTarget {
    ExactTile {
        tile: SnapshotTilePoint,
    },
    AnchoredRect {
        anchor: SnapshotTilePoint,
        width: u16,
        height: u16,
    },
    OrderedPath {
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    EndpointPair {
        source: SnapshotTilePoint,
        destination: SnapshotTilePoint,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacementBounds {
    pub maximum_width: u16,
    pub maximum_height: u16,
    pub maximum_ordered_tiles: usize,
}

impl PlacementBounds {
    pub const STANDARD: Self = Self {
        maximum_width: MAX_PLACEMENT_DIMENSION,
        maximum_height: MAX_PLACEMENT_DIMENSION,
        maximum_ordered_tiles: MAX_PLACEMENT_TILES,
    };

    pub fn validate(self, target: &SiteRefActionTarget) -> Result<(), ActionDecodeError> {
        match target {
            SiteRefActionTarget::ExactTile { .. } | SiteRefActionTarget::EndpointPair { .. } => {
                Ok(())
            }
            SiteRefActionTarget::AnchoredRect { width, height, .. } => {
                if *width == 0
                    || *height == 0
                    || *width > self.maximum_width
                    || *height > self.maximum_height
                {
                    Err(ActionDecodeError::InvalidBounds("placement_rectangle"))
                } else {
                    Ok(())
                }
            }
            SiteRefActionTarget::OrderedPath { ordered_tiles } => {
                if ordered_tiles.is_empty() || ordered_tiles.len() > self.maximum_ordered_tiles {
                    Err(ActionDecodeError::InvalidBounds("placement_path"))
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "placementAction",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PhysicalPlacementActionPayload {
    PlanBuilding {
        building_type: BuildingType,
        site: SiteRefActionTarget,
    },
    DesignateFarm {
        site: SiteRefActionTarget,
        crop: CropKind,
    },
    DesignateStockpile {
        site: SiteRefActionTarget,
        accepts: Vec<ResourceKind>,
    },
    DesignateGatherSpot {
        site: SiteRefActionTarget,
        resource: ResourceKind,
    },
    DesignateFishingSpot {
        site: SiteRefActionTarget,
    },
    BuildRoad {
        route: SiteRefActionTarget,
    },
    BuildBridge {
        site: SiteRefActionTarget,
    },
    DesignateRail {
        route: SiteRefActionTarget,
        worker_cat_id: BoundedEntityId,
    },
    BuildDock {
        endpoints: SiteRefActionTarget,
        worker_cat_id: BoundedEntityId,
    },
    BuildTransportVehicle {
        mode: TransportMode,
        home: SiteRefActionTarget,
        worker_cat_id: BoundedEntityId,
    },
    CreateTransportRoute {
        mode: TransportMode,
        source_stockpile_id: BoundedEntityId,
        destination_stockpile_id: BoundedEntityId,
        resource: ResourceKind,
        amount: BoundedTradeAmount,
        route: SiteRefActionTarget,
        worker_cat_id: BoundedEntityId,
        repeat: bool,
    },
}

impl PhysicalPlacementActionPayload {
    fn target(&self) -> &SiteRefActionTarget {
        match self {
            Self::PlanBuilding { site, .. }
            | Self::DesignateFarm { site, .. }
            | Self::DesignateStockpile { site, .. }
            | Self::DesignateGatherSpot { site, .. }
            | Self::DesignateFishingSpot { site }
            | Self::BuildBridge { site } => site,
            Self::BuildRoad { route }
            | Self::DesignateRail { route, .. }
            | Self::CreateTransportRoute { route, .. } => route,
            Self::BuildDock { endpoints, .. } => endpoints,
            Self::BuildTransportVehicle { home, .. } => home,
        }
    }
}

pub fn validate_lai25_action_bounds(
    envelope: &LeaderAiActionEnvelope,
) -> Result<(), ActionDecodeError> {
    reject_unknown_action_version(envelope.protocol_version.get())?;
    match &envelope.payload {
        LeaderAiActionPayload::ActivateDivineBoost { duration_hours, .. }
            if *duration_hours == 0 || *duration_hours > 24 * 365 =>
        {
            Err(ActionDecodeError::InvalidBounds("boost_duration_hours"))
        }
        LeaderAiActionPayload::UpdateStandingOrder { patch, .. }
            if patch.instruction.is_none()
                && patch.priority_basis_points.is_none()
                && patch.target_id.is_none()
                && !patch.clear_target
                && patch.expires_at_ms.is_none()
                && !patch.clear_expiry =>
        {
            Err(ActionDecodeError::InvalidBounds("standing_order_patch"))
        }
        LeaderAiActionPayload::CreateStandingOrder { .. }
        | LeaderAiActionPayload::UpdateStandingOrder { .. }
        | LeaderAiActionPayload::DeleteStandingOrder { .. }
            if envelope
                .expected_versions
                .expected_standing_order_version
                .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "standing_order_expected_version",
            ))
        }
        LeaderAiActionPayload::AppointOfficer { .. }
        | LeaderAiActionPayload::UnappointOfficer { .. }
        | LeaderAiActionPayload::OfficerAuthorityOverride { .. }
            if envelope
                .expected_versions
                .expected_officer_version
                .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds("officer_expected_version"))
        }
        LeaderAiActionPayload::RequestTreatment { .. }
            if envelope.expected_versions.expected_care_version.is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds("care_expected_version"))
        }
        LeaderAiActionPayload::PhysicalPlacement { placement } => {
            if envelope
                .expected_versions
                .expected_spatial_version
                .is_none()
                || envelope
                    .expected_versions
                    .expected_reservation_version
                    .is_none()
            {
                return Err(ActionDecodeError::InvalidBounds(
                    "placement_expected_versions",
                ));
            }
            PlacementBounds::STANDARD.validate(placement.target())
        }
        LeaderAiActionPayload::PurchaseResearchWithFavor { .. }
            if envelope
                .expected_versions
                .expected_research_version
                .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "research_expected_version",
            ))
        }
        LeaderAiActionPayload::PrepareScholarStudy { .. }
            if envelope
                .expected_versions
                .expected_scholar_version
                .is_none()
                || envelope
                    .expected_versions
                    .expected_research_version
                    .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "scholar_expected_versions",
            ))
        }
        LeaderAiActionPayload::ActivateDivineBoost { .. }
            if envelope.expected_versions.expected_boost_version.is_none()
                || envelope
                    .expected_versions
                    .expected_research_version
                    .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds("boost_expected_versions"))
        }
        LeaderAiActionPayload::ChangeDiplomacy { .. }
        | LeaderAiActionPayload::ApproveAlliance { .. }
        | LeaderAiActionPayload::BlockColony { .. }
            if envelope
                .expected_versions
                .expected_diplomacy_version
                .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "diplomacy_expected_version",
            ))
        }
        LeaderAiActionPayload::AcceptTradeContract { .. }
            if envelope.expected_versions.expected_trade_version.is_none()
                || envelope
                    .expected_versions
                    .expected_diplomacy_version
                    .is_none()
                || envelope
                    .expected_versions
                    .expected_reservation_version
                    .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "trade_accept_expected_versions",
            ))
        }
        LeaderAiActionPayload::RejectTradeContract { .. }
            if envelope.expected_versions.expected_trade_version.is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds("trade_expected_version"))
        }
        LeaderAiActionPayload::FitProsthetic { .. }
            if envelope
                .expected_versions
                .expected_prosthetic_version
                .is_none()
                || envelope
                    .expected_versions
                    .expected_spatial_version
                    .is_none()
                || envelope
                    .expected_versions
                    .expected_reservation_version
                    .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "prosthetic_fit_expected_versions",
            ))
        }
        LeaderAiActionPayload::RepairProsthetic { .. }
            if envelope
                .expected_versions
                .expected_prosthetic_version
                .is_none()
                || envelope
                    .expected_versions
                    .expected_spatial_version
                    .is_none()
                || envelope
                    .expected_versions
                    .expected_reservation_version
                    .is_none() =>
        {
            Err(ActionDecodeError::InvalidBounds(
                "prosthetic_repair_expected_versions",
            ))
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerOnlyAction {
    ActivateDivineBoost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderSimulationAuthority {
    Planning,
    Succession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerSimulationAuthority {
    OwnedDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerAppointmentAuthority {
    AuthenticatedColonyPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentAuthority {
    AuthenticatedColonyPlayer,
    MedicalDomainOfficer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiplomacyConsentAuthority {
    AuthenticatedColonyPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionAuthorityClass {
    AuthenticatedPlayer,
    PlayerOnly(PlayerOnlyAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionValidationStep {
    ProtocolCompatibility,
    Authentication,
    ColonyOwnership,
    ActionAuthority,
    ExpectedVersions,
    DuplicateReplay,
    CurrentPreconditions,
    FavorOrReservationCommit,
}

pub struct ActionValidationPipeline;

impl ActionValidationPipeline {
    #[must_use]
    pub const fn ordered_steps() -> [ActionValidationStep; 8] {
        [
            check_protocol_compatibility(),
            check_authentication(),
            check_colony_ownership(),
            check_action_authority(),
            check_expected_versions(),
            check_duplicate_replay(),
            check_current_preconditions(),
            commit_favor_or_reservation(),
        ]
    }
}

const fn check_protocol_compatibility() -> ActionValidationStep {
    ActionValidationStep::ProtocolCompatibility
}

const fn check_authentication() -> ActionValidationStep {
    ActionValidationStep::Authentication
}

const fn check_colony_ownership() -> ActionValidationStep {
    ActionValidationStep::ColonyOwnership
}

const fn check_action_authority() -> ActionValidationStep {
    ActionValidationStep::ActionAuthority
}

const fn check_expected_versions() -> ActionValidationStep {
    ActionValidationStep::ExpectedVersions
}

const fn check_duplicate_replay() -> ActionValidationStep {
    ActionValidationStep::DuplicateReplay
}

const fn check_current_preconditions() -> ActionValidationStep {
    ActionValidationStep::CurrentPreconditions
}

const fn commit_favor_or_reservation() -> ActionValidationStep {
    ActionValidationStep::FavorOrReservationCommit
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LeaderAiActionResponse {
    pub protocol_version: ActionProtocolVersion,
    pub idempotency_id: ActionIdempotencyId,
    pub colony_id: SelectedColonyId,
    pub result: LeaderAiActionResult,
    pub refresh: Option<StaleClientRefresh>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "outcome",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LeaderAiActionResult {
    Accepted { accepted: ActionAcceptedResult },
    Rejected { conflict: ActionConflict },
    DuplicateReplay { replay: ActionReplayResult },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ActionAcceptedResult {
    pub result_code: ReportSafeString,
    pub changed_ids: Vec<BoundedEntityId>,
    pub committed_versions: CurrentVersionHint,
    pub current_state_hint: Option<CurrentStateHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ActionReplayResult {
    pub original_accepted: bool,
    pub result_code: ReportSafeString,
    pub committed_versions: Option<CurrentVersionHint>,
    pub current_state_hint: Option<CurrentStateHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct StaleClientRefresh {
    pub current_versions: CurrentVersionHint,
    pub current_state_hint: CurrentStateHint,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CurrentVersionHint {
    pub planner_version: Option<u64>,
    pub domain_version: Option<u64>,
    pub resource_version: Option<u64>,
    pub spatial_version: Option<u64>,
    pub reservation_version: Option<u64>,
    pub research_version: Option<u64>,
    pub scholar_version: Option<u64>,
    pub boost_version: Option<u64>,
    pub diplomacy_version: Option<u64>,
    pub trade_version: Option<u64>,
    pub prosthetic_version: Option<u64>,
    pub care_version: Option<u64>,
    pub officer_version: Option<u64>,
    pub standing_order_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CurrentStateHint {
    pub state_code: ReportSafeString,
    pub visible_entity_id: Option<BoundedEntityId>,
    pub visible_stage: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "conflict",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ActionConflict {
    UpdateRequired {
        code: UpdateRequiredCode,
        minimum_supported_version: u32,
        current_protocol_version: u32,
    },
    Unauthorized,
    OwnershipDenied,
    AuthorityDenied {
        reason_class: AuthorityDenialReason,
    },
    VersionMismatch {
        current_version_hint: CurrentVersionHint,
        current_state_hint: CurrentStateHint,
    },
    DuplicateReplay {
        replay: ActionReplayResult,
    },
    PreconditionFailed {
        reason: ReportSafeString,
    },
    InsufficientFavor {
        current_state_hint: CurrentStateHint,
    },
    ReservationConflict {
        current_state_hint: CurrentStateHint,
    },
    MalformedActionId,
    UnknownActionVariant,
    MalformedPayload,
    RateLimited {
        retry_after_ms: u64,
    },
    LeaderCannotActivateBoost,
    OfficerCannotActivateBoost,
}

impl ActionConflict {
    #[must_use]
    pub const fn update_required() -> Self {
        Self::UpdateRequired {
            code: UpdateRequiredCode::UpdateRequired,
            minimum_supported_version: PROTOCOL_VERSION,
            current_protocol_version: PROTOCOL_VERSION,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateRequiredCode {
    #[serde(rename = "UPDATE_REQUIRED")]
    UpdateRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityDenialReason {
    PlayerOnly,
    OutsideOfficerDomain,
    SelectedColonyOnly,
    ConsentRequired,
}
