//! LAI.60-A canonical physical storage authority.
//!
//! This module deliberately keeps `QualityLotLedger` as the only owner of bulk
//! quantities and exact item payloads.  The surrounding maps are physical
//! placement, reservation, and replay indexes; they never shadow an aggregate
//! inventory.  World-tick adapters are intentionally deferred to LAI.63.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{MaterialInstanceId, PhysicalLotId},
    physical_storage::{ContainerKind, StorageCompatibility, VISIBLE_SLOTS_PER_STORAGE_TILE},
    quality_lots::{ItemInstance, LotLocation, PhysicalLot, QualityLotError, QualityLotLedger},
    spatial_tasks::{TaskFootprint, TilePoint},
};

pub const STORAGE_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_STORAGE_ZONES: usize = 1_024;
pub const MAX_STORAGE_CONTAINERS: usize = 4_096;
pub const MAX_STORAGE_RECEIPTS: usize = 1_024;
pub const MAX_ZONE_TILES: usize = 4_096;

pub type ColonyStorageId = String;
pub type StorageCommandId = String;
pub type StorageFingerprint = String;
pub type StorageZoneId = String;
pub type StorageContainerId = String;
pub type ConstructionProjectId = String;
pub type ReservationOwnerId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageAuthorityError {
    UnsupportedVersion(u32),
    EmptyStableId,
    TooManyZones,
    TooManyContainers,
    TooManyReceipts,
    InvalidFootprint,
    DuplicateZone(StorageZoneId),
    MissingZone(StorageZoneId),
    DuplicateContainer(StorageContainerId),
    MissingContainer(StorageContainerId),
    DuplicateIdentity(String),
    MissingIdentity(String),
    WrongColony,
    TileOutsideZone,
    InvalidSlot,
    SlotOccupied,
    SlotEmpty,
    ContainerNotAtAddress,
    ContainerFull,
    IncompatibleContainer,
    MixedContainerContent,
    Reserved(String),
    ReservationOwnerMismatch,
    RemovalBlocked,
    InvalidDestination,
    NoRecoveryDestination,
    ReplayConflict,
    ReceiptDrained,
    ArithmeticOverflow,
    Ledger(QualityLotError),
    Invariant(&'static str),
}

impl fmt::Display for StorageAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid canonical storage authority state: {self:?}")
    }
}

impl std::error::Error for StorageAuthorityError {}

impl From<QualityLotError> for StorageAuthorityError {
    fn from(value: QualityLotError) -> Self {
        Self::Ledger(value)
    }
}

