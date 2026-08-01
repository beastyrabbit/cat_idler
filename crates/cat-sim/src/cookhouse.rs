//! LAI.39 manifest-driven Cookhouse transaction authority.
//!
//! This pure leaf consumes the LAI.36 catalog, LAI.37 identities/quality, and
//! LAI.3 spatial ordering. World mutation and hauling adapters remain later
//! integration work; this module emits complete, deterministic transactions.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    content_manifest::{
        CONTENT_MANIFEST_SCHEMA_VERSION, CapabilityId, CapabilityRequirement, ContentId,
        ContentManifest, ItemDefinitionId, MaterialInstanceId, PLAN1_BREW_RECIPE_IDS,
        PLAN1_COOKHOUSE_RECIPE_IDS, PhysicalLotId, RecipeDescriptor, RecipeId,
    },
    quality_lots::{
        BulkLotKey, ItemInstance, LotLocation, LotProvenance, PhysicalLot, ProductionComplexity,
        ProductionQualityInput, QualityBand, QualityLotLedger, QualityVariationKey,
        keyed_variation, production_quality_score, quality_from_score,
    },
    spatial_tasks::{Rect, SpatialInvariantError, TaskFootprint, TilePoint},
};

pub const COOKHOUSE_BATCH_SCHEMA_VERSION: u32 = 1;
pub const COOKHOUSE_QUEUE_SCHEMA_VERSION: u32 = 1;
pub const MAX_COOKHOUSE_QUEUE_ENTRIES: usize = 64;
pub const WORK_UNITS_PER_COMPLEXITY: u64 = 100;
const FUEL_CONTENT_ID: &str = "resource_fuel";
const CONTAINER_CONTENT_ID: &str = "resource_clay";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookhouseBlockedReason {
    Catalog,
    Station,
    Tier,
    Capability,
    Worker,
    Tool,
    Fixture,
    Ingredient,
    Reservation,
    Capacity,
    Queue,
    State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookhouseError {
    InvalidBatchId,
    InvalidStationId,
    InvalidWorkerId,
    UnknownRecipe(RecipeId),
    RetiredRecipe(RecipeId),
    NotCookhouseRecipe(RecipeId),
    InvalidCookhouseManifest,
    Spatial(SpatialInvariantError),
    InsufficientStationTier {
        required: u8,
        actual: u8,
    },
    NoWorkers,
    InvalidWorkerSkill(u8),
    MissingCapability(CapabilityId),
    MissingTool(ItemDefinitionId),
    InvalidTool(MaterialInstanceId),
    MissingFixture(ItemDefinitionId),
    InvalidFixture(MaterialInstanceId),
    InsufficientOutputCapacity {
        required: u32,
        available: u32,
    },
    DuplicateInputLot(PhysicalLotId),
    ReservedInput(PhysicalLotId),
    LotOutsideEligibleInput(PhysicalLotId),
    MissingIngredient {
        content_id: ContentId,
        required: u32,
        selected: u32,
    },
    UnknownContent(ContentId),
    DuplicateOutputLotId(PhysicalLotId),
    ArithmeticOverflow,
    QualityCalculation,
    InvalidSchemaVersion(u32),
    ManifestSchemaMismatch(u32),
    InvalidPersistedState,
    InvalidStage,
    TerminalBatch,
    QueueFull,
    DuplicateQueueEntry(String),
    MissingQueueEntry(String),
}

impl CookhouseError {
    #[must_use]
    pub const fn blocked_reason(&self) -> CookhouseBlockedReason {
        match self {
            Self::InvalidCookhouseManifest
            | Self::UnknownRecipe(_)
            | Self::RetiredRecipe(_)
            | Self::NotCookhouseRecipe(_)
            | Self::UnknownContent(_)
            | Self::ManifestSchemaMismatch(_) => CookhouseBlockedReason::Catalog,
            Self::InvalidStationId | Self::Spatial(_) => CookhouseBlockedReason::Station,
            Self::InsufficientStationTier { .. } => CookhouseBlockedReason::Tier,
            Self::MissingCapability(_) => CookhouseBlockedReason::Capability,
            Self::NoWorkers | Self::InvalidWorkerId | Self::InvalidWorkerSkill(_) => {
                CookhouseBlockedReason::Worker
            }
            Self::MissingTool(_) | Self::InvalidTool(_) => CookhouseBlockedReason::Tool,
            Self::MissingFixture(_) | Self::InvalidFixture(_) => CookhouseBlockedReason::Fixture,
            Self::MissingIngredient { .. }
            | Self::DuplicateInputLot(_)
            | Self::LotOutsideEligibleInput(_) => CookhouseBlockedReason::Ingredient,
            Self::ReservedInput(_) => CookhouseBlockedReason::Reservation,
            Self::InsufficientOutputCapacity { .. } => CookhouseBlockedReason::Capacity,
            Self::QueueFull | Self::DuplicateQueueEntry(_) | Self::MissingQueueEntry(_) => {
                CookhouseBlockedReason::Queue
            }
            Self::InvalidBatchId
            | Self::DuplicateOutputLotId(_)
            | Self::ArithmeticOverflow
            | Self::QualityCalculation
            | Self::InvalidSchemaVersion(_)
            | Self::InvalidPersistedState
            | Self::InvalidStage
            | Self::TerminalBatch => CookhouseBlockedReason::State,
        }
    }
}

impl fmt::Display for CookhouseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Cookhouse operation: {self:?}")
    }
}

