//! LAI.37 universal quality and physical-lot authority.
//!
//! This pure leaf consumes the stable content IDs from `content_manifest`. It
//! deliberately owns neither world mutation nor protocol/persistence adapters.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{
        AugmentationSlot, ContentId, EquipmentSlot, FixtureSlot, ItemDefinitionId, MaterialId,
        MaterialInstanceId, PhysicalLotId,
    },
    rng,
};

pub const QUALITY_LOTS_SCHEMA_VERSION: u32 = 2;
pub const MAX_PHYSICAL_LOTS: usize = 4_096;
pub const MAX_ITEM_INSTANCES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityLotError {
    InvalidQualityOrdinal(u8),
    InvalidSkill(u8),
    InvalidStationTier,
    InvalidVariation(i16),
    ArithmeticOverflow,
    InvalidSchemaVersion(u32),
    EmptyLocation,
    EmptyProvenance,
    EmptyReservation,
    ZeroQuantity,
    DuplicateLotId(PhysicalLotId),
    DuplicateItemId(MaterialInstanceId),
    MissingLot(PhysicalLotId),
    MissingItem(MaterialInstanceId),
    DuplicateDebitLotId(PhysicalLotId),
    InvalidDebitQuantity,
    ReservedLot(PhysicalLotId),
    DuplicateSplitLotId(PhysicalLotId),
    InvalidSplitQuantity,
    IncompatibleMerge,
    IneligibleAugmentation,
    IncompatibleFixture,
    InventoryLimitExceeded,
}

impl fmt::Display for QualityLotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid quality-lot state: {self:?}")
    }
}

impl std::error::Error for QualityLotError {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityBand {
    Crude = 0,
    Common = 1,
    Fine = 2,
    Superior = 3,
    Masterwork = 4,
}

impl QualityBand {
    pub const ALL: [Self; 5] = [
        Self::Crude,
        Self::Common,
        Self::Fine,
        Self::Superior,
        Self::Masterwork,
    ];

    #[must_use]
    pub const fn ordinal(self) -> u8 {
        self as u8
    }

    pub const fn from_ordinal(ordinal: u8) -> Result<Self, QualityLotError> {
        match ordinal {
            0 => Ok(Self::Crude),
            1 => Ok(Self::Common),
            2 => Ok(Self::Fine),
            3 => Ok(Self::Superior),
            4 => Ok(Self::Masterwork),
            _ => Err(QualityLotError::InvalidQualityOrdinal(ordinal)),
        }
    }

    #[must_use]
    pub const fn input_quality_milli(self) -> i32 {
        (self.ordinal() as i32) * 1_000
    }

    #[must_use]
    pub const fn food_nutrition_percent(self) -> u16 {
        [80, 100, 120, 145, 175][self.ordinal() as usize]
    }

    #[must_use]
    pub const fn trade_hole_value_percent(self) -> u16 {
        [75, 100, 130, 170, 225][self.ordinal() as usize]
    }

