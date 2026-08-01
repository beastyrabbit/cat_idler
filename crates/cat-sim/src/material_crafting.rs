//! LAI.43 versioned material-processing, crafting, augmentation, and fixture authority.
//!
//! This pure authority consumes the canonical LAI.36 manifest and the LAI.37
//! physical inventory. Every mutation is checked, staged, and committed through
//! one bounded idempotent command lane.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{
        AugmentationDescriptor, CapabilityId, CapabilityRequirement, ContentId, ContentManifest,
        EffectOperation, FixtureDescriptor, ItemDefinitionDescriptor, ItemDefinitionId,
        MaterialDescriptor, MaterialId, MaterialInstanceId, MaterialUseDescriptor,
        PLAN1_RARE_MATERIAL_IDS, PhysicalLotId, RecipeDescriptor, RecipeId, StationDescriptor,
    },
    quality_lots::{
        ExactItemPayload, FixtureInstance, ItemAugmentation, ItemInstance, LotLocation,
        LotProvenance, PhysicalLot, ProductionComplexity, ProductionQualityInput, QualityBand,
        QualityLotLedger, StationFixture, production_quality_score, quality_from_score,
    },
};

pub const MATERIAL_CRAFTING_SCHEMA_VERSION: u32 = 2;
pub const MAX_NAMED_MATERIALS: usize = 1_024;
pub const MAX_FIXTURE_TARGETS: usize = 256;
pub const MAX_MATERIAL_RECEIPTS: usize = 1_024;
const MAX_COMMAND_ID_BYTES: usize = 128;
const MAX_DURABILITY: u32 = 100;
const PLANK_PROCESSING_CAPABILITY: &str = "plank_processing";
const LOGS_TO_PLANKS_RECIPE: &str = "logs_to_planks";
const LOGS_CONTENT: &str = "resource_logs";
const PLANKS_CONTENT: &str = "resource_planks";
const PLAN1_STATION_IDS: [&str; 15] = [
    "black_hole",
    "mill",
    "cookhouse",
    "fishing_hut",
    "workshop",
    "tannery",
    "clothier",
    "woodworking",
    "smithy",
    "research_hut",
    "school",
    "sawmill",
    "smelter",
    "wood_cutter",
    "stone_prep",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialCraftingError {
    ManifestInvalid(String),
    MaterialCatalogMismatch,
    StationCatalogMismatch,
    DuplicateIdentity(MaterialInstanceId),
    DuplicateTarget(String),
    MissingMaterial(MaterialId),
    MissingMaterialInstance(MaterialInstanceId),
    MissingStation(ItemDefinitionId),
    MissingItemDefinition(ItemDefinitionId),
    MissingOutputDefinition(ContentId),
    MissingAugmentation(ItemDefinitionId),
    MissingFixture(ItemDefinitionId),
    MissingFixtureTarget(String),
    MissingRecipe(RecipeId),
    MissingCapability(CapabilityId),
    MissingUse {
        material_id: MaterialId,
        station_id: ItemDefinitionId,
        operation: EffectOperation,
    },
    WrongMaterialState,
    ReservedMaterial(MaterialInstanceId),
    CarriedMaterial(MaterialInstanceId),
    IncompatibleMaterial(MaterialId),
    IncompatibleItem(ItemDefinitionId),
    IncompatibleFixture(ItemDefinitionId),
    OccupiedSlot,
    InvalidDurabilityAmount,
    InvalidSchemaVersion(u32),
    InvalidCommandId,
    VersionConflict {
        expected: u64,
        actual: u64,
    },
    CommandConflict(String),
    InventoryLimitExceeded,
    ReceiptLimitExceeded,
    NonCanonicalState,
    EmptyIdentity,
    ArithmeticOverflow,
    Ledger(String),
}

impl fmt::Display for MaterialCraftingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid material-crafting operation: {self:?}")
    }
}

impl std::error::Error for MaterialCraftingError {}

#[derive(Debug, Clone)]
pub struct MaterialCraftingCatalog<'a> {
    manifest: &'a ContentManifest,
    materials: BTreeMap<MaterialId, usize>,
    stations: BTreeMap<ItemDefinitionId, usize>,
    items_by_id: BTreeMap<ItemDefinitionId, usize>,
    items_by_content: BTreeMap<ContentId, usize>,
    augmentations: BTreeMap<ItemDefinitionId, usize>,
    fixtures: BTreeMap<ItemDefinitionId, usize>,
    recipes: BTreeMap<RecipeId, usize>,
}

impl<'a> MaterialCraftingCatalog<'a> {
    pub fn from_manifest(manifest: &'a ContentManifest) -> Result<Self, MaterialCraftingError> {
        manifest
            .validate()
            .map_err(|errors| MaterialCraftingError::ManifestInvalid(format!("{errors:?}")))?;
        validate_exact_materials(manifest)?;
        validate_station_scope(manifest)?;
        Ok(Self {
            manifest,
            materials: manifest
                .materials
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
            stations: manifest
                .stations
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
            items_by_id: manifest
                .item_definitions
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
            items_by_content: manifest
                .item_definitions
                .iter()
                .enumerate()
                .map(|(index, value)| (value.content_id.clone(), index))
                .collect(),
            augmentations: manifest
                .augmentations
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
            fixtures: manifest
                .fixtures
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
            recipes: manifest
                .recipes
                .iter()
                .enumerate()
                .map(|(index, value)| (value.id.clone(), index))
                .collect(),
        })
    }

    #[must_use]
    pub fn embedded() -> Self {
        Self::from_manifest(ContentManifest::embedded())
            .expect("embedded manifest must be a valid LAI.43 catalog")
    }

    #[must_use]
    pub fn materials(&self) -> &[MaterialDescriptor] {
        &self.manifest.materials
    }

    pub fn material(&self, id: &MaterialId) -> Result<&MaterialDescriptor, MaterialCraftingError> {
        self.materials
            .get(id)
            .and_then(|index| self.manifest.materials.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingMaterial(id.clone()))
    }

    pub fn station(
        &self,
        id: &ItemDefinitionId,
    ) -> Result<&StationDescriptor, MaterialCraftingError> {
        self.stations
            .get(id)
            .and_then(|index| self.manifest.stations.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingStation(id.clone()))
    }

    pub fn item_definition(
        &self,
        id: &ItemDefinitionId,
    ) -> Result<&ItemDefinitionDescriptor, MaterialCraftingError> {
        self.items_by_id
            .get(id)
            .and_then(|index| self.manifest.item_definitions.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingItemDefinition(id.clone()))
    }

    pub fn item_definition_for_content(
        &self,
        id: &ContentId,
    ) -> Result<&ItemDefinitionDescriptor, MaterialCraftingError> {
        self.items_by_content
            .get(id)
            .and_then(|index| self.manifest.item_definitions.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingOutputDefinition(id.clone()))
    }

