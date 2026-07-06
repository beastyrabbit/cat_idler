import { describe, expect, it } from "vitest";
import {
	blockedElevationEdgeMask,
	ELEVATION_DIR,
	elevationBlocksStep,
	stairBridgesStep,
	stairElevationEdgeMask,
	type ElevationField,
} from "@/lib/game/elevation";

function field(
	heightAt: (x: number, y: number) => number,
	stairs: Set<string> = new Set(),
): ElevationField {
	return {
		heightAt,
		hasStair: (x, y) => stairs.has(`${x},${y}`),
	};
}

describe("elevation walk seams", () => {
	it("keeps worlds without a height field flat", () => {
		expect(elevationBlocksStep({}, 0, 0, 1, 0)).toBe(false);
		expect(stairBridgesStep({}, 0, 0, 1, 0)).toBe(false);
	});

	it("blocks floor-changing edges without stairs", () => {
		const ridge = field((x) => (x >= 1 ? 1 : 0));
		expect(elevationBlocksStep(ridge, 0, 0, 1, 0)).toBe(true);
		expect(elevationBlocksStep(ridge, 1, 0, 0, 0)).toBe(true);
		expect(elevationBlocksStep(ridge, 1, 0, 2, 0)).toBe(false);
	});

	it("only a single-floor stair bridges the edge", () => {
		const oneFloor = field((x) => (x >= 1 ? 1 : 0), new Set(["1,0"]));
		expect(stairBridgesStep(oneFloor, 0, 0, 1, 0)).toBe(true);
		expect(elevationBlocksStep(oneFloor, 0, 0, 1, 0)).toBe(false);

		const twoFloors = field((x) => (x >= 1 ? 2 : 0), new Set(["1,0"]));
		expect(stairBridgesStep(twoFloors, 0, 0, 1, 0)).toBe(false);
		expect(elevationBlocksStep(twoFloors, 0, 0, 1, 0)).toBe(true);
	});

	it("reports only blocked downhill ridge edges for flat rendering", () => {
		const ridge = field((x, y) => (x === 1 && y === 1 ? 2 : 1));
		expect(blockedElevationEdgeMask({ x: 1, y: 1 }, ridge)).toBe(
			ELEVATION_DIR.E | ELEVATION_DIR.W | ELEVATION_DIR.N | ELEVATION_DIR.S,
		);
		expect(blockedElevationEdgeMask({ x: 0, y: 1 }, ridge)).toBe(0);
	});

	it("reports stair hatches instead of blocked ridge masks at stair edges", () => {
		const ridge = field((x) => (x >= 1 ? 1 : 0), new Set(["1,0"]));
		expect(blockedElevationEdgeMask({ x: 1, y: 0 }, ridge)).toBe(0);
		expect(stairElevationEdgeMask({ x: 1, y: 0 }, ridge)).toBe(ELEVATION_DIR.W);
	});
});
