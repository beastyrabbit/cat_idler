import { isChoppedStumpTile } from "@/lib/game/depletion";
import {
	fencePerimeter,
	fromTiles,
	type GatePlacement,
	isInsideVillage,
	SIDE_DELTA,
	toTiles,
	type VillageArea,
} from "@/lib/game/villageArea";
import type { WorldTile } from "@/types/game";

const GRASS = "/images/iso/tiles/grass.png";
const posKey = (x: number, y: number): string => `${x},${y}`;

/**
 * Terrain sprite per tile type (Kenney Isometric Miniature series,
 * 256x512 bottom-anchored). Standalone tree sprites have no ground in the
 * source art, so they declare a grass `base` underlay.
 */
export const TILE_SPRITES: Record<
	string,
	{ src: string; filter?: string; base?: string }
> = {
	field: { src: GRASS },
	meadow: { src: "/images/iso/tiles/grass-clearing.png" },
	forest: { src: "/images/iso/tiles/tree-pine-small.png", base: GRASS },
	oak_forest: { src: "/images/iso/tiles/tree-pine-large.png", base: GRASS },
	pine_forest: { src: "/images/iso/tiles/tree-pine-huge.png", base: GRASS },
	dense_woods: {
		src: "/images/iso/tiles/tree-pine-huge.png",
		filter: "brightness(0.75)",
		base: GRASS,
	},
	jungle: {
		src: "/images/iso/tiles/tree-pine-large.png",
		filter: "saturate(1.6) hue-rotate(15deg)",
		base: GRASS,
	},
	dead_forest: { src: "/images/iso/tiles/tree-dead-large.png", base: GRASS },
	mountains: { src: "/images/iso/tiles/grass-hill-high.png" },
	swamp: {
		src: "/images/iso/tiles/grass-tree-stump.png",
		filter: "saturate(0.7) hue-rotate(30deg)",
	},
	desert: { src: "/images/iso/tiles/dirt.png" },
	tundra: { src: "/images/iso/tiles/snow.png" },
	cave_entrance: { src: "/images/iso/tiles/grass-stone-large.png" },
	enemy_territory: {
		src: "/images/iso/tiles/tree-dead-small.png",
		filter: "sepia(0.3) hue-rotate(-20deg)",
		base: GRASS,
	},
	enemy_lair: {
		src: "/images/iso/tiles/grass-stone-large.png",
		filter: "sepia(0.5) hue-rotate(-30deg) brightness(0.8)",
	},
};

/** Palisade fence pieces; the village shape decides where each segment sits. */
export const VILLAGE_RING_RADIUS = 4;
export const FENCE_X_SPRITE = "/images/iso/tiles/fence-x.png";
export const FENCE_Y_SPRITE = "/images/iso/tiles/fence-y.png";
export const GATE_SPRITE = "/images/iso/tiles/gate.png";

/** Water terrain (Isometric Nature pack, remapped to our diamond). */
export const WATER_SPRITE = "/images/iso/tiles/water.png";

/** Chopped-forest stump (drawn where a felled forest tile became field). */
export const STUMP_SPRITE = "/images/iso/tiles/stump.png";

export const ROAD_SPRITES = {
	straightX: "/images/iso/tiles/path-straight-e.png",
	straightY: "/images/iso/tiles/path-straight-n.png",
	cornerEN: "/images/iso/tiles/path-corner-e.png",
	cornerES: "/images/iso/tiles/path-corner-s.png",
	cornerWN: "/images/iso/tiles/path-corner-n.png",
	cornerWS: "/images/iso/tiles/path-corner-w.png",
	endE: "/images/iso/tiles/path-end-e.png",
	endW: "/images/iso/tiles/path-end-w.png",
	endN: "/images/iso/tiles/path-end-s.png",
	endS: "/images/iso/tiles/path-end-n.png",
	clearing: "/images/iso/tiles/path-clearing-s.png",
	crossing: "/images/iso/tiles/path-crossing.png",
} as const;

/** Worn trails render dimmer than paved roads (same oriented sprites). */
export const ROAD_WORN_FILTER = "brightness(0.82) saturate(0.85)";

/** Road-neighbour direction bits, in tile space (x east, y south). */
export const ROAD_DIR = { E: 1, W: 2, N: 4, S: 8 } as const;

/** Fog-of-war shades by distance to the nearest explored tile. */
export const FOG_SHADES = ["#33422a", "#26321f", "#1b2416", "#141c12"];

/** Deepest fog shade, used for ungenerated chunks and unknown fogged terrain. */
export const SOLID_FOG = FOG_SHADES[FOG_SHADES.length - 1];

/** Fog brightness by Chebyshev distance to the nearest explored tile. */
export const FOG_BRIGHTNESS = [0.55, 0.37, 0.24, 0.14] as const;

/** Euclidean halo of always-revealed ground beyond the square ring fence. */
export const VILLAGE_VISION_MARGIN = 1.5;

export interface FenceSprite {
	src: string;
	ox: number;
	oy: number;
}

