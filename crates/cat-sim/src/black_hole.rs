//! Pure, catalog-bound authority for The Hole.
//!
//! This leaf owns no world tick, report projection, transport executor, wire
//! protocol, persistence adapter, or renderer. Those integrations deliberately
//! remain for LAI.46–LAI.52. It accepts already-resolved catalog policy and
//! validates it against exact physical identities before producing micro-Void.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{
        CapabilityId, CapabilityRequirement, ConstructionMiracleInputClass, ContentId,
        ContentManifest, ManifestHoleValueStage, MaterialInstanceId, PhysicalLotId,
    },
    quality_lots::{LotLocation, QualityBand},
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
};

pub const BLACK_HOLE_SCHEMA_VERSION: u32 = 2;
pub const AXIS_MIN: u8 = 0;
pub const AXIS_MAX: u8 = 10;
pub const OPENING_GAME_MINUTES: u64 = 40;
pub const MAX_COMMAND_RECEIPTS: usize = 256;
pub const MAX_RECOVERY_RECEIPTS: usize = 256;
pub const MAX_OUTPUT_HISTORY: usize = 256;
pub const MAX_PENDING_UPGRADE_COMPLETIONS: usize = 16;
const MAX_UPGRADE_PROVENANCE_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoleError {
    InvalidSchemaVersion(u32),
    InvalidAxis { axis: HoleAxis, value: u8 },
    InvalidIdentity(String),
    EmptyOrder,
    DuplicateEntryIdentity,
    ZeroQuantity,
    InvalidProvenance,
    QuantityOverflow,
    OrderCapacityExceeded { maximum: u32, requested: u32 },
    ActiveFeedExists,
    ActiveUpgradeExists,
    NoActiveFeed,
    NoActiveUpgrade,
    InvalidPhysicalStage,
    InvalidLocation,
    InvalidPolicy,
    MissingManifestFeedPolicy,
    IneligibleManifestFeedClass,
    ContentMismatch,
    CapabilityLocked,
    DarknessLocked,
    QualityLocked,
    OwnershipRejected,
    ReservationRejected,
    RouteRejected,
    InvalidCondition,
    ArithmeticOverflow,
    OutputBackpressure,
    CommandConflict,
    RecoveryConflict,
    UnknownEntry,
    UnknownOrder,
    UnknownUpgrade,
    UpgradeBillMismatch,
    RecoveryDispositionRejected,
    CoordinateOverflow,
    MalformedState,
}

impl fmt::Display for HoleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Hole state: {self:?}")
    }
}

impl std::error::Error for HoleError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoleAxis {
    Width,
    Depth,
    Darkness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoleAxes {
    pub width: u8,
    pub depth: u8,
    pub darkness: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HoleAxesWire {
    width: u8,
    depth: u8,
    darkness: u8,
}

impl<'de> Deserialize<'de> for HoleAxes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HoleAxesWire::deserialize(deserializer)?;
        Self::new(wire.width, wire.depth, wire.darkness).map_err(de::Error::custom)
    }
}

impl Default for HoleAxes {
    fn default() -> Self {
        Self {
            width: 0,
            depth: 0,
            darkness: 0,
        }
    }
}

impl HoleAxes {
    pub fn new(width: u8, depth: u8, darkness: u8) -> Result<Self, HoleError> {
        validate_axis(HoleAxis::Width, width)?;
        validate_axis(HoleAxis::Depth, depth)?;
        validate_axis(HoleAxis::Darkness, darkness)?;
        Ok(Self {
            width,
            depth,
            darkness,
        })
    }

    #[must_use]
    pub const fn level(self, axis: HoleAxis) -> u8 {
        match axis {
            HoleAxis::Width => self.width,
            HoleAxis::Depth => self.depth,
            HoleAxis::Darkness => self.darkness,
        }
    }

    #[must_use]
    pub const fn intake_width(self) -> u32 {
        intake_width(self.width)
    }

    #[must_use]
    pub const fn maximum_order_units(self) -> u32 {
        maximum_order_units(self.depth)
    }
}

pub fn validate_axis(axis: HoleAxis, value: u8) -> Result<(), HoleError> {
    if (AXIS_MIN..=AXIS_MAX).contains(&value) {
        Ok(())
    } else {
        Err(HoleError::InvalidAxis { axis, value })
    }
}

#[must_use]
pub const fn intake_width(width: u8) -> u32 {
    1 + width as u32
}

#[must_use]
pub const fn maximum_order_units(depth: u8) -> u32 {
    10 * (1 + depth as u32)
}

/// Fixed geometry; neither Width, Depth, nor Darkness changes this landmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoleFootprint {
    pub landmark: TaskFootprint,
    pub work: TaskFootprint,
    pub ring: Vec<TilePoint>,
    pub pinned_delivery_edge: TilePoint,
}