    pub fn augmentation(
        &self,
        id: &ItemDefinitionId,
    ) -> Result<&AugmentationDescriptor, MaterialCraftingError> {
        self.augmentations
            .get(id)
            .and_then(|index| self.manifest.augmentations.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingAugmentation(id.clone()))
    }

    pub fn fixture(
        &self,
        id: &ItemDefinitionId,
    ) -> Result<&FixtureDescriptor, MaterialCraftingError> {
        self.fixtures
            .get(id)
            .and_then(|index| self.manifest.fixtures.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingFixture(id.clone()))
    }

    pub fn recipe(&self, id: &RecipeId) -> Result<&RecipeDescriptor, MaterialCraftingError> {
        self.recipes
            .get(id)
            .and_then(|index| self.manifest.recipes.get(*index))
            .ok_or_else(|| MaterialCraftingError::MissingRecipe(id.clone()))
    }
}

fn validate_exact_materials(manifest: &ContentManifest) -> Result<(), MaterialCraftingError> {
    if manifest.materials.len() != PLAN1_RARE_MATERIAL_IDS.len()
        || !PLAN1_RARE_MATERIAL_IDS
            .iter()
            .zip(&manifest.materials)
            .all(|(expected, actual)| {
                *expected == actual.id.as_str()
                    && actual.raw_state != actual.processed_state
                    && !actual.tags.is_empty()
                    && !actual.uses.is_empty()
                    && actual.hole_darkness_gate <= 10
                    && actual.hole_value_milli > 0
                    && actual.canonical_capability.required_id().is_some()
                    && actual.behavior_handler == "process_material"
            })
    {
        return Err(MaterialCraftingError::MaterialCatalogMismatch);
    }
    Ok(())
}

