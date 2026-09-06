//! Warrior combat and raid muster rules ported from `lib/game/warriors.ts`.

use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Serialize};

use crate::{
    life_sim::trade_level,
    types::{CatSpecialization, LifeStage},
};

pub type WarriorSpecialization = Option<CatSpecialization>;

/// Attack a single weapon adds when equipped.
pub const WEAPON_ATTACK_BONUS: f64 = 25.0;
/// Defense a single piece of armor adds when equipped.
pub const ARMOR_DEFENSE_BONUS: f64 = 25.0;
/// Combat effectiveness of a trained warrior.
pub const WARRIOR_COMBAT_FACTOR: f64 = 1.0;
/// Combat effectiveness of a hunter pressed into the fight.
pub const HUNTER_COMBAT_FACTOR: f64 = 0.45;
/// Combat effectiveness of ordinary militia.
pub const MILITIA_COMBAT_FACTOR: f64 = 0.28;
/// Extra combat power per warrior trade level.
pub const WARRIOR_XP_POWER_PER_LEVEL: f64 = 0.1;
/// Warrior-trade XP a cat earns for surviving a defended raid.
pub const WARRIOR_XP_PER_RAID: f64 = 4.0;
/// Warrior-trade XP track key on `RoleXp`.
pub const WARRIOR_XP_KEY: &str = "warrior";

/// Upgrade-tree combat modifiers applied during combat power scoring.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatModifiers {
    /// Upgrade-tree combatPowerMult (weaponsmithing).
    pub combat_power_mult: f64,
    /// Upgrade-tree defenseMult (armorsmithing).
    pub defense_mult: f64,
}

impl Default for CombatModifiers {
    fn default() -> Self {
        NEUTRAL_MODS
    }
}

pub const NEUTRAL_MODS: CombatModifiers = CombatModifiers {
    combat_power_mult: 1.0,
    defense_mult: 1.0,
};

/// Inputs for scoring one cat's combat power.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatCombatPowerInput {
    pub attack: f64,
    pub defense: f64,
    pub specialization: WarriorSpecialization,
    #[serde(default)]
    pub warrior_xp: f64,
    #[serde(default)]
    pub weapon: bool,
    #[serde(default)]
    pub armor: bool,
    #[serde(default)]
    pub mods: CombatModifiers,
    #[serde(default = "adult_stage_factor")]
    pub stage_factor: f64,
}

impl CatCombatPowerInput {
    #[must_use]
    pub fn new(attack: f64, defense: f64, specialization: WarriorSpecialization) -> Self {
        Self {
            attack,
            defense,
            specialization,
            warrior_xp: 0.0,
            weapon: false,
            armor: false,
            mods: NEUTRAL_MODS,
            stage_factor: 1.0,
        }
    }
}

/// Alias matching the TS object accepted by `catCombatPower`.
pub type CombatCat = CatCombatPowerInput;

/// Cat shape accepted by [`muster_defense`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusterCombatant {
    pub id: String,
    pub attack: f64,
    pub defense: f64,
    pub specialization: WarriorSpecialization,
    #[serde(default)]
    pub warrior_xp: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_stage: Option<LifeStage>,
}

impl MusterCombatant {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        attack: f64,
        defense: f64,
        specialization: WarriorSpecialization,
    ) -> Self {
        Self {
            id: id.into(),
            attack,
            defense,
            specialization,
            warrior_xp: 0.0,
            life_stage: None,
        }
    }
}

/// Gear available to draw from the stockpile at raid time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DefenseStock {
    pub weapons: f64,
    pub armor: f64,
}

/// Per-cat power and gear assignment in muster order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusteredCat {
    pub id: String,
    pub power: f64,
    pub weapon: bool,
    pub armor: bool,
}

/// Total defenders and consumable gear drawn for one raid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenseMuster {
    pub total_power: f64,
    pub weapons_used: u32,
    pub armor_used: u32,
    pub per_cat: Vec<MusteredCat>,
    pub combatants: usize,
}

#[must_use]
pub fn combat_role_factor(spec: WarriorSpecialization) -> f64 {
    match spec {
        Some(CatSpecialization::Warrior) => WARRIOR_COMBAT_FACTOR,
        Some(CatSpecialization::Hunter) => HUNTER_COMBAT_FACTOR,
        Some(CatSpecialization::Architect | CatSpecialization::Ritualist) | None => {
            MILITIA_COMBAT_FACTOR
        }
    }
}

#[must_use]
pub fn combat_stage_factor(stage: LifeStage) -> f64 {
    match stage {
        LifeStage::Kitten => 0.0,
        LifeStage::Young => 0.85,
        LifeStage::Adult => 1.0,
        LifeStage::Elder => 0.6,
    }
}