export interface OrganicFenceSprite extends FenceSprite {
	key: string;
}

export interface OrganicVillageView {
	area: VillageArea;
	claimed: Array<{ x: number; y: number }>;
	fenceByTile: Map<string, OrganicFenceSprite[]>;
}

export interface TileGround {
	/** Main sprite URL (tree, water, grass, hill, road, etc.). */
	src: string;
	/** Optional grass underlay for standalone tree/stump sprites. */
	base?: string;
	/** CSS filter string carried for renderer-specific tinting. */
	filter?: string;
}

export type RoadKind = "built" | "worn";

export interface RoadSprite {
	src: string;
	filter?: string;
}

export function buildOrganicVillageView(
	claimedTiles: Array<{ x: number; y: number }> | undefined,
	gate: GatePlacement | null | undefined,
): OrganicVillageView | null {
	if (!claimedTiles || claimedTiles.length === 0) {
		return null;
	}
	const area = fromTiles(claimedTiles);
	const fenceByTile = new Map<string, OrganicFenceSprite[]>();
	for (const seg of fencePerimeter(area, gate)) {
		const outside = SIDE_DELTA[seg.side];
		const drawX = seg.x + outside.x;
		const drawY = seg.y + outside.y;
		const key = posKey(drawX, drawY);
		const sprites = fenceByTile.get(key) ?? [];
		sprites.push({
			key: `${seg.x},${seg.y},${seg.side}`,
			src: seg.gate
				? GATE_SPRITE
				: seg.axis === "x"
					? FENCE_X_SPRITE
					: FENCE_Y_SPRITE,
			ox: 0,
			oy: 0,
		});
		fenceByTile.set(key, sprites);
	}
	return { area, claimed: toTiles(area), fenceByTile };
}

/** Chebyshev distance from the village anchor. */
export function villageDistance(
	tile: { x: number; y: number },
	anchor: { x: number; y: number },
): number {
	return Math.max(Math.abs(tile.x - anchor.x), Math.abs(tile.y - anchor.y));
}

export function isVisibleWater(
	tile: Pick<WorldTile, "type" | "overlayFeature">,
) {
	return tile.type === "river" || tile.overlayFeature === "river";
}

function isFenceWaterGap(tile: WorldTile): boolean {
	return isVisibleWater(tile) || tile.resources.water > 0;
}

function isClaimedOrHalo(
	tile: { x: number; y: number },
	village: OrganicVillageView,
): boolean {
	if (isInsideVillage({ x: tile.x, y: tile.y }, village.area)) {
		return true;
	}
	return village.claimed.some(
		(pos) => Math.max(Math.abs(tile.x - pos.x), Math.abs(tile.y - pos.y)) <= 1,
	);
}

export function isExplored(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): boolean {
	if (tile.pathWear > 62) return true;
	if (village) return isClaimedOrHalo(tile, village);
	if (villageDistance(tile, anchor) <= ringRadius) return true;
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	return Math.sqrt(dx * dx + dy * dy) < ringRadius + VILLAGE_VISION_MARGIN;
}

/**
 * Fog brightness for unexplored tiles in a chunk, keyed by tile id.
 * Explored tiles are omitted; consumers should use brightness 1 for missing ids.
 */
export function computeFogDim(
	tiles: WorldTile[],
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): Map<string, number> {
	const explored = tiles.filter((tile) =>
		isExplored(tile, anchor, ringRadius, village),
	);
	const dims = new Map<string, number>();

	for (const tile of tiles) {
		if (isExplored(tile, anchor, ringRadius, village)) {
			continue;
		}
		if (explored.length === 0) {
			dims.set(tile._id, FOG_BRIGHTNESS[FOG_BRIGHTNESS.length - 1]);
			continue;
		}
		let nearest = Number.POSITIVE_INFINITY;
		for (const e of explored) {
			const dist = Math.max(Math.abs(tile.x - e.x), Math.abs(tile.y - e.y));
			if (dist < nearest) {
				nearest = dist;
				if (nearest === 1) {
					break;
				}
			}
		}
		const idx = Math.min(nearest - 1, FOG_BRIGHTNESS.length - 1);
		dims.set(tile._id, FOG_BRIGHTNESS[idx]);
	}

	return dims;
}

/**
 * Ground inside and on the square embankment, or inside the organic claimed
 * village, is cleared for construction unless it is water.
 */
export function isVillageClearing(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): boolean {
	if (village) {
		return (
			isInsideVillage({ x: tile.x, y: tile.y }, village.area) &&
			!isFenceWaterGap(tile)
		);
	}
	return villageDistance(tile, anchor) <= ringRadius && !isFenceWaterGap(tile);
}

/**
 * Fence sprite for a legacy village-ring tile: fences follow the edge they sit
 * on, the south side gets an open gate, water gaps stay open.
 */
