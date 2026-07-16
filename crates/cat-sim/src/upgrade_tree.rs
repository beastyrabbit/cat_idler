//! God / cat upgrade tree rules ported from `lib/game/upgradeTree.ts`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    research_catalog::{
        BuildingAttribute, EffectOperation, ResearchNode as CatalogNode, ResearchPayload,
        research_catalog, research_node_is_implemented,
    },
    types::{BuildingType, JobKind},
};

/// Every modifier a node can grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectKey {
    #[serde(rename = "huntYieldMult")]
    HuntYieldMult,
    #[serde(rename = "gatherYieldMult")]
    GatherYieldMult,
    #[serde(rename = "materialYieldMult")]
    MaterialYieldMult,
    #[serde(rename = "farmYieldMult")]
    FarmYieldMult,
    #[serde(rename = "moveSpeedMult")]
    MoveSpeedMult,
    #[serde(rename = "combatPowerMult")]
    CombatPowerMult,
    #[serde(rename = "defenseMult")]
    DefenseMult,
    #[serde(rename = "researchRateMult")]
    ResearchRateMult,
    #[serde(rename = "storagePerLevelMult")]
    StoragePerLevelMult,
    #[serde(rename = "housingPerDen")]
    HousingPerDen,
    #[serde(rename = "waterCarryCapacity")]
    WaterCarryCapacity,
}

impl EffectKey {
    pub const ALL: &'static [Self] = &[
        Self::HuntYieldMult,
        Self::GatherYieldMult,
        Self::MaterialYieldMult,
        Self::FarmYieldMult,
        Self::MoveSpeedMult,
        Self::CombatPowerMult,
        Self::DefenseMult,
        Self::ResearchRateMult,
        Self::StoragePerLevelMult,
        Self::HousingPerDen,
        Self::WaterCarryCapacity,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HuntYieldMult => "huntYieldMult",
            Self::GatherYieldMult => "gatherYieldMult",
            Self::MaterialYieldMult => "materialYieldMult",
            Self::FarmYieldMult => "farmYieldMult",
            Self::MoveSpeedMult => "moveSpeedMult",
            Self::CombatPowerMult => "combatPowerMult",
            Self::DefenseMult => "defenseMult",
            Self::ResearchRateMult => "researchRateMult",
            Self::StoragePerLevelMult => "storagePerLevelMult",
            Self::HousingPerDen => "housingPerDen",
            Self::WaterCarryCapacity => "waterCarryCapacity",
        }
    }

    /// Runtime systems that consume this resolved scalar effect.
    ///
    /// Keep this match exhaustive: adding a legacy scalar effect must name at least one
    /// truthful gameplay path instead of stopping at catalog resolution. Planned string-keyed
    /// unlock registries and per-building future modifiers are intentionally outside this map.
    #[must_use]
    pub const fn runtime_consumers(self) -> &'static [&'static str] {
        match self {
            Self::HuntYieldMult => &["physical hunt load"],
            Self::GatherYieldMult => &["explicit fibre forage"],
            Self::MaterialYieldMult => &["physical logging load", "physical quarry load"],
            Self::FarmYieldMult => &["field and farm-plot harvest"],
            Self::MoveSpeedMult => &["cat world movement"],
            Self::CombatPowerMult => &["raid combat power"],
            Self::DefenseMult => &["raid defense power"],
            Self::ResearchRateMult => &["staffed research accrual"],
            Self::StoragePerLevelMult => &["legacy storage-building capacity"],
            Self::HousingPerDen => &["den housing capacity"],
            Self::WaterCarryCapacity => &["physical water-fetch load"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    Mult,
    Add,
}

#[must_use]
pub const fn effect_kind(key: EffectKey) -> EffectKind {
    match key {
        EffectKey::HuntYieldMult
        | EffectKey::GatherYieldMult
        | EffectKey::MaterialYieldMult
        | EffectKey::FarmYieldMult
        | EffectKey::MoveSpeedMult
        | EffectKey::CombatPowerMult
        | EffectKey::DefenseMult
        | EffectKey::ResearchRateMult
        | EffectKey::StoragePerLevelMult => EffectKind::Mult,
        EffectKey::HousingPerDen | EffectKey::WaterCarryCapacity => EffectKind::Add,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEffect {
    pub key: EffectKey,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeUnlocks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buildings: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jobs: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<&'static [NodeEffect]>,
}

pub type UpgradeEra = u8;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeNode {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub era: UpgradeEra,
    pub cost: f64,
    pub prerequisites: &'static [&'static str],
    pub unlocks: UpgradeUnlocks,
}

/// Tech node that makes mountain-biome tiles passable (slow) to pathfinding.
/// Owned via god blessing or cat auto-research like any other node.
pub const MOUNTAINEERING_NODE_ID: &str = "mountaineering";

/// Tech node that grants rail blueprints. Ownership alone is intentionally
/// neutral until physical tracks, rolling stock, boarding, and routes exist.
pub const RAIL_NODE_ID: &str = "rail";

/// Tech node that grants shipping blueprints. Ownership alone is intentionally
/// neutral until physical vessels, docks, boarding, and routes exist.
pub const SHIPPING_NODE_ID: &str = "shipping";

/// Tech node that unlocks the smelter building (P17/P19 ore→metal chain). Its
/// catalog payload targets the wire building id (`smelter`), so the shared
/// catalog-derived placement resolver handles the differing node id.
pub const SMELTING_NODE_ID: &str = "smelting";

pub const UPGRADE_NODES: &[UpgradeNode] = &[
    UpgradeNode {
        id: "research_hut",
        name: "Research Hut",
        description: "The research hut is available from founding. Assign a scholar, then codify the colony's first study — nothing advances until a mouth is spared to learn.",
        era: 1,
        cost: 5.0,
        prerequisites: &[],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "basic_tools",
        name: "Basic Tools",
        description: "Knapped claws and better snares. Hunters bring back more.",
        era: 1,
        cost: 5.0,
        prerequisites: &["research_hut"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::HuntYieldMult,
                value: 0.1,
            }]),
        },
    },
    UpgradeNode {
        id: "water_carriers",
        name: "Water Carriers",
        description: "Woven gourds let a fetch-water trip haul far more per run.",
        era: 1,
        cost: 8.0,
        prerequisites: &["research_hut"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::WaterCarryCapacity,
                value: 1.0,
            }]),
        },
    },
    UpgradeNode {
        id: "den_insulation",
        name: "Den Insulation",
        description: "Moss-lined dens shelter another cat each without the chill.",
        era: 1,
        cost: 8.0,
        prerequisites: &["research_hut"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::HousingPerDen,
                value: 1.0,
            }]),
        },
    },
    UpgradeNode {
        id: "foraging_lore",
        name: "Foraging Lore",
        description: "Elders teach which berries feed and which kill.",
        era: 1,
        cost: 6.0,
        prerequisites: &["basic_tools"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::GatherYieldMult,
                value: 0.15,
            }]),
        },
    },
    UpgradeNode {
        id: "textiles",
        name: "Textiles",
        description: "Raise the clothier and tannery. Fibre and hunt hides become cloth, leather, and warm clothing.",
        era: 2,
        cost: 12.0,
        prerequisites: &["foraging_lore"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[
                BuildingType::Clothier.as_str(),
                BuildingType::Tannery.as_str(),
            ]),
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "sawmill",
        name: "Sawmill",
        description: "Raise the Sägewerk. Felled timber becomes usable materials far faster.",
        era: 2,
        cost: 12.0,
        prerequisites: &["foraging_lore"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Sawmill.as_str()]),
            jobs: Some(&[JobKind::GatherLogs.as_str()]),
            effects: Some(&[NodeEffect {
                key: EffectKey::MaterialYieldMult,
                value: 0.2,
            }]),
        },
    },
    UpgradeNode {
        id: "masonry",
        name: "Masonry",
        description: "Stacked stone stores. Every storehouse level holds more.",
        era: 2,
        cost: 12.0,
        prerequisites: &["sawmill"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::StoragePerLevelMult,
                value: 0.25,
            }]),
        },
    },
    UpgradeNode {
        id: MOUNTAINEERING_NODE_ID,
        name: "Mountaineering",
        description: "Pitons, ropes, and cut switchback trails. Cats can finally cross the peaks — slow going, but the high ore is within reach.",
        era: 2,
        cost: 15.0,
        prerequisites: &["masonry"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: SMELTING_NODE_ID,
        name: "Smelting",
        description: "A proper forge-hearth for the ore mountaineering finally opened up. Raw stone off the peak comes back down as metal bars.",
        era: 3,
        cost: 22.0,
        prerequisites: &[MOUNTAINEERING_NODE_ID],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Smelter.as_str()]),
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "smithy",
        name: "Smithy",
        description: "Build the smithy. Metal tools open the path to weapons.",
        era: 2,
        cost: 15.0,
        prerequisites: &["sawmill"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Smithy.as_str()]),
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "barracks",
        name: "Barracks",
        description: "Raise the barracks so cats can drill into real warriors.",
        era: 2,
        cost: 18.0,
        prerequisites: &["basic_tools"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Barracks.as_str()]),
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "school",
        name: "School",
        description: "Build the school. Kittens sit and learn, feeding the research effort while they grow.",
        era: 2,
        cost: 15.0,
        prerequisites: &["den_insulation"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&["school"]),
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::ResearchRateMult,
                value: 0.5,
            }]),
        },
    },
    UpgradeNode {
        id: "irrigation",
        name: "Irrigation",
        description: "Dug channels feed the fields. Crops come in heavier.",
        era: 2,
        cost: 10.0,
        prerequisites: &["water_carriers"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Field.as_str()]),
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::FarmYieldMult,
                value: 0.2,
            }]),
        },
    },
    UpgradeNode {
        id: "milling",
        name: "Milling",
        description: "Raise the mill. Grain is ground into flour, then baked into food by the same staffed works.",
        era: 2,
        cost: 14.0,
        prerequisites: &["irrigation"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Mill.as_str()]),
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: "housing_tier_2",
        name: "Timbered Longdens",
        description: "Two-storey dens. Each den now shelters a small clan.",
        era: 3,
        cost: 20.0,
        prerequisites: &["masonry"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::HousingPerDen,
                value: 2.0,
            }]),
        },
    },
    UpgradeNode {
        id: "weaponsmithing",
        name: "Weaponsmithing",
        description: "Forge claws of iron. Warriors strike far harder.",
        era: 3,
        cost: 22.0,
        prerequisites: &["smithy"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::CombatPowerMult,
                value: 0.25,
            }]),
        },
    },
    UpgradeNode {
        id: "armorsmithing",
        name: "Armorsmithing",
        description: "Hammered plate. Defenders shrug off blows that once felled them.",
        era: 3,
        cost: 22.0,
        prerequisites: &["smithy"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::DefenseMult,
                value: 0.25,
            }]),
        },
    },
    UpgradeNode {
        id: "advanced_storage",
        name: "Advanced Storage",
        description: "Sealed cellars and lofts. Storehouses hold half again as much.",
        era: 3,
        cost: 18.0,
        prerequisites: &["masonry"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::StoragePerLevelMult,
                value: 0.5,
            }]),
        },
    },
    UpgradeNode {
        id: "scholars_guild",
        name: "Scholars' Guild",
        description: "A true academy. Research races ahead.",
        era: 3,
        cost: 25.0,
        prerequisites: &["school"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::ResearchRateMult,
                value: 0.75,
            }]),
        },
    },
    UpgradeNode {
        id: "mounted_scouts",
        name: "Mounted Scouts",
        description: "Trained runners cover far more ground between waypoints.",
        era: 3,
        cost: 20.0,
        prerequisites: &["barracks"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::MoveSpeedMult,
                value: 0.3,
            }]),
        },
    },
    UpgradeNode {
        id: "grand_housing",
        name: "Grand Housing",
        description: "Stone halls. A single den now houses a whole lineage.",
        era: 3,
        cost: 25.0,
        prerequisites: &["housing_tier_2"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: Some(&[NodeEffect {
                key: EffectKey::HousingPerDen,
                value: 3.0,
            }]),
        },
    },
    UpgradeNode {
        id: RAIL_NODE_ID,
        name: "Rail Line",
        description: "Survey grades and draft rail blueprints. Physical tracks, rolling stock, and staffed routes are still required before cats travel faster.",
        era: 3,
        cost: 20.0,
        // Rails follow the graded routes mountaineering's switchbacks already cut,
        // so the line only makes narrative (and pathing) sense once mountains are
        // crossable at all.
        prerequisites: &[MOUNTAINEERING_NODE_ID],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: None,
        },
    },
    UpgradeNode {
        id: SHIPPING_NODE_ID,
        name: "Shipping",
        description: "Draft hull and dock blueprints. Physical vessels and staffed routes are still required before cats can cross open water.",
        era: 3,
        cost: 24.0,
        // Timber for hulls comes from the sawmill's output chain.
        prerequisites: &["sawmill"],
        unlocks: UpgradeUnlocks {
            buildings: None,
            jobs: None,
            effects: None,
        },
    },
];

