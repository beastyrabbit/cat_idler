import { describe, expect, it } from "vitest";

import {
	accrueThreat,
	colonyWealth,
	MAX_RAID_CASUALTIES,
	MAX_RAID_SIZE,
	planRaid,
	RAID_GRACE_SEC,
	RAID_SPAWN_THRESHOLD,
	resolveRaid,
	shouldSpawnRaid,
	threatBand,
	threatRatePerHour,
} from "@/lib/game/threat";

const snap = (over: Partial<Parameters<typeof threatRatePerHour>[0]> = {}) => ({
	wealth: 500,
	population: 20,
	warriors: 2,
	colonyAgeSec: RAID_GRACE_SEC + 3600,
	...over,
});

describe("threat", () => {
	describe("colonyWealth", () => {
		it("weights refined and gear above raw stores", () => {
			const raw = colonyWealth({ food: 10 });
			const gear = colonyWealth({ weapons: 10 });
			expect(gear).toBeGreaterThan(raw);
			expect(colonyWealth({})).toBe(0);
		});
	});

	describe("threatRatePerHour", () => {
		it("is zero during the grace window", () => {
			expect(threatRatePerHour(snap({ colonyAgeSec: 0 }))).toBe(0);
			expect(
				threatRatePerHour(snap({ colonyAgeSec: RAID_GRACE_SEC - 1 })),
			).toBe(0);
		});

		it("turns on once grace elapses", () => {
			expect(
				threatRatePerHour(snap({ colonyAgeSec: RAID_GRACE_SEC })),
			).toBeGreaterThan(0);
		});

		it("rises with wealth, population and warriors", () => {
			const base = threatRatePerHour(snap());
			expect(threatRatePerHour(snap({ wealth: 5000 }))).toBeGreaterThan(base);
			expect(threatRatePerHour(snap({ population: 40 }))).toBeGreaterThan(base);
			expect(threatRatePerHour(snap({ warriors: 10 }))).toBeGreaterThan(base);
		});
	});

	describe("accrueThreat", () => {
		it("adds pressure over elapsed game-time", () => {
			const next = accrueThreat(0, snap(), 3600);
			expect(next).toBeGreaterThan(0);
		});

		it("stays flat during grace", () => {
			expect(accrueThreat(10, snap({ colonyAgeSec: 0 }), 3600)).toBe(10);
		});

		it("never goes negative", () => {
			expect(accrueThreat(-5, snap(), 0)).toBe(0);
		});
	});

	describe("shouldSpawnRaid / threatBand", () => {
		it("spawns at the threshold", () => {
			expect(shouldSpawnRaid(RAID_SPAWN_THRESHOLD - 0.01)).toBe(false);
			expect(shouldSpawnRaid(RAID_SPAWN_THRESHOLD)).toBe(true);
		});

		it("bands the HUD indicator", () => {
			expect(threatBand(0)).toBe("calm");
			expect(threatBand(RAID_SPAWN_THRESHOLD / 2)).toBe("rising");
			expect(threatBand(RAID_SPAWN_THRESHOLD)).toBe("imminent");
		});
	});

	describe("planRaid", () => {
		it("fields at least one raider", () => {
			expect(
				planRaid(snap({ warriors: 0, wealth: 0 })).count,
			).toBeGreaterThanOrEqual(1);
		});

		it("scales the warband with warriors and wealth", () => {
			const small = planRaid(snap({ warriors: 0, wealth: 100 })).count;
			const big = planRaid(snap({ warriors: 8, wealth: 5000 })).count;
			expect(big).toBeGreaterThan(small);
		});

		it("caps the warband size", () => {
			expect(
				planRaid(snap({ warriors: 100, wealth: 1_000_000 })).count,
			).toBeLessThanOrEqual(MAX_RAID_SIZE);
		});

		it("stiffens raiders as the colony ages", () => {
			const young = planRaid(
				snap({ colonyAgeSec: RAID_GRACE_SEC }),
			).strengthEach;
			const old = planRaid(
				snap({ colonyAgeSec: RAID_GRACE_SEC + 40 * 3600 }),
			).strengthEach;
			expect(old).toBeGreaterThan(young);
		});

		it("fields a single weak raider against a fresh settlement", () => {
			// Starter colony: no warriors, ~240 stored value, starter roster, just
			// past the grace window. Its first raid is one lone raider.
			const starter = planRaid({
				wealth: 240,
				population: 20,
				warriors: 0,
				colonyAgeSec: RAID_GRACE_SEC,
			});
			expect(starter.count).toBe(1);
		});

		it("a wealthy, populous, well-armed colony draws a meaningfully bigger warband", () => {
			const starter = planRaid({
				wealth: 240,
				population: 20,
				warriors: 0,
				colonyAgeSec: RAID_GRACE_SEC,
			});
			const wealthy = planRaid({
				wealth: 6000,
				population: 45,
				warriors: 8,
				colonyAgeSec: RAID_GRACE_SEC + 30 * 3600,
			});
			// Both count and per-raider strength should have grown.
			expect(wealthy.count).toBeGreaterThan(starter.count + 3);
			expect(wealthy.strengthEach).toBeGreaterThan(starter.strengthEach);
			// Total warband power dwarfs the opening raid.
			expect(wealthy.count * wealthy.strengthEach).toBeGreaterThan(
				5 * (starter.count * starter.strengthEach),
			);
		});
	});

	describe("resolveRaid", () => {
		it("defenders rout a much weaker warband with no losses", () => {
			const out = resolveRaid(1000, 100, 0.5);
			expect(out.defendersWin).toBe(true);
			expect(out.defenderCasualties).toBe(0);
			expect(out.lootFraction).toBe(0);
		});

		it("defenders lose to an overwhelming warband and stores are stolen", () => {
			const out = resolveRaid(50, 1000, 0.5);
			expect(out.defendersWin).toBe(false);
			expect(out.lootFraction).toBeGreaterThan(0);
			expect(out.defenderCasualties).toBe(MAX_RAID_CASUALTIES);
		});

		it("caps casualties at one death and theft at a minority of stores", () => {
			// However lopsided the fight, a lost raid can never kill more than the
			// cap or carry off more than a bounded share — one bad fight never wipes.
			for (const raider of [200, 1000, 10_000, 1_000_000]) {
				for (const roll of [0, 0.25, 0.5, 0.75, 0.999]) {
					const out = resolveRaid(10, raider, roll);
					expect(out.defenderCasualties).toBeLessThanOrEqual(
						MAX_RAID_CASUALTIES,
					);
					expect(out.lootFraction).toBeLessThanOrEqual(0.3);
				}
			}
			expect(MAX_RAID_CASUALTIES).toBe(1);
		});

		it("is deterministic in the roll", () => {
			expect(resolveRaid(100, 100, 0.3)).toEqual(resolveRaid(100, 100, 0.3));
		});

		it("the roll swings close fights", () => {
			// At power parity, a low roll (0.75x swing) loses, a high roll wins.
			expect(resolveRaid(100, 100, 0).defendersWin).toBe(false);
			expect(resolveRaid(100, 100, 0.999).defendersWin).toBe(true);
		});
	});
});
