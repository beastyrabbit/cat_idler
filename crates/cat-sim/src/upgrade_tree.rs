//! God / cat upgrade tree rules ported from `lib/game/upgradeTree.ts`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::BuildingType;

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

pub const UPGRADE_NODES: &[UpgradeNode] = &[
    UpgradeNode {
        id: "research_hut",
        name: "Research Hut",
        description: "Build the research hut and assign a scholar. The root of the whole tree — nothing is researched until a mouth is spared to study.",
        era: 1,
        cost: 5.0,
        prerequisites: &[],
        unlocks: UpgradeUnlocks {
            buildings: Some(&["research_hut"]),
            jobs: Some(&["research"]),
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
            jobs: Some(&["fetch_water"]),
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
        id: "sawmill",
        name: "Sawmill",
        description: "Raise the Sägewerk. Felled timber becomes usable materials far faster.",
        era: 2,
        cost: 12.0,
        prerequisites: &["foraging_lore"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&["sawmill"]),
            jobs: Some(&["quarry"]),
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
        id: "smithy",
        name: "Smithy",
        description: "Build the smithy. Metal tools open the path to weapons.",
        era: 2,
        cost: 15.0,
        prerequisites: &["sawmill"],
        unlocks: UpgradeUnlocks {
            buildings: Some(&[BuildingType::Smithy.as_str()]),
            jobs: Some(&["forge_tools"]),
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
            jobs: Some(&["train_warrior"]),
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
            jobs: Some(&["teach"]),
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
            jobs: Some(&["forge_weapon"]),
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
            jobs: Some(&["forge_armor"]),
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
            jobs: Some(&["explore"]),
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
            if get_node(id).is_some() && !owned_node_ids.iter().any(|owned| owned == id) {
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

#[must_use]
pub fn prerequisites_met(state: &UpgradeTreeState, id: &str) -> bool {
    let Some(node) = get_node(id) else {
        return false;
    };

    node.prerequisites
        .iter()
        .all(|prerequisite| is_owned(state, prerequisite))
}

#[must_use]
pub fn can_unlock(state: &UpgradeTreeState, id: &str) -> bool {
    get_node(id).is_some() && !is_owned(state, id) && prerequisites_met(state, id)
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
pub const RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK: f64 = 10.0;
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
pub fn next_research_target(state: &UpgradeTreeState) -> Option<&'static UpgradeNode> {
    let mut best = None;
    for node in unlockable_nodes(state) {
        if best.is_none_or(|current: &UpgradeNode| {
            node.cost < current.cost || (node.cost == current.cost && node.id < current.id)
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

#[must_use]
pub fn cat_auto_unlock(state: &UpgradeTreeState) -> AutoUnlockResult {
    let mut best = None;
    for node in unlockable_nodes(state) {
        if node.cost > state.research_points {
            continue;
        }
        if best.is_none_or(|current: &UpgradeNode| {
            node.cost < current.cost || (node.cost == current.cost && node.id < current.id)
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
            owned_node_ids: with_owned(state, node.id),
            research_points: state.research_points - node.cost,
        },
        node_id: Some(node.id.to_owned()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    pub water_carry_capacity: f64,
}

impl ResolvedEffects {
    fn add(&mut self, key: EffectKey, value: f64) {
        match key {
            EffectKey::HuntYieldMult => self.hunt_yield_mult += value,
            EffectKey::GatherYieldMult => self.gather_yield_mult += value,
            EffectKey::MaterialYieldMult => self.material_yield_mult += value,
            EffectKey::FarmYieldMult => self.farm_yield_mult += value,
            EffectKey::MoveSpeedMult => self.move_speed_mult += value,
            EffectKey::CombatPowerMult => self.combat_power_mult += value,
            EffectKey::DefenseMult => self.defense_mult += value,
            EffectKey::ResearchRateMult => self.research_rate_mult += value,
            EffectKey::StoragePerLevelMult => self.storage_per_level_mult += value,
            EffectKey::HousingPerDen => self.housing_per_den += value,
            EffectKey::WaterCarryCapacity => self.water_carry_capacity += value,
        }
    }

    fn resolve_mults(self) -> Self {
        Self {
            hunt_yield_mult: 1.0 + self.hunt_yield_mult,
            gather_yield_mult: 1.0 + self.gather_yield_mult,
            material_yield_mult: 1.0 + self.material_yield_mult,
            farm_yield_mult: 1.0 + self.farm_yield_mult,
            move_speed_mult: 1.0 + self.move_speed_mult,
            combat_power_mult: 1.0 + self.combat_power_mult,
            defense_mult: 1.0 + self.defense_mult,
            research_rate_mult: 1.0 + self.research_rate_mult,
            storage_per_level_mult: 1.0 + self.storage_per_level_mult,
            housing_per_den: self.housing_per_den,
            water_carry_capacity: self.water_carry_capacity,
        }
    }
}

#[must_use]
pub const fn neutral_effects() -> ResolvedEffects {
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
        water_carry_capacity: 0.0,
    }
}

fn zero_effect_sums() -> ResolvedEffects {
    ResolvedEffects {
        hunt_yield_mult: 0.0,
        gather_yield_mult: 0.0,
        material_yield_mult: 0.0,
        farm_yield_mult: 0.0,
        move_speed_mult: 0.0,
        combat_power_mult: 0.0,
        defense_mult: 0.0,
        research_rate_mult: 0.0,
        storage_per_level_mult: 0.0,
        housing_per_den: 0.0,
        water_carry_capacity: 0.0,
    }
}

#[must_use]
pub fn resolve_effects<I, S>(owned_node_ids: I) -> ResolvedEffects
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut sums = zero_effect_sums();
    for id in owned_node_ids {
        let Some(node) = get_node(id.as_ref()) else {
            continue;
        };
        let Some(effects) = node.unlocks.effects else {
            continue;
        };
        for effect in effects {
            sums.add(effect.key, effect.value);
        }
    }

    sums.resolve_mults()
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
        EffectKey, EffectKind, PurchaseFailureReason, RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK,
        RESEARCH_POINTS_PER_SECOND, UPGRADE_NODE_BY_ID, UPGRADE_NODES, WEEK_SECONDS,
        accrue_research, can_unlock, cat_auto_unlock, create_upgrade_tree_state,
        deserialize_upgrade_tree_state, effect_kind, get_node, god_purchase, neutral_effects,
        next_research_target, points_per_tick_for, points_per_tick_for_default, prerequisites_met,
        resolve_effects, serialize_upgrade_tree_state, unlockable_nodes,
    };
    use crate::types::BuildingType;

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
    fn node_table_matches_the_typescript_tree_shape() {
        assert_eq!(UPGRADE_NODES.len(), 18);
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
            Some(&["research_hut"][..])
        );
        assert_eq!(
            get_node("sawmill").expect("sawmill node").unlocks.jobs,
            Some(&["quarry"][..])
        );
        assert_eq!(
            get_node("irrigation")
                .expect("irrigation node")
                .unlocks
                .buildings,
            Some(&[BuildingType::Field.as_str()][..])
        );
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
        let one_researcher_hour = 10.0 / 168.0;
        close(points_per_tick_for_default(1.0, hour), one_researcher_hour);
        close(
            points_per_tick_for_default(2.0, hour),
            one_researcher_hour * 2.0,
        );
        close(
            points_per_tick_for(1.0, hour * 2.0, 1.0),
            one_researcher_hour * 2.0,
        );
        assert_eq!(points_per_tick_for(1.0, WEEK_SECONDS, 1.5), 15.0);
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
    fn cat_auto_unlock_and_next_target_are_cheapest_then_id_deterministic() {
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
            next_research_target(&broke).map(|node| node.id),
            Some("basic_tools")
        );
        let all_owned = UPGRADE_NODES.iter().map(|node| node.id).collect::<Vec<_>>();
        assert!(next_research_target(&state_with(&all_owned, 0.0)).is_none());
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
        assert_eq!(resolved, neutral_effects());
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
}
