import { describe, expect, it } from "vitest";
import {
	BRIDGE_DETOUR_SAVING_THRESHOLD,
	selectBestBridgeCandidate,
	validateBridgePlacement,
	type BridgeTile,
} from "@/lib/game/bridges";
import { buildColonyWalkGrid } from "@/lib/game/pathfinding";

function tile(
	x: number,
	y: number,
	overrides: Partial<BridgeTile> = {},
): BridgeTile {
	return {
		x,
		y,
		type: "field",
		overlayFeature: null,
		resources: { water: 0 },
		pathWear: 0,
		...overrides,
	};
}

describe("bridge placement", () => {
	it("rejects non-water tiles", () => {
		const tiles = [tile(0, 0), tile(-1, 0), tile(1, 0)];
		expect(validateBridgePlacement(tiles, { x: 0, y: 0 })).toEqual({
			ok: false,
			reason: "not_water",
		});
	});

	it("rejects water without both opposing banks", () => {
		const tiles = [
			tile(0, 0, { type: "river", overlayFeature: "river" }),
			tile(-1, 0),
			tile(0, 1),
		];
		expect(validateBridgePlacement(tiles, { x: 0, y: 0 })).toEqual({
			ok: false,
			reason: "missing_banks",
		});
	});

	it("accepts a one-tile river crossing with opposing dry banks", () => {
		const tiles = [
			tile(0, 0, { type: "river", overlayFeature: "river" }),
			tile(-1, 0),
			tile(1, 0),
		];
		expect(validateBridgePlacement(tiles, { x: 0, y: 0 })).toEqual({
			ok: true,
			position: { x: 0, y: 0 },
			orientation: "east_west",
			banks: [
				{ x: -1, y: 0 },
				{ x: 1, y: 0 },
			],
		});
	});
});

describe("bridge candidate economics", () => {
	it("scores the best valid crossing from detour savings", () => {
		const tiles: BridgeTile[] = [];
		for (let y = -8; y <= 8; y += 1) {
			tiles.push(tile(0, y, { type: "river", overlayFeature: "river" }));
		}
		tiles.push(tile(-1, 0), tile(1, 0));
		// A dry gap far north creates a long but finite detour.
		tiles.push(tile(0, -9));
		const grid = buildColonyWalkGrid({
			tiles,
			anchor: { x: 50, y: 50 },
			ringRadius: 4,
			gate: { x: 50, y: 54 },
		});

		const candidate = selectBestBridgeCandidate({
			tiles,
			grid,
			isExplored: () => true,
		});

		expect(candidate?.position).toEqual({ x: 0, y: 0 });
		expect(candidate?.weightedSaving).toBeGreaterThan(
			BRIDGE_DETOUR_SAVING_THRESHOLD,
		);
	});
});
