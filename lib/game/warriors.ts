/**
 * Warriors: training rules, combat power, and equipment muster (pure) —
 * Roadmap 4, Military.
 *
 * A cat trains into the `warrior` specialization at the barracks. Warriors
 * fight well; hunters fight passably; every other able-bodied adult or young
 * cat still turns out as militia and swings a paw when raiders reach the
 * village. Only kittens (and, via the caller, the dead) sit a raid out. Combat
 * power blends a cat's raw stats, its role, its life stage (elders fade,
 * kittens can't fight), its warrior trade level, the gear it draws from the
 * colony stockpile, and the upgrade-tree combat modifiers (weaponsmithing /
 * armorsmithing).
 *
 * Equipment is drawn from the stockpile at RAID TIME, not at training time:
 * {@link musterDefense} allocates the colony's weapons/armor across the
 * mustered combatants (warriors first), and the caller consumes exactly the
 * `weaponsUsed` / `armorUsed` it reports. Gear is a consumable — a raid burns
 * through what it musters, which is what keeps the smithy busy.
 */

import type { LifeStage } from "@/types/game";

import { tradeLevel } from "./lifeSim";

export type WarriorSpecialization =
	| "hunter"
	| "architect"
	| "ritualist"
	| "warrior"
	| null;

/** Attack a single weapon adds when equipped. */
export const WEAPON_ATTACK_BONUS = 25;
/** Defense a single piece of armor adds when equipped. */
export const ARMOR_DEFENSE_BONUS = 25;
/** Combat effectiveness of a trained warrior (full strength). */
export const WARRIOR_COMBAT_FACTOR = 1;
/** Combat effectiveness of a hunter pressed into the fight. */
export const HUNTER_COMBAT_FACTOR = 0.45;
/**
 * Combat effectiveness of an ordinary villager turning out as militia — a
 * builder, ritualist or untrained cat grabbing whatever's to hand. Weak per
 * head, but 20 of them are still a wall a first warband breaks against.
 */
export const MILITIA_COMBAT_FACTOR = 0.28;
/** Extra combat power per warrior trade level (diminishing via tradeLevel). */
export const WARRIOR_XP_POWER_PER_LEVEL = 0.1;
/** Warrior-trade XP a cat earns for surviving a defended raid. */
export const WARRIOR_XP_PER_RAID = 4;

/**
 * How much a role contributes to the fight. Warriors fight at full strength,
 * hunters passably, and everyone else forms the militia. Never 0 — every
 * specialization can defend its home; the life-stage gate (not this) is what
 * keeps kittens out.
 */
export function combatRoleFactor(spec: WarriorSpecialization): number {
	if (spec === "warrior") {
		return WARRIOR_COMBAT_FACTOR;
	}
	if (spec === "hunter") {
		return HUNTER_COMBAT_FACTOR;
	}
	return MILITIA_COMBAT_FACTOR;
}

/**
 * How much a cat's life stage scales its combat power. Kittens can't fight at
 * all, young cats aren't fully grown, adults are the backbone, and elders have
 * lost a step. Mirrors {@link stageWorkEffectiveness} but tuned for the fight.
 */
export function combatStageFactor(stage: LifeStage): number {
	switch (stage) {
		case "kitten":
			return 0;
		case "young":
			return 0.85;
		case "adult":
			return 1;
		case "elder":
			return 0.6;
		default:
			return 1;
	}
}

/**
 * True when a cat's specialization lets it join the muster at all. Every role
 * can (the militia includes untrained cats); the caller still gates on life
 * stage so kittens and the dead never turn out.
 */
export function canFight(spec: WarriorSpecialization): boolean {
	return combatRoleFactor(spec) > 0;
}

export interface CombatModifiers {
	/** Upgrade-tree combatPowerMult (weaponsmithing). Default 1. */
	combatPowerMult: number;
	/** Upgrade-tree defenseMult (armorsmithing). Default 1. */
	defenseMult: number;
}

const NEUTRAL_MODS: CombatModifiers = { combatPowerMult: 1, defenseMult: 1 };

/**
 * Combat power of one cat: (attack + weapon) scaled by combatPowerMult plus
 * (defense + armor) scaled by defenseMult, weighted by its role factor, scaled
 * by its life stage (`stageFactor`, default 1 for an adult), and lifted by its
 * warrior trade level. A `stageFactor` of 0 (a kitten) scores 0.
 */