pub struct UpgradeNodeById;

pub static UPGRADE_NODE_BY_ID: UpgradeNodeById = UpgradeNodeById;

impl UpgradeNodeById {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&'static UpgradeNode> {
        get_node(id)
    }

    #[must_use]
    pub fn contains_key(&self, id: &str) -> bool {
        get_node(id).is_some()
    }
}

#[must_use]
pub fn get_node(id: &str) -> Option<&'static UpgradeNode> {
    UPGRADE_NODES.iter().find(|node| node.id == id)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeTreeState {
    pub owned_node_ids: Vec<String>,
    pub research_points: f64,
}

#[must_use]
pub fn create_upgrade_tree_state() -> UpgradeTreeState {
    UpgradeTreeState {
        owned_node_ids: Vec::new(),
        research_points: 0.0,
    }
}

#[must_use]
pub fn serialize_upgrade_tree_state(state: &UpgradeTreeState) -> UpgradeTreeState {
    UpgradeTreeState {
        owned_node_ids: state.owned_node_ids.clone(),
        research_points: state.research_points,
    }
}

pub trait DeserializeUpgradeTreeInput {
    fn value(&self) -> Option<&Value>;
}

impl DeserializeUpgradeTreeInput for &Value {
    fn value(&self) -> Option<&Value> {
        Some(*self)
    }
}

impl DeserializeUpgradeTreeInput for Value {
    fn value(&self) -> Option<&Value> {
        Some(self)
    }
}

impl DeserializeUpgradeTreeInput for Option<&Value> {
    fn value(&self) -> Option<&Value> {
        *self
    }
}