impl std::error::Error for CookhouseError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CookhouseFixture {
    pub instance_id: MaterialInstanceId,
    pub definition_id: ItemDefinitionId,
    pub quality: QualityBand,
    pub station_id: String,
    pub reserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookhouseReadiness {
    pub station_id: String,
    pub station_tier: u8,
    pub worker_id: String,
    pub worker_skill: u8,
    pub capabilities: BTreeSet<CapabilityId>,
    pub tools: Vec<ItemInstance>,
    pub fixtures: Vec<CookhouseFixture>,
    pub output_free_units: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookhouseBatchRequest {
    pub batch_id: String,
    pub recipe_id: RecipeId,
    pub world_seed: u32,
    pub completion_index: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStage {
    Reserved,
    InTransit,
    Ready,
    Cooking,
    OutputReady,
    PickedUp,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngredientState {
    Reserved,
    InTransit,
    StationInput,
    Consumed,
    Recovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputState {
    Planned,
    StationOutput,
    Cargo,
    Recovered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedIngredient {
    pub lot_id: PhysicalLotId,
    pub content_id: ContentId,
    pub quality: QualityBand,
    pub provenance: LotProvenance,
    pub original_location: LotLocation,
    pub reserved_quantity: u32,
    pub consumed_quantity: u32,
    pub state: IngredientState,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReservedIngredientWire {
    lot_id: PhysicalLotId,
    content_id: ContentId,
    quality: QualityBand,
    provenance_origin: String,
    provenance_created_tick: u64,
    original_location: LotLocationWire,
    reserved_quantity: u32,
    consumed_quantity: u32,
    state: IngredientState,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LotLocationWire {
    kind: LotLocationKind,
    id: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
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

impl From<&LotLocation> for LotLocationWire {
    fn from(location: &LotLocation) -> Self {
        let (kind, id) = match location {
            LotLocation::Source(id) => (LotLocationKind::Source, id),
            LotLocation::Stockpile(id) => (LotLocationKind::Stockpile, id),
            LotLocation::StationInput(id) => (LotLocationKind::StationInput, id),
            LotLocation::StationOutput(id) => (LotLocationKind::StationOutput, id),
            LotLocation::Cargo(id) => (LotLocationKind::Cargo, id),
            LotLocation::Cache(id) => (LotLocationKind::Cache, id),
            LotLocation::Hole(id) => (LotLocationKind::Hole, id),
        };
        Self {
            kind,
            id: id.clone(),
        }
    }
}

impl LotLocationWire {
    fn into_location(self) -> LotLocation {
        match self.kind {
            LotLocationKind::Source => LotLocation::Source(self.id),
            LotLocationKind::Stockpile => LotLocation::Stockpile(self.id),
            LotLocationKind::StationInput => LotLocation::StationInput(self.id),
            LotLocationKind::StationOutput => LotLocation::StationOutput(self.id),
            LotLocationKind::Cargo => LotLocation::Cargo(self.id),
            LotLocationKind::Cache => LotLocation::Cache(self.id),
            LotLocationKind::Hole => LotLocation::Hole(self.id),
        }
    }
}

impl Serialize for ReservedIngredient {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ReservedIngredientWire {
            lot_id: self.lot_id.clone(),
            content_id: self.content_id.clone(),
            quality: self.quality,
            provenance_origin: self.provenance.origin.clone(),
            provenance_created_tick: self.provenance.created_tick,
            original_location: LotLocationWire::from(&self.original_location),
            reserved_quantity: self.reserved_quantity,
            consumed_quantity: self.consumed_quantity,
            state: self.state,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ReservedIngredient {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ReservedIngredientWire::deserialize(deserializer)?;
        Ok(Self {
            lot_id: wire.lot_id,
            content_id: wire.content_id,
            quality: wire.quality,
            provenance: LotProvenance {
                origin: wire.provenance_origin,
                created_tick: wire.provenance_created_tick,
            },
            original_location: wire.original_location.into_location(),
            reserved_quantity: wire.reserved_quantity,
            consumed_quantity: wire.consumed_quantity,
            state: wire.state,
        })
    }
}

impl ReservedIngredient {
    fn restored_lot(&self) -> PhysicalLot {
        PhysicalLot {
            id: self.lot_id.clone(),
            key: BulkLotKey::new(self.content_id.clone(), self.quality),
            provenance: self.provenance.clone(),
            quantity: self.reserved_quantity,
            location: self.original_location.clone(),
            reservation: None,
        }
    }

    fn remainder_lot(&self) -> Option<PhysicalLot> {
        let quantity = self.reserved_quantity.checked_sub(self.consumed_quantity)?;
        (quantity > 0).then(|| PhysicalLot {
            id: self.lot_id.clone(),
            key: BulkLotKey::new(self.content_id.clone(), self.quality),
            provenance: self.provenance.clone(),
            quantity,
            location: self.original_location.clone(),
            reservation: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputLotPlan {
    pub lot_id: PhysicalLotId,
    pub content_id: ContentId,
    pub quality: QualityBand,
    pub quantity: u32,
    pub state: OutputState,
}

impl OutputLotPlan {
    fn physical_lot(&self, batch_id: &str, station_id: &str, completed_tick: u64) -> PhysicalLot {
        let location = match self.state {
            OutputState::Cargo => LotLocation::Cargo(format!("cookhouse_{batch_id}")),
            OutputState::Planned | OutputState::StationOutput | OutputState::Recovered => {
                LotLocation::StationOutput(station_id.to_owned())
            }
        };
        PhysicalLot {
            id: self.lot_id.clone(),
            key: BulkLotKey::new(self.content_id.clone(), self.quality),
            provenance: LotProvenance {
                origin: format!("cookhouse_batch_{batch_id}"),
                created_tick: completed_tick,
            },
            quantity: self.quantity,
            location,
            reservation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedTool {
    pub instance_id: MaterialInstanceId,
    pub definition_id: ItemDefinitionId,
    pub quality: QualityBand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectedFixture {
    pub instance_id: MaterialInstanceId,
    pub definition_id: ItemDefinitionId,
    pub quality: QualityBand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookhouseCompletion {
    pub batch_id: String,
    pub recipe_id: RecipeId,
    pub consumed: Vec<ReservedIngredient>,
    pub remainders: Vec<PhysicalLot>,
    pub outputs: Vec<PhysicalLot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookhouseRecoveryReason {
    Cancelled,
    WorkerDeath,
    RouteLoss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookhouseRecovery {
    pub reason: CookhouseRecoveryReason,
    pub inputs: Vec<PhysicalLot>,
    pub outputs: Vec<PhysicalLot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CookhouseBatch {
    schema_version: u32,
    manifest_schema_version: u32,
    batch_id: String,
    station_id: String,
    station_tier: u8,
    worker_id: String,
    worker_skill: u8,
    recipe_id: RecipeId,
    world_seed: u32,
    completion_index: u64,
    stage: BatchStage,
    paused: bool,
    work_required: u64,
    work_completed: u64,
    completed_tick: Option<u64>,
    weighted_input_quality_milli: i32,
    reserved_inputs: Vec<ReservedIngredient>,
    selected_tools: Vec<SelectedTool>,
    selected_fixtures: Vec<SelectedFixture>,
    output_plans: Vec<OutputLotPlan>,
    recovery_reason: Option<CookhouseRecoveryReason>,
}

impl CookhouseBatch {
    #[must_use]
    pub const fn stage(&self) -> BatchStage {
        self.stage
    }

    #[must_use]
    pub const fn paused(&self) -> bool {
        self.paused
    }

    #[must_use]
    pub const fn work_required(&self) -> u64 {
        self.work_required
    }

    #[must_use]
    pub const fn work_completed(&self) -> u64 {
        self.work_completed
    }

    #[must_use]
    pub const fn weighted_input_quality_milli(&self) -> i32 {
        self.weighted_input_quality_milli
    }

    #[must_use]
    pub fn reserved_inputs(&self) -> &[ReservedIngredient] {
        &self.reserved_inputs
    }

    /// Project the current physical input lots for a later ledger/hauling adapter.
    ///
    /// Consumed inputs disappear; every other stage preserves the original lot ID,
    /// quality, provenance, and quantity while changing only reservation/location.
    #[must_use]
    pub fn physical_inputs(&self) -> Vec<PhysicalLot> {
        self.reserved_inputs
            .iter()
            .filter_map(|input| {
                let (location, reservation) = match input.state {
                    IngredientState::Reserved => {
                        (input.original_location.clone(), Some(self.batch_id.clone()))
                    }
                    IngredientState::InTransit => (
                        LotLocation::Cargo(format!("cookhouse_input_{}", self.batch_id)),
                        Some(self.batch_id.clone()),
                    ),
                    IngredientState::StationInput => (
                        LotLocation::StationInput(self.station_id.clone()),
                        Some(self.batch_id.clone()),
                    ),
                    IngredientState::Recovered => (input.original_location.clone(), None),
                    IngredientState::Consumed => return None,
                };
                Some(PhysicalLot {
                    id: input.lot_id.clone(),
                    key: BulkLotKey::new(input.content_id.clone(), input.quality),
                    provenance: input.provenance.clone(),
                    quantity: input.reserved_quantity,
                    location,
                    reservation,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn output_plans(&self) -> &[OutputLotPlan] {
        &self.output_plans
    }

    #[must_use]
    pub fn selected_tools(&self) -> &[SelectedTool] {
        &self.selected_tools
    }

    #[must_use]
    pub fn selected_fixtures(&self) -> &[SelectedFixture] {
        &self.selected_fixtures
    }

    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("Cookhouse batch serialization is infallible")
    }

    pub fn decode_strict(json: &str) -> Result<Self, CookhouseError> {
        let batch = serde_json::from_str::<Self>(json)
            .map_err(|_| CookhouseError::InvalidPersistedState)?;
        batch.validate_persisted(ContentManifest::embedded())?;
        Ok(batch)
    }

    pub fn set_paused(&mut self, paused: bool) -> Result<(), CookhouseError> {
        if matches!(
            self.stage,
            BatchStage::OutputReady | BatchStage::PickedUp | BatchStage::Cancelled
        ) {
            return Err(CookhouseError::TerminalBatch);
        }
        self.paused = paused;
        Ok(())
    }

    pub fn mark_inputs_in_transit(&mut self) -> Result<(), CookhouseError> {
        match self.stage {
            BatchStage::Reserved => {
                self.stage = BatchStage::InTransit;
                for input in &mut self.reserved_inputs {
                    input.state = IngredientState::InTransit;
                }
                Ok(())
            }
            BatchStage::InTransit => Ok(()),
            _ => Err(CookhouseError::InvalidStage),
        }
    }

    pub fn deliver_inputs(&mut self) -> Result<(), CookhouseError> {
        match self.stage {
            BatchStage::Reserved | BatchStage::InTransit => {
                self.stage = BatchStage::Ready;
                for input in &mut self.reserved_inputs {
                    input.state = IngredientState::StationInput;
                }
                Ok(())
            }
            BatchStage::Ready | BatchStage::Cooking => Ok(()),
            _ => Err(CookhouseError::InvalidStage),
        }
    }

    pub fn advance_work(
        &mut self,
        work_units: u64,
        current_tick: u64,
    ) -> Result<Option<CookhouseCompletion>, CookhouseError> {
        match self.stage {
            BatchStage::OutputReady | BatchStage::PickedUp => {
                return Ok(Some(self.completion()?));
            }
            BatchStage::Cancelled => return Err(CookhouseError::TerminalBatch),
            BatchStage::Reserved | BatchStage::InTransit => {
                return Err(CookhouseError::InvalidStage);
            }
            BatchStage::Ready | BatchStage::Cooking => {}
        }
        if self.paused || work_units == 0 {
            return Ok(None);
        }
        let next = self
            .work_completed
            .checked_add(work_units)
            .ok_or(CookhouseError::ArithmeticOverflow)?
            .min(self.work_required);
        self.work_completed = next;
        if next < self.work_required {
            self.stage = BatchStage::Cooking;
            return Ok(None);
        }
        self.stage = BatchStage::OutputReady;
        self.completed_tick = Some(current_tick);
        for input in &mut self.reserved_inputs {
            input.state = IngredientState::Consumed;
        }
        for output in &mut self.output_plans {
            output.state = OutputState::StationOutput;
        }
        Ok(Some(self.completion()?))
    }

    pub fn pickup_outputs(&mut self) -> Result<Vec<PhysicalLot>, CookhouseError> {
        match self.stage {
            BatchStage::OutputReady => {
                self.stage = BatchStage::PickedUp;
                for output in &mut self.output_plans {
                    output.state = OutputState::Cargo;
                }
                Ok(self.completion()?.outputs)
            }
            BatchStage::PickedUp => Ok(self.completion()?.outputs),
            _ => Err(CookhouseError::InvalidStage),
        }
    }

    pub fn recover(
        &mut self,
        reason: CookhouseRecoveryReason,
    ) -> Result<CookhouseRecovery, CookhouseError> {
        if let Some(existing) = self.recovery_reason {
            return Ok(self.recovery_result(existing));
        }
        if self.stage == BatchStage::Cancelled {
            return Err(CookhouseError::TerminalBatch);
        }
        let completed_tick = self.completed_tick.unwrap_or(0);
        let before_consumption = matches!(
            self.stage,
            BatchStage::Reserved | BatchStage::InTransit | BatchStage::Ready | BatchStage::Cooking
        );
        let inputs = if before_consumption {
            for input in &mut self.reserved_inputs {
                input.state = IngredientState::Recovered;
            }
            self.reserved_inputs
                .iter()
                .map(ReservedIngredient::restored_lot)
                .collect()
        } else {
            Vec::new()
        };
        let outputs = if before_consumption {
            Vec::new()
        } else {
            for output in &mut self.output_plans {
                output.state = OutputState::Recovered;
            }
            self.output_plans
                .iter()
                .map(|output| output.physical_lot(&self.batch_id, &self.station_id, completed_tick))
                .collect()
        };
        self.stage = BatchStage::Cancelled;
        self.paused = false;
        let recovery = CookhouseRecovery {
            reason,
            inputs,
            outputs,
        };
        self.recovery_reason = Some(reason);
        Ok(recovery)
    }

    fn recovery_result(&self, reason: CookhouseRecoveryReason) -> CookhouseRecovery {
        let completed_tick = self.completed_tick.unwrap_or(0);
        CookhouseRecovery {
            reason,
            inputs: self
                .reserved_inputs
                .iter()
                .filter(|input| input.state == IngredientState::Recovered)
                .map(ReservedIngredient::restored_lot)
                .collect(),
            outputs: self
                .output_plans
                .iter()
                .filter(|output| output.state == OutputState::Recovered)
                .map(|output| output.physical_lot(&self.batch_id, &self.station_id, completed_tick))
                .collect(),
        }
    }

    fn completion(&self) -> Result<CookhouseCompletion, CookhouseError> {
        let completed_tick = self
            .completed_tick
            .ok_or(CookhouseError::InvalidPersistedState)?;
        Ok(CookhouseCompletion {
            batch_id: self.batch_id.clone(),
            recipe_id: self.recipe_id.clone(),
            consumed: self.reserved_inputs.clone(),
            remainders: self
                .reserved_inputs
                .iter()
                .filter_map(ReservedIngredient::remainder_lot)
                .collect(),
            outputs: self
                .output_plans
                .iter()
                .map(|output| output.physical_lot(&self.batch_id, &self.station_id, completed_tick))
                .collect(),
        })
    }

    fn validate_persisted(&self, manifest: &ContentManifest) -> Result<(), CookhouseError> {
        if self.schema_version != COOKHOUSE_BATCH_SCHEMA_VERSION {
            return Err(CookhouseError::InvalidSchemaVersion(self.schema_version));
        }
        if self.manifest_schema_version != manifest.version {
            return Err(CookhouseError::ManifestSchemaMismatch(
                self.manifest_schema_version,
            ));
        }
        if !valid_local_id(&self.batch_id)
            || !valid_local_id(&self.station_id)
            || !valid_local_id(&self.worker_id)
            || self.station_tier == 0
            || self.worker_skill > 100
        {
            return Err(CookhouseError::InvalidPersistedState);
        }
        let recipe = recipe_for_execution(manifest, &self.recipe_id)?;
        let station = cookhouse_station(manifest)?;
        if self.work_required != work_duration(recipe.complexity)?
            || self.work_completed > self.work_required
            || self.reserved_inputs.is_empty()
            || self.output_plans.is_empty()
            || self.station_tier < station.min_tier.max(recipe.station_tier)
        {
            return Err(CookhouseError::InvalidPersistedState);
        }
        validate_stage(self)?;
        let mut input_ids = BTreeSet::new();
        let mut actual_inputs = BTreeMap::<ContentId, u32>::new();
        for input in &self.reserved_inputs {
            if input.reserved_quantity == 0
                || input.consumed_quantity == 0
                || input.consumed_quantity > input.reserved_quantity
                || !input_ids.insert(input.lot_id.clone())
                || input.provenance.origin.trim().is_empty()
                || !match &input.original_location {
                    LotLocation::Stockpile(id) => !id.trim().is_empty(),
                    LotLocation::StationInput(id) => id == &self.station_id,
                    _ => false,
                }
            {
                return Err(CookhouseError::InvalidPersistedState);
            }
            let quantity = actual_inputs.entry(input.content_id.clone()).or_default();
            *quantity = quantity
                .checked_add(input.consumed_quantity)
                .ok_or(CookhouseError::ArithmeticOverflow)?;
        }
        if actual_inputs != required_inputs(recipe)? {
            return Err(CookhouseError::InvalidPersistedState);
        }
        if weighted_input_quality(&self.reserved_inputs)? != self.weighted_input_quality_milli {
            return Err(CookhouseError::InvalidPersistedState);
        }
        if self
            .selected_tools
            .iter()
            .map(|tool| &tool.definition_id)
            .ne(recipe.tools.iter())
            || self
                .selected_fixtures
                .iter()
                .map(|fixture| &fixture.definition_id)
                .ne(recipe.fixtures.iter())
        {
            return Err(CookhouseError::InvalidPersistedState);
        }
        let mut selected_instances = BTreeSet::new();
        if self
            .selected_tools
            .iter()
            .map(|tool| &tool.instance_id)
            .chain(
                self.selected_fixtures
                    .iter()
                    .map(|fixture| &fixture.instance_id),
            )
            .any(|instance_id| !selected_instances.insert(instance_id.clone()))
        {
            return Err(CookhouseError::InvalidPersistedState);
        }
        let mut output_ids = BTreeSet::new();
        let mut output_quality = None;
        for (index, output) in self.output_plans.iter().enumerate() {
            let descriptor = recipe
                .outputs
                .get(index)
                .ok_or(CookhouseError::InvalidPersistedState)?;
            if output.quantity != descriptor.units
                || output.content_id != descriptor.content_id
                || input_ids.contains(&output.lot_id)
                || !output_ids.insert(output.lot_id.clone())
                || output.lot_id != stable_output_lot_id(&self.batch_id, &self.recipe_id, index)?
                || output_quality.is_some_and(|quality| quality != output.quality)
            {
                return Err(CookhouseError::InvalidPersistedState);
            }
            output_quality = Some(output.quality);
        }
        if self.output_plans.len() != recipe.outputs.len() {
            return Err(CookhouseError::InvalidPersistedState);
        }
        let first_output = self
            .output_plans
            .first()
            .ok_or(CookhouseError::InvalidPersistedState)?;
        let variation = keyed_variation(&QualityVariationKey {
            world_seed: self.world_seed,
            content_id: first_output.content_id.clone(),
            lot_id: first_output.lot_id.clone(),
            completion_index: self.completion_index,
        });
        let expected_score = production_quality_score(ProductionQualityInput {
            weighted_input_quality_milli: self.weighted_input_quality_milli,
            worker_skill: self.worker_skill,
            tool_quality: self.selected_tools.first().map(|tool| tool.quality),
            fixture_quality: self
                .selected_fixtures
                .first()
                .map(|fixture| fixture.quality),
            station_tier: self.station_tier,
            complexity: production_complexity(recipe.complexity)?,
            keyed_variation: variation,
        })
        .map_err(|_| CookhouseError::InvalidPersistedState)?;
        if output_quality != Some(quality_from_score(expected_score)) {
            return Err(CookhouseError::InvalidPersistedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CookhouseQueueEntry {
    pub entry_id: String,
    pub recipe_id: RecipeId,
    pub repeat: bool,
    pub paused: bool,
    pub progress_work_units: u64,
    pub completed_batches: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CookhouseQueue {
    schema_version: u32,
    manifest_schema_version: u32,
    station_id: String,
    entries: Vec<CookhouseQueueEntry>,
}

impl CookhouseQueue {
    pub fn new(station_id: String) -> Result<Self, CookhouseError> {
        if !valid_local_id(&station_id) {
            return Err(CookhouseError::InvalidStationId);
        }
        Ok(Self {
            schema_version: COOKHOUSE_QUEUE_SCHEMA_VERSION,
            manifest_schema_version: CONTENT_MANIFEST_SCHEMA_VERSION,
            station_id,
            entries: Vec::new(),
        })
    }

    #[must_use]
    pub fn entries(&self) -> &[CookhouseQueueEntry] {
        &self.entries
    }

    pub fn enqueue(
        &mut self,
        manifest: &ContentManifest,
        entry: CookhouseQueueEntry,
    ) -> Result<(), CookhouseError> {
        validate_cookhouse_catalog(manifest)?;
        validate_queue_entry(manifest, &entry)?;
        if self.entries.len() >= MAX_COOKHOUSE_QUEUE_ENTRIES {
            return Err(CookhouseError::QueueFull);
        }
        if self
            .entries
            .iter()
            .any(|existing| existing.entry_id == entry.entry_id)
        {
            return Err(CookhouseError::DuplicateQueueEntry(entry.entry_id));
        }
        self.entries.push(entry);
        Ok(())
    }

    pub fn set_paused(&mut self, entry_id: &str, paused: bool) -> Result<(), CookhouseError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.entry_id == entry_id)
            .ok_or_else(|| CookhouseError::MissingQueueEntry(entry_id.to_owned()))?;
        entry.paused = paused;
        Ok(())
    }

    pub fn add_progress(
        &mut self,
        manifest: &ContentManifest,
        entry_id: &str,
        work_units: u64,
    ) -> Result<(), CookhouseError> {
        validate_cookhouse_catalog(manifest)?;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.entry_id == entry_id)
            .ok_or_else(|| CookhouseError::MissingQueueEntry(entry_id.to_owned()))?;
        if !entry.paused {
            let recipe = recipe_for_execution(manifest, &entry.recipe_id)?;
            let required = work_duration(recipe.complexity)?;
            entry.progress_work_units = entry
                .progress_work_units
                .checked_add(work_units)
                .ok_or(CookhouseError::ArithmeticOverflow)?
                .min(required);
        }
        Ok(())
    }

    pub fn complete_front(&mut self, manifest: &ContentManifest) -> Result<(), CookhouseError> {
        validate_cookhouse_catalog(manifest)?;
        let mut entry = self
            .entries
            .first()
            .cloned()
            .ok_or_else(|| CookhouseError::MissingQueueEntry("front".to_owned()))?;
        if entry.paused {
            return Err(CookhouseError::InvalidStage);
        }
        let recipe = recipe_for_execution(manifest, &entry.recipe_id)?;
        if entry.progress_work_units < work_duration(recipe.complexity)? {
            return Err(CookhouseError::InvalidStage);
        }
        let next_completed_batches = if entry.repeat {
            Some(
                entry
                    .completed_batches
                    .checked_add(1)
                    .ok_or(CookhouseError::ArithmeticOverflow)?,
            )
        } else {
            None
        };
        self.entries.remove(0);
        if let Some(completed_batches) = next_completed_batches {
            entry.progress_work_units = 0;
            entry.completed_batches = completed_batches;
            self.entries.push(entry);
        }
        Ok(())
    }

    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("Cookhouse queue serialization is infallible")
    }

    pub fn decode_strict(json: &str) -> Result<Self, CookhouseError> {
        let queue = serde_json::from_str::<Self>(json)
            .map_err(|_| CookhouseError::InvalidPersistedState)?;
        queue.validate(ContentManifest::embedded())?;
        Ok(queue)
    }

    fn validate(&self, manifest: &ContentManifest) -> Result<(), CookhouseError> {
        if self.schema_version != COOKHOUSE_QUEUE_SCHEMA_VERSION {
            return Err(CookhouseError::InvalidSchemaVersion(self.schema_version));
        }
        if self.manifest_schema_version != manifest.version {
            return Err(CookhouseError::ManifestSchemaMismatch(
                self.manifest_schema_version,
            ));
        }
        if !valid_local_id(&self.station_id) || self.entries.len() > MAX_COOKHOUSE_QUEUE_ENTRIES {
            return Err(CookhouseError::InvalidPersistedState);
        }
        validate_cookhouse_catalog(manifest)?;
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            validate_queue_entry(manifest, entry)?;
            if !ids.insert(entry.entry_id.clone()) {
                return Err(CookhouseError::InvalidPersistedState);
            }
        }
        Ok(())
    }
}

pub fn cookhouse_task_footprint(
    manifest: &ContentManifest,
    anchor: TilePoint,
) -> Result<TaskFootprint, CookhouseError> {
    let station = cookhouse_station(manifest)?;
    let geometry = &station.work_geometry;
    let rect = Rect::try_new(
        TilePoint {
            x: anchor
                .x
                .checked_add(i32::from(geometry.origin_x))
                .ok_or(CookhouseError::ArithmeticOverflow)?,
            y: anchor
                .y
                .checked_add(i32::from(geometry.origin_y))
                .ok_or(CookhouseError::ArithmeticOverflow)?,
        },
        i32::from(geometry.width),
        i32::from(geometry.height),
    )
    .map_err(CookhouseError::Spatial)?;
    if rect.tile_count() != usize::from(geometry.occupied_cells)
        || geometry.occupied_cells != station.footprint_cells
    {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    Ok(TaskFootprint::rectangular(rect))
}

pub fn prepare_batch(
    manifest: &ContentManifest,
    readiness: &CookhouseReadiness,
    input_lots: &[PhysicalLot],
    request: CookhouseBatchRequest,
) -> Result<CookhouseBatch, CookhouseError> {
    validate_identity_and_worker(readiness, &request)?;
    validate_cookhouse_catalog(manifest)?;
    let recipe = recipe_for_execution(manifest, &request.recipe_id)?;
    let station = cookhouse_station(manifest)?;
    let required_tier = station.min_tier.max(recipe.station_tier);
    if readiness.station_tier < required_tier {
        return Err(CookhouseError::InsufficientStationTier {
            required: required_tier,
            actual: readiness.station_tier,
        });
    }
    require_capability(&station.canonical_capability, &readiness.capabilities)?;
    require_capability(&recipe.canonical_capability, &readiness.capabilities)?;
    validate_bundle_owner(manifest, recipe, &readiness.capabilities)?;

    let required_inputs = required_inputs(recipe)?;
    for content_id in required_inputs.keys() {
        let requirement = content_capability(manifest, content_id)
            .ok_or_else(|| CookhouseError::UnknownContent(content_id.clone()))?;
        require_capability(requirement, &readiness.capabilities)?;
    }
    let selected_tools = select_tools(recipe, readiness)?;
    let selected_fixtures = select_fixtures(recipe, readiness)?;

    let output_units = recipe.outputs.iter().try_fold(0_u32, |total, output| {
        total
            .checked_add(output.units)
            .ok_or(CookhouseError::ArithmeticOverflow)
    })?;
    if output_units > readiness.output_free_units {
        return Err(CookhouseError::InsufficientOutputCapacity {
            required: output_units,
            available: readiness.output_free_units,
        });
    }
    let reserved_inputs = select_ingredients(&readiness.station_id, input_lots, &required_inputs)?;
    let weighted_input_quality_milli = weighted_input_quality(&reserved_inputs)?;
    let output_ids = recipe
        .outputs
        .iter()
        .enumerate()
        .map(|(index, _)| stable_output_lot_id(&request.batch_id, &request.recipe_id, index))
        .collect::<Result<Vec<_>, _>>()?;
    let input_ids = reserved_inputs
        .iter()
        .map(|input| input.lot_id.clone())
        .collect::<BTreeSet<_>>();
    let mut distinct_output_ids = BTreeSet::new();
    for output_id in &output_ids {
        if input_ids.contains(output_id) || !distinct_output_ids.insert(output_id.clone()) {
            return Err(CookhouseError::DuplicateOutputLotId(output_id.clone()));
        }
    }
    let first_output = recipe
        .outputs
        .first()
        .ok_or(CookhouseError::InvalidCookhouseManifest)?;
    let variation = keyed_variation(&QualityVariationKey {
        world_seed: request.world_seed,
        content_id: first_output.content_id.clone(),
        lot_id: output_ids[0].clone(),
        completion_index: request.completion_index,
    });
    let score = production_quality_score(ProductionQualityInput {
        weighted_input_quality_milli,
        worker_skill: readiness.worker_skill,
        tool_quality: selected_tools.first().map(|tool| tool.quality),
        fixture_quality: selected_fixtures.first().map(|fixture| fixture.quality),
        station_tier: readiness.station_tier,
        complexity: production_complexity(recipe.complexity)?,
        keyed_variation: variation,
    })
    .map_err(|_| CookhouseError::QualityCalculation)?;
    let quality = quality_from_score(score);
    let output_plans = recipe
        .outputs
        .iter()
        .zip(output_ids)
        .map(|(output, lot_id)| OutputLotPlan {
            lot_id,
            content_id: output.content_id.clone(),
            quality,
            quantity: output.units,
            state: OutputState::Planned,
        })
        .collect();
    let batch = CookhouseBatch {
        schema_version: COOKHOUSE_BATCH_SCHEMA_VERSION,
        manifest_schema_version: manifest.version,
        batch_id: request.batch_id,
        station_id: readiness.station_id.clone(),
        station_tier: readiness.station_tier,
        worker_id: readiness.worker_id.clone(),
        worker_skill: readiness.worker_skill,
        recipe_id: recipe.id.clone(),
        world_seed: request.world_seed,
        completion_index: request.completion_index,
        stage: BatchStage::Reserved,
        paused: false,
        work_required: work_duration(recipe.complexity)?,
        work_completed: 0,
        completed_tick: None,
        weighted_input_quality_milli,
        reserved_inputs,
        selected_tools,
        selected_fixtures,
        output_plans,
        recovery_reason: None,
    };
    batch.validate_persisted(manifest)?;
    Ok(batch)
}

pub fn prepare_batch_from_ledger(
    manifest: &ContentManifest,
    readiness: &CookhouseReadiness,
    ledger: &QualityLotLedger,
    request: CookhouseBatchRequest,
) -> Result<CookhouseBatch, CookhouseError> {
    let lots = ledger.lots().cloned().collect::<Vec<_>>();
    prepare_batch(manifest, readiness, &lots, request)
}

fn validate_identity_and_worker(
    readiness: &CookhouseReadiness,
    request: &CookhouseBatchRequest,
) -> Result<(), CookhouseError> {
    if !valid_local_id(&request.batch_id) {
        return Err(CookhouseError::InvalidBatchId);
    }
    if !valid_local_id(&readiness.station_id) {
        return Err(CookhouseError::InvalidStationId);
    }
    if readiness.worker_id.is_empty() {
        return Err(CookhouseError::NoWorkers);
    }
    if !valid_local_id(&readiness.worker_id) {
        return Err(CookhouseError::InvalidWorkerId);
    }
    if readiness.worker_skill > 100 {
        return Err(CookhouseError::InvalidWorkerSkill(readiness.worker_skill));
    }
    Ok(())
}

fn validate_cookhouse_catalog(manifest: &ContentManifest) -> Result<(), CookhouseError> {
    if manifest.version != CONTENT_MANIFEST_SCHEMA_VERSION {
        return Err(CookhouseError::ManifestSchemaMismatch(manifest.version));
    }
    cookhouse_station(manifest)?;
    let cookhouse = manifest
        .recipes
        .iter()
        .filter(|recipe| recipe.station.as_str() == "cookhouse")
        .map(|recipe| recipe.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = PLAN1_COOKHOUSE_RECIPE_IDS
        .iter()
        .chain(PLAN1_BREW_RECIPE_IDS.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    if cookhouse != expected {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    let mill = manifest
        .recipes
        .iter()
        .filter(|recipe| recipe.station.as_str() == "mill")
        .map(|recipe| recipe.id.as_str())
        .collect::<Vec<_>>();
    if mill.as_slice() != ["mill_flour"] {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    Ok(())
}

fn cookhouse_station(
    manifest: &ContentManifest,
) -> Result<&crate::content_manifest::StationDescriptor, CookhouseError> {
    let station = manifest
        .stations
        .iter()
        .find(|station| station.id.as_str() == "cookhouse")
        .ok_or(CookhouseError::InvalidCookhouseManifest)?;
    if station.footprint_cells != 9
        || station.work_geometry.width != 3
        || station.work_geometry.height != 3
        || station.work_geometry.occupied_cells != 9
    {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    Ok(station)
}

fn recipe_for_execution<'a>(
    manifest: &'a ContentManifest,
    recipe_id: &RecipeId,
) -> Result<&'a RecipeDescriptor, CookhouseError> {
    if manifest
        .recipe_cutover
        .iter()
        .any(|receipt| &receipt.legacy_id == recipe_id)
    {
        return Err(CookhouseError::RetiredRecipe(recipe_id.clone()));
    }
    let recipe = manifest
        .recipes
        .iter()
        .find(|recipe| &recipe.id == recipe_id)
        .ok_or_else(|| CookhouseError::UnknownRecipe(recipe_id.clone()))?;
    if recipe.station.as_str() != "cookhouse"
        || (!PLAN1_COOKHOUSE_RECIPE_IDS.contains(&recipe.id.as_str())
            && !PLAN1_BREW_RECIPE_IDS.contains(&recipe.id.as_str()))
    {
        return Err(CookhouseError::NotCookhouseRecipe(recipe.id.clone()));
    }
    Ok(recipe)
}

fn validate_bundle_owner(
    manifest: &ContentManifest,
    recipe: &RecipeDescriptor,
    capabilities: &BTreeSet<CapabilityId>,
) -> Result<(), CookhouseError> {
    let owners = manifest
        .recipe_bundles
        .iter()
        .filter(|bundle| bundle.recipes.contains(&recipe.id))
        .collect::<Vec<_>>();
    let [bundle] = owners.as_slice() else {
        return Err(CookhouseError::InvalidCookhouseManifest);
    };
    if bundle.capability != recipe.bundle_capability {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    if !capabilities.contains(&bundle.capability) {
        return Err(CookhouseError::MissingCapability(bundle.capability.clone()));
    }
    let owner_capability = content_capability(manifest, &bundle.owner)
        .ok_or_else(|| CookhouseError::UnknownContent(bundle.owner.clone()))?;
    require_capability(owner_capability, capabilities)?;
    Ok(())
}

fn select_tools(
    recipe: &RecipeDescriptor,
    readiness: &CookhouseReadiness,
) -> Result<Vec<SelectedTool>, CookhouseError> {
    let mut selected = Vec::new();
    for required in &recipe.tools {
        let mut candidates = readiness
            .tools
            .iter()
            .filter(|tool| &tool.definition_id == required)
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.id.cmp(&right.id));
        let first = candidates
            .first()
            .ok_or_else(|| CookhouseError::MissingTool(required.clone()))?;
        let tool = candidates
            .iter()
            .copied()
            .find(|tool| {
                tool.durability > 0
                    && tool.reservation.is_none()
                    && matches!(
                        &tool.location,
                        LotLocation::StationInput(station) if station == &readiness.station_id
                    )
            })
            .ok_or_else(|| CookhouseError::InvalidTool(first.id.clone()))?;
        selected.push(SelectedTool {
            instance_id: tool.id.clone(),
            definition_id: tool.definition_id.clone(),
            quality: tool.quality,
        });
    }
    Ok(selected)
}

fn select_fixtures(
    recipe: &RecipeDescriptor,
    readiness: &CookhouseReadiness,
) -> Result<Vec<SelectedFixture>, CookhouseError> {
    let mut selected = Vec::new();
    for required in &recipe.fixtures {
        let mut candidates = readiness
            .fixtures
            .iter()
            .filter(|fixture| &fixture.definition_id == required)
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        let first = candidates
            .first()
            .ok_or_else(|| CookhouseError::MissingFixture(required.clone()))?;
        let fixture = candidates
            .iter()
            .copied()
            .find(|fixture| !fixture.reserved && fixture.station_id == readiness.station_id)
            .ok_or_else(|| CookhouseError::InvalidFixture(first.instance_id.clone()))?;
        selected.push(SelectedFixture {
            instance_id: fixture.instance_id.clone(),
            definition_id: fixture.definition_id.clone(),
            quality: fixture.quality,
        });
    }
    Ok(selected)
}

fn select_ingredients(
    station_id: &str,
    input_lots: &[PhysicalLot],
    required: &BTreeMap<ContentId, u32>,
) -> Result<Vec<ReservedIngredient>, CookhouseError> {
    let mut ids = BTreeSet::new();
    for lot in input_lots {
        if !ids.insert(lot.id.clone()) {
            return Err(CookhouseError::DuplicateInputLot(lot.id.clone()));
        }
    }
    let mut result = Vec::new();
    for (content_id, required_units) in required {
        let mut matching = input_lots
            .iter()
            .filter(|lot| &lot.key.content_id == content_id)
            .collect::<Vec<_>>();
        matching.sort_by(|left, right| left.id.cmp(&right.id));
        let mut selected = 0_u32;
        let mut first_reserved = None;
        let mut first_outside = None;
        for lot in matching {
            if selected >= *required_units {
                break;
            }
            if lot.reservation.is_some() {
                first_reserved.get_or_insert_with(|| lot.id.clone());
                continue;
            }
            let eligible = match &lot.location {
                LotLocation::Stockpile(_) => true,
                LotLocation::StationInput(existing) => existing == station_id,
                _ => false,
            };
            if !eligible {
                first_outside.get_or_insert_with(|| lot.id.clone());
                continue;
            }
            let consume = (*required_units - selected).min(lot.quantity);
            if consume == 0 {
                continue;
            }
            selected = selected
                .checked_add(consume)
                .ok_or(CookhouseError::ArithmeticOverflow)?;
            result.push(ReservedIngredient {
                lot_id: lot.id.clone(),
                content_id: lot.key.content_id.clone(),
                quality: lot.key.quality,
                provenance: lot.provenance.clone(),
                original_location: lot.location.clone(),
                reserved_quantity: lot.quantity,
                consumed_quantity: consume,
                state: IngredientState::Reserved,
            });
        }
        if selected != *required_units {
            if selected == 0 {
                if let Some(lot_id) = first_reserved {
                    return Err(CookhouseError::ReservedInput(lot_id));
                }
                if let Some(lot_id) = first_outside {
                    return Err(CookhouseError::LotOutsideEligibleInput(lot_id));
                }
            }
            return Err(CookhouseError::MissingIngredient {
                content_id: content_id.clone(),
                required: *required_units,
                selected,
            });
        }
    }
    result.sort_by(|left, right| left.lot_id.cmp(&right.lot_id));
    Ok(result)
}

fn validate_queue_entry(
    manifest: &ContentManifest,
    entry: &CookhouseQueueEntry,
) -> Result<(), CookhouseError> {
    if !valid_local_id(&entry.entry_id) {
        return Err(CookhouseError::InvalidPersistedState);
    }
    let recipe = recipe_for_execution(manifest, &entry.recipe_id)?;
    if entry.progress_work_units > work_duration(recipe.complexity)? {
        return Err(CookhouseError::InvalidPersistedState);
    }
    Ok(())
}

fn validate_stage(batch: &CookhouseBatch) -> Result<(), CookhouseError> {
    let input_state = match batch.stage {
        BatchStage::Reserved => IngredientState::Reserved,
        BatchStage::InTransit => IngredientState::InTransit,
        BatchStage::Ready | BatchStage::Cooking => IngredientState::StationInput,
        BatchStage::OutputReady | BatchStage::PickedUp => IngredientState::Consumed,
        BatchStage::Cancelled => {
            if batch.recovery_reason.is_none() {
                return Err(CookhouseError::InvalidPersistedState);
            }
            let recovered_inputs = batch
                .reserved_inputs
                .iter()
                .all(|input| input.state == IngredientState::Recovered);
            let consumed_inputs = batch
                .reserved_inputs
                .iter()
                .all(|input| input.state == IngredientState::Consumed);
            let recovered_outputs = batch
                .output_plans
                .iter()
                .all(|output| output.state == OutputState::Recovered);
            let planned_outputs = batch
                .output_plans
                .iter()
                .all(|output| output.state == OutputState::Planned);
            let recovered_before_consumption = recovered_inputs
                && planned_outputs
                && batch.completed_tick.is_none()
                && batch.work_completed < batch.work_required;
            let recovered_after_consumption = consumed_inputs
                && recovered_outputs
                && batch.completed_tick.is_some()
                && batch.work_completed == batch.work_required;
            if !(recovered_before_consumption || recovered_after_consumption) {
                return Err(CookhouseError::InvalidPersistedState);
            }
            return Ok(());
        }
    };
    if batch
        .reserved_inputs
        .iter()
        .any(|input| input.state != input_state)
    {
        return Err(CookhouseError::InvalidPersistedState);
    }
    let expected_output_state = match batch.stage {
        BatchStage::Reserved | BatchStage::InTransit | BatchStage::Ready | BatchStage::Cooking => {
            OutputState::Planned
        }
        BatchStage::OutputReady => OutputState::StationOutput,
        BatchStage::PickedUp => OutputState::Cargo,
        BatchStage::Cancelled => unreachable!("cancelled returned above"),
    };
    if batch
        .output_plans
        .iter()
        .any(|output| output.state != expected_output_state)
    {
        return Err(CookhouseError::InvalidPersistedState);
    }
    match batch.stage {
        BatchStage::Reserved | BatchStage::InTransit | BatchStage::Ready
            if batch.work_completed != 0 || batch.completed_tick.is_some() =>
        {
            Err(CookhouseError::InvalidPersistedState)
        }
        BatchStage::Cooking
            if batch.work_completed == 0
                || batch.work_completed >= batch.work_required
                || batch.completed_tick.is_some() =>
        {
            Err(CookhouseError::InvalidPersistedState)
        }
        BatchStage::OutputReady | BatchStage::PickedUp
            if batch.work_completed != batch.work_required || batch.completed_tick.is_none() =>
        {
            Err(CookhouseError::InvalidPersistedState)
        }
        _ => Ok(()),
    }
}

fn valid_local_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

fn require_capability(
    requirement: &CapabilityRequirement,
    held: &BTreeSet<CapabilityId>,
) -> Result<(), CookhouseError> {
    if let Some(required) = requirement.required_id()
        && !held.contains(required)
    {
        return Err(CookhouseError::MissingCapability(required.clone()));
    }
    Ok(())
}

fn content_capability<'a>(
    manifest: &'a ContentManifest,
    content_id: &ContentId,
) -> Option<&'a CapabilityRequirement> {
    manifest
        .resources
        .iter()
        .find(|entry| &entry.content_id == content_id)
        .map(|entry| &entry.canonical_capability)
        .or_else(|| {
            manifest
                .foods
                .iter()
                .find(|entry| &entry.content_id == content_id)
                .map(|entry| &entry.canonical_capability)
        })
        .or_else(|| {
            manifest
                .item_definitions
                .iter()
                .find(|entry| &entry.content_id == content_id)
                .map(|entry| &entry.canonical_capability)
        })
        .or_else(|| {
            manifest
                .materials
                .iter()
                .find(|entry| &entry.content_id == content_id)
                .map(|entry| &entry.canonical_capability)
        })
}

fn required_inputs(recipe: &RecipeDescriptor) -> Result<BTreeMap<ContentId, u32>, CookhouseError> {
    let mut required = BTreeMap::<ContentId, u32>::new();
    for ingredient in &recipe.ingredients {
        let units = required.entry(ingredient.content_id.clone()).or_default();
        *units = units
            .checked_add(ingredient.units)
            .ok_or(CookhouseError::ArithmeticOverflow)?;
    }
    if recipe.requires_fuel {
        let fuel = ContentId::from_str(FUEL_CONTENT_ID)
            .map_err(|_| CookhouseError::InvalidCookhouseManifest)?;
        let units = required.entry(fuel).or_default();
        *units = units
            .checked_add(1)
            .ok_or(CookhouseError::ArithmeticOverflow)?;
    }
    if recipe.requires_container {
        let container = ContentId::from_str(CONTAINER_CONTENT_ID)
            .map_err(|_| CookhouseError::InvalidCookhouseManifest)?;
        if required.get(&container).copied().unwrap_or(0) == 0 {
            return Err(CookhouseError::InvalidCookhouseManifest);
        }
    }
    Ok(required)
}

fn weighted_input_quality(inputs: &[ReservedIngredient]) -> Result<i32, CookhouseError> {
    let mut weighted = 0_i64;
    let mut units = 0_u64;
    for input in inputs {
        weighted = weighted
            .checked_add(
                i64::from(input.quality.input_quality_milli())
                    .checked_mul(i64::from(input.consumed_quantity))
                    .ok_or(CookhouseError::ArithmeticOverflow)?,
            )
            .ok_or(CookhouseError::ArithmeticOverflow)?;
        units = units
            .checked_add(u64::from(input.consumed_quantity))
            .ok_or(CookhouseError::ArithmeticOverflow)?;
    }
    if units == 0 {
        return Err(CookhouseError::InvalidCookhouseManifest);
    }
    i32::try_from(weighted / i64::try_from(units).map_err(|_| CookhouseError::ArithmeticOverflow)?)
        .map_err(|_| CookhouseError::ArithmeticOverflow)
}

fn production_complexity(value: u8) -> Result<ProductionComplexity, CookhouseError> {
    match value {
        1 => Ok(ProductionComplexity::Raw),
        2 => Ok(ProductionComplexity::Simple),
        3 => Ok(ProductionComplexity::Prepared),
        4 => Ok(ProductionComplexity::Complex),
        5 => Ok(ProductionComplexity::Feast),
        _ => Err(CookhouseError::InvalidCookhouseManifest),
    }
}

fn work_duration(complexity: u8) -> Result<u64, CookhouseError> {
    u64::from(complexity)
        .checked_mul(WORK_UNITS_PER_COMPLEXITY)
        .ok_or(CookhouseError::ArithmeticOverflow)
}

fn stable_output_lot_id(
    batch_id: &str,
    recipe_id: &RecipeId,
    output_index: usize,
) -> Result<PhysicalLotId, CookhouseError> {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let output_index_bytes = output_index.to_le_bytes();
    for bytes in [
        batch_id.as_bytes(),
        recipe_id.as_str().as_bytes(),
        &output_index_bytes,
    ] {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    PhysicalLotId::from_str(&format!("lot_cook_{hash:016x}_{output_index}"))
        .map_err(|_| CookhouseError::InvalidPersistedState)
}
