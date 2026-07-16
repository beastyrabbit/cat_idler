//! Expanded research catalog for the post-cutover design.
//!
//! The 24 legacy nodes from `upgrade_tree.rs` are owned records in an embedded
//! data file. Compact, named family templates deterministically expand the rest
//! of the roughly 500-node graph. This module is deliberately additive: it performs no
//! research ticks and grants no unlocks until later integration slices consume
//! its typed payloads.

use std::{
    collections::{BTreeSet, HashMap},
    hash::{BuildHasherDefault, Hasher},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

use crate::types::JobKind;

pub const RESEARCH_NODE_COUNT: usize = 487;
pub const APPROVED_BUILDING_IDS: &[&str] = &[
    "den",
    "food_storage",
    "water_bowl",
    "beds",
    "herb_garden",
    "nursery",
    "elder_corner",
    "walls",
    "mouse_farm",
    "shrine",
    "workshop",
    "field",
    "research_hut",
    "school",
    "smithy",
    "barracks",
    "wood_cutter",
    "stone_prep",
    "woodworking",
    "clothier",
    "tannery",
    "smelter",
    "accounting_tent",
    "mill",
    "sawmill",
];
pub const APPROVED_EFFECT_IDS: &[&str] = &[
    "productionRate",
    "storageCapacity",
    "movementSpeed",
    "farmYield",
    "researchRate",
    "constructionSpeed",
    "haulCapacity",
    "defensePower",
    "combatPower",
    "housingCapacity",
    "waterEfficiency",
    "tradeValue",
    "healthRecovery",
    "spoilageResistance",
    "huntYieldMult",
    "gatherYieldMult",
    "materialYieldMult",
    "farmYieldMult",
    "moveSpeedMult",
    "combatPowerMult",
    "defenseMult",
    "researchRateMult",
    "storagePerLevelMult",
    "housingPerDen",
    "waterCarryCapacity",
];
const LEGACY_SOURCE: &str = include_str!("research_catalog_legacy.json");
const TRACK_SOURCE: &str = include_str!("research_catalog_tracks.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCategory {
    Building,
    RecipeResource,
    Upgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchLayout {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOperation {
    Add,
    Multiply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingAttribute {
    Capacity,
    Output,
    CycleTime,
    WorkerSlots,
    Durability,
}

/// A catalog unlock is string-keyed on purpose: planned content such as
/// `accounting_tent`, `mill`, and `sawmill` must not require an existing sim or
/// protocol enum variant before a later integration layer is ready for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResearchPayload {
    /// Placement truth for a building that exists before this study is owned.
    /// Unlike `UnlockBuilding`, purchasing the node does not grant this access.
    BuildingAvailableAtFounding {
        building_id: String,
    },
    UnlockBuilding {
        building_id: String,
    },
    UnlockRecipe {
        recipe_id: String,
    },
    UnlockResource {
        resource_id: String,
    },
    UnlockJob {
        job_id: String,
    },
    ModifyBuilding {
        building_id: String,
        attribute: BuildingAttribute,
        operation: EffectOperation,
        value: f64,
    },
    Modify {
        effect_id: String,
        operation: EffectOperation,
        value: f64,
    },
    UnlockCapability {
        capability_id: String,
    },
}

impl ResearchPayload {
    fn is_non_inert(&self) -> bool {
        match self {
            Self::BuildingAvailableAtFounding { building_id } => !building_id.is_empty(),
            Self::UnlockBuilding { building_id } => !building_id.is_empty(),
            Self::UnlockRecipe { recipe_id } => !recipe_id.is_empty(),
            Self::UnlockResource { resource_id } => !resource_id.is_empty(),
            Self::UnlockJob { job_id } => !job_id.is_empty(),
            Self::ModifyBuilding {
                building_id, value, ..
            } => !building_id.is_empty() && value.is_finite() && *value > 0.0,
            Self::Modify {
                effect_id, value, ..
            } => !effect_id.is_empty() && value.is_finite() && *value > 0.0,
            Self::UnlockCapability { capability_id } => !capability_id.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ResearchCategory,
    pub cost: f64,
    /// All entries are required; prerequisite semantics are logical AND.
    pub prerequisites: Vec<String>,
    pub era: u8,
    pub layout: ResearchLayout,
    pub leader_priority: u16,
    pub payloads: Vec<ResearchPayload>,
}

impl ResearchNode {
    /// Catalog promises without a physical runtime object are deliberately future
    /// content. They remain visible in the 500-study graph but cannot consume points.
    #[must_use]
    pub fn is_future_content(&self) -> bool {
        self.payloads.iter().any(|payload| match payload {
            ResearchPayload::UnlockRecipe { recipe_id } => {
                !crate::station_recipes::is_runtime_recipe_id(recipe_id)
            }
            ResearchPayload::UnlockResource { resource_id } => {
                !is_runtime_resource_unlock_id(resource_id)
            }
            ResearchPayload::ModifyBuilding {
                building_id,
                attribute: BuildingAttribute::WorkerSlots,
                ..
            } => !worker_slots_building_is_implemented(building_id),
            _ => false,
        })
    }
}

/// Generated family-stage resource promises with exact, observable runtime
/// consumers. Keeping this allow-list explicit prevents later catalog prose from
/// silently becoming purchasable no-ops.
pub const RUNTIME_RESOURCE_UNLOCK_IDS: &[&str] = &[
    "grain_milling_sources",
    "grain_milling_preservation",
    "grain_milling_bulk",
    "grain_milling_reserves",
    "baking_sources",
    "baking_preservation",
    "baking_bulk",
    "baking_reserves",
    "herbalism_sources",
    "herbalism_preservation",
    "herbalism_bulk",
    "food_preservation_sources",
    "food_preservation_preservation",
    "food_preservation_bulk",
    "brewing_sources",
    "brewing_preservation",
    "brewing_bulk",
];

#[must_use]
pub fn is_runtime_resource_unlock_id(resource_id: &str) -> bool {
    RUNTIME_RESOURCE_UNLOCK_IDS.contains(&resource_id)
}

/// Fixed FNV-1a hashing keeps the O(1) index deterministic and avoids
/// `RandomState`'s process-random seed. Catalog order always comes from `nodes`.
#[derive(Default)]
struct StableHasher(u64);

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = hash;
    }
}

type StableBuildHasher = BuildHasherDefault<StableHasher>;
type StableIndex = HashMap<String, usize, StableBuildHasher>;

#[derive(Debug)]
pub struct ResearchCatalog {
    nodes: Vec<ResearchNode>,
    by_id: StableIndex,
    recipe_unlocks: StableIndex,
    job_unlocks: StableIndex,
}

impl ResearchCatalog {
    #[must_use]
    pub fn nodes(&self) -> &[ResearchNode] {
        &self.nodes
    }

    /// Average-case O(1) lookup through the deterministic index.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ResearchNode> {
        self.by_id.get(id).map(|index| &self.nodes[*index])
    }

    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    #[must_use]
    pub fn category_count(&self, category: ResearchCategory) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.category == category)
            .count()
    }

    /// O(1) reverse lookup for the study whose typed payload unlocks `recipe_id`.
    /// Built once with the catalog, so production hot loops never scan the entire graph.
    #[must_use]
    pub fn recipe_unlock_study(&self, recipe_id: &str) -> Option<&ResearchNode> {
        self.recipe_unlocks
            .get(recipe_id)
            .map(|index| &self.nodes[*index])
    }

    /// O(1) reverse lookup for the study whose typed payload unlocks `job_id`.
    /// Validation guarantees every indexed ID is a real runtime [`JobKind`] and
    /// that no second study competes for the same job entitlement.
    #[must_use]
    pub fn job_unlock_study(&self, job_id: &str) -> Option<&ResearchNode> {
        self.job_unlocks
            .get(job_id)
            .map(|index| &self.nodes[*index])
    }

    /// Test AND-prerequisite ownership without imposing a collection type on a
    /// later consumer.
    #[must_use]
    pub fn prerequisites_met(&self, id: &str, mut is_owned: impl FnMut(&str) -> bool) -> bool {
        self.get(id).is_some_and(|node| {
            node.prerequisites
                .iter()
                .all(|prerequisite| is_owned(prerequisite))
        })
    }
}

