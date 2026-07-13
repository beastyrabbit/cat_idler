//! Expanded research catalog for the post-cutover design.
//!
//! The 23 legacy nodes from `upgrade_tree.rs` are owned records in an embedded
//! data file. Compact, named family templates deterministically expand the rest
//! of the 500-node graph. This module is deliberately additive: it performs no
//! research ticks and grants no unlocks until later integration slices consume
//! its typed payloads.

use std::{
    collections::{BTreeSet, HashMap},
    hash::{BuildHasherDefault, Hasher},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

pub const RESEARCH_NODE_COUNT: usize = 500;
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
    root_prerequisites: Vec<String>,
    era_start: u8,
    cost_base: f64,
    layout_x: i32,
    leader_priority_base: u16,
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
    root_prerequisites: Vec<String>,
    era_start: u8,
    cost_base: f64,
    layout_x: i32,
    leader_priority_base: u16,
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
        let id = format!("{}_{}", family.building_id, stage.id);
        let prerequisites = if index == 0 {
            family.root_prerequisites.clone()
        } else {
            vec![nodes[index - 1].id.clone()]
        };
        let era_offset = u8::try_from(index / 2)
            .map_err(|_| format!("building era overflow for {}", family.building_id))?;
        let era = family
            .era_start
            .checked_add(era_offset)
            .ok_or_else(|| format!("building era overflow for {}", family.building_id))?;
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
        nodes.push(ResearchNode {
            id,
            name: format!("{} {}", family.display_name, stage.name),
            description: format!("{} {}", family.display_name, stage.description),
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
            payloads: vec![payload],
        });
    }
    Ok(nodes)
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
        let prerequisites = if index == 0 {
            family.root_prerequisites.clone()
        } else {
            vec![nodes[index - 1].id.clone()]
        };
        let payload_id = format!("{}_{}", family.id, stage.id);
        let payload = match stage.payload {
            RecipePayloadKind::Recipe => ResearchPayload::UnlockRecipe {
                recipe_id: payload_id,
            },
            RecipePayloadKind::Resource => ResearchPayload::UnlockResource {
                resource_id: payload_id,
            },
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
            category: ResearchCategory::Upgrade,
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
    }

    validate_categories(&nodes)?;
    validate_topology(&nodes, &by_id)?;
    Ok(ResearchCatalog { nodes, by_id })
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
            ResearchPayload::UnlockBuilding { building_id }
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
                ResearchPayload::UnlockBuilding { .. } | ResearchPayload::ModifyBuilding { .. }
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
    if buildings * 3 < nodes.len() || recipes * 3 < nodes.len() {
        return Err("building and recipe/resource categories must each fill one third".to_owned());
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
        assert_eq!(catalog.nodes().len(), 500);
        assert_eq!(catalog.category_count(ResearchCategory::Building), 167);
        assert_eq!(
            catalog.category_count(ResearchCategory::RecipeResource),
            167
        );
        assert_eq!(catalog.category_count(ResearchCategory::Upgrade), 166);
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
        assert_eq!(ids.get(23), Some(&"den_foundations"));
        assert_eq!(ids.get(181), Some(&"sawmill_reinforcement"));
        assert_eq!(ids.get(182), Some(&"hunting_sources"));
        assert_eq!(ids.get(345), Some(&"expedition_supplies_masterwork"));
        assert_eq!(ids.get(346), Some(&"logistics_basics"));
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
    fn all_legacy_nodes_are_byte_faithful_in_identity_cost_edges_and_payload_intent() {
        let catalog = research_catalog();
        assert_eq!(
            UPGRADE_NODES.len(),
            23,
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
            let jobs: Vec<&str> = node
                .payloads
                .iter()
                .filter_map(|payload| match payload {
                    ResearchPayload::UnlockJob { job_id } => Some(job_id.as_str()),
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
            assert_eq!(jobs, legacy.unlocks.jobs.unwrap_or_default());
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
    }
}
