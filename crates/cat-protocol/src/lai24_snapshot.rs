//! Strict, report-safe snapshot DTOs for the leader-AI cutover.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::{CurrentVersionHint, PROTOCOL_VERSION, TilePoint, VillageCapabilities};

pub const LAI24_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MANIFEST_STUDY_COUNT: usize = 531;

const MAX_STABLE_ID_BYTES: usize = 128;
const MAX_REPORT_STRING_BYTES: usize = 512;
const MAX_VISIBLE_COLONIES: usize = 256;
const MAX_REPORTS: usize = 128;
const MAX_PLANS: usize = 8;
const MAX_REQUESTS: usize = 128;
const MAX_TASKS: usize = 256;
const MAX_CATS: usize = 1_024;
const MAX_EVENTS: usize = 256;
const MAX_SITES_PER_TASK: usize = 256;
const MAX_ROUTE_TILES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotDecodeError {
    UnsupportedProtocolVersion(u32),
    UnsupportedSchemaVersion(u32),
    UnknownSnapshotVariant,
    InvalidBounds(&'static str),
    PrivateColonyState,
}

impl fmt::Display for SnapshotDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported protocol version {version}")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported snapshot schema version {version}")
            }
            Self::UnknownSnapshotVariant => formatter.write_str("unknown snapshot variant"),
            Self::InvalidBounds(field) => write!(formatter, "invalid snapshot bounds: {field}"),
            Self::PrivateColonyState => formatter.write_str("private colony state is forbidden"),
        }
    }
}

impl std::error::Error for SnapshotDecodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotProtocolVersion(u32);

impl SnapshotProtocolVersion {
    #[must_use]
    pub const fn current() -> Self {
        Self(PROTOCOL_VERSION)
    }