pub fn hole_footprint(anchor: TilePoint) -> Result<HoleFootprint, HoleError> {
    let landmark_rect = Rect::try_new(anchor, 5, 5).map_err(|_| HoleError::CoordinateOverflow)?;
    let work_anchor = TilePoint {
        x: anchor
            .x
            .checked_add(1)
            .ok_or(HoleError::CoordinateOverflow)?,
        y: anchor
            .y
            .checked_add(1)
            .ok_or(HoleError::CoordinateOverflow)?,
    };
    let work_rect = Rect::try_new(work_anchor, 3, 3).map_err(|_| HoleError::CoordinateOverflow)?;
    let work = TaskFootprint::rectangular(work_rect);
    let ring = landmark_rect
        .ordered_tiles()
        .into_vec()
        .into_iter()
        .filter(|tile| !work.tiles.as_slice().contains(tile))
        .collect::<Vec<_>>();
    debug_assert_eq!(ring.len(), 16);
    Ok(HoleFootprint {
        landmark: TaskFootprint::rectangular(landmark_rect),
        work,
        ring,
        pinned_delivery_edge: TilePoint {
            x: anchor
                .x
                .checked_add(2)
                .ok_or(HoleError::CoordinateOverflow)?,
            y: anchor.y,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedValueStage {
    Raw,
    Processed,
    Simple,
    Prepared,
    Complex,
    Feast,
}

impl FeedValueStage {
    #[must_use]
    pub const fn value_percent(self) -> u64 {
        match self {
            Self::Raw => 100,
            Self::Processed | Self::Simple => 125,
            Self::Prepared => 160,
            Self::Complex => 210,
            Self::Feast => 280,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeedIdentity {
    BulkLot { lot_id: PhysicalLotId },
    ItemInstance { item_id: MaterialInstanceId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedPhysicalStage {
    Queued,
    Reserved,
    Carried,
    Delivered,
    Credited,
    Released,
    Recovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradePhysicalStage {
    Queued,
    Reserved,
    Carried,
    Delivered,
    Consumed,
    Released,
    Recovered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogResolvedFeedPolicy {
    pub content_id: ContentId,
    pub capability_id: CapabilityId,
    pub content_is_canonical: bool,
    pub capability_is_owned: bool,
    pub ownership_is_authorized: bool,
    pub reservation_is_authorized: bool,
    pub route_is_authorized: bool,
    pub required_darkness: u8,
    pub maximum_quality: QualityBand,
    pub base_value_milli: u64,
    pub stage: FeedValueStage,
    pub installed_augmentation_value_milli: u64,
    pub current_condition: u64,
    pub maximum_condition: u64,
}

impl CatalogResolvedFeedPolicy {
    pub fn micro_void_for(&self, quality: QualityBand) -> Result<u64, HoleError> {
        if self.maximum_condition == 0 {
            return Err(HoleError::InvalidCondition);
        }
        self.base_value_milli
            .checked_add(self.installed_augmentation_value_milli)
            .and_then(|value| value.checked_mul(1_000))
            .and_then(|value| value.checked_mul(self.stage.value_percent()))
            .and_then(|value| value.checked_mul(u64::from(quality.trade_hole_value_percent())))
            .and_then(|value| value.checked_mul(self.current_condition))
            .and_then(|numerator| {
                100_u64
                    .checked_mul(100)
                    .and_then(|denominator| denominator.checked_mul(self.maximum_condition))
                    .map(|denominator| numerator / denominator)
            })
            .ok_or(HoleError::ArithmeticOverflow)
    }

    fn validate(
        &self,
        axes: HoleAxes,
        content_id: &ContentId,
        quality: QualityBand,
    ) -> Result<(), HoleError> {
        validate_axis(HoleAxis::Darkness, self.required_darkness)?;
        if !self.content_is_canonical || &self.content_id != content_id {
            return Err(HoleError::ContentMismatch);
        }
        if !self.capability_is_owned {
            return Err(HoleError::CapabilityLocked);
        }
        if !self.ownership_is_authorized {
            return Err(HoleError::OwnershipRejected);
        }
        if !self.reservation_is_authorized {
            return Err(HoleError::ReservationRejected);
        }
        if !self.route_is_authorized {
            return Err(HoleError::RouteRejected);
        }
        if axes.darkness < self.required_darkness {
            return Err(HoleError::DarknessLocked);
        }
        if quality > self.maximum_quality {
            return Err(HoleError::QualityLocked);
        }
        let _ = self.micro_void_for(quality)?;
        Ok(())
    }
}

/// Dynamic authorization and condition facts layered over the manifest's
/// canonical Hole value policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestHoleFeedFacts {
    pub capability_is_owned: bool,
    pub ownership_is_authorized: bool,
    pub reservation_is_authorized: bool,
    pub route_is_authorized: bool,
    pub maximum_quality: QualityBand,
    pub installed_augmentation_value_milli: u64,
    pub current_condition: u64,
    pub maximum_condition: u64,
}

/// Resolve the one manifest-owned policy shape used by ordinary Hole feeding
/// and by construction-miracle package valuation.
pub fn resolve_manifest_hole_feed_policy(
    manifest: &ContentManifest,
    content_id: &ContentId,
    facts: ManifestHoleFeedFacts,
) -> Result<CatalogResolvedFeedPolicy, HoleError> {
    let descriptor = manifest
        .construction_miracle_input(content_id)
        .ok_or(HoleError::MissingManifestFeedPolicy)?;
    if descriptor.physical_class == ConstructionMiracleInputClass::Ineligible {
        return Err(HoleError::IneligibleManifestFeedClass);
    }
    let authored = descriptor
        .hole_feed_policy
        .as_ref()
        .ok_or(HoleError::MissingManifestFeedPolicy)?;
    let requirement = match descriptor.physical_class {
        ConstructionMiracleInputClass::BulkLot => manifest
            .resources
            .iter()
            .find(|record| &record.content_id == content_id)
            .map(|record| &record.canonical_capability),
        ConstructionMiracleInputClass::ExactItem => manifest
            .item_definitions
            .iter()
            .find(|record| &record.content_id == content_id)
            .map(|record| &record.canonical_capability),
        ConstructionMiracleInputClass::Fixture => manifest
            .fixtures
            .iter()
            .find(|record| &record.content_id == content_id)
            .map(|record| &record.canonical_capability),
        ConstructionMiracleInputClass::Ineligible => None,
    }
    .ok_or(HoleError::ContentMismatch)?;
    let capability_id = match requirement {
        CapabilityRequirement::Required(capability_id) => capability_id.clone(),
        CapabilityRequirement::Free => {
            CapabilityId::new("free").map_err(|_| HoleError::InvalidPolicy)?
        }
    };
    let stage = match authored.stage {
        ManifestHoleValueStage::Raw => FeedValueStage::Raw,
        ManifestHoleValueStage::Processed => FeedValueStage::Processed,
        ManifestHoleValueStage::Simple => FeedValueStage::Simple,
        ManifestHoleValueStage::Prepared => FeedValueStage::Prepared,
        ManifestHoleValueStage::Complex => FeedValueStage::Complex,
        ManifestHoleValueStage::Feast => FeedValueStage::Feast,
    };
    let policy = CatalogResolvedFeedPolicy {
        content_id: content_id.clone(),
        capability_id,
        content_is_canonical: true,
        capability_is_owned: facts.capability_is_owned,
        ownership_is_authorized: facts.ownership_is_authorized,
        reservation_is_authorized: facts.reservation_is_authorized,
        route_is_authorized: facts.route_is_authorized,
        required_darkness: authored.required_darkness,
        maximum_quality: facts.maximum_quality,
        base_value_milli: authored.base_value_milli,
        stage,
        installed_augmentation_value_milli: facts.installed_augmentation_value_milli,
        current_condition: facts.current_condition,
        maximum_condition: facts.maximum_condition,
    };
    let _ = policy.micro_void_for(QualityBand::Common)?;
    Ok(policy)
}

/// Derive the Common-quality, unaugmented value used to compose a canonical
/// construction miracle. This intentionally has no trader or coin fallback.
pub fn canonical_construction_input_unit_value_micros(
    manifest: &ContentManifest,
    content_id: &ContentId,
) -> Result<u64, HoleError> {
    resolve_manifest_hole_feed_policy(
        manifest,
        content_id,
        ManifestHoleFeedFacts {
            capability_is_owned: true,
            ownership_is_authorized: true,
            reservation_is_authorized: true,
            route_is_authorized: true,
            maximum_quality: QualityBand::Masterwork,
            installed_augmentation_value_milli: 0,
            current_condition: 1,
            maximum_condition: 1,
        },
    )?
    .micro_void_for(QualityBand::Common)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedEntry {
    pub identity: FeedIdentity,
    pub content_id: ContentId,
    pub quality: QualityBand,
    pub provenance: String,
    pub units: u32,
    pub credited_units: u32,
    #[serde(with = "lot_location_serde")]
    pub origin: LotLocation,
    #[serde(with = "lot_location_serde")]
    pub location: LotLocation,
    pub reservation_id: String,
    pub route_id: String,
    pub stage: FeedPhysicalStage,
    pub policy: CatalogResolvedFeedPolicy,
}

/// Strict Hole-state wire adapter for the closed physical-location type owned
/// by `quality_lots`.  This keeps location identity in the persisted Hole
/// record without defining a second location authority.
mod lot_location_serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum Wire {
        Source { id: String },
        Stockpile { id: String },
        StationInput { id: String },
        StationOutput { id: String },
        Cargo { id: String },
        Cache { id: String },
        Hole { id: String },
    }

    pub fn serialize<S>(location: &LotLocation, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = match location {
            LotLocation::Source(id) => Wire::Source { id: id.clone() },
            LotLocation::Stockpile(id) => Wire::Stockpile { id: id.clone() },
            LotLocation::StationInput(id) => Wire::StationInput { id: id.clone() },
            LotLocation::StationOutput(id) => Wire::StationOutput { id: id.clone() },
            LotLocation::Cargo(id) => Wire::Cargo { id: id.clone() },
            LotLocation::Cache(id) => Wire::Cache { id: id.clone() },
            LotLocation::Hole(id) => Wire::Hole { id: id.clone() },
        };
        wire.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<LotLocation, D::Error>
    where
        D: Deserializer<'de>,
    {
        let location = match Wire::deserialize(deserializer)? {
            Wire::Source { id } => LotLocation::Source(id),
            Wire::Stockpile { id } => LotLocation::Stockpile(id),
            Wire::StationInput { id } => LotLocation::StationInput(id),
            Wire::StationOutput { id } => LotLocation::StationOutput(id),
            Wire::Cargo { id } => LotLocation::Cargo(id),
            Wire::Cache { id } => LotLocation::Cache(id),
            Wire::Hole { id } => LotLocation::Hole(id),
        };
        Ok(location)
    }
}

impl FeedEntry {
    fn remaining_units(&self) -> Result<u32, HoleError> {
        self.units
            .checked_sub(self.credited_units)
            .ok_or(HoleError::MalformedState)
    }

    fn validate(&self, axes: HoleAxes, hole_id: &str) -> Result<(), HoleError> {
        if self.units == 0 || self.provenance.trim().is_empty() {
            return Err(HoleError::ZeroQuantity);
        }
        validate_location(&self.origin)?;
        validate_location(&self.location)?;
        validate_stable_id(&self.route_id)?;
        if ContentManifest::embedded()
            .construction_miracle_input(&self.content_id)
            .is_some()
        {
            if ContentManifest::embedded()
                .construction_miracle_input(&self.content_id)
                .is_none_or(|descriptor| {
                    descriptor.physical_class != ConstructionMiracleInputClass::BulkLot
                })
            {
                return Err(HoleError::IneligibleManifestFeedClass);
            }
            let resolved = resolve_manifest_hole_feed_policy(
                ContentManifest::embedded(),
                &self.content_id,
                ManifestHoleFeedFacts {
                    capability_is_owned: self.policy.capability_is_owned,
                    ownership_is_authorized: self.policy.ownership_is_authorized,
                    reservation_is_authorized: self.policy.reservation_is_authorized,
                    route_is_authorized: self.policy.route_is_authorized,
                    maximum_quality: self.policy.maximum_quality,
                    installed_augmentation_value_milli: self
                        .policy
                        .installed_augmentation_value_milli,
                    current_condition: self.policy.current_condition,
                    maximum_condition: self.policy.maximum_condition,
                },
            )?;
            if self.policy != resolved {
                return Err(HoleError::InvalidPolicy);
            }
        }
        self.policy.validate(axes, &self.content_id, self.quality)?;
        if matches!(&self.identity, FeedIdentity::BulkLot { .. })
            && (self.policy.installed_augmentation_value_milli != 0
                || self.policy.current_condition != 1
                || self.policy.maximum_condition != 1)
        {
            return Err(HoleError::InvalidCondition);
        }
        let remaining = self.remaining_units()?;
        match self.stage {
            FeedPhysicalStage::Queued => {
                if self.location != self.origin
                    || self.credited_units != 0
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            FeedPhysicalStage::Reserved => {
                if self.location != self.origin
                    || self.credited_units != 0
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            FeedPhysicalStage::Carried => {
                if !matches!(&self.location, LotLocation::Cargo(_))
                    || self.credited_units != 0
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            FeedPhysicalStage::Delivered => {
                if self.location != LotLocation::Hole(hole_id.to_owned())
                    || remaining == 0
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            FeedPhysicalStage::Credited => {
                if self.credited_units != self.units
                    || remaining != 0
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            FeedPhysicalStage::Released | FeedPhysicalStage::Recovered => {
                if self.credited_units != 0 || !self.reservation_id.is_empty() {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedOrder {
    pub id: String,
    pub command_id: String,
    pub entries: Vec<FeedEntry>,
    pub created_game_minute: u64,
}

impl FeedOrder {
    pub fn accepted_units(&self) -> Result<u32, HoleError> {
        self.entries.iter().try_fold(0_u32, |total, entry| {
            total
                .checked_add(entry.remaining_units()?)
                .ok_or(HoleError::QuantityOverflow)
        })
    }

    fn validate(&self, axes: HoleAxes, hole_id: &str) -> Result<(), HoleError> {
        validate_stable_id(&self.id)?;
        validate_stable_id(&self.command_id)?;
        if self.entries.is_empty() {
            return Err(HoleError::EmptyOrder);
        }
        let mut identities = BTreeSet::new();
        for entry in &self.entries {
            if !identities.insert(entry.identity.clone()) {
                return Err(HoleError::DuplicateEntryIdentity);
            }
            entry.validate(axes, hole_id)?;
        }
        let accepted = self.accepted_units()?;
        if accepted == 0 {
            return Err(HoleError::ZeroQuantity);
        }
        if accepted > axes.maximum_order_units() {
            return Err(HoleError::OrderCapacityExceeded {
                maximum: axes.maximum_order_units(),
                requested: accepted,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AxisUpgradeProject {
    pub id: String,
    pub command_id: String,
    pub axis: HoleAxis,
    pub target_level: u8,
    pub inputs: Vec<UpgradeInput>,
    bound_bill: Option<UpgradeBill>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeInput {
    pub identity: FeedIdentity,
    pub content_id: ContentId,
    pub quality: QualityBand,
    pub quantity: u32,
    pub provenance: String,
    #[serde(with = "lot_location_serde")]
    pub origin: LotLocation,
    #[serde(with = "lot_location_serde")]
    pub location: LotLocation,
    pub reservation_id: String,
    pub route_id: String,
    pub stage: UpgradePhysicalStage,
}

impl AxisUpgradeProject {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        command_id: impl Into<String>,
        axis: HoleAxis,
        target_level: u8,
        inputs: Vec<UpgradeInput>,
    ) -> Self {
        Self {
            id: id.into(),
            command_id: command_id.into(),
            axis,
            target_level,
            inputs,
            bound_bill: None,
        }
    }

    #[must_use]
    pub fn bound_bill(&self) -> Option<&UpgradeBill> {
        self.bound_bill.as_ref()
    }

    fn bind_and_validate(&mut self, axes: HoleAxes, hole_id: &str) -> Result<(), HoleError> {
        let exact_bill = upgrade_bill(axes, self.axis)?;
        if self.target_level != exact_bill.target_level {
            return Err(HoleError::UpgradeBillMismatch);
        }
        self.bound_bill = Some(exact_bill);
        self.validate(axes, hole_id, Some(UpgradePhysicalStage::Reserved))
    }

    fn validate(
        &self,
        axes: HoleAxes,
        hole_id: &str,
        required_stage: Option<UpgradePhysicalStage>,
    ) -> Result<(), HoleError> {
        validate_stable_id(&self.id)?;
        validate_stable_id(&self.command_id)?;
        validate_axis(self.axis, self.target_level)?;
        if self.target_level
            != axes
                .level(self.axis)
                .checked_add(1)
                .ok_or(HoleError::ArithmeticOverflow)?
        {
            return Err(HoleError::MalformedState);
        }
        let bill = self
            .bound_bill
            .as_ref()
            .ok_or(HoleError::UpgradeBillMismatch)?;
        if bill != &upgrade_bill(axes, self.axis)? {
            return Err(HoleError::UpgradeBillMismatch);
        }
        let mut seen = BTreeSet::new();
        for input in &self.inputs {
            if !seen.insert(input.identity.clone()) {
                return Err(HoleError::DuplicateEntryIdentity);
            }
            input.validate_active(hole_id)?;
            if required_stage.is_some_and(|required| input.stage != required) {
                return Err(HoleError::InvalidPhysicalStage);
            }
        }
        validate_upgrade_bill_inputs(bill, &self.inputs)
    }
}

fn validate_upgrade_bill_inputs(
    bill: &UpgradeBill,
    inputs: &[UpgradeInput],
) -> Result<(), HoleError> {
    let mut requirements = BTreeMap::<ContentId, (u32, Option<QualityBand>)>::new();
    for requirement in &bill.physical_inputs {
        let required = requirements
            .entry(requirement.content_id.clone())
            .or_insert((0, None));
        required.0 = required
            .0
            .checked_add(requirement.quantity)
            .ok_or(HoleError::QuantityOverflow)?;
        if let Some(minimum) = requirement.minimum_quality {
            required.1 = Some(required.1.map_or(minimum, |current| current.max(minimum)));
        }
    }
    let mut supplied = BTreeMap::<ContentId, u32>::new();
    for input in inputs {
        let Some((_, minimum_quality)) = requirements.get(&input.content_id) else {
            return Err(HoleError::ContentMismatch);
        };
        if minimum_quality.is_some_and(|minimum| input.quality < minimum) {
            return Err(HoleError::QualityLocked);
        }
        let requires_item = input.content_id.as_str() == "item_generic_tool";
        if requires_item != matches!(&input.identity, FeedIdentity::ItemInstance { .. })
            || (requires_item && input.quantity != 1)
        {
            return Err(HoleError::ContentMismatch);
        }
        let total = supplied.entry(input.content_id.clone()).or_default();
        *total = total
            .checked_add(input.quantity)
            .ok_or(HoleError::QuantityOverflow)?;
    }
    if supplied.len() != requirements.len()
        || requirements
            .iter()
            .any(|(content_id, (quantity, _))| supplied.get(content_id) != Some(quantity))
    {
        return Err(HoleError::UpgradeBillMismatch);
    }
    Ok(())
}

impl UpgradeInput {
    fn validate_active(&self, hole_id: &str) -> Result<(), HoleError> {
        if self.quantity == 0 {
            return Err(HoleError::ZeroQuantity);
        }
        if self.provenance.trim().is_empty() || self.provenance.len() > MAX_UPGRADE_PROVENANCE_BYTES
        {
            return Err(HoleError::InvalidProvenance);
        }
        validate_location(&self.origin)?;
        validate_location(&self.location)?;
        validate_stable_id(&self.route_id)?;
        if matches!(&self.origin, LotLocation::Cargo(_) | LotLocation::Hole(_)) {
            return Err(HoleError::InvalidLocation);
        }
        match self.stage {
            UpgradePhysicalStage::Queued => {
                return Err(HoleError::InvalidPhysicalStage);
            }
            UpgradePhysicalStage::Reserved => {
                if self.location != self.origin || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            UpgradePhysicalStage::Carried => {
                if !matches!(&self.location, LotLocation::Cargo(_))
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            UpgradePhysicalStage::Delivered => {
                if self.location != LotLocation::Hole(hole_id.to_owned())
                    || validate_stable_id(&self.reservation_id).is_err()
                {
                    return Err(HoleError::InvalidPhysicalStage);
                }
            }
            UpgradePhysicalStage::Consumed
            | UpgradePhysicalStage::Released
            | UpgradePhysicalStage::Recovered => {
                return Err(HoleError::InvalidPhysicalStage);
            }
        }
        Ok(())
    }
}

impl RecoveredUpgradeInput {
    fn validate(&self) -> Result<(), HoleError> {
        if self.input.quantity == 0 {
            return Err(HoleError::ZeroQuantity);
        }
        if self.input.provenance.trim().is_empty()
            || self.input.provenance.len() > MAX_UPGRADE_PROVENANCE_BYTES
        {
            return Err(HoleError::InvalidProvenance);
        }
        validate_location(&self.input.origin)?;
        validate_location(&self.input.location)?;
        validate_stable_id(&self.input.route_id)?;
        if matches!(
            &self.input.origin,
            LotLocation::Cargo(_) | LotLocation::Hole(_)
        ) {
            return Err(HoleError::InvalidLocation);
        }
        if !self.input.reservation_id.is_empty() {
            return Err(HoleError::InvalidPhysicalStage);
        }
        match self.input.stage {
            UpgradePhysicalStage::Released => {
                if self.disposition != RecoveryDisposition::ReleasedAtOrigin
                    || self.input.location != self.input.origin
                {
                    return Err(HoleError::RecoveryDispositionRejected);
                }
            }
            UpgradePhysicalStage::Recovered => {
                if self.input.location != recovery_location(&self.input.origin, &self.disposition)?
                {
                    return Err(HoleError::RecoveryDispositionRejected);
                }
            }
            UpgradePhysicalStage::Queued
            | UpgradePhysicalStage::Reserved
            | UpgradePhysicalStage::Carried
            | UpgradePhysicalStage::Delivered
            | UpgradePhysicalStage::Consumed => {
                return Err(HoleError::InvalidPhysicalStage);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpeningCredit {
    pub id: String,
    pub opening_id: String,
    pub order_id: String,
    pub identity: FeedIdentity,
    pub quantity: u32,
    pub micro_void: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningResult {
    pub opening_id: String,
    pub game_minute: u64,
    pub credits: Vec<OpeningCredit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandApply {
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandReceipt {
    id: String,
    sequence: u64,
    fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum RecoveryDisposition {
    ReleasedAtOrigin,
    ReturnedToOrigin,
    NearestStockpile {
        stockpile_id: String,
    },
    LastLandCache {
        cache_id: String,
        #[serde(with = "tile_point_serde")]
        tile: TilePoint,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryCause {
    Cancelled,
    CarrierDeath,
    RouteLost,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecoveryReceipt {
    id: String,
    sequence: u64,
    fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryApply {
    Applied,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeRecoveryRequest {
    pub identity: FeedIdentity,
    pub disposition: RecoveryDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveredUpgradeInput {
    pub disposition: RecoveryDisposition,
    pub input: UpgradeInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveredAxisUpgrade {
    pub id: String,
    pub project_id: String,
    pub axis: HoleAxis,
    pub target_level: u8,
    pub bound_bill: UpgradeBill,
    pub cause: RecoveryCause,
    pub recovered_inputs: Vec<RecoveredUpgradeInput>,
}

impl RecoveredAxisUpgrade {
    fn validate(&self) -> Result<(), HoleError> {
        validate_stable_id(&self.id)?;
        validate_stable_id(&self.project_id)?;
        validate_axis(self.axis, self.target_level)?;
        if self.target_level == 0
            || self.bound_bill.axis != self.axis
            || self.bound_bill.target_level != self.target_level
        {
            return Err(HoleError::UpgradeBillMismatch);
        }
        let prior_level = self
            .target_level
            .checked_sub(1)
            .ok_or(HoleError::UpgradeBillMismatch)?;
        let prior_axes = match self.axis {
            HoleAxis::Width => HoleAxes::new(prior_level, 0, 0)?,
            HoleAxis::Depth => HoleAxes::new(0, prior_level, 0)?,
            HoleAxis::Darkness => HoleAxes::new(0, 0, prior_level)?,
        };
        if self.bound_bill != upgrade_bill(prior_axes, self.axis)? {
            return Err(HoleError::UpgradeBillMismatch);
        }
        let mut identities = BTreeSet::new();
        for recovery in &self.recovered_inputs {
            recovery.validate()?;
            if !identities.insert(recovery.input.identity.clone()) {
                return Err(HoleError::DuplicateEntryIdentity);
            }
        }
        let inputs = self
            .recovered_inputs
            .iter()
            .map(|recovery| recovery.input.clone())
            .collect::<Vec<_>>();
        validate_upgrade_bill_inputs(&self.bound_bill, &inputs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletedAxisUpgrade {
    pub id: String,
    pub project_id: String,
    pub axis: HoleAxis,
    pub target_level: u8,
    pub bound_bill: UpgradeBill,
    pub consumed_inputs: Vec<UpgradeInput>,
}

impl CompletedAxisUpgrade {
    fn validate(&self, hole_id: &str) -> Result<(), HoleError> {
        validate_stable_id(&self.id)?;
        validate_stable_id(&self.project_id)?;
        validate_axis(self.axis, self.target_level)?;
        if self.target_level == 0
            || self.bound_bill.axis != self.axis
            || self.bound_bill.target_level != self.target_level
        {
            return Err(HoleError::UpgradeBillMismatch);
        }
        let prior_level = self
            .target_level
            .checked_sub(1)
            .ok_or(HoleError::UpgradeBillMismatch)?;
        let prior_axes = match self.axis {
            HoleAxis::Width => HoleAxes::new(prior_level, 0, 0)?,
            HoleAxis::Depth => HoleAxes::new(0, prior_level, 0)?,
            HoleAxis::Darkness => HoleAxes::new(0, 0, prior_level)?,
        };
        if self.bound_bill != upgrade_bill(prior_axes, self.axis)? {
            return Err(HoleError::UpgradeBillMismatch);
        }
        let mut identities = BTreeSet::new();
        for input in &self.consumed_inputs {
            if input.quantity == 0
                || !identities.insert(input.identity.clone())
                || input.stage != UpgradePhysicalStage::Consumed
                || input.location != LotLocation::Hole(hole_id.to_owned())
                || !input.reservation_id.is_empty()
            {
                return Err(HoleError::InvalidPhysicalStage);
            }
            if input.provenance.trim().is_empty()
                || input.provenance.len() > MAX_UPGRADE_PROVENANCE_BYTES
            {
                return Err(HoleError::InvalidProvenance);
            }
            validate_location(&input.origin)?;
            validate_location(&input.location)?;
            validate_stable_id(&input.route_id)?;
            if matches!(&input.origin, LotLocation::Cargo(_) | LotLocation::Hole(_)) {
                return Err(HoleError::InvalidLocation);
            }
        }
        validate_upgrade_bill_inputs(&self.bound_bill, &self.consumed_inputs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlackHoleState {
    pub hole_id: String,
    pub anchor: TilePoint,
    pub axes: HoleAxes,
    pub next_opening_game_minute: u64,
    pub next_opening_index: u64,
    pub micro_void_balance: u64,
    pub active_feed: Option<FeedOrder>,
    pub active_upgrade: Option<AxisUpgradeProject>,
    credits: Vec<OpeningCredit>,
    terminal_entries: Vec<FeedEntry>,
    terminal_upgrade_recoveries: Vec<RecoveredAxisUpgrade>,
    completed_upgrades: Vec<CompletedAxisUpgrade>,
    command_receipts: BTreeMap<String, CommandReceipt>,
    recovery_receipts: BTreeMap<String, RecoveryReceipt>,
    next_command_sequence: u64,
    next_recovery_sequence: u64,
}

impl BlackHoleState {
    pub fn new(
        hole_id: impl Into<String>,
        anchor: TilePoint,
        axes: HoleAxes,
        first_opening_game_minute: u64,
    ) -> Result<Self, HoleError> {
        let hole_id = hole_id.into();
        validate_stable_id(&hole_id)?;
        let state = Self {
            hole_id,
            anchor,
            axes,
            next_opening_game_minute: first_opening_game_minute,
            next_opening_index: 0,
            micro_void_balance: 0,
            active_feed: None,
            active_upgrade: None,
            credits: Vec::new(),
            terminal_entries: Vec::new(),
            terminal_upgrade_recoveries: Vec::new(),
            completed_upgrades: Vec::new(),
            command_receipts: BTreeMap::new(),
            recovery_receipts: BTreeMap::new(),
            next_command_sequence: 0,
            next_recovery_sequence: 0,
        };
        state.validate()?;
        Ok(state)
    }

    #[must_use]
    pub fn footprint(&self) -> HoleFootprint {
        hole_footprint(self.anchor).expect("validated Hole anchor has a fixed footprint")
    }

    #[must_use]
    pub fn credits(&self) -> &[OpeningCredit] {
        &self.credits
    }

    #[must_use]
    pub fn terminal_entries(&self) -> &[FeedEntry] {
        &self.terminal_entries
    }

    #[must_use]
    pub fn terminal_upgrade_recoveries(&self) -> &[RecoveredAxisUpgrade] {
        &self.terminal_upgrade_recoveries
    }

    #[must_use]
    pub fn completed_upgrades(&self) -> &[CompletedAxisUpgrade] {
        &self.completed_upgrades
    }

    /// Deterministically drains acknowledged feed credits, preventing history
    /// growth while preserving every undrained physical record.
    pub fn take_credits(&mut self, maximum: usize) -> Vec<OpeningCredit> {
        let count = maximum.min(self.credits.len());
        self.credits.drain(..count).collect()
    }

    /// Deterministically drains recovered physical entries for the runtime
    /// owner to return to the canonical lot ledger.
    pub fn take_terminal_entries(&mut self, maximum: usize) -> Vec<FeedEntry> {
        let count = maximum.min(self.terminal_entries.len());
        self.terminal_entries.drain(..count).collect()
    }

    /// Drains recovered upgrade cargo only after handing the complete physical
    /// records to the canonical lot/item ledger owner.
    pub fn take_terminal_upgrade_recoveries(
        &mut self,
        maximum: usize,
    ) -> Vec<RecoveredAxisUpgrade> {
        let count = maximum.min(self.terminal_upgrade_recoveries.len());
        self.terminal_upgrade_recoveries.drain(..count).collect()
    }

    pub fn take_completed_upgrades(&mut self, maximum: usize) -> Vec<CompletedAxisUpgrade> {
        let count = maximum.min(self.completed_upgrades.len());
        self.completed_upgrades.drain(..count).collect()
    }

    fn command_replay(
        &self,
        command_id: &str,
        fingerprint: &str,
    ) -> Result<Option<CommandApply>, HoleError> {
        validate_stable_id(command_id)?;
        self.command_receipts
            .get(command_id)
            .map_or(Ok(None), |receipt| {
                if receipt.fingerprint == fingerprint {
                    Ok(Some(CommandApply::AlreadyApplied))
                } else {
                    Err(HoleError::CommandConflict)
                }
            })
    }

    fn record_command(&mut self, command_id: String, fingerprint: String) -> Result<(), HoleError> {
        let sequence = self.next_command_sequence;
        self.next_command_sequence = self
            .next_command_sequence
            .checked_add(1)
            .ok_or(HoleError::ArithmeticOverflow)?;
        self.command_receipts.insert(
            command_id.clone(),
            CommandReceipt {
                id: command_id,
                sequence,
                fingerprint,
            },
        );
        prune_command_receipts(&mut self.command_receipts, MAX_COMMAND_RECEIPTS);
        Ok(())
    }

    fn record_recovery(
        &mut self,
        recovery_id: String,
        fingerprint: String,
    ) -> Result<(), HoleError> {
        let sequence = self.next_recovery_sequence;
        self.next_recovery_sequence = self
            .next_recovery_sequence
            .checked_add(1)
            .ok_or(HoleError::ArithmeticOverflow)?;
        self.recovery_receipts.insert(
            recovery_id.clone(),
            RecoveryReceipt {
                id: recovery_id,
                sequence,
                fingerprint,
            },
        );
        prune_recovery_receipts(&mut self.recovery_receipts, MAX_RECOVERY_RECEIPTS);
        Ok(())
    }

    fn physical_identity_is_pending(&self, identity: &FeedIdentity) -> bool {
        self.active_feed.as_ref().is_some_and(|order| {
            order
                .entries
                .iter()
                .any(|entry| &entry.identity == identity)
        }) || self.active_upgrade.as_ref().is_some_and(|project| {
            project
                .inputs
                .iter()
                .any(|input| &input.identity == identity)
        }) || self
            .credits
            .iter()
            .any(|credit| &credit.identity == identity)
            || self
                .terminal_entries
                .iter()
                .any(|entry| &entry.identity == identity)
            || self.terminal_upgrade_recoveries.iter().any(|recovery| {
                recovery
                    .recovered_inputs
                    .iter()
                    .any(|input| &input.input.identity == identity)
            })
            || self.completed_upgrades.iter().any(|completion| {
                completion
                    .consumed_inputs
                    .iter()
                    .any(|input| &input.identity == identity)
            })
    }

    pub fn begin_feed(&mut self, order: FeedOrder) -> Result<CommandApply, HoleError> {
        let mut next = self.clone();
        let fingerprint = format!("begin_feed:{order:?}");
        if let Some(replay) = next.command_replay(&order.command_id, &fingerprint)? {
            return Ok(replay);
        }
        order.validate(next.axes, &next.hole_id)?;
        if order
            .entries
            .iter()
            .any(|entry| next.physical_identity_is_pending(&entry.identity))
        {
            return Err(HoleError::DuplicateEntryIdentity);
        }
        if next.active_feed.is_some() {
            return Err(HoleError::ActiveFeedExists);
        }
        next.record_command(order.command_id.clone(), fingerprint)?;
        next.active_feed = Some(order);
        next.validate()?;
        *self = next;
        Ok(CommandApply::Applied)
    }

    pub fn begin_upgrade(
        &mut self,
        mut project: AxisUpgradeProject,
    ) -> Result<CommandApply, HoleError> {
        let mut next = self.clone();
        let fingerprint = format!(
            "begin_upgrade:{:?}:{:?}:{:?}:{:?}",
            project.id, project.axis, project.target_level, project.inputs
        );
        if let Some(replay) = next.command_replay(&project.command_id, &fingerprint)? {
            return Ok(replay);
        }
        project.bind_and_validate(next.axes, &next.hole_id)?;
        if project
            .inputs
            .iter()
            .any(|input| next.physical_identity_is_pending(&input.identity))
        {
            return Err(HoleError::DuplicateEntryIdentity);
        }
        if next.active_upgrade.is_some() {
            return Err(HoleError::ActiveUpgradeExists);
        }
        next.record_command(project.command_id.clone(), fingerprint)?;
        next.active_upgrade = Some(project);
        next.validate()?;
        *self = next;
        Ok(CommandApply::Applied)
    }

    pub fn mark_upgrade_carried(
        &mut self,
        command_id: String,
        project_id: &str,
        identity: &FeedIdentity,
        cargo_id: String,
    ) -> Result<CommandApply, HoleError> {
        validate_stable_id(&cargo_id)?;
        self.transition_upgrade_input(
            command_id,
            project_id,
            identity,
            UpgradePhysicalStage::Reserved,
            UpgradePhysicalStage::Carried,
            Some(LotLocation::Cargo(cargo_id)),
        )
    }

    pub fn mark_upgrade_delivered(
        &mut self,
        command_id: String,
        project_id: &str,
        identity: &FeedIdentity,
    ) -> Result<CommandApply, HoleError> {
        self.transition_upgrade_input(
            command_id,
            project_id,
            identity,
            UpgradePhysicalStage::Carried,
            UpgradePhysicalStage::Delivered,
            Some(LotLocation::Hole(self.hole_id.clone())),
        )
    }

    pub fn complete_upgrade(
        &mut self,
        command_id: String,
        project_id: &str,
    ) -> Result<CommandApply, HoleError> {
        let mut next = self.clone();
        let fingerprint = format!("complete_upgrade:{project_id}");
        if let Some(replay) = next.command_replay(&command_id, &fingerprint)? {
            return Ok(replay);
        }
        let project = next
            .active_upgrade
            .as_ref()
            .ok_or(HoleError::NoActiveUpgrade)?;
        if project.id != project_id {
            return Err(HoleError::UnknownUpgrade);
        }
        if project
            .inputs
            .iter()
            .any(|input| input.stage != UpgradePhysicalStage::Delivered)
        {
            return Err(HoleError::InvalidPhysicalStage);
        }
        if next.completed_upgrades.len() >= MAX_PENDING_UPGRADE_COMPLETIONS {
            return Err(HoleError::OutputBackpressure);
        }
        let mut project = next
            .active_upgrade
            .take()
            .ok_or(HoleError::NoActiveUpgrade)?;
        let bound_bill = project
            .bound_bill
            .take()
            .ok_or(HoleError::UpgradeBillMismatch)?;
        for input in &mut project.inputs {
            input.stage = UpgradePhysicalStage::Consumed;
            input.reservation_id.clear();
        }
        let new_level = project.target_level;
        match project.axis {
            HoleAxis::Width => next.axes.width = new_level,
            HoleAxis::Depth => next.axes.depth = new_level,
            HoleAxis::Darkness => next.axes.darkness = new_level,
        }
        let target_level_id = new_level.to_string();
        next.completed_upgrades.push(CompletedAxisUpgrade {
            id: deterministic_id(
                "upgrade_completion",
                &[project_id, target_level_id.as_str()],
            ),
            project_id: project.id,
            axis: project.axis,
            target_level: new_level,
            bound_bill,
            consumed_inputs: project.inputs,
        });
        next.record_command(command_id, fingerprint)?;
        next.validate()?;
        *self = next;
        Ok(CommandApply::Applied)
    }

    fn transition_upgrade_input(
        &mut self,
        command_id: String,
        project_id: &str,
        identity: &FeedIdentity,
        expected: UpgradePhysicalStage,
        stage: UpgradePhysicalStage,
        location: Option<LotLocation>,
    ) -> Result<CommandApply, HoleError> {
        let mut next = self.clone();
        let fingerprint = format!(
            "upgrade_transition:{project_id:?}:{identity:?}:{expected:?}:{stage:?}:{location:?}"
        );
        if let Some(replay) = next.command_replay(&command_id, &fingerprint)? {
            return Ok(replay);
        }
        let project = next
            .active_upgrade
            .as_mut()
            .ok_or(HoleError::NoActiveUpgrade)?;
        if project.id != project_id {
            return Err(HoleError::UnknownUpgrade);
        }
        let input = project
            .inputs
            .iter_mut()
            .find(|input| &input.identity == identity)
            .ok_or(HoleError::UnknownEntry)?;
        if input.stage != expected {
            return Err(HoleError::InvalidPhysicalStage);
        }
        input.stage = stage;
        if let Some(location) = location {
            input.location = location;
        }
        next.record_command(command_id, fingerprint)?;
        next.validate()?;
        *self = next;
        Ok(CommandApply::Applied)
    }

    pub fn mark_carried(
        &mut self,
        order_id: &str,
        identity: &FeedIdentity,
        cargo_id: String,
    ) -> Result<(), HoleError> {
        validate_stable_id(&cargo_id)?;
        self.transition_entry(
            order_id,
            identity,
            FeedPhysicalStage::Reserved,
            FeedPhysicalStage::Carried,
            LotLocation::Cargo(cargo_id),
        )
    }

    pub fn mark_delivered(
        &mut self,
        order_id: &str,
        identity: &FeedIdentity,
    ) -> Result<(), HoleError> {
        self.transition_entry(
            order_id,
            identity,
            FeedPhysicalStage::Carried,
            FeedPhysicalStage::Delivered,
            LotLocation::Hole(self.hole_id.clone()),
        )
    }

    fn transition_entry(
        &mut self,
        order_id: &str,
        identity: &FeedIdentity,
        expected: FeedPhysicalStage,
        next_stage: FeedPhysicalStage,
        location: LotLocation,
    ) -> Result<(), HoleError> {
        let mut next = self.clone();
        let order = next.active_feed.as_mut().ok_or(HoleError::NoActiveFeed)?;
        if order.id != order_id {
            return Err(HoleError::UnknownOrder);
        }
        let entry_index = order
            .entries
            .iter()
            .position(|entry| &entry.identity == identity)
            .ok_or(HoleError::UnknownEntry)?;
        let entry = order
            .entries
            .get_mut(entry_index)
            .expect("entry index was found in this order");
        if entry.stage != expected {
            return Err(HoleError::InvalidPhysicalStage);
        }
        entry.location = location;
        entry.stage = next_stage;
        next.validate()?;
        *self = next;
        Ok(())
    }

    /// Process every due opening that has delivered physical units. A missing
    /// delivered unit leaves the absolute cursor untouched for later delivery.
    pub fn advance_to(&mut self, game_minute: u64) -> Result<Vec<OpeningResult>, HoleError> {
        let mut next = self.clone();
        let mut results = Vec::new();
        while next.next_opening_game_minute <= game_minute {
            if !next.has_delivered_units()? {
                break;
            }
            results.push(next.process_one_opening()?);
            if next.active_feed.is_none() {
                break;
            }
        }
        next.validate()?;
        *self = next;
        Ok(results)
    }

    fn has_delivered_units(&self) -> Result<bool, HoleError> {
        self.active_feed.as_ref().map_or(Ok(false), |order| {
            order.entries.iter().try_fold(false, |any, entry| {
                Ok(any
                    || (entry.stage == FeedPhysicalStage::Delivered
                        && entry.remaining_units()? > 0))
            })
        })
    }

    fn process_one_opening(&mut self) -> Result<OpeningResult, HoleError> {
        let opening_index = self.next_opening_index;
        let opening_game_minute = self.next_opening_game_minute;
        let order_id = self
            .active_feed
            .as_ref()
            .ok_or(HoleError::NoActiveFeed)?
            .id
            .clone();
        let opening_id = opening_id(&order_id, opening_index);
        let mut capacity = self.axes.intake_width();
        let mut credits = Vec::new();
        let existing_credit_ids = self
            .credits
            .iter()
            .map(|credit| credit.id.clone())
            .collect::<BTreeSet<_>>();
        {
            let order = self.active_feed.as_mut().ok_or(HoleError::NoActiveFeed)?;
            for (sequence, entry) in order.entries.iter_mut().enumerate() {
                if capacity == 0 || entry.stage != FeedPhysicalStage::Delivered {
                    continue;
                }
                let remaining = entry.remaining_units()?;
                if remaining == 0 {
                    continue;
                }
                let quantity = remaining.min(capacity);
                let unit_reward = entry.policy.micro_void_for(entry.quality)?;
                let micro_void = unit_reward
                    .checked_mul(u64::from(quantity))
                    .ok_or(HoleError::ArithmeticOverflow)?;
                let credit_id = credit_id(
                    &order_id,
                    opening_index,
                    u32::try_from(sequence).map_err(|_| HoleError::ArithmeticOverflow)?,
                );
                if existing_credit_ids.contains(&credit_id) {
                    return Err(HoleError::MalformedState);
                }
                entry.credited_units = entry
                    .credited_units
                    .checked_add(quantity)
                    .ok_or(HoleError::QuantityOverflow)?;
                if entry.credited_units == entry.units {
                    entry.stage = FeedPhysicalStage::Credited;
                }
                capacity -= quantity;
                credits.push(OpeningCredit {
                    id: credit_id,
                    opening_id: opening_id.clone(),
                    order_id: order_id.clone(),
                    identity: entry.identity.clone(),
                    quantity,
                    micro_void,
                });
            }
        }
        if credits.is_empty() {
            return Err(HoleError::NoActiveFeed);
        }
        let opening_void = credits.iter().try_fold(0_u64, |total, credit| {
            total
                .checked_add(credit.micro_void)
                .ok_or(HoleError::ArithmeticOverflow)
        })?;
        if self
            .credits
            .len()
            .checked_add(credits.len())
            .ok_or(HoleError::ArithmeticOverflow)?
            > MAX_OUTPUT_HISTORY
        {
            return Err(HoleError::OutputBackpressure);
        }
        self.micro_void_balance = self
            .micro_void_balance
            .checked_add(opening_void)
            .ok_or(HoleError::ArithmeticOverflow)?;
        self.credits.extend(credits.iter().cloned());
        self.next_opening_index = self
            .next_opening_index
            .checked_add(1)
            .ok_or(HoleError::ArithmeticOverflow)?;
        self.next_opening_game_minute = self
            .next_opening_game_minute
            .checked_add(OPENING_GAME_MINUTES)
            .ok_or(HoleError::ArithmeticOverflow)?;
        if self.active_feed.as_ref().is_some_and(|order| {
            order
                .entries
                .iter()
                .all(|entry| entry.credited_units == entry.units)
        }) {
            self.active_feed = None;
        }
        Ok(OpeningResult {
            opening_id,
            game_minute: opening_game_minute,
            credits,
        })
    }

    pub fn recover_entry(
        &mut self,
        recovery_id: String,
        order_id: &str,
        identity: &FeedIdentity,
        cause: RecoveryCause,
        disposition: RecoveryDisposition,
    ) -> Result<RecoveryApply, HoleError> {
        validate_stable_id(&recovery_id)?;
        let fingerprint = format!("{order_id:?}:{identity:?}:{cause:?}:{disposition:?}");
        if let Some(receipt) = self.recovery_receipts.get(&recovery_id) {
            return if receipt.fingerprint == fingerprint {
                Ok(RecoveryApply::AlreadyApplied)
            } else {
                Err(HoleError::RecoveryConflict)
            };
        }
        let mut next = self.clone();
        let order = next.active_feed.as_mut().ok_or(HoleError::NoActiveFeed)?;
        if order.id != order_id {
            return Err(HoleError::UnknownOrder);
        }
        let entry_index = order
            .entries
            .iter()
            .position(|entry| &entry.identity == identity)
            .ok_or(HoleError::UnknownEntry)?;
        let entry = order
            .entries
            .get_mut(entry_index)
            .expect("entry index was found in this order");
        match entry.stage {
            FeedPhysicalStage::Queued | FeedPhysicalStage::Reserved => {
                if disposition != RecoveryDisposition::ReleasedAtOrigin {
                    return Err(HoleError::RecoveryDispositionRejected);
                }
                entry.location = entry.origin.clone();
                entry.stage = FeedPhysicalStage::Released;
            }
            FeedPhysicalStage::Carried | FeedPhysicalStage::Delivered => {
                entry.location = recovery_location(&entry.origin, &disposition)?;
                entry.stage = FeedPhysicalStage::Recovered;
            }
            _ => return Err(HoleError::RecoveryDispositionRejected),
        }
        entry.reservation_id.clear();
        let terminal_entry = order.entries.remove(entry_index);
        if next.terminal_entries.len() >= MAX_OUTPUT_HISTORY {
            return Err(HoleError::OutputBackpressure);
        }
        next.terminal_entries.push(terminal_entry);
        next.record_recovery(recovery_id, fingerprint)?;
        if next.active_feed.as_ref().is_some_and(|order| {
            order.entries.is_empty()
                || order
                    .entries
                    .iter()
                    .all(|entry| entry.credited_units == entry.units)
        }) {
            next.active_feed = None;
        }
        next.validate()?;
        *self = next;
        Ok(RecoveryApply::Applied)
    }

    pub fn recover_upgrade(
        &mut self,
        recovery_id: String,
        project_id: &str,
        cause: RecoveryCause,
        requests: Vec<UpgradeRecoveryRequest>,
    ) -> Result<RecoveryApply, HoleError> {
        validate_stable_id(&recovery_id)?;
        let fingerprint = format!("upgrade:{project_id:?}:{cause:?}:{requests:?}");
        if let Some(receipt) = self.recovery_receipts.get(&recovery_id) {
            return if receipt.fingerprint == fingerprint {
                Ok(RecoveryApply::AlreadyApplied)
            } else {
                Err(HoleError::RecoveryConflict)
            };
        }
        let mut next = self.clone();
        let project = next
            .active_upgrade
            .take()
            .ok_or(HoleError::NoActiveUpgrade)?;
        if project.id != project_id {
            return Err(HoleError::UnknownUpgrade);
        }
        let recoverable_count = project.inputs.len();
        if recoverable_count == 0 || recoverable_count != requests.len() {
            return Err(HoleError::RecoveryDispositionRejected);
        }
        if next.terminal_upgrade_recoveries.len() >= MAX_OUTPUT_HISTORY {
            return Err(HoleError::OutputBackpressure);
        }
        let mut dispositions = BTreeMap::new();
        for request in requests {
            if dispositions
                .insert(request.identity, request.disposition)
                .is_some()
            {
                return Err(HoleError::DuplicateEntryIdentity);
            }
        }
        let mut recovered = Vec::with_capacity(recoverable_count);
        for mut input in project.inputs {
            let disposition = dispositions
                .remove(&input.identity)
                .ok_or(HoleError::UnknownEntry)?;
            match input.stage {
                UpgradePhysicalStage::Reserved => {
                    if disposition != RecoveryDisposition::ReleasedAtOrigin {
                        return Err(HoleError::RecoveryDispositionRejected);
                    }
                    input.location = input.origin.clone();
                    input.stage = UpgradePhysicalStage::Released;
                }
                UpgradePhysicalStage::Carried | UpgradePhysicalStage::Delivered => {
                    input.location = recovery_location(&input.origin, &disposition)?;
                    input.stage = UpgradePhysicalStage::Recovered;
                }
                UpgradePhysicalStage::Queued
                | UpgradePhysicalStage::Consumed
                | UpgradePhysicalStage::Released
                | UpgradePhysicalStage::Recovered => {
                    return Err(HoleError::RecoveryDispositionRejected);
                }
            }
            input.reservation_id.clear();
            recovered.push(RecoveredUpgradeInput { disposition, input });
        }
        if !dispositions.is_empty() {
            return Err(HoleError::UnknownEntry);
        }
        next.terminal_upgrade_recoveries.push(RecoveredAxisUpgrade {
            id: deterministic_id("upgrade_recovery", &[recovery_id.as_str()]),
            project_id: project.id,
            axis: project.axis,
            target_level: project.target_level,
            bound_bill: project.bound_bill.ok_or(HoleError::UpgradeBillMismatch)?,
            cause,
            recovered_inputs: recovered,
        });
        next.record_recovery(recovery_id, fingerprint)?;
        next.validate()?;
        *self = next;
        Ok(RecoveryApply::Applied)
    }

    fn validate(&self) -> Result<(), HoleError> {
        validate_stable_id(&self.hole_id)?;
        let _ = hole_footprint(self.anchor)?;
        validate_axis(HoleAxis::Width, self.axes.width)?;
        validate_axis(HoleAxis::Depth, self.axes.depth)?;
        validate_axis(HoleAxis::Darkness, self.axes.darkness)?;
        if self.command_receipts.len() > MAX_COMMAND_RECEIPTS
            || self.recovery_receipts.len() > MAX_RECOVERY_RECEIPTS
            || self.credits.len() > MAX_OUTPUT_HISTORY
            || self.terminal_entries.len() > MAX_OUTPUT_HISTORY
            || self.terminal_upgrade_recoveries.len() > MAX_OUTPUT_HISTORY
            || self.completed_upgrades.len() > MAX_PENDING_UPGRADE_COMPLETIONS
        {
            return Err(HoleError::MalformedState);
        }
        let mut command_sequences = BTreeSet::new();
        if self.command_receipts.iter().any(|(key, receipt)| {
            key != &receipt.id
                || validate_stable_id(&receipt.id).is_err()
                || receipt.fingerprint.is_empty()
                || receipt.sequence >= self.next_command_sequence
                || !command_sequences.insert(receipt.sequence)
        }) {
            return Err(HoleError::MalformedState);
        }
        let mut recovery_sequences = BTreeSet::new();
        if self.recovery_receipts.iter().any(|(key, receipt)| {
            key != &receipt.id
                || validate_stable_id(&receipt.id).is_err()
                || receipt.fingerprint.is_empty()
                || receipt.sequence >= self.next_recovery_sequence
                || !recovery_sequences.insert(receipt.sequence)
        }) {
            return Err(HoleError::MalformedState);
        }
        if let Some(order) = &self.active_feed {
            order.validate(self.axes, &self.hole_id)?;
        }
        for entry in &self.terminal_entries {
            entry.validate(self.axes, &self.hole_id)?;
        }
        let mut upgrade_recovery_ids = BTreeSet::new();
        for recovery in &self.terminal_upgrade_recoveries {
            recovery.validate()?;
            if recovery.target_level
                > self
                    .axes
                    .level(recovery.axis)
                    .checked_add(1)
                    .ok_or(HoleError::ArithmeticOverflow)?
            {
                return Err(HoleError::MalformedState);
            }
            if !upgrade_recovery_ids.insert(recovery.id.clone()) {
                return Err(HoleError::MalformedState);
            }
        }
        let mut completion_ids = BTreeSet::new();
        for completion in &self.completed_upgrades {
            completion.validate(&self.hole_id)?;
            if completion.target_level > self.axes.level(completion.axis) {
                return Err(HoleError::MalformedState);
            }
            if !completion_ids.insert(completion.id.clone()) {
                return Err(HoleError::MalformedState);
            }
        }
        let mut upgrade_physical_identities = BTreeSet::new();
        if let Some(project) = &self.active_upgrade {
            for input in &project.inputs {
                if !upgrade_physical_identities.insert(input.identity.clone()) {
                    return Err(HoleError::DuplicateEntryIdentity);
                }
            }
        }
        for recovery in &self.terminal_upgrade_recoveries {
            for input in &recovery.recovered_inputs {
                if !upgrade_physical_identities.insert(input.input.identity.clone()) {
                    return Err(HoleError::DuplicateEntryIdentity);
                }
            }
        }
        for completion in &self.completed_upgrades {
            for input in &completion.consumed_inputs {
                if !upgrade_physical_identities.insert(input.identity.clone()) {
                    return Err(HoleError::DuplicateEntryIdentity);
                }
            }
        }
        if self.active_feed.as_ref().is_some_and(|order| {
            order
                .entries
                .iter()
                .any(|entry| upgrade_physical_identities.contains(&entry.identity))
        }) || self
            .terminal_entries
            .iter()
            .any(|entry| upgrade_physical_identities.contains(&entry.identity))
            || self
                .credits
                .iter()
                .any(|credit| upgrade_physical_identities.contains(&credit.identity))
        {
            return Err(HoleError::DuplicateEntryIdentity);
        }
        if let Some(project) = &self.active_upgrade {
            project.validate(self.axes, &self.hole_id, None)?;
        }
        let mut ids = BTreeSet::new();
        if self.credits.iter().any(|credit| {
            validate_stable_id(&credit.id).is_err()
                || validate_stable_id(&credit.opening_id).is_err()
                || validate_stable_id(&credit.order_id).is_err()
                || !ids.insert(credit.id.clone())
        }) {
            return Err(HoleError::MalformedState);
        }
        Ok(())
    }
}

fn recovery_location(
    origin: &LotLocation,
    disposition: &RecoveryDisposition,
) -> Result<LotLocation, HoleError> {
    match disposition {
        RecoveryDisposition::ReleasedAtOrigin => Err(HoleError::RecoveryDispositionRejected),
        RecoveryDisposition::ReturnedToOrigin => Ok(origin.clone()),
        RecoveryDisposition::NearestStockpile { stockpile_id } => {
            validate_stable_id(stockpile_id)?;
            Ok(LotLocation::Stockpile(stockpile_id.clone()))
        }
        RecoveryDisposition::LastLandCache { cache_id, .. } => {
            validate_stable_id(cache_id)?;
            Ok(LotLocation::Cache(cache_id.clone()))
        }
    }
}

fn validate_location(location: &LotLocation) -> Result<(), HoleError> {
    let identifier = match location {
        LotLocation::Source(id)
        | LotLocation::Stockpile(id)
        | LotLocation::StationInput(id)
        | LotLocation::StationOutput(id)
        | LotLocation::Cargo(id)
        | LotLocation::Cache(id)
        | LotLocation::Hole(id) => id,
    };
    validate_stable_id(identifier).map_err(|_| HoleError::InvalidLocation)
}

fn validate_stable_id(value: &str) -> Result<(), HoleError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase()
        || !bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(HoleError::InvalidIdentity(value.to_owned()));
    }
    Ok(())
}

fn prune_command_receipts(receipts: &mut BTreeMap<String, CommandReceipt>, maximum: usize) {
    while receipts.len() > maximum {
        let Some(oldest) = receipts
            .iter()
            .min_by(|left, right| {
                left.1
                    .sequence
                    .cmp(&right.1.sequence)
                    .then_with(|| left.0.cmp(right.0))
            })
            .map(|(id, _)| id.clone())
        else {
            break;
        };
        receipts.remove(&oldest);
    }
}

fn prune_recovery_receipts(receipts: &mut BTreeMap<String, RecoveryReceipt>, maximum: usize) {
    while receipts.len() > maximum {
        let Some(oldest) = receipts
            .iter()
            .min_by(|left, right| {
                left.1
                    .sequence
                    .cmp(&right.1.sequence)
                    .then_with(|| left.0.cmp(right.0))
            })
            .map(|(id, _)| id.clone())
        else {
            break;
        };
        receipts.remove(&oldest);
    }
}

fn deterministic_id(namespace: &str, parts: &[&str]) -> String {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for part in std::iter::once(namespace).chain(parts.iter().copied()) {
        for byte in part.bytes().chain(std::iter::once(0xff)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    format!("{namespace}_{hash:016x}")
}

#[must_use]
pub fn opening_id(order_id: &str, opening_index: u64) -> String {
    deterministic_id("opening", &[order_id, &opening_index.to_string()])
}

#[must_use]
pub fn credit_id(order_id: &str, opening_index: u64, sequence: u32) -> String {
    deterministic_id(
        "credit",
        &[order_id, &opening_index.to_string(), &sequence.to_string()],
    )
}

#[must_use]
pub fn recovery_id(order_id: &str, identity: &FeedIdentity, sequence: u64) -> String {
    deterministic_id(
        "recovery",
        &[order_id, &format!("{identity:?}"), &sequence.to_string()],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalRequirement {
    pub content_id: ContentId,
    pub quantity: u32,
    pub minimum_quality: Option<QualityBand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpgradeBill {
    pub axis: HoleAxis,
    pub target_level: u8,
    pub physical_inputs: Vec<PhysicalRequirement>,
}

pub fn upgrade_bill(axes: HoleAxes, axis: HoleAxis) -> Result<UpgradeBill, HoleError> {
    let target_level = axes
        .level(axis)
        .checked_add(1)
        .ok_or(HoleError::ArithmeticOverflow)?;
    validate_axis(axis, target_level)?;
    let mut physical_inputs = vec![requirement(
        "resource_refined",
        5 * u32::from(target_level),
        None,
    )];
    let (raw, processed) = match axis {
        HoleAxis::Width => ("resource_logs", "resource_planks"),
        HoleAxis::Depth => ("resource_stone", "resource_blocks"),
        HoleAxis::Darkness => ("resource_herbs", "resource_refined"),
    };
    physical_inputs.push(requirement(raw, 2 * u32::from(target_level), None));
    if target_level >= 4 {
        physical_inputs.push(requirement(
            processed,
            2 * u32::from(target_level - 3),
            None,
        ));
    }
    if target_level >= 7 {
        physical_inputs.push(requirement(
            "resource_metal",
            2 * u32::from(target_level - 6),
            None,
        ));
    }
    if target_level == 10 {
        physical_inputs.push(requirement("resource_gem", 4, None));
    }
    let tool = match target_level {
        1 => None,
        2..=4 => Some((QualityBand::Crude, 1)),
        5..=6 => Some((QualityBand::Common, 1)),
        7..=8 => Some((QualityBand::Fine, 2)),
        9 => Some((QualityBand::Superior, 2)),
        10 => Some((QualityBand::Masterwork, 3)),
        _ => return Err(HoleError::MalformedState),
    };
    if let Some((quality, quantity)) = tool {
        physical_inputs.push(requirement("item_generic_tool", quantity, Some(quality)));
    }
    Ok(UpgradeBill {
        axis,
        target_level,
        physical_inputs,
    })
}

fn requirement(
    content_id: &str,
    quantity: u32,
    minimum_quality: Option<QualityBand>,
) -> PhysicalRequirement {
    PhysicalRequirement {
        content_id: ContentId::new(content_id).expect("Plan-owned content ID is stable"),
        quantity,
        minimum_quality,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TilePointWire {
    x: i32,
    y: i32,
}

/// `TilePoint` is consumed from the spatial authority, whose general-purpose
/// wire permits additive fields. Hole persistence is fail-closed instead.
mod tile_point_serde {
    use super::*;

    pub fn serialize<S>(tile: &TilePoint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        TilePointWire {
            x: tile.x,
            y: tile.y,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TilePoint, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TilePointWire::deserialize(deserializer)?;
        Ok(TilePoint {
            x: wire.x,
            y: wire.y,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BlackHoleStateWire {
    schema_version: u32,
    hole_id: String,
    anchor: TilePointWire,
    axes: HoleAxes,
    next_opening_game_minute: u64,
    next_opening_index: u64,
    micro_void_balance: u64,
    active_feed: Option<FeedOrder>,
    active_upgrade: Option<AxisUpgradeProject>,
    credits: Vec<OpeningCredit>,
    terminal_entries: Vec<FeedEntry>,
    terminal_upgrade_recoveries: Vec<RecoveredAxisUpgrade>,
    completed_upgrades: Vec<CompletedAxisUpgrade>,
    command_receipts: Vec<CommandReceipt>,
    recovery_receipts: Vec<RecoveryReceipt>,
    next_command_sequence: u64,
    next_recovery_sequence: u64,
}

impl Serialize for BlackHoleState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BlackHoleStateWire {
            schema_version: BLACK_HOLE_SCHEMA_VERSION,
            hole_id: self.hole_id.clone(),
            anchor: TilePointWire {
                x: self.anchor.x,
                y: self.anchor.y,
            },
            axes: self.axes,
            next_opening_game_minute: self.next_opening_game_minute,
            next_opening_index: self.next_opening_index,
            micro_void_balance: self.micro_void_balance,
            active_feed: self.active_feed.clone(),
            active_upgrade: self.active_upgrade.clone(),
            credits: self.credits.clone(),
            terminal_entries: self.terminal_entries.clone(),
            terminal_upgrade_recoveries: self.terminal_upgrade_recoveries.clone(),
            completed_upgrades: self.completed_upgrades.clone(),
            command_receipts: self.command_receipts.values().cloned().collect(),
            recovery_receipts: self.recovery_receipts.values().cloned().collect(),
            next_command_sequence: self.next_command_sequence,
            next_recovery_sequence: self.next_recovery_sequence,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlackHoleState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BlackHoleStateWire::deserialize(deserializer)?;
        if wire.schema_version != BLACK_HOLE_SCHEMA_VERSION {
            return Err(de::Error::custom(HoleError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        let mut command_receipts = BTreeMap::new();
        for receipt in wire.command_receipts {
            if command_receipts
                .insert(receipt.id.clone(), receipt)
                .is_some()
            {
                return Err(de::Error::custom(HoleError::MalformedState));
            }
        }
        let mut recovery_receipts = BTreeMap::new();
        for receipt in wire.recovery_receipts {
            if recovery_receipts
                .insert(receipt.id.clone(), receipt)
                .is_some()
            {
                return Err(de::Error::custom(HoleError::MalformedState));
            }
        }
        let state = Self {
            hole_id: wire.hole_id,
            anchor: TilePoint {
                x: wire.anchor.x,
                y: wire.anchor.y,
            },
            axes: wire.axes,
            next_opening_game_minute: wire.next_opening_game_minute,
            next_opening_index: wire.next_opening_index,
            micro_void_balance: wire.micro_void_balance,
            active_feed: wire.active_feed,
            active_upgrade: wire.active_upgrade,
            credits: wire.credits,
            terminal_entries: wire.terminal_entries,
            terminal_upgrade_recoveries: wire.terminal_upgrade_recoveries,
            completed_upgrades: wire.completed_upgrades,
            command_receipts,
            recovery_receipts,
            next_command_sequence: wire.next_command_sequence,
            next_recovery_sequence: wire.next_recovery_sequence,
        };
        state.validate().map_err(de::Error::custom)?;
        Ok(state)
    }
}