#[must_use]
pub fn deserialize_upgrade_tree_state(raw: impl DeserializeUpgradeTreeInput) -> UpgradeTreeState {
    let Some(Value::Object(obj)) = raw.value() else {
        return create_upgrade_tree_state();
    };

    let mut owned_node_ids = Vec::new();
    if let Some(Value::Array(ids)) = obj.get("ownedNodeIds") {
        for id in ids {
            let Some(id) = id.as_str() else {
                continue;
            };
            if research_catalog().contains(id) && !owned_node_ids.iter().any(|owned| owned == id) {
                owned_node_ids.push(id.to_owned());
            }
        }
    }

    let research_points = obj
        .get("researchPoints")
        .and_then(Value::as_f64)
        .filter(|points| points.is_finite())
        .map_or(0.0, |points| js_max(0.0, points));

    UpgradeTreeState {
        owned_node_ids,
        research_points,
    }
}

#[must_use]
pub fn is_owned(state: &UpgradeTreeState, id: &str) -> bool {
    state.owned_node_ids.iter().any(|owned| owned == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingPlacementResearch {
    Available,
    Requires {
        node_id: &'static str,
        node_name: &'static str,
    },
}

/// Resolve the catalog's single authoritative research rule for placing a
/// building. A founding marker is available before its node is owned; an unlock
/// payload requires ownership of the node that carries it; buildings without
/// either declaration have no direct catalog research gate.
#[must_use]
pub fn building_placement_research(
    state: &UpgradeTreeState,
    building_id: &str,
) -> BuildingPlacementResearch {
    let mut required = None;
    for node in research_catalog().nodes() {
        for payload in &node.payloads {
            match payload {
                ResearchPayload::BuildingAvailableAtFounding { building_id: id }
                    if id == building_id =>
                {
                    return BuildingPlacementResearch::Available;
                }
                ResearchPayload::UnlockBuilding { building_id: id } if id == building_id => {
                    if is_owned(state, &node.id) {
                        return BuildingPlacementResearch::Available;
                    }
                    required = Some(node);
                }
                _ => {}
            }
        }
    }
    required.map_or(BuildingPlacementResearch::Available, |node| {
        BuildingPlacementResearch::Requires {
            node_id: node.id.as_str(),
            node_name: node.name.as_str(),
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobResearchEntitlement {
    Available,
    Requires {
        node_id: &'static str,
        node_name: &'static str,
    },
}

/// Resolve the catalog's sole research entitlement for a runtime job. Jobs with
/// no `UnlockJob` payload are founding or building capabilities and remain
/// available; an indexed payload requires ownership of its declaring study.
#[must_use]
pub fn job_research_entitlement(state: &UpgradeTreeState, job_id: &str) -> JobResearchEntitlement {
    let Some(node) = research_catalog().job_unlock_study(job_id) else {
        return JobResearchEntitlement::Available;
    };
    if is_owned(state, &node.id) {
        JobResearchEntitlement::Available
    } else {
        JobResearchEntitlement::Requires {
            node_id: node.id.as_str(),
            node_name: node.name.as_str(),
        }
    }
}

#[must_use]
pub fn prerequisites_met(state: &UpgradeTreeState, id: &str) -> bool {
    let Some(node) = research_catalog().get(id) else {
        return false;
    };

    node.prerequisites
        .iter()
        .all(|prerequisite| is_owned(state, prerequisite))
}

#[must_use]
pub fn can_unlock(state: &UpgradeTreeState, id: &str) -> bool {
    research_catalog()
        .get(id)
        .is_some_and(research_node_is_implemented)
        && !is_owned(state, id)
        && prerequisites_met(state, id)
}

/// Every currently available study in stable catalog order.
#[must_use]
pub fn unlockable_catalog_nodes(state: &UpgradeTreeState) -> Vec<&'static CatalogNode> {
    research_catalog()
        .nodes()
        .iter()
        .filter(|node| can_unlock(state, &node.id))
        .collect()
}

#[must_use]
pub fn unlockable_nodes(state: &UpgradeTreeState) -> Vec<&'static UpgradeNode> {
    UPGRADE_NODES
        .iter()
        .filter(|node| can_unlock(state, node.id))
        .collect()
}

fn with_owned(state: &UpgradeTreeState, id: &str) -> Vec<String> {
    let mut owned = state.owned_node_ids.clone();
    owned.push(id.to_owned());
    owned
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PurchaseFailureReason {
    UnknownNode,
    AlreadyOwned,
    PrerequisitesUnmet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseResult {
    pub ok: bool,
    pub state: UpgradeTreeState,
    pub blessings_cost: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<PurchaseFailureReason>,
}

#[must_use]
pub fn god_purchase(state: &UpgradeTreeState, id: &str) -> PurchaseResult {
    let Some(node) = get_node(id) else {
        return PurchaseResult {
            ok: false,
            state: state.clone(),
            blessings_cost: 0.0,
            reason: Some(PurchaseFailureReason::UnknownNode),
        };
    };
    if is_owned(state, id) {
        return PurchaseResult {
            ok: false,
            state: state.clone(),
            blessings_cost: 0.0,
            reason: Some(PurchaseFailureReason::AlreadyOwned),
        };
    }
    if !prerequisites_met(state, id) {
        return PurchaseResult {
            ok: false,
            state: state.clone(),
            blessings_cost: 0.0,
            reason: Some(PurchaseFailureReason::PrerequisitesUnmet),
        };
    }

    PurchaseResult {
        ok: true,
        state: UpgradeTreeState {
            owned_node_ids: with_owned(state, id),
            research_points: state.research_points,
        },
        blessings_cost: node.cost,
        reason: None,
    }
}

/// Target accrual for one full-time researcher.
pub const RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK: f64 = 20.0;
/// Seconds in a week (7 * 24 * 60 * 60).
pub const WEEK_SECONDS: f64 = 604_800.0;
/// Per-second research rate for a single full-time researcher.
pub const RESEARCH_POINTS_PER_SECOND: f64 = RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK / WEEK_SECONDS;

#[must_use]
pub fn points_per_tick_for(researcher_count: f64, elapsed_sec: f64, rate_mult: f64) -> f64 {
    if researcher_count <= 0.0 || elapsed_sec <= 0.0 {
        return 0.0;
    }

    researcher_count * elapsed_sec * RESEARCH_POINTS_PER_SECOND * js_max(0.0, rate_mult)
}

#[must_use]
pub fn points_per_tick_for_default(researcher_count: f64, elapsed_sec: f64) -> f64 {
    points_per_tick_for(researcher_count, elapsed_sec, 1.0)
}

#[must_use]
pub fn accrue_research(state: &UpgradeTreeState, points: f64) -> UpgradeTreeState {
    if !points.is_finite() || points == 0.0 {
        return state.clone();
    }

    UpgradeTreeState {
        owned_node_ids: state.owned_node_ids.clone(),
        research_points: js_max(0.0, state.research_points + points),
    }
}

#[must_use]
pub fn next_research_target(state: &UpgradeTreeState) -> Option<&'static CatalogNode> {
    let mut best = None;
    for node in unlockable_catalog_nodes(state) {
        if best.is_none_or(|current: &CatalogNode| {
            node.leader_priority < current.leader_priority
                || (node.leader_priority == current.leader_priority
                    && (node.cost < current.cost
                        || (node.cost == current.cost && node.id < current.id)))
        }) {
            best = Some(node);
        }
    }
    best
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUnlockResult {
    pub ok: bool,
    pub state: UpgradeTreeState,
    pub node_id: Option<String>,
}

/// Explicit cat-research purchase used by the manual technology action. This is
/// deliberately separate from [`god_purchase`]: it spends banked research points,
/// never shrine blessings, and never silently picks a different node.
#[must_use]
pub fn cat_purchase(state: &UpgradeTreeState, id: &str) -> AutoUnlockResult {
    let Some(node) = research_catalog().get(id) else {
        return AutoUnlockResult {
            ok: false,
            state: state.clone(),
            node_id: None,
        };
    };
    if !can_unlock(state, id) || state.research_points < node.cost {
        return AutoUnlockResult {
            ok: false,
            state: state.clone(),
            node_id: None,
        };
    }

    AutoUnlockResult {
        ok: true,
        state: UpgradeTreeState {
            owned_node_ids: with_owned(state, id),
            research_points: state.research_points - node.cost,
        },
        node_id: Some(id.to_owned()),
    }
}

#[must_use]
pub fn cat_auto_unlock(state: &UpgradeTreeState) -> AutoUnlockResult {
    let mut best = None;
    for node in unlockable_catalog_nodes(state) {
        if node.cost > state.research_points {
            continue;
        }
        if best.is_none_or(|current: &CatalogNode| {
            node.leader_priority < current.leader_priority
                || (node.leader_priority == current.leader_priority
                    && (node.cost < current.cost
                        || (node.cost == current.cost && node.id < current.id)))
        }) {
            best = Some(node);
        }
    }

    let Some(node) = best else {
        return AutoUnlockResult {
            ok: false,
            state: state.clone(),
            node_id: None,
        };
    };

    AutoUnlockResult {
        ok: true,
        state: UpgradeTreeState {
            owned_node_ids: with_owned(state, &node.id),
            research_points: state.research_points - node.cost,
        },
        node_id: Some(node.id.clone()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedEffects {
    pub hunt_yield_mult: f64,
    pub gather_yield_mult: f64,
    pub material_yield_mult: f64,
    pub farm_yield_mult: f64,
    pub move_speed_mult: f64,
    pub combat_power_mult: f64,
    pub defense_mult: f64,
    pub research_rate_mult: f64,
    pub storage_per_level_mult: f64,
    pub housing_per_den: f64,
    pub housing_capacity_mult: f64,
    pub water_carry_capacity: f64,
    pub production_rate_mult: f64,
    pub storage_capacity_mult: f64,
    pub construction_speed_mult: f64,
    pub haul_capacity_mult: f64,
    pub trade_value_mult: f64,
    pub health_recovery_mult: f64,
    pub spoilage_resistance: f64,
    pub water_efficiency_mult: f64,
    pub rest_recovery_mult: f64,
    pub herb_medicine_efficacy_mult: f64,
    pub kitten_growth_mult: f64,
    pub elder_protection_mult: f64,
    pub mouse_farm_food_mult: f64,
    pub shrine_blessing_yield_mult: f64,
    pub accounting_speed_mult: f64,
    pub food_storekeeping: f64,
    pub water_stewardship_mult: f64,
    pub wall_defense_mult: f64,
    pub field_stewardship_mult: f64,
    pub barracks_readiness_mult: f64,
    pub den_stewardship_mult: f64,
    /// String-keyed ownership makes planned content truthful without pretending a
    /// corresponding enum-backed building, recipe, resource, or job exists yet.
    pub unlocked_buildings: BTreeSet<String>,
    pub unlocked_recipes: BTreeSet<String>,
    pub unlocked_resources: BTreeSet<String>,
    pub unlocked_jobs: BTreeSet<String>,
    pub unlocked_capabilities: BTreeSet<String>,
    pub building_modifiers: BTreeMap<String, BuildingModifiers>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingModifiers {
    pub capacity_mult: f64,
    pub output_mult: f64,
    pub cycle_time_mult: f64,
    pub worker_slots: u32,
    pub durability_mult: f64,
}

impl Default for BuildingModifiers {
    fn default() -> Self {
        Self {
            capacity_mult: 1.0,
            output_mult: 1.0,
            cycle_time_mult: 1.0,
            worker_slots: 0,
            durability_mult: 1.0,
        }
    }
}

impl ResolvedEffects {
    fn apply_value(target: &mut f64, operation: EffectOperation, value: f64) {
        match operation {
            EffectOperation::Add => *target += value,
            EffectOperation::Multiply => *target *= value,
        }
    }

    fn apply_named_effect(&mut self, id: &str, operation: EffectOperation, value: f64) {
        let target = match id {
            "huntYieldMult" => &mut self.hunt_yield_mult,
            "gatherYieldMult" => &mut self.gather_yield_mult,
            "materialYieldMult" => &mut self.material_yield_mult,
            "farmYieldMult" | "farmYield" => &mut self.farm_yield_mult,
            "moveSpeedMult" | "movementSpeed" => &mut self.move_speed_mult,
            "combatPowerMult" | "combatPower" => &mut self.combat_power_mult,
            "defenseMult" | "defensePower" => &mut self.defense_mult,
            "researchRateMult" | "researchRate" => &mut self.research_rate_mult,
            "storagePerLevelMult" => &mut self.storage_per_level_mult,
            "housingPerDen" => &mut self.housing_per_den,
            "housingCapacity" => &mut self.housing_capacity_mult,
            "waterCarryCapacity" => &mut self.water_carry_capacity,
            "productionRate" => &mut self.production_rate_mult,
            "storageCapacity" => &mut self.storage_capacity_mult,
            "constructionSpeed" => &mut self.construction_speed_mult,
            "haulCapacity" => &mut self.haul_capacity_mult,
            "tradeValue" => &mut self.trade_value_mult,
            "healthRecovery" => &mut self.health_recovery_mult,
            "spoilageResistance" => &mut self.spoilage_resistance,
            "waterEfficiency" => &mut self.water_efficiency_mult,
            "restRecovery" => &mut self.rest_recovery_mult,
            "herbMedicineEfficacy" => &mut self.herb_medicine_efficacy_mult,
            "kittenGrowth" => &mut self.kitten_growth_mult,
            "elderProtection" => &mut self.elder_protection_mult,
            "mouseFarmFood" => &mut self.mouse_farm_food_mult,
            "shrineBlessingYield" => &mut self.shrine_blessing_yield_mult,
            "accountingSpeed" => &mut self.accounting_speed_mult,
            "foodStorekeeping" => &mut self.food_storekeeping,
            "waterStewardship" => &mut self.water_stewardship_mult,
            "wallDefense" => &mut self.wall_defense_mult,
            "fieldStewardship" => &mut self.field_stewardship_mult,
            "barracksReadiness" => &mut self.barracks_readiness_mult,
            "denStewardship" => &mut self.den_stewardship_mult,
            _ => return,
        };
        Self::apply_value(target, operation, value);
    }

    fn apply_payload(&mut self, payload: &ResearchPayload) {
        match payload {
            ResearchPayload::BuildingAvailableAtFounding { .. } => {}
            ResearchPayload::UnlockBuilding { building_id } => {
                self.unlocked_buildings.insert(building_id.clone());
            }
            ResearchPayload::UnlockRecipe { recipe_id } => {
                self.unlocked_recipes.insert(recipe_id.clone());
            }
            ResearchPayload::UnlockResource { resource_id } => {
                self.unlocked_resources.insert(resource_id.clone());
            }
            ResearchPayload::UnlockJob { job_id } => {
                self.unlocked_jobs.insert(job_id.clone());
            }
            ResearchPayload::UnlockCapability { capability_id } => {
                self.unlocked_capabilities.insert(capability_id.clone());
            }
            ResearchPayload::Modify {
                effect_id,
                operation,
                value,
            } => self.apply_named_effect(effect_id, *operation, *value),
            ResearchPayload::ModifyBuilding {
                building_id,
                attribute,
                operation,
                value,
            } => {
                let modifiers = self
                    .building_modifiers
                    .entry(building_id.clone())
                    .or_default();
                match attribute {
                    BuildingAttribute::Capacity => {
                        Self::apply_value(&mut modifiers.capacity_mult, *operation, *value)
                    }
                    BuildingAttribute::Output => {
                        Self::apply_value(&mut modifiers.output_mult, *operation, *value)
                    }
                    BuildingAttribute::CycleTime => {
                        Self::apply_value(&mut modifiers.cycle_time_mult, *operation, *value)
                    }
                    BuildingAttribute::WorkerSlots => {
                        if *operation == EffectOperation::Add
                            && crate::research_catalog::worker_slots_building_is_implemented(
                                building_id,
                            )
                        {
                            modifiers.worker_slots = modifiers
                                .worker_slots
                                .saturating_add(value.max(0.0).floor() as u32);
                        }
                    }
                    BuildingAttribute::Durability => {
                        Self::apply_value(&mut modifiers.durability_mult, *operation, *value)
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn building(&self, building_id: &str) -> BuildingModifiers {
        self.building_modifiers
            .get(building_id)
            .cloned()
            .unwrap_or_default()
    }

    #[must_use]
    pub fn unlocks_building(&self, building_id: &str) -> bool {
        self.unlocked_buildings.contains(building_id)
    }
}

#[must_use]
pub fn neutral_effects() -> ResolvedEffects {
    ResolvedEffects {
        hunt_yield_mult: 1.0,
        gather_yield_mult: 1.0,
        material_yield_mult: 1.0,
        farm_yield_mult: 1.0,
        move_speed_mult: 1.0,
        combat_power_mult: 1.0,
        defense_mult: 1.0,
        research_rate_mult: 1.0,
        storage_per_level_mult: 1.0,
        housing_per_den: 0.0,
        housing_capacity_mult: 1.0,
        water_carry_capacity: 0.0,
        production_rate_mult: 1.0,
        storage_capacity_mult: 1.0,
        construction_speed_mult: 1.0,
        haul_capacity_mult: 1.0,
        trade_value_mult: 1.0,
        health_recovery_mult: 1.0,
        spoilage_resistance: 0.0,
        water_efficiency_mult: 1.0,
        rest_recovery_mult: 1.0,
        herb_medicine_efficacy_mult: 1.0,
        kitten_growth_mult: 1.0,
        elder_protection_mult: 1.0,
        mouse_farm_food_mult: 1.0,
        shrine_blessing_yield_mult: 1.0,
        accounting_speed_mult: 1.0,
        food_storekeeping: 0.0,
        water_stewardship_mult: 1.0,
        wall_defense_mult: 1.0,
        field_stewardship_mult: 1.0,
        barracks_readiness_mult: 1.0,
        den_stewardship_mult: 1.0,
        unlocked_buildings: BTreeSet::new(),
        unlocked_recipes: BTreeSet::new(),
        unlocked_resources: BTreeSet::new(),
        unlocked_jobs: BTreeSet::new(),
        unlocked_capabilities: BTreeSet::new(),
        building_modifiers: BTreeMap::new(),
    }
}

#[must_use]
pub fn resolve_effects<I, S>(owned_node_ids: I) -> ResolvedEffects
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut resolved = neutral_effects();
    for id in owned_node_ids {
        let Some(node) = research_catalog().get(id.as_ref()) else {
            continue;
        };
        for payload in &node.payloads {
            resolved.apply_payload(payload);
        }
    }
    resolved.spoilage_resistance = resolved.spoilage_resistance.clamp(0.0, 0.95);
    resolved
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        BuildingPlacementResearch, EffectKey, EffectKind, JobResearchEntitlement,
        MOUNTAINEERING_NODE_ID, PurchaseFailureReason, RAIL_NODE_ID,
        RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK, RESEARCH_POINTS_PER_SECOND, SHIPPING_NODE_ID,
        UPGRADE_NODE_BY_ID, UPGRADE_NODES, WEEK_SECONDS, accrue_research,
        building_placement_research, can_unlock, cat_auto_unlock, cat_purchase,
        create_upgrade_tree_state, deserialize_upgrade_tree_state, effect_kind, get_node,
        god_purchase, is_owned, job_research_entitlement, neutral_effects, next_research_target,
        points_per_tick_for, points_per_tick_for_default, prerequisites_met, resolve_effects,
        serialize_upgrade_tree_state, unlockable_nodes,
    };
    use crate::types::{BuildingType, JobKind};

    fn state_with(owned_node_ids: &[&str], research_points: f64) -> super::UpgradeTreeState {
        super::UpgradeTreeState {
            owned_node_ids: owned_node_ids
                .iter()
                .map(|node_id| (*node_id).to_owned())
                .collect(),
            research_points,
        }
    }

    #[test]
    fn mountaineering_node_gates_on_masonry_and_is_ownership_detectable() {
        let node = get_node(MOUNTAINEERING_NODE_ID).expect("mountaineering node exists");
        assert_eq!(node.prerequisites, ["masonry"]);
        assert!(node.unlocks.buildings.is_none());
        assert!(node.unlocks.jobs.is_none());
        assert!(node.unlocks.effects.is_none());

        let without = state_with(&["research_hut"], 0.0);
        assert!(!is_owned(&without, MOUNTAINEERING_NODE_ID));
        let with = state_with(&["research_hut", MOUNTAINEERING_NODE_ID], 0.0);
        assert!(is_owned(&with, MOUNTAINEERING_NODE_ID));
    }

    #[test]
    fn rail_and_shipping_nodes_are_era_3_escalating_cost_with_the_documented_prerequisites() {
        let rail = get_node(RAIL_NODE_ID).expect("rail node exists");
        assert_eq!(rail.era, 3);
        assert_eq!(rail.cost, 20.0);
        assert_eq!(rail.prerequisites, [MOUNTAINEERING_NODE_ID]);
        assert!(rail.unlocks.buildings.is_none());
        assert!(rail.unlocks.jobs.is_none());
        assert!(rail.unlocks.effects.is_none());

        let shipping = get_node(SHIPPING_NODE_ID).expect("shipping node exists");
        assert_eq!(shipping.era, 3);
        assert_eq!(shipping.cost, 24.0);
        assert_eq!(shipping.prerequisites, ["sawmill"]);
        assert!(shipping.unlocks.buildings.is_none());
        assert!(shipping.unlocks.jobs.is_none());
        assert!(shipping.unlocks.effects.is_none());

        // Escalating cost vs. the era-2 gate they build on.
        assert!(
            rail.cost
                > get_node(MOUNTAINEERING_NODE_ID)
                    .expect("mountaineering")
                    .cost
        );
        assert!(shipping.cost > get_node("sawmill").expect("sawmill").cost);

        // Ownership is independently detectable, like mountaineering.
        let without = state_with(&["research_hut"], 0.0);
        assert!(!is_owned(&without, RAIL_NODE_ID));
        assert!(!is_owned(&without, SHIPPING_NODE_ID));
        let with = state_with(&["research_hut", RAIL_NODE_ID, SHIPPING_NODE_ID], 0.0);
        assert!(is_owned(&with, RAIL_NODE_ID));
        assert!(is_owned(&with, SHIPPING_NODE_ID));
    }

    #[test]
    fn node_table_preserves_the_legacy_tree_shape_with_truthful_bootstrap_metadata() {
        // 18 TS-parity nodes + the Rust-side `mountaineering` node (unlocks
        // mountain-tile traversal in pathfinding) + the Rust-side `textiles` node
        // (unlocks the clothier/tannery clothing chain, P16/P19 deferred slice) +
        // the Rust-side `rail`/`shipping` P17 transport-blueprint pair (ownership
        // remains physically neutral until vehicles and routes exist) + the Rust-side `smelting` node
        // (unlocks the smelter building, P17/P19 ore→metal chain). The stable
        // Research Hut root no longer claims it grants a building that is
        // deliberately available before the root study can be purchased.
        assert_eq!(UPGRADE_NODES.len(), 24);
        assert_eq!(EffectKey::ALL.len(), 11);
        assert_eq!(effect_kind(EffectKey::HuntYieldMult), EffectKind::Mult);
        assert_eq!(effect_kind(EffectKey::WaterCarryCapacity), EffectKind::Add);

        assert!(
            UPGRADE_NODES
                .iter()
                .all(|node| (5.0..=25.0).contains(&node.cost))
        );
        assert_eq!(
            UPGRADE_NODES
                .iter()
                .filter(|node| node.prerequisites.is_empty())
                .map(|node| node.id)
                .collect::<Vec<_>>(),
            ["research_hut"]
        );

        for node in UPGRADE_NODES {
            assert_eq!(UPGRADE_NODE_BY_ID.get(node.id), Some(node));
            assert_eq!(get_node(node.id), Some(node));
            for prerequisite in node.prerequisites {
                assert!(UPGRADE_NODE_BY_ID.contains_key(prerequisite));
            }
        }
        assert!(get_node("does-not-exist").is_none());

        assert_eq!(
            get_node("research_hut")
                .expect("research_hut node")
                .unlocks
                .buildings,
            None
        );
        assert_eq!(
            building_placement_research(&create_upgrade_tree_state(), "research_hut"),
            BuildingPlacementResearch::Available
        );
        assert_eq!(
            get_node("sawmill").expect("sawmill node").unlocks.jobs,
            Some(&[JobKind::GatherLogs.as_str()][..])
        );
        assert_eq!(
            get_node("irrigation")
                .expect("irrigation node")
                .unlocks
                .buildings,
            Some(&[BuildingType::Field.as_str()][..])
        );
        let mill = get_node("milling").expect("milling node");
        assert_eq!(mill.prerequisites, ["irrigation"]);
        assert_eq!(
            mill.unlocks.buildings,
            Some(&[BuildingType::Mill.as_str()][..])
        );
        assert!(mill.cost > get_node("irrigation").unwrap().cost);
    }

    #[test]
    fn prerequisites_and_unlocking_match_ts_gating() {
        let fresh = create_upgrade_tree_state();
        assert!(prerequisites_met(&fresh, "research_hut"));
        assert!(!prerequisites_met(&fresh, "basic_tools"));
        assert!(!prerequisites_met(&fresh, "ghost"));
        assert!(can_unlock(&fresh, "research_hut"));
        assert!(!can_unlock(&fresh, "basic_tools"));

        let with_root = state_with(&["research_hut"], 0.0);
        assert!(can_unlock(&with_root, "basic_tools"));
        assert!(!can_unlock(&with_root, "research_hut"));
        assert!(!can_unlock(&with_root, "ghost"));

        let partial = state_with(&["research_hut", "basic_tools", "foraging_lore"], 0.0);
        assert!(can_unlock(&partial, "sawmill"));
        assert!(!can_unlock(&partial, "smithy"));

        let ids = unlockable_nodes(&with_root)
            .into_iter()
            .map(|node| node.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["basic_tools", "water_carriers", "den_insulation"]);
    }

    #[test]
    fn catalog_placement_rule_keeps_the_hut_bootstrapped_and_milling_authoritative() {
        let fresh = create_upgrade_tree_state();
        assert_eq!(
            building_placement_research(&fresh, BuildingType::ResearchHut.as_str()),
            BuildingPlacementResearch::Available
        );
        assert_eq!(
            building_placement_research(&fresh, BuildingType::Mill.as_str()),
            BuildingPlacementResearch::Requires {
                node_id: "milling",
                node_name: "Milling",
            }
        );

        let foundations_only = state_with(&["mill_foundations"], 0.0);
        assert_eq!(
            building_placement_research(&foundations_only, BuildingType::Mill.as_str()),
            BuildingPlacementResearch::Requires {
                node_id: "milling",
                node_name: "Milling",
            },
            "the generated durability study must not become an alternate placement gate"
        );
        let milling = state_with(&["milling"], 0.0);
        assert_eq!(
            building_placement_research(&milling, BuildingType::Mill.as_str()),
            BuildingPlacementResearch::Available
        );
    }

    #[test]
    fn catalog_job_rule_gates_only_logging_on_sawmill() {
        let fresh = create_upgrade_tree_state();
        assert_eq!(
            job_research_entitlement(&fresh, JobKind::GatherLogs.as_str()),
            JobResearchEntitlement::Requires {
                node_id: "sawmill",
                node_name: "Sawmill",
            }
        );
        for founding_job in [JobKind::FetchWater, JobKind::Explore, JobKind::TrainWarrior] {
            assert_eq!(
                job_research_entitlement(&fresh, founding_job.as_str()),
                JobResearchEntitlement::Available
            );
        }
        assert_eq!(
            job_research_entitlement(&state_with(&["sawmill"], 0.0), JobKind::GatherLogs.as_str()),
            JobResearchEntitlement::Available
        );
    }

    #[test]
    fn every_declared_building_has_one_catalog_placement_source() {
        use std::collections::BTreeMap;

        let mut sources = BTreeMap::<&str, (&str, bool)>::new();
        for node in crate::research_catalog::research_catalog().nodes() {
            for payload in &node.payloads {
                let (building_id, founding) = match payload {
                    crate::research_catalog::ResearchPayload::BuildingAvailableAtFounding {
                        building_id,
                    } => (building_id.as_str(), true),
                    crate::research_catalog::ResearchPayload::UnlockBuilding { building_id } => {
                        (building_id.as_str(), false)
                    }
                    _ => continue,
                };
                assert!(
                    sources.insert(building_id, (&node.id, founding)).is_none(),
                    "{building_id} has competing placement declarations"
                );

                let fresh = create_upgrade_tree_state();
                if founding {
                    assert_eq!(
                        building_placement_research(&fresh, building_id),
                        BuildingPlacementResearch::Available
                    );
                } else {
                    assert_eq!(
                        building_placement_research(&fresh, building_id),
                        BuildingPlacementResearch::Requires {
                            node_id: node.id.as_str(),
                            node_name: node.name.as_str(),
                        }
                    );
                    assert_eq!(
                        building_placement_research(
                            &state_with(&[node.id.as_str()], 0.0),
                            building_id
                        ),
                        BuildingPlacementResearch::Available
                    );
                }
            }
        }
        assert_eq!(sources.get("research_hut"), Some(&("research_hut", true)));
        let fresh = create_upgrade_tree_state();
        for (building_id, node_id) in [
            ("wood_cutter", "wood_cutter_foundations"),
            ("stone_prep", "stone_prep_foundations"),
            ("woodworking", "woodworking_foundations"),
        ] {
            assert_eq!(sources.get(building_id), Some(&(node_id, true)));
            assert_eq!(
                building_placement_research(&fresh, building_id),
                BuildingPlacementResearch::Available
            );
            assert_eq!(
                building_placement_research(&state_with(&["basic_tools"], 0.0), building_id),
                BuildingPlacementResearch::Available,
                "basic_tools ownership must not change {building_id} placement"
            );
        }
        assert_eq!(sources.get("mill"), Some(&("milling", false)));
    }

    #[test]
    fn god_purchase_reports_costs_and_failure_reasons() {
        let result = god_purchase(&create_upgrade_tree_state(), "research_hut");
        assert!(result.ok);
        assert_eq!(result.blessings_cost, 5.0);
        assert_eq!(result.state.owned_node_ids, ["research_hut"]);
        assert_eq!(result.state.research_points, 0.0);
        assert_eq!(result.reason, None);

        let owned = state_with(&["research_hut"], 3.0);
        let result = god_purchase(&owned, "research_hut");
        assert!(!result.ok);
        assert_eq!(result.reason, Some(PurchaseFailureReason::AlreadyOwned));
        assert_eq!(result.blessings_cost, 0.0);
        assert_eq!(result.state, owned);

        let result = god_purchase(&create_upgrade_tree_state(), "smithy");
        assert!(!result.ok);
        assert_eq!(
            result.reason,
            Some(PurchaseFailureReason::PrerequisitesUnmet)
        );

        let result = god_purchase(&create_upgrade_tree_state(), "ghost");
        assert!(!result.ok);
        assert_eq!(result.reason, Some(PurchaseFailureReason::UnknownNode));
    }

    #[test]
    fn research_accrual_math_matches_hand_derived_vectors() {
        assert_eq!(WEEK_SECONDS, 604_800.0);
        assert_eq!(
            RESEARCH_POINTS_PER_SECOND,
            RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK / WEEK_SECONDS
        );
        assert_eq!(
            points_per_tick_for_default(1.0, WEEK_SECONDS),
            RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK
        );

        // The accrual is researchers * seconds * (per_week / week_seconds); comparing
        // against an algebraically-simplified literal can differ by 1 ULP, so use a
        // tight tolerance (the mathematical value is what matters).
        let close = |a: f64, b: f64| assert!((a - b).abs() <= 1e-12, "{a} vs {b}");
        let hour = 3_600.0;
        let one_researcher_hour = 20.0 / 168.0;
        close(points_per_tick_for_default(1.0, hour), one_researcher_hour);
        close(
            points_per_tick_for_default(2.0, hour),
            one_researcher_hour * 2.0,
        );
        close(
            points_per_tick_for(1.0, hour * 2.0, 1.0),
            one_researcher_hour * 2.0,
        );
        assert_eq!(points_per_tick_for(1.0, WEEK_SECONDS, 1.5), 30.0);
        assert_eq!(points_per_tick_for(0.0, WEEK_SECONDS, 1.0), 0.0);
        assert_eq!(points_per_tick_for(1.0, 0.0, 1.0), 0.0);
        assert_eq!(points_per_tick_for(-3.0, 10.0, 1.0), 0.0);
        assert_eq!(points_per_tick_for(1.0, WEEK_SECONDS, -2.0), 0.0);
        assert!(points_per_tick_for(1.0, WEEK_SECONDS, f64::NAN).is_nan());

        assert_eq!(
            accrue_research(&state_with(&[], 4.0), 2.5).research_points,
            6.5
        );
        assert_eq!(
            accrue_research(&state_with(&[], 1.0), -5.0).research_points,
            0.0
        );
        assert_eq!(
            accrue_research(&state_with(&[], 4.0), 0.0).research_points,
            4.0
        );
        assert_eq!(
            accrue_research(&state_with(&[], 4.0), f64::INFINITY).research_points,
            4.0
        );
    }

    #[test]
    fn cat_auto_unlock_and_next_target_follow_leader_priority_then_cost_and_id() {
        let result = cat_auto_unlock(&state_with(&[], 6.0));
        assert!(result.ok);
        assert_eq!(result.node_id.as_deref(), Some("research_hut"));
        assert_eq!(result.state.owned_node_ids, ["research_hut"]);
        assert_eq!(result.state.research_points, 1.0);

        let poor = state_with(&["research_hut"], 4.0);
        let result = cat_auto_unlock(&poor);
        assert!(!result.ok);
        assert_eq!(result.node_id, None);
        assert_eq!(result.state, poor);

        let s = state_with(&["research_hut"], 8.0);
        let first = cat_auto_unlock(&s);
        assert_eq!(first.node_id.as_deref(), Some("basic_tools"));
        assert!(!cat_auto_unlock(&first.state).ok);

        let tie = state_with(&["research_hut", "basic_tools", "foraging_lore"], 8.0);
        let result = cat_auto_unlock(&tie);
        assert_eq!(result.node_id.as_deref(), Some("den_insulation"));

        let broke = state_with(&["research_hut"], 0.0);
        assert_eq!(
            next_research_target(&broke).map(|node| node.id.as_str()),
            Some("basic_tools")
        );
        let all_owned = crate::research_catalog::research_catalog()
            .nodes()
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert!(next_research_target(&state_with(&all_owned, 0.0)).is_none());
    }

    #[test]
    fn manual_cat_purchase_spends_points_only_on_the_requested_node() {
        let state = state_with(&["research_hut"], 8.0);
        let bought = cat_purchase(&state, "water_carriers");
        assert!(bought.ok);
        assert_eq!(bought.node_id.as_deref(), Some("water_carriers"));
        assert_eq!(bought.state.research_points, 0.0);
        assert!(is_owned(&bought.state, "water_carriers"));
        assert!(!is_owned(&bought.state, "basic_tools"));

        assert!(!cat_purchase(&state, "irrigation").ok, "prerequisites gate");
        assert!(!cat_purchase(&state, "unknown").ok, "unknown node gate");
        assert!(!cat_purchase(&state_with(&["research_hut"], 7.99), "water_carriers").ok);
    }

    #[test]
    fn resolve_effects_uses_neutral_mults_and_additive_stacking() {
        let resolved = resolve_effects(std::iter::empty::<&str>());
        assert_eq!(resolved, neutral_effects());
        assert_eq!(resolved.hunt_yield_mult, 1.0);
        assert_eq!(resolved.housing_per_den, 0.0);

        let resolved = resolve_effects([
            "basic_tools",
            "den_insulation",
            "housing_tier_2",
            "grand_housing",
        ]);
        assert_eq!(resolved.hunt_yield_mult, 1.1);
        assert_eq!(resolved.housing_per_den, 6.0);

        let resolved = resolve_effects(["school", "scholars_guild"]);
        assert_eq!(resolved.research_rate_mult, 2.25);

        let resolved = resolve_effects(["ghost", "research_hut", "smithy"]);
        assert_eq!(resolved.hunt_yield_mult, 1.0);
        assert!(!resolved.unlocked_buildings.contains("research_hut"));
        assert_eq!(
            building_placement_research(&create_upgrade_tree_state(), "research_hut"),
            BuildingPlacementResearch::Available
        );
        assert!(resolved.unlocked_buildings.contains("smithy"));
        assert!(resolved.unlocked_jobs.is_empty());

        let resolved = resolve_effects(["sawmill"]);
        assert_eq!(
            resolved.unlocked_jobs,
            [JobKind::GatherLogs.as_str().to_owned()].into()
        );
    }

    #[test]
    fn every_resolved_legacy_scalar_effect_names_a_truthful_runtime_consumer() {
        for key in EffectKey::ALL {
            assert!(
                !key.runtime_consumers().is_empty(),
                "{} resolves from research but has no runtime consumer",
                key.as_str()
            );
            assert!(
                key.runtime_consumers()
                    .iter()
                    .all(|consumer| !consumer.trim().is_empty()),
                "{} has a blank runtime consumer",
                key.as_str()
            );
        }

        assert_eq!(
            EffectKey::GatherYieldMult.runtime_consumers(),
            ["explicit fibre forage"]
        );
        assert_eq!(
            EffectKey::MaterialYieldMult.runtime_consumers(),
            ["physical logging load", "physical quarry load"]
        );
    }

    #[test]
    fn state_serialization_deserialization_sanitizes_like_typescript() {
        let state = state_with(&["research_hut", "basic_tools"], 12.5);
        let serialized = serialize_upgrade_tree_state(&state);
        assert_eq!(serialized, state);

        let restored = deserialize_upgrade_tree_state(json!({
            "ownedNodeIds": ["research_hut", "basic_tools"],
            "researchPoints": 12.5
        }));
        assert_eq!(restored, state);

        assert_eq!(
            deserialize_upgrade_tree_state(None),
            create_upgrade_tree_state()
        );
        assert_eq!(
            deserialize_upgrade_tree_state(json!(null)),
            create_upgrade_tree_state()
        );
        assert_eq!(
            deserialize_upgrade_tree_state(json!("nope")),
            create_upgrade_tree_state()
        );
        assert_eq!(
            deserialize_upgrade_tree_state(json!({})),
            create_upgrade_tree_state()
        );

        let restored = deserialize_upgrade_tree_state(json!({
            "ownedNodeIds": ["research_hut", "research_hut", "ghost", 42],
            "researchPoints": -5
        }));
        assert_eq!(restored.owned_node_ids, ["research_hut"]);
        assert_eq!(restored.research_points, 0.0);

        let restored = deserialize_upgrade_tree_state(json!({
            "ownedNodeIds": [],
            "researchPoints": "lots"
        }));
        assert_eq!(restored.research_points, 0.0);
    }

    #[test]
    fn every_implemented_catalog_study_is_buyable_in_dependency_order() {
        let mut state = state_with(&[], 1_000_000.0);
        let implemented_count = crate::research_catalog::research_catalog()
            .nodes()
            .iter()
            .filter(|node| crate::research_catalog::research_node_is_implemented(node))
            .count();
        assert_eq!(implemented_count, 487);
        while state.owned_node_ids.len() < implemented_count {
            let next = crate::research_catalog::research_catalog()
                .nodes()
                .iter()
                .find(|node| can_unlock(&state, &node.id))
                .unwrap_or_else(|| {
                    panic!(
                        "catalog stalled after {} owned studies",
                        state.owned_node_ids.len()
                    )
                });
            let result = cat_purchase(&state, &next.id);
            assert!(result.ok, "{} must be purchasable", next.id);
            state = result.state;
        }
        assert_eq!(state.owned_node_ids.len(), implemented_count);
        assert!(
            crate::research_catalog::research_catalog()
                .nodes()
                .iter()
                .filter(|node| !crate::research_catalog::research_node_is_implemented(node))
                .all(|node| !is_owned(&state, &node.id))
        );
    }

    #[test]
    fn every_catalog_payload_resolves_into_a_truthful_effect_or_unlock_registry() {
        use crate::research_catalog::{ResearchPayload, research_catalog};

        for node in research_catalog().nodes() {
            let resolved = resolve_effects([node.id.as_str()]);
            for payload in &node.payloads {
                match payload {
                    ResearchPayload::BuildingAvailableAtFounding { building_id } => {
                        assert_eq!(
                            building_placement_research(&create_upgrade_tree_state(), building_id),
                            BuildingPlacementResearch::Available
                        );
                    }
                    ResearchPayload::UnlockBuilding { building_id } => {
                        assert!(resolved.unlocked_buildings.contains(building_id));
                    }
                    ResearchPayload::UnlockRecipe { recipe_id } => {
                        assert!(resolved.unlocked_recipes.contains(recipe_id));
                    }
                    ResearchPayload::UnlockResource { resource_id } => {
                        assert!(resolved.unlocked_resources.contains(resource_id));
                    }
                    ResearchPayload::UnlockJob { job_id } => {
                        assert!(resolved.unlocked_jobs.contains(job_id));
                    }
                    ResearchPayload::UnlockCapability { capability_id } => {
                        assert!(resolved.unlocked_capabilities.contains(capability_id));
                    }
                    ResearchPayload::ModifyBuilding { building_id, .. }
                        if crate::research_catalog::research_node_is_implemented(node) =>
                    {
                        assert!(resolved.building_modifiers.contains_key(building_id));
                        assert_ne!(
                            resolved.building(building_id),
                            super::BuildingModifiers::default()
                        );
                    }
                    ResearchPayload::ModifyBuilding { building_id, .. } => {
                        assert_eq!(
                            resolved.building(building_id),
                            super::BuildingModifiers::default(),
                            "future worker study {} must not resolve a hidden effect",
                            node.id
                        );
                    }
                    ResearchPayload::Modify { effect_id, .. } => {
                        let encoded = serde_json::to_value(&resolved).unwrap();
                        assert_ne!(
                            encoded,
                            serde_json::to_value(neutral_effects()).unwrap(),
                            "{effect_id} from {} must alter the resolved model",
                            node.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn physical_recipe_studies_resolve_only_their_authoritative_recipe_ids() {
        for (study, recipe) in [
            ("grain_milling_preparation", "grain_to_flour"),
            ("grain_milling_staples", "flour_to_food"),
            ("carpentry_preparation", "logs_to_lumber"),
            ("metallurgy_preparation", "ore_to_metal"),
            ("trade_goods_preparation", "materials_to_refined"),
            ("toolmaking_staples", "smithy_tool"),
        ] {
            let effects = resolve_effects([study]);
            assert_eq!(
                effects.unlocked_recipes,
                std::collections::BTreeSet::from([recipe.to_owned()])
            );
        }
    }

    #[test]
    fn every_generated_recipe_resource_and_building_service_is_live() {
        let catalog = crate::research_catalog::research_catalog();
        assert_eq!(catalog.nodes().len(), 487);
        assert!(catalog.nodes().iter().all(|node| !node.is_future_content()));
        let textile_sources = catalog.get("textile_work_sources").unwrap();
        let state = state_with(
            &textile_sources
                .prerequisites
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            100.0,
        );
        assert!(can_unlock(&state, "textile_work_sources"));
        let result = cat_purchase(&state, "textile_work_sources");
        assert!(result.ok);
        assert!(is_owned(&result.state, "textile_work_sources"));

        let den_service = catalog.get("den_crews").unwrap();
        assert!(!den_service.is_future_content());
        let service_state = state_with(
            &den_service
                .prerequisites
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            100.0,
        );
        assert!(can_unlock(&service_state, "den_crews"));
        let purchased = cat_purchase(&service_state, "den_crews");
        assert!(purchased.ok);
        assert!(is_owned(&purchased.state, "den_crews"));

        let supported = state_with(
            &[
                "research_hut",
                "basic_tools",
                "foraging_lore",
                "sawmill",
                "masonry",
                "irrigation",
                "grain_milling_sources",
            ],
            100.0,
        );
        let node = crate::research_catalog::research_catalog()
            .get("grain_milling_preparation")
            .unwrap();
        assert!(!node.is_future_content(), "{node:?}");
        assert!(
            prerequisites_met(&supported, "grain_milling_preparation"),
            "prereqs={:?} owned={:?}",
            node.prerequisites,
            supported.owned_node_ids
        );
        assert!(can_unlock(&supported, "grain_milling_preparation"));
    }

    #[test]
    fn generated_ownership_survives_legacy_save_shape() {
        let restored = deserialize_upgrade_tree_state(json!({
            "ownedNodeIds": ["research_hut", "research_hut_foundations", "logistics_basics"],
            "researchPoints": 12.5
        }));
        assert_eq!(
            restored.owned_node_ids,
            [
                "research_hut",
                "research_hut_foundations",
                "logistics_basics"
            ]
        );
        assert_eq!(restored.research_points, 12.5);
    }
}