#[must_use]
pub fn can_fight(spec: WarriorSpecialization) -> bool {
    combat_role_factor(spec) > 0.0
}

#[must_use]
pub fn cat_combat_power(cat: CatCombatPowerInput) -> f64 {
    let role = combat_role_factor(cat.specialization);
    let stage_factor = js_max(0.0, cat.stage_factor);
    if role <= 0.0 || stage_factor <= 0.0 {
        return 0.0;
    }

    let atk = (cat.attack + if cat.weapon { WEAPON_ATTACK_BONUS } else { 0.0 })
        * js_max(0.0, cat.mods.combat_power_mult);
    let def = (cat.defense + if cat.armor { ARMOR_DEFENSE_BONUS } else { 0.0 })
        * js_max(0.0, cat.mods.defense_mult);
    let xp_bonus = 1.0 + trade_level(cat.warrior_xp) * WARRIOR_XP_POWER_PER_LEVEL;

    (atk + def) * role * xp_bonus * stage_factor
}

#[must_use]
pub fn muster_defense(
    combatants: &[MusterCombatant],
    stock: DefenseStock,
    mods: CombatModifiers,
) -> DefenseMuster {
    let mut order: Vec<&MusterCombatant> = combatants
        .iter()
        .filter(|cat| can_fight(cat.specialization))
        .collect();
    order.sort_by(|a, b| compare_muster_order(a, b));

    let mut weapons = js_max(0.0, stock.weapons.floor());
    let mut armor = js_max(0.0, stock.armor.floor());
    let mut weapons_used = 0;
    let mut armor_used = 0;
    let mut total_power = 0.0;
    let mut per_cat = Vec::with_capacity(order.len());

    for cat in &order {
        let has_weapon = weapons > 0.0;
        let has_armor = armor > 0.0;

        if has_weapon {
            weapons -= 1.0;
            weapons_used += 1;
        }
        if has_armor {
            armor -= 1.0;
            armor_used += 1;
        }

        let power = cat_combat_power(CatCombatPowerInput {
            attack: cat.attack,
            defense: cat.defense,
            specialization: cat.specialization,
            warrior_xp: cat.warrior_xp,
            weapon: has_weapon,
            armor: has_armor,
            mods,
            stage_factor: cat.life_stage.map_or(1.0, combat_stage_factor),
        });
        total_power += power;
        per_cat.push(MusteredCat {
            id: cat.id.clone(),
            power,
            weapon: has_weapon,
            armor: has_armor,
        });
    }

    DefenseMuster {
        total_power,
        weapons_used,
        armor_used,
        per_cat,
        combatants: order.len(),
    }
}

/// Muster using the exact gear already equipped by each defender. This keeps
/// stable item identities attached to their owner instead of treating equipment
/// as a fungible armory counter at combat time.
#[must_use]
pub fn muster_defense_with_loadout(
    combatants: &[MusterCombatant],
    loadout: &BTreeMap<String, (bool, bool)>,
    mods: CombatModifiers,
) -> DefenseMuster {
    let mut order: Vec<&MusterCombatant> = combatants
        .iter()
        .filter(|cat| can_fight(cat.specialization))
        .collect();
    order.sort_by(|a, b| compare_muster_order(a, b));

    let mut weapons_used = 0;
    let mut armor_used = 0;
    let mut total_power = 0.0;
    let mut per_cat = Vec::with_capacity(order.len());
    for cat in order {
        let (weapon, armor) = loadout.get(&cat.id).copied().unwrap_or_default();
        weapons_used += u32::from(weapon);
        armor_used += u32::from(armor);
        let power = cat_combat_power(CatCombatPowerInput {
            attack: cat.attack,
            defense: cat.defense,
            specialization: cat.specialization,
            warrior_xp: cat.warrior_xp,
            weapon,
            armor,
            mods,
            stage_factor: cat.life_stage.map_or(1.0, combat_stage_factor),
        });
        total_power += power;
        per_cat.push(MusteredCat {
            id: cat.id.clone(),
            power,
            weapon,
            armor,
        });
    }
    DefenseMuster {
        total_power,
        weapons_used,
        armor_used,
        combatants: per_cat.len(),
        per_cat,
    }
}

#[must_use]
pub fn muster_defense_neutral(
    combatants: &[MusterCombatant],
    stock: DefenseStock,
) -> DefenseMuster {
    muster_defense(combatants, stock, NEUTRAL_MODS)
}

