//! Visible storage tiles, typed containers, and preserved internal lots.
//!
//! The legacy scalar capacity system remains outside this additive leaf. This
//! module defines the deterministic physical contract used by the new Leader
//! planner, persistence, protocol, and world renderer.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::spatial_tasks::{TaskFootprint, TilePoint};

pub const CURRENT_PHYSICAL_STORAGE_VERSION: u32 = 1;
pub const VISIBLE_SLOTS_PER_STORAGE_TILE: usize = 4;

pub type StorageLotId = String;
pub type ContainerId = String;
pub type StorageZoneId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCompatibility {
    Food,
    Herb,
    Fibre,
    Liquid,
    BulkMaterial,
    UniqueItem,
    SmallItem,
    Tool,
    Weapon,
    LongItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageLot {
    pub lot_id: StorageLotId,
    pub content_id: String,
    pub compatibility: StorageCompatibility,
    pub units: u32,
    pub quality_band: u8,
    pub produced_at_ms: i64,
    pub expires_at_ms: Option<i64>,
    pub provenance_id: String,
    pub reserved_units: u32,
}

impl StorageLot {
    pub fn validate(&self) -> Result<(), PhysicalStorageError> {
        if self.lot_id.trim().is_empty() {
            return Err(PhysicalStorageError::EmptyStableId);
        }
        if self.content_id.trim().is_empty() || self.provenance_id.trim().is_empty() {
            return Err(PhysicalStorageError::EmptyContentIdentity);
        }
        if self.units == 0 || self.reserved_units > self.units {
            return Err(PhysicalStorageError::InvalidLotQuantity);
        }
        if self
            .expires_at_ms
            .is_some_and(|expires_at| expires_at < self.produced_at_ms)
        {
            return Err(PhysicalStorageError::ExpiryBeforeProduction);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    Basket,
    Barrel,
    Crate,
    Chest,
    Rack,
}

impl ContainerKind {
    #[must_use]
    pub const fn lot_capacity(self) -> usize {
        match self {
            Self::Basket => 4,
            Self::Barrel | Self::Crate | Self::Rack => 8,
            Self::Chest => 16,
        }
    }

    #[must_use]
    pub const fn requires_same_content(self) -> bool {
        matches!(self, Self::Barrel | Self::Crate)
    }

    #[must_use]
    pub const fn accepts(self, compatibility: StorageCompatibility) -> bool {
        match self {
            Self::Basket => matches!(
                compatibility,
                StorageCompatibility::Food
                    | StorageCompatibility::Herb
                    | StorageCompatibility::Fibre
            ),
            Self::Barrel => matches!(
                compatibility,
                StorageCompatibility::Liquid | StorageCompatibility::Food
            ),
            Self::Crate => matches!(compatibility, StorageCompatibility::BulkMaterial),
            Self::Chest => matches!(
                compatibility,
                StorageCompatibility::UniqueItem | StorageCompatibility::SmallItem
            ),
            Self::Rack => matches!(
                compatibility,
                StorageCompatibility::Tool
                    | StorageCompatibility::Weapon
                    | StorageCompatibility::LongItem
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PhysicalContainer {
    pub version: u32,
    pub container_id: ContainerId,
    pub kind: ContainerKind,
    pub lots: BTreeMap<StorageLotId, StorageLot>,
}

impl PhysicalContainer {
    #[must_use]
    pub fn new(container_id: impl Into<String>, kind: ContainerKind) -> Self {
        Self {
            version: CURRENT_PHYSICAL_STORAGE_VERSION,
            container_id: container_id.into(),
            kind,
            lots: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), PhysicalStorageError> {
        if self.version != CURRENT_PHYSICAL_STORAGE_VERSION {
            return Err(PhysicalStorageError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.container_id.trim().is_empty() {
            return Err(PhysicalStorageError::EmptyStableId);
        }
        if self.lots.len() > self.kind.lot_capacity() {
            return Err(PhysicalStorageError::ContainerFull);
        }
        let mut content_ids = BTreeSet::new();
        for (key, lot) in &self.lots {
            lot.validate()?;
            if key != &lot.lot_id {
                return Err(PhysicalStorageError::LotKeyMismatch);
            }
            if !self.kind.accepts(lot.compatibility) {
                return Err(PhysicalStorageError::IncompatibleLot {
                    lot_id: lot.lot_id.clone(),
                });
            }
            content_ids.insert(lot.content_id.as_str());
        }
        if self.kind.requires_same_content() && content_ids.len() > 1 {
            return Err(PhysicalStorageError::MixedContent);
        }
        Ok(())
    }

    pub fn insert(&mut self, lot: StorageLot) -> Result<(), PhysicalStorageError> {
        lot.validate()?;
        if self.lots.contains_key(&lot.lot_id) {
            return Err(PhysicalStorageError::DuplicateLot { lot_id: lot.lot_id });
        }
        if self.lots.len() >= self.kind.lot_capacity() {
            return Err(PhysicalStorageError::ContainerFull);
        }
        if !self.kind.accepts(lot.compatibility) {
            return Err(PhysicalStorageError::IncompatibleLot { lot_id: lot.lot_id });
        }
        if self.kind.requires_same_content()
            && self
                .lots
                .values()
                .next()
                .is_some_and(|existing| existing.content_id != lot.content_id)
        {
            return Err(PhysicalStorageError::MixedContent);
        }
        self.lots.insert(lot.lot_id.clone(), lot);
        Ok(())
    }

    pub fn remove(&mut self, lot_id: &str) -> Option<StorageLot> {
        self.lots.remove(lot_id)
    }

    #[must_use]
    pub fn fullness_slots(&self) -> (usize, usize) {
        (self.lots.len(), self.kind.lot_capacity())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VisibleStorageSlot {
    LooseLot { loose_lot: StorageLot },
    Container { container: PhysicalContainer },
}

impl VisibleStorageSlot {
    fn stable_id(&self) -> &str {
        match self {
            Self::LooseLot { loose_lot } => &loose_lot.lot_id,
            Self::Container { container } => &container.container_id,
        }
    }

    fn validate(&self) -> Result<(), PhysicalStorageError> {
        match self {
            Self::LooseLot { loose_lot } => loose_lot.validate(),
            Self::Container { container } => container.validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageTile {
    pub version: u32,
    pub position: TilePoint,
    pub slots: Vec<VisibleStorageSlot>,
}

impl StorageTile {
    #[must_use]
    pub fn new(position: TilePoint) -> Self {
        Self {
            version: CURRENT_PHYSICAL_STORAGE_VERSION,
            position,
            slots: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), PhysicalStorageError> {
        if self.version != CURRENT_PHYSICAL_STORAGE_VERSION {
            return Err(PhysicalStorageError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.slots.len() > VISIBLE_SLOTS_PER_STORAGE_TILE {
            return Err(PhysicalStorageError::TileFull);
        }
        let mut ids = BTreeSet::new();
        for slot in &self.slots {
            slot.validate()?;
            if !ids.insert(slot.stable_id()) {
                return Err(PhysicalStorageError::DuplicateVisibleSlotId);
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, slot: VisibleStorageSlot) -> Result<usize, PhysicalStorageError> {
        slot.validate()?;
        if self.slots.len() >= VISIBLE_SLOTS_PER_STORAGE_TILE {
            return Err(PhysicalStorageError::TileFull);
        }
        if self
            .slots
            .iter()
            .any(|existing| existing.stable_id() == slot.stable_id())
        {
            return Err(PhysicalStorageError::DuplicateVisibleSlotId);
        }
        self.slots.push(slot);
        Ok(self.slots.len() - 1)
    }

    pub fn remove(&mut self, stable_id: &str) -> Option<VisibleStorageSlot> {
        let index = self
            .slots
            .iter()
            .position(|slot| slot.stable_id() == stable_id)?;
        Some(self.slots.remove(index))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkshopInputZoneLink {
    pub version: u32,
    pub workshop_id: String,
    pub workshop_footprint: TaskFootprint,
    pub storage_zone_id: StorageZoneId,
    pub storage_footprint: TaskFootprint,
}

impl WorkshopInputZoneLink {
    pub fn validate(&self) -> Result<(), PhysicalStorageError> {
        if self.version != CURRENT_PHYSICAL_STORAGE_VERSION {
            return Err(PhysicalStorageError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.workshop_id.trim().is_empty() || self.storage_zone_id.trim().is_empty() {
            return Err(PhysicalStorageError::EmptyStableId);
        }
        self.workshop_footprint
            .validate()
            .map_err(|_| PhysicalStorageError::InvalidFootprint)?;
        self.storage_footprint
            .validate()
            .map_err(|_| PhysicalStorageError::InvalidFootprint)?;
        if self.workshop_footprint.width != 3
            || self.workshop_footprint.height != 3
            || self.workshop_footprint.tiles.len() != 9
        {
            return Err(PhysicalStorageError::WorkshopIsNotThreeByThree);
        }

        let workshop = self
            .workshop_footprint
            .tiles
            .as_slice()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let storage = self
            .storage_footprint
            .tiles
            .as_slice()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if !workshop.is_disjoint(&storage) {
            return Err(PhysicalStorageError::StorageOverlapsWorkshop);
        }
        let adjacent = workshop.iter().any(|workshop_tile| {
            storage.iter().any(|storage_tile| {
                u64::from(workshop_tile.x.abs_diff(storage_tile.x))
                    + u64::from(workshop_tile.y.abs_diff(storage_tile.y))
                    == 1
            })
        });
        if !adjacent {
            return Err(PhysicalStorageError::StorageNotAdjacent);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalStorageError {
    UnsupportedVersion { found: u32 },
    EmptyStableId,
    EmptyContentIdentity,
    InvalidLotQuantity,
    ExpiryBeforeProduction,
    ContainerFull,
    TileFull,
    DuplicateLot { lot_id: String },
    DuplicateVisibleSlotId,
    LotKeyMismatch,
    IncompatibleLot { lot_id: String },
    MixedContent,
    InvalidFootprint,
    WorkshopIsNotThreeByThree,
    StorageOverlapsWorkshop,
    StorageNotAdjacent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial_tasks::Rect;

    fn lot(id: &str, content_id: &str, compatibility: StorageCompatibility) -> StorageLot {
        StorageLot {
            lot_id: id.to_owned(),
            content_id: content_id.to_owned(),
            compatibility,
            units: 4,
            quality_band: 2,
            produced_at_ms: 10,
            expires_at_ms: None,
            provenance_id: "source:test".to_owned(),
            reserved_units: 0,
        }
    }

    #[test]
    fn catalog_capacities_and_compatibility_are_exact() {
        assert_eq!(ContainerKind::Basket.lot_capacity(), 4);
        assert_eq!(ContainerKind::Barrel.lot_capacity(), 8);
        assert_eq!(ContainerKind::Crate.lot_capacity(), 8);
        assert_eq!(ContainerKind::Chest.lot_capacity(), 16);
        assert_eq!(ContainerKind::Rack.lot_capacity(), 8);
        assert!(ContainerKind::Basket.accepts(StorageCompatibility::Food));
        assert!(ContainerKind::Barrel.accepts(StorageCompatibility::Liquid));
        assert!(ContainerKind::Crate.accepts(StorageCompatibility::BulkMaterial));
        assert!(ContainerKind::Chest.accepts(StorageCompatibility::UniqueItem));
        assert!(ContainerKind::Rack.accepts(StorageCompatibility::Weapon));
        assert!(!ContainerKind::Rack.accepts(StorageCompatibility::Food));
    }

    #[test]
    fn barrel_preserves_lots_but_rejects_mixed_content() {
        let mut barrel = PhysicalContainer::new("container:barrel", ContainerKind::Barrel);
        barrel
            .insert(lot("lot:water:a", "water", StorageCompatibility::Liquid))
            .expect("first water lot");
        barrel
            .insert(lot("lot:water:b", "water", StorageCompatibility::Liquid))
            .expect("second water lot");
        assert_eq!(
            barrel.insert(lot("lot:broth", "broth", StorageCompatibility::Liquid)),
            Err(PhysicalStorageError::MixedContent)
        );
        assert_eq!(barrel.fullness_slots(), (2, 8));
    }

    #[test]
    fn one_container_occupies_one_of_four_visible_tile_slots() {
        let mut tile = StorageTile::new(TilePoint { x: 7, y: 8 });
        tile.insert(VisibleStorageSlot::Container {
            container: PhysicalContainer::new("container:basket", ContainerKind::Basket),
        })
        .expect("container slot");
        for ordinal in 0..3 {
            tile.insert(VisibleStorageSlot::LooseLot {
                loose_lot: lot(
                    &format!("lot:{ordinal}"),
                    "apple",
                    StorageCompatibility::Food,
                ),
            })
            .expect("loose slot");
        }
        assert_eq!(tile.slots.len(), VISIBLE_SLOTS_PER_STORAGE_TILE);
        assert_eq!(
            tile.insert(VisibleStorageSlot::LooseLot {
                loose_lot: lot("lot:overflow", "apple", StorageCompatibility::Food),
            }),
            Err(PhysicalStorageError::TileFull)
        );
    }

    #[test]
    fn workshop_input_zone_must_be_adjacent_and_outside_full_footprint() {
        let workshop = TaskFootprint::rectangular(
            Rect::try_new(TilePoint { x: 4, y: 4 }, 3, 3).expect("workshop"),
        );
        let adjacent = WorkshopInputZoneLink {
            version: CURRENT_PHYSICAL_STORAGE_VERSION,
            workshop_id: "building:workshop".to_owned(),
            workshop_footprint: workshop.clone(),
            storage_zone_id: "zone:input".to_owned(),
            storage_footprint: TaskFootprint::rectangular(
                Rect::try_new(TilePoint { x: 7, y: 4 }, 2, 3).expect("adjacent zone"),
            ),
        };
        adjacent.validate().expect("adjacent zone is valid");

        let overlapping = WorkshopInputZoneLink {
            storage_footprint: TaskFootprint::rectangular(
                Rect::try_new(TilePoint { x: 6, y: 4 }, 2, 2).expect("overlap"),
            ),
            ..adjacent
        };
        assert_eq!(
            overlapping.validate(),
            Err(PhysicalStorageError::StorageOverlapsWorkshop)
        );
    }
}
