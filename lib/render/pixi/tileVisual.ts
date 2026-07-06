/**
 * Per-tile visual selection for the PixiJS map spike.
 *
 * A faithful, dependency-light re-implementation of the sprite/fog/fence
 * decisions in `components/map/TileLayer.tsx` (whose internals are not exported,
 * and which another agent is actively rewiring for the organic village — so we
 * deliberately do NOT import from it). Everything here is pure and reads only
 * the shared `constants.ts` art table plus the pure `isChoppedStumpTile` helper.
 *
 * Kept intentionally in sync with TileLayer; the eventual cutover would hoist
 * these pure helpers into a shared module both renderers import.
 */

import {
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	GATE_SPRITE,
	ISO,
	STUMP_SPRITE,
	TILE_SPRITES,
	WATER_SPRITE,
} from "@/components/map/constants";
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

const GRASS = TILE_SPRITES.field.src;
const posKey = (x: number, y: number): string => `${x},${y}`;

export interface OrganicFenceSprite extends FenceSprite {
	key: string;
}

export interface OrganicVillageView {
	area: VillageArea;
	claimed: Array<{ x: number; y: number }>;
	fenceByTile: Map<string, OrganicFenceSprite[]>;
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

function hasWater(tile: WorldTile): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		tile.resources.water > 0
	);
}

/** Halo of always-revealed ground beyond the fence (mirrors TileLayer). */
const VILLAGE_VISION_MARGIN = 1.5;

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
	village: OrganicVillageView | null = null,
): boolean {
	if (tile.pathWear > 62) return true;
	if (village) return isClaimedOrHalo(tile, village);
	if (villageDistance(tile, anchor) <= ringRadius) return true;
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	return Math.sqrt(dx * dx + dy * dy) < ringRadius + VILLAGE_VISION_MARGIN;
}

/** Fog brightness by Chebyshev distance to the nearest explored tile. */
const FOG_BRIGHTNESS = [0.55, 0.37, 0.24, 0.14];
const SOLID_FOG_BRIGHTNESS = FOG_BRIGHTNESS[FOG_BRIGHTNESS.length - 1];

/**
 * Fog brightness (1 = fully lit) for every tile in a chunk, keyed by tile id.
 * Explored tiles map to 1; unexplored tiles darken by distance to the nearest
 * explored tile within the chunk — the "land, unlit" silhouette look.
 */
export function computeFogBrightness(
	tiles: WorldTile[],
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
): Map<string, number> {
	const explored = tiles.filter((t) =>
		isExplored(t, anchor, ringRadius, village),
	);
	const out = new Map<string, number>();
	for (const tile of tiles) {
		if (isExplored(tile, anchor, ringRadius, village)) {
			out.set(tile._id, 1);
			continue;
		}
		if (explored.length === 0) {
			out.set(tile._id, SOLID_FOG_BRIGHTNESS);
			continue;
		}
		let nearest = Number.POSITIVE_INFINITY;
		for (const e of explored) {
			const d = Math.max(Math.abs(tile.x - e.x), Math.abs(tile.y - e.y));
			if (d < nearest) {
				nearest = d;
				if (nearest === 1) break;
			}
		}
		const idx = Math.min(nearest - 1, FOG_BRIGHTNESS.length - 1);
		out.set(tile._id, FOG_BRIGHTNESS[idx]);
	}
	return out;
}

function isVillageClearing(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): boolean {
	if (village) {
		return (
			isInsideVillage({ x: tile.x, y: tile.y }, village.area) && !hasWater(tile)
		);
	}
	return villageDistance(tile, anchor) <= ringRadius && !hasWater(tile);
}

export interface TileGround {
	/** Main sprite URL (tree, water, grass, hill…). */
	src: string;
	/** Optional grass underlay for standalone tree/stump sprites. */
	base?: string;
	/** CSS filter string carried for brightness tinting. */
	filter?: string;
}

/**
 * Ground sprite(s) for a tile, mirroring TileLayer's precedence: water wins;
 * then the cleared village grass; then a felled-forest stump; else the biome
 * sprite. Roads are intentionally omitted in the spike (autotiling is cosmetic
 * and out of the terrain/fence/cat fidelity bar).
 */
export function tileGround(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
): TileGround {
	if (tile.type === "river" || tile.overlayFeature === "river") {
		return { src: WATER_SPRITE };
	}
	if (isVillageClearing(tile, anchor, ringRadius, village)) {
		return { src: GRASS };
	}
	if (isChoppedStumpTile(tile)) {
		return { src: STUMP_SPRITE, base: GRASS };
	}
	const entry = TILE_SPRITES[tile.type];
	return entry ?? { src: GRASS };
}

export interface FenceSprite {
	src: string;
	ox: number;
	oy: number;
}

/**
 * Fence sprite(s) for a village-ring tile — a re-implementation of TileLayer's
 * `ringSprites`: edges follow the side they sit on, the south tile is a gate,
 * water gaps stay open, corners seat two half-shifted rails as an L.
 */
export function fenceSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
): FenceSprite[] {
	if (village) {
		return village.fenceByTile.get(posKey(tile.x, tile.y)) ?? [];
	}
	if (villageDistance(tile, anchor) !== ringRadius || hasWater(tile)) {
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

/** Sprite draw box: full 256x512 canvas at (left, top - surfaceOffset). */
export const SPRITE_W = ISO.tileWidth;
export const SPRITE_H = ISO.imageHeight;
export const SPRITE_TOP_OFFSET = ISO.surfaceOffset;
