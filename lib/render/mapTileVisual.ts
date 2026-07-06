import {
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	GATE_SPRITE,
	ROAD_DIR,
	ROAD_WORN_FILTER,
	roadSpriteFor,
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

export type { GatePlacement } from "@/lib/game/villageArea";

const GRASS = TILE_SPRITES.field.src;
const VILLAGE_VISION_MARGIN = 1.5;
const FOG_BRIGHTNESS = [0.55, 0.37, 0.24, 0.14] as const;
const posKey = (x: number, y: number): string => `${x},${y}`;

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
	src: string;
	base?: string;
	filter?: string;
}

export type RoadKind = "built" | "worn";

export interface RoadVisual {
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

export function villageDistance(
	tile: { x: number; y: number },
	anchor: { x: number; y: number },
): number {
	return Math.max(Math.abs(tile.x - anchor.x), Math.abs(tile.y - anchor.y));
}

export function hasWater(tile: WorldTile): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		tile.resources.water > 0
	);
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
	village: OrganicVillageView | null = null,
): boolean {
	if (tile.pathWear > 62) return true;
	if (village) return isClaimedOrHalo(tile, village);
	if (villageDistance(tile, anchor) <= ringRadius) return true;
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	return Math.sqrt(dx * dx + dy * dy) < ringRadius + VILLAGE_VISION_MARGIN;
}

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
			out.set(tile._id, FOG_BRIGHTNESS[FOG_BRIGHTNESS.length - 1]);
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

export function isVillageClearing(
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

export function roadKind(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null,
): RoadKind | null {
	if (tile.type === "river" || tile.overlayFeature === "river") return null;
	if (tile.overlayFeature === "road_built") return "built";
	if (
		tile.pathWear >= 70 &&
		!isVillageClearing(tile, anchor, ringRadius, village)
	) {
		return "worn";
	}
	return null;
}

export function computeRoadSprites(
	tiles: WorldTile[],
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
): Map<string, RoadVisual> {
	const roads = new Map<string, RoadKind>();
	for (const tile of tiles) {
		const kind = roadKind(tile, anchor, ringRadius, village);
		if (kind) roads.set(posKey(tile.x, tile.y), kind);
	}

	const sprites = new Map<string, RoadVisual>();
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
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
	road?: RoadVisual,
): TileGround {
	if (tile.type === "river" || tile.overlayFeature === "river") {
		return { src: WATER_SPRITE };
	}
	if (road) {
		return road;
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

export function ringSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
): FenceSprite[] {
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

export function fenceSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
	ringRadius: number,
	village: OrganicVillageView | null = null,
): FenceSprite[] {
	if (village) {
		return village.fenceByTile.get(posKey(tile.x, tile.y)) ?? [];
	}
	return ringSprites(tile, anchor, ringRadius);
}
