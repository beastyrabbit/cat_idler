import { describe, expect, it } from "vitest";

import {
	advanceMovement,
	destinationForJob,
	MOVE_SPEED_TILES_PER_SEC,
	pickWanderTarget,
	WANDER_RADIUS,
} from "@/lib/game/movement";

const ANCHOR = { x: 6, y: 6 };

describe("movement", () => {
	describe("advanceMovement", () => {
		it("steps along x before y (greedy axis)", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 3, y: 3 }, 2, 1);
			expect(step.position).toEqual({ x: 2, y: 0 });
			expect(step.arrived).toBe(false);
		});

		it("spills leftover budget from x into y", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 1, y: 5 }, 3, 1);
			expect(step.position).toEqual({ x: 1, y: 2 });
			expect(step.arrived).toBe(false);
		});

		it("arrives exactly at the destination without overshooting", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 1, y: 1 }, 100, 1);
			expect(step.position).toEqual({ x: 1, y: 1 });
			expect(step.arrived).toBe(true);
		});

		it("moves at the configured speed", () => {
			const step = advanceMovement(
				{ x: 0, y: 0 },
				{ x: 10, y: 0 },
				2,
				MOVE_SPEED_TILES_PER_SEC,
			);
			expect(step.position.x).toBeCloseTo(2 * MOVE_SPEED_TILES_PER_SEC, 6);
		});

		it("handles negative directions", () => {
			const step = advanceMovement({ x: 5, y: 5 }, { x: 2, y: 5 }, 2, 1);
			expect(step.position).toEqual({ x: 3, y: 5 });
		});

		it("is a no-op when already at the destination", () => {
			const step = advanceMovement({ x: 4, y: 4 }, { x: 4, y: 4 }, 10, 1);
			expect(step.position).toEqual({ x: 4, y: 4 });
			expect(step.arrived).toBe(true);
		});

		it("clamps zero/negative elapsed time", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 5, y: 0 }, 0, 1);
			expect(step.position).toEqual({ x: 0, y: 0 });
			expect(step.arrived).toBe(false);
		});
	});

	describe("pickWanderTarget", () => {
		it("stays within the wander radius of the anchor", () => {
			for (let i = 0; i < 20; i++) {
				const roll1 = ((i * 37) % 100) / 100;
				const roll2 = ((i * 61) % 100) / 100;
				const target = pickWanderTarget(ANCHOR, roll1, roll2);
				expect(Math.abs(target.x - ANCHOR.x)).toBeLessThanOrEqual(
					WANDER_RADIUS,
				);
				expect(Math.abs(target.y - ANCHOR.y)).toBeLessThanOrEqual(
					WANDER_RADIUS,
				);
			}
		});

		it("returns integer tiles and is deterministic in its rolls", () => {
			const a = pickWanderTarget(ANCHOR, 0.42, 0.77);
			const b = pickWanderTarget(ANCHOR, 0.42, 0.77);
			expect(a).toEqual(b);
			expect(Number.isInteger(a.x)).toBe(true);
			expect(Number.isInteger(a.y)).toBe(true);
		});

		it("covers different targets for different rolls", () => {
			const a = pickWanderTarget(ANCHOR, 0.1, 0.1);
			const b = pickWanderTarget(ANCHOR, 0.9, 0.9);
			expect(a).not.toEqual(b);
		});
	});

	describe("destinationForJob", () => {
		it("sends ritualists to the shrine", () => {
			const dest = destinationForJob("ritual", {
				anchor: ANCHOR,
				shrine: { x: 6, y: 6 },
				foodTiles: [],
				roll: 0.5,
			});
			expect(dest).toEqual({ x: 6, y: 6 });
		});

		it("sends hunters to a food-rich tile chosen by the roll", () => {
			const foodTiles = [
				{ x: 20, y: 4 },
				{ x: -3, y: 14 },
				{ x: 11, y: -6 },
			];
			const dest = destinationForJob("hunt_expedition", {
				anchor: ANCHOR,
				shrine: { x: 6, y: 6 },
				foodTiles,
				roll: 0.5,
			});
			expect(dest).toEqual(foodTiles[1]);
		});

		it("falls back to a distant point when no food tiles are known", () => {
			const dest = destinationForJob("hunt_expedition", {
				anchor: ANCHOR,
				shrine: { x: 6, y: 6 },
				foodTiles: [],
				roll: 0.25,
			});
			expect(dest).not.toBeNull();
			const distance = Math.max(
				Math.abs((dest as { x: number }).x - ANCHOR.x),
				Math.abs((dest as { y: number }).y - ANCHOR.y),
			);
			expect(distance).toBeGreaterThanOrEqual(8);
		});

		it("sends builders to the given site", () => {
			const dest = destinationForJob("build_house", {
				anchor: ANCHOR,
				shrine: { x: 6, y: 6 },
				foodTiles: [],
				roll: 0.5,
				site: { x: 8, y: 5 },
			});
			expect(dest).toEqual({ x: 8, y: 5 });
		});

		it("returns null for player supply actions", () => {
			for (const kind of [
				"supply_food",
				"supply_water",
				"leader_plan_hunt",
				"leader_plan_house",
			]) {
				expect(
					destinationForJob(kind, {
						anchor: ANCHOR,
						shrine: { x: 6, y: 6 },
						foodTiles: [],
						roll: 0.5,
					}),
				).toBeNull();
			}
		});
	});
});