fn compare_muster_order(a: &MusterCombatant, b: &MusterCombatant) -> Ordering {
    let role_diff = combat_role_factor(b.specialization) - combat_role_factor(a.specialization);
    if role_diff < 0.0 {
        return Ordering::Less;
    }
    if role_diff > 0.0 {
        return Ordering::Greater;
    }

    let stat_diff = (b.attack + b.defense) - (a.attack + a.defense);
    if stat_diff < 0.0 {
        Ordering::Less
    } else if stat_diff > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
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

const fn adult_stage_factor() -> f64 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn assert_float_close(actual: f64, expected: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= 1e-9,
            "actual {actual} did not match expected {expected}; diff {diff}"
        );
    }

    fn warrior(id: &str, attack: f64, defense: f64) -> MusterCombatant {
        MusterCombatant::new(id, attack, defense, Some(CatSpecialization::Warrior))
    }

    #[test]
    fn combat_role_factor_and_can_fight_match_warriors_ts() {
        assert_float_eq(
            combat_role_factor(Some(CatSpecialization::Warrior)),
            WARRIOR_COMBAT_FACTOR,
        );
        assert_float_eq(
            combat_role_factor(Some(CatSpecialization::Hunter)),
            HUNTER_COMBAT_FACTOR,
        );
        assert_float_eq(
            combat_role_factor(Some(CatSpecialization::Architect)),
            MILITIA_COMBAT_FACTOR,
        );
        assert_float_eq(
            combat_role_factor(Some(CatSpecialization::Ritualist)),
            MILITIA_COMBAT_FACTOR,
        );
        assert_float_eq(combat_role_factor(None), MILITIA_COMBAT_FACTOR);
        assert!(can_fight(Some(CatSpecialization::Warrior)));
        assert!(can_fight(Some(CatSpecialization::Hunter)));
        assert!(can_fight(None));
    }

    #[test]
    fn combat_stage_factor_matches_warriors_ts() {
        assert_float_eq(combat_stage_factor(LifeStage::Kitten), 0.0);
        assert_float_eq(combat_stage_factor(LifeStage::Young), 0.85);
        assert_float_eq(combat_stage_factor(LifeStage::Adult), 1.0);
        assert_float_eq(combat_stage_factor(LifeStage::Elder), 0.6);
    }

    #[test]
    fn cat_combat_power_matches_hand_derived_vectors() {
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput::new(
                50.0,
                50.0,
                Some(CatSpecialization::Architect),
            )),
            28.000000000000004,
        );
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput::new(
                50.0,
                50.0,
                Some(CatSpecialization::Warrior),
            )),
            100.0,
        );
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput {
                stage_factor: 0.0,
                ..CatCombatPowerInput::new(99.0, 99.0, Some(CatSpecialization::Warrior))
            }),
            0.0,
        );
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput {
                weapon: true,
                armor: true,
                ..CatCombatPowerInput::new(40.0, 40.0, Some(CatSpecialization::Warrior))
            }),
            130.0,
        );
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput {
                warrior_xp: 100.0,
                ..CatCombatPowerInput::new(40.0, 40.0, Some(CatSpecialization::Warrior))
            }),
            160.0,
        );
        assert_float_eq(
            cat_combat_power(CatCombatPowerInput {
                mods: CombatModifiers {
                    combat_power_mult: 1.25,
                    defense_mult: 1.25,
                },
                ..CatCombatPowerInput::new(40.0, 40.0, Some(CatSpecialization::Warrior))
            }),
            100.0,
        );
    }

    #[test]
    fn muster_defense_sorts_by_role_then_base_stats() {
        let muster = muster_defense(
            &[
                warrior("warr", 50.0, 50.0),
                MusterCombatant::new("arch", 99.0, 99.0, Some(CatSpecialization::Architect)),
                MusterCombatant::new("hunter", 80.0, 80.0, Some(CatSpecialization::Hunter)),
            ],
            DefenseStock {
                weapons: 0.0,
                armor: 0.0,
            },
            NEUTRAL_MODS,
        );

        assert_eq!(muster.combatants, 3);
        assert_eq!(muster.per_cat[0].id, "warr");
        assert_eq!(muster.per_cat[1].id, "hunter");
        assert_eq!(muster.per_cat[2].id, "arch");
        assert_float_close(muster.total_power, 227.44);
    }

    #[test]
    fn muster_defense_arms_strongest_warriors_first_with_scarce_gear() {
        let muster = muster_defense(
            &[warrior("weak", 10.0, 10.0), warrior("strong", 80.0, 80.0)],
            DefenseStock {
                weapons: 1.0,
                armor: 1.0,
            },
            NEUTRAL_MODS,
        );

        assert_eq!(muster.weapons_used, 1);
        assert_eq!(muster.armor_used, 1);
        assert_eq!(muster.per_cat[0].id, "strong");
        assert!(muster.per_cat[0].weapon);
        assert!(muster.per_cat[0].armor);
        assert!(!muster.per_cat[1].weapon);
        assert!(!muster.per_cat[1].armor);
        assert_float_eq(muster.total_power, 230.0);
    }

    #[test]
    fn muster_defense_prioritizes_warriors_over_hunters_for_gear() {
        let muster = muster_defense(
            &[
                MusterCombatant::new("h", 90.0, 90.0, Some(CatSpecialization::Hunter)),
                warrior("warr", 40.0, 40.0),
            ],
            DefenseStock {
                weapons: 1.0,
                armor: 1.0,
            },
            NEUTRAL_MODS,
        );

        assert_eq!(muster.per_cat[0].id, "warr");
        assert!(muster.per_cat[0].weapon);
        assert_float_eq(muster.total_power, 211.0);
    }

    #[test]
    fn muster_defense_consumes_no_more_gear_than_combatants_can_hold() {
        let muster = muster_defense(
            &[warrior("a", 50.0, 50.0)],
            DefenseStock {
                weapons: 5.0,
                armor: 5.0,
            },
            NEUTRAL_MODS,
        );

        assert_eq!(muster.weapons_used, 1);
        assert_eq!(muster.armor_used, 1);
        assert_float_eq(muster.total_power, 150.0);
    }

    #[test]
    fn muster_defense_scales_mustered_power_by_life_stage() {
        let adult = muster_defense(
            &[MusterCombatant {
                life_stage: Some(LifeStage::Adult),
                ..warrior("a", 50.0, 50.0)
            }],
            DefenseStock {
                weapons: 0.0,
                armor: 0.0,
            },
            NEUTRAL_MODS,
        );
        let elder = muster_defense(
            &[MusterCombatant {
                life_stage: Some(LifeStage::Elder),
                ..warrior("a", 50.0, 50.0)
            }],
            DefenseStock {
                weapons: 0.0,
                armor: 0.0,
            },
            NEUTRAL_MODS,
        );
        let kitten = muster_defense(
            &[MusterCombatant {
                life_stage: Some(LifeStage::Kitten),
                ..warrior("a", 50.0, 50.0)
            }],
            DefenseStock {
                weapons: 0.0,
                armor: 0.0,
            },
            NEUTRAL_MODS,
        );

        assert_float_eq(adult.total_power, 100.0);
        assert_float_eq(elder.total_power, 60.0);
        assert_float_eq(kitten.total_power, 0.0);
        assert_eq!(kitten.combatants, 1);
    }

    #[test]
    fn muster_defense_floors_stock_and_treats_bad_stock_as_empty() {
        let fractional = muster_defense(
            &[warrior("a", 50.0, 50.0), warrior("b", 30.0, 30.0)],
            DefenseStock {
                weapons: 1.9,
                armor: 1.1,
            },
            NEUTRAL_MODS,
        );
        assert_eq!(fractional.weapons_used, 1);
        assert_eq!(fractional.armor_used, 1);
        assert!(fractional.per_cat[0].weapon);
        assert!(!fractional.per_cat[1].weapon);

        let bad = muster_defense(
            &[warrior("a", 50.0, 50.0)],
            DefenseStock {
                weapons: f64::NAN,
                armor: -1.0,
            },
            NEUTRAL_MODS,
        );
        assert_eq!(bad.weapons_used, 0);
        assert_eq!(bad.armor_used, 0);
        assert!(!bad.per_cat[0].weapon);
        assert!(!bad.per_cat[0].armor);
    }

    #[test]
    fn exact_loadout_never_reassigns_another_cats_gear() {
        let combatants = [warrior("a", 50.0, 50.0), warrior("b", 30.0, 30.0)];
        let loadout = BTreeMap::from([
            ("a".to_owned(), (false, true)),
            ("b".to_owned(), (true, false)),
            ("noncombatant".to_owned(), (true, true)),
        ]);
        let muster = muster_defense_with_loadout(&combatants, &loadout, NEUTRAL_MODS);
        let a = muster.per_cat.iter().find(|cat| cat.id == "a").unwrap();
        let b = muster.per_cat.iter().find(|cat| cat.id == "b").unwrap();
        assert_eq!((a.weapon, a.armor), (false, true));
        assert_eq!((b.weapon, b.armor), (true, false));
        assert_eq!(muster.weapons_used, 1);
        assert_eq!(muster.armor_used, 1);
    }
}
