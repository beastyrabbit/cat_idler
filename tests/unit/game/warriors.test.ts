import { describe, expect, it } from "vitest";

import {
	ARMOR_DEFENSE_BONUS,
	canFight,
	catCombatPower,
	combatRoleFactor,
	combatStageFactor,
	HUNTER_COMBAT_FACTOR,
	MILITIA_COMBAT_FACTOR,
	musterDefense,
	WARRIOR_COMBAT_FACTOR,
	WEAPON_ATTACK_BONUS,
} from "@/lib/game/warriors";

describe("warriors", () => {
	describe("combatRoleFactor / canFight", () => {
		it("ranks warriors above hunters above militia, all able to fight", () => {
			expect(combatRoleFactor("warrior")).toBe(WARRIOR_COMBAT_FACTOR);
			expect(combatRoleFactor("hunter")).toBe(HUNTER_COMBAT_FACTOR);
			// Every other cat forms the militia — a smaller, positive contribution.
			expect(combatRoleFactor("architect")).toBe(MILITIA_COMBAT_FACTOR);
			expect(combatRoleFactor("ritualist")).toBe(MILITIA_COMBAT_FACTOR);
			expect(combatRoleFactor(null)).toBe(MILITIA_COMBAT_FACTOR);
			// Tier ordering: warrior > hunter > militia > 0.
			expect(WARRIOR_COMBAT_FACTOR).toBeGreaterThan(HUNTER_COMBAT_FACTOR);
			expect(HUNTER_COMBAT_FACTOR).toBeGreaterThan(MILITIA_COMBAT_FACTOR);
			expect(MILITIA_COMBAT_FACTOR).toBeGreaterThan(0);
			// Every specialization can turn out — the caller gates kittens by stage.
			expect(canFight("warrior")).toBe(true);
			expect(canFight("hunter")).toBe(true);
			expect(canFight(null)).toBe(true);
		});
	});

	describe("combatStageFactor", () => {
		it("kittens never fight, elders fade, adults are the backbone", () => {
			expect(combatStageFactor("kitten")).toBe(0);
			expect(combatStageFactor("adult")).toBe(1);
			// Young cats aren't fully grown; elders have lost a step.
			expect(combatStageFactor("young")).toBeGreaterThan(0);
			expect(combatStageFactor("young")).toBeLessThan(1);
			expect(combatStageFactor("elder")).toBeGreaterThan(0);
			expect(combatStageFactor("elder")).toBeLessThan(
				combatStageFactor("young"),
			);
		});
	});

	describe("catCombatPower", () => {
		it("an untrained cat still fights as militia", () => {
			const militia = catCombatPower({
				attack: 50,
				defense: 50,
				specialization: "architect",
			});
			expect(militia).toBeGreaterThan(0);
			// But well below a warrior with the same stats.
			const warrior = catCombatPower({
				attack: 50,
				defense: 50,
				specialization: "warrior",
			});
			expect(militia).toBeLessThan(warrior);
		});

		it("a kitten stage factor zeroes out combat power", () => {
			expect(
				catCombatPower({
					attack: 99,
					defense: 99,
					specialization: "warrior",
					stageFactor: 0,
				}),
			).toBe(0);
		});

		it("an elder stage factor lowers power below an adult", () => {
			const base = {
				attack: 50,
				defense: 50,
				specialization: "warrior" as const,
			};
			const adult = catCombatPower({ ...base, stageFactor: 1 });
			const elder = catCombatPower({
				...base,
				stageFactor: combatStageFactor("elder"),
			});
			expect(elder).toBeGreaterThan(0);
			expect(elder).toBeLessThan(adult);
		});

		it("warrior > hunter > militia with identical stats", () => {
			const base = { attack: 50, defense: 50 } as const;
			const warrior = catCombatPower({ ...base, specialization: "warrior" });
			const hunter = catCombatPower({ ...base, specialization: "hunter" });
			const militia = catCombatPower({ ...base, specialization: null });
			expect(warrior).toBeGreaterThan(hunter);
			expect(hunter).toBeGreaterThan(militia);
			expect(militia).toBeGreaterThan(0);
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

		it("musters militia behind the warriors", () => {
			const muster = musterDefense(
				[
					w("warr", 50, 50),
					{ id: "arch", attack: 99, defense: 99, specialization: "architect" },
				],
				{ weapons: 0, armor: 0 },
			);
			// Both turn out now, but the warrior sorts ahead of the militia.
			expect(muster.combatants).toBe(2);
			expect(muster.perCat).toHaveLength(2);
			expect(muster.perCat[0].id).toBe("warr");
			expect(muster.perCat[1].id).toBe("arch");
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

		it("scales mustered power by life stage", () => {
			const adult = musterDefense(
				[{ ...w("a", 50, 50), lifeStage: "adult" as const }],
				{ weapons: 0, armor: 0 },
			);
			const elder = musterDefense(
				[{ ...w("a", 50, 50), lifeStage: "elder" as const }],
				{ weapons: 0, armor: 0 },
			);
			const kitten = musterDefense(
				[{ ...w("a", 50, 50), lifeStage: "kitten" as const }],
				{ weapons: 0, armor: 0 },
			);
			expect(elder.totalPower).toBeGreaterThan(0);
			expect(elder.totalPower).toBeLessThan(adult.totalPower);
			expect(kitten.totalPower).toBe(0);
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
