/**
 * Terrain-driven world tiles.
 *
 * Bridges the pure Isometric-Nature terrain generator (`terrainGen.ts`, which
 * emits abstract height/biome/river roles) onto the gameplay `WorldTile`
 * vocabulary the simulation reads (biome type, food/herb/water resources,
 * danger, path wear). Using the *same* terrain field for both means the map a
 * player sees and the map cats walk are one and the same — a forest tile the
 * renderer draws with trees actually carries the forest's food/herbs, a river
 * the renderer draws as water actually blocks pathing and quenches thirst.
 *
 * Pure and deterministic: identical `(chunkX, chunkY, seed, colony)` always
 * yields identical tiles, so a chunk regenerates the same on the client, the
 * server, and across restarts.
 */

import type { WorldTile } from "@/types/game";
import {
	BIOME_PROPERTIES,
	type BiomeType,
	calculateDangerLevel,
} from "./biomes";
import { createSeededRandom, hashSeed } from "./noise";
import {
	type BiomeRole,
	generateTerrainChunk,
	type TerrainTile,
	WORLD_TERRAIN_OPTIONS,
} from "./terrainGen";
import { COLONY_SAFE_RADIUS, COLONY_WATER_RADIUS } from "./worldGen";

export type WorldTileData = Omit<WorldTile, "_id" | "colonyId">;

/**
 * Gameplay biome each terrain role borrows its resource/danger table from.
 * Lowland/grassland read as gentle meadow; forest as oak forest (food + herbs);
 * rocky/highland as mountains (sparse, dangerous). Keeping the mapping here (not
 * in `terrainGen`) leaves the terrain layer purely cosmetic over the heightmap.
 */
const BIOME_ROLE_TO_TYPE: Record<BiomeRole, BiomeType> = {
	lowland: "meadow",
	grassland: "meadow",
	forest: "oak_forest",
	rocky: "mountains",
	highland: "mountains",
};

/** Human/gameplay tile type stored on the row (drives titles + the river checks). */
const BIOME_ROLE_TO_TILE_TYPE: Record<BiomeRole, WorldTile["type"]> = {
	lowland: "meadow",
	grassland: "field",
	forest: "forest",
	rocky: "mountains",
	highland: "mountains",
};

function distanceTo(x: number, y: number, cx: number, cy: number): number {
	return Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);
}

/** Map one terrain tile to its gameplay world tile. */
function terrainToWorldTile(
	tile: TerrainTile,
	seed: number,
	colonyX: number,
	colonyY: number,
): WorldTileData {
	const dist = distanceTo(tile.x, tile.y, colonyX, colonyY);

	// Rivers: infinite water, no forage, low danger — same contract worldGen used.
	if (tile.river) {
		return {
			x: tile.x,
			y: tile.y,
			type: "river",
			resources: { food: 0, herbs: 0, water: 999 },
			maxResources: { food: 0, herbs: 0 },
			dangerLevel: 5,
			pathWear: 0,
			lastDepleted: 0,
			overlayFeature: "river",
		};
	}

	const biomeType = BIOME_ROLE_TO_TYPE[tile.biome];
	const props = BIOME_PROPERTIES[biomeType];
	const rng = createSeededRandom(hashSeed(seed, tile.x, tile.y));

	return {
		x: tile.x,
		y: tile.y,
		type: BIOME_ROLE_TO_TILE_TYPE[tile.biome],
		resources: {
			food: rng.int(props.baseResources.food.min, props.baseResources.food.max),
			herbs: rng.int(
				props.baseResources.herbs.min,
				props.baseResources.herbs.max,
			),
			water: props.baseResources.water,
		},
		maxResources: {
			food: props.maxResources.food,
			herbs: props.maxResources.herbs,
		},
		dangerLevel: calculateDangerLevel(biomeType, null, dist),
		pathWear: 0,
		lastDepleted: 0,
		overlayFeature: null,
	};
}

/**
 * The colony must be able to reach water: the plateau keeps rivers out of the
 * village core, so an unlucky seed could otherwise strand the cats. If no water
 * exists within `COLONY_WATER_RADIUS`, force one nearby tile (just outside the
 * safe ring) into a pond. Ported from `worldGen.ensureWaterNearColony`.
 */
function ensureWaterNearColony(
	tiles: WorldTileData[],
	seed: number,
	colonyX: number,
	colonyY: number,
): void {
	const near = (t: { x: number; y: number }) =>
		distanceTo(t.x, t.y, colonyX, colonyY);

	const hasWater = tiles.some(
		(t) => t.resources.water > 0 && near(t) <= COLONY_WATER_RADIUS,
	);
	if (hasWater) {
		return;
	}

	const candidates = tiles.filter((t) => {
		const d = near(t);
		return d > COLONY_SAFE_RADIUS && d <= COLONY_WATER_RADIUS;
	});
	if (candidates.length === 0) {
		return;
	}

	const rng = createSeededRandom(hashSeed(seed, "starter_pond"));
	const pond = candidates[Math.floor(rng.next() * candidates.length)];
	pond.type = "river";
	pond.overlayFeature = "river";
	pond.resources = { food: 0, herbs: 0, water: 999 };
	pond.maxResources = { food: 0, herbs: 0 };
	pond.dangerLevel = 5;
}

/**
 * Generate a 12x12 chunk of gameplay world tiles from the shared terrain field.
 * Drop-in replacement for `worldGen.generateChunk`, sampled from the same
 * heightmap the renderer draws.
 */
export function generateWorldChunk(
	chunkX: number,
	chunkY: number,
	seed: number,
	colonyX: number,
	colonyY: number,
): WorldTileData[] {
	const terrain = generateTerrainChunk(
		chunkX,
		chunkY,
		seed,
		WORLD_TERRAIN_OPTIONS,
	);
	const tiles = terrain.map((t) =>
		terrainToWorldTile(t, seed, colonyX, colonyY),
	);

	const minX = chunkX * 12;
	const minY = chunkY * 12;
	const containsColony =
		colonyX >= minX &&
		colonyX < minX + 12 &&
		colonyY >= minY &&
		colonyY < minY + 12;
	if (containsColony) {
		ensureWaterNearColony(tiles, seed, colonyX, colonyY);
	}

	return tiles;
}
