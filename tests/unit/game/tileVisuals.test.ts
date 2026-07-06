import { describe, expect, it } from "vitest";
import type { GatePlacement } from "@/lib/game/villageArea";
import {
	buildOrganicVillageView,
	computeFogDim,
	computeRoadSprites,
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	FOG_BRIGHTNESS,
	GATE_SPRITE,
	isExplored,
	ROAD_DIR,
	ROAD_SPRITES,
	ROAD_WORN_FILTER,
	roadKind,
	roadSpriteFor,
	ringSprites,
	STUMP_SPRITE,
	TILE_SPRITES,
	tileGround,
	WATER_SPRITE,
} from "@/lib/game/tileVisuals";
import type { OverlayFeature, TileType, WorldTile } from "@/types/game";

const ANCHOR = { x: 100, y: 100 };

function tile(overrides: Partial<WorldTile> = {}): WorldTile {
	const x = overrides.x ?? 0;
	const y = overrides.y ?? 0;
	return {
		_id: `${x},${y}`,
		colonyId: "colony",
		x,
		y,
		type: "field",
		resources: { food: 0, herbs: 0, water: 0 },
		maxResources: { food: 40, herbs: 20 },
		dangerLevel: 0,
		pathWear: 0,
		lastDepleted: 0,
		overlayFeature: null,
		...overrides,
	} as WorldTile;
}

function ground(
	t: WorldTile,
	options: {
		explored?: boolean;
		roadSprite?: { src: string; filter?: string };
	} = {},
) {
	return tileGround(t, {
		anchor: ANCHOR,
		ringRadius: 0,
		village: null,
		explored: options.explored ?? true,
		roadSprite: options.roadSprite,
	});
}

describe("tileVisuals sprite selection", () => {
	it("maps every configured terrain type to its shared sprite entry", () => {
		for (const [type, entry] of Object.entries(TILE_SPRITES)) {
			expect(ground(tile({ type: type as TileType }))).toEqual(entry);
		}
	});

	it("uses the water sprite for river terrain or a river overlay", () => {
		expect(ground(tile({ type: "river" }))).toEqual({ src: WATER_SPRITE });
		expect(ground(tile({ overlayFeature: "river" }))).toEqual({
			src: WATER_SPRITE,
		});
	});

	it("keeps grass under standalone tree sprites", () => {
		expect(ground(tile({ type: "forest" }))).toEqual({
			src: TILE_SPRITES.forest.src,
			base: TILE_SPRITES.field.src,
		});
	});

	it("renders chopped field tiles as stumps only after exploration", () => {
		const stump = tile({
			type: "field",
			lastDepleted: 1,
			maxResources: { food: 5, herbs: 0 },
		});

		expect(ground(stump)).toEqual({
			src: STUMP_SPRITE,
			base: TILE_SPRITES.field.src,
		});
		expect(ground(stump, { explored: false })).toEqual(TILE_SPRITES.field);
	});

	it("lets explored road sprites win over village clearing grass", () => {
		const road = { src: ROAD_SPRITES.crossing };
		const insideClearing = tile({ x: ANCHOR.x, y: ANCHOR.y });

		expect(ground(insideClearing, { roadSprite: road })).toEqual(road);
		expect(
			ground(insideClearing, { explored: false, roadSprite: road }),
		).toEqual(TILE_SPRITES.field);
	});
});

describe("tileVisuals fog", () => {
	it("uses strict path-wear exploration threshold", () => {
		expect(
			isExplored(tile({ x: 0, y: 0, pathWear: 62 }), ANCHOR, 0, null),
		).toBe(false);
		expect(
			isExplored(tile({ x: 0, y: 0, pathWear: 63 }), ANCHOR, 0, null),
		).toBe(true);
	});

	it("reveals the square ring and Euclidean margin for legacy villages", () => {
		const anchor = { x: 0, y: 0 };

		expect(isExplored(tile({ x: 4, y: 4 }), anchor, 4, null)).toBe(true);
		expect(isExplored(tile({ x: 5, y: 0 }), anchor, 4, null)).toBe(true);
		expect(isExplored(tile({ x: 6, y: 0 }), anchor, 4, null)).toBe(false);
	});

	it("reveals organic claimed tiles plus a one-tile Chebyshev halo", () => {
		const village = buildOrganicVillageView([{ x: 0, y: 0 }], null);

		expect(isExplored(tile({ x: 0, y: 0 }), ANCHOR, 0, village)).toBe(true);
		expect(isExplored(tile({ x: 1, y: 1 }), ANCHOR, 0, village)).toBe(true);
		expect(isExplored(tile({ x: 2, y: 0 }), ANCHOR, 0, village)).toBe(false);
	});

	it("dims unexplored tiles by Chebyshev distance to explored ground", () => {
		const tiles = [
			tile({ x: 0, y: 0, pathWear: 63 }),
			tile({ x: 1, y: 0 }),
			tile({ x: 2, y: 0 }),
			tile({ x: 3, y: 0 }),
			tile({ x: 4, y: 0 }),
			tile({ x: 5, y: 0 }),
		];

		const dims = computeFogDim(tiles, ANCHOR, 0, null);

		expect(dims.has("0,0")).toBe(false);
		expect(dims.get("1,0")).toBe(FOG_BRIGHTNESS[0]);
		expect(dims.get("2,0")).toBe(FOG_BRIGHTNESS[1]);
		expect(dims.get("3,0")).toBe(FOG_BRIGHTNESS[2]);
		expect(dims.get("4,0")).toBe(FOG_BRIGHTNESS[3]);
		expect(dims.get("5,0")).toBe(FOG_BRIGHTNESS[3]);
	});

	it("uses deepest dim when a chunk has no explored tile", () => {
		const dims = computeFogDim([tile({ x: 0, y: 0 })], ANCHOR, 0, null);

		expect(dims.get("0,0")).toBe(FOG_BRIGHTNESS[FOG_BRIGHTNESS.length - 1]);
	});
});