    #[must_use]
    pub const fn item_effect_durability_percent(self) -> u16 {
        [80, 100, 115, 135, 160][self.ordinal() as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionComplexity {
    Raw,
    Simple,
    Prepared,
    Complex,
    Feast,
}

impl ProductionComplexity {
    const fn penalty(self) -> i32 {
        match self {
            Self::Raw | Self::Simple => 0,
            Self::Prepared => 250,
            Self::Complex => 500,
            Self::Feast => 750,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionQualityInput {
    pub weighted_input_quality_milli: i32,
    pub worker_skill: u8,
    pub tool_quality: Option<QualityBand>,
    pub fixture_quality: Option<QualityBand>,
    pub station_tier: u8,
    pub complexity: ProductionComplexity,
    pub keyed_variation: i16,
}

fn skill_bonus(skill: u8) -> Result<i32, QualityLotError> {
    match skill {
        0..=19 => Ok(-500),
        20..=39 => Ok(0),
        40..=59 => Ok(250),
        60..=79 => Ok(500),
        80..=94 => Ok(750),
        95..=100 => Ok(1_000),
        _ => Err(QualityLotError::InvalidSkill(skill)),
    }
}

fn quality_bonus(quality: Option<QualityBand>) -> i32 {
    quality.map_or(0, |value| (i32::from(value.ordinal()) + 1) * 100)
}

fn checked_score(
    input_quality_milli: i32,
    worker_skill: u8,
    tool_quality: Option<QualityBand>,
    fixture_quality: Option<QualityBand>,
    station_tier: u8,
    penalty: i32,
    keyed_variation: i16,
) -> Result<i32, QualityLotError> {
    if station_tier == 0 {
        return Err(QualityLotError::InvalidStationTier);
    }
    if !(-250..=250).contains(&keyed_variation) {
        return Err(QualityLotError::InvalidVariation(keyed_variation));
    }
    let station_bonus = i32::from(station_tier - 1)
        .checked_mul(125)
        .ok_or(QualityLotError::ArithmeticOverflow)?;
    input_quality_milli
        .checked_add(skill_bonus(worker_skill)?)
        .and_then(|score| score.checked_add(quality_bonus(tool_quality)))
        .and_then(|score| score.checked_add(quality_bonus(fixture_quality)))
        .and_then(|score| score.checked_add(station_bonus))
        .and_then(|score| score.checked_sub(penalty))
        .and_then(|score| score.checked_add(i32::from(keyed_variation)))
        .ok_or(QualityLotError::ArithmeticOverflow)
}

pub fn production_quality_score(input: ProductionQualityInput) -> Result<i32, QualityLotError> {
    checked_score(
        input.weighted_input_quality_milli,
        input.worker_skill,
        input.tool_quality,
        input.fixture_quality,
        input.station_tier,
        input.complexity.penalty(),
        input.keyed_variation,
    )
}

pub fn gathering_quality_score(
    input: ProductionQualityInput,
    source_quality: QualityBand,
) -> Result<i32, QualityLotError> {
    checked_score(
        source_quality.input_quality_milli(),
        input.worker_skill,
        input.tool_quality,
        input.fixture_quality,
        input.station_tier,
        0,
        input.keyed_variation,
    )
}

#[must_use]
pub const fn quality_from_score(score: i32) -> QualityBand {
    match score {
        ..750 => QualityBand::Crude,
        750..=1_749 => QualityBand::Common,
        1_750..=2_749 => QualityBand::Fine,
        2_750..=3_749 => QualityBand::Superior,
        _ => QualityBand::Masterwork,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityVariationKey {
    pub world_seed: u32,
    pub content_id: ContentId,
    pub lot_id: PhysicalLotId,
    pub completion_index: u64,
}

#[must_use]
pub fn keyed_variation(key: &QualityVariationKey) -> i16 {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;
    let mut hash = key.world_seed ^ FNV_OFFSET;
    let completion_bytes = key.completion_index.to_le_bytes();
    for bytes in [
        key.content_id.as_str().as_bytes(),
        key.lot_id.as_str().as_bytes(),
        &completion_bytes,
    ] {
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    let roll = rng::roll_seeded(f64::from(hash.max(1))).value;
    (roll.mul_add(501.0, 0.0).floor() as i16) - 250
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BulkLotKey {
    pub content_id: ContentId,
    pub quality: QualityBand,
}

impl BulkLotKey {
    #[must_use]
    pub fn new(content_id: ContentId, quality: QualityBand) -> Self {
        Self {
            content_id,
            quality,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotProvenance {
    pub origin: String,
    pub created_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LotLocation {
    Source(String),
    Stockpile(String),
    StationInput(String),
    StationOutput(String),
    Cargo(String),
    Cache(String),
    Hole(String),
}

impl LotLocation {
    fn identifier(&self) -> &str {
        match self {
            Self::Source(id)
            | Self::Stockpile(id)
            | Self::StationInput(id)
            | Self::StationOutput(id)
            | Self::Cargo(id)
            | Self::Cache(id)
            | Self::Hole(id) => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalLot {
    pub id: PhysicalLotId,
    pub key: BulkLotKey,
    pub provenance: LotProvenance,
    pub quantity: u32,
    pub location: LotLocation,
    pub reservation: Option<String>,
}

impl PhysicalLot {
    fn validate(&self) -> Result<(), QualityLotError> {
        if self.quantity == 0 {
            return Err(QualityLotError::ZeroQuantity);
        }
        if self.provenance.origin.trim().is_empty() {
            return Err(QualityLotError::EmptyProvenance);
        }
        if self.location.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        if self
            .reservation
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(QualityLotError::EmptyReservation);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExactItemPayload {
    pub id: MaterialInstanceId,
    pub definition_id: ItemDefinitionId,
    pub material_id: MaterialId,
    pub quality: QualityBand,
    pub durability: u32,
    pub location: LotLocation,
    pub reservation: Option<String>,
    pub equipment_slot: Option<EquipmentSlot>,
    pub augmentation_slot: Option<AugmentationSlot>,
}

impl ExactItemPayload {
    fn validate(&self) -> Result<(), QualityLotError> {
        if self.location.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        if self
            .reservation
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(QualityLotError::EmptyReservation);
        }
        Ok(())
    }

    #[must_use]
    pub fn from_unaugmented_item(item: ItemInstance) -> Option<Self> {
        if item.augmentation.is_some() {
            return None;
        }
        Some(Self {
            id: item.id,
            definition_id: item.definition_id,
            material_id: item.material_id,
            quality: item.quality,
            durability: item.durability,
            location: item.location,
            reservation: item.reservation,
            equipment_slot: item.equipment_slot,
            augmentation_slot: item.augmentation_slot,
        })
    }

    #[must_use]
    pub fn into_item(self) -> ItemInstance {
        ItemInstance {
            id: self.id,
            definition_id: self.definition_id,
            material_id: self.material_id,
            quality: self.quality,
            durability: self.durability,
            location: self.location,
            reservation: self.reservation,
            equipment_slot: self.equipment_slot,
            augmentation_slot: self.augmentation_slot,
            augmentation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemAugmentation {
    pub item: ExactItemPayload,
    pub slot: AugmentationSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstance {
    pub id: MaterialInstanceId,
    pub definition_id: ItemDefinitionId,
    pub material_id: MaterialId,
    pub quality: QualityBand,
    pub durability: u32,
    pub location: LotLocation,
    pub reservation: Option<String>,
    pub equipment_slot: Option<EquipmentSlot>,
    /// Catalog-authorized augmentation compatibility; `None` means this item
    /// is not augmentable.
    pub augmentation_slot: Option<AugmentationSlot>,
    pub augmentation: Option<ItemAugmentation>,
}

impl ItemInstance {
    fn validate(&self) -> Result<(), QualityLotError> {
        if self.location.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        if self
            .reservation
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(QualityLotError::EmptyReservation);
        }
        if self.augmentation.as_ref().is_some_and(|augmentation| {
            self.augmentation_slot != Some(augmentation.slot)
                || augmentation.item.augmentation_slot != Some(augmentation.slot)
                || augmentation.item.validate().is_err()
        }) {
            return Err(QualityLotError::IneligibleAugmentation);
        }
        Ok(())
    }

    pub fn install_augmentation(
        &mut self,
        augmentation: ItemAugmentation,
    ) -> Result<(), QualityLotError> {
        if self.reservation.is_some()
            || self.equipment_slot.is_some()
            || matches!(&self.location, LotLocation::Cargo(_))
            || self.durability == 0
            || self.augmentation.is_some()
            || self.augmentation_slot != Some(augmentation.slot)
        {
            return Err(QualityLotError::IneligibleAugmentation);
        }
        self.augmentation = Some(augmentation);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StationFixture {
    pub item: ExactItemPayload,
    pub slot: FixtureSlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureInstance {
    pub slot: FixtureSlot,
    pub installed: Option<StationFixture>,
    pub reserved: bool,
}

impl FixtureInstance {
    pub fn install_fixture(&mut self, fixture: StationFixture) -> Result<(), QualityLotError> {
        if self.reserved
            || self.installed.is_some()
            || fixture.slot != self.slot
            || fixture.item.validate().is_err()
        {
            return Err(QualityLotError::IncompatibleFixture);
        }
        self.installed = Some(fixture);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReason {
    Cancelled,
    CarrierDeath,
    RouteLost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityLotLedger {
    lots: BTreeMap<PhysicalLotId, PhysicalLot>,
    items: BTreeMap<MaterialInstanceId, ItemInstance>,
}

impl QualityLotLedger {
    pub fn new(lots: Vec<PhysicalLot>, items: Vec<ItemInstance>) -> Result<Self, QualityLotError> {
        if lots.len() > MAX_PHYSICAL_LOTS || items.len() > MAX_ITEM_INSTANCES {
            return Err(QualityLotError::InventoryLimitExceeded);
        }
        let mut lot_index = BTreeMap::new();
        for lot in lots {
            lot.validate()?;
            let id = lot.id.clone();
            if lot_index.insert(id.clone(), lot).is_some() {
                return Err(QualityLotError::DuplicateLotId(id));
            }
        }
        let mut item_index = BTreeMap::new();
        for item in items {
            item.validate()?;
            let id = item.id.clone();
            if item_index.insert(id.clone(), item).is_some() {
                return Err(QualityLotError::DuplicateItemId(id));
            }
        }
        Ok(Self {
            lots: lot_index,
            items: item_index,
        })
    }

    #[must_use]
    pub fn total_bulk_quantity(&self) -> u64 {
        self.lots.values().map(|lot| u64::from(lot.quantity)).sum()
    }

    #[must_use]
    pub fn lot(&self, id: &PhysicalLotId) -> Option<&PhysicalLot> {
        self.lots.get(id)
    }

    /// Iterate physical lots in stable lot-ID order.
    pub fn lots(&self) -> impl ExactSizeIterator<Item = &PhysicalLot> {
        self.lots.values()
    }

    #[must_use]
    pub fn item(&self, id: &MaterialInstanceId) -> Option<&ItemInstance> {
        self.items.get(id)
    }

    /// Iterate exact item instances in stable item-ID order.
    pub fn items(&self) -> impl ExactSizeIterator<Item = &ItemInstance> {
        self.items.values()
    }

    /// Atomically insert one newly produced exact item instance.
    pub fn insert_item(&mut self, item: ItemInstance) -> Result<(), QualityLotError> {
        self.insert_items(vec![item])
    }

    /// Atomically validate and insert a batch of exact item instances.
    pub fn insert_items(&mut self, items: Vec<ItemInstance>) -> Result<(), QualityLotError> {
        if self.items.len().saturating_add(items.len()) > MAX_ITEM_INSTANCES {
            return Err(QualityLotError::InventoryLimitExceeded);
        }
        let mut staged = BTreeMap::new();
        for item in items {
            item.validate()?;
            let id = item.id.clone();
            if self.items.contains_key(&id) || staged.insert(id.clone(), item).is_some() {
                return Err(QualityLotError::DuplicateItemId(id));
            }
        }
        self.items.extend(staged);
        Ok(())
    }

    pub fn remove_item(
        &mut self,
        id: &MaterialInstanceId,
    ) -> Result<ItemInstance, QualityLotError> {
        self.items
            .remove(id)
            .ok_or_else(|| QualityLotError::MissingItem(id.clone()))
    }

    pub fn replace_item(&mut self, item: ItemInstance) -> Result<(), QualityLotError> {
        item.validate()?;
        if !self.items.contains_key(&item.id) {
            return Err(QualityLotError::MissingItem(item.id));
        }
        self.items.insert(item.id.clone(), item);
        Ok(())
    }

    pub fn recover_item(
        &mut self,
        id: &MaterialInstanceId,
        reason: RecoveryReason,
        destination: LotLocation,
    ) -> Result<(), QualityLotError> {
        if destination.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        let item = self
            .items
            .get_mut(id)
            .ok_or_else(|| QualityLotError::MissingItem(id.clone()))?;
        item.location = destination;
        match reason {
            RecoveryReason::Cancelled
            | RecoveryReason::CarrierDeath
            | RecoveryReason::RouteLost => {
                item.reservation = None;
            }
        }
        Ok(())
    }

    /// Atomically insert one newly produced physical lot.
    pub fn insert_lot(&mut self, lot: PhysicalLot) -> Result<(), QualityLotError> {
        self.insert_lots(vec![lot])
    }

    /// Atomically validate and insert a batch of newly produced physical lots.
    pub fn insert_lots(&mut self, lots: Vec<PhysicalLot>) -> Result<(), QualityLotError> {
        if self.lots.len().saturating_add(lots.len()) > MAX_PHYSICAL_LOTS {
            return Err(QualityLotError::InventoryLimitExceeded);
        }
        let mut staged = BTreeMap::new();
        for lot in lots {
            lot.validate()?;
            let id = lot.id.clone();
            if self.lots.contains_key(&id) || staged.insert(id.clone(), lot).is_some() {
                return Err(QualityLotError::DuplicateLotId(id));
            }
        }
        self.lots.extend(staged);
        Ok(())
    }

    /// Atomically debit one unreserved physical lot without changing its identity.
    pub fn debit_lot(&mut self, id: &PhysicalLotId, quantity: u32) -> Result<(), QualityLotError> {
        self.debit_lots(&[(id.clone(), quantity)])
    }

    /// Atomically debit several unreserved lots after validating the whole batch.
    pub fn debit_lots(&mut self, debits: &[(PhysicalLotId, u32)]) -> Result<(), QualityLotError> {
        let mut checked = BTreeMap::<PhysicalLotId, u32>::new();
        for (id, quantity) in debits {
            if *quantity == 0 {
                return Err(QualityLotError::InvalidDebitQuantity);
            }
            if checked.insert(id.clone(), *quantity).is_some() {
                return Err(QualityLotError::DuplicateDebitLotId(id.clone()));
            }
            let lot = self
                .lots
                .get(id)
                .ok_or_else(|| QualityLotError::MissingLot(id.clone()))?;
            if lot.reservation.is_some() {
                return Err(QualityLotError::ReservedLot(id.clone()));
            }
            if *quantity > lot.quantity {
                return Err(QualityLotError::InvalidDebitQuantity);
            }
        }

        for (id, quantity) in checked {
            let remove = self
                .lots
                .get(&id)
                .is_some_and(|lot| lot.quantity == quantity);
            if remove {
                self.lots.remove(&id);
            } else {
                self.lots
                    .get_mut(&id)
                    .expect("debit batch was preflighted")
                    .quantity -= quantity;
            }
        }
        Ok(())
    }

    /// Atomically remove complete expired lots, including reserved cargo.
    ///
    /// Spoilage is a physical event, so a reservation cannot preserve expired
    /// food. Returned reservation IDs let the task/runtime owner invalidate
    /// dependent work without reconstructing hidden inventory state.
    pub fn expire_lots(&mut self, ids: &[PhysicalLotId]) -> Result<Vec<String>, QualityLotError> {
        let mut checked = BTreeMap::<PhysicalLotId, Option<String>>::new();
        for id in ids {
            let lot = self
                .lots
                .get(id)
                .ok_or_else(|| QualityLotError::MissingLot(id.clone()))?;
            if checked
                .insert(id.clone(), lot.reservation.clone())
                .is_some()
            {
                return Err(QualityLotError::DuplicateDebitLotId(id.clone()));
            }
        }

        let released_reservations = checked
            .values()
            .filter_map(Clone::clone)
            .collect::<Vec<_>>();
        for id in checked.keys() {
            self.lots.remove(id);
        }
        Ok(released_reservations)
    }

    pub fn move_lot(
        &mut self,
        id: &PhysicalLotId,
        location: LotLocation,
    ) -> Result<(), QualityLotError> {
        if location.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        let lot = self
            .lots
            .get_mut(id)
            .ok_or_else(|| QualityLotError::MissingLot(id.clone()))?;
        lot.location = location;
        Ok(())
    }

    pub fn split_lot(
        &mut self,
        source_id: &PhysicalLotId,
        split_id: PhysicalLotId,
        quantity: u32,
    ) -> Result<(), QualityLotError> {
        if quantity == 0 {
            return Err(QualityLotError::InvalidSplitQuantity);
        }
        if self.lots.contains_key(&split_id) {
            return Err(QualityLotError::DuplicateSplitLotId(split_id));
        }
        let source = self
            .lots
            .get(source_id)
            .cloned()
            .ok_or_else(|| QualityLotError::MissingLot(source_id.clone()))?;
        if quantity >= source.quantity {
            return Err(QualityLotError::InvalidSplitQuantity);
        }
        let mut split = source.clone();
        split.id = split_id;
        split.quantity = quantity;
        let remaining = source.quantity - quantity;
        self.lots
            .get_mut(source_id)
            .expect("source checked above")
            .quantity = remaining;
        self.lots.insert(split.id.clone(), split);
        Ok(())
    }

    pub fn merge_lots(
        &mut self,
        left_id: &PhysicalLotId,
        right_id: &PhysicalLotId,
    ) -> Result<(), QualityLotError> {
        if left_id == right_id {
            return Err(QualityLotError::IncompatibleMerge);
        }
        let left = self
            .lots
            .get(left_id)
            .cloned()
            .ok_or_else(|| QualityLotError::MissingLot(left_id.clone()))?;
        let right = self
            .lots
            .get(right_id)
            .cloned()
            .ok_or_else(|| QualityLotError::MissingLot(right_id.clone()))?;
        if left.key != right.key
            || left.provenance != right.provenance
            || left.location != right.location
            || left.reservation.is_some()
            || right.reservation.is_some()
        {
            return Err(QualityLotError::IncompatibleMerge);
        }
        let quantity = left
            .quantity
            .checked_add(right.quantity)
            .ok_or(QualityLotError::ArithmeticOverflow)?;
        self.lots
            .get_mut(left_id)
            .expect("left checked above")
            .quantity = quantity;
        self.lots.remove(right_id);
        Ok(())
    }

    pub fn recover_lot(
        &mut self,
        id: &PhysicalLotId,
        reason: RecoveryReason,
        destination: LotLocation,
    ) -> Result<(), QualityLotError> {
        if destination.identifier().trim().is_empty() {
            return Err(QualityLotError::EmptyLocation);
        }
        let lot = self
            .lots
            .get_mut(id)
            .ok_or_else(|| QualityLotError::MissingLot(id.clone()))?;
        lot.location = destination;
        match reason {
            RecoveryReason::Cancelled
            | RecoveryReason::CarrierDeath
            | RecoveryReason::RouteLost => {
                lot.reservation = None;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BulkLotKeyWire {
    schema_version: u32,
    content_id: ContentId,
    quality: QualityBand,
}

impl Serialize for BulkLotKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        BulkLotKeyWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            content_id: self.content_id.clone(),
            quality: self.quality,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BulkLotKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BulkLotKeyWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        Ok(Self::new(wire.content_id, wire.quality))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LotProvenanceWire {
    schema_version: u32,
    origin: String,
    created_tick: u64,
}

impl Serialize for LotProvenance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        LotProvenanceWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            origin: self.origin.clone(),
            created_tick: self.created_tick,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LotProvenance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = LotProvenanceWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        if wire.origin.trim().is_empty() {
            return Err(de::Error::custom(QualityLotError::EmptyProvenance));
        }
        Ok(Self {
            origin: wire.origin,
            created_tick: wire.created_tick,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LotLocationKind {
    Source,
    Stockpile,
    StationInput,
    StationOutput,
    Cargo,
    Cache,
    Hole,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LotLocationWire {
    schema_version: u32,
    kind: LotLocationKind,
    id: String,
}

impl Serialize for LotLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (kind, id) = match self {
            Self::Source(id) => (LotLocationKind::Source, id),
            Self::Stockpile(id) => (LotLocationKind::Stockpile, id),
            Self::StationInput(id) => (LotLocationKind::StationInput, id),
            Self::StationOutput(id) => (LotLocationKind::StationOutput, id),
            Self::Cargo(id) => (LotLocationKind::Cargo, id),
            Self::Cache(id) => (LotLocationKind::Cache, id),
            Self::Hole(id) => (LotLocationKind::Hole, id),
        };
        LotLocationWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            kind,
            id: id.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LotLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = LotLocationWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        if wire.id.trim().is_empty() {
            return Err(de::Error::custom(QualityLotError::EmptyLocation));
        }
        Ok(match wire.kind {
            LotLocationKind::Source => Self::Source(wire.id),
            LotLocationKind::Stockpile => Self::Stockpile(wire.id),
            LotLocationKind::StationInput => Self::StationInput(wire.id),
            LotLocationKind::StationOutput => Self::StationOutput(wire.id),
            LotLocationKind::Cargo => Self::Cargo(wire.id),
            LotLocationKind::Cache => Self::Cache(wire.id),
            LotLocationKind::Hole => Self::Hole(wire.id),
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PhysicalLotWire {
    schema_version: u32,
    id: PhysicalLotId,
    key: BulkLotKey,
    provenance: LotProvenance,
    quantity: u32,
    location: LotLocation,
    reservation: Option<String>,
}

impl Serialize for PhysicalLot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PhysicalLotWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            id: self.id.clone(),
            key: self.key.clone(),
            provenance: self.provenance.clone(),
            quantity: self.quantity,
            location: self.location.clone(),
            reservation: self.reservation.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PhysicalLot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PhysicalLotWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        let lot = Self {
            id: wire.id,
            key: wire.key,
            provenance: wire.provenance,
            quantity: wire.quantity,
            location: wire.location,
            reservation: wire.reservation,
        };
        lot.validate().map_err(de::Error::custom)?;
        Ok(lot)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ItemInstanceWire {
    schema_version: u32,
    id: MaterialInstanceId,
    definition_id: ItemDefinitionId,
    material_id: MaterialId,
    quality: QualityBand,
    durability: u32,
    location: LotLocation,
    reservation: Option<String>,
    equipment_slot: Option<EquipmentSlot>,
    augmentation_slot: Option<AugmentationSlot>,
    augmentation: Option<ItemAugmentation>,
}

impl Serialize for ItemInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ItemInstanceWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            id: self.id.clone(),
            definition_id: self.definition_id.clone(),
            material_id: self.material_id.clone(),
            quality: self.quality,
            durability: self.durability,
            location: self.location.clone(),
            reservation: self.reservation.clone(),
            equipment_slot: self.equipment_slot,
            augmentation_slot: self.augmentation_slot,
            augmentation: self.augmentation.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ItemInstance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ItemInstanceWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        let item = Self {
            id: wire.id,
            definition_id: wire.definition_id,
            material_id: wire.material_id,
            quality: wire.quality,
            durability: wire.durability,
            location: wire.location,
            reservation: wire.reservation,
            equipment_slot: wire.equipment_slot,
            augmentation_slot: wire.augmentation_slot,
            augmentation: wire.augmentation,
        };
        item.validate().map_err(de::Error::custom)?;
        Ok(item)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LedgerWire {
    schema_version: u32,
    lots: Vec<PhysicalLot>,
    items: Vec<ItemInstance>,
}

impl Serialize for QualityLotLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        LedgerWire {
            schema_version: QUALITY_LOTS_SCHEMA_VERSION,
            lots: self.lots.values().cloned().collect(),
            items: self.items.values().cloned().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QualityLotLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = LedgerWire::deserialize(deserializer)?;
        if wire.schema_version != QUALITY_LOTS_SCHEMA_VERSION {
            return Err(de::Error::custom(QualityLotError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        Self::new(wire.lots, wire.items).map_err(de::Error::custom)
    }
}
