import { describe, expect, it } from "vitest";

import {
	ARMOR_DEFENSE_BONUS,
	canFight,
	catCombatPower,
	combatRoleFactor,
	HUNTER_COMBAT_FACTOR,
	musterDefense,
	WARRIOR_COMBAT_FACTOR,
	WEAPON_ATTACK_BONUS,
} from "@/lib/game/warriors";

describe("warriors", () => {
	describe("combatRoleFactor / canFight", () => {
		it("only warriors and hunters can fight", () => {
			expect(combatRoleFactor("warrior")).toBe(WARRIOR_COMBAT_FACTOR);
			expect(combatRoleFactor("hunter")).toBe(HUNTER_COMBAT_FACTOR);
			expect(combatRoleFactor("architect")).toBe(0);
			expect(combatRoleFactor("ritualist")).toBe(0);
			expect(combatRoleFactor(null)).toBe(0);
			expect(canFight("warrior")).toBe(true);
			expect(canFight("hunter")).toBe(true);
			expect(canFight(null)).toBe(false);
		});
	});

	describe("catCombatPower", () => {
		it("non-combatants score zero", () => {
			expect(
				catCombatPower({
					attack: 99,
					defense: 99,
					specialization: "architect",
				}),
			).toBe(0);
		});

		it("a warrior out-fights a hunter with identical stats", () => {
			const base = { attack: 50, defense: 50 } as const;
			const warrior = catCombatPower({ ...base, specialization: "warrior" });
			const hunter = catCombatPower({ ...base, specialization: "hunter" });
			expect(warrior).toBeGreaterThan(hunter);
			expect(hunter).toBeGreaterThan(0);
		});

		it("equipment raises power by the gear bonuses", () => {
			const bare = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
			});
			const armed = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
				weapon: true,
				armor: true,
			});
			// Weapon adds attack, armor adds defense; both fold into power.
			expect(armed).toBeCloseTo(
				bare +
					(WEAPON_ATTACK_BONUS + ARMOR_DEFENSE_BONUS) * WARRIOR_COMBAT_FACTOR,
			);
		});

		it("warrior trade level lifts power", () => {
			const green = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
				warriorXp: 0,
			});
			const veteran = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
				warriorXp: 100,
			});
			expect(veteran).toBeGreaterThan(green);
		});

		it("combat modifiers scale attack and defense contributions", () => {
			const plain = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
			});
			const buffed = catCombatPower({
				attack: 40,
				defense: 40,
				specialization: "warrior",
				mods: { combatPowerMult: 1.25, defenseMult: 1.25 },
			});
			expect(buffed).toBeCloseTo(plain * 1.25);
		});
	});

	describe("musterDefense", () => {
		const w = (id: string, atk: number, def: number) => ({
			id,
			attack: atk,
			defense: def,
			specialization: "warrior" as const,
		});

		it("excludes non-combatants", () => {
			const muster = musterDefense(
				[
					w("a", 50, 50),
					{ id: "b", attack: 99, defense: 99, specialization: "architect" },
				],
				{ weapons: 0, armor: 0 },
			);
			expect(muster.combatants).toBe(1);
			expect(muster.perCat).toHaveLength(1);
			expect(muster.perCat[0].id).toBe("a");
		});

		it("arms the strongest warriors first with scarce gear", () => {
			const muster = musterDefense([w("weak", 10, 10), w("strong", 80, 80)], {
				weapons: 1,
				armor: 1,
			});
			expect(muster.weaponsUsed).toBe(1);
			expect(muster.armorUsed).toBe(1);
			// Strong warrior sorted first and gets the gear.
			expect(muster.perCat[0].id).toBe("strong");
			expect(muster.perCat[0].weapon).toBe(true);
			expect(muster.perCat[1].weapon).toBe(false);
		});

		it("prioritizes warriors over hunters for gear", () => {
			const muster = musterDefense(
				[
					{
						id: "h",
						attack: 90,
						defense: 90,
						specialization: "hunter" as const,
					},
					w("warr", 40, 40),
				],
				{ weapons: 1, armor: 1 },
			);
			expect(muster.perCat[0].id).toBe("warr");
			expect(muster.perCat[0].weapon).toBe(true);
		});

		it("consumes no more gear than combatants can hold", () => {
			const muster = musterDefense([w("a", 50, 50)], {
				weapons: 5,
				armor: 5,
			});
			expect(muster.weaponsUsed).toBe(1);
			expect(muster.armorUsed).toBe(1);
		});

		it("totals the mustered power", () => {
			const muster = musterDefense([w("a", 50, 50), w("b", 30, 30)], {
				weapons: 0,
				armor: 0,
			});
			const expected =
				catCombatPower({ attack: 50, defense: 50, specialization: "warrior" }) +
				catCombatPower({ attack: 30, defense: 30, specialization: "warrior" });
			expect(muster.totalPower).toBeCloseTo(expected);
		});
	});
});
