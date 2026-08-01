//! LAI.38 deterministic founding-food ecology.
//!
//! This leaf consumes LAI.36 content descriptors and LAI.37 physical lots. It
//! owns source state and food-use rules, but no planner choice or world mutation.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{
        CONTENT_MANIFEST_SCHEMA_VERSION, CapabilityId, ContentId, ContentManifest,
        ContentOperation, FoodDescriptor, FoodId, PhysicalLotId,
    },
    quality_lots::{
        BulkLotKey, LotLocation, LotProvenance, PhysicalLot, ProductionComplexity,
        ProductionQualityInput, QualityBand, QualityLotError, QualityLotLedger,
        QualityVariationKey, gathering_quality_score, keyed_variation, quality_from_score,
    },
};

pub const FOOD_ECOLOGY_SCHEMA_VERSION: u32 = 1;
pub const FISH_CAPACITY: u32 = 24;
pub const FISH_REPLENISH_GAME_MINUTES: u64 = 120;
const GAME_MINUTES_PER_HOUR: u64 = 60;
const APPLE_REGROWTH_STAGE_GAME_MINUTES: u64 = 1_440;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FoodEcologyError {
    UnsupportedSchemaVersion(u32),
    ManifestSchemaMismatch { stored: u32, embedded: u32 },
    InvalidFoundingSites,
    MissingFoundingCapability(CapabilityId),
    UnknownAppleTree,
    InvalidAppleTask,
    AppleObstruction,
    AppleEmpty,
    NoAppleRegrowthScheduled,
    HarvestIndexMismatch { expected: u64, actual: u64 },
    FishIndexMismatch { expected: u64, actual: u64 },
    InvalidFishTask,
    UnsupportedFishingEquipment,
    FishEmpty,
    TickNotIncreasing,
    EcologyAdvanceRequired,
    ClockOverflow,
    UnknownFood(FoodId),
    UnknownFoodContent(ContentId),
    FoodUseForbidden,
    ReservedFood,
    SpoiledFood,
    InvalidQuantity,
    NoEligibleFood,
    DuplicatePolicyFood(FoodId),
    ArithmeticOverflow,
    InvalidStableLotId,
    QualityLot(QualityLotError),
}

impl fmt::Display for FoodEcologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid food-ecology operation: {self:?}")
    }
}

impl std::error::Error for FoodEcologyError {}

