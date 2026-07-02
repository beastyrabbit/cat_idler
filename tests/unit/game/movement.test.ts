import { describe, expect, it } from "vitest";

import {
	advanceMovement,
	destinationForJob,
	EXPLORE_SPEED_FACTOR,
	MOVE_SPEED_TILES_PER_SEC,
	pathTiles,
	pickWanderTarget,
	WANDER_RADIUS,
	type WorldPos,
	walkPath,
} from "@/lib/game/movement";

const key = (p: WorldPos) => `${p.x},${p.y}`;
const keys = (tiles: WorldPos[]) => tiles.map(key);

const ANCHOR = { x: 6, y: 6 };

describe("movement", () => {
	describe("advanceMovement", () => {
		it("steps along x before y (greedy axis)", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 3, y: 3 }, 2, 1);
			expect(step.position).toEqual({ x: 2, y: 0 });
			expect(step.arrived).toBe(false);
		});

		it("turns corners on separate steps (no diagonal cutting)", () => {
			const step = advanceMovement({ x: 0, y: 0 }, { x: 1, y: 5 }, 3, 1);
			expect(step.position).toEqual({ x: 1, y: 0 });
			expect(step.arrived).toBe(false);
			const next = advanceMovement(step.position, { x: 1, y: 5 }, 3, 1);
			expect(next.position).toEqual({ x: 1, y: 3 });
		});

		it("arrives exactly at the destination without overshooting", () => {
			const leg = advanceMovement({ x: 0, y: 0 }, { x: 1, y: 1 }, 100, 1);
			const step = advanceMovement(leg.position, { x: 1, y: 1 }, 100, 1);
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

	describe("pathTiles", () => {
		it("lists a straight horizontal run inclusive of both ends", () => {
			expect(keys(pathTiles({ x: 2, y: 5 }, { x: 5, y: 5 }))).toEqual([
				"2,5",
				"3,5",
				"4,5",
				"5,5",
			]);
		});

		it("lists a straight vertical run (going negative)", () => {
			expect(keys(pathTiles({ x: 4, y: 3 }, { x: 4, y: 0 }))).toEqual([
				"4,3",
				"4,2",
				"4,1",
				"4,0",
			]);
		});

		it("walks x before y for an L-shaped route, no duplicate corner", () => {
			const tiles = pathTiles({ x: 0, y: 0 }, { x: 2, y: 2 });
			expect(keys(tiles)).toEqual(["0,0", "1,0", "2,0", "2,1", "2,2"]);
			// The corner tile (2,0) appears exactly once.
			expect(keys(tiles).filter((k) => k === "2,0")).toHaveLength(1);
		});

		it("returns a single tile for a zero-length hop", () => {
			expect(keys(pathTiles({ x: 7, y: 7 }, { x: 7, y: 7 }))).toEqual(["7,7"]);
		});

		it("rounds fractional endpoints onto the tile grid", () => {
			expect(keys(pathTiles({ x: 0.4, y: 1.6 }, { x: 2.5, y: 1.6 }))).toEqual([
				"0,2",
				"1,2",
				"2,2",
				"3,2",
			]);
		});

		it("covers every integer tile with no gaps along the route", () => {
			const tiles = pathTiles({ x: -3, y: 4 }, { x: 5, y: -2 });
			// Every step is exactly one tile from the previous (4-directional).
			for (let i = 1; i < tiles.length; i++) {
				const dist =
					Math.abs(tiles[i].x - tiles[i - 1].x) +
					Math.abs(tiles[i].y - tiles[i - 1].y);
				expect(dist).toBe(1);
			}
			expect(tiles[0]).toEqual({ x: -3, y: 4 });
			expect(tiles[tiles.length - 1]).toEqual({ x: 5, y: -2 });
		});
	});

	describe("walkPath", () => {
		it("spends the whole budget across both axes and arrives", () => {
			const walk = walkPath({ x: 0, y: 0 }, { x: 3, y: 2 }, 100);
			expect(walk.position).toEqual({ x: 3, y: 2 });
			expect(walk.arrived).toBe(true);
			expect(keys(walk.tiles)).toEqual([
				"0,0",
				"1,0",
				"2,0",
				"3,0",
				"3,1",
				"3,2",
			]);
		});

		it("stops where the budget runs out and reports not-arrived", () => {
			// x-first: 3 tiles east exhausts the budget before any y movement.
			const walk = walkPath({ x: 0, y: 0 }, { x: 5, y: 4 }, 3);
			expect(walk.position).toEqual({ x: 3, y: 0 });
			expect(walk.arrived).toBe(false);
			expect(keys(walk.tiles)).toEqual(["0,0", "1,0", "2,0", "3,0"]);
		});

		it("turns the corner within a single call once x is satisfied", () => {
			// 2 east + 3 remaining spent going north.
			const walk = walkPath({ x: 0, y: 0 }, { x: 2, y: 10 }, 5);
			expect(walk.position).toEqual({ x: 2, y: 3 });
			expect(walk.arrived).toBe(false);
			expect(keys(walk.tiles)).toEqual([
				"0,0",
				"1,0",
				"2,0",
				"2,1",
				"2,2",
				"2,3",
			]);
		});

		it("routes through a waypoint (the gate) before the destination", () => {
			const gate = { x: 6, y: 10 };
			const walk = walkPath({ x: 6, y: 6 }, { x: 12, y: 12 }, 100, [gate]);
			expect(walk.arrived).toBe(true);
			expect(walk.position).toEqual({ x: 12, y: 12 });
			// Trail dips down to the gate first, then heads out — so it passes
			// through (6,10), which a straight x-first walk would never touch.
			expect(keys(walk.tiles)).toContain("6,10");
			// Still one contiguous 4-directional chain.
			for (let i = 1; i < walk.tiles.length; i++) {
				const dist =
					Math.abs(walk.tiles[i].x - walk.tiles[i - 1].x) +
					Math.abs(walk.tiles[i].y - walk.tiles[i - 1].y);
				expect(dist).toBe(1);
			}
		});

		it("records the start tile and no movement for a zero budget", () => {
			const walk = walkPath({ x: 4, y: 4 }, { x: 9, y: 9 }, 0);
			expect(walk.position).toEqual({ x: 4, y: 4 });
			expect(walk.arrived).toBe(false);
			expect(keys(walk.tiles)).toEqual(["4,4"]);
		});

		it("is already arrived when start equals destination", () => {
			const walk = walkPath({ x: 5, y: 5 }, { x: 5, y: 5 }, 10);
			expect(walk.arrived).toBe(true);
			expect(keys(walk.tiles)).toEqual(["5,5"]);
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

	describe("EXPLORE_SPEED_FACTOR", () => {
		it("makes explorers travel a fraction of normal speed", () => {
			expect(EXPLORE_SPEED_FACTOR).toBeGreaterThan(0);
			expect(EXPLORE_SPEED_FACTOR).toBeLessThan(1);
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

		it("sends water fetchers to the given water site", () => {
			const dest = destinationForJob("fetch_water", {
				anchor: ANCHOR,
				shrine: { x: 6, y: 6 },
				foodTiles: [],
				roll: 0.5,
				waterSite: { x: 9, y: 12 },
			});
			expect(dest).toEqual({ x: 9, y: 12 });
		});

		it("returns null for a water fetch with no known water tile", () => {
			expect(
				destinationForJob("fetch_water", {
					anchor: ANCHOR,
					shrine: { x: 6, y: 6 },
					foodTiles: [],
					roll: 0.5,
				}),
			).toBeNull();
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
