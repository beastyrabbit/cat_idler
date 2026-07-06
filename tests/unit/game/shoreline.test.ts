import { describe, expect, it } from "vitest";

import {
	shorelineFishingSites,
	tileHasFishableWater,
	type ShorelineTile,
} from "@/lib/game/shoreline";
import { fromTiles } from "@/lib/game/villageArea";

const anchor = { x: 6, y: 6 };

function tile(
	x: number,
	y: number,
	type = "grassland",
	water = 0,
): ShorelineTile & { pathWear: number } {
	return {
		x,
		y,
		type,
		overlayFeature: type === "river" ? "river" : null,
		resources: { water },
		pathWear: 99,
	};
}

describe("shorelineFishingSites", () => {
	it("finds explored land orthogonally adjacent to water", () => {
		const tiles = [
			tile(6, 7, "river", 999),
			tile(6, 6),
			tile(6, 8),
			tile(7, 7),
			tile(5, 7),
		];

		expect(
			shorelineFishingSites({
				tiles,
				anchor,
				isExplored: () => true,
			}),
		).toEqual([
			{ x: 6, y: 6 },
			{ x: 5, y: 7 },
			{ x: 7, y: 7 },
			{ x: 6, y: 8 },
		]);
	});

	it("never returns the water tile itself", () => {
		const water = tile(6, 7, "river", 999);
		expect(tileHasFishableWater(water)).toBe(true);
		expect(
			shorelineFishingSites({
				tiles: [water, tile(6, 6), tile(6, 8, "river", 999)],
				anchor,
				isExplored: () => true,
			}),
		).toEqual([{ x: 6, y: 6 }]);
	});

	it("filters unexplored water and shoreline candidates", () => {
		const tiles = [
			{ ...tile(6, 7, "river", 999), pathWear: 99 },
			{ ...tile(6, 6), pathWear: 0 },
			{ ...tile(7, 7), pathWear: 99 },
		];
		expect(
			shorelineFishingSites({
				tiles,
				anchor,
				isExplored: (t) => t.pathWear > 62,
			}),
		).toEqual([{ x: 7, y: 7 }]);
	});

	it("does not choose a land tile that fishes across a blocked fence edge", () => {
		const area = fromTiles([{ x: 6, y: 6 }]);
		const tiles = [tile(6, 7, "river", 999), tile(6, 6), tile(7, 7)];

		expect(
			shorelineFishingSites({
				tiles,
				anchor,
				isExplored: () => true,
				area,
				gate: { x: 6, y: 6, side: "E" },
			}),
		).toEqual([{ x: 7, y: 7 }]);
	});
});