impl From<QualityLotError> for FoodEcologyError {
    fn from(error: QualityLotError) -> Self {
        Self::QualityLot(error)
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterSource {
    pub source_tile: Tile,
    pub valid_bank_tile: Tile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishHabitat {
    pub water_tile: Tile,
    pub shoreline_task_tile: Tile,
    pub stock: u32,
    pub capacity: u32,
    /// Absolute game-minute cursor for the next whole-unit replenishment.
    pub next_replenish_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundingFoodSites {
    pub revealed_reachable_tiles: BTreeSet<Tile>,
    pub water: WaterSource,
    pub apple_tree_tile: Tile,
    pub fish_habitat: FishHabitat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppleState {
    Full,
    Medium,
    Low,
    Empty,
}

impl AppleState {
    const fn remaining(self) -> u32 {
        match self {
            Self::Full => 3,
            Self::Medium => 2,
            Self::Low => 1,
            Self::Empty => 0,
        }
    }

    const fn after_harvest(self) -> Option<Self> {
        match self {
            Self::Full => Some(Self::Medium),
            Self::Medium => Some(Self::Low),
            Self::Low => Some(Self::Empty),
            Self::Empty => None,
        }
    }

    const fn after_regrowth(self) -> Self {
        match self {
            Self::Empty => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium | Self::Full => Self::Full,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppleTask {
    pub tree_tile: Tile,
    pub task_tile: Tile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FishTask {
    pub task_tile: Tile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppleHarvestRequest {
    pub task: AppleTask,
    pub source_quality: QualityBand,
    pub worker_skill: u8,
    pub tool_quality: Option<QualityBand>,
    pub fixture_quality: Option<QualityBand>,
    pub world_seed: u32,
    pub harvest_index: u64,
    /// Absolute game minute, carried in the shared simulation tick field.
    pub now_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandFishingRequest {
    pub task: FishTask,
    pub source_quality: QualityBand,
    pub worker_skill: u8,
    pub tool_quality: Option<QualityBand>,
    pub fixture_quality: Option<QualityBand>,
    pub world_seed: u32,
    pub catch_index: u64,
    /// Absolute game minute, carried in the shared simulation tick field.
    pub now_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppleObstructionFootprint {
    pub cells: BTreeSet<Tile>,
    pub trunk_work_tile: Tile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportAudience {
    Leader,
    God,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportLevel(pub u8);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EcologyReport {
    Hidden,
    Estimate {
        lower: u32,
        upper: u32,
        relative_error_percent: u8,
    },
}

impl EcologyReport {
    #[must_use]
    pub const fn relative_error_percent(&self) -> Option<u8> {
        match self {
            Self::Hidden => None,
            Self::Estimate {
                relative_error_percent,
                ..
            } => Some(*relative_error_percent),
        }
    }

    #[must_use]
    pub const fn exposes_exact_regrowth_deadline(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodUse {
    RawEat,
    CookhouseIngredient,
    Trade,
    HoleFeed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodPermission {
    Allowed,
    Reserve,
    Forbidden,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoodPolicy {
    entries: BTreeMap<FoodId, FoodPermission>,
}

impl FoodPolicy {
    pub fn from_entries(
        entries: impl IntoIterator<Item = (FoodId, FoodPermission)>,
    ) -> Result<Self, FoodEcologyError> {
        let mut policy = BTreeMap::new();
        for (food_id, permission) in entries {
            if policy.insert(food_id.clone(), permission).is_some() {
                return Err(FoodEcologyError::DuplicatePolicyFood(food_id));
            }
        }
        Ok(Self { entries: policy })
    }

    fn permission(&self, food_id: &FoodId) -> FoodPermission {
        self.entries
            .get(food_id)
            .copied()
            .unwrap_or(FoodPermission::Allowed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodNeed {
    Hunger,
    Hydration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedFoodLot {
    pub lot_id: PhysicalLotId,
    pub food_id: FoodId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumptionRequest {
    pub lot_id: PhysicalLotId,
    pub quantity: u32,
    pub permission: FoodPermission,
    pub owned_capabilities: BTreeSet<CapabilityId>,
    pub now_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumptionOutcome {
    pub food_id: FoodId,
    pub quality: QualityBand,
    pub quantity: u32,
    pub nutrition: i32,
    pub hydration: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpoilageOutcome {
    pub removed_quantity: u32,
    pub released_reservations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AppleRuntime {
    state: AppleState,
    next_harvest_index: u64,
    next_regrowth_tick: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodEcology {
    manifest_schema_version: u32,
    founding_sites: FoundingFoodSites,
    apple: AppleRuntime,
    next_catch_index: u64,
    last_apple_advance_tick: u64,
    last_fish_advance_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FoodEcologyWire {
    schema_version: u32,
    manifest_schema_version: u32,
    founding_sites: FoundingFoodSites,
    apple: AppleRuntime,
    next_catch_index: u64,
    last_apple_advance_tick: u64,
    last_fish_advance_tick: u64,
}

impl Serialize for FoodEcology {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        FoodEcologyWire {
            schema_version: FOOD_ECOLOGY_SCHEMA_VERSION,
            manifest_schema_version: self.manifest_schema_version,
            founding_sites: self.founding_sites.clone(),
            apple: self.apple.clone(),
            next_catch_index: self.next_catch_index,
            last_apple_advance_tick: self.last_apple_advance_tick,
            last_fish_advance_tick: self.last_fish_advance_tick,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FoodEcology {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FoodEcologyWire::deserialize(deserializer)?;
        if wire.schema_version != FOOD_ECOLOGY_SCHEMA_VERSION {
            return Err(de::Error::custom(
                FoodEcologyError::UnsupportedSchemaVersion(wire.schema_version),
            ));
        }
        let embedded_version = ContentManifest::embedded().version;
        if wire.manifest_schema_version != embedded_version {
            return Err(de::Error::custom(
                FoodEcologyError::ManifestSchemaMismatch {
                    stored: wire.manifest_schema_version,
                    embedded: embedded_version,
                },
            ));
        }
        let ecology = Self {
            manifest_schema_version: wire.manifest_schema_version,
            founding_sites: wire.founding_sites,
            apple: wire.apple,
            next_catch_index: wire.next_catch_index,
            last_apple_advance_tick: wire.last_apple_advance_tick,
            last_fish_advance_tick: wire.last_fish_advance_tick,
        };
        ecology.validate_persisted().map_err(de::Error::custom)?;
        Ok(ecology)
    }
}

impl FoodEcology {
    pub fn new(
        manifest: &ContentManifest,
        founding_sites: FoundingFoodSites,
        founding_game_minute: u64,
    ) -> Result<Self, FoodEcologyError> {
        if manifest.version != CONTENT_MANIFEST_SCHEMA_VERSION {
            return Err(FoodEcologyError::ManifestSchemaMismatch {
                stored: manifest.version,
                embedded: CONTENT_MANIFEST_SCHEMA_VERSION,
            });
        }
        validate_founding_sites(&founding_sites, founding_game_minute)?;
        Ok(Self {
            manifest_schema_version: manifest.version,
            founding_sites,
            apple: AppleRuntime {
                state: AppleState::Full,
                next_harvest_index: 0,
                next_regrowth_tick: None,
            },
            next_catch_index: 0,
            last_apple_advance_tick: founding_game_minute,
            last_fish_advance_tick: founding_game_minute,
        })
    }

    fn validate_persisted(&self) -> Result<(), FoodEcologyError> {
        if self.founding_sites.fish_habitat.capacity != FISH_CAPACITY
            || self.founding_sites.fish_habitat.stock > FISH_CAPACITY
            || self.founding_sites.fish_habitat.next_replenish_tick <= self.last_fish_advance_tick
            || self.founding_sites.fish_habitat.next_replenish_tick - self.last_fish_advance_tick
                > FISH_REPLENISH_GAME_MINUTES
        {
            return Err(FoodEcologyError::InvalidFoundingSites);
        }
        validate_site_geometry(&self.founding_sites)?;
        match (self.apple.state, self.apple.next_regrowth_tick) {
            (AppleState::Full, None) => {}
            (AppleState::Full, Some(_))
            | (AppleState::Medium | AppleState::Low | AppleState::Empty, None) => {
                return Err(FoodEcologyError::InvalidFoundingSites);
            }
            (_, Some(deadline)) if deadline <= self.last_apple_advance_tick => {
                return Err(FoodEcologyError::InvalidFoundingSites);
            }
            _ => {}
        }
        Ok(())
    }

    #[must_use]
    pub fn founding_sites(&self) -> &FoundingFoodSites {
        &self.founding_sites
    }

    pub fn validate_founding_capabilities(
        &self,
        owned_capabilities: &BTreeSet<CapabilityId>,
    ) -> Result<(), FoodEcologyError> {
        for required in [
            "water_collection",
            "apple_gathering",
            "hand_fishing",
            "basic_food_handling",
        ] {
            let id = CapabilityId::from_str(required)
                .expect("founding food capability IDs are compile-time constants");
            if !owned_capabilities.contains(&id) {
                return Err(FoodEcologyError::MissingFoundingCapability(id));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn initial_food_lots(&self) -> Vec<PhysicalLot> {
        Vec::new()
    }

    pub fn apple_task(&self, task: AppleTask) -> Result<(), FoodEcologyError> {
        if task.tree_tile != self.founding_sites.apple_tree_tile
            || task.task_tile != self.founding_sites.apple_tree_tile
        {
            return Err(FoodEcologyError::InvalidAppleTask);
        }
        Ok(())
    }

    pub fn apple_state(&self, tree: Tile) -> Result<AppleState, FoodEcologyError> {
        if tree != self.founding_sites.apple_tree_tile {
            return Err(FoodEcologyError::UnknownAppleTree);
        }
        Ok(self.apple.state)
    }

    pub fn apple_obstruction_footprint(
        &self,
        tree: Tile,
    ) -> Result<AppleObstructionFootprint, FoodEcologyError> {
        if tree != self.founding_sites.apple_tree_tile {
            return Err(FoodEcologyError::UnknownAppleTree);
        }
        let mut cells = BTreeSet::new();
        for y_offset in -1..=1 {
            for x_offset in -1..=1 {
                cells.insert(Tile {
                    x: tree
                        .x
                        .checked_add(x_offset)
                        .ok_or(FoodEcologyError::ArithmeticOverflow)?,
                    y: tree
                        .y
                        .checked_add(y_offset)
                        .ok_or(FoodEcologyError::ArithmeticOverflow)?,
                });
            }
        }
        Ok(AppleObstructionFootprint {
            cells,
            trunk_work_tile: tree,
        })
    }

    pub fn validate_non_apple_placement(&self, tile: Tile) -> Result<(), FoodEcologyError> {
        if self
            .apple_obstruction_footprint(self.founding_sites.apple_tree_tile)?
            .cells
            .contains(&tile)
        {
            return Err(FoodEcologyError::AppleObstruction);
        }
        Ok(())
    }

    pub fn harvest_apple(
        &mut self,
        request: AppleHarvestRequest,
    ) -> Result<PhysicalLot, FoodEcologyError> {
        self.apple_task(request.task)?;
        if request.now_tick < self.last_apple_advance_tick {
            return Err(FoodEcologyError::TickNotIncreasing);
        }
        if self
            .apple
            .next_regrowth_tick
            .is_some_and(|deadline| deadline <= request.now_tick)
        {
            return Err(FoodEcologyError::EcologyAdvanceRequired);
        }
        if request.harvest_index != self.apple.next_harvest_index {
            return Err(FoodEcologyError::HarvestIndexMismatch {
                expected: self.apple.next_harvest_index,
                actual: request.harvest_index,
            });
        }
        let next_state = self
            .apple
            .state
            .after_harvest()
            .ok_or(FoodEcologyError::AppleEmpty)?;
        let lot = source_lot(
            "apple",
            request.world_seed,
            self.founding_sites.apple_tree_tile,
            request.harvest_index,
            content_id("food_apple"),
            request.source_quality,
            request.worker_skill,
            request.tool_quality,
            request.fixture_quality,
            1,
            request.now_tick,
        )?;
        let next_index = self
            .apple
            .next_harvest_index
            .checked_add(1)
            .ok_or(FoodEcologyError::ArithmeticOverflow)?;
        let next_deadline = match self.apple.next_regrowth_tick {
            Some(deadline) => Some(deadline),
            None => Some(
                request
                    .now_tick
                    .checked_add(APPLE_REGROWTH_STAGE_GAME_MINUTES)
                    .ok_or(FoodEcologyError::ClockOverflow)?,
            ),
        };
        self.apple.state = next_state;
        self.apple.next_harvest_index = next_index;
        self.apple.next_regrowth_tick = next_deadline;
        Ok(lot)
    }

    pub fn next_regrowth_tick(&self, tree: Tile) -> Result<u64, FoodEcologyError> {
        if tree != self.founding_sites.apple_tree_tile {
            return Err(FoodEcologyError::UnknownAppleTree);
        }
        self.apple
            .next_regrowth_tick
            .ok_or(FoodEcologyError::NoAppleRegrowthScheduled)
    }

    pub fn advance_regrowth(&mut self, now_game_minute: u64) -> Result<(), FoodEcologyError> {
        if now_game_minute <= self.last_apple_advance_tick {
            return Err(FoodEcologyError::TickNotIncreasing);
        }
        let mut state = self.apple.state;
        let mut deadline = self.apple.next_regrowth_tick;
        while let Some(due) = deadline {
            if due > now_game_minute {
                break;
            }
            state = state.after_regrowth();
            deadline = if state == AppleState::Full {
                None
            } else {
                Some(
                    due.checked_add(APPLE_REGROWTH_STAGE_GAME_MINUTES)
                        .ok_or(FoodEcologyError::ClockOverflow)?,
                )
            };
        }
        self.apple.state = state;
        self.apple.next_regrowth_tick = deadline;
        self.last_apple_advance_tick = now_game_minute;
        Ok(())
    }

    #[must_use]
    pub fn ecology_report(
        &self,
        _audience: ReportAudience,
        level: ReportLevel,
        tree: Tile,
    ) -> EcologyReport {
        if tree != self.founding_sites.apple_tree_tile {
            return EcologyReport::Hidden;
        }
        report(level, self.apple.state.remaining())
    }

    #[must_use]
    pub fn fish_habitat(&self) -> &FishHabitat {
        &self.founding_sites.fish_habitat
    }

    pub fn hand_fish(
        &mut self,
        request: HandFishingRequest,
    ) -> Result<PhysicalLot, FoodEcologyError> {
        if request.tool_quality.is_some() || request.fixture_quality.is_some() {
            return Err(FoodEcologyError::UnsupportedFishingEquipment);
        }
        self.catch_fish_units(request, 1)
    }

    /// Debit one accepted fishing attempt from the finite habitat.
    ///
    /// LAI.40 owns success profiles, Rod/Hut interpretation, cycle time, and
    /// wear. This source primitive owns only shoreline validation, the stable
    /// catch cursor, physical lot quality/provenance, and finite stock.
    pub fn catch_fish_units(
        &mut self,
        request: HandFishingRequest,
        quantity: u32,
    ) -> Result<PhysicalLot, FoodEcologyError> {
        if request.task.task_tile != self.founding_sites.fish_habitat.shoreline_task_tile {
            return Err(FoodEcologyError::InvalidFishTask);
        }
        if request.now_tick < self.last_fish_advance_tick {
            return Err(FoodEcologyError::TickNotIncreasing);
        }
        if self.founding_sites.fish_habitat.next_replenish_tick <= request.now_tick {
            return Err(FoodEcologyError::EcologyAdvanceRequired);
        }
        if request.catch_index != self.next_catch_index {
            return Err(FoodEcologyError::FishIndexMismatch {
                expected: self.next_catch_index,
                actual: request.catch_index,
            });
        }
        if quantity == 0 || quantity > self.founding_sites.fish_habitat.stock {
            if self.founding_sites.fish_habitat.stock == 0 {
                return Err(FoodEcologyError::FishEmpty);
            }
            return Err(FoodEcologyError::InvalidQuantity);
        }
        let lot = source_lot(
            "fish",
            request.world_seed,
            self.founding_sites.fish_habitat.water_tile,
            request.catch_index,
            content_id("food_raw_fish"),
            request.source_quality,
            request.worker_skill,
            request.tool_quality,
            request.fixture_quality,
            quantity,
            request.now_tick,
        )?;
        let next_index = self
            .next_catch_index
            .checked_add(1)
            .ok_or(FoodEcologyError::ArithmeticOverflow)?;
        self.founding_sites.fish_habitat.stock = self
            .founding_sites
            .fish_habitat
            .stock
            .checked_sub(quantity)
            .ok_or(FoodEcologyError::ArithmeticOverflow)?;
        self.next_catch_index = next_index;
        Ok(lot)
    }

    #[must_use]
    pub const fn next_catch_index(&self) -> u64 {
        self.next_catch_index
    }

    pub fn advance_fish_replenishment(
        &mut self,
        now_game_minute: u64,
    ) -> Result<(), FoodEcologyError> {
        if now_game_minute <= self.last_fish_advance_tick {
            return Err(FoodEcologyError::TickNotIncreasing);
        }
        let mut stock = self.founding_sites.fish_habitat.stock;
        let mut next = self.founding_sites.fish_habitat.next_replenish_tick;
        if next <= now_game_minute {
            let elapsed = now_game_minute - next;
            let intervals = elapsed
                .checked_div(FISH_REPLENISH_GAME_MINUTES)
                .and_then(|value| value.checked_add(1))
                .ok_or(FoodEcologyError::ClockOverflow)?;
            let cursor_advance = intervals
                .checked_mul(FISH_REPLENISH_GAME_MINUTES)
                .ok_or(FoodEcologyError::ClockOverflow)?;
            next = next
                .checked_add(cursor_advance)
                .ok_or(FoodEcologyError::ClockOverflow)?;
            let capacity_left = u64::from(FISH_CAPACITY - stock);
            let replenished = intervals.min(capacity_left);
            stock = stock
                .checked_add(
                    u32::try_from(replenished).map_err(|_| FoodEcologyError::ArithmeticOverflow)?,
                )
                .ok_or(FoodEcologyError::ArithmeticOverflow)?;
        }
        self.founding_sites.fish_habitat.stock = stock;
        self.founding_sites.fish_habitat.next_replenish_tick = next;
        self.last_fish_advance_tick = now_game_minute;
        Ok(())
    }

    #[must_use]
    pub fn fish_report(&self, _audience: ReportAudience, level: ReportLevel) -> EcologyReport {
        report(level, self.founding_sites.fish_habitat.stock)
    }

    pub fn food_use_permitted(
        &self,
        food_id: &FoodId,
        use_kind: FoodUse,
        owned_capabilities: &BTreeSet<CapabilityId>,
    ) -> Result<bool, FoodEcologyError> {
        let descriptor = food_descriptor_by_id(food_id)?;
        if use_kind == FoodUse::RawEat && !descriptor.raw_safe {
            return Ok(false);
        }
        let operation = match use_kind {
            FoodUse::Trade => ContentOperation::Trade,
            FoodUse::RawEat => ContentOperation::Process,
            FoodUse::CookhouseIngredient => ContentOperation::Craft,
            FoodUse::HoleFeed => ContentOperation::FeedHole,
        };
        ContentManifest::embedded()
            .is_operation_permitted(&descriptor.content_id, operation, owned_capabilities)
            .map_err(|_| FoodEcologyError::UnknownFood(food_id.clone()))
    }

    pub fn select_eligible_lot(
        &self,
        ledger: &QualityLotLedger,
        need: FoodNeed,
        policy: &FoodPolicy,
        owned_capabilities: &BTreeSet<CapabilityId>,
        lethal_override: bool,
        now_tick: u64,
    ) -> Result<SelectedFoodLot, FoodEcologyError> {
        let mut candidates = Vec::new();
        for lot in ledger.lots() {
            if lot.reservation.is_some() {
                continue;
            }
            let Some(descriptor) = food_descriptor_by_content(&lot.key.content_id) else {
                continue;
            };
            let permission = policy.permission(&descriptor.id);
            if permission == FoodPermission::Reserve
                || (permission == FoodPermission::Forbidden && !lethal_override)
                || is_spoiled(descriptor, lot, now_tick)?
                || !is_directly_edible(descriptor)
                || !self.food_use_permitted_for_descriptor(descriptor, owned_capabilities)?
            {
                continue;
            }
            let base = match need {
                FoodNeed::Hunger => i32::from(descriptor.nutrition),
                FoodNeed::Hydration => i32::from(descriptor.hydration),
            };
            if base <= 0 {
                continue;
            }
            let benefit = scale_signed(base, lot.key.quality, 1)?;
            candidates.push((
                std::cmp::Reverse(benefit),
                descriptor.id.clone(),
                lot.id.clone(),
            ));
        }
        candidates.sort();
        let (_, food_id, lot_id) = candidates
            .into_iter()
            .next()
            .ok_or(FoodEcologyError::NoEligibleFood)?;
        Ok(SelectedFoodLot { lot_id, food_id })
    }

    pub fn consume(
        &self,
        ledger: &mut QualityLotLedger,
        request: ConsumptionRequest,
    ) -> Result<ConsumptionOutcome, FoodEcologyError> {
        if request.quantity == 0 {
            return Err(FoodEcologyError::InvalidQuantity);
        }
        if request.permission != FoodPermission::Allowed {
            return Err(FoodEcologyError::FoodUseForbidden);
        }
        let lot = ledger.lot(&request.lot_id).cloned().ok_or_else(|| {
            FoodEcologyError::QualityLot(QualityLotError::MissingLot(request.lot_id.clone()))
        })?;
        if lot.reservation.is_some() {
            return Err(FoodEcologyError::ReservedFood);
        }
        if request.quantity > lot.quantity {
            return Err(FoodEcologyError::InvalidQuantity);
        }
        let descriptor = food_descriptor_by_content(&lot.key.content_id)
            .ok_or_else(|| FoodEcologyError::UnknownFoodContent(lot.key.content_id.clone()))?;
        if is_spoiled(descriptor, &lot, request.now_tick)? {
            return Err(FoodEcologyError::SpoiledFood);
        }
        if !is_directly_edible(descriptor)
            || !self.food_use_permitted_for_descriptor(descriptor, &request.owned_capabilities)?
        {
            return Err(FoodEcologyError::FoodUseForbidden);
        }
        let nutrition = scale_signed(
            i32::from(descriptor.nutrition),
            lot.key.quality,
            request.quantity,
        )?;
        let hydration = scale_signed(
            i32::from(descriptor.hydration),
            lot.key.quality,
            request.quantity,
        )?;
        let outcome = ConsumptionOutcome {
            food_id: descriptor.id.clone(),
            quality: lot.key.quality,
            quantity: request.quantity,
            nutrition,
            hydration,
        };
        ledger.debit_lot(&request.lot_id, request.quantity)?;
        Ok(outcome)
    }

    pub fn apply_spoilage(
        &self,
        ledger: &mut QualityLotLedger,
        now_tick: u64,
    ) -> Result<SpoilageOutcome, FoodEcologyError> {
        let mut removed_quantity = 0_u32;
        let mut expired_ids = Vec::new();
        for lot in ledger.lots() {
            let Some(descriptor) = food_descriptor_by_content(&lot.key.content_id) else {
                continue;
            };
            if is_spoiled(descriptor, lot, now_tick)? {
                removed_quantity = removed_quantity
                    .checked_add(lot.quantity)
                    .ok_or(FoodEcologyError::ArithmeticOverflow)?;
                expired_ids.push(lot.id.clone());
            }
        }
        let released_reservations = if expired_ids.is_empty() {
            Vec::new()
        } else {
            ledger.expire_lots(&expired_ids)?
        };
        Ok(SpoilageOutcome {
            removed_quantity,
            released_reservations,
        })
    }

    pub fn trade_value_milli(
        &self,
        food_id: &FoodId,
        quality: QualityBand,
    ) -> Result<u32, FoodEcologyError> {
        value_milli(food_descriptor_by_id(food_id)?, quality)
    }

    pub fn hole_value_milli(
        &self,
        food_id: &FoodId,
        quality: QualityBand,
        owned_capabilities: &BTreeSet<CapabilityId>,
    ) -> Result<u32, FoodEcologyError> {
        if !self.food_use_permitted(food_id, FoodUse::HoleFeed, owned_capabilities)? {
            return Err(FoodEcologyError::FoodUseForbidden);
        }
        value_milli(food_descriptor_by_id(food_id)?, quality)
    }

    #[must_use]
    pub fn clamp_hydration(current: i32, signed_delta: i32) -> i32 {
        current.saturating_add(signed_delta).max(0)
    }

    fn food_use_permitted_for_descriptor(
        &self,
        descriptor: &FoodDescriptor,
        owned_capabilities: &BTreeSet<CapabilityId>,
    ) -> Result<bool, FoodEcologyError> {
        ContentManifest::embedded()
            .is_operation_permitted(
                &descriptor.content_id,
                ContentOperation::Process,
                owned_capabilities,
            )
            .map_err(|_| FoodEcologyError::UnknownFood(descriptor.id.clone()))
    }
}

fn validate_founding_sites(
    sites: &FoundingFoodSites,
    founding_game_minute: u64,
) -> Result<(), FoodEcologyError> {
    validate_site_geometry(sites)?;
    if sites.fish_habitat.capacity != FISH_CAPACITY
        || sites.fish_habitat.stock != FISH_CAPACITY
        || sites.fish_habitat.next_replenish_tick
            != founding_game_minute
                .checked_add(FISH_REPLENISH_GAME_MINUTES)
                .ok_or(FoodEcologyError::ClockOverflow)?
    {
        return Err(FoodEcologyError::InvalidFoundingSites);
    }
    Ok(())
}

fn validate_site_geometry(sites: &FoundingFoodSites) -> Result<(), FoodEcologyError> {
    if sites.apple_tree_tile.x.checked_sub(1).is_none()
        || sites.apple_tree_tile.x.checked_add(1).is_none()
        || sites.apple_tree_tile.y.checked_sub(1).is_none()
        || sites.apple_tree_tile.y.checked_add(1).is_none()
    {
        return Err(FoodEcologyError::InvalidFoundingSites);
    }
    for tile in [
        sites.water.source_tile,
        sites.water.valid_bank_tile,
        sites.apple_tree_tile,
        sites.fish_habitat.water_tile,
        sites.fish_habitat.shoreline_task_tile,
    ] {
        if !sites.revealed_reachable_tiles.contains(&tile) {
            return Err(FoodEcologyError::InvalidFoundingSites);
        }
    }
    if manhattan(sites.water.source_tile, sites.water.valid_bank_tile) != Some(1)
        || manhattan(
            sites.fish_habitat.water_tile,
            sites.fish_habitat.shoreline_task_tile,
        ) != Some(1)
    {
        return Err(FoodEcologyError::InvalidFoundingSites);
    }
    Ok(())
}

fn manhattan(left: Tile, right: Tile) -> Option<u64> {
    let dx = i64::from(left.x).checked_sub(i64::from(right.x))?.abs();
    let dy = i64::from(left.y).checked_sub(i64::from(right.y))?.abs();
    u64::try_from(dx.checked_add(dy)?).ok()
}

fn report(level: ReportLevel, exact: u32) -> EcologyReport {
    let uncertainty = match level.0 {
        4 => 25,
        5.. => 10,
        _ => return EcologyReport::Hidden,
    };
    let delta = exact
        .saturating_mul(u32::from(uncertainty))
        .saturating_add(99)
        / 100;
    EcologyReport::Estimate {
        lower: exact.saturating_sub(delta),
        upper: exact.saturating_add(delta),
        relative_error_percent: uncertainty,
    }
}

#[allow(clippy::too_many_arguments)]
fn source_lot(
    source: &str,
    world_seed: u32,
    tile: Tile,
    completion_index: u64,
    content_id: ContentId,
    source_quality: QualityBand,
    worker_skill: u8,
    tool_quality: Option<QualityBand>,
    fixture_quality: Option<QualityBand>,
    quantity: u32,
    now_tick: u64,
) -> Result<PhysicalLot, FoodEcologyError> {
    let id_text = format!(
        "lot_{source}_{:016x}",
        stable_source_hash(world_seed, source, tile, completion_index)
    );
    let id = PhysicalLotId::from_str(&id_text).map_err(|_| FoodEcologyError::InvalidStableLotId)?;
    let variation = keyed_variation(&QualityVariationKey {
        world_seed,
        content_id: content_id.clone(),
        lot_id: id.clone(),
        completion_index,
    });
    let score = gathering_quality_score(
        ProductionQualityInput {
            weighted_input_quality_milli: 0,
            worker_skill,
            tool_quality,
            fixture_quality,
            station_tier: 1,
            complexity: ProductionComplexity::Raw,
            keyed_variation: variation,
        },
        source_quality,
    )?;
    Ok(PhysicalLot {
        id,
        key: BulkLotKey::new(content_id, quality_from_score(score)),
        provenance: LotProvenance {
            origin: format!("founding_{source}_source"),
            created_tick: now_tick,
        },
        quantity,
        location: LotLocation::Source(format!("{source}_source")),
        reservation: None,
    })
}

fn stable_source_hash(world_seed: u32, source: &str, tile: Tile, index: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in world_seed
        .to_le_bytes()
        .into_iter()
        .chain(source.bytes())
        .chain(tile.x.to_le_bytes())
        .chain(tile.y.to_le_bytes())
        .chain(index.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn content_id(value: &str) -> ContentId {
    ContentId::from_str(value).expect("food source content IDs are compile-time constants")
}

fn food_descriptor_by_id(food_id: &FoodId) -> Result<&'static FoodDescriptor, FoodEcologyError> {
    ContentManifest::embedded()
        .foods
        .iter()
        .find(|descriptor| &descriptor.id == food_id)
        .ok_or_else(|| FoodEcologyError::UnknownFood(food_id.clone()))
}

fn food_descriptor_by_content(content_id: &ContentId) -> Option<&'static FoodDescriptor> {
    ContentManifest::embedded()
        .foods
        .iter()
        .find(|descriptor| &descriptor.content_id == content_id)
}

fn is_directly_edible(descriptor: &FoodDescriptor) -> bool {
    descriptor.raw_safe
        || descriptor.ingredient_tags.iter().any(|tag| {
            matches!(
                tag.as_str(),
                "meal" | "preserved" | "drink" | "feast" | "prepared"
            )
        })
}

fn is_spoiled(
    descriptor: &FoodDescriptor,
    lot: &PhysicalLot,
    now_tick: u64,
) -> Result<bool, FoodEcologyError> {
    let Some(lifetime) = descriptor.spoilage_hours else {
        return Ok(false);
    };
    let deadline = lot
        .provenance
        .created_tick
        .checked_add(
            u64::from(lifetime)
                .checked_mul(GAME_MINUTES_PER_HOUR)
                .ok_or(FoodEcologyError::ClockOverflow)?,
        )
        .ok_or(FoodEcologyError::ClockOverflow)?;
    Ok(now_tick >= deadline)
}

fn scale_signed(base: i32, quality: QualityBand, quantity: u32) -> Result<i32, FoodEcologyError> {
    let scaled = i64::from(base)
        .checked_mul(i64::from(quality.food_nutrition_percent()))
        .and_then(|value| value.checked_mul(i64::from(quantity)))
        .ok_or(FoodEcologyError::ArithmeticOverflow)?
        / 100;
    i32::try_from(scaled).map_err(|_| FoodEcologyError::ArithmeticOverflow)
}

fn value_milli(descriptor: &FoodDescriptor, quality: QualityBand) -> Result<u32, FoodEcologyError> {
    descriptor
        .value_milli
        .checked_mul(u32::from(quality.trade_hole_value_percent()))
        .ok_or(FoodEcologyError::ArithmeticOverflow)
        .map(|value| value / 100)
}