static RESEARCH_CATALOG: OnceLock<ResearchCatalog> = OnceLock::new();

#[must_use]
pub fn research_catalog() -> &'static ResearchCatalog {
    RESEARCH_CATALOG.get_or_init(|| {
        build_catalog(LEGACY_SOURCE, TRACK_SOURCE)
            .unwrap_or_else(|error| panic!("embedded research catalog is invalid: {error}"))
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyData {
    nodes: Vec<ResearchNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TemplateData {
    building_stages: Vec<BuildingStage>,
    building_families: Vec<BuildingFamily>,
    recipe_stages: Vec<RecipeStage>,
    recipe_families: Vec<RecipeFamily>,
    upgrade_stages: Vec<UpgradeStage>,
    upgrade_families: Vec<UpgradeFamily>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildingStage {
    id: String,
    name: String,
    description: String,
    attribute: BuildingAttribute,
    operation: EffectOperation,
    value: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildingFamily {
    building_id: String,
    display_name: String,
    count: usize,
    unlock_first: bool,
    #[serde(default)]
    available_at_founding: bool,
    root_prerequisites: Vec<String>,
    era_start: u8,
    cost_base: f64,
    layout_x: i32,
    leader_priority_base: u16,
    /// Whether this runtime building owns a real, routed physical capacity domain.
    /// Families without one omit the generic `stores` stage entirely rather than
    /// charging research points for an inert modifier.
    #[serde(default)]
    physical_capacity: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecipeStage {
    id: String,
    name: String,
    description: String,
    payload: RecipePayloadKind,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecipePayloadKind {
    Recipe,
    Resource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecipeFamily {
    id: String,
    display_name: String,
    count: usize,
    root_prerequisites: Vec<String>,
    era_start: u8,
    cost_base: f64,
    layout_x: i32,
    leader_priority_base: u16,
    /// Data-owned bindings from a generated stage id to an already-implemented
    /// authoritative recipe id. Unlisted stages keep their catalog registry id.
    #[serde(default)]
    payload_overrides: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeStage {
    id: String,
    name: String,
    description: String,
    value: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpgradeFamily {
    id: String,
    display_name: String,
    effect_id: String,
    /// Some cross-cutting effect tracks belong to a product category other than
    /// the default Upgrade bucket. For example, every Construction study changes
    /// the real physical scaffold timer and is therefore building research.
    #[serde(default = "upgrade_research_category")]
    category: ResearchCategory,
    root_prerequisites: Vec<String>,
    era_start: u8,
    cost_base: f64,
    layout_x: i32,
    leader_priority_base: u16,
}

const fn upgrade_research_category() -> ResearchCategory {
    ResearchCategory::Upgrade
}

fn build_catalog(legacy_source: &str, track_source: &str) -> Result<ResearchCatalog, String> {
    let mut nodes = serde_json::from_str::<LegacyData>(legacy_source)
        .map_err(|error| format!("legacy JSON: {error}"))?
        .nodes;
    let templates = serde_json::from_str::<TemplateData>(track_source)
        .map_err(|error| format!("track JSON: {error}"))?;
    for family in &templates.building_families {
        nodes.extend(expand_building_family(family, &templates.building_stages)?);
    }
    for family in &templates.recipe_families {
        nodes.extend(expand_recipe_family(family, &templates.recipe_stages)?);
    }
    for family in &templates.upgrade_families {
        nodes.extend(expand_upgrade_family(family, &templates.upgrade_stages)?);
    }
    validate_and_index(nodes)
}

fn expand_building_family(
    family: &BuildingFamily,
    stages: &[BuildingStage],
) -> Result<Vec<ResearchNode>, String> {
    if family.count == 0 || family.count > stages.len() {
        return Err(format!("invalid stage count for {}", family.building_id));
    }
    let mut nodes: Vec<ResearchNode> = Vec::with_capacity(family.count);
    for (index, stage) in stages.iter().take(family.count).enumerate() {
        if stage.attribute == BuildingAttribute::Capacity && !family.physical_capacity {
            continue;
        }
        let id = format!("{}_{}", family.building_id, stage.id);
        let prerequisites = if nodes.is_empty() {
            family.root_prerequisites.clone()
        } else {
            let previous = nodes
                .iter()
                .rev()
                .find(|node| !node.is_future_content())
                .expect("non-empty family has a supported predecessor");
            vec![previous.id.clone()]
        };
        let era_offset = u8::try_from(index / 2)
            .map_err(|_| format!("building era overflow for {}", family.building_id))?;
        let era = family
            .era_start
            .checked_add(era_offset)
            .ok_or_else(|| format!("building era overflow for {}", family.building_id))?;
        if index == 0 && family.unlock_first && family.available_at_founding {
            return Err(format!(
                "{} cannot be both founding-available and research-unlocked",
                family.building_id
            ));
        }
        let payload = if family.unlock_first && index == 0 {
            ResearchPayload::UnlockBuilding {
                building_id: family.building_id.clone(),
            }
        } else {
            ResearchPayload::ModifyBuilding {
                building_id: family.building_id.clone(),
                attribute: stage.attribute,
                operation: stage.operation,
                value: stage.value,
            }
        };
        let mut payloads = vec![payload];
        if index == 0 && family.available_at_founding {
            payloads.insert(
                0,
                ResearchPayload::BuildingAvailableAtFounding {
                    building_id: family.building_id.clone(),
                },
            );
        }
        let future_worker_study = stage.attribute == BuildingAttribute::WorkerSlots
            && !worker_slots_building_is_implemented(&family.building_id);
        nodes.push(ResearchNode {
            id,
            name: format!(
                "{} {}{}",
                family.display_name,
                stage.name,
                if future_worker_study { " (future)" } else { "" }
            ),
            description: if future_worker_study {
                format!(
                    "Future study: {} has no independent physical worker station yet; this node cannot be purchased.",
                    family.display_name
                )
            } else {
                format!("{} {}", family.display_name, stage.description)
            },
            category: ResearchCategory::Building,
            cost: family.cost_base + 4.0 * index as f64,
            prerequisites,
            era,
            layout: ResearchLayout {
                x: family.layout_x,
                y: i32::try_from(index + 1).map_err(|_| "building layout overflow")?,
            },
            leader_priority: family.leader_priority_base
                + u16::try_from(index).map_err(|_| "building priority overflow")?,
            payloads,
        });
    }
    Ok(nodes)
}

/// Building families with a real independent worker-state consumer. Generated
/// `*_crews` studies for every other family stay catalog-visible as future work but
/// cannot be purchased or become a hidden no-op.
#[must_use]
pub fn worker_slots_building_is_implemented(building_id: &str) -> bool {
    matches!(
        building_id,
        "workshop"
            | "wood_cutter"
            | "stone_prep"
            | "woodworking"
            | "smithy"
            | "clothier"
            | "tannery"
            | "smelter"
            | "mill"
            | "sawmill"
            | "research_hut"
            | "school"
    )
}

#[must_use]
pub fn research_node_is_implemented(node: &ResearchNode) -> bool {
    !node.is_future_content()
}

fn expand_recipe_family(
    family: &RecipeFamily,
    stages: &[RecipeStage],
) -> Result<Vec<ResearchNode>, String> {
    if family.count == 0 || family.count > stages.len() {
        return Err(format!("invalid stage count for {}", family.id));
    }
    let mut nodes: Vec<ResearchNode> = Vec::with_capacity(family.count);
    for (index, stage) in stages.iter().take(family.count).enumerate() {
        let id = format!("{}_{}", family.id, stage.id);
        let generated_payload_id = format!("{}_{}", family.id, stage.id);
        let payload_id = family
            .payload_overrides
            .get(&stage.id)
            .cloned()
            .unwrap_or(generated_payload_id);
        let payload = match stage.payload {
            RecipePayloadKind::Recipe => ResearchPayload::UnlockRecipe {
                recipe_id: payload_id,
            },
            RecipePayloadKind::Resource => ResearchPayload::UnlockResource {
                resource_id: payload_id,
            },
        };
        // An implemented physical recipe must never sit behind a generic resource
        // registry promise that has no source entitlement. Give that recipe the
        // family's real maintained prerequisites directly; future nodes remain
        // visible on their own branch and cannot consume points.
        let bypass_future_predecessor =
            matches!(
                &payload,
                ResearchPayload::UnlockRecipe { recipe_id }
                    if crate::station_recipes::is_runtime_recipe_id(recipe_id)
            ) && nodes.last().is_some_and(ResearchNode::is_future_content);
        let prerequisites = if index == 0 || bypass_future_predecessor {
            family.root_prerequisites.clone()
        } else {
            vec![nodes[index - 1].id.clone()]
        };
        nodes.push(ResearchNode {
            id,
            name: format!("{} {}", family.display_name, stage.name),
            description: format!("{} {}", family.display_name, stage.description),
            category: ResearchCategory::RecipeResource,
            cost: family.cost_base + 3.5 * index as f64,
            prerequisites,
            era: family
                .era_start
                .checked_add(u8::try_from(index / 3).map_err(|_| "recipe era overflow")?)
                .ok_or_else(|| format!("recipe era overflow for {}", family.id))?,
            layout: ResearchLayout {
                x: family.layout_x,
                y: i32::try_from(index + 1).map_err(|_| "recipe layout overflow")?,
            },
            leader_priority: family.leader_priority_base
                + u16::try_from(index).map_err(|_| "recipe priority overflow")?,
            payloads: vec![payload],
        });
    }
    Ok(nodes)
}

fn expand_upgrade_family(
    family: &UpgradeFamily,
    stages: &[UpgradeStage],
) -> Result<Vec<ResearchNode>, String> {
    if stages.len() != 11 {
        return Err(format!(
            "upgrade family {} requires eleven stages",
            family.id
        ));
    }
    let mut nodes: Vec<ResearchNode> = Vec::with_capacity(stages.len());
    for (index, stage) in stages.iter().enumerate() {
        let id = format!("{}_{}", family.id, stage.id);
        let prerequisites = if index == 0 {
            family.root_prerequisites.clone()
        } else {
            vec![nodes[index - 1].id.clone()]
        };
        nodes.push(ResearchNode {
            id,
            name: format!("{} {}", family.display_name, stage.name),
            description: format!("{} {}", family.display_name, stage.description),
            category: family.category,
            cost: family.cost_base + 4.5 * index as f64,
            prerequisites,
            era: family
                .era_start
                .checked_add(u8::try_from(index / 3).map_err(|_| "upgrade era overflow")?)
                .ok_or_else(|| format!("upgrade era overflow for {}", family.id))?,
            layout: ResearchLayout {
                x: family.layout_x,
                y: i32::try_from(index + 1).map_err(|_| "upgrade layout overflow")?,
            },
            leader_priority: family.leader_priority_base
                + u16::try_from(index).map_err(|_| "upgrade priority overflow")?,
            payloads: vec![ResearchPayload::Modify {
                effect_id: family.effect_id.clone(),
                operation: EffectOperation::Add,
                value: stage.value,
            }],
        });
    }
    Ok(nodes)
}

fn validate_and_index(nodes: Vec<ResearchNode>) -> Result<ResearchCatalog, String> {
    if nodes.len() != RESEARCH_NODE_COUNT {
        return Err(format!(
            "expected {RESEARCH_NODE_COUNT} nodes, found {}",
            nodes.len()
        ));
    }

    let mut by_id =
        StableIndex::with_capacity_and_hasher(nodes.len(), StableBuildHasher::default());
    let mut recipe_unlocks =
        StableIndex::with_capacity_and_hasher(nodes.len(), StableBuildHasher::default());
    let mut job_unlocks =
        StableIndex::with_capacity_and_hasher(nodes.len(), StableBuildHasher::default());
    let mut layouts = BTreeSet::new();
    for (index, node) in nodes.iter().enumerate() {
        validate_node(node)?;
        if by_id.insert(node.id.clone(), index).is_some() {
            return Err(format!("duplicate node id {}", node.id));
        }
        if !layouts.insert((node.layout.x, node.layout.y)) {
            return Err(format!(
                "duplicate layout slot {},{}",
                node.layout.x, node.layout.y
            ));
        }
        for payload in &node.payloads {
            match payload {
                ResearchPayload::UnlockRecipe { recipe_id } => {
                    if recipe_unlocks.insert(recipe_id.clone(), index).is_some() {
                        return Err(format!("duplicate recipe unlock {recipe_id}"));
                    }
                }
                ResearchPayload::UnlockJob { job_id } => {
                    if !JobKind::ALL.iter().any(|kind| kind.as_str() == job_id) {
                        return Err(format!(
                            "node {} targets unknown runtime job {job_id}",
                            node.id
                        ));
                    }
                    if job_unlocks.insert(job_id.clone(), index).is_some() {
                        return Err(format!("duplicate job unlock {job_id}"));
                    }
                }
                _ => {}
            }
        }
    }

    validate_categories(&nodes)?;
    validate_building_placement_sources(&nodes)?;
    validate_topology(&nodes, &by_id)?;
    Ok(ResearchCatalog {
        nodes,
        by_id,
        recipe_unlocks,
        job_unlocks,
    })
}

fn validate_node(node: &ResearchNode) -> Result<(), String> {
    if node.id.is_empty() || node.name.is_empty() || node.description.is_empty() {
        return Err(format!("node {} has blank identity text", node.id));
    }
    if node.era == 0 || !node.cost.is_finite() || node.cost < 0.0 {
        return Err(format!("node {} has invalid era/cost", node.id));
    }
    if node.payloads.is_empty() || !node.payloads.iter().all(ResearchPayload::is_non_inert) {
        return Err(format!("node {} has an inert payload", node.id));
    }
    for payload in &node.payloads {
        match payload {
            ResearchPayload::BuildingAvailableAtFounding { building_id }
            | ResearchPayload::UnlockBuilding { building_id }
            | ResearchPayload::ModifyBuilding { building_id, .. }
                if !APPROVED_BUILDING_IDS.contains(&building_id.as_str()) =>
            {
                return Err(format!(
                    "node {} targets unknown building {building_id}",
                    node.id
                ));
            }
            ResearchPayload::Modify { effect_id, .. }
                if !APPROVED_EFFECT_IDS.contains(&effect_id.as_str()) =>
            {
                return Err(format!(
                    "node {} targets unknown effect {effect_id}",
                    node.id
                ));
            }
            _ => {}
        }
    }
    let category_payload = match node.category {
        ResearchCategory::Building => node.payloads.iter().any(|payload| {
            matches!(
                payload,
                ResearchPayload::BuildingAvailableAtFounding { .. }
                    | ResearchPayload::UnlockBuilding { .. }
                    | ResearchPayload::ModifyBuilding { .. }
            ) || matches!(
                payload,
                ResearchPayload::Modify { effect_id, .. }
                    if matches!(effect_id.as_str(), "housingPerDen" | "constructionSpeed")
            )
        }),
        ResearchCategory::RecipeResource => node.payloads.iter().any(|payload| {
            matches!(
                payload,
                ResearchPayload::UnlockRecipe { .. }
                    | ResearchPayload::UnlockResource { .. }
                    | ResearchPayload::UnlockJob { .. }
            )
        }),
        ResearchCategory::Upgrade => node.payloads.iter().any(|payload| {
            matches!(
                payload,
                ResearchPayload::Modify { .. } | ResearchPayload::UnlockCapability { .. }
            )
        }),
    };
    if !category_payload {
        return Err(format!("node {} has no category payload", node.id));
    }
    Ok(())
}

fn validate_categories(nodes: &[ResearchNode]) -> Result<(), String> {
    let buildings = nodes
        .iter()
        .filter(|node| node.category == ResearchCategory::Building)
        .count();
    let recipes = nodes
        .iter()
        .filter(|node| node.category == ResearchCategory::RecipeResource)
        .count();
    // Building families deliberately omit a generic `stores` stage when their runtime
    // building owns no physical capacity domain. Construction's eleven live scaffold-
    // speed studies keep the truthful graph above the one-third product requirement
    // without restoring those inert purchases.
    if buildings * 3 < nodes.len() || recipes * 3 < nodes.len() {
        return Err(format!(
            "research categories miss the one-third design floor: {buildings} building, {recipes} recipe/resource, {} total",
            nodes.len()
        ));
    }
    Ok(())
}

/// A building may have exactly one placement source: either it is available at
/// founding or one study unlocks it. This prevents a generated family node from
/// silently competing with a legacy placement gate.
fn validate_building_placement_sources(nodes: &[ResearchNode]) -> Result<(), String> {
    let mut sources: HashMap<&str, (&str, bool)> = HashMap::new();
    for node in nodes {
        for payload in &node.payloads {
            let (building_id, founding) = match payload {
                ResearchPayload::BuildingAvailableAtFounding { building_id } => {
                    (building_id.as_str(), true)
                }
                ResearchPayload::UnlockBuilding { building_id } => (building_id.as_str(), false),
                _ => continue,
            };
            if let Some((prior_node, prior_founding)) =
                sources.insert(building_id, (node.id.as_str(), founding))
            {
                let prior_kind = if prior_founding {
                    "founding"
                } else {
                    "research"
                };
                let kind = if founding { "founding" } else { "research" };
                return Err(format!(
                    "building {building_id} has competing placement sources: {prior_kind} node {prior_node} and {kind} node {}",
                    node.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_topology(nodes: &[ResearchNode], by_id: &StableIndex) -> Result<(), String> {
    let mut indegree = vec![0_usize; nodes.len()];
    let mut dependents = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if node.prerequisites.is_empty() && node.id != "research_hut" {
            return Err(format!(
                "node {} is disconnected from the research root",
                node.id
            ));
        }
        let mut unique = BTreeSet::new();
        for prerequisite in &node.prerequisites {
            if !unique.insert(prerequisite) {
                return Err(format!(
                    "node {} repeats prerequisite {prerequisite}",
                    node.id
                ));
            }
            let Some(prerequisite_index) = by_id.get(prerequisite).copied() else {
                return Err(format!(
                    "node {} references missing prerequisite {prerequisite}",
                    node.id
                ));
            };
            if prerequisite_index == index {
                return Err(format!("node {} depends on itself", node.id));
            }
            if nodes[prerequisite_index].era > node.era {
                return Err(format!(
                    "node {} precedes later-era prerequisite {prerequisite}",
                    node.id
                ));
            }
            indegree[index] += 1;
            dependents[prerequisite_index].push(index);
        }
    }

    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut visited = 0;
    while let Some(index) = ready.pop_first() {
        visited += 1;
        for dependent in &dependents[index] {
            indegree[*dependent] -= 1;
            if indegree[*dependent] == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if visited != nodes.len() {
        return Err(format!(
            "catalog graph is cyclic or unreachable: visited {visited}/{}",
            nodes.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade_tree::{
        MOUNTAINEERING_NODE_ID, RAIL_NODE_ID, SHIPPING_NODE_ID, UPGRADE_NODES,
    };

    #[test]
    fn embedded_catalog_expands_to_the_exact_design_size_and_ratios() {
        let catalog = research_catalog();
        assert_eq!(catalog.nodes().len(), RESEARCH_NODE_COUNT);
        let buildings = catalog.category_count(ResearchCategory::Building);
        let recipes = catalog.category_count(ResearchCategory::RecipeResource);
        assert_eq!(buildings, 165);
        assert_eq!(recipes, 167);
        assert_eq!(catalog.category_count(ResearchCategory::Upgrade), 155);
        assert!(
            buildings * 3 >= RESEARCH_NODE_COUNT,
            "building research must remain at least one third of the catalog"
        );
        assert!(
            recipes * 3 >= RESEARCH_NODE_COUNT,
            "recipe/resource research must remain at least one third of the catalog"
        );
    }

    #[test]
    fn catalog_order_lookup_and_expansion_boundaries_are_stable() {
        let catalog = research_catalog();
        let ids: Vec<&str> = catalog
            .nodes()
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        assert_eq!(ids.first(), Some(&"research_hut"));
        assert_eq!(ids.get(22), Some(&"shipping"));
        assert_eq!(ids.get(23), Some(&"milling"));
        assert_eq!(ids.get(24), Some(&"den_foundations"));
        assert_eq!(ids.get(168), Some(&"sawmill_crews"));
        assert_eq!(ids.get(169), Some(&"hunting_sources"));
        assert_eq!(ids.get(332), Some(&"expedition_supplies_masterwork"));
        assert_eq!(ids.get(333), Some(&"logistics_basics"));
        assert_eq!(ids.last(), Some(&"resilience_mastery"));
        for (index, node) in catalog.nodes().iter().enumerate() {
            assert!(std::ptr::eq(
                catalog.get(&node.id).unwrap(),
                &catalog.nodes()[index]
            ));
        }
        assert!(catalog.get("not_a_node").is_none());
    }

    #[test]
    fn all_nodes_have_unique_slots_valid_values_real_payloads_and_valid_edges() {
        let catalog = research_catalog();
        let mut ids = BTreeSet::new();
        let mut layouts = BTreeSet::new();
        for node in catalog.nodes() {
            assert!(ids.insert(node.id.as_str()), "duplicate {}", node.id);
            assert!(layouts.insert((node.layout.x, node.layout.y)));
            assert!(node.cost.is_finite() && node.cost >= 0.0);
            assert!(node.era > 0);
            assert!(!node.payloads.is_empty());
            assert!(node.payloads.iter().all(ResearchPayload::is_non_inert));
            for prerequisite in &node.prerequisites {
                let prerequisite = catalog.get(prerequisite).expect("prerequisite exists");
                assert!(prerequisite.era <= node.era);
            }
        }
        validate_topology(catalog.nodes(), &catalog.by_id).expect("acyclic reachable graph");
    }

    #[test]
    fn every_building_capacity_study_names_a_real_physical_store() {
        let actual = research_catalog()
            .nodes()
            .iter()
            .flat_map(|node| {
                node.payloads
                    .iter()
                    .filter_map(move |payload| match payload {
                        ResearchPayload::ModifyBuilding {
                            building_id,
                            attribute: BuildingAttribute::Capacity,
                            ..
                        } => Some((node.id.as_str(), building_id.as_str())),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        let expected = [
            ("food_storage_stores", "food_storage"),
            ("water_bowl_stores", "water_bowl"),
            ("workshop_stores", "workshop"),
            ("smithy_stores", "smithy"),
            ("wood_cutter_stores", "wood_cutter"),
            ("stone_prep_stores", "stone_prep"),
            ("woodworking_stores", "woodworking"),
            ("clothier_stores", "clothier"),
            ("tannery_stores", "tannery"),
            ("smelter_stores", "smelter"),
            ("mill_stores", "mill"),
            ("sawmill_stores", "sawmill"),
        ];

        assert_eq!(actual, expected);
        for retired in [
            "den_stores",
            "beds_stores",
            "herb_garden_stores",
            "nursery_stores",
            "elder_corner_stores",
            "walls_stores",
            "mouse_farm_stores",
            "shrine_stores",
            "field_stores",
            "research_hut_stores",
            "school_stores",
            "barracks_stores",
            "accounting_tent_stores",
        ] {
            assert!(
                research_catalog().get(retired).is_none(),
                "{retired} must not charge points for a container that does not exist"
            );
        }
    }

    #[test]
    fn bootstrap_hut_and_mill_foundations_do_not_claim_false_building_unlocks() {
        let catalog = research_catalog();
        let research_hut = catalog.get("research_hut").expect("research root");
        assert!(
            research_hut.payloads.iter().all(|payload| !matches!(
                payload,
                ResearchPayload::UnlockBuilding { building_id } if building_id == "research_hut"
            )),
            "the hut is placeable before its root study and must not claim to unlock itself"
        );
        assert!(
            research_hut.description.contains("available from founding"),
            "the player-visible root copy must explain the bootstrap"
        );

        let mill_foundations = catalog
            .get("mill_foundations")
            .expect("generated mill foundations study");
        assert!(mill_foundations.payloads.iter().any(|payload| matches!(
            payload,
            ResearchPayload::ModifyBuilding {
                building_id,
                attribute: BuildingAttribute::Durability,
                ..
            } if building_id == "mill"
        )));
        assert!(mill_foundations.payloads.iter().all(|payload| !matches!(
            payload,
            ResearchPayload::UnlockBuilding { building_id } if building_id == "mill"
        )));

        let mill_unlockers = catalog
            .nodes()
            .iter()
            .filter(|node| {
                node.payloads.iter().any(|payload| {
                    matches!(
                        payload,
                        ResearchPayload::UnlockBuilding { building_id } if building_id == "mill"
                    )
                })
            })
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(mill_unlockers, ["milling"]);
    }

    #[test]
    fn the_three_blueprint_benches_have_unique_founding_placement_sources() {
        let catalog = research_catalog();
        let expected = [
            (
                "wood_cutter",
                "wood_cutter_foundations",
                &["sawmill"][..],
                14.0_f64,
            ),
            (
                "stone_prep",
                "stone_prep_foundations",
                &["masonry"][..],
                16.0,
            ),
            (
                "woodworking",
                "woodworking_foundations",
                &["sawmill", "basic_tools"][..],
                15.0,
            ),
        ];

        for (building_id, expected_node_id, prerequisites, cost) in expected {
            let sources = catalog
                .nodes()
                .iter()
                .flat_map(|node| {
                    node.payloads
                        .iter()
                        .filter_map(move |payload| match payload {
                            ResearchPayload::BuildingAvailableAtFounding {
                                building_id: candidate,
                            } if candidate == building_id => Some((node.id.as_str(), true)),
                            ResearchPayload::UnlockBuilding {
                                building_id: candidate,
                            } if candidate == building_id => Some((node.id.as_str(), false)),
                            _ => None,
                        })
                })
                .collect::<Vec<_>>();
            assert_eq!(sources, [(expected_node_id, true)], "{building_id}");

            let first_study = catalog.get(expected_node_id).expect("first family study");
            assert_eq!(first_study.category, ResearchCategory::Building);
            assert_eq!(first_study.cost.to_bits(), cost.to_bits());
            assert_eq!(
                first_study
                    .prerequisites
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                prerequisites
            );
            assert!(first_study.payloads.iter().any(|payload| matches!(
                payload,
                ResearchPayload::ModifyBuilding {
                    building_id: target,
                    attribute: BuildingAttribute::Durability,
                    operation: EffectOperation::Add,
                    value,
                } if target == building_id && value.to_bits() == 0.15_f64.to_bits()
            )));
        }

        assert_eq!(catalog.category_count(ResearchCategory::Building), 165);
        assert_eq!(
            catalog.category_count(ResearchCategory::RecipeResource),
            167
        );
        assert_eq!(catalog.category_count(ResearchCategory::Upgrade), 155);
    }

    #[test]
    fn construction_track_is_truthful_building_research_with_live_effects() {
        let construction = research_catalog()
            .nodes()
            .iter()
            .filter(|node| node.id.starts_with("construction_"))
            .collect::<Vec<_>>();
        assert_eq!(construction.len(), 11);
        for node in construction {
            assert_eq!(node.category, ResearchCategory::Building, "{}", node.id);
            assert!(!node.is_future_content(), "{}", node.id);
            assert!(node.payloads.iter().any(|payload| matches!(
                payload,
                ResearchPayload::Modify {
                    effect_id,
                    operation: EffectOperation::Add,
                    value,
                } if effect_id == "constructionSpeed" && *value > 0.0
            )));
        }
    }

    #[test]
    fn legacy_nodes_preserve_stable_identity_cost_edges_and_order() {
        let catalog = research_catalog();
        assert_eq!(
            UPGRADE_NODES.len(),
            24,
            "update the embedded legacy catalog"
        );
        for legacy in UPGRADE_NODES {
            let node = catalog.get(legacy.id).expect("legacy id preserved");
            assert_eq!(node.name, legacy.name, "name for {}", legacy.id);
            assert_eq!(
                node.description, legacy.description,
                "description for {}",
                legacy.id
            );
            assert_eq!(node.era, legacy.era, "era for {}", legacy.id);
            assert_eq!(
                node.cost.to_bits(),
                legacy.cost.to_bits(),
                "cost for {}",
                legacy.id
            );
            assert_eq!(
                node.prerequisites, legacy.prerequisites,
                "prerequisites for {}",
                legacy.id
            );

            let buildings: Vec<&str> = node
                .payloads
                .iter()
                .filter_map(|payload| match payload {
                    ResearchPayload::UnlockBuilding { building_id } => Some(building_id.as_str()),
                    _ => None,
                })
                .collect();
            let effects: Vec<(&str, u64)> = node
                .payloads
                .iter()
                .filter_map(|payload| match payload {
                    ResearchPayload::Modify {
                        effect_id, value, ..
                    } => Some((effect_id.as_str(), value.to_bits())),
                    _ => None,
                })
                .collect();
            assert_eq!(buildings, legacy.unlocks.buildings.unwrap_or_default());
            let legacy_effects: Vec<(&str, u64)> = legacy
                .unlocks
                .effects
                .unwrap_or_default()
                .iter()
                .map(|effect| (effect.key.as_str(), effect.value.to_bits()))
                .collect();
            assert_eq!(effects, legacy_effects, "effects for {}", legacy.id);
        }

        for (id, capability) in [
            (MOUNTAINEERING_NODE_ID, "mountain_travel"),
            (RAIL_NODE_ID, "rail_logistics"),
            (SHIPPING_NODE_ID, "water_travel"),
        ] {
            assert!(catalog.get(id).unwrap().payloads.iter().any(|payload| {
                matches!(payload, ResearchPayload::UnlockCapability { capability_id } if capability_id == capability)
            }));
        }
    }

    #[test]
    fn catalog_job_entitlement_is_unique_runtime_logging_only() {
        let catalog = research_catalog();
        let job_payloads = catalog
            .nodes()
            .iter()
            .flat_map(|node| {
                node.payloads
                    .iter()
                    .filter_map(move |payload| match payload {
                        ResearchPayload::UnlockJob { job_id } => {
                            Some((node.id.as_str(), job_id.as_str()))
                        }
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(job_payloads, [("sawmill", "gather_logs")]);
        assert_eq!(
            catalog
                .job_unlock_study(JobKind::GatherLogs.as_str())
                .map(|node| node.name.as_str()),
            Some("Sawmill")
        );
        for founding_job in [JobKind::FetchWater, JobKind::Explore, JobKind::TrainWarrior] {
            assert!(catalog.job_unlock_study(founding_job.as_str()).is_none());
        }
    }

    #[test]
    fn catalog_rejects_duplicate_and_unknown_runtime_job_payloads() {
        let mut duplicate = research_catalog().nodes().to_vec();
        duplicate[1].payloads.push(ResearchPayload::UnlockJob {
            job_id: JobKind::GatherLogs.as_str().to_owned(),
        });
        assert_eq!(
            validate_and_index(duplicate).unwrap_err(),
            "duplicate job unlock gather_logs"
        );

        let mut unknown = research_catalog().nodes().to_vec();
        let sawmill = unknown
            .iter_mut()
            .find(|node| node.id == "sawmill")
            .expect("sawmill node");
        let ResearchPayload::UnlockJob { job_id } = sawmill
            .payloads
            .iter_mut()
            .find(|payload| matches!(payload, ResearchPayload::UnlockJob { .. }))
            .expect("logging payload")
        else {
            unreachable!();
        };
        *job_id = "forge_tools".to_owned();
        assert_eq!(
            validate_and_index(unknown).unwrap_err(),
            "node sawmill targets unknown runtime job forge_tools"
        );
    }

    #[test]
    fn legacy_category_reconciliation_uses_only_live_payloads() {
        let catalog = research_catalog();
        for (node_id, category) in [
            ("water_carriers", ResearchCategory::Upgrade),
            ("textiles", ResearchCategory::RecipeResource),
            ("den_insulation", ResearchCategory::Building),
            ("weaponsmithing", ResearchCategory::RecipeResource),
            ("armorsmithing", ResearchCategory::RecipeResource),
        ] {
            assert_eq!(
                catalog.get(node_id).unwrap().category,
                category,
                "{node_id}"
            );
        }
        assert!(catalog.get("den_insulation").unwrap().payloads.iter().any(
            |payload| matches!(payload, ResearchPayload::Modify { effect_id, .. } if effect_id == "housingPerDen")
        ));
        for (node_id, recipe_id) in [
            ("textiles", "fibre_to_cloth"),
            ("textiles", "hide_to_leather"),
            ("weaponsmithing", "smithy_weapon"),
            ("armorsmithing", "smithy_armor"),
        ] {
            assert_eq!(
                catalog
                    .recipe_unlock_study(recipe_id)
                    .map(|node| node.id.as_str()),
                Some(node_id)
            );
        }
    }

    #[test]
    fn planned_building_strings_need_no_sim_enum_variants() {
        let catalog = research_catalog();
        for expected in ["accounting_tent", "mill", "sawmill"] {
            assert!(catalog.nodes().iter().any(|node| node.payloads.iter().any(
                |payload| matches!(payload, ResearchPayload::UnlockBuilding { building_id } if building_id == expected)
            )), "missing planned building {expected}");
        }
    }

    #[test]
    fn building_research_targets_approved_buildings_with_typed_improvements() {
        let catalog = research_catalog();
        let approved: BTreeSet<&str> = APPROVED_BUILDING_IDS.iter().copied().collect();
        let mut targeted = BTreeSet::new();
        let mut attributes = BTreeSet::new();
        for node in catalog
            .nodes()
            .iter()
            .filter(|node| node.category == ResearchCategory::Building)
        {
            for payload in &node.payloads {
                match payload {
                    ResearchPayload::UnlockBuilding { building_id } => {
                        assert!(approved.contains(building_id.as_str()), "{}", node.id);
                        targeted.insert(building_id.as_str());
                    }
                    ResearchPayload::ModifyBuilding {
                        building_id,
                        attribute,
                        ..
                    } => {
                        assert!(approved.contains(building_id.as_str()), "{}", node.id);
                        targeted.insert(building_id.as_str());
                        attributes.insert(*attribute);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(targeted, approved);
        assert_eq!(
            attributes.len(),
            5,
            "all typed building attributes are represented"
        );
    }

    #[test]
    fn generated_research_uses_named_families_and_approved_effects() {
        let approved_effects: BTreeSet<&str> = APPROVED_EFFECT_IDS.iter().copied().collect();
        for node in &research_catalog().nodes()[23..] {
            assert!(!node.id.contains("catalog"), "generic id {}", node.id);
            assert!(!node.name.contains("Pattern"), "generic name {}", node.name);
            assert!(
                !node.name.contains("Practice"),
                "generic name {}",
                node.name
            );
            assert!(!node.name.contains("Method"), "generic name {}", node.name);
            for payload in &node.payloads {
                if let ResearchPayload::Modify { effect_id, .. } = payload {
                    assert!(approved_effects.contains(effect_id.as_str()), "{effect_id}");
                }
            }
        }
    }

    #[test]
    fn catalog_nodes_unlock_every_maintained_station_recipe_by_stable_id() {
        for (node_id, recipe_id) in [
            ("grain_milling_preparation", "grain_to_flour"),
            ("grain_milling_staples", "flour_to_food"),
            ("carpentry_preparation", "logs_to_lumber"),
            ("carpentry_staples", "logs_to_planks"),
            ("stonecraft_preparation", "stone_to_blocks"),
            ("toolmaking_preparation", "planks_and_blocks_to_tools"),
            ("toolmaking_staples", "smithy_tool"),
            ("metallurgy_preparation", "ore_to_metal"),
            ("trade_goods_preparation", "materials_to_refined"),
            ("hunting_preparation", "bone_trinket"),
            ("hunting_staples", "bone_toy"),
            ("toolmaking_quality", "bone_tool"),
            ("stonecraft_staples", "clay_mug"),
            ("stonecraft_quality", "clay_bowl"),
            ("stonecraft_specialty", "clay_brick"),
            ("trade_goods_staples", "gem_jewelry"),
            ("trade_goods_quality", "sand_glass_mug"),
            ("trade_goods_specialty", "sand_glass_bowl"),
            ("trade_goods_masterwork", "sand_glass_trinket"),
            ("textiles", "fibre_to_cloth"),
            ("textiles", "hide_to_leather"),
            ("weaponsmithing", "smithy_weapon"),
            ("armorsmithing", "smithy_armor"),
        ] {
            let node = research_catalog().get(node_id).expect("study exists");
            assert!(
                node.payloads.iter().any(|payload| {
                    payload
                        == &ResearchPayload::UnlockRecipe {
                            recipe_id: recipe_id.to_owned(),
                        }
                }),
                "{node_id} must bind {recipe_id} rather than a parallel registry id"
            );
            assert_eq!(
                research_catalog()
                    .recipe_unlock_study(recipe_id)
                    .map(|node| node.id.as_str()),
                Some(node_id)
            );
        }
    }

    #[test]
    fn activated_food_plant_breadth_is_runtime_while_other_promises_remain_future() {
        let catalog = research_catalog();
        assert!(
            !catalog
                .get("baking_preparation")
                .unwrap()
                .is_future_content()
        );
        assert!(!catalog.get("brewing_bulk").unwrap().is_future_content());
        assert!(catalog.get("hunting_sources").unwrap().is_future_content());
        assert!(
            !catalog
                .get("grain_milling_preparation")
                .unwrap()
                .is_future_content()
        );
        assert!(
            !catalog
                .get("grain_milling_staples")
                .unwrap()
                .is_future_content()
        );
        assert!(
            !catalog
                .get("toolmaking_staples")
                .unwrap()
                .is_future_content()
        );
    }

    #[test]
    fn unsupported_generated_breadth_count_is_explicit_and_regression_guarded() {
        let catalog = research_catalog();
        let unsupported_recipes = catalog
            .nodes()
            .iter()
            .flat_map(|node| &node.payloads)
            .filter(|payload| {
                matches!(
                    payload,
                    ResearchPayload::UnlockRecipe { recipe_id }
                        if !crate::station_recipes::is_runtime_recipe_id(recipe_id)
                )
            })
            .count();
        let unsupported_resources = catalog
            .nodes()
            .iter()
            .flat_map(|node| &node.payloads)
            .filter(|payload| {
                matches!(payload, ResearchPayload::UnlockResource { resource_id }
                    if !is_runtime_resource_unlock_id(resource_id))
            })
            .count();

        assert_eq!(unsupported_recipes, 58);
        assert_eq!(unsupported_resources, 47);
    }

    #[test]
    fn every_food_plant_family_stage_has_an_observable_runtime_consumer() {
        let catalog = research_catalog();
        let mut activated = 0;
        for family in [
            "grain_milling_",
            "baking_",
            "herbalism_",
            "food_preservation_",
            "brewing_",
        ] {
            for node in catalog
                .nodes()
                .iter()
                .filter(|node| node.id.starts_with(family))
            {
                assert!(!node.is_future_content(), "{} remained FUTURE", node.id);
                activated += 1;
            }
        }
        assert_eq!(activated, 42);
    }

    #[test]
    fn prerequisite_checks_use_and_semantics() {
        let catalog = research_catalog();
        let node = catalog
            .nodes()
            .iter()
            .find(|node| node.prerequisites.len() >= 2)
            .expect("template emits AND prerequisites");
        let first = node.prerequisites[0].as_str();
        assert!(!catalog.prerequisites_met(&node.id, |id| id == first));
        assert!(catalog.prerequisites_met(&node.id, |id| {
            node.prerequisites
                .iter()
                .any(|prerequisite| prerequisite == id)
        }));
        assert!(!catalog.prerequisites_met("missing", |_| true));
    }

    #[test]
    fn rebuilding_embedded_sources_is_deterministic() {
        let left = build_catalog(LEGACY_SOURCE, TRACK_SOURCE).unwrap();
        let right = build_catalog(LEGACY_SOURCE, TRACK_SOURCE).unwrap();
        assert_eq!(left.nodes, right.nodes);
        for node in &left.nodes {
            assert_eq!(left.by_id.get(&node.id), right.by_id.get(&node.id));
        }
        assert_eq!(left.recipe_unlocks, right.recipe_unlocks);
    }
}
