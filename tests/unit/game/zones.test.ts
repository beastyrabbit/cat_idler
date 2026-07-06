import { describe, expect, it } from "vitest";

import {
	dominantZoneFloor,
	filterTargetsByZones,
	isInZone,
	normalizeRect,
	pickTargetWithZones,
	scoreTileWithZones,
	validateZone,
	zoneAppliesTo,
	ZONE_MAX_DURATION_MS,
	ZONE_MAX_EDGE,
	ZONE_MAX_PER_PLAYER,
	ZONE_MIN_DURATION_MS,
	type Zone,
} from "@/lib/game/zones";

const GATHER: Zone = { kind: "gather", x1: 0, y1: 0, x2: 3, y2: 3 };
const AVOID: Zone = { kind: "avoid", x1: 10, y1: 10, x2: 12, y2: 12 };

describe("zones", () => {
	describe("normalizeRect", () => {
		it("orders corners and rounds to tiles", () => {
			expect(normalizeRect({ x: 5.6, y: 1 }, { x: 2, y: 4.2 })).toEqual({
				x1: 2,
				y1: 1,
				x2: 6,
				y2: 4,
			});
		});
	});

	describe("isInZone", () => {
		it("includes all four edges", () => {
			for (const pos of [
				{ x: 10, y: 10 },
				{ x: 12, y: 12 },
				{ x: 10, y: 12 },
				{ x: 12, y: 10 },
				{ x: 11, y: 11 },
			]) {
				expect(isInZone(pos, AVOID)).toBe(true);
			}
		});

		it("excludes just outside the edges", () => {
			expect(isInZone({ x: 9, y: 10 }, AVOID)).toBe(false);
			expect(isInZone({ x: 13, y: 12 }, AVOID)).toBe(false);
			expect(isInZone({ x: 11, y: 13 }, AVOID)).toBe(false);
		});
	});

	describe("elevation-aware zone matching", () => {
		const mixed = { kind: "gather" as const, x1: 0, y1: 0, x2: 2, y2: 1 };
		const heightAt = (x: number) => (x === 2 ? 1 : 0);

		it("snaps a mixed rectangle to its dominant floor", () => {
			expect(dominantZoneFloor(mixed, heightAt)).toBe(0);
			expect(zoneAppliesTo({ x: 1, y: 1 }, mixed, { heightAt })).toBe(true);
			expect(zoneAppliesTo({ x: 2, y: 1 }, mixed, { heightAt })).toBe(false);
		});

		it("uses the lower floor as a deterministic tie-break", () => {
			const tied = { x1: 0, y1: 0, x2: 1, y2: 0 };
			expect(dominantZoneFloor(tied, (x) => x)).toBe(0);
		});
	});

	describe("validateZone", () => {
		const rect = { x1: 0, y1: 0, x2: 7, y2: 7 };

		it("accepts an 8x8 zone at the limit", () => {
			expect(validateZone(rect, ZONE_MIN_DURATION_MS, 0)).toBeNull();
			expect(validateZone(rect, ZONE_MAX_DURATION_MS, 1)).toBeNull();
		});

		it("rejects a 9-wide zone", () => {
			expect(
				validateZone({ x1: 0, y1: 0, x2: ZONE_MAX_EDGE, y2: 2 }, 600_000, 0),
			).toMatch(/limited/);
		});

		it("rejects too many zones per player", () => {
			expect(validateZone(rect, 600_000, ZONE_MAX_PER_PLAYER)).toMatch(
				/active zones/,
			);
		});

		it("rejects out-of-range durations", () => {
			expect(validateZone(rect, ZONE_MIN_DURATION_MS - 1, 0)).toMatch(
				/duration/,
			);
			expect(validateZone(rect, ZONE_MAX_DURATION_MS + 1, 0)).toMatch(
				/duration/,
			);
		});
	});

	describe("scoreTileWithZones", () => {
		it("doubles inside gather zones", () => {
			expect(scoreTileWithZones(10, { x: 1, y: 1 }, [GATHER])).toBe(20);
		});

		it("zeroes inside avoid zones", () => {
			expect(scoreTileWithZones(10, { x: 11, y: 11 }, [AVOID])).toBe(0);
		});

		it("keeps avoid tiles for critical needs", () => {
			expect(scoreTileWithZones(10, { x: 11, y: 11 }, [AVOID], true)).toBe(10);
		});

		it("is unchanged outside all zones", () => {
			expect(scoreTileWithZones(10, { x: 50, y: 50 }, [GATHER, AVOID])).toBe(
				10,
			);
		});

		it("only applies gather weighting on the zone's dominant floor", () => {
			const zone: Zone = { kind: "gather", x1: 0, y1: 0, x2: 2, y2: 0 };
			const heightAt = (x: number) => (x === 2 ? 1 : 0);
			expect(scoreTileWithZones(10, { x: 1, y: 0 }, [zone], { heightAt })).toBe(
				20,
			);
			expect(scoreTileWithZones(10, { x: 2, y: 0 }, [zone], { heightAt })).toBe(
				10,
			);
		});
	});

	describe("filterTargetsByZones", () => {
		const targets = [
			{ x: 1, y: 1 },
			{ x: 11, y: 11 },
			{ x: 50, y: 50 },
		];

		it("removes avoid-zone targets", () => {
			expect(filterTargetsByZones(targets, [AVOID])).toEqual([
				{ x: 1, y: 1 },
				{ x: 50, y: 50 },
			]);
		});

		it("keeps everything under critical needs", () => {
			expect(filterTargetsByZones(targets, [AVOID], true)).toEqual(targets);
		});

		it("does not let an avoid zone remove targets on a different floor", () => {
			const zone: Zone = { kind: "avoid", x1: 0, y1: 0, x2: 2, y2: 0 };
			const heightAt = (x: number) => (x === 2 ? 1 : 0);
			expect(
				filterTargetsByZones(
					[
						{ x: 1, y: 0 },
						{ x: 2, y: 0 },
					],
					[zone],
					{ heightAt },
				),
			).toEqual([{ x: 2, y: 0 }]);
		});
	});

	describe("pickTargetWithZones", () => {
		it("never picks an avoid tile when alternatives exist", () => {
			const targets = [
				{ x: 11, y: 11 },
				{ x: 50, y: 50 },
			];
			for (const roll of [0, 0.3, 0.6, 0.99]) {
				expect(pickTargetWithZones(targets, [AVOID], roll)).toEqual({
					x: 50,
					y: 50,
				});
			}
		});

		it("falls back to avoid tiles when nothing else exists", () => {
			const targets = [{ x: 11, y: 11 }];
			expect(pickTargetWithZones(targets, [AVOID], 0.5)).toEqual({
				x: 11,
				y: 11,
			});
		});

		it("weights gather tiles double", () => {
			const targets = [
				{ x: 1, y: 1 }, // in gather zone → 2 slots of 3
				{ x: 50, y: 50 },
			];
			expect(pickTargetWithZones(targets, [GATHER], 0.0)).toEqual({
				x: 1,
				y: 1,
			});
			expect(pickTargetWithZones(targets, [GATHER], 0.5)).toEqual({
				x: 1,
				y: 1,
			});
			expect(pickTargetWithZones(targets, [GATHER], 0.9)).toEqual({
				x: 50,
				y: 50,
			});
		});

		it("returns null for no targets", () => {
			expect(pickTargetWithZones([], [], 0.5)).toBeNull();
		});

		it("weights only same-floor gather targets", () => {
			const zone: Zone = { kind: "gather", x1: 0, y1: 0, x2: 2, y2: 0 };
			const targets = [
				{ x: 1, y: 0 },
				{ x: 2, y: 0 },
			];
			const heightAt = (x: number) => (x === 2 ? 1 : 0);
			expect(pickTargetWithZones(targets, [zone], 0.6, { heightAt })).toEqual({
				x: 1,
				y: 0,
			});
			expect(pickTargetWithZones(targets, [zone], 0.9, { heightAt })).toEqual({
				x: 2,
				y: 0,
			});
		});
	});
});