fn stable(value: &str) -> Result<(), StorageAuthorityError> {
    if value.trim().is_empty() {
        Err(StorageAuthorityError::EmptyStableId)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageZoneKind {
    Stockpile,
    WorkshopInput,
    Cache,
    ConstructionApron,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageZone {
    pub id: StorageZoneId,
    pub kind: StorageZoneKind,
    pub footprint: TaskFootprint,
    pub tiles: BTreeMap<TilePoint, StorageTileSlots>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StorageZoneWire {
    id: StorageZoneId,
    kind: StorageZoneKind,
    footprint: TaskFootprint,
    tiles: Vec<StorageZoneTileWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StorageZoneTileWire {
    tile: TilePoint,
    slots: StorageTileSlots,
}

impl Serialize for StorageZone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        StorageZoneWire {
            id: self.id.clone(),
            kind: self.kind,
            footprint: self.footprint.clone(),
            tiles: self
                .tiles
                .iter()
                .map(|(tile, slots)| StorageZoneTileWire {
                    tile: *tile,
                    slots: slots.clone(),
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StorageZone {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StorageZoneWire::deserialize(deserializer)?;
        let expected_tile_count = wire.tiles.len();
        let tiles = wire
            .tiles
            .into_iter()
            .map(|entry| (entry.tile, entry.slots))
            .collect::<BTreeMap<_, _>>();
        if tiles.len() != expected_tile_count {
            return Err(de::Error::custom("duplicate storage-zone tile"));
        }
        let zone = Self {
            id: wire.id,
            kind: wire.kind,
            footprint: wire.footprint,
            tiles,
        };
        zone.validate().map_err(de::Error::custom)?;
        Ok(zone)
    }
}

impl StorageZone {
    pub fn new(
        id: impl Into<String>,
        kind: StorageZoneKind,
        footprint: TaskFootprint,
    ) -> Result<Self, StorageAuthorityError> {
        let id = id.into();
        stable(&id)?;
        footprint
            .validate()
            .map_err(|_| StorageAuthorityError::InvalidFootprint)?;
        if footprint.tiles.as_slice().len() > MAX_ZONE_TILES {
            return Err(StorageAuthorityError::InvalidFootprint);
        }
        let tiles = footprint
            .tiles
            .as_slice()
            .iter()
            .copied()
            .map(|tile| (tile, StorageTileSlots::default()))
            .collect();
        Ok(Self {
            id,
            kind,
            footprint,
            tiles,
        })
    }

    fn validate(&self) -> Result<(), StorageAuthorityError> {
        stable(&self.id)?;
        self.footprint
            .validate()
            .map_err(|_| StorageAuthorityError::InvalidFootprint)?;
        let expected = self.footprint.tiles.as_slice();
        if expected.len() > MAX_ZONE_TILES
            || self.tiles.len() != expected.len()
            || expected.iter().any(|tile| !self.tiles.contains_key(tile))
        {
            return Err(StorageAuthorityError::InvalidFootprint);
        }
        self.tiles.values().try_for_each(StorageTileSlots::validate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageTileSlots {
    pub slots: BTreeMap<u8, VisibleStorageSlot>,
}

impl StorageTileSlots {
    fn validate(&self) -> Result<(), StorageAuthorityError> {
        if self.slots.len() > VISIBLE_SLOTS_PER_STORAGE_TILE
            || self
                .slots
                .keys()
                .any(|slot| usize::from(*slot) >= VISIBLE_SLOTS_PER_STORAGE_TILE)
        {
            return Err(StorageAuthorityError::InvalidSlot);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleStorageSlot {
    Loose(StorageIdentity),
    Container(StorageContainerId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StorageIdentity {
    Lot(PhysicalLotId),
    Item(MaterialInstanceId),
}

impl StorageIdentity {
    fn text(&self) -> String {
        match self {
            Self::Lot(id) => format!("lot:{id}"),
            Self::Item(id) => format!("item:{id}"),
        }
    }
}

// JSON object keys must be strings. Keep the same explicit, collision-free
// identity prefix everywhere so locations, reservations, and metadata can
// round-trip as canonical BTreeMap keys without inventing a shadow numeric ID.
impl Serialize for StorageIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.text())
    }
}

impl<'de> Deserialize<'de> for StorageIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(id) = value.strip_prefix("lot:") {
            return PhysicalLotId::new(id)
                .map(Self::Lot)
                .map_err(de::Error::custom);
        }
        if let Some(id) = value.strip_prefix("item:") {
            return MaterialInstanceId::new(id)
                .map(Self::Item)
                .map_err(de::Error::custom);
        }
        Err(de::Error::custom("invalid storage identity prefix"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageContainer {
    pub id: StorageContainerId,
    pub kind: ContainerKind,
    pub zone_id: StorageZoneId,
    pub tile: TilePoint,
    pub slot: u8,
    pub contents: BTreeSet<StorageIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageAddress {
    Loose {
        zone_id: StorageZoneId,
        tile: TilePoint,
        slot: u8,
    },
    Container {
        container_id: StorageContainerId,
    },
    ConstructionCargo {
        project_id: ConstructionProjectId,
    },
    RouteCargo {
        route_id: String,
    },
    /// Purpose-bound divine cargo placed at a real world site (currently the
    /// Hole delivery apron). It is intentionally unbounded by storage slots:
    /// rescue supplies have no stock cap, but retain exact identity, location,
    /// provenance, and reservation until a physical hauling task consumes them.
    PurposeCargo {
        site_id: String,
    },
    /// A visible one-tile cache slot. It must refer to a registered `Cache`
    /// zone, so recovery cannot fabricate an invisible fallback inventory.
    LandCache {
        zone_id: StorageZoneId,
        tile: TilePoint,
        slot: u8,
    },
}

impl StorageAddress {
    fn canonical_location(&self) -> LotLocation {
        match self {
            Self::Loose { zone_id, .. }
            | Self::Container {
                container_id: zone_id,
            } => LotLocation::Stockpile(zone_id.clone()),
            Self::ConstructionCargo { project_id } => {
                LotLocation::Cargo(format!("construction:{project_id}"))
            }
            Self::RouteCargo { route_id } => LotLocation::Cargo(format!("route:{route_id}")),
            Self::PurposeCargo { site_id } => LotLocation::Source(site_id.clone()),
            Self::LandCache { zone_id, .. } => LotLocation::Cache(zone_id.clone()),
        }
    }

    fn validate(&self) -> Result<(), StorageAuthorityError> {
        match self {
            Self::Loose { zone_id, slot, .. } => {
                stable(zone_id)?;
                if usize::from(*slot) >= VISIBLE_SLOTS_PER_STORAGE_TILE {
                    Err(StorageAuthorityError::InvalidSlot)
                } else {
                    Ok(())
                }
            }
            Self::Container { container_id } => stable(container_id),
            Self::ConstructionCargo { project_id } => stable(project_id),
            Self::RouteCargo { route_id } => stable(route_id),
            Self::PurposeCargo { site_id } => stable(site_id),
            Self::LandCache { zone_id, slot, .. } => {
                stable(zone_id)?;
                if usize::from(*slot) >= VISIBLE_SLOTS_PER_STORAGE_TILE {
                    Err(StorageAuthorityError::InvalidSlot)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityMetadata {
    pub compatibility: StorageCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkshopStorageLink {
    pub workshop_id: String,
    pub workshop_footprint: TaskFootprint,
    pub zone_id: StorageZoneId,
}

impl WorkshopStorageLink {
    pub fn validate(&self, zone: &StorageZone) -> Result<(), StorageAuthorityError> {
        stable(&self.workshop_id)?;
        self.workshop_footprint
            .validate()
            .map_err(|_| StorageAuthorityError::InvalidFootprint)?;
        if self.workshop_footprint.width != 3
            || self.workshop_footprint.height != 3
            || self.workshop_footprint.tiles.as_slice().len() != 9
        {
            return Err(StorageAuthorityError::InvalidFootprint);
        }
        if self.zone_id != zone.id || zone.kind != StorageZoneKind::WorkshopInput {
            return Err(StorageAuthorityError::InvalidDestination);
        }
        let work = self
            .workshop_footprint
            .tiles
            .as_slice()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let storage = zone
            .footprint
            .tiles
            .as_slice()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if !work.is_disjoint(&storage) {
            return Err(StorageAuthorityError::InvalidDestination);
        }
        if !work.iter().any(|left| {
            storage
                .iter()
                .any(|right| left.x.abs_diff(right.x) + left.y.abs_diff(right.y) == 1)
        }) {
            return Err(StorageAuthorityError::InvalidDestination);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageCommandReceipt {
    pub command_id: StorageCommandId,
    pub fingerprint: StorageFingerprint,
    pub sequence: u64,
    pub version_after: u64,
    pub result: StorageCommandResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCommandResult {
    Applied,
    Recovered { destination: StorageAddress },
    Consumed { bulk_units: u64, items: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageCommandEnvelope {
    pub colony_id: ColonyStorageId,
    pub command_id: StorageCommandId,
    pub fingerprint: StorageFingerprint,
    pub sequence: u64,
    pub command: StorageCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCommand {
    RegisterZone {
        zone: StorageZone,
    },
    RemoveZone {
        zone_id: StorageZoneId,
    },
    RegisterContainer {
        container: StorageContainer,
    },
    LinkWorkshop {
        link: WorkshopStorageLink,
    },
    RemoveContainer {
        container_id: StorageContainerId,
    },
    DepositLot {
        lot: PhysicalLot,
        compatibility: StorageCompatibility,
        destination: StorageAddress,
    },
    DepositItem {
        item: ItemInstance,
        compatibility: StorageCompatibility,
        destination: StorageAddress,
    },
    Move {
        identity: StorageIdentity,
        destination: StorageAddress,
    },
    Reserve {
        identity: StorageIdentity,
        owner: ReservationOwnerId,
    },
    Unreserve {
        identity: StorageIdentity,
        owner: ReservationOwnerId,
    },
    Consume {
        bulk: Vec<(PhysicalLotId, u32)>,
        items: Vec<MaterialInstanceId>,
    },
    SplitBulk {
        source: PhysicalLotId,
        split: PhysicalLotId,
        units: u32,
        destination: StorageAddress,
    },
    MergeBulk {
        left: PhysicalLotId,
        right: PhysicalLotId,
    },
    StageConstruction {
        project_id: ConstructionProjectId,
        identities: Vec<StorageIdentity>,
    },
    Recover {
        identities: Vec<StorageIdentity>,
        origin: Option<StorageAddress>,
        stockpile: Option<StorageAddress>,
        cache: Option<StorageAddress>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageAuthority {
    colony_id: ColonyStorageId,
    version: u64,
    ledger: QualityLotLedger,
    zones: BTreeMap<StorageZoneId, StorageZone>,
    containers: BTreeMap<StorageContainerId, StorageContainer>,
    locations: BTreeMap<StorageIdentity, StorageAddress>,
    reservations: BTreeMap<StorageIdentity, ReservationOwnerId>,
    metadata: BTreeMap<StorageIdentity, IdentityMetadata>,
    workshop_links: BTreeMap<String, WorkshopStorageLink>,
    receipts: BTreeMap<StorageCommandId, StorageCommandReceipt>,
    replay_watermark: u64,
}

impl StorageAuthority {
    pub fn new(colony_id: impl Into<String>) -> Result<Self, StorageAuthorityError> {
        let colony_id = colony_id.into();
        stable(&colony_id)?;
        Ok(Self {
            colony_id,
            version: 0,
            ledger: QualityLotLedger::new(Vec::new(), Vec::new())?,
            zones: BTreeMap::new(),
            containers: BTreeMap::new(),
            locations: BTreeMap::new(),
            reservations: BTreeMap::new(),
            metadata: BTreeMap::new(),
            workshop_links: BTreeMap::new(),
            receipts: BTreeMap::new(),
            replay_watermark: 0,
        })
    }

    pub fn version(&self) -> u64 {
        self.version
    }
    pub fn colony_id(&self) -> &str {
        &self.colony_id
    }
    pub fn ledger(&self) -> &QualityLotLedger {
        &self.ledger
    }

    /// Returns every visible storage zone in its canonical stable-ID order.
    ///
    /// The returned zone contains placement and visible-slot topology only;
    /// `QualityLotLedger` remains the sole authority for lot quantities and
    /// exact item payloads.  The `BTreeMap` and `MAX_STORAGE_ZONES` invariant
    /// make this both deterministic and bounded for report projection.
    pub fn report_zones(&self) -> impl ExactSizeIterator<Item = &StorageZone> {
        self.zones.values()
    }

    /// Returns every visible physical container in canonical stable-ID order.
    ///
    /// Container contents are stable physical identities, not copied stock
    /// quantities.  Callers that need an allowed quantity must resolve the
    /// identity through the existing `QualityLotLedger` authority.
    pub fn report_containers(&self) -> impl ExactSizeIterator<Item = &StorageContainer> {
        self.containers.values()
    }

    /// Returns the authored Workshop-to-input-zone links in stable Workshop-ID
    /// order.  Links are report-safe physical references; they do not expose a
    /// hidden station buffer or create a new placement authority.
    pub fn report_workshop_links(&self) -> impl ExactSizeIterator<Item = &WorkshopStorageLink> {
        self.workshop_links.values()
    }

    pub fn zone(&self, id: &str) -> Option<&StorageZone> {
        self.zones.get(id)
    }
    pub fn container(&self, id: &str) -> Option<&StorageContainer> {
        self.containers.get(id)
    }
    pub fn location(&self, identity: &StorageIdentity) -> Option<&StorageAddress> {
        self.locations.get(identity)
    }
    pub fn receipt(&self, command_id: &str) -> Option<&StorageCommandReceipt> {
        self.receipts.get(command_id)
    }
    pub fn receipts(&self) -> impl ExactSizeIterator<Item = &StorageCommandReceipt> {
        self.receipts.values()
    }
    pub fn replay_watermark(&self) -> u64 {
        self.replay_watermark
    }

    fn link_workshop(&mut self, link: WorkshopStorageLink) -> Result<(), StorageAuthorityError> {
        let zone = self
            .zones
            .get(&link.zone_id)
            .ok_or_else(|| StorageAuthorityError::MissingZone(link.zone_id.clone()))?;
        link.validate(zone)?;
        if self
            .workshop_links
            .insert(link.workshop_id.clone(), link)
            .is_some()
        {
            return Err(StorageAuthorityError::DuplicateIdentity(
                "workshop link".to_owned(),
            ));
        }
        self.validate()
    }

    pub fn execute(
        &mut self,
        envelope: StorageCommandEnvelope,
    ) -> Result<StorageCommandReceipt, StorageAuthorityError> {
        stable(&envelope.command_id)?;
        stable(&envelope.fingerprint)?;
        if envelope.colony_id != self.colony_id {
            return Err(StorageAuthorityError::WrongColony);
        }
        if let Some(receipt) = self.receipts.get(&envelope.command_id) {
            if receipt.fingerprint == envelope.fingerprint {
                return Ok(receipt.clone());
            }
            return Err(StorageAuthorityError::ReplayConflict);
        }
        if envelope.sequence <= self.replay_watermark {
            return Err(StorageAuthorityError::ReceiptDrained);
        }
        if self.receipts.len() >= MAX_STORAGE_RECEIPTS {
            return Err(StorageAuthorityError::TooManyReceipts);
        }
        let mut staged = self.clone();
        let result = staged.apply(envelope.command)?;
        staged.version = staged
            .version
            .checked_add(1)
            .ok_or(StorageAuthorityError::ArithmeticOverflow)?;
        staged.validate()?;
        let receipt = StorageCommandReceipt {
            command_id: envelope.command_id.clone(),
            fingerprint: envelope.fingerprint,
            sequence: envelope.sequence,
            version_after: staged.version,
            result,
        };
        staged.receipts.insert(envelope.command_id, receipt.clone());
        *self = staged;
        Ok(receipt)
    }

    /// Drops completed replay data through `sequence`, while retaining a monotonic
    /// watermark so an old command cannot be re-applied after compaction.
    pub fn drain_terminal_receipts_through(&mut self, sequence: u64) {
        let highest = self
            .receipts
            .values()
            .filter(|receipt| receipt.sequence <= sequence)
            .map(|receipt| receipt.sequence)
            .max();
        self.receipts
            .retain(|_, receipt| receipt.sequence > sequence);
        if let Some(highest) = highest {
            self.replay_watermark = self.replay_watermark.max(highest);
        }
    }

    pub fn fullness(
        &self,
        zone_id: &str,
        tile: TilePoint,
    ) -> Result<(usize, usize), StorageAuthorityError> {
        let zone = self
            .zones
            .get(zone_id)
            .ok_or_else(|| StorageAuthorityError::MissingZone(zone_id.to_owned()))?;
        let tile = zone
            .tiles
            .get(&tile)
            .ok_or(StorageAuthorityError::TileOutsideZone)?;
        Ok((tile.slots.len(), VISIBLE_SLOTS_PER_STORAGE_TILE))
    }

    pub fn compatible_kinds(
        &self,
        container_id: &str,
    ) -> Result<Vec<StorageCompatibility>, StorageAuthorityError> {
        let container = self
            .containers
            .get(container_id)
            .ok_or_else(|| StorageAuthorityError::MissingContainer(container_id.to_owned()))?;
        Ok(StorageCompatibility::all()
            .into_iter()
            .filter(|kind| container.kind.accepts(*kind))
            .collect())
    }

    fn apply(
        &mut self,
        command: StorageCommand,
    ) -> Result<StorageCommandResult, StorageAuthorityError> {
        match command {
            StorageCommand::RegisterZone { zone } => {
                self.register_zone(zone)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::RemoveZone { zone_id } => {
                self.remove_zone(&zone_id)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::RegisterContainer { container } => {
                self.register_container(container)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::LinkWorkshop { link } => {
                self.link_workshop(link)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::RemoveContainer { container_id } => {
                self.remove_container(&container_id)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::DepositLot {
                lot,
                compatibility,
                destination,
            } => {
                self.deposit_lot(lot, compatibility, destination)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::DepositItem {
                item,
                compatibility,
                destination,
            } => {
                self.deposit_item(item, compatibility, destination)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::Move {
                identity,
                destination,
            } => {
                self.move_identity(&identity, destination)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::Reserve { identity, owner } => {
                self.reserve(&identity, &owner)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::Unreserve { identity, owner } => {
                self.unreserve(&identity, &owner)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::Consume { bulk, items } => self.consume(bulk, items),
            StorageCommand::SplitBulk {
                source,
                split,
                units,
                destination,
            } => {
                self.split_bulk(&source, split, units, destination)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::MergeBulk { left, right } => {
                self.merge_bulk(&left, &right)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::StageConstruction {
                project_id,
                identities,
            } => {
                self.stage_construction(&project_id, identities)?;
                Ok(StorageCommandResult::Applied)
            }
            StorageCommand::Recover {
                identities,
                origin,
                stockpile,
                cache,
            } => self.recover(identities, origin, stockpile, cache),
        }
    }

    fn register_zone(&mut self, zone: StorageZone) -> Result<(), StorageAuthorityError> {
        zone.validate()?;
        if self.zones.len() >= MAX_STORAGE_ZONES {
            return Err(StorageAuthorityError::TooManyZones);
        }
        if self.zones.insert(zone.id.clone(), zone).is_some() {
            return Err(StorageAuthorityError::DuplicateZone("duplicate".to_owned()));
        }
        Ok(())
    }

    fn remove_zone(&mut self, id: &str) -> Result<(), StorageAuthorityError> {
        let zone = self
            .zones
            .get(id)
            .ok_or_else(|| StorageAuthorityError::MissingZone(id.to_owned()))?;
        if zone.tiles.values().any(|tile| !tile.slots.is_empty())
            || self.workshop_links.values().any(|link| link.zone_id == id)
        {
            return Err(StorageAuthorityError::RemovalBlocked);
        }
        self.zones.remove(id);
        Ok(())
    }

    fn register_container(
        &mut self,
        container: StorageContainer,
    ) -> Result<(), StorageAuthorityError> {
        stable(&container.id)?;
        stable(&container.zone_id)?;
        if self.containers.len() >= MAX_STORAGE_CONTAINERS {
            return Err(StorageAuthorityError::TooManyContainers);
        }
        if !container.contents.is_empty() {
            return Err(StorageAuthorityError::RemovalBlocked);
        }
        self.ensure_empty_slot(&container.zone_id, container.tile, container.slot)?;
        if self
            .containers
            .insert(container.id.clone(), container.clone())
            .is_some()
        {
            return Err(StorageAuthorityError::DuplicateContainer(container.id));
        }
        self.slot_mut(&container.zone_id, container.tile, container.slot)?
            .slots
            .insert(container.slot, VisibleStorageSlot::Container(container.id));
        Ok(())
    }

    fn remove_container(&mut self, id: &str) -> Result<(), StorageAuthorityError> {
        let container = self
            .containers
            .get(id)
            .ok_or_else(|| StorageAuthorityError::MissingContainer(id.to_owned()))?
            .clone();
        if !container.contents.is_empty() {
            return Err(StorageAuthorityError::RemovalBlocked);
        }
        self.tile_mut(&container.zone_id, container.tile)?
            .slots
            .remove(&container.slot);
        self.containers.remove(id);
        Ok(())
    }

    fn deposit_lot(
        &mut self,
        mut lot: PhysicalLot,
        compatibility: StorageCompatibility,
        destination: StorageAddress,
    ) -> Result<(), StorageAuthorityError> {
        let identity = StorageIdentity::Lot(lot.id.clone());
        self.ensure_absent(&identity)?;
        lot.location = destination.canonical_location();
        // The ledger is installed before physical compatibility checking so a
        // same-kind container can inspect the real content ID. `execute` uses
        // a clone, so a rejected command cannot leak this staged insertion.
        self.ledger.insert_lot(lot)?;
        self.place_identity(&identity, compatibility, &destination)?;
        self.locations.insert(identity.clone(), destination);
        self.metadata
            .insert(identity, IdentityMetadata { compatibility });
        Ok(())
    }

    fn deposit_item(
        &mut self,
        mut item: ItemInstance,
        compatibility: StorageCompatibility,
        destination: StorageAddress,
    ) -> Result<(), StorageAuthorityError> {
        let identity = StorageIdentity::Item(item.id.clone());
        self.ensure_absent(&identity)?;
        item.location = destination.canonical_location();
        self.ledger.insert_item(item)?;
        self.place_identity(&identity, compatibility, &destination)?;
        self.locations.insert(identity.clone(), destination);
        self.metadata
            .insert(identity, IdentityMetadata { compatibility });
        Ok(())
    }

    fn move_identity(
        &mut self,
        identity: &StorageIdentity,
        destination: StorageAddress,
    ) -> Result<(), StorageAuthorityError> {
        self.ensure_present(identity)?;
        if self.reservations.contains_key(identity) {
            return Err(StorageAuthorityError::Reserved(identity.text()));
        }
        let meta = self
            .metadata
            .get(identity)
            .ok_or(StorageAuthorityError::Invariant(
                "identity metadata missing",
            ))?
            .compatibility;
        self.remove_placement(identity)?;
        self.place_identity(identity, meta, &destination)?;
        self.set_ledger_location(identity, destination.canonical_location())?;
        self.locations.insert(identity.clone(), destination);
        Ok(())
    }

    fn reserve(
        &mut self,
        identity: &StorageIdentity,
        owner: &str,
    ) -> Result<(), StorageAuthorityError> {
        self.ensure_present(identity)?;
        stable(owner)?;
        if self.reservations.contains_key(identity) {
            return Err(StorageAuthorityError::Reserved(identity.text()));
        }
        self.set_ledger_reservation(identity, Some(owner.to_owned()))?;
        self.reservations.insert(identity.clone(), owner.to_owned());
        Ok(())
    }

    fn unreserve(
        &mut self,
        identity: &StorageIdentity,
        owner: &str,
    ) -> Result<(), StorageAuthorityError> {
        self.ensure_present(identity)?;
        if self
            .reservations
            .get(identity)
            .is_none_or(|existing| existing != owner)
        {
            return Err(StorageAuthorityError::ReservationOwnerMismatch);
        }
        self.set_ledger_reservation(identity, None)?;
        self.reservations.remove(identity);
        Ok(())
    }

    fn consume(
        &mut self,
        bulk: Vec<(PhysicalLotId, u32)>,
        items: Vec<MaterialInstanceId>,
    ) -> Result<StorageCommandResult, StorageAuthorityError> {
        let mut seen = BTreeSet::new();
        let mut bulk_units = 0_u64;
        for (id, units) in &bulk {
            let identity = StorageIdentity::Lot(id.clone());
            self.ensure_present(&identity)?;
            if self.reservations.contains_key(&identity) || !seen.insert(identity) || *units == 0 {
                return Err(StorageAuthorityError::Reserved(id.to_string()));
            }
            bulk_units = bulk_units
                .checked_add(u64::from(*units))
                .ok_or(StorageAuthorityError::ArithmeticOverflow)?;
        }
        for id in &items {
            let identity = StorageIdentity::Item(id.clone());
            self.ensure_present(&identity)?;
            if self.reservations.contains_key(&identity) || !seen.insert(identity) {
                return Err(StorageAuthorityError::Reserved(id.to_string()));
            }
        }
        for (id, units) in &bulk {
            self.ledger.debit_lot(id, *units)?;
            if self.ledger.lot(id).is_none() {
                self.remove_placement(&StorageIdentity::Lot(id.clone()))?;
                self.locations.remove(&StorageIdentity::Lot(id.clone()));
                self.metadata.remove(&StorageIdentity::Lot(id.clone()));
            }
        }
        for id in &items {
            self.ledger.remove_item(id)?;
            self.remove_placement(&StorageIdentity::Item(id.clone()))?;
            self.locations.remove(&StorageIdentity::Item(id.clone()));
            self.metadata.remove(&StorageIdentity::Item(id.clone()));
        }
        Ok(StorageCommandResult::Consumed {
            bulk_units,
            items: items.len(),
        })
    }

    fn split_bulk(
        &mut self,
        source: &PhysicalLotId,
        split: PhysicalLotId,
        units: u32,
        destination: StorageAddress,
    ) -> Result<(), StorageAuthorityError> {
        let source_identity = StorageIdentity::Lot(source.clone());
        self.ensure_present(&source_identity)?;
        if self.reservations.contains_key(&source_identity) {
            return Err(StorageAuthorityError::Reserved(source.to_string()));
        }
        let compatibility = self
            .metadata
            .get(&source_identity)
            .ok_or(StorageAuthorityError::Invariant("metadata missing"))?
            .compatibility;
        let source_location = self
            .locations
            .get(&source_identity)
            .cloned()
            .ok_or(StorageAuthorityError::Invariant("location missing"))?;
        self.ledger.split_lot(source, split.clone(), units)?;
        let split_identity = StorageIdentity::Lot(split);
        self.place_identity(&split_identity, compatibility, &destination)?;
        self.set_ledger_location(&split_identity, destination.canonical_location())?;
        self.locations.insert(split_identity.clone(), destination);
        self.metadata
            .insert(split_identity, IdentityMetadata { compatibility });
        self.locations.insert(source_identity, source_location);
        Ok(())
    }

    fn merge_bulk(
        &mut self,
        left: &PhysicalLotId,
        right: &PhysicalLotId,
    ) -> Result<(), StorageAuthorityError> {
        let left_identity = StorageIdentity::Lot(left.clone());
        let right_identity = StorageIdentity::Lot(right.clone());
        self.ensure_present(&left_identity)?;
        self.ensure_present(&right_identity)?;
        if self.locations.get(&left_identity) != self.locations.get(&right_identity)
            || self.metadata.get(&left_identity) != self.metadata.get(&right_identity)
        {
            return Err(StorageAuthorityError::InvalidDestination);
        }
        self.ledger.merge_lots(left, right)?;
        self.remove_placement(&right_identity)?;
        self.locations.remove(&right_identity);
        self.metadata.remove(&right_identity);
        Ok(())
    }

    fn stage_construction(
        &mut self,
        project_id: &str,
        identities: Vec<StorageIdentity>,
    ) -> Result<(), StorageAuthorityError> {
        stable(project_id)?;
        let mut seen = BTreeSet::new();
        for identity in &identities {
            self.ensure_present(identity)?;
            if !seen.insert(identity.clone()) || self.reservations.contains_key(identity) {
                return Err(StorageAuthorityError::Reserved(identity.text()));
            }
        }
        let destination = StorageAddress::ConstructionCargo {
            project_id: project_id.to_owned(),
        };
        for identity in identities {
            self.reserve(&identity, project_id)?;
            self.remove_placement(&identity)?;
            self.set_ledger_location(&identity, destination.canonical_location())?;
            self.locations.insert(identity, destination.clone());
        }
        Ok(())
    }

    fn recover(
        &mut self,
        identities: Vec<StorageIdentity>,
        origin: Option<StorageAddress>,
        stockpile: Option<StorageAddress>,
        cache: Option<StorageAddress>,
    ) -> Result<StorageCommandResult, StorageAuthorityError> {
        if identities.is_empty() {
            return Err(StorageAuthorityError::MissingIdentity(
                "empty recovery".to_owned(),
            ));
        }
        let mut seen = BTreeSet::new();
        for identity in &identities {
            self.ensure_present(identity)?;
            if !seen.insert(identity.clone()) {
                return Err(StorageAuthorityError::DuplicateIdentity(identity.text()));
            }
        }
        let candidates = [origin, stockpile, cache];
        let Some(destination) = candidates
            .into_iter()
            .flatten()
            .find(|candidate| self.can_place_all(&identities, candidate))
        else {
            return Err(StorageAuthorityError::NoRecoveryDestination);
        };
        for identity in identities {
            if self.is_placed(&identity) {
                self.remove_placement(&identity)?;
            }
            self.set_ledger_reservation(&identity, None)?;
            self.reservations.remove(&identity);
            let compatibility = self
                .metadata
                .get(&identity)
                .ok_or(StorageAuthorityError::Invariant("metadata missing"))?
                .compatibility;
            self.place_identity(&identity, compatibility, &destination)?;
            self.set_ledger_location(&identity, destination.canonical_location())?;
            self.locations.insert(identity, destination.clone());
        }
        Ok(StorageCommandResult::Recovered { destination })
    }

    fn ensure_absent(&self, identity: &StorageIdentity) -> Result<(), StorageAuthorityError> {
        if self.locations.contains_key(identity) {
            Err(StorageAuthorityError::DuplicateIdentity(identity.text()))
        } else {
            Ok(())
        }
    }
    fn ensure_present(&self, identity: &StorageIdentity) -> Result<(), StorageAuthorityError> {
        if self.locations.contains_key(identity) {
            Ok(())
        } else {
            Err(StorageAuthorityError::MissingIdentity(identity.text()))
        }
    }
    fn is_placed(&self, identity: &StorageIdentity) -> bool {
        matches!(
            self.locations.get(identity),
            Some(
                StorageAddress::Loose { .. }
                    | StorageAddress::Container { .. }
                    | StorageAddress::PurposeCargo { .. }
                    | StorageAddress::LandCache { .. }
            )
        )
    }

    fn tile_mut(
        &mut self,
        zone: &str,
        tile: TilePoint,
    ) -> Result<&mut StorageTileSlots, StorageAuthorityError> {
        self.zones
            .get_mut(zone)
            .ok_or_else(|| StorageAuthorityError::MissingZone(zone.to_owned()))?
            .tiles
            .get_mut(&tile)
            .ok_or(StorageAuthorityError::TileOutsideZone)
    }
    fn slot_mut(
        &mut self,
        zone: &str,
        tile: TilePoint,
        _slot: u8,
    ) -> Result<&mut StorageTileSlots, StorageAuthorityError> {
        self.tile_mut(zone, tile)
    }
    fn ensure_empty_slot(
        &self,
        zone: &str,
        tile: TilePoint,
        slot: u8,
    ) -> Result<(), StorageAuthorityError> {
        if usize::from(slot) >= VISIBLE_SLOTS_PER_STORAGE_TILE {
            return Err(StorageAuthorityError::InvalidSlot);
        }
        let slots = self
            .zones
            .get(zone)
            .ok_or_else(|| StorageAuthorityError::MissingZone(zone.to_owned()))?
            .tiles
            .get(&tile)
            .ok_or(StorageAuthorityError::TileOutsideZone)?;
        if slots.slots.contains_key(&slot) {
            Err(StorageAuthorityError::SlotOccupied)
        } else {
            Ok(())
        }
    }

    fn place_identity(
        &mut self,
        identity: &StorageIdentity,
        compatibility: StorageCompatibility,
        destination: &StorageAddress,
    ) -> Result<(), StorageAuthorityError> {
        destination.validate()?;
        match destination {
            StorageAddress::Loose {
                zone_id,
                tile,
                slot,
            } => {
                self.ensure_empty_slot(zone_id, *tile, *slot)?;
                self.tile_mut(zone_id, *tile)?
                    .slots
                    .insert(*slot, VisibleStorageSlot::Loose(identity.clone()));
            }
            StorageAddress::Container { container_id } => {
                let container = self
                    .containers
                    .get(container_id)
                    .ok_or_else(|| StorageAuthorityError::MissingContainer(container_id.clone()))?;
                if !container.kind.accepts(compatibility) {
                    return Err(StorageAuthorityError::IncompatibleContainer);
                }
                if container.contents.len() >= container.kind.lot_capacity() {
                    return Err(StorageAuthorityError::ContainerFull);
                }
                if container.kind.requires_same_content() && !container.contents.is_empty() {
                    let existing = container
                        .contents
                        .iter()
                        .next()
                        .and_then(|identity| self.content_name(identity));
                    let candidate = self.content_name(identity);
                    if existing != candidate {
                        return Err(StorageAuthorityError::MixedContainerContent);
                    }
                }
                self.containers
                    .get_mut(container_id)
                    .expect("checked")
                    .contents
                    .insert(identity.clone());
            }
            StorageAddress::ConstructionCargo { .. }
            | StorageAddress::RouteCargo { .. }
            | StorageAddress::PurposeCargo { .. } => {}
            StorageAddress::LandCache {
                zone_id,
                tile,
                slot,
            } => {
                let zone = self
                    .zones
                    .get(zone_id)
                    .ok_or_else(|| StorageAuthorityError::MissingZone(zone_id.clone()))?;
                if zone.kind != StorageZoneKind::Cache {
                    return Err(StorageAuthorityError::InvalidDestination);
                }
                self.ensure_empty_slot(zone_id, *tile, *slot)?;
                self.tile_mut(zone_id, *tile)?
                    .slots
                    .insert(*slot, VisibleStorageSlot::Loose(identity.clone()));
            }
        }
        Ok(())
    }

    fn can_place_all(&self, identities: &[StorageIdentity], destination: &StorageAddress) -> bool {
        match destination {
            StorageAddress::Loose {
                zone_id,
                tile,
                slot,
            } => identities.len() == 1 && self.ensure_empty_slot(zone_id, *tile, *slot).is_ok(),
            StorageAddress::Container { container_id } => {
                self.containers.get(container_id).is_some_and(|container| {
                    identities.len()
                        <= container
                            .kind
                            .lot_capacity()
                            .saturating_sub(container.contents.len())
                        && identities.iter().all(|identity| {
                            self.metadata
                                .get(identity)
                                .is_some_and(|meta| container.kind.accepts(meta.compatibility))
                        })
                        && (!container.kind.requires_same_content()
                            || container.contents.is_empty()
                            || identities.iter().all(|identity| {
                                self.content_name(identity)
                                    == container
                                        .contents
                                        .iter()
                                        .next()
                                        .and_then(|existing| self.content_name(existing))
                            }))
                })
            }
            StorageAddress::ConstructionCargo { .. }
            | StorageAddress::RouteCargo { .. }
            | StorageAddress::PurposeCargo { .. } => true,
            StorageAddress::LandCache {
                zone_id,
                tile,
                slot,
            } => {
                self.zones
                    .get(zone_id)
                    .is_some_and(|zone| zone.kind == StorageZoneKind::Cache)
                    && identities.len() == 1
                    && self.ensure_empty_slot(zone_id, *tile, *slot).is_ok()
            }
        }
    }

    fn remove_placement(
        &mut self,
        identity: &StorageIdentity,
    ) -> Result<(), StorageAuthorityError> {
        match self
            .locations
            .get(identity)
            .cloned()
            .ok_or_else(|| StorageAuthorityError::MissingIdentity(identity.text()))?
        {
            StorageAddress::Loose {
                zone_id,
                tile,
                slot,
            } => {
                let actual = self
                    .tile_mut(&zone_id, tile)?
                    .slots
                    .remove(&slot)
                    .ok_or(StorageAuthorityError::SlotEmpty)?;
                if actual != VisibleStorageSlot::Loose(identity.clone()) {
                    return Err(StorageAuthorityError::Invariant(
                        "loose slot identity mismatch",
                    ));
                }
            }
            StorageAddress::Container { container_id } => {
                let container = self
                    .containers
                    .get_mut(&container_id)
                    .ok_or(StorageAuthorityError::MissingContainer(container_id))?;
                if !container.contents.remove(identity) {
                    return Err(StorageAuthorityError::Invariant(
                        "container identity missing",
                    ));
                }
            }
            StorageAddress::ConstructionCargo { .. }
            | StorageAddress::RouteCargo { .. }
            | StorageAddress::PurposeCargo { .. } => {}
            StorageAddress::LandCache {
                zone_id,
                tile,
                slot,
            } => {
                let actual = self
                    .tile_mut(&zone_id, tile)?
                    .slots
                    .remove(&slot)
                    .ok_or(StorageAuthorityError::SlotEmpty)?;
                if actual != VisibleStorageSlot::Loose(identity.clone()) {
                    return Err(StorageAuthorityError::Invariant(
                        "cache slot identity mismatch",
                    ));
                }
            }
        }
        Ok(())
    }

    fn content_name(&self, identity: &StorageIdentity) -> Option<String> {
        match identity {
            StorageIdentity::Lot(id) => self
                .ledger
                .lot(id)
                .map(|lot| lot.key.content_id.to_string()),
            StorageIdentity::Item(id) => self
                .ledger
                .item(id)
                .map(|item| item.definition_id.to_string()),
        }
    }

    fn set_ledger_location(
        &mut self,
        identity: &StorageIdentity,
        location: LotLocation,
    ) -> Result<(), StorageAuthorityError> {
        match identity {
            StorageIdentity::Lot(id) => self.ledger.move_lot(id, location)?,
            StorageIdentity::Item(id) => {
                let mut item = self.ledger.remove_item(id)?;
                item.location = location;
                self.ledger.insert_item(item)?;
            }
        }
        Ok(())
    }
    fn set_ledger_reservation(
        &mut self,
        identity: &StorageIdentity,
        reservation: Option<String>,
    ) -> Result<(), StorageAuthorityError> {
        // `QualityLotLedger` deliberately prevents debit of reserved cargo.
        // Reservation is metadata, not consumption, so rebuild the same ledger
        // identities after changing exactly one reservation field.
        let mut lots = self.ledger.lots().cloned().collect::<Vec<_>>();
        let mut items = self.ledger.items().cloned().collect::<Vec<_>>();
        match identity {
            StorageIdentity::Lot(id) => {
                lots.iter_mut()
                    .find(|lot| &lot.id == id)
                    .ok_or_else(|| StorageAuthorityError::MissingIdentity(identity.text()))?
                    .reservation = reservation
            }
            StorageIdentity::Item(id) => {
                items
                    .iter_mut()
                    .find(|item| &item.id == id)
                    .ok_or_else(|| StorageAuthorityError::MissingIdentity(identity.text()))?
                    .reservation = reservation
            }
        }
        self.ledger = QualityLotLedger::new(lots, items)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), StorageAuthorityError> {
        stable(&self.colony_id)?;
        if self.zones.len() > MAX_STORAGE_ZONES
            || self.containers.len() > MAX_STORAGE_CONTAINERS
            || self.receipts.len() > MAX_STORAGE_RECEIPTS
        {
            return Err(StorageAuthorityError::Invariant(
                "bounded collection exceeded",
            ));
        }
        self.zones.values().try_for_each(StorageZone::validate)?;
        for container in self.containers.values() {
            stable(&container.id)?;
            let zone = self
                .zones
                .get(&container.zone_id)
                .ok_or(StorageAuthorityError::Invariant("orphan container zone"))?;
            let slots = zone
                .tiles
                .get(&container.tile)
                .ok_or(StorageAuthorityError::Invariant("orphan container tile"))?;
            if slots.slots.get(&container.slot)
                != Some(&VisibleStorageSlot::Container(container.id.clone()))
            {
                return Err(StorageAuthorityError::Invariant("container slot mismatch"));
            }
            if container.contents.len() > container.kind.lot_capacity() {
                return Err(StorageAuthorityError::ContainerFull);
            }
            let mut content_names = BTreeSet::new();
            for identity in &container.contents {
                let metadata =
                    self.metadata
                        .get(identity)
                        .ok_or(StorageAuthorityError::Invariant(
                            "container identity metadata missing",
                        ))?;
                if !container.kind.accepts(metadata.compatibility) {
                    return Err(StorageAuthorityError::IncompatibleContainer);
                }
                content_names.insert(self.content_name(identity).ok_or(
                    StorageAuthorityError::Invariant("container identity missing from ledger"),
                )?);
            }
            if container.kind.requires_same_content() && content_names.len() > 1 {
                return Err(StorageAuthorityError::MixedContainerContent);
            }
        }
        for link in self.workshop_links.values() {
            link.validate(
                self.zones
                    .get(&link.zone_id)
                    .ok_or(StorageAuthorityError::Invariant("orphan workshop link"))?,
            )?;
        }
        let ledger_identities = self
            .ledger
            .lots()
            .map(|lot| StorageIdentity::Lot(lot.id.clone()))
            .chain(
                self.ledger
                    .items()
                    .map(|item| StorageIdentity::Item(item.id.clone())),
            )
            .collect::<BTreeSet<_>>();
        if ledger_identities != self.locations.keys().cloned().collect()
            || ledger_identities != self.metadata.keys().cloned().collect()
        {
            return Err(StorageAuthorityError::Invariant(
                "identity indexes disagree with ledger",
            ));
        }
        for (identity, address) in &self.locations {
            address.validate()?;
            let ledger_location = match identity {
                StorageIdentity::Lot(id) => self.ledger.lot(id).map(|lot| &lot.location),
                StorageIdentity::Item(id) => self.ledger.item(id).map(|item| &item.location),
            }
            .ok_or(StorageAuthorityError::Invariant(
                "indexed identity absent from ledger",
            ))?;
            if ledger_location != &address.canonical_location() {
                return Err(StorageAuthorityError::Invariant(
                    "ledger location differs from physical index",
                ));
            }
            let reservation = match identity {
                StorageIdentity::Lot(id) => {
                    self.ledger.lot(id).and_then(|lot| lot.reservation.as_ref())
                }
                StorageIdentity::Item(id) => self
                    .ledger
                    .item(id)
                    .and_then(|item| item.reservation.as_ref()),
            };
            if reservation != self.reservations.get(identity) {
                return Err(StorageAuthorityError::Invariant(
                    "reservation index differs from ledger",
                ));
            }
        }
        let mut placed = BTreeSet::new();
        for zone in self.zones.values() {
            for (tile_point, tile) in &zone.tiles {
                tile.validate()?;
                for (slot, visible) in &tile.slots {
                    match visible {
                        VisibleStorageSlot::Loose(identity) => {
                            let expected = if zone.kind == StorageZoneKind::Cache {
                                StorageAddress::LandCache {
                                    zone_id: zone.id.clone(),
                                    tile: *tile_point,
                                    slot: *slot,
                                }
                            } else {
                                StorageAddress::Loose {
                                    zone_id: zone.id.clone(),
                                    tile: *tile_point,
                                    slot: *slot,
                                }
                            };
                            if !placed.insert(identity.clone())
                                || self.locations.get(identity) != Some(&expected)
                            {
                                return Err(StorageAuthorityError::Invariant(
                                    "loose placement mismatch",
                                ));
                            }
                        }
                        VisibleStorageSlot::Container(id) => {
                            if !self.containers.contains_key(id) {
                                return Err(StorageAuthorityError::Invariant(
                                    "orphan container slot",
                                ));
                            }
                        }
                    }
                }
            }
        }
        for container in self.containers.values() {
            for identity in &container.contents {
                if !placed.insert(identity.clone())
                    || self.locations.get(identity)
                        != Some(&StorageAddress::Container {
                            container_id: container.id.clone(),
                        })
                {
                    return Err(StorageAuthorityError::Invariant(
                        "container placement mismatch",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn to_canonical_json(&self) -> Result<String, StorageAuthorityError> {
        self.validate()?;
        serde_json::to_string(self)
            .map_err(|_| StorageAuthorityError::Invariant("serialization failed"))
    }
    pub fn decode_strict(json: &str) -> Result<Self, StorageAuthorityError> {
        serde_json::from_str(json)
            .map_err(|_| StorageAuthorityError::Invariant("strict decode rejected"))
    }
}

impl StorageCompatibility {
    fn all() -> [Self; 10] {
        [
            Self::Food,
            Self::Herb,
            Self::Fibre,
            Self::Liquid,
            Self::BulkMaterial,
            Self::UniqueItem,
            Self::SmallItem,
            Self::Tool,
            Self::Weapon,
            Self::LongItem,
        ]
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StorageAuthorityWire {
    schema_version: u32,
    colony_id: ColonyStorageId,
    version: u64,
    ledger: QualityLotLedger,
    zones: BTreeMap<StorageZoneId, StorageZone>,
    containers: BTreeMap<StorageContainerId, StorageContainer>,
    locations: BTreeMap<StorageIdentity, StorageAddress>,
    reservations: BTreeMap<StorageIdentity, ReservationOwnerId>,
    metadata: BTreeMap<StorageIdentity, IdentityMetadata>,
    workshop_links: BTreeMap<String, WorkshopStorageLink>,
    receipts: BTreeMap<StorageCommandId, StorageCommandReceipt>,
    replay_watermark: u64,
}

impl Serialize for StorageAuthority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        StorageAuthorityWire {
            schema_version: STORAGE_AUTHORITY_SCHEMA_VERSION,
            colony_id: self.colony_id.clone(),
            version: self.version,
            ledger: self.ledger.clone(),
            zones: self.zones.clone(),
            containers: self.containers.clone(),
            locations: self.locations.clone(),
            reservations: self.reservations.clone(),
            metadata: self.metadata.clone(),
            workshop_links: self.workshop_links.clone(),
            receipts: self.receipts.clone(),
            replay_watermark: self.replay_watermark,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StorageAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = StorageAuthorityWire::deserialize(deserializer)?;
        if wire.schema_version != STORAGE_AUTHORITY_SCHEMA_VERSION {
            return Err(de::Error::custom(
                StorageAuthorityError::UnsupportedVersion(wire.schema_version),
            ));
        }
        let authority = Self {
            colony_id: wire.colony_id,
            version: wire.version,
            ledger: wire.ledger,
            zones: wire.zones,
            containers: wire.containers,
            locations: wire.locations,
            reservations: wire.reservations,
            metadata: wire.metadata,
            workshop_links: wire.workshop_links,
            receipts: wire.receipts,
            replay_watermark: wire.replay_watermark,
        };
        authority.validate().map_err(de::Error::custom)?;
        Ok(authority)
    }
}
