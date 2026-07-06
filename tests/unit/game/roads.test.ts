import { describe, expect, it } from "vitest";
import type { WalkTile } from "@/lib/game/pathfinding";
import { ROAD_PAVE_WEAR, selectRoadCorridor } from "@/lib/game/roads";

const anchor = { x: 6, y: 6 };
const ringRadius = 4;

function tile(
	x: number,
	y: number,
	pathWear: number,
	overlayFeature: string | null = null,
): WalkTile {
	return {
		x,
		y,
		type: "grass",
		overlayFeature,
		resources: { water: 0 },
		pathWear,
	};
}

describe("selectRoadCorridor", () => {
	it("returns nothing when no tile clears the wear threshold", () => {
		const tiles = [tile(6, 12, 40), tile(6, 13, 20)];
		expect(
			selectRoadCorridor(tiles, { anchor, ringRadius, maxTiles: 6 }),
		).toEqual([]);
	});

	it("grows a connected corridor from the highest-wear trail tile", () => {
		// A worn 4-tile run heading south out of the gate, plus a stray worn tile
		// off on its own that should not be pulled in.
		const tiles = [
			tile(6, 11, 95),
			tile(6, 12, 90),
			tile(6, 13, 85),
			tile(6, 14, 80),
			tile(20, 20, 99), // disconnected — highest wear but not adjacent
		];
		const corridor = selectRoadCorridor(tiles, {
			anchor,
			ringRadius,
			maxTiles: 4,
		});
		// Starts at the global highest-wear tile, then grows through neighbours.
		expect(corridor[0]).toEqual({ x: 20, y: 20 });
		expect(corridor).toHaveLength(1); // the stray tile has no worn neighbour
	});

	it("paves the trafficked corridor, highest wear first, capped by budget", () => {
		const tiles = [
			tile(6, 11, 95),
			tile(6, 12, 90),
			tile(6, 13, 85),
			tile(6, 14, 80),
		];
		const corridor = selectRoadCorridor(tiles, {
			anchor,
			ringRadius,
			maxTiles: 3,
		});
		expect(corridor).toEqual([
			{ x: 6, y: 11 },
			{ x: 6, y: 12 },
			{ x: 6, y: 13 },
		]);
	});

	it("skips already-paved tiles and interior clearing", () => {
		const tiles = [
			tile(6, 11, 95, "road_built"), // already a road
			tile(6, 7, 99), // inside the fence (cheb 1) — clearing, not paved
			tile(6, 12, 90),
			tile(6, 13, 88),
		];
		const corridor = selectRoadCorridor(tiles, {
			anchor,
			ringRadius,
			maxTiles: 6,
		});
		expect(corridor.map((c) => `${c.x},${c.y}`)).toEqual(["6,12", "6,13"]);
	});

	it("exposes the pave threshold", () => {
		expect(ROAD_PAVE_WEAR).toBe(70);
	});
});