describe("tileVisuals fences", () => {
	it("maps legacy ring sides, gate, corners, and water gaps", () => {
		const anchor = { x: 0, y: 0 };

		expect(ringSprites(tile({ x: 0, y: 2 }), anchor, 2)).toEqual([
			{ src: GATE_SPRITE, ox: 0, oy: 0 },
		]);
		expect(ringSprites(tile({ x: 0, y: -2 }), anchor, 2)).toEqual([
			{ src: FENCE_X_SPRITE, ox: 0, oy: 0 },
		]);
		expect(ringSprites(tile({ x: 2, y: 0 }), anchor, 2)).toEqual([
			{ src: FENCE_Y_SPRITE, ox: 0, oy: 0 },
		]);
		expect(ringSprites(tile({ x: 2, y: -2 }), anchor, 2)).toEqual([
			{ src: FENCE_X_SPRITE, ox: -64, oy: -32 },
			{ src: FENCE_Y_SPRITE, ox: -64, oy: 32 },
		]);
		expect(
			ringSprites(
				tile({ x: 0, y: -2, resources: { food: 0, herbs: 0, water: 1 } }),
				anchor,
				2,
			),
		).toEqual([]);
		expect(
			ringSprites(tile({ x: 0, y: -2, overlayFeature: "river" }), anchor, 2),
		).toEqual([]);
	});

	it("maps organic fence segments to outside tiles with centered offsets", () => {
		const gate: GatePlacement = { x: 0, y: 0, side: "N" };
		const village = buildOrganicVillageView([{ x: 0, y: 0 }], gate);

		expect(village?.fenceByTile.get("0,-1")).toEqual([
			{ key: "0,0,N", src: GATE_SPRITE, ox: 0, oy: 0 },
		]);
		expect(village?.fenceByTile.get("1,0")).toEqual([
			{ key: "0,0,E", src: FENCE_Y_SPRITE, ox: 0, oy: 0 },
		]);
		expect(village?.fenceByTile.get("0,1")).toEqual([
			{ key: "0,0,S", src: FENCE_X_SPRITE, ox: 0, oy: 0 },
		]);
		expect(village?.fenceByTile.get("-1,0")).toEqual([
			{ key: "0,0,W", src: FENCE_Y_SPRITE, ox: 0, oy: 0 },
		]);
	});
});

describe("tileVisuals roads", () => {
	it("covers the full road mask table", () => {
		const { E, W, N, S } = ROAD_DIR;
		const expected: Record<number, string> = {
			0: ROAD_SPRITES.clearing,
			[E]: ROAD_SPRITES.endE,
			[W]: ROAD_SPRITES.endW,
			[N]: ROAD_SPRITES.endN,
			[S]: ROAD_SPRITES.endS,
			[E | W]: ROAD_SPRITES.straightX,
			[N | S]: ROAD_SPRITES.straightY,
			[E | N]: ROAD_SPRITES.cornerEN,
			[E | S]: ROAD_SPRITES.cornerES,
			[W | N]: ROAD_SPRITES.cornerWN,
			[W | S]: ROAD_SPRITES.cornerWS,
			[E | W | N]: ROAD_SPRITES.crossing,
			[E | W | S]: ROAD_SPRITES.crossing,
			[E | N | S]: ROAD_SPRITES.crossing,
			[W | N | S]: ROAD_SPRITES.crossing,
			[E | W | N | S]: ROAD_SPRITES.crossing,
		};

		for (let mask = 0; mask <= 15; mask++) {
			expect(roadSpriteFor(mask)).toBe(expected[mask]);
		}
	});

	it("classifies built, worn, below-threshold, clearing, and water roads", () => {
		expect(
			roadKind(tile({ overlayFeature: "road_built" }), ANCHOR, 0, null),
		).toBe("built");
		expect(roadKind(tile({ pathWear: 69 }), ANCHOR, 0, null)).toBeNull();
		expect(roadKind(tile({ pathWear: 70 }), ANCHOR, 0, null)).toBe("worn");
		expect(
			roadKind(
				tile({ x: ANCHOR.x, y: ANCHOR.y, pathWear: 100 }),
				ANCHOR,
				0,
				null,
			),
		).toBeNull();
		expect(
			roadKind(
				tile({ overlayFeature: "road_built" as OverlayFeature, type: "river" }),
				ANCHOR,
				0,
				null,
			),
		).toBeNull();
	});

	it("builds per-chunk road masks from orthogonal road neighbours", () => {
		const tiles = [
			tile({ x: 0, y: 0, overlayFeature: "road_built" }),
			tile({ x: 1, y: 0, overlayFeature: "road_built" }),
			tile({ x: -1, y: 0, overlayFeature: "road_built" }),
			tile({ x: 0, y: -1, pathWear: 70 }),
			tile({ x: 0, y: 1, pathWear: 70 }),
			tile({ x: 2, y: 0, pathWear: 70 }),
		];

		const sprites = computeRoadSprites(tiles, ANCHOR, 0, null);

		expect(sprites.get("0,0")).toEqual({ src: ROAD_SPRITES.crossing });
		expect(sprites.get("0,-1")).toEqual({
			src: ROAD_SPRITES.endS,
			filter: ROAD_WORN_FILTER,
		});
		expect(sprites.get("2,0")).toEqual({
			src: ROAD_SPRITES.endW,
			filter: ROAD_WORN_FILTER,
		});
	});
});
