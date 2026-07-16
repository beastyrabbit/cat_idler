//! Per-resource storage capacity ported from `lib/game/storage.ts`.

use serde::{Deserialize, Serialize};

use crate::{
    research_catalog::{BuildingAttribute, EffectOperation, ResearchPayload, research_catalog},
    stockpiles::{ResourceKind, Stockpile},
    types::BuildingType,
    upgrade_tree::ResolvedEffects,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageResearchEffects {
    pub storage_per_level_mult: f64,
    pub storage_capacity_mult: f64,
    pub food_storage_capacity_mult: f64,
    pub water_bowl_capacity_mult: f64,
    pub smithy_capacity_mult: f64,
    pub grain_capacity_add: f64,
    pub flour_capacity_add: f64,
    pub food_capacity_add: f64,
    pub preserves_capacity_add: f64,
    pub medicine_capacity_add: f64,
    pub brew_capacity_add: f64,
    pub hide_capacity_add: f64,
    pub bone_capacity_add: f64,
    pub fibre_capacity_add: f64,
    pub herbs_capacity_add: f64,
    pub catnip_capacity_add: f64,
    pub cloth_capacity_add: f64,
    pub leather_capacity_add: f64,
    pub lumber_capacity_add: f64,
    pub planks_capacity_add: f64,
    pub blocks_capacity_add: f64,
    pub metal_capacity_add: f64,
    pub tools_capacity_add: f64,
    pub weapons_capacity_add: f64,
    pub armor_capacity_add: f64,
    pub refined_capacity_add: f64,
}

impl Default for StorageResearchEffects {
    fn default() -> Self {
        Self {
            storage_per_level_mult: 1.0,
            storage_capacity_mult: 1.0,
            food_storage_capacity_mult: 1.0,
            water_bowl_capacity_mult: 1.0,
            smithy_capacity_mult: 1.0,
            grain_capacity_add: 0.0,
            flour_capacity_add: 0.0,
            food_capacity_add: 0.0,
            preserves_capacity_add: 0.0,
            medicine_capacity_add: 0.0,
            brew_capacity_add: 0.0,
            hide_capacity_add: 0.0,
            bone_capacity_add: 0.0,
            fibre_capacity_add: 0.0,
            herbs_capacity_add: 0.0,
            catnip_capacity_add: 0.0,
            cloth_capacity_add: 0.0,
            leather_capacity_add: 0.0,
            lumber_capacity_add: 0.0,
            planks_capacity_add: 0.0,
            blocks_capacity_add: 0.0,
            metal_capacity_add: 0.0,
            tools_capacity_add: 0.0,
            weapons_capacity_add: 0.0,
            armor_capacity_add: 0.0,
            refined_capacity_add: 0.0,
        }
    }
}

impl StorageResearchEffects {
    fn apply(target: &mut f64, operation: EffectOperation, value: f64) {
        match operation {
            EffectOperation::Add => *target += value,
            EffectOperation::Multiply => *target *= value,
        }
    }

    fn building_capacity_mut(&mut self, building_id: &str) -> Option<&mut f64> {
        match building_id {
            "food_storage" => Some(&mut self.food_storage_capacity_mult),
            "water_bowl" => Some(&mut self.water_bowl_capacity_mult),
            "smithy" => Some(&mut self.smithy_capacity_mult),
            _ => None,
        }
    }
}

/// Resolve only the five scalar values consumed by storage capacity. Routing asks for
/// these values frequently; constructing every unrelated unlock set and building
/// modifier map would make physical hauling cost grow with the entire research catalog.
#[must_use]
pub fn resolve_storage_research_effects<I, S>(owned_node_ids: I) -> StorageResearchEffects
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut effects = StorageResearchEffects::default();
    for id in owned_node_ids {
        match id.as_ref() {
            "grain_milling_preservation" => effects.flour_capacity_add += 50.0,
            "grain_milling_reserves" => effects.grain_capacity_add += 100.0,
            "baking_reserves" => effects.food_capacity_add += 100.0,
            "herbalism_preservation" => effects.medicine_capacity_add += 50.0,
            "food_preservation_preservation" => effects.preserves_capacity_add += 50.0,
            "brewing_preservation" => effects.brew_capacity_add += 50.0,
            "hunting_reserves" => {
                effects.food_capacity_add += 50.0;
                effects.hide_capacity_add += 50.0;
                effects.bone_capacity_add += 50.0;
            }
            "foraging_reserves" => {
                effects.fibre_capacity_add += 100.0;
                effects.herbs_capacity_add += 50.0;
                effects.catnip_capacity_add += 50.0;
            }
            "textile_work_preservation" => effects.cloth_capacity_add += 50.0,
            "leatherworking_preservation" => effects.leather_capacity_add += 50.0,
            "carpentry_preservation" => {
                effects.lumber_capacity_add += 50.0;
                effects.planks_capacity_add += 50.0;
            }
            "stonecraft_preservation" => effects.blocks_capacity_add += 50.0,
            "metallurgy_preservation" => effects.metal_capacity_add += 50.0,
            "toolmaking_preservation" => effects.tools_capacity_add += 50.0,
            "weaponcraft_preservation" => effects.weapons_capacity_add += 50.0,
            "armorcraft_preservation" => effects.armor_capacity_add += 50.0,
            "trade_goods_preservation" => effects.refined_capacity_add += 50.0,
            _ => {}
        }
        let Some(node) = research_catalog().get(id.as_ref()) else {
            continue;
        };
        for payload in &node.payloads {
            match payload {
                ResearchPayload::Modify {
                    effect_id,
                    operation,
                    value,
                } => match effect_id.as_str() {
                    "storagePerLevelMult" => StorageResearchEffects::apply(
                        &mut effects.storage_per_level_mult,
                        *operation,
                        *value,
                    ),
                    "storageCapacity" => StorageResearchEffects::apply(
                        &mut effects.storage_capacity_mult,
                        *operation,
                        *value,
                    ),
                    _ => {}
                },
                ResearchPayload::ModifyBuilding {
                    building_id,
                    attribute: BuildingAttribute::Capacity,
                    operation,
                    value,
                } => {
                    if let Some(target) = effects.building_capacity_mut(building_id) {
                        StorageResearchEffects::apply(target, *operation, *value);
                    }
                }
                _ => {}
            }
        }
    }
    effects
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StorageCapacities {
    pub food: f64,
    /// Fresh fish shares the food-storage tier but remains separately visible.
    #[serde(default)]
    pub fish: f64,
    pub water: f64,
    pub herbs: f64,
    /// Farm and mill chain resources.
    #[serde(default)]
    pub catnip: f64,
    #[serde(default)]
    pub grain: f64,
    #[serde(default)]
    pub flour: f64,
    #[serde(default)]
    pub preserves: f64,
    #[serde(default)]
    pub medicine: f64,
    #[serde(default)]
    pub brew: f64,
    pub materials: f64,
    #[serde(default)]
    pub stone: f64,
    pub refined: f64,
    pub weapons: f64,
    pub armor: f64,
    /// Refinement tier (P12.4b): planks/blocks/tools from the wood-cutter,
    /// stone-prep, and woodworking chains.
    pub planks: f64,
    /// New forestry chain, separate from legacy materials/planks.
    #[serde(default)]
    pub logs: f64,
    #[serde(default)]
    pub lumber: f64,
    pub blocks: f64,
    pub tools: f64,
    /// Clothing chain (P16/P19 deferred slice): raw fibre/hide and their
    /// clothier/tannery refines, cloth/leather. Flat base capacity, no granary/smithy
    /// bonus — mirrors `planks`/`blocks`/`tools` above exactly.
    pub fibre: f64,
    pub hide: f64,
    #[serde(default)]
    pub bone: f64,
    pub cloth: f64,
    pub leather: f64,
    /// Ore/metal chain. Flat base capacity like the clothing intermediates.
    #[serde(default)]
    pub ore: f64,
    #[serde(default)]
    pub gem: f64,
    #[serde(default)]
    pub clay: f64,
    #[serde(default)]
    pub sand: f64,
    #[serde(default)]
    pub metal: f64,
}

impl StorageCapacities {
    #[must_use]
    pub fn scaled(self, multiplier: f64) -> Self {
        let multiplier = multiplier.max(0.0);
        Self {
            food: self.food * multiplier,
            fish: self.fish * multiplier,
            water: self.water * multiplier,
            herbs: self.herbs * multiplier,
            catnip: self.catnip * multiplier,
            grain: self.grain * multiplier,
            flour: self.flour * multiplier,
            preserves: self.preserves * multiplier,
            medicine: self.medicine * multiplier,
            brew: self.brew * multiplier,
            materials: self.materials * multiplier,
            stone: self.stone * multiplier,
            refined: self.refined * multiplier,
            weapons: self.weapons * multiplier,
            armor: self.armor * multiplier,
            planks: self.planks * multiplier,
            logs: self.logs * multiplier,
            lumber: self.lumber * multiplier,
            blocks: self.blocks * multiplier,
            tools: self.tools * multiplier,
            fibre: self.fibre * multiplier,
            hide: self.hide * multiplier,
            bone: self.bone * multiplier,
            cloth: self.cloth * multiplier,
            leather: self.leather * multiplier,
            ore: self.ore * multiplier,
            gem: self.gem * multiplier,
            clay: self.clay * multiplier,
            sand: self.sand * multiplier,
            metal: self.metal * multiplier,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GranaryBonus {
    pub food: f64,
    pub herbs: f64,
    pub materials: f64,
    pub refined: f64,
}

/// Minimal building shape the capacity math needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StorageBuilding {
    pub building_type: BuildingType,
    pub construction_progress: f64,
    pub level: Option<f64>,
}

impl StorageBuilding {
    #[must_use]
    pub const fn new(
        building_type: BuildingType,
        construction_progress: f64,
        level: Option<f64>,
    ) -> Self {
        Self {
            building_type,
            construction_progress,
            level,
        }
    }
}

/// Base capacity every settlement starts with, before any storehouses.
pub const BASE_CAPACITY: StorageCapacities = StorageCapacities {
    food: 200.0,
    fish: 200.0,
    water: 200.0,
    herbs: 100.0,
    catnip: 100.0,
    grain: 100.0,
    flour: 100.0,
    preserves: 100.0,
    medicine: 100.0,
    brew: 100.0,
    materials: 100.0,
    stone: 100.0,
    refined: 100.0,
    weapons: 50.0,
    armor: 50.0,
    planks: 100.0,
    logs: 100.0,
    lumber: 100.0,
    blocks: 100.0,
    tools: 100.0,
    fibre: 100.0,
    hide: 100.0,
    bone: 100.0,
    cloth: 100.0,
    leather: 100.0,
    ore: 100.0,
    gem: 100.0,
    clay: 100.0,
    sand: 100.0,
    metal: 100.0,
};

/// Dry goods a single finished granary adds per level.
pub const GRANARY_BONUS: GranaryBonus = GranaryBonus {
    food: 400.0,
    herbs: 100.0,
    materials: 100.0,
    refined: 50.0,
};

/// Extra water a single finished water bowl holds per level.
pub const WATER_BOWL_BONUS: f64 = 200.0;

/// Extra armory capacity a single finished smithy holds per level.
pub const SMITHY_ARMORY_BONUS: f64 = 50.0;

#[must_use]
pub fn storage_capacities_default(buildings: &[StorageBuilding]) -> StorageCapacities {
    storage_capacities(buildings, 1.0)
}

#[must_use]
pub fn storage_capacities(buildings: &[StorageBuilding], storage_mult: f64) -> StorageCapacities {
    storage_capacities_with_building_mult(buildings, storage_mult, |_| 1.0)
}

/// The one authoritative research-aware capacity calculation.
///
/// A building-target capacity payload may only scale storage that the existing
/// storage model physically assigns to that building type. Food stores own the
/// dry-goods domains, water bowls own water, and smithies own armory space. The
/// other catalog building families do not silently become global warehouses.
#[must_use]
pub fn authoritative_storage_capacities(
    buildings: &[StorageBuilding],
    stockpiles: &[Stockpile],
    effects: &ResolvedEffects,
) -> StorageCapacities {
    let researched = research_aware_storage_capacities(buildings, effects);
    // Legacy rows may have only the old unbounded shrine id, while narrow
    // fixtures can omit the seeded storehouse altogether. Reconciliation turns
    // either state into the general storehouse backed by `researched`; do not
    // erase resources before that migration gets a chance to run.
    if !stockpiles.iter().any(|pile| pile.is_general_storehouse()) {
        return researched;
    }
    researched.min(physical_storage_capacities(stockpiles, researched))
}

/// Authoritative capacity from owned node ids without resolving unrelated research
/// payloads. This is equivalent to [`authoritative_storage_capacities`] for every
/// capacity consumer and is the runtime hauling path.
#[must_use]
pub fn authoritative_storage_capacities_for_owned<I, S>(
    buildings: &[StorageBuilding],
    stockpiles: &[Stockpile],
    owned_node_ids: I,
) -> StorageCapacities
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let effects = resolve_storage_research_effects(owned_node_ids);
    let researched = research_aware_storage_capacities_compact(buildings, effects);
    if !stockpiles.iter().any(|pile| pile.is_general_storehouse()) {
        return researched;
    }
    researched.min(physical_storage_capacities(stockpiles, researched))
}

/// Research-aware capacity owned by completed storage buildings, before spatial
/// placement is considered. The general storehouse uses these per-resource values
/// as its real headroom, so a targeted study expands the matching physical store.
#[must_use]
pub fn research_aware_storage_capacities(
    buildings: &[StorageBuilding],
    effects: &ResolvedEffects,
) -> StorageCapacities {
    let mut caps = storage_capacities_with_building_mult(
        buildings,
        effects.storage_per_level_mult,
        |building_type| effects.building(building_type.as_str()).capacity_mult,
    )
    .scaled(effects.storage_capacity_mult);
    apply_food_plant_storage_additions(
        &mut caps,
        effects
            .unlocked_resources
            .contains("grain_milling_reserves"),
        effects
            .unlocked_resources
            .contains("grain_milling_preservation"),
        effects.unlocked_resources.contains("baking_reserves"),
        effects
            .unlocked_resources
            .contains("food_preservation_preservation"),
        effects
            .unlocked_resources
            .contains("herbalism_preservation"),
        effects.unlocked_resources.contains("brewing_preservation"),
    );
    apply_subsistence_frontier_storage_additions(
        &mut caps,
        effects.unlocked_resources.contains("hunting_reserves"),
        effects.unlocked_resources.contains("foraging_reserves"),
    );
    apply_industrial_storage_additions(
        &mut caps,
        effects
            .unlocked_resources
            .contains("textile_work_preservation"),
        effects
            .unlocked_resources
            .contains("leatherworking_preservation"),
        effects
            .unlocked_resources
            .contains("carpentry_preservation"),
        effects
            .unlocked_resources
            .contains("stonecraft_preservation"),
        effects
            .unlocked_resources
            .contains("metallurgy_preservation"),
        effects
            .unlocked_resources
            .contains("toolmaking_preservation"),
        effects
            .unlocked_resources
            .contains("weaponcraft_preservation"),
        effects
            .unlocked_resources
            .contains("armorcraft_preservation"),
        effects
            .unlocked_resources
            .contains("trade_goods_preservation"),
    );
    caps
}

fn research_aware_storage_capacities_compact(
    buildings: &[StorageBuilding],
    effects: StorageResearchEffects,
) -> StorageCapacities {
    let mut caps = storage_capacities_with_building_mult(
        buildings,
        effects.storage_per_level_mult,
        |building_type| match building_type {
            BuildingType::FoodStorage => effects.food_storage_capacity_mult,
            BuildingType::WaterBowl => effects.water_bowl_capacity_mult,
            BuildingType::Smithy => effects.smithy_capacity_mult,
            _ => 1.0,
        },
    )
    .scaled(effects.storage_capacity_mult);
    caps.grain += effects.grain_capacity_add;
    caps.flour += effects.flour_capacity_add;
    caps.food += effects.food_capacity_add;
    caps.preserves += effects.preserves_capacity_add;
    caps.medicine += effects.medicine_capacity_add;
    caps.brew += effects.brew_capacity_add;
    caps.hide += effects.hide_capacity_add;
    caps.bone += effects.bone_capacity_add;
    caps.fibre += effects.fibre_capacity_add;
    caps.herbs += effects.herbs_capacity_add;
    caps.catnip += effects.catnip_capacity_add;
    caps.cloth += effects.cloth_capacity_add;
    caps.leather += effects.leather_capacity_add;
    caps.lumber += effects.lumber_capacity_add;
    caps.planks += effects.planks_capacity_add;
    caps.blocks += effects.blocks_capacity_add;
    caps.metal += effects.metal_capacity_add;
    caps.tools += effects.tools_capacity_add;
    caps.weapons += effects.weapons_capacity_add;
    caps.armor += effects.armor_capacity_add;
    caps.refined += effects.refined_capacity_add;
    caps
}

fn apply_food_plant_storage_additions(
    caps: &mut StorageCapacities,
    grain_reserves: bool,
    flour_preservation: bool,
    baking_reserves: bool,
    preserves_storage: bool,
    medicine_storage: bool,
    brew_storage: bool,
) {
    caps.grain += if grain_reserves { 100.0 } else { 0.0 };
    caps.flour += if flour_preservation { 50.0 } else { 0.0 };
    caps.food += if baking_reserves { 100.0 } else { 0.0 };
    caps.preserves += if preserves_storage { 50.0 } else { 0.0 };
    caps.medicine += if medicine_storage { 50.0 } else { 0.0 };
    caps.brew += if brew_storage { 50.0 } else { 0.0 };
}

fn apply_subsistence_frontier_storage_additions(
    caps: &mut StorageCapacities,
    hunting_reserves: bool,
    foraging_reserves: bool,
) {
    if hunting_reserves {
        caps.food += 50.0;
        caps.hide += 50.0;
        caps.bone += 50.0;
    }
    if foraging_reserves {
        caps.fibre += 100.0;
        caps.herbs += 50.0;
        caps.catnip += 50.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_industrial_storage_additions(
    caps: &mut StorageCapacities,
    cloth: bool,
    leather: bool,
    carpentry: bool,
    blocks: bool,
    metal: bool,
    tools: bool,
    weapons: bool,
    armor: bool,
    refined: bool,
) {
    caps.cloth += if cloth { 50.0 } else { 0.0 };
    caps.leather += if leather { 50.0 } else { 0.0 };
    caps.lumber += if carpentry { 50.0 } else { 0.0 };
    caps.planks += if carpentry { 50.0 } else { 0.0 };
    caps.blocks += if blocks { 50.0 } else { 0.0 };
    caps.metal += if metal { 50.0 } else { 0.0 };
    caps.tools += if tools { 50.0 } else { 0.0 };
    caps.weapons += if weapons { 50.0 } else { 0.0 };
    caps.armor += if armor { 50.0 } else { 0.0 };
    caps.refined += if refined { 50.0 } else { 0.0 };
}

impl StorageCapacities {
    fn min(self, other: Self) -> Self {
        Self {
            food: self.food.min(other.food),
            fish: self.fish.min(other.fish),
            water: self.water.min(other.water),
            herbs: self.herbs.min(other.herbs),
            catnip: self.catnip.min(other.catnip),
            grain: self.grain.min(other.grain),
            flour: self.flour.min(other.flour),
            preserves: self.preserves.min(other.preserves),
            medicine: self.medicine.min(other.medicine),
            brew: self.brew.min(other.brew),
            materials: self.materials.min(other.materials),
            stone: self.stone.min(other.stone),
            refined: self.refined.min(other.refined),
            weapons: self.weapons.min(other.weapons),
            armor: self.armor.min(other.armor),
            planks: self.planks.min(other.planks),
            logs: self.logs.min(other.logs),
            lumber: self.lumber.min(other.lumber),
            blocks: self.blocks.min(other.blocks),
            tools: self.tools.min(other.tools),
            fibre: self.fibre.min(other.fibre),
            hide: self.hide.min(other.hide),
            bone: self.bone.min(other.bone),
            cloth: self.cloth.min(other.cloth),
            leather: self.leather.min(other.leather),
            ore: self.ore.min(other.ore),
            gem: self.gem.min(other.gem),
            clay: self.clay.min(other.clay),
            sand: self.sand.min(other.sand),
            metal: self.metal.min(other.metal),
        }
    }
}

fn physical_storage_capacities(
    stockpiles: &[Stockpile],
    storehouse_caps: StorageCapacities,
) -> StorageCapacities {
    let capacity = |kind| {
        stockpiles
            .iter()
            .filter(|pile| !pile.is_station_local() && pile.accepts.contains(&kind))
            .filter_map(|pile| crate::stockpiles::capacity_for(pile, kind, &storehouse_caps))
            .sum()
    };
    StorageCapacities {
        food: capacity(ResourceKind::Food),
        fish: capacity(ResourceKind::Fish),
        water: capacity(ResourceKind::Water),
        herbs: capacity(ResourceKind::Herbs),
        catnip: capacity(ResourceKind::Catnip),
        grain: capacity(ResourceKind::Grain),
        flour: capacity(ResourceKind::Flour),
        preserves: capacity(ResourceKind::Preserves),
        medicine: capacity(ResourceKind::Medicine),
        brew: capacity(ResourceKind::Brew),
        materials: capacity(ResourceKind::Materials),
        stone: capacity(ResourceKind::Stone),
        refined: capacity(ResourceKind::Refined),
        weapons: capacity(ResourceKind::Weapons),
        armor: capacity(ResourceKind::Armor),
        planks: capacity(ResourceKind::Planks),
        logs: capacity(ResourceKind::Logs),
        lumber: capacity(ResourceKind::Lumber),
        blocks: capacity(ResourceKind::Blocks),
        tools: capacity(ResourceKind::Tools),
        fibre: capacity(ResourceKind::Fibre),
        hide: capacity(ResourceKind::Hide),
        bone: capacity(ResourceKind::Bone),
        cloth: capacity(ResourceKind::Cloth),
        leather: capacity(ResourceKind::Leather),
        ore: capacity(ResourceKind::Ore),
        gem: capacity(ResourceKind::Gem),
        clay: capacity(ResourceKind::Clay),
        sand: capacity(ResourceKind::Sand),
        metal: capacity(ResourceKind::Metal),
    }
}

fn storage_capacities_with_building_mult(
    buildings: &[StorageBuilding],
    storage_mult: f64,
    building_capacity_mult: impl Fn(BuildingType) -> f64,
) -> StorageCapacities {
    let mut caps = BASE_CAPACITY;
    let mult = js_max(0.0, storage_mult);

    for building in buildings {
        if !is_finished(*building) {
            continue;
        }

        let level = level_of(*building);
        let local_mult = js_max(0.0, building_capacity_mult(building.building_type));
        match building.building_type {
            BuildingType::FoodStorage => {
                caps.food += GRANARY_BONUS.food * level * mult * local_mult;
                caps.fish += GRANARY_BONUS.food * level * mult * local_mult;
                caps.herbs += GRANARY_BONUS.herbs * level * mult * local_mult;
                caps.materials += GRANARY_BONUS.materials * level * mult * local_mult;
                caps.refined += GRANARY_BONUS.refined * level * mult * local_mult;
            }
            BuildingType::WaterBowl => {
                caps.water += WATER_BOWL_BONUS * level * mult * local_mult;
            }
            BuildingType::Smithy => {
                caps.weapons += SMITHY_ARMORY_BONUS * level * mult * local_mult;
                caps.armor += SMITHY_ARMORY_BONUS * level * mult * local_mult;
            }
            _ => {}
        }
    }

    caps
}

#[must_use]
pub fn storage_capacities_with_mult(
    buildings: &[StorageBuilding],
    storage_mult: f64,
) -> StorageCapacities {
    storage_capacities(buildings, storage_mult)
}

#[must_use]
pub const fn storehouse_cap(population: u32) -> u32 {
    let cap = population / 6;
    if cap < 1 { 1 } else { cap }
}

#[must_use]
pub fn count_storehouses(buildings: &[StorageBuilding]) -> u32 {
    let count = buildings
        .iter()
        .filter(|building| {
            building.building_type == BuildingType::FoodStorage && is_finished(**building)
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn is_finished(building: StorageBuilding) -> bool {
    building.construction_progress >= 100.0
}

fn level_of(building: StorageBuilding) -> f64 {
    js_max(1.0, building.level.unwrap_or(1.0))
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
    use super::{
        BASE_CAPACITY, GRANARY_BONUS, SMITHY_ARMORY_BONUS, StorageBuilding, StorageCapacities,
        WATER_BOWL_BONUS, authoritative_storage_capacities,
        authoritative_storage_capacities_for_owned, count_storehouses, storage_capacities,
        storage_capacities_default, storage_capacities_with_building_mult,
        storage_capacities_with_mult, storehouse_cap,
    };
    use crate::research_catalog::research_catalog;
    use crate::types::BuildingType;
    use crate::upgrade_tree::resolve_effects;
    use crate::{
        entities::Resources,
        stockpiles::{ResourceKind, SHRINE_STOCKPILE_ID, Stockpile, make_shrine},
        zones::ZoneRect,
    };

    fn building(
        building_type: BuildingType,
        construction_progress: f64,
        level: Option<f64>,
    ) -> StorageBuilding {
        StorageBuilding {
            building_type,
            construction_progress,
            level,
        }
    }

    fn roomy_physical_store() -> Stockpile {
        Stockpile {
            id: "roomy-store".to_owned(),
            rect: ZoneRect {
                x1: 0,
                y1: 0,
                x2: 99,
                y2: 99,
            },
            accepts: ResourceKind::ALL.iter().copied().collect(),
            contents: Resources::default(),
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_caps_bits(actual: StorageCapacities, expected: StorageCapacities, label: &str) {
        assert_f64_bits(actual.food, expected.food, &format!("{label} food"));
        assert_f64_bits(actual.fish, expected.fish, &format!("{label} fish"));
        assert_f64_bits(actual.water, expected.water, &format!("{label} water"));
        assert_f64_bits(actual.herbs, expected.herbs, &format!("{label} herbs"));
        assert_f64_bits(actual.catnip, expected.catnip, &format!("{label} catnip"));
        assert_f64_bits(actual.grain, expected.grain, &format!("{label} grain"));
        assert_f64_bits(actual.flour, expected.flour, &format!("{label} flour"));
        assert_f64_bits(
            actual.materials,
            expected.materials,
            &format!("{label} materials"),
        );
        assert_f64_bits(
            actual.refined,
            expected.refined,
            &format!("{label} refined"),
        );
        assert_f64_bits(
            actual.weapons,
            expected.weapons,
            &format!("{label} weapons"),
        );
        assert_f64_bits(actual.armor, expected.armor, &format!("{label} armor"));
        assert_f64_bits(actual.planks, expected.planks, &format!("{label} planks"));
        assert_f64_bits(actual.logs, expected.logs, &format!("{label} logs"));
        assert_f64_bits(actual.lumber, expected.lumber, &format!("{label} lumber"));
        assert_f64_bits(actual.blocks, expected.blocks, &format!("{label} blocks"));
        assert_f64_bits(actual.tools, expected.tools, &format!("{label} tools"));
        assert_f64_bits(actual.fibre, expected.fibre, &format!("{label} fibre"));
        assert_f64_bits(actual.hide, expected.hide, &format!("{label} hide"));
        assert_f64_bits(actual.bone, expected.bone, &format!("{label} bone"));
        assert_f64_bits(actual.cloth, expected.cloth, &format!("{label} cloth"));
        assert_f64_bits(
            actual.leather,
            expected.leather,
            &format!("{label} leather"),
        );
        assert_f64_bits(actual.ore, expected.ore, &format!("{label} ore"));
        assert_f64_bits(actual.metal, expected.metal, &format!("{label} metal"));
    }

    #[test]
    fn capacity_only_resolver_matches_full_effect_resolution_across_the_catalog() {
        let buildings = [
            building(BuildingType::FoodStorage, 100.0, Some(2.0)),
            building(BuildingType::WaterBowl, 100.0, Some(2.0)),
            building(BuildingType::Smithy, 100.0, Some(2.0)),
        ];
        let physical = [roomy_physical_store()];
        let mut owned = Vec::new();
        for node in research_catalog().nodes() {
            owned.push(node.id.as_str());
            let expected = authoritative_storage_capacities(
                &buildings,
                &physical,
                &resolve_effects(owned.iter().copied()),
            );
            let actual = authoritative_storage_capacities_for_owned(
                &buildings,
                &physical,
                owned.iter().copied(),
            );
            assert_caps_bits(actual, expected, &node.id);
        }
    }

    #[test]
    fn building_capacity_research_scales_only_the_buildings_existing_storage_domains() {
        let buildings = [
            building(BuildingType::FoodStorage, 100.0, Some(1.0)),
            building(BuildingType::WaterBowl, 100.0, Some(1.0)),
            building(BuildingType::Smithy, 100.0, Some(1.0)),
            building(BuildingType::Den, 100.0, Some(1.0)),
        ];
        let physical = [roomy_physical_store()];
        let baseline = authoritative_storage_capacities(
            &buildings,
            &physical,
            &resolve_effects([] as [&str; 0]),
        );

        let food = authoritative_storage_capacities(
            &buildings,
            &physical,
            &resolve_effects(["food_storage_stores"]),
        );
        assert_eq!(food.food, baseline.food + GRANARY_BONUS.food * 0.2);
        assert_eq!(food.fish, baseline.fish + GRANARY_BONUS.food * 0.2);
        assert_eq!(food.herbs, baseline.herbs + GRANARY_BONUS.herbs * 0.2);
        assert_eq!(
            food.materials,
            baseline.materials + GRANARY_BONUS.materials * 0.2
        );
        assert_eq!(food.refined, baseline.refined + GRANARY_BONUS.refined * 0.2);
        assert_eq!(food.water, baseline.water);
        assert_eq!(food.weapons, baseline.weapons);
        assert_eq!(food.armor, baseline.armor);

        let water = authoritative_storage_capacities(
            &buildings,
            &physical,
            &resolve_effects(["water_bowl_stores"]),
        );
        assert_eq!(water.water, baseline.water + WATER_BOWL_BONUS * 0.2);
        assert_eq!(water.food, baseline.food);
        assert_eq!(water.weapons, baseline.weapons);

        let smithy = authoritative_storage_capacities(
            &buildings,
            &physical,
            &resolve_effects(["smithy_stores"]),
        );
        assert_eq!(smithy.weapons, baseline.weapons + SMITHY_ARMORY_BONUS * 0.2);
        assert_eq!(smithy.armor, baseline.armor + SMITHY_ARMORY_BONUS * 0.2);
        assert_eq!(smithy.food, baseline.food);
        assert_eq!(smithy.water, baseline.water);

        let general = make_shrine(ZoneRect {
            x1: 1,
            y1: 1,
            x2: 1,
            y2: 1,
        });
        let physical_food = authoritative_storage_capacities(
            &buildings,
            std::slice::from_ref(&general),
            &resolve_effects(["food_storage_stores"]),
        );
        assert_eq!(
            crate::stockpiles::capacity_for(&general, ResourceKind::Food, &physical_food),
            Some(physical_food.food),
            "the targeted study expands real founding-store headroom"
        );
    }

    #[test]
    fn hunting_and_foraging_reserves_expand_only_their_physical_stores() {
        let physical = [roomy_physical_store()];
        let baseline =
            authoritative_storage_capacities(&[], &physical, &resolve_effects([] as [&str; 0]));
        let hunting = authoritative_storage_capacities(
            &[],
            &physical,
            &resolve_effects(["hunting_reserves"]),
        );
        assert_eq!(hunting.food, baseline.food + 50.0);
        assert_eq!(hunting.hide, baseline.hide + 50.0);
        assert_eq!(hunting.bone, baseline.bone + 50.0);
        assert_eq!(hunting.fibre, baseline.fibre);

        let foraging = authoritative_storage_capacities(
            &[],
            &physical,
            &resolve_effects(["foraging_reserves"]),
        );
        assert_eq!(foraging.fibre, baseline.fibre + 100.0);
        assert_eq!(foraging.herbs, baseline.herbs + 50.0);
        assert_eq!(foraging.catnip, baseline.catnip + 50.0);
        assert_eq!(foraging.hide, baseline.hide);
    }

    #[test]
    fn non_storehouse_capacity_research_is_isolated_and_deterministic() {
        let buildings = [
            building(BuildingType::FoodStorage, 100.0, Some(2.0)),
            building(BuildingType::WaterBowl, 100.0, Some(1.0)),
            building(BuildingType::Smithy, 100.0, Some(1.0)),
            building(BuildingType::Den, 100.0, Some(1.0)),
            building(BuildingType::Workshop, 100.0, Some(1.0)),
        ];
        let physical = [roomy_physical_store()];
        let baseline = authoritative_storage_capacities(
            &buildings,
            &physical,
            &resolve_effects([] as [&str; 0]),
        );
        let station_only = resolve_effects(["den_stores", "workshop_stores"]);
        let left = authoritative_storage_capacities(&buildings, &physical, &station_only);
        let right = authoritative_storage_capacities(&buildings, &physical, &station_only);

        assert_caps_bits(left, baseline, "station target isolation");
        assert_caps_bits(right, left, "deterministic twin");
    }

    #[test]
    fn legacy_shrine_and_missing_storehouse_preserve_capacity_until_reconciliation() {
        let buildings = [building(BuildingType::FoodStorage, 100.0, Some(1.0))];
        let effects = resolve_effects(["food_storage_stores"]);
        let expected = storage_capacities_with_building_mult(
            &buildings,
            effects.storage_per_level_mult,
            |building_type| effects.building(building_type.as_str()).capacity_mult,
        )
        .scaled(effects.storage_capacity_mult);

        assert_caps_bits(
            authoritative_storage_capacities(&buildings, &[], &effects),
            expected,
            "missing storehouse default",
        );

        let mut legacy_shrine = make_shrine(ZoneRect {
            x1: 2,
            y1: 2,
            x2: 2,
            y2: 2,
        });
        legacy_shrine.id = SHRINE_STOCKPILE_ID.to_owned();
        assert_caps_bits(
            authoritative_storage_capacities(&buildings, &[legacy_shrine], &effects),
            expected,
            "legacy shrine migration default",
        );

        assert_caps_bits(
            authoritative_storage_capacities(&buildings, &[roomy_physical_store()], &effects),
            expected,
            "designated-only migration default",
        );
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_caps_bits(
            BASE_CAPACITY,
            StorageCapacities {
                food: 200.0,
                fish: 200.0,
                water: 200.0,
                herbs: 100.0,
                catnip: 100.0,
                grain: 100.0,
                flour: 100.0,
                preserves: 100.0,
                medicine: 100.0,
                brew: 100.0,
                materials: 100.0,
                stone: 100.0,
                refined: 100.0,
                weapons: 50.0,
                armor: 50.0,
                planks: 100.0,
                logs: 100.0,
                lumber: 100.0,
                blocks: 100.0,
                tools: 100.0,
                fibre: 100.0,
                hide: 100.0,
                bone: 100.0,
                cloth: 100.0,
                leather: 100.0,
                ore: 100.0,
                gem: 100.0,
                clay: 100.0,
                sand: 100.0,
                metal: 100.0,
            },
            "base capacity",
        );
        assert_f64_bits(GRANARY_BONUS.food, 400.0, "granary food");
        assert_f64_bits(GRANARY_BONUS.herbs, 100.0, "granary herbs");
        assert_f64_bits(GRANARY_BONUS.materials, 100.0, "granary materials");
        assert_f64_bits(GRANARY_BONUS.refined, 50.0, "granary refined");
        assert_f64_bits(WATER_BOWL_BONUS, 200.0, "water bowl");
        assert_f64_bits(SMITHY_ARMORY_BONUS, 50.0, "smithy armory");
    }

    #[test]
    fn new_chain_capacities_default_when_deserializing_legacy_payloads() {
        let legacy = serde_json::json!({
            "food": 200.0,
            "water": 200.0,
            "herbs": 100.0,
            "materials": 100.0,
            "refined": 100.0,
            "weapons": 50.0,
            "armor": 50.0,
            "planks": 100.0,
            "blocks": 100.0,
            "tools": 100.0,
            "fibre": 100.0,
            "hide": 100.0,
            "cloth": 100.0,
            "leather": 100.0
        });
        let decoded: StorageCapacities = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded.catnip, 0.0);
        assert_eq!(decoded.grain, 0.0);
        assert_eq!(decoded.flour, 0.0);
        assert_eq!(decoded.logs, 0.0);
        assert_eq!(decoded.lumber, 0.0);
        assert_eq!(decoded.bone, 0.0);
        assert_eq!(decoded.ore, 0.0);
        assert_eq!(decoded.metal, 0.0);
    }

    #[test]
    fn storage_capacities_match_hand_derived_vectors() {
        assert_caps_bits(
            storage_capacities_default(&[]),
            BASE_CAPACITY,
            "empty settlement",
        );

        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 100.0, Some(1.0)),
                building(BuildingType::FoodStorage, 100.0, Some(2.0)),
                building(BuildingType::WaterBowl, 100.0, Some(1.0)),
                building(BuildingType::Smithy, 100.0, Some(3.0)),
            ]),
            StorageCapacities {
                food: 1_400.0,
                fish: 1_400.0,
                water: 400.0,
                herbs: 400.0,
                catnip: 100.0,
                grain: 100.0,
                flour: 100.0,
                preserves: 100.0,
                medicine: 100.0,
                brew: 100.0,
                materials: 400.0,
                stone: 100.0,
                refined: 250.0,
                weapons: 200.0,
                armor: 200.0,
                planks: 100.0,
                logs: 100.0,
                lumber: 100.0,
                blocks: 100.0,
                tools: 100.0,
                fibre: 100.0,
                hide: 100.0,
                bone: 100.0,
                cloth: 100.0,
                leather: 100.0,
                ore: 100.0,
                gem: 100.0,
                clay: 100.0,
                sand: 100.0,
                metal: 100.0,
            },
            "mixed finished buildings",
        );

        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 40.0, Some(9.0)),
                building(BuildingType::WaterBowl, 99.99, Some(9.0)),
                building(BuildingType::Smithy, f64::NAN, Some(9.0)),
            ]),
            BASE_CAPACITY,
            "unfinished buildings",
        );
    }

    #[test]
    fn storage_multiplier_scales_only_building_bonus() {
        assert_caps_bits(
            storage_capacities(
                &[
                    building(BuildingType::FoodStorage, 100.0, None),
                    building(BuildingType::WaterBowl, 100.0, Some(2.0)),
                    building(BuildingType::Smithy, 100.0, Some(2.0)),
                ],
                1.25,
            ),
            StorageCapacities {
                food: 700.0,
                fish: 700.0,
                water: 700.0,
                herbs: 225.0,
                catnip: 100.0,
                grain: 100.0,
                flour: 100.0,
                preserves: 100.0,
                medicine: 100.0,
                brew: 100.0,
                materials: 225.0,
                stone: 100.0,
                refined: 162.5,
                weapons: 175.0,
                armor: 175.0,
                planks: 100.0,
                logs: 100.0,
                lumber: 100.0,
                blocks: 100.0,
                tools: 100.0,
                fibre: 100.0,
                hide: 100.0,
                bone: 100.0,
                cloth: 100.0,
                leather: 100.0,
                ore: 100.0,
                gem: 100.0,
                clay: 100.0,
                sand: 100.0,
                metal: 100.0,
            },
            "scaled building bonuses",
        );

        assert_caps_bits(
            storage_capacities_with_mult(
                &[building(BuildingType::FoodStorage, 100.0, Some(1.0))],
                -2.0,
            ),
            BASE_CAPACITY,
            "negative multiplier clamps to zero",
        );
    }

    #[test]
    fn level_defaults_and_clamps_like_typescript() {
        assert_caps_bits(
            storage_capacities_default(&[
                building(BuildingType::FoodStorage, 100.0, None),
                building(BuildingType::WaterBowl, 100.0, Some(0.0)),
                building(BuildingType::Smithy, 100.0, Some(-4.0)),
            ]),
            StorageCapacities {
                food: 600.0,
                fish: 600.0,
                water: 400.0,
                herbs: 200.0,
                catnip: 100.0,
                grain: 100.0,
                flour: 100.0,
                preserves: 100.0,
                medicine: 100.0,
                brew: 100.0,
                materials: 200.0,
                stone: 100.0,
                refined: 150.0,
                weapons: 100.0,
                armor: 100.0,
                planks: 100.0,
                logs: 100.0,
                lumber: 100.0,
                blocks: 100.0,
                tools: 100.0,
                fibre: 100.0,
                hide: 100.0,
                bone: 100.0,
                cloth: 100.0,
                leather: 100.0,
                ore: 100.0,
                gem: 100.0,
                clay: 100.0,
                sand: 100.0,
                metal: 100.0,
            },
            "default and minimum level",
        );
    }

    #[test]
    fn storehouse_cap_matches_population_floor() {
        assert_eq!(storehouse_cap(0), 1);
        assert_eq!(storehouse_cap(5), 1);
        assert_eq!(storehouse_cap(6), 1);
        assert_eq!(storehouse_cap(20), 3);
        assert_eq!(storehouse_cap(60), 10);
    }

    #[test]
    fn count_storehouses_counts_only_finished_granaries() {
        assert_eq!(
            count_storehouses(&[
                building(BuildingType::FoodStorage, 100.0, Some(1.0)),
                building(BuildingType::FoodStorage, 60.0, Some(1.0)),
                building(BuildingType::WaterBowl, 100.0, Some(1.0)),
                building(BuildingType::Den, 100.0, Some(1.0)),
            ]),
            1
        );
    }

    #[test]
    fn nan_inputs_follow_typescript_math_max_and_comparisons() {
        let caps = storage_capacities_with_mult(
            &[building(BuildingType::FoodStorage, 100.0, Some(1.0))],
            f64::NAN,
        );
        assert!(caps.food.is_nan());
        assert_f64_bits(caps.water, BASE_CAPACITY.water, "water unchanged");
        assert!(caps.herbs.is_nan());
        assert!(caps.materials.is_nan());
        assert!(caps.refined.is_nan());

        let caps =
            storage_capacities_default(&[building(BuildingType::WaterBowl, 100.0, Some(f64::NAN))]);
        assert!(caps.water.is_nan());
        assert_f64_bits(caps.food, BASE_CAPACITY.food, "food unchanged");
    }

    #[test]
    fn food_plant_storage_studies_expand_only_their_real_finite_goods() {
        let caps = authoritative_storage_capacities_for_owned(
            &[],
            &[],
            [
                "grain_milling_preservation",
                "grain_milling_reserves",
                "baking_reserves",
                "herbalism_preservation",
                "food_preservation_preservation",
                "brewing_preservation",
            ],
        );
        assert_eq!(caps.grain, BASE_CAPACITY.grain + 100.0);
        assert_eq!(caps.flour, BASE_CAPACITY.flour + 50.0);
        assert_eq!(caps.food, BASE_CAPACITY.food + 100.0);
        assert_eq!(caps.preserves, BASE_CAPACITY.preserves + 50.0);
        assert_eq!(caps.medicine, BASE_CAPACITY.medicine + 50.0);
        assert_eq!(caps.brew, BASE_CAPACITY.brew + 50.0);
        assert_eq!(caps.water, BASE_CAPACITY.water);
    }

    #[test]
    fn industrial_preservation_studies_expand_only_their_real_finite_goods() {
        let caps = authoritative_storage_capacities_for_owned(
            &[],
            &[],
            [
                "textile_work_preservation",
                "leatherworking_preservation",
                "carpentry_preservation",
                "stonecraft_preservation",
                "metallurgy_preservation",
                "toolmaking_preservation",
                "weaponcraft_preservation",
                "armorcraft_preservation",
                "trade_goods_preservation",
            ],
        );
        assert_eq!(caps.cloth, BASE_CAPACITY.cloth + 50.0);
        assert_eq!(caps.leather, BASE_CAPACITY.leather + 50.0);
        assert_eq!(caps.lumber, BASE_CAPACITY.lumber + 50.0);
        assert_eq!(caps.planks, BASE_CAPACITY.planks + 50.0);
        assert_eq!(caps.blocks, BASE_CAPACITY.blocks + 50.0);
        assert_eq!(caps.metal, BASE_CAPACITY.metal + 50.0);
        assert_eq!(caps.tools, BASE_CAPACITY.tools + 50.0);
        assert_eq!(caps.weapons, BASE_CAPACITY.weapons + 50.0);
        assert_eq!(caps.armor, BASE_CAPACITY.armor + 50.0);
        assert_eq!(caps.refined, BASE_CAPACITY.refined + 50.0);
        assert_eq!(caps.food, BASE_CAPACITY.food);
        assert_eq!(caps.water, BASE_CAPACITY.water);
    }
}