    pub fn new(version: u32) -> Result<Self, SnapshotDecodeError> {
        if version == PROTOCOL_VERSION {
            Ok(Self(version))
        } else {
            Err(SnapshotDecodeError::UnsupportedProtocolVersion(version))
        }
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Serialize for SnapshotProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for SnapshotProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = u32::deserialize(deserializer)?;
        Self::new(version).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct BoundedBasisPoints(u16);

impl BoundedBasisPoints {
    pub fn new(value: u16) -> Result<Self, SnapshotDecodeError> {
        if value <= 10_000 {
            Ok(Self(value))
        } else {
            Err(SnapshotDecodeError::InvalidBounds("basis_points"))
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for BoundedBasisPoints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BoundedAgeMs(u64);

impl BoundedAgeMs {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

macro_rules! bounded_string {
    ($name:ident, $maximum:expr, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SnapshotDecodeError> {
                let value = value.into();
                if value.trim().is_empty() || value.len() > $maximum {
                    return Err(SnapshotDecodeError::InvalidBounds($label));
                }
                Ok(Self(value))
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

bounded_string!(NonEmptyStableId, MAX_STABLE_ID_BYTES, "stable_id");
bounded_string!(
    ReportSafeString,
    MAX_REPORT_STRING_BYTES,
    "report_safe_string"
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LeaderAiSnapshotEnvelope {
    pub protocol_version: SnapshotProtocolVersion,
    pub schema_version: u32,
    pub now_ms: i64,
    pub world_seed: i64,
    pub selected_colony_id: NonEmptyStableId,
    pub public_villages: Vec<PublicVillageSnapshot>,
    pub colonies: Vec<ColonyAiSnapshot>,
}

impl LeaderAiSnapshotEnvelope {
    /// Decode the version header before any nested LAI.24 payload.
    ///
    /// Server routing must use this entry point rather than directly invoking the
    /// derived deserializer so incompatible clients cannot make nested decoding
    /// observable before the update-required decision.
    pub fn decode_json(encoded: &str) -> Result<Self, SnapshotDecodeError> {
        let value: serde_json::Value = serde_json::from_str(encoded)
            .map_err(|_| SnapshotDecodeError::InvalidBounds("snapshot_json"))?;
        let object = value
            .as_object()
            .ok_or(SnapshotDecodeError::InvalidBounds("snapshot_envelope"))?;
        let protocol_version = object
            .get("protocolVersion")
            .and_then(serde_json::Value::as_u64)
            .and_then(|version| u32::try_from(version).ok())
            .ok_or(SnapshotDecodeError::InvalidBounds("protocol_version"))?;
        SnapshotProtocolVersion::new(protocol_version)?;
        let schema_version = object
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .and_then(|version| u32::try_from(version).ok())
            .ok_or(SnapshotDecodeError::InvalidBounds("schema_version"))?;
        if schema_version != LAI24_SNAPSHOT_SCHEMA_VERSION {
            return Err(SnapshotDecodeError::UnsupportedSchemaVersion(
                schema_version,
            ));
        }
        let envelope = serde_json::from_value(value)
            .map_err(|_| SnapshotDecodeError::UnknownSnapshotVariant)?;
        validate_lai24_snapshot_bounds(&envelope)?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), SnapshotDecodeError> {
        validate_lai24_snapshot_bounds(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PublicVillageSnapshot {
    pub colony_id: NonEmptyStableId,
    pub display_name: ReportSafeString,
    pub capabilities: SnapshotVillageCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotVillageCapabilities {
    pub can_view: bool,
    pub can_control: bool,
    pub is_owner: bool,
}

impl From<VillageCapabilities> for SnapshotVillageCapabilities {
    fn from(value: VillageCapabilities) -> Self {
        Self {
            can_view: value.can_view,
            can_control: value.can_control,
            is_owner: value.is_owner,
        }
    }
}

impl From<SnapshotVillageCapabilities> for VillageCapabilities {
    fn from(value: SnapshotVillageCapabilities) -> Self {
        Self {
            can_view: value.can_view,
            can_control: value.can_control,
            is_owner: value.is_owner,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTilePoint {
    pub x: i32,
    pub y: i32,
}

impl From<TilePoint> for SnapshotTilePoint {
    fn from(value: TilePoint) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<SnapshotTilePoint> for TilePoint {
    fn from(value: SnapshotTilePoint) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ColonyAiSnapshot {
    pub colony_id: NonEmptyStableId,
    pub state_version: u64,
    /// Exact report-safe optimistic-concurrency values required to construct a
    /// LAI.25 action from this snapshot. `state_version` remains the aggregate
    /// refresh identity and must not be guessed into individual domains.
    #[serde(default)]
    pub action_versions: CurrentVersionHint,
    pub capabilities: SnapshotVillageCapabilities,
    pub reports: Vec<BeliefReportSnapshot>,
    pub plans: PlanQueueSnapshot,
    pub officer_requests: Vec<OfficerRequestSnapshot>,
    #[serde(default)]
    pub officer_institution: Option<OfficerInstitutionSnapshot>,
    #[serde(default)]
    pub standing_orders: Vec<StandingOrderSnapshot>,
    #[serde(default)]
    pub refresh_hints: Vec<RefreshHintSnapshot>,
    pub visible_tasks: Vec<VisibleTaskSnapshot>,
    pub cats: Vec<CatCareSnapshot>,
    pub shrine: ShrineSnapshot,
    pub favor: FavorLedgerSnapshot,
    pub research: ResearchFrontierSnapshot,
    pub boosts: Vec<DivineBoostSnapshot>,
    pub diplomacy: DiplomacySnapshot,
    pub trade: Vec<TradeContractSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfficerInstitutionSnapshot {
    pub institution_version: u64,
    pub leader_cat_id: Option<NonEmptyStableId>,
    pub offices: Vec<OfficeSnapshot>,
    pub vacancies: Vec<VacancySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfficeSnapshot {
    pub office: ReportSafeString,
    pub appointment_id: Option<NonEmptyStableId>,
    pub cat_id: Option<NonEmptyStableId>,
    pub appointed_at_tick: Option<u64>,
    pub expertise_level: u8,
    pub personal_expertise_level: u8,
    pub report_cadence_ticks: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct VacancySnapshot {
    pub vacancy_id: NonEmptyStableId,
    pub office: ReportSafeString,
    pub occurrence: u64,
    pub opened_at_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct StandingOrderSnapshot {
    pub order_id: NonEmptyStableId,
    pub order_kind: ReportSafeString,
    pub domain: ReportSafeString,
    pub target_id: Option<NonEmptyStableId>,
    pub instruction: ReportSafeString,
    pub priority_basis_points: BoundedBasisPoints,
    pub created_at_tick: u64,
    pub expires_at_tick: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RefreshHintSnapshot {
    pub state_version: u64,
    pub refresh_after_ms: Option<i64>,
    pub reason: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ReportEstimateSnapshot {
    pub minimum: i64,
    pub maximum: i64,
    pub unit: ReportSafeString,
}

impl ReportEstimateSnapshot {
    fn validate(&self) -> Result<(), SnapshotDecodeError> {
        if self.minimum <= self.maximum {
            Ok(())
        } else {
            Err(SnapshotDecodeError::InvalidBounds("report_estimate"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ReportProvenanceSnapshot {
    pub source_report_ids: Vec<NonEmptyStableId>,
    pub observer_id: Option<NonEmptyStableId>,
    pub method: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "availability",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RegenerationReportSnapshot {
    UnavailableBelowLevel4,
    Estimated {
        level_4_or_higher: bool,
        estimate: ReportEstimateSnapshot,
        provenance: ReportProvenanceSnapshot,
    },
}

impl RegenerationReportSnapshot {
    fn validate(&self) -> Result<(), SnapshotDecodeError> {
        match self {
            Self::UnavailableBelowLevel4 => Ok(()),
            Self::Estimated {
                level_4_or_higher,
                estimate,
                ..
            } if *level_4_or_higher => estimate.validate(),
            Self::Estimated { .. } => Err(SnapshotDecodeError::InvalidBounds(
                "regeneration_report_level",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct BeliefReportSnapshot {
    pub report_id: NonEmptyStableId,
    pub report_version: u64,
    pub subject_id: NonEmptyStableId,
    pub domain: ReportSafeString,
    pub estimate: ReportEstimateSnapshot,
    pub confidence_basis_points: BoundedBasisPoints,
    pub age_ms: BoundedAgeMs,
    pub observed_at_ms: i64,
    pub expires_at_ms: i64,
    pub report_level: u8,
    pub provenance: ReportProvenanceSnapshot,
    pub contradicts_report_ids: Vec<NonEmptyStableId>,
    pub replaces_report_id: Option<NonEmptyStableId>,
    pub unavailable_reason: Option<ReportSafeString>,
    pub regeneration: RegenerationReportSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PlanQueueSnapshot {
    pub planner_version: u64,
    /// The deterministic planning epoch required by dismiss-intent actions.
    /// This is distinct from the queue/version fingerprint.
    #[serde(default)]
    pub planning_epoch: u64,
    pub plans: Vec<PlanSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PlanSnapshot {
    pub plan_id: NonEmptyStableId,
    pub intent_id: NonEmptyStableId,
    pub lifecycle_state: ReportSafeString,
    pub responsible_actor_id: NonEmptyStableId,
    pub responsible_office: Option<ReportSafeString>,
    pub dependency_intent_ids: Vec<NonEmptyStableId>,
    pub score_bucket: i16,
    pub rationale: ReportSafeString,
    pub expected_cost: ReportEstimateSnapshot,
    pub expected_benefit: ReportEstimateSnapshot,
    pub reasons: Vec<PlanReasonSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PlanReasonSnapshot {
    pub code: NonEmptyStableId,
    pub summary: ReportSafeString,
    pub confidence_basis_points: BoundedBasisPoints,
    pub source_report_ids: Vec<NonEmptyStableId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfficerRequestSnapshot {
    pub request_id: NonEmptyStableId,
    pub request_version: u64,
    pub office: ReportSafeString,
    pub domain: ReportSafeString,
    pub requested_action: ReportSafeString,
    pub budget: ReportEstimateSnapshot,
    pub priority_basis_points: BoundedBasisPoints,
    pub source_report_ids: Vec<NonEmptyStableId>,
    pub expires_at_ms: i64,
    pub merged_into_request_id: Option<NonEmptyStableId>,
    pub supersedes_request_ids: Vec<NonEmptyStableId>,
    pub blocked_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WorkSlotSnapshot {
    pub slot_id: NonEmptyStableId,
    pub tile: SnapshotTilePoint,
    pub state: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ReservationSummarySnapshot {
    pub reservation_ids: Vec<NonEmptyStableId>,
    pub reservation_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TaskCargoSnapshot {
    pub cargo_ids: Vec<NonEmptyStableId>,
    pub summary: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct VisibleTaskSnapshot {
    pub task_id: NonEmptyStableId,
    pub intent_id: NonEmptyStableId,
    pub category: ReportSafeString,
    pub stage: ReportSafeString,
    pub assigned_cat_ids: Vec<NonEmptyStableId>,
    pub objective: SiteRefSnapshot,
    pub work_slots: Vec<WorkSlotSnapshot>,
    pub endpoint: Option<SiteRefSnapshot>,
    pub footprint: Vec<SnapshotTilePoint>,
    pub progress_basis_points: BoundedBasisPoints,
    pub reservations: ReservationSummarySnapshot,
    pub blocked_reason: Option<ReportSafeString>,
    pub cargo: TaskCargoSnapshot,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteVisibilitySnapshot {
    Visible,
    Reported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteLifecycleStageSnapshot {
    Proposed,
    Reserved,
    Active,
    Complete,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SiteSnapshot {
    pub site_id: NonEmptyStableId,
    pub visibility: SiteVisibilitySnapshot,
    pub lifecycle_stage: SiteLifecycleStageSnapshot,
    pub blocked_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SiteRefSnapshot {
    Tile {
        site: SiteSnapshot,
        tile: SnapshotTilePoint,
    },
    AnchoredRect {
        site: SiteSnapshot,
        anchor: SnapshotTilePoint,
        width: u16,
        height: u16,
    },
    OrderedTileSet {
        site: SiteSnapshot,
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    BuildingFootprint {
        site: SiteSnapshot,
        building_id: NonEmptyStableId,
        building_kind: ReportSafeString,
        anchor: SnapshotTilePoint,
        width: u16,
        height: u16,
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    StockpileFootprint {
        site: SiteSnapshot,
        stockpile_id: NonEmptyStableId,
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    ResourceSource {
        site: SiteSnapshot,
        source_id: NonEmptyStableId,
        resource_kind: ReportSafeString,
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    HuntSource {
        site: SiteSnapshot,
        cave_id: NonEmptyStableId,
        source_tile: SnapshotTilePoint,
    },
    WaterSourceAndBank {
        site: SiteSnapshot,
        source_tile: SnapshotTilePoint,
        bank_tile: SnapshotTilePoint,
    },
    OrderedRoute {
        site: SiteSnapshot,
        route_id: NonEmptyStableId,
        ordered_tiles: Vec<SnapshotTilePoint>,
    },
    Shrine {
        site: SiteSnapshot,
        shrine_id: NonEmptyStableId,
        endpoint: SnapshotTilePoint,
    },
    VillageEndpoint {
        site: SiteSnapshot,
        colony_id: NonEmptyStableId,
        endpoint: SnapshotTilePoint,
    },
    TradeEndpoint {
        site: SiteSnapshot,
        contract_id: NonEmptyStableId,
        colony_id: NonEmptyStableId,
        endpoint: SnapshotTilePoint,
    },
}

impl SiteRefSnapshot {
    fn validate(&self) -> Result<(), SnapshotDecodeError> {
        let ordered_tiles = match self {
            Self::OrderedTileSet { ordered_tiles, .. }
            | Self::StockpileFootprint { ordered_tiles, .. }
            | Self::ResourceSource { ordered_tiles, .. }
            | Self::OrderedRoute { ordered_tiles, .. } => Some(ordered_tiles),
            Self::BuildingFootprint {
                building_kind,
                anchor,
                width,
                height,
                ordered_tiles,
                ..
            } => {
                validate_rect_tiles(*anchor, *width, *height, ordered_tiles)?;
                if building_kind.as_str() == "workshop" {
                    WorkshopFootprintSnapshot {
                        anchor: *anchor,
                        width: *width,
                        height: *height,
                        ordered_tiles: ordered_tiles.clone(),
                    }
                    .validate_workshop_three_by_three()?;
                }
                Some(ordered_tiles)
            }
            Self::AnchoredRect { width, height, .. } if *width == 0 || *height == 0 => {
                return Err(SnapshotDecodeError::InvalidBounds("anchored_rect"));
            }
            _ => None,
        };
        if ordered_tiles.is_some_and(|tiles| tiles.is_empty() || tiles.len() > MAX_ROUTE_TILES) {
            return Err(SnapshotDecodeError::InvalidBounds("site_ordered_tiles"));
        }
        Ok(())
    }
}

fn validate_rect_tiles(
    anchor: SnapshotTilePoint,
    width: u16,
    height: u16,
    ordered_tiles: &[SnapshotTilePoint],
) -> Result<(), SnapshotDecodeError> {
    let tile_count = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or(SnapshotDecodeError::InvalidBounds("site_dimensions"))?;
    if width == 0
        || height == 0
        || tile_count > MAX_ROUTE_TILES
        || ordered_tiles.len() != tile_count
    {
        return Err(SnapshotDecodeError::InvalidBounds("site_dimensions"));
    }
    for (index, tile) in ordered_tiles.iter().enumerate() {
        let dx = i32::try_from(index % usize::from(width))
            .map_err(|_| SnapshotDecodeError::InvalidBounds("site_dimensions"))?;
        let dy = i32::try_from(index / usize::from(width))
            .map_err(|_| SnapshotDecodeError::InvalidBounds("site_dimensions"))?;
        if tile.x != anchor.x + dx || tile.y != anchor.y + dy {
            return Err(SnapshotDecodeError::InvalidBounds("site_ordered_tiles"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopFootprintSnapshot {
    pub anchor: SnapshotTilePoint,
    pub width: u16,
    pub height: u16,
    pub ordered_tiles: Vec<SnapshotTilePoint>,
}

impl WorkshopFootprintSnapshot {
    pub fn validate_workshop_three_by_three(&self) -> Result<(), SnapshotDecodeError> {
        if self.width != 3 || self.height != 3 {
            return Err(SnapshotDecodeError::InvalidBounds("workshop_dimensions"));
        }
        self.validate_nine_row_major_tiles()
    }

    pub fn validate_nine_row_major_tiles(&self) -> Result<(), SnapshotDecodeError> {
        let expected = (0..3)
            .flat_map(|dy| {
                (0..3).map(move |dx| SnapshotTilePoint {
                    x: self.anchor.x + dx,
                    y: self.anchor.y + dy,
                })
            })
            .collect::<Vec<_>>();
        if self.ordered_tiles == expected {
            Ok(())
        } else {
            Err(SnapshotDecodeError::InvalidBounds("workshop_ordered_tiles"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CatTraitsSnapshot {
    pub innate_attributes: Vec<NamedBasisPointSnapshot>,
    pub learned_skills: Vec<NamedBasisPointSnapshot>,
    pub office_experience: Vec<NamedBasisPointSnapshot>,
    pub acquired_traits: Vec<NonEmptyStableId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct NamedBasisPointSnapshot {
    pub name: NonEmptyStableId,
    pub value_basis_points: BoundedBasisPoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CatPersonalitySnapshot {
    pub sociability: BoundedBasisPoints,
    pub diligence: BoundedBasisPoints,
    pub courage: BoundedBasisPoints,
    pub empathy: BoundedBasisPoints,
    pub curiosity: BoundedBasisPoints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct StressSnapshot {
    pub stress_basis_points: BoundedBasisPoints,
    pub recovery_basis_points: BoundedBasisPoints,
    pub refusing: bool,
    pub refusal_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WillingnessSnapshot {
    pub total_basis_points: BoundedBasisPoints,
    pub factors: Vec<NamedBasisPointSnapshot>,
    pub eligible: bool,
    pub eligibility_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnatomySnapshot {
    pub body_parts: Vec<BodyPartSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct BodyPartSnapshot {
    pub body_part_id: NonEmptyStableId,
    pub side: Option<ReportSafeString>,
    pub functional_basis_points: BoundedBasisPoints,
    pub injury: Option<InjurySnapshot>,
    pub prosthetic_id: Option<NonEmptyStableId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct InjurySnapshot {
    pub injury_id: NonEmptyStableId,
    pub injury_kind: ReportSafeString,
    pub severity_basis_points: BoundedBasisPoints,
    pub sustained_at_ms: i64,
    pub treatment: Option<TreatmentSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TreatmentSnapshot {
    pub treatment_id: NonEmptyStableId,
    pub stage: ReportSafeString,
    pub medic_cat_id: Option<NonEmptyStableId>,
    pub care_site: Option<SiteRefSnapshot>,
    pub task_id: Option<NonEmptyStableId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ProstheticSnapshot {
    pub prosthetic_id: NonEmptyStableId,
    pub body_part_id: NonEmptyStableId,
    pub prosthetic_kind: ReportSafeString,
    pub restoration_basis_points: BoundedBasisPoints,
    pub wear: ProstheticWearSnapshot,
    pub fitting_task_id: Option<NonEmptyStableId>,
    pub repair_task_id: Option<NonEmptyStableId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ProstheticWearSnapshot {
    pub durability_basis_points: BoundedBasisPoints,
    pub wear_basis_points: BoundedBasisPoints,
    pub repair_eligible: bool,
    pub repair_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CareStatusSnapshot {
    pub care_site: Option<SiteRefSnapshot>,
    pub treatment_task_id: Option<NonEmptyStableId>,
    pub fitting_task_id: Option<NonEmptyStableId>,
    pub repair_task_id: Option<NonEmptyStableId>,
    pub status: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CatCareSnapshot {
    pub cat_id: NonEmptyStableId,
    pub display_name: ReportSafeString,
    pub active_task_id: Option<NonEmptyStableId>,
    pub traits: CatTraitsSnapshot,
    pub personality: CatPersonalitySnapshot,
    pub stress: StressSnapshot,
    pub willingness: WillingnessSnapshot,
    pub anatomy: AnatomySnapshot,
    pub prosthetics: Vec<ProstheticSnapshot>,
    pub care: CareStatusSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ShrineSnapshot {
    pub shrine_id: NonEmptyStableId,
    pub endpoint: SiteRefSnapshot,
    pub pipeline: Option<ShrineOfferingPipelineSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ShrineOfferingPipelineSnapshot {
    pub offering_id: NonEmptyStableId,
    pub package: OfferingPackageSnapshot,
    pub stage: OfferingStageSnapshot,
    pub source_report_ids: Vec<NonEmptyStableId>,
    pub shrine_endpoint: SiteRefSnapshot,
    pub cargo_disposition: ReportSafeString,
    pub rationale: ReportSafeString,
    pub blocked_reason: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "stage",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OfferingStageSnapshot {
    Proposed,
    Reserved,
    Hauling { carrier_cat_id: NonEmptyStableId },
    Ritual { ritualist_cat_id: NonEmptyStableId },
    Complete { completed_at_ms: i64 },
    Blocked { reason: ReportSafeString },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfferingPackageSnapshot {
    pub package_id: NonEmptyStableId,
    pub package_kind: ReportSafeString,
    pub cargo_ids: Vec<NonEmptyStableId>,
    pub favor_reward_micro_favor: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FavorLedgerSnapshot {
    pub ledger_version: u64,
    pub micro_favor: u64,
    pub favor_events: Vec<FavorEventSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FavorEventSnapshot {
    pub event_id: NonEmptyStableId,
    pub delta_micro_favor: i64,
    pub resulting_micro_favor: u64,
    pub occurred_at_ms: i64,
    pub reason: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ResearchFrontierSnapshot {
    pub research_version: u64,
    pub manifest_study_count: usize,
    pub owned_study_ids: Vec<NonEmptyStableId>,
    pub frontier: Vec<ResearchStudySnapshot>,
    pub automatic_quota: AutomaticResearchQuotaSnapshot,
    pub insight: InsightSnapshot,
    pub preparations: Vec<ScholarPreparationSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ResearchStudySnapshot {
    pub study_id: NonEmptyStableId,
    pub display_name: ReportSafeString,
    pub prerequisite_ids: Vec<NonEmptyStableId>,
    pub price_micro_favor: u64,
    pub prepared_price_micro_favor: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticResearchQuotaSnapshot {
    pub quota_used: u8,
    pub quota_limit: u8,
    pub quota_window_started_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct InsightSnapshot {
    pub insight_balance: u64,
    pub generated_this_week: u64,
    pub week_started_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ScholarPreparationSnapshot {
    pub preparation_id: NonEmptyStableId,
    pub study_id: NonEmptyStableId,
    pub scholar_cat_id: Option<NonEmptyStableId>,
    pub progress_basis_points: BoundedBasisPoints,
    pub committed_insight_cost: u64,
    pub player_discount_basis_points: BoundedBasisPoints,
    pub prepared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct DivineBoostSnapshot {
    pub boost_id: NonEmptyStableId,
    pub boost_kind: ReportSafeString,
    pub effect_basis_points: BoundedBasisPoints,
    pub boost_price_micro_favor: u64,
    pub duration_ms: u64,
    pub boost_started_at_ms: i64,
    pub boost_expires_at_ms: i64,
    pub effect_stage: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct DiplomacySnapshot {
    pub diplomacy_version: u64,
    pub relationships: Vec<RelationshipSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipSnapshot {
    pub relationship_id: NonEmptyStableId,
    pub other_colony_id: NonEmptyStableId,
    pub relationship_version: u64,
    pub state: ReportSafeString,
    pub consent: ConsentSnapshot,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ConsentSnapshot {
    pub local_approved: bool,
    pub remote_approved: bool,
    pub consent_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TradeContractSnapshot {
    pub contract_id: NonEmptyStableId,
    pub contract_version: u64,
    pub partner_colony_id: NonEmptyStableId,
    pub stage: TradeStageSnapshot,
    pub actor_cat_ids: Vec<NonEmptyStableId>,
    pub valuation_report_ids: Vec<NonEmptyStableId>,
    pub valuation_confidence_basis_points: BoundedBasisPoints,
    pub escrow: TradeEscrowSnapshot,
    pub route: TradeRouteSnapshot,
    pub cargo: Vec<TradeCargoSnapshot>,
    pub next_event_at_ms: Option<i64>,
    pub reservations: ReservationSummarySnapshot,
    pub bounded_failure: Option<ReportSafeString>,
    pub recovery_state: Option<ReportSafeString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TradeEscrowSnapshot {
    pub escrow_id: NonEmptyStableId,
    pub cargo_ids: Vec<NonEmptyStableId>,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TradeCargoSnapshot {
    pub cargo_id: NonEmptyStableId,
    pub cargo_kind: ReportSafeString,
    pub quantity: u64,
    pub state: ReportSafeString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TradeRouteSnapshot {
    pub route_id: NonEmptyStableId,
    pub ordered_tiles: Vec<SnapshotTilePoint>,
    pub endpoint: SiteRefSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "stage",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TradeStageSnapshot {
    Proposed,
    AwaitingConsent,
    Escrowed,
    Outbound,
    Returning,
    Complete,
    Stranded { recovery_task_id: NonEmptyStableId },
    Failed { bounded_failure: ReportSafeString },
}

/// This uninhabited type documents that private state has no wire representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateColonyStateSnapshot {}

pub fn reject_private_colony_state(
    private_state: Option<&PrivateColonyStateSnapshot>,
) -> Result<(), SnapshotDecodeError> {
    if private_state.is_some() {
        Err(SnapshotDecodeError::PrivateColonyState)
    } else {
        Ok(())
    }
}

pub fn validate_lai24_snapshot_bounds(
    envelope: &LeaderAiSnapshotEnvelope,
) -> Result<(), SnapshotDecodeError> {
    if envelope.protocol_version.get() != PROTOCOL_VERSION {
        return Err(SnapshotDecodeError::UnsupportedProtocolVersion(
            envelope.protocol_version.get(),
        ));
    }
    if envelope.schema_version != LAI24_SNAPSHOT_SCHEMA_VERSION {
        return Err(SnapshotDecodeError::UnsupportedSchemaVersion(
            envelope.schema_version,
        ));
    }
    if envelope.public_villages.len() > MAX_VISIBLE_COLONIES
        || envelope.colonies.is_empty()
        || envelope.colonies.len() > MAX_VISIBLE_COLONIES
    {
        return Err(SnapshotDecodeError::InvalidBounds("visible_colonies"));
    }
    if !envelope
        .colonies
        .iter()
        .any(|colony| colony.colony_id == envelope.selected_colony_id)
    {
        return Err(SnapshotDecodeError::InvalidBounds("selected_colony_id"));
    }

    for colony in &envelope.colonies {
        validate_colony(colony)?;
    }
    Ok(())
}

fn validate_colony(colony: &ColonyAiSnapshot) -> Result<(), SnapshotDecodeError> {
    if colony.reports.len() > MAX_REPORTS
        || colony.plans.plans.len() > MAX_PLANS
        || colony.officer_requests.len() > MAX_REQUESTS
        || colony.visible_tasks.len() > MAX_TASKS
        || colony.cats.len() > MAX_CATS
        || colony.favor.favor_events.len() > MAX_EVENTS
    {
        return Err(SnapshotDecodeError::InvalidBounds("colony_collections"));
    }
    for report in &colony.reports {
        report.estimate.validate()?;
        report.regeneration.validate()?;
        if report.report_level > 5 || report.observed_at_ms > report.expires_at_ms {
            return Err(SnapshotDecodeError::InvalidBounds("belief_report"));
        }
        if report.report_level < 4
            && !matches!(
                report.regeneration,
                RegenerationReportSnapshot::UnavailableBelowLevel4
            )
        {
            return Err(SnapshotDecodeError::InvalidBounds(
                "regeneration_report_level",
            ));
        }
    }
    for plan in &colony.plans.plans {
        plan.expected_cost.validate()?;
        plan.expected_benefit.validate()?;
    }
    for request in &colony.officer_requests {
        request.budget.validate()?;
    }
    for task in &colony.visible_tasks {
        if task.assigned_cat_ids.len() > MAX_CATS
            || task.work_slots.len() > MAX_SITES_PER_TASK
            || task.footprint.len() > MAX_SITES_PER_TASK
        {
            return Err(SnapshotDecodeError::InvalidBounds("visible_task"));
        }
        task.objective.validate()?;
        if let Some(endpoint) = &task.endpoint {
            endpoint.validate()?;
        }
    }
    validate_active_task_links(&colony.cats, &colony.visible_tasks)?;
    colony.shrine.endpoint.validate()?;
    if let Some(pipeline) = &colony.shrine.pipeline {
        pipeline.shrine_endpoint.validate()?;
    }
    if colony.research.manifest_study_count != MANIFEST_STUDY_COUNT
        || colony.research.frontier.len() > MANIFEST_STUDY_COUNT
        || colony.research.owned_study_ids.len() > MANIFEST_STUDY_COUNT
        || colony.research.automatic_quota.quota_used > colony.research.automatic_quota.quota_limit
    {
        return Err(SnapshotDecodeError::InvalidBounds("research"));
    }
    for boost in &colony.boosts {
        let duration = boost
            .boost_expires_at_ms
            .checked_sub(boost.boost_started_at_ms)
            .and_then(|value| u64::try_from(value).ok());
        if duration != Some(boost.duration_ms) {
            return Err(SnapshotDecodeError::InvalidBounds("divine_boost"));
        }
    }
    for contract in &colony.trade {
        if contract.route.ordered_tiles.is_empty()
            || contract.route.ordered_tiles.len() > MAX_ROUTE_TILES
        {
            return Err(SnapshotDecodeError::InvalidBounds("trade_route"));
        }
        contract.route.endpoint.validate()?;
    }
    Ok(())
}

fn validate_active_task_links(
    cats: &[CatCareSnapshot],
    tasks: &[VisibleTaskSnapshot],
) -> Result<(), SnapshotDecodeError> {
    let mut cat_ids = BTreeSet::new();
    for cat in cats {
        if !cat_ids.insert(&cat.cat_id) {
            return Err(SnapshotDecodeError::InvalidBounds("duplicate_cat_id"));
        }
        if let Some(active_task_id) = &cat.active_task_id {
            let task = tasks
                .iter()
                .find(|task| task.task_id == *active_task_id)
                .ok_or(SnapshotDecodeError::InvalidBounds("active_task_id"))?;
            if !task
                .assigned_cat_ids
                .iter()
                .any(|assigned_cat_id| assigned_cat_id == &cat.cat_id)
            {
                return Err(SnapshotDecodeError::InvalidBounds("active_task_id"));
            }
        }
    }
    Ok(())
}