fn validate_station_scope(manifest: &ContentManifest) -> Result<(), MaterialCraftingError> {
    if manifest.stations.len() != PLAN1_STATION_IDS.len()
        || !PLAN1_STATION_IDS
            .iter()
            .zip(&manifest.stations)
            .all(|(expected, actual)| *expected == actual.id.as_str())
    {
        return Err(MaterialCraftingError::StationCatalogMismatch);
    }
    if manifest
        .stations
        .iter()
        .filter(|station| station.id.as_str().contains("cloth"))
        .count()
        != 1
    {
        return Err(MaterialCraftingError::StationCatalogMismatch);
    }
    for station_id in ["cookhouse", "fishing_hut"] {
        let station = manifest
            .stations
            .iter()
            .find(|candidate| candidate.id.as_str() == station_id)
            .ok_or(MaterialCraftingError::StationCatalogMismatch)?;
        if station.footprint_cells != 9 {
            return Err(MaterialCraftingError::StationCatalogMismatch);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilitySet {
    owned: BTreeSet<CapabilityId>,
}

impl CapabilitySet {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_owned(owned: impl IntoIterator<Item = CapabilityId>) -> Self {
        Self {
            owned: owned.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn owns(&self, capability: &CapabilityId) -> bool {
        self.owned.contains(capability)
    }

    fn require(&self, requirement: &CapabilityRequirement) -> Result<(), MaterialCraftingError> {
        match requirement {
            CapabilityRequirement::Free => Ok(()),
            CapabilityRequirement::Required(id) if self.owns(id) => Ok(()),
            CapabilityRequirement::Required(id) => {
                Err(MaterialCraftingError::MissingCapability(id.clone()))
            }
        }
    }

    fn require_id(&self, id: &CapabilityId) -> Result<(), MaterialCraftingError> {
        if self.owns(id) {
            Ok(())
        } else {
            Err(MaterialCraftingError::MissingCapability(id.clone()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialProcessingState {
    Raw,
    Processed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedMaterialInstance {
    pub instance_id: MaterialInstanceId,
    pub material_id: MaterialId,
    pub state: MaterialProcessingState,
    pub content_state: ContentId,
    pub quality: QualityBand,
    pub provenance: LotProvenance,
    pub location: LotLocation,
    pub reservation: Option<String>,
}

impl NamedMaterialInstance {
    pub fn raw_from_hunt(
        catalog: &MaterialCraftingCatalog<'_>,
        instance_id: MaterialInstanceId,
        material_id: MaterialId,
        quality: QualityBand,
        provenance: LotProvenance,
        location: LotLocation,
        reservation: Option<String>,
    ) -> Result<Self, MaterialCraftingError> {
        let descriptor = catalog.material(&material_id)?;
        let instance = Self {
            instance_id,
            material_id,
            state: MaterialProcessingState::Raw,
            content_state: descriptor.raw_state.clone(),
            quality,
            provenance,
            location,
            reservation,
        };
        instance.validate_against(catalog)?;
        Ok(instance)
    }

    fn validate_against(
        &self,
        catalog: &MaterialCraftingCatalog<'_>,
    ) -> Result<(), MaterialCraftingError> {
        let descriptor = catalog.material(&self.material_id)?;
        let expected = match self.state {
            MaterialProcessingState::Raw => &descriptor.raw_state,
            MaterialProcessingState::Processed => &descriptor.processed_state,
        };
        if &self.content_state != expected {
            return Err(MaterialCraftingError::WrongMaterialState);
        }
        if self.provenance.origin.trim().is_empty()
            || self
                .reservation
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(MaterialCraftingError::EmptyIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionContext {
    pub world_seed: u32,
    pub station_tier: u8,
    pub worker_skill: u8,
    pub tool_quality: Option<QualityBand>,
    pub fixture_quality: Option<QualityBand>,
    pub completion_index: u64,
    pub destination: LotLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixtureTarget {
    pub target_id: String,
    pub station_id: ItemDefinitionId,
    pub fixture: FixtureInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialRecoveryReason {
    Cancelled,
    CarrierDeath,
    RouteLost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "id",
    deny_unknown_fields
)]
pub enum RecoveryDestination {
    Origin,
    Stockpile(String),
    Cache(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "value",
    deny_unknown_fields
)]
pub enum DurabilityTarget {
    Inventory(MaterialInstanceId),
    InstalledAugmentation(MaterialInstanceId),
    InstalledFixture(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum MaterialCommandOperation {
    Process {
        material_instance_id: MaterialInstanceId,
        station_id: ItemDefinitionId,
    },
    CraftItem {
        material_instance_id: MaterialInstanceId,
        station_id: ItemDefinitionId,
        output_content_id: ContentId,
        context: ProductionContext,
    },
    CraftAugmentation {
        material_instance_id: MaterialInstanceId,
        augmentation_id: ItemDefinitionId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    },
    CraftFixture {
        material_instance_id: MaterialInstanceId,
        fixture_id: ItemDefinitionId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    },
    InstallAugmentation {
        target_item_id: MaterialInstanceId,
        augmentation_item_id: MaterialInstanceId,
    },
    RemoveAugmentation {
        target_item_id: MaterialInstanceId,
        destination: RecoveryDestination,
    },
    InstallFixture {
        target_id: String,
        fixture_item_id: MaterialInstanceId,
    },
    RemoveFixture {
        target_id: String,
        destination: RecoveryDestination,
    },
    RecoverMaterial {
        material_instance_id: MaterialInstanceId,
        reason: MaterialRecoveryReason,
        destination: RecoveryDestination,
    },
    RecoverItem {
        item_id: MaterialInstanceId,
        reason: MaterialRecoveryReason,
        destination: RecoveryDestination,
    },
    Wear {
        target: DurabilityTarget,
        amount: u32,
    },
    LogsToPlanks {
        input_lot_id: PhysicalLotId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialCommand {
    pub command_id: String,
    pub expected_version: u64,
    pub operation: MaterialCommandOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum MaterialCommandResult {
    Processed {
        material_instance_id: MaterialInstanceId,
    },
    Crafted {
        consumed_material_id: MaterialInstanceId,
        produced_item_id: MaterialInstanceId,
    },
    AugmentationInstalled {
        target_item_id: MaterialInstanceId,
        augmentation_item_id: MaterialInstanceId,
    },
    AugmentationRemoved {
        target_item_id: MaterialInstanceId,
        augmentation_item_id: MaterialInstanceId,
    },
    FixtureInstalled {
        target_id: String,
        fixture_item_id: MaterialInstanceId,
    },
    FixtureRemoved {
        target_id: String,
        fixture_item_id: MaterialInstanceId,
    },
    Recovered {
        instance_id: MaterialInstanceId,
    },
    Worn {
        instance_id: MaterialInstanceId,
        durability: u32,
    },
    PlanksProduced {
        consumed_lot_id: PhysicalLotId,
        produced_lot_id: PhysicalLotId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialCommandReceipt {
    pub command_id: String,
    pub fingerprint: u64,
    pub resulting_version: u64,
    pub result: MaterialCommandResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OriginRecord {
    instance_id: MaterialInstanceId,
    location: LotLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialCraftingAuthority {
    version: u64,
    capabilities: CapabilitySet,
    materials: BTreeMap<MaterialInstanceId, NamedMaterialInstance>,
    ledger: QualityLotLedger,
    fixture_targets: BTreeMap<String, FixtureTarget>,
    origins: BTreeMap<MaterialInstanceId, LotLocation>,
    receipts: BTreeMap<String, MaterialCommandReceipt>,
}

impl MaterialCraftingAuthority {
    pub fn new(
        catalog: &MaterialCraftingCatalog<'_>,
        capabilities: CapabilitySet,
        materials: Vec<NamedMaterialInstance>,
        ledger: QualityLotLedger,
        fixture_targets: Vec<FixtureTarget>,
    ) -> Result<Self, MaterialCraftingError> {
        if materials.len() > MAX_NAMED_MATERIALS || fixture_targets.len() > MAX_FIXTURE_TARGETS {
            return Err(MaterialCraftingError::InventoryLimitExceeded);
        }
        let mut material_index = BTreeMap::new();
        let mut origins = BTreeMap::new();
        for material in materials {
            material.validate_against(catalog)?;
            let id = material.instance_id.clone();
            if material_index
                .insert(id.clone(), material.clone())
                .is_some()
            {
                return Err(MaterialCraftingError::DuplicateIdentity(id));
            }
            origins.insert(id, material.location);
        }
        for item in ledger.items() {
            if material_index.contains_key(&item.id)
                || origins
                    .insert(item.id.clone(), item.location.clone())
                    .is_some()
            {
                return Err(MaterialCraftingError::DuplicateIdentity(item.id.clone()));
            }
            if let Some(augmentation) = &item.augmentation {
                let id = augmentation.item.id.clone();
                if material_index.contains_key(&id)
                    || origins
                        .insert(id.clone(), augmentation.item.location.clone())
                        .is_some()
                {
                    return Err(MaterialCraftingError::DuplicateIdentity(id));
                }
            }
        }
        let mut targets = BTreeMap::new();
        for target in fixture_targets {
            validate_target(catalog, &target)?;
            if target.target_id.trim().is_empty()
                || targets
                    .insert(target.target_id.clone(), target.clone())
                    .is_some()
            {
                return Err(MaterialCraftingError::DuplicateTarget(target.target_id));
            }
            if let Some(installed) = &target.fixture.installed {
                let id = installed.item.id.clone();
                if material_index.contains_key(&id)
                    || origins
                        .insert(id.clone(), installed.item.location.clone())
                        .is_some()
                {
                    return Err(MaterialCraftingError::DuplicateIdentity(id));
                }
            }
        }
        let authority = Self {
            version: 0,
            capabilities,
            materials: material_index,
            ledger,
            fixture_targets: targets,
            origins,
            receipts: BTreeMap::new(),
        };
        authority.validate(catalog)?;
        Ok(authority)
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn material(&self, id: &MaterialInstanceId) -> Option<&NamedMaterialInstance> {
        self.materials.get(id)
    }

    #[must_use]
    pub const fn ledger(&self) -> &QualityLotLedger {
        &self.ledger
    }

    #[must_use]
    pub fn fixture_target(&self, id: &str) -> Option<&FixtureTarget> {
        self.fixture_targets.get(id)
    }

    #[must_use]
    pub fn receipts(&self) -> impl ExactSizeIterator<Item = &MaterialCommandReceipt> {
        self.receipts.values()
    }

    pub fn execute(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command: MaterialCommand,
    ) -> Result<MaterialCommandReceipt, MaterialCraftingError> {
        validate_command_id(&command.command_id)?;
        let fingerprint = command_fingerprint(&command)?;
        if let Some(receipt) = self.receipts.get(&command.command_id) {
            return if receipt.fingerprint == fingerprint {
                Ok(receipt.clone())
            } else {
                Err(MaterialCraftingError::CommandConflict(command.command_id))
            };
        }
        if command.expected_version != self.version {
            return Err(MaterialCraftingError::VersionConflict {
                expected: command.expected_version,
                actual: self.version,
            });
        }
        if self.receipts.len() >= MAX_MATERIAL_RECEIPTS {
            return Err(MaterialCraftingError::ReceiptLimitExceeded);
        }

        let mut staged = self.clone();
        let result = staged.apply(catalog, &command.command_id, command.operation)?;
        staged.version = staged
            .version
            .checked_add(1)
            .ok_or(MaterialCraftingError::ArithmeticOverflow)?;
        let receipt = MaterialCommandReceipt {
            command_id: command.command_id.clone(),
            fingerprint,
            resulting_version: staged.version,
            result,
        };
        staged.receipts.insert(command.command_id, receipt.clone());
        staged.validate(catalog)?;
        *self = staged;
        Ok(receipt)
    }

    fn apply(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command_id: &str,
        operation: MaterialCommandOperation,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        match operation {
            MaterialCommandOperation::Process {
                material_instance_id,
                station_id,
            } => self.process(catalog, material_instance_id, station_id),
            MaterialCommandOperation::CraftItem {
                material_instance_id,
                station_id,
                output_content_id,
                context,
            } => self.craft_item(
                catalog,
                command_id,
                material_instance_id,
                station_id,
                output_content_id,
                context,
            ),
            MaterialCommandOperation::CraftAugmentation {
                material_instance_id,
                augmentation_id,
                station_id,
                context,
            } => self.craft_augmentation(
                catalog,
                command_id,
                material_instance_id,
                augmentation_id,
                station_id,
                context,
            ),
            MaterialCommandOperation::CraftFixture {
                material_instance_id,
                fixture_id,
                station_id,
                context,
            } => self.craft_fixture(
                catalog,
                command_id,
                material_instance_id,
                fixture_id,
                station_id,
                context,
            ),
            MaterialCommandOperation::InstallAugmentation {
                target_item_id,
                augmentation_item_id,
            } => self.install_augmentation(catalog, target_item_id, augmentation_item_id),
            MaterialCommandOperation::RemoveAugmentation {
                target_item_id,
                destination,
            } => self.remove_augmentation(target_item_id, destination),
            MaterialCommandOperation::InstallFixture {
                target_id,
                fixture_item_id,
            } => self.install_fixture(catalog, target_id, fixture_item_id),
            MaterialCommandOperation::RemoveFixture {
                target_id,
                destination,
            } => self.remove_fixture(target_id, destination),
            MaterialCommandOperation::RecoverMaterial {
                material_instance_id,
                reason: _,
                destination,
            } => self.recover_material(material_instance_id, destination),
            MaterialCommandOperation::RecoverItem {
                item_id,
                reason: _,
                destination,
            } => self.recover_item(item_id, destination),
            MaterialCommandOperation::Wear { target, amount } => self.wear(target, amount),
            MaterialCommandOperation::LogsToPlanks {
                input_lot_id,
                station_id,
                context,
            } => self.logs_to_planks(catalog, command_id, input_lot_id, station_id, context),
        }
    }

    fn process(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        id: MaterialInstanceId,
        station_id: ItemDefinitionId,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let material = self
            .materials
            .get(&id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::MissingMaterialInstance(id.clone()))?;
        material.validate_against(catalog)?;
        require_available_material(&material, MaterialProcessingState::Raw)?;
        let descriptor = catalog.material(&material.material_id)?;
        let station = catalog.station(&station_id)?;
        self.capabilities
            .require(&descriptor.canonical_capability)?;
        self.capabilities.require(&station.canonical_capability)?;
        find_exact_use(descriptor, &station_id, EffectOperation::Process, None)?;

        let stored = self
            .materials
            .get_mut(&id)
            .expect("material was checked above");
        stored.state = MaterialProcessingState::Processed;
        stored.content_state = descriptor.processed_state.clone();
        stored.location = LotLocation::StationOutput(station_id.as_str().to_owned());
        Ok(MaterialCommandResult::Processed {
            material_instance_id: id,
        })
    }

    fn craft_item(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command_id: &str,
        material_id: MaterialInstanceId,
        station_id: ItemDefinitionId,
        output_content_id: ContentId,
        context: ProductionContext,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let material = self.checked_processed_material(catalog, &material_id)?;
        let descriptor = catalog.material(&material.material_id)?;
        let station = catalog.station(&station_id)?;
        self.capabilities
            .require(&descriptor.canonical_capability)?;
        self.capabilities.require(&station.canonical_capability)?;
        find_exact_use(
            descriptor,
            &station_id,
            EffectOperation::Craft,
            Some(&output_content_id),
        )?;
        let definition = catalog.item_definition_for_content(&output_content_id)?;
        self.capabilities
            .require(&definition.canonical_capability)?;
        if definition.base_materials.is_empty()
            || !definition.base_materials.contains(&material.material_id)
        {
            return Err(MaterialCraftingError::IncompatibleMaterial(
                material.material_id,
            ));
        }
        let item = produced_item(
            command_id,
            &material,
            definition.id.clone(),
            definition.augmentation_slot,
            &output_content_id,
            &context,
        )?;
        self.commit_material_to_item(material_id.clone(), item.clone())?;
        Ok(MaterialCommandResult::Crafted {
            consumed_material_id: material_id,
            produced_item_id: item.id,
        })
    }

    fn craft_augmentation(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command_id: &str,
        material_id: MaterialInstanceId,
        augmentation_id: ItemDefinitionId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let material = self.checked_processed_material(catalog, &material_id)?;
        let descriptor = catalog.material(&material.material_id)?;
        let station = catalog.station(&station_id)?;
        let augmentation = catalog.augmentation(&augmentation_id)?;
        self.capabilities
            .require(&descriptor.canonical_capability)?;
        self.capabilities.require(&station.canonical_capability)?;
        self.capabilities
            .require(&augmentation.canonical_capability)?;
        if augmentation.consumed_materials != vec![material.material_id.clone()] {
            return Err(MaterialCraftingError::IncompatibleMaterial(
                material.material_id,
            ));
        }
        find_exact_use(
            descriptor,
            &station_id,
            EffectOperation::Augment,
            Some(&augmentation.content_id),
        )?;
        let item = produced_item(
            command_id,
            &material,
            augmentation.id.clone(),
            Some(augmentation.slot),
            &augmentation.content_id,
            &context,
        )?;
        self.commit_material_to_item(material_id.clone(), item.clone())?;
        Ok(MaterialCommandResult::Crafted {
            consumed_material_id: material_id,
            produced_item_id: item.id,
        })
    }

    fn craft_fixture(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command_id: &str,
        material_id: MaterialInstanceId,
        fixture_id: ItemDefinitionId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let material = self.checked_processed_material(catalog, &material_id)?;
        let descriptor = catalog.material(&material.material_id)?;
        let station = catalog.station(&station_id)?;
        let fixture = catalog.fixture(&fixture_id)?;
        self.capabilities
            .require(&descriptor.canonical_capability)?;
        self.capabilities.require(&station.canonical_capability)?;
        self.capabilities.require(&fixture.canonical_capability)?;
        if fixture.consumed_materials != vec![material.material_id.clone()] {
            return Err(MaterialCraftingError::IncompatibleMaterial(
                material.material_id,
            ));
        }
        find_exact_use(
            descriptor,
            &station_id,
            fixture.effect_operation,
            Some(&fixture.content_id),
        )?;
        let item = produced_item(
            command_id,
            &material,
            fixture.id.clone(),
            None,
            &fixture.content_id,
            &context,
        )?;
        self.commit_material_to_item(material_id.clone(), item.clone())?;
        Ok(MaterialCommandResult::Crafted {
            consumed_material_id: material_id,
            produced_item_id: item.id,
        })
    }

    fn checked_processed_material(
        &self,
        catalog: &MaterialCraftingCatalog<'_>,
        id: &MaterialInstanceId,
    ) -> Result<NamedMaterialInstance, MaterialCraftingError> {
        let material = self
            .materials
            .get(id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::MissingMaterialInstance(id.clone()))?;
        material.validate_against(catalog)?;
        require_available_material(&material, MaterialProcessingState::Processed)?;
        Ok(material)
    }

    fn commit_material_to_item(
        &mut self,
        material_id: MaterialInstanceId,
        item: ItemInstance,
    ) -> Result<(), MaterialCraftingError> {
        self.ledger
            .insert_item(item.clone())
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        self.materials.remove(&material_id);
        self.origins.remove(&material_id);
        self.origins.insert(item.id.clone(), item.location);
        Ok(())
    }

    fn install_augmentation(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        target_id: MaterialInstanceId,
        augmentation_id: MaterialInstanceId,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let mut target = self
            .ledger
            .item(&target_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::Ledger("missing target item".to_owned()))?;
        let mut augmentation_item =
            self.ledger.item(&augmentation_id).cloned().ok_or_else(|| {
                MaterialCraftingError::Ledger("missing augmentation item".to_owned())
            })?;
        if target.reservation.is_some()
            || target.equipment_slot.is_some()
            || matches!(target.location, LotLocation::Cargo(_))
            || target.durability == 0
            || target.augmentation.is_some()
        {
            return Err(MaterialCraftingError::IncompatibleItem(
                target.definition_id,
            ));
        }
        if augmentation_item.reservation.is_some()
            || augmentation_item.equipment_slot.is_some()
            || matches!(augmentation_item.location, LotLocation::Cargo(_))
            || augmentation_item.durability == 0
            || augmentation_item.augmentation.is_some()
        {
            return Err(MaterialCraftingError::IncompatibleItem(
                augmentation_item.definition_id,
            ));
        }
        let augmentation = catalog.augmentation(&augmentation_item.definition_id)?;
        let target_definition = catalog.item_definition(&target.definition_id)?;
        self.capabilities
            .require(&augmentation.canonical_capability)?;
        if augmentation.consumed_materials != vec![augmentation_item.material_id.clone()]
            || augmentation.effect_operation != EffectOperation::Augment
            || augmentation_item.augmentation_slot != Some(augmentation.slot)
            || target_definition.augmentation_slot != Some(augmentation.slot)
            || !augmentation
                .compatible_item_classes
                .contains(&target_definition.class)
        {
            return Err(MaterialCraftingError::IncompatibleItem(
                target.definition_id,
            ));
        }
        augmentation_item.location = LotLocation::StationInput(target_id.as_str().to_owned());
        let payload = ExactItemPayload::from_unaugmented_item(augmentation_item)
            .ok_or_else(|| MaterialCraftingError::IncompatibleItem(target.definition_id.clone()))?;
        target.augmentation = Some(ItemAugmentation {
            item: payload,
            slot: augmentation.slot,
        });
        self.ledger
            .replace_item(target)
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        self.ledger
            .remove_item(&augmentation_id)
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        Ok(MaterialCommandResult::AugmentationInstalled {
            target_item_id: target_id,
            augmentation_item_id: augmentation_id,
        })
    }

    fn remove_augmentation(
        &mut self,
        target_id: MaterialInstanceId,
        destination: RecoveryDestination,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let mut target = self
            .ledger
            .item(&target_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::Ledger("missing target item".to_owned()))?;
        let installed = target
            .augmentation
            .take()
            .ok_or_else(|| MaterialCraftingError::IncompatibleItem(target.definition_id.clone()))?;
        let installed_id = installed.item.id.clone();
        let location = self.resolve_destination(&installed_id, destination)?;
        let mut item = installed.item.into_item();
        item.location = location;
        item.reservation = None;
        self.ledger
            .replace_item(target)
            .and_then(|()| self.ledger.insert_item(item))
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        Ok(MaterialCommandResult::AugmentationRemoved {
            target_item_id: target_id,
            augmentation_item_id: installed_id,
        })
    }

    fn install_fixture(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        target_id: String,
        fixture_item_id: MaterialInstanceId,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let target = self
            .fixture_targets
            .get(&target_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::MissingFixtureTarget(target_id.clone()))?;
        let station = catalog.station(&target.station_id)?;
        let mut fixture_item = self
            .ledger
            .item(&fixture_item_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::Ledger("missing fixture item".to_owned()))?;
        let fixture = catalog.fixture(&fixture_item.definition_id)?;
        self.capabilities.require(&station.canonical_capability)?;
        self.capabilities.require(&fixture.canonical_capability)?;
        if target.fixture.reserved
            || target.fixture.installed.is_some()
            || station.fixture_slot != Some(target.fixture.slot)
        {
            return Err(MaterialCraftingError::OccupiedSlot);
        }
        if fixture_item.reservation.is_some()
            || fixture_item.equipment_slot.is_some()
            || fixture_item.augmentation.is_some()
            || fixture_item.durability == 0
            || matches!(fixture_item.location, LotLocation::Cargo(_))
            || fixture.slot != target.fixture.slot
            || !fixture.compatible_stations.contains(&target.station_id)
            || fixture.consumed_materials != vec![fixture_item.material_id.clone()]
            || fixture.effect_operation != EffectOperation::InstallFixture
        {
            return Err(MaterialCraftingError::IncompatibleFixture(
                fixture_item.definition_id,
            ));
        }
        fixture_item.location = LotLocation::StationInput(target_id.clone());
        let payload = ExactItemPayload::from_unaugmented_item(fixture_item)
            .ok_or_else(|| MaterialCraftingError::IncompatibleFixture(fixture.id.clone()))?;
        let mut fixture_state = target.fixture;
        fixture_state
            .install_fixture(StationFixture {
                item: payload,
                slot: fixture.slot,
            })
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        self.fixture_targets
            .get_mut(&target_id)
            .expect("fixture target was checked above")
            .fixture = fixture_state;
        self.ledger
            .remove_item(&fixture_item_id)
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        Ok(MaterialCommandResult::FixtureInstalled {
            target_id,
            fixture_item_id,
        })
    }

    fn remove_fixture(
        &mut self,
        target_id: String,
        destination: RecoveryDestination,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let target = self
            .fixture_targets
            .get(&target_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::MissingFixtureTarget(target_id.clone()))?;
        let installed = target
            .fixture
            .installed
            .ok_or_else(|| MaterialCraftingError::MissingFixtureTarget(target_id.clone()))?;
        let fixture_item_id = installed.item.id.clone();
        let location = self.resolve_destination(&fixture_item_id, destination)?;
        let mut item = installed.item.into_item();
        item.location = location;
        item.reservation = None;
        self.ledger
            .insert_item(item)
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        self.fixture_targets
            .get_mut(&target_id)
            .expect("fixture target was checked above")
            .fixture
            .installed = None;
        Ok(MaterialCommandResult::FixtureRemoved {
            target_id,
            fixture_item_id,
        })
    }

    fn recover_material(
        &mut self,
        id: MaterialInstanceId,
        destination: RecoveryDestination,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let location = self.resolve_destination(&id, destination)?;
        let material = self
            .materials
            .get_mut(&id)
            .ok_or_else(|| MaterialCraftingError::MissingMaterialInstance(id.clone()))?;
        material.location = location;
        material.reservation = None;
        Ok(MaterialCommandResult::Recovered { instance_id: id })
    }

    fn recover_item(
        &mut self,
        id: MaterialInstanceId,
        destination: RecoveryDestination,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let location = self.resolve_destination(&id, destination)?;
        let mut item = self
            .ledger
            .item(&id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::Ledger("missing recovery item".to_owned()))?;
        item.location = location;
        item.reservation = None;
        self.ledger
            .replace_item(item)
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        Ok(MaterialCommandResult::Recovered { instance_id: id })
    }

    fn wear(
        &mut self,
        target: DurabilityTarget,
        amount: u32,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        if amount == 0 {
            return Err(MaterialCraftingError::InvalidDurabilityAmount);
        }
        let (id, durability) = match target {
            DurabilityTarget::Inventory(id) => {
                let mut item =
                    self.ledger.item(&id).cloned().ok_or_else(|| {
                        MaterialCraftingError::Ledger("missing worn item".to_owned())
                    })?;
                item.durability = item.durability.saturating_sub(amount);
                let remaining = item.durability;
                self.ledger
                    .replace_item(item)
                    .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
                (id, remaining)
            }
            DurabilityTarget::InstalledAugmentation(target_id) => {
                let mut item = self.ledger.item(&target_id).cloned().ok_or_else(|| {
                    MaterialCraftingError::Ledger("missing augmented item".to_owned())
                })?;
                let augmentation = item.augmentation.as_mut().ok_or_else(|| {
                    MaterialCraftingError::IncompatibleItem(item.definition_id.clone())
                })?;
                augmentation.item.durability = augmentation.item.durability.saturating_sub(amount);
                let installed_id = augmentation.item.id.clone();
                let remaining = augmentation.item.durability;
                self.ledger
                    .replace_item(item)
                    .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
                (installed_id, remaining)
            }
            DurabilityTarget::InstalledFixture(target_id) => {
                let target = self.fixture_targets.get_mut(&target_id).ok_or_else(|| {
                    MaterialCraftingError::MissingFixtureTarget(target_id.clone())
                })?;
                let installed = target
                    .fixture
                    .installed
                    .as_mut()
                    .ok_or_else(|| MaterialCraftingError::MissingFixtureTarget(target_id))?;
                installed.item.durability = installed.item.durability.saturating_sub(amount);
                (installed.item.id.clone(), installed.item.durability)
            }
        };
        Ok(MaterialCommandResult::Worn {
            instance_id: id,
            durability,
        })
    }

    fn logs_to_planks(
        &mut self,
        catalog: &MaterialCraftingCatalog<'_>,
        command_id: &str,
        input_lot_id: PhysicalLotId,
        station_id: ItemDefinitionId,
        context: ProductionContext,
    ) -> Result<MaterialCommandResult, MaterialCraftingError> {
        let recipe_id = RecipeId::new(LOGS_TO_PLANKS_RECIPE)
            .map_err(|_| MaterialCraftingError::EmptyIdentity)?;
        let recipe = catalog.recipe(&recipe_id)?;
        let station = catalog.station(&station_id)?;
        let plank_capability = CapabilityId::new(PLANK_PROCESSING_CAPABILITY)
            .map_err(|_| MaterialCraftingError::EmptyIdentity)?;
        self.capabilities.require_id(&plank_capability)?;
        self.capabilities.require(&station.canonical_capability)?;
        self.capabilities.require(&recipe.canonical_capability)?;
        self.capabilities.require_id(&recipe.bundle_capability)?;
        if recipe.station != station_id
            || recipe.ingredients.len() != 1
            || recipe.ingredients[0].content_id.as_str() != LOGS_CONTENT
            || recipe.ingredients[0].units != 1
            || recipe.outputs.len() != 1
            || recipe.outputs[0].content_id.as_str() != PLANKS_CONTENT
            || recipe.outputs[0].units != 1
        {
            return Err(MaterialCraftingError::ManifestInvalid(
                "logs_to_planks is not the canonical 1:1 recipe".to_owned(),
            ));
        }
        let input = self
            .ledger
            .lot(&input_lot_id)
            .cloned()
            .ok_or_else(|| MaterialCraftingError::Ledger("missing Logs lot".to_owned()))?;
        if input.key.content_id.as_str() != LOGS_CONTENT
            || input.reservation.is_some()
            || matches!(input.location, LotLocation::Cargo(_) | LotLocation::Hole(_))
        {
            return Err(MaterialCraftingError::Ledger(
                "Logs input is unavailable".to_owned(),
            ));
        }
        let output_content =
            ContentId::new(PLANKS_CONTENT).map_err(|_| MaterialCraftingError::EmptyIdentity)?;
        let output_quality = produced_quality(
            input.key.quality,
            input_lot_id.as_str(),
            &output_content,
            &context,
        )?;
        let output_id = stable_lot_id(command_id, &input_lot_id, context.completion_index)?;
        self.ledger
            .debit_lot(&input_lot_id, 1)
            .and_then(|()| {
                self.ledger.insert_lot(PhysicalLot {
                    id: output_id.clone(),
                    key: crate::quality_lots::BulkLotKey::new(output_content, output_quality),
                    provenance: LotProvenance {
                        origin: format!(
                            "logs_to_planks:{}:{}",
                            input_lot_id.as_str(),
                            input.provenance.origin
                        ),
                        created_tick: context.completion_index,
                    },
                    quantity: 1,
                    location: context.destination,
                    reservation: None,
                })
            })
            .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        Ok(MaterialCommandResult::PlanksProduced {
            consumed_lot_id: input_lot_id,
            produced_lot_id: output_id,
        })
    }

    fn resolve_destination(
        &self,
        id: &MaterialInstanceId,
        destination: RecoveryDestination,
    ) -> Result<LotLocation, MaterialCraftingError> {
        match destination {
            RecoveryDestination::Origin => self
                .origins
                .get(id)
                .cloned()
                .ok_or_else(|| MaterialCraftingError::DuplicateIdentity(id.clone())),
            RecoveryDestination::Stockpile(id) if !id.trim().is_empty() => {
                Ok(LotLocation::Stockpile(id))
            }
            RecoveryDestination::Cache(id) if !id.trim().is_empty() => Ok(LotLocation::Cache(id)),
            RecoveryDestination::Stockpile(_) | RecoveryDestination::Cache(_) => {
                Err(MaterialCraftingError::EmptyIdentity)
            }
        }
    }

    fn validate(&self, catalog: &MaterialCraftingCatalog<'_>) -> Result<(), MaterialCraftingError> {
        if self.materials.len() > MAX_NAMED_MATERIALS
            || self.fixture_targets.len() > MAX_FIXTURE_TARGETS
            || self.receipts.len() > MAX_MATERIAL_RECEIPTS
        {
            return Err(MaterialCraftingError::InventoryLimitExceeded);
        }
        let rebuilt = QualityLotLedger::new(
            self.ledger.lots().cloned().collect(),
            self.ledger.items().cloned().collect(),
        )
        .map_err(|error| MaterialCraftingError::Ledger(error.to_string()))?;
        if rebuilt != self.ledger {
            return Err(MaterialCraftingError::NonCanonicalState);
        }
        let mut identities = BTreeSet::new();
        for material in self.materials.values() {
            material.validate_against(catalog)?;
            if !identities.insert(material.instance_id.clone()) {
                return Err(MaterialCraftingError::DuplicateIdentity(
                    material.instance_id.clone(),
                ));
            }
        }
        for item in self.ledger.items() {
            if !identities.insert(item.id.clone()) {
                return Err(MaterialCraftingError::DuplicateIdentity(item.id.clone()));
            }
            if let Some(installed) = &item.augmentation {
                if !identities.insert(installed.item.id.clone()) {
                    return Err(MaterialCraftingError::DuplicateIdentity(
                        installed.item.id.clone(),
                    ));
                }
                let augmentation = catalog.augmentation(&installed.item.definition_id)?;
                let definition = catalog.item_definition(&item.definition_id)?;
                if augmentation.slot != installed.slot
                    || installed.item.augmentation_slot != Some(installed.slot)
                    || augmentation.consumed_materials != vec![installed.item.material_id.clone()]
                    || definition.augmentation_slot != Some(installed.slot)
                    || !augmentation
                        .compatible_item_classes
                        .contains(&definition.class)
                    || installed.item.reservation.is_some()
                    || installed.item.equipment_slot.is_some()
                    || installed.item.location
                        != LotLocation::StationInput(item.id.as_str().to_owned())
                {
                    return Err(MaterialCraftingError::IncompatibleItem(
                        item.definition_id.clone(),
                    ));
                }
            }
        }
        for target in self.fixture_targets.values() {
            validate_target(catalog, target)?;
            if let Some(fixture) = &target.fixture.installed
                && !identities.insert(fixture.item.id.clone())
            {
                return Err(MaterialCraftingError::DuplicateIdentity(
                    fixture.item.id.clone(),
                ));
            }
        }
        let receipt_versions = self
            .receipts
            .values()
            .map(|receipt| receipt.resulting_version)
            .collect::<BTreeSet<_>>();
        if identities.len() != self.origins.len()
            || identities.iter().any(|id| !self.origins.contains_key(id))
            || self.version != self.receipts.len() as u64
            || receipt_versions != (1..=self.version).collect::<BTreeSet<_>>()
            || self.receipts.iter().any(|(id, receipt)| {
                id != &receipt.command_id
                    || receipt.fingerprint == 0
                    || validate_command_id(id).is_err()
            })
        {
            return Err(MaterialCraftingError::NonCanonicalState);
        }
        Ok(())
    }
}

fn validate_target(
    catalog: &MaterialCraftingCatalog<'_>,
    target: &FixtureTarget,
) -> Result<(), MaterialCraftingError> {
    if target.target_id.trim().is_empty() {
        return Err(MaterialCraftingError::EmptyIdentity);
    }
    let station = catalog.station(&target.station_id)?;
    if station.fixture_slot != Some(target.fixture.slot) {
        return Err(MaterialCraftingError::IncompatibleFixture(
            target.station_id.clone(),
        ));
    }
    if let Some(installed) = &target.fixture.installed {
        let fixture = catalog.fixture(&installed.item.definition_id)?;
        if fixture.slot != installed.slot
            || installed.slot != target.fixture.slot
            || !fixture.compatible_stations.contains(&target.station_id)
            || fixture.consumed_materials != vec![installed.item.material_id.clone()]
            || installed.item.reservation.is_some()
            || installed.item.equipment_slot.is_some()
            || installed.item.augmentation_slot.is_some()
            || installed.item.location != LotLocation::StationInput(target.target_id.clone())
        {
            return Err(MaterialCraftingError::IncompatibleFixture(
                installed.item.definition_id.clone(),
            ));
        }
    }
    Ok(())
}

fn require_available_material(
    material: &NamedMaterialInstance,
    state: MaterialProcessingState,
) -> Result<(), MaterialCraftingError> {
    if material.state != state {
        return Err(MaterialCraftingError::WrongMaterialState);
    }
    if material.reservation.is_some() {
        return Err(MaterialCraftingError::ReservedMaterial(
            material.instance_id.clone(),
        ));
    }
    if matches!(material.location, LotLocation::Cargo(_)) {
        return Err(MaterialCraftingError::CarriedMaterial(
            material.instance_id.clone(),
        ));
    }
    Ok(())
}

fn find_exact_use<'a>(
    material: &'a MaterialDescriptor,
    station_id: &ItemDefinitionId,
    operation: EffectOperation,
    output: Option<&ContentId>,
) -> Result<&'a MaterialUseDescriptor, MaterialCraftingError> {
    material
        .uses
        .iter()
        .find(|candidate| {
            candidate.station == *station_id
                && candidate.operation == operation
                && output.is_none_or(|expected| &candidate.output == expected)
        })
        .ok_or_else(|| MaterialCraftingError::MissingUse {
            material_id: material.id.clone(),
            station_id: station_id.clone(),
            operation,
        })
}

fn produced_item(
    command_id: &str,
    material: &NamedMaterialInstance,
    definition_id: ItemDefinitionId,
    augmentation_slot: Option<crate::content_manifest::AugmentationSlot>,
    output_content_id: &ContentId,
    context: &ProductionContext,
) -> Result<ItemInstance, MaterialCraftingError> {
    let quality = produced_quality(
        material.quality,
        material.instance_id.as_str(),
        output_content_id,
        context,
    )?;
    Ok(ItemInstance {
        id: stable_item_id(
            command_id,
            &material.instance_id,
            &definition_id,
            context.completion_index,
        )?,
        definition_id,
        material_id: material.material_id.clone(),
        quality,
        durability: durability_for(quality),
        location: context.destination.clone(),
        reservation: None,
        equipment_slot: None,
        augmentation_slot,
        augmentation: None,
    })
}

fn produced_quality(
    input_quality: QualityBand,
    source_id: &str,
    output_content_id: &ContentId,
    context: &ProductionContext,
) -> Result<QualityBand, MaterialCraftingError> {
    let score = production_quality_score(ProductionQualityInput {
        weighted_input_quality_milli: input_quality.input_quality_milli(),
        worker_skill: context.worker_skill,
        tool_quality: context.tool_quality,
        fixture_quality: context.fixture_quality,
        station_tier: context.station_tier,
        complexity: ProductionComplexity::Simple,
        keyed_variation: deterministic_variation(
            context.world_seed,
            source_id,
            output_content_id.as_str(),
            context.completion_index,
        ),
    })
    .map_err(|_| MaterialCraftingError::ArithmeticOverflow)?;
    Ok(quality_from_score(score))
}

fn durability_for(quality: QualityBand) -> u32 {
    MAX_DURABILITY.saturating_mul(u32::from(quality.item_effect_durability_percent())) / 100
}

fn validate_command_id(id: &str) -> Result<(), MaterialCraftingError> {
    if id.is_empty()
        || id.len() > MAX_COMMAND_ID_BYTES
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':'))
    {
        return Err(MaterialCraftingError::InvalidCommandId);
    }
    Ok(())
}

fn command_fingerprint(command: &MaterialCommand) -> Result<u64, MaterialCraftingError> {
    let bytes =
        serde_json::to_vec(command).map_err(|_| MaterialCraftingError::ArithmeticOverflow)?;
    Ok(stable_hash64(&bytes))
}

fn stable_item_id(
    command_id: &str,
    source: &MaterialInstanceId,
    definition: &ItemDefinitionId,
    completion_index: u64,
) -> Result<MaterialInstanceId, MaterialCraftingError> {
    let hash = stable_hash64(
        format!(
            "{command_id}:{}:{}:{completion_index}",
            source.as_str(),
            definition.as_str()
        )
        .as_bytes(),
    );
    MaterialInstanceId::new(format!("crafted_{hash:016x}"))
        .map_err(|_| MaterialCraftingError::EmptyIdentity)
}

fn stable_lot_id(
    command_id: &str,
    source: &PhysicalLotId,
    completion_index: u64,
) -> Result<PhysicalLotId, MaterialCraftingError> {
    let hash =
        stable_hash64(format!("{command_id}:{}:{completion_index}", source.as_str()).as_bytes());
    PhysicalLotId::new(format!("planks_{hash:016x}"))
        .map_err(|_| MaterialCraftingError::EmptyIdentity)
}

fn stable_hash64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

fn deterministic_variation(
    world_seed: u32,
    source_id: &str,
    output_id: &str,
    completion_index: u64,
) -> i16 {
    let hash = stable_hash64(
        format!("{world_seed}:{source_id}:{output_id}:{completion_index}").as_bytes(),
    );
    let seed = ((hash >> 32) as u32) ^ (hash as u32);
    let roll = crate::rng::roll_seeded(f64::from(seed.max(1))).next_seed;
    (roll % 501) as i16 - 250
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MaterialCraftingAuthorityWire {
    schema_version: u32,
    version: u64,
    capabilities: CapabilitySet,
    materials: Vec<NamedMaterialInstance>,
    ledger: QualityLotLedger,
    fixture_targets: Vec<FixtureTarget>,
    origins: Vec<OriginRecord>,
    receipts: Vec<MaterialCommandReceipt>,
}

impl Serialize for MaterialCraftingAuthority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        MaterialCraftingAuthorityWire {
            schema_version: MATERIAL_CRAFTING_SCHEMA_VERSION,
            version: self.version,
            capabilities: self.capabilities.clone(),
            materials: self.materials.values().cloned().collect(),
            ledger: self.ledger.clone(),
            fixture_targets: self.fixture_targets.values().cloned().collect(),
            origins: self
                .origins
                .iter()
                .map(|(instance_id, location)| OriginRecord {
                    instance_id: instance_id.clone(),
                    location: location.clone(),
                })
                .collect(),
            receipts: self.receipts.values().cloned().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MaterialCraftingAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = MaterialCraftingAuthorityWire::deserialize(deserializer)?;
        if wire.schema_version != MATERIAL_CRAFTING_SCHEMA_VERSION {
            return Err(de::Error::custom(
                MaterialCraftingError::InvalidSchemaVersion(wire.schema_version),
            ));
        }
        if wire.materials.len() > MAX_NAMED_MATERIALS
            || wire.fixture_targets.len() > MAX_FIXTURE_TARGETS
            || wire.receipts.len() > MAX_MATERIAL_RECEIPTS
            || wire.origins.len()
                > MAX_NAMED_MATERIALS
                    + crate::quality_lots::MAX_ITEM_INSTANCES
                    + MAX_FIXTURE_TARGETS
        {
            return Err(de::Error::custom(
                MaterialCraftingError::InventoryLimitExceeded,
            ));
        }
        if !wire
            .materials
            .windows(2)
            .all(|pair| pair[0].instance_id < pair[1].instance_id)
            || !wire
                .fixture_targets
                .windows(2)
                .all(|pair| pair[0].target_id < pair[1].target_id)
            || !wire
                .origins
                .windows(2)
                .all(|pair| pair[0].instance_id < pair[1].instance_id)
            || !wire
                .receipts
                .windows(2)
                .all(|pair| pair[0].command_id < pair[1].command_id)
        {
            return Err(de::Error::custom(MaterialCraftingError::NonCanonicalState));
        }
        let authority = Self {
            version: wire.version,
            capabilities: wire.capabilities,
            materials: wire
                .materials
                .into_iter()
                .map(|value| (value.instance_id.clone(), value))
                .collect(),
            ledger: wire.ledger,
            fixture_targets: wire
                .fixture_targets
                .into_iter()
                .map(|value| (value.target_id.clone(), value))
                .collect(),
            origins: wire
                .origins
                .into_iter()
                .map(|value| (value.instance_id, value.location))
                .collect(),
            receipts: wire
                .receipts
                .into_iter()
                .map(|value| (value.command_id.clone(), value))
                .collect(),
        };
        authority
            .validate(&MaterialCraftingCatalog::embedded())
            .map_err(de::Error::custom)?;
        Ok(authority)
    }
}