export function catCombatPower(cat: {
	attack: number;
	defense: number;
	specialization: WarriorSpecialization;
	warriorXp?: number;
	weapon?: boolean;
	armor?: boolean;
	mods?: CombatModifiers;
	stageFactor?: number;
}): number {
	const role = combatRoleFactor(cat.specialization);
	const stageFactor = Math.max(0, cat.stageFactor ?? 1);
	if (role <= 0 || stageFactor <= 0) {
		return 0;
	}
	const mods = cat.mods ?? NEUTRAL_MODS;
	const atk =
		(cat.attack + (cat.weapon ? WEAPON_ATTACK_BONUS : 0)) *
		Math.max(0, mods.combatPowerMult);
	const def =
		(cat.defense + (cat.armor ? ARMOR_DEFENSE_BONUS : 0)) *
		Math.max(0, mods.defenseMult);
	const xpBonus =
		1 + tradeLevel(cat.warriorXp ?? 0) * WARRIOR_XP_POWER_PER_LEVEL;
	return (atk + def) * role * xpBonus * stageFactor;
}

export interface MusterCombatant {
	id: string;
	attack: number;
	defense: number;
	specialization: WarriorSpecialization;
	warriorXp?: number;
	/** Life stage, so elders muster weaker and kittens not at all. */
	lifeStage?: LifeStage;
}

export interface MusteredCat {
	id: string;
	power: number;
	weapon: boolean;
	armor: boolean;
}

export interface DefenseMuster {
	/** Total combat power of everyone who turned out, with gear applied. */
	totalPower: number;
	/** Weapons drawn from the stockpile (to be consumed by the caller). */
	weaponsUsed: number;
	/** Armor drawn from the stockpile (to be consumed by the caller). */
	armorUsed: number;
	/** Per-cat breakdown, warriors first, in muster order. */
	perCat: MusteredCat[];
	/** Warriors + hunters who joined the fight. */
	combatants: number;
}

/**
 * Muster the colony's defenders against a raid. Every able-bodied cat turns
 * out — warriors and hunters at the front, the rest as militia; the available
 * weapons and armor are handed to the strongest warriors first (then hunters,
 * then militia), so scarce gear arms the cats who use it best. Returns the
 * total power and exactly how much gear was consumed.
 */
export function musterDefense(
	combatants: MusterCombatant[],
	stock: { weapons: number; armor: number },
	mods: CombatModifiers = NEUTRAL_MODS,
): DefenseMuster {
	// Warriors before hunters; within a role, stronger base stats first so the
	// best fighters get the scarce gear.
	const order = combatants
		.filter((c) => canFight(c.specialization))
		.sort((a, b) => {
			const roleDiff =
				combatRoleFactor(b.specialization) - combatRoleFactor(a.specialization);
			if (roleDiff !== 0) {
				return roleDiff;
			}
			return b.attack + b.defense - (a.attack + a.defense);
		});

	let weapons = Math.max(0, Math.floor(stock.weapons));
	let armor = Math.max(0, Math.floor(stock.armor));
	let weaponsUsed = 0;
	let armorUsed = 0;
	let totalPower = 0;
	const perCat: MusteredCat[] = [];

	for (const cat of order) {
		const hasWeapon = weapons > 0;
		const hasArmor = armor > 0;
		if (hasWeapon) {
			weapons -= 1;
			weaponsUsed += 1;
		}
		if (hasArmor) {
			armor -= 1;
			armorUsed += 1;
		}
		const power = catCombatPower({
			attack: cat.attack,
			defense: cat.defense,
			specialization: cat.specialization,
			warriorXp: cat.warriorXp,
			weapon: hasWeapon,
			armor: hasArmor,
			mods,
			stageFactor: cat.lifeStage ? combatStageFactor(cat.lifeStage) : 1,
		});
		totalPower += power;
		perCat.push({ id: cat.id, power, weapon: hasWeapon, armor: hasArmor });
	}

	return {
		totalPower,
		weaponsUsed,
		armorUsed,
		perCat,
		combatants: order.length,
	};
}

/** Warrior-trade XP track key on RoleXpJson. */
export const WARRIOR_XP_KEY = "warrior" as const;