export function ringSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
): FenceSprite[] {
	if (villageDistance(tile, anchor) !== ringRadius || isFenceWaterGap(tile)) {
		return [];
	}
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	if (dx === 0 && dy === ringRadius) {
		return [{ src: GATE_SPRITE, ox: 0, oy: 0 }];
	}
	const onRow = Math.abs(dy) === ringRadius;
	const onColumn = Math.abs(dx) === ringRadius;
	if (onRow && onColumn) {
		const sx = Math.sign(dx);
		const sy = Math.sign(dy);
		return [
			{ src: FENCE_X_SPRITE, ox: -sx * 64, oy: -sx * 32 },
			{ src: FENCE_Y_SPRITE, ox: sy * 64, oy: -sy * 32 },
		];
	}
	return onRow
		? [{ src: FENCE_X_SPRITE, ox: 0, oy: 0 }]
		: [{ src: FENCE_Y_SPRITE, ox: 0, oy: 0 }];
}

export function fenceSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): FenceSprite[] {
	if (village) {
		return village.fenceByTile.get(posKey(tile.x, tile.y)) ?? [];
	}
	return ringSprites(tile, anchor, ringRadius);
}

/**
 * Oriented road sprite for a tile, given which of its orthogonal neighbours are
 * also roads (a bitmask of {@link ROAD_DIR}).
 */
export function roadSpriteFor(mask: number): string {
	const e = (mask & ROAD_DIR.E) !== 0;
	const w = (mask & ROAD_DIR.W) !== 0;
	const n = (mask & ROAD_DIR.N) !== 0;
	const s = (mask & ROAD_DIR.S) !== 0;
	const horizontal = Number(e) + Number(w);
	const vertical = Number(n) + Number(s);
	const total = horizontal + vertical;

	if (total >= 3) return ROAD_SPRITES.crossing;
	if (total === 0) return ROAD_SPRITES.clearing;
	if (total === 1) {
		if (e) return ROAD_SPRITES.endE;
		if (w) return ROAD_SPRITES.endW;
		if (n) return ROAD_SPRITES.endN;
		return ROAD_SPRITES.endS;
	}
	if (e && w) return ROAD_SPRITES.straightX;
	if (n && s) return ROAD_SPRITES.straightY;
	if (e && n) return ROAD_SPRITES.cornerEN;
	if (e && s) return ROAD_SPRITES.cornerES;
	if (w && n) return ROAD_SPRITES.cornerWN;
	return ROAD_SPRITES.cornerWS;
}

/**
 * Whether a tile renders as a road and its grade. Leader-paved roads always
 * show; ordinary ground becomes a worn trail only once heavily trodden outside
 * the cleared village. Water is never a road.
 */
export function roadKind(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): RoadKind | null {
	if (isVisibleWater(tile)) return null;
	if (tile.overlayFeature === "road_built") return "built";
	if (
		tile.pathWear >= 70 &&
		!isVillageClearing(tile, anchor, ringRadius, village)
	) {
		return "worn";
	}
	return null;
}

/**
 * Oriented road sprite and worn-trail dimming for every road tile in a chunk,
 * keyed by tile id.
 */
export function computeRoadSprites(
	tiles: WorldTile[],
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): Map<string, RoadSprite> {
	const roads = new Map<string, RoadKind>();
	for (const tile of tiles) {
		const kind = roadKind(tile, anchor, ringRadius, village);
		if (kind) roads.set(posKey(tile.x, tile.y), kind);
	}

	const sprites = new Map<string, RoadSprite>();
	for (const tile of tiles) {
		const kind = roads.get(posKey(tile.x, tile.y));
		if (!kind) continue;
		let mask = 0;
		if (roads.has(posKey(tile.x + 1, tile.y))) mask |= ROAD_DIR.E;
		if (roads.has(posKey(tile.x - 1, tile.y))) mask |= ROAD_DIR.W;
		if (roads.has(posKey(tile.x, tile.y - 1))) mask |= ROAD_DIR.N;
		if (roads.has(posKey(tile.x, tile.y + 1))) mask |= ROAD_DIR.S;
		sprites.set(tile._id, {
			src: roadSpriteFor(mask),
			filter: kind === "worn" ? ROAD_WORN_FILTER : undefined,
		});
	}
	return sprites;
}

export function tileGround(
	tile: WorldTile,
	{
		anchor,
		ringRadius,
		village,
		explored,
		roadSprite,
	}: {
		anchor: { x: number; y: number };
		ringRadius: number;
		village: OrganicVillageView | null;
		explored: boolean;
		roadSprite?: RoadSprite;
	},
): TileGround | undefined {
	if (isVisibleWater(tile)) {
		return { src: WATER_SPRITE };
	}
	if (explored && roadSprite) {
		return roadSprite;
	}
	if (explored && isVillageClearing(tile, anchor, ringRadius, village)) {
		return TILE_SPRITES.field;
	}
	if (explored && isChoppedStumpTile(tile)) {
		return { src: STUMP_SPRITE, base: TILE_SPRITES.field.src };
	}
	return TILE_SPRITES[tile.type];
}
