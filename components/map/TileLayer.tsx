"use client";

import { memo, useEffect, useMemo, useState } from "react";
import {
	elevationOffset,
	tileToIso,
	zIndexFor,
} from "@/lib/game/isoProjection";
import type { ChunkCoord } from "@/lib/game/mapView";
import { chunkKey } from "@/lib/game/mapView";
import {
	type Direction,
	generateTerrainChunk,
	type TerrainTile,
	WORLD_TERRAIN_OPTIONS,
} from "@/lib/game/terrainGen";
import type { WorldTile } from "@/types/game";
import {
	ACTOR,
	BUILT_ROAD_FILL,
	DIAMOND_CLIP,
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	FOG_OPACITIES,
	FOG_SHADES,
	GATE_SPRITE,
	ISO,
	ROAD_FILL,
	VILLAGE_RING_RADIUS,
} from "./constants";
import {
	cliffSprite,
	groundSprite,
	type NatureSprite,
	natureSpriteUrl,
	riverSprite,
	rockSprite,
	stairsSprite,
	treeSprite,
} from "./natureMapping";

interface TileLayerProps {
	chunks: ChunkCoord[];
	anchor: { x: number; y: number };
	/** World seed — the client regenerates terrain from it (matches the server). */
	seed: number | null;
	/** Fence/clearing ring radius (grows as the village fills). */
	ringRadius?: number;
	/** Info mode: draw resource markers on rich tiles. */
	showInfo?: boolean;
}

// Terrain is derived client-side from the seed; only the gameplay overlay
// (pathWear -> fog reveal + roads, rich-tile markers) comes from the DB, so we
// still fetch chunks and refresh them once a minute.
const CHUNK_TTL_MS = 60_000;
const chunkCache = new Map<string, { tiles: WorldTile[]; fetchedAt: number }>();

function useChunkTiles(chunkX: number, chunkY: number): WorldTile[] | null {
	const key = `${chunkX},${chunkY}`;
	const [tiles, setTiles] = useState<WorldTile[] | null>(
		() => chunkCache.get(key)?.tiles ?? null,
	);

	useEffect(() => {
		let cancelled = false;

		const load = () => {
			fetch(`/api/game/chunks?x=${chunkX}&y=${chunkY}`)
				.then((response) => (response.ok ? response.json() : null))
				.then((data) => {
					const fetched = (data?.tiles ?? []) as WorldTile[];
					chunkCache.set(key, { tiles: fetched, fetchedAt: Date.now() });
					if (!cancelled) {
						setTiles(fetched);
					}
				})
				.catch((err) => {
					console.warn(`chunk (${chunkX}, ${chunkY}) fetch failed:`, err);
				});
		};

		const cached = chunkCache.get(key);
		if (!cached || Date.now() - cached.fetchedAt > CHUNK_TTL_MS) {
			load();
		}
		const interval = setInterval(load, CHUNK_TTL_MS);

		return () => {
			cancelled = true;
			clearInterval(interval);
		};
	}, [key, chunkX, chunkY]);

	return tiles;
}

/**
 * Tiles within this distance of the village are always revealed — the colony
 * knows the land immediately around it. Sized to show the terrain ring (cliffs,
 * nearby rivers, forest) just outside the flat village plateau.
 */
const VILLAGE_VISION_RADIUS = 12;

/** Resource markers only appear on notably rich tiles (biome max ~60/25). */
const RICH_FOOD = 35;
const RICH_HERBS = 12;

const tileKey = (x: number, y: number): string => `${x},${y}`;

/** Chebyshev distance from the village anchor. */
function villageDistance(
	x: number,
	y: number,
	anchor: { x: number; y: number },
): number {
	return Math.max(Math.abs(x - anchor.x), Math.abs(y - anchor.y));
}

/**
 * Whether a tile is revealed. Reveal is driven by the village vision halo (known
 * without DB data) plus trodden path wear (from the DB overlay, absent until the
 * chunk loads). A tile carrying drawable water inside the clearing still counts
 * as village ground.
 */
function isExplored(
	x: number,
	y: number,
	anchor: { x: number; y: number },
	ringRadius: number,
	worldTile: WorldTile | undefined,
): boolean {
	if ((worldTile?.pathWear ?? 0) > 62) return true;
	if (villageDistance(x, y, anchor) < ringRadius) return true;
	const dx = x - anchor.x;
	const dy = y - anchor.y;
	return Math.sqrt(dx * dx + dy * dy) < VILLAGE_VISION_RADIUS;
}

/** Deepest fog opacity — used for far tiles and ungenerated chunks. */
const SOLID_FOG_OPACITY = FOG_OPACITIES[FOG_OPACITIES.length - 1];
const FOG_COLOR = FOG_SHADES[FOG_SHADES.length - 1];

/**
 * Fog opacity for every unexplored terrain tile in a chunk, keyed by "x,y".
 * Shaded by Chebyshev distance to the nearest explored tile within this chunk,
 * so fog fades from a light frontier haze into near-solid unknown. Explored
 * tiles are omitted (they render clear terrain). Cross-chunk neighbors are not
 * consulted, so seams are approximate — fine for a soft haze.
 */
function computeFogOpacities(
	terrain: TerrainTile[],
	anchor: { x: number; y: number },
	ringRadius: number,
	worldByKey: Map<string, WorldTile>,
): Map<string, number> {
	const opacities = new Map<string, number>();
	const explored = terrain.filter((t) =>
		isExplored(t.x, t.y, anchor, ringRadius, worldByKey.get(tileKey(t.x, t.y))),
	);

	for (const t of terrain) {
		const wt = worldByKey.get(tileKey(t.x, t.y));
		if (isExplored(t.x, t.y, anchor, ringRadius, wt)) {
			continue;
		}
		if (explored.length === 0) {
			opacities.set(tileKey(t.x, t.y), SOLID_FOG_OPACITY);
			continue;
		}
		let nearest = Number.POSITIVE_INFINITY;
		for (const e of explored) {
			const dist = Math.max(Math.abs(t.x - e.x), Math.abs(t.y - e.y));
			if (dist < nearest) {
				nearest = dist;
				if (nearest === 1) break;
			}
		}
		const idx = Math.min(nearest - 1, FOG_OPACITIES.length - 1);
		opacities.set(tileKey(t.x, t.y), FOG_OPACITIES[idx]);
	}

	return opacities;
}

function hasWater(worldTile: WorldTile | undefined): boolean {
	return (
		worldTile?.type === "river" ||
		worldTile?.overlayFeature === "river" ||
		(worldTile?.resources?.water ?? 0) > 0
	);
}

/** Ground inside the fence is cleared for construction — always flat grass. */
function isVillageClearing(
	x: number,
	y: number,
	anchor: { x: number; y: number },
	ringRadius: number,
	worldTile: WorldTile | undefined,
): boolean {
	return villageDistance(x, y, anchor) < ringRadius && !hasWater(worldTile);
}

/**
 * Fence sprite for a village-ring tile: fences follow the edge they sit on, the
 * south side gets an open gate, water gaps stay open. Offsets are in Nature
 * ground-diamond units (half tile = tileWidth/2, quarter height = tileHeight/2).
 */
function ringSprites(
	x: number,
	y: number,
	anchor: { x: number; y: number },
	ringRadius: number,
	worldTile: WorldTile | undefined,
): Array<{ src: string; ox: number; oy: number }> {
	if (villageDistance(x, y, anchor) !== ringRadius || hasWater(worldTile)) {
		return [];
	}
	const dx = x - anchor.x;
	const dy = y - anchor.y;
	if (dx === 0 && dy === ringRadius) {
		return [{ src: GATE_SPRITE, ox: 0, oy: 0 }];
	}
	const onRow = Math.abs(dy) === ringRadius;
	const onColumn = Math.abs(dx) === ringRadius;
	const halfW = ISO.tileWidth / 2;
	const quarterH = ISO.tileHeight / 2;
	if (onRow && onColumn) {
		const sx = Math.sign(dx);
		const sy = Math.sign(dy);
		return [
			{ src: FENCE_X_SPRITE, ox: -sx * halfW, oy: -sx * quarterH },
			{ src: FENCE_Y_SPRITE, ox: sy * halfW, oy: -sy * quarterH },
		];
	}
	return onRow
		? [{ src: FENCE_X_SPRITE, ox: 0, oy: 0 }]
		: [{ src: FENCE_Y_SPRITE, ox: 0, oy: 0 }];
}

/** A Nature terrain sprite drawn on a tile, raised by its floor height. */
function NatureImg({
	sprite,
	left,
	top,
	height,
	z,
	title,
}: {
	sprite: NatureSprite;
	left: number;
	top: number;
	height: number;
	z: number;
	title?: string;
}) {
	return (
		<img
			src={natureSpriteUrl(sprite)}
			alt=""
			title={title}
			draggable={false}
			className="pointer-events-none absolute select-none"
			style={{
				left,
				top: top - ISO.surfaceOffset - elevationOffset(height),
				width: ISO.tileWidth,
				height: ISO.imageHeight,
				zIndex: z,
			}}
		/>
	);
}

/** Miniature actor sprite (fence/gate) re-scaled onto the Nature diamond. */
function ActorImg({
	src,
	left,
	top,
	height,
	z,
	ox = 0,
	oy = 0,
}: {
	src: string;
	left: number;
	top: number;
	height: number;
	z: number;
	ox?: number;
	oy?: number;
}) {
	return (
		<img
			src={src}
			alt=""
			draggable={false}
			className="pointer-events-none absolute select-none"
			style={{
				left: left + ox,
				top: top - ACTOR.surfaceOffset - elevationOffset(height) + oy,
				width: ACTOR.width,
				height: ACTOR.height,
				zIndex: z,
			}}
		/>
	);
}

/** Which oriented river sprite a tile shows (terrain segment, else a pond). */
function riverSpriteFor(terrain: TerrainTile): NatureSprite {
	if (terrain.river) {
		return riverSprite(terrain.river.segment, terrain.river.facing);
	}
	// A gameplay-forced pond (starter water) with no terrain river role.
	return riverSprite("start", "N" as Direction);
}

const IsoTile = memo(function IsoTile({
	terrain,
	worldTile,
	anchor,
	ringRadius,
	showInfo,
	fogOpacity,
}: {
	terrain: TerrainTile;
	worldTile: WorldTile | undefined;
	anchor: { x: number; y: number };
	ringRadius: number;
	showInfo: boolean;
	/** Fog opacity for an unexplored tile, or 0 when explored. */
	fogOpacity: number;
}) {
	const { x, y, height } = terrain;
	const { left, top } = tileToIso(x, y, ISO);
	const tileZ = zIndexFor(x, y, "tile", ISO, height);
	const objectZ = zIndexFor(x, y, "object", ISO, height);
	const clearing = isVillageClearing(x, y, anchor, ringRadius, worldTile);
	const water = hasWater(worldTile) || Boolean(terrain.river);

	// Base ground/cliff/river sprite.
	let base: NatureSprite;
	if (water) {
		base = riverSpriteFor(terrain);
	} else if (!clearing && terrain.terrain.kind === "cliff") {
		base = cliffSprite(
			terrain.terrain.base,
			terrain.terrain.variant,
			terrain.terrain.facing,
		);
	} else {
		base = groundSprite(clearing ? "grassland" : terrain.biome);
	}

	const surfaceTop = top - ISO.surfaceOffset - elevationOffset(height);
	const isBuiltRoad = worldTile?.overlayFeature === "road_built";
	const isWornRoad = !water && !clearing && (worldTile?.pathWear ?? 0) >= 70;
	const title = `${(worldTile?.type ?? terrain.biome).replaceAll("_", " ")} (${x}, ${y})`;

	return (
		<>
			<NatureImg
				sprite={base}
				left={left}
				top={top}
				height={height}
				z={tileZ}
				title={title}
			/>

			{/* Worn trails / paved roads: a translucent diamond over the surface. */}
			{(isBuiltRoad || isWornRoad) && (
				<div
					className="pointer-events-none absolute"
					style={{
						left,
						top: top - elevationOffset(height),
						width: ISO.tileWidth,
						height: ISO.tileHeight,
						zIndex: tileZ + 1,
						clipPath: DIAMOND_CLIP,
						background: isBuiltRoad ? BUILT_ROAD_FILL : ROAD_FILL,
					}}
				/>
			)}

			{/* Staircase linking this cliff to the level below. */}
			{!clearing && terrain.stairs && (
				<NatureImg
					sprite={stairsSprite(terrain.stairs.facing)}
					left={left}
					top={top}
					height={height}
					z={objectZ}
				/>
			)}

			{/* Scattered tree/rock decoration (never inside the cleared village). */}
			{!clearing && !water && terrain.decoration && (
				<NatureImg
					sprite={
						terrain.decoration.kind === "tree"
							? treeSprite(terrain.decoration.species)
							: rockSprite(terrain.decoration.size)
					}
					left={left}
					top={top}
					height={height}
					z={objectZ}
				/>
			)}

			{/* Fence ring (with a south gate) around the founding village. */}
			{ringSprites(x, y, anchor, ringRadius, worldTile).map((fence) => (
				<ActorImg
					key={fence.src}
					src={fence.src}
					left={left}
					top={top}
					height={height}
					z={objectZ}
					ox={fence.ox}
					oy={fence.oy}
				/>
			))}

			{/* Only notably rich tiles get a marker — keeps the map readable. */}
			{showInfo &&
				worldTile &&
				fogOpacity === 0 &&
				(worldTile.resources.food >= RICH_FOOD ||
					worldTile.resources.herbs >= RICH_HERBS) && (
					<div
						className="pointer-events-none absolute flex gap-1 text-base leading-none drop-shadow"
						style={{
							left: left + ISO.tileWidth / 2 - 16,
							top: top + ISO.tileHeight / 2 - 8 - elevationOffset(height),
							zIndex: objectZ,
						}}
					>
						{worldTile.resources.food >= RICH_FOOD && <span>🍖</span>}
						{worldTile.resources.herbs >= RICH_HERBS && <span>🌿</span>}
					</div>
				)}

			{/* Fog: a translucent haze over unexplored terrain (dim silhouette). */}
			{fogOpacity > 0 && (
				<div
					className="pointer-events-none absolute"
					style={{
						left,
						top: surfaceTop,
						width: ISO.tileWidth,
						height: ISO.imageHeight,
						zIndex: objectZ,
						maskImage: `url(${natureSpriteUrl(base)})`,
						WebkitMaskImage: `url(${natureSpriteUrl(base)})`,
						maskSize: "100% 100%",
						WebkitMaskSize: "100% 100%",
						background: FOG_COLOR,
						opacity: fogOpacity,
					}}
				/>
			)}
		</>
	);
});

const ChunkView = memo(function ChunkView({
	chunkX,
	chunkY,
	anchor,
	ringRadius,
	showInfo,
	seed,
}: {
	chunkX: number;
	chunkY: number;
	anchor: { x: number; y: number };
	ringRadius: number;
	showInfo: boolean;
	seed: number | null;
}) {
	const worldTiles = useChunkTiles(chunkX, chunkY);
	const terrain = useMemo(
		() =>
			seed === null
				? []
				: generateTerrainChunk(chunkX, chunkY, seed, WORLD_TERRAIN_OPTIONS),
		[seed, chunkX, chunkY],
	);
	const worldByKey = useMemo(() => {
		const map = new Map<string, WorldTile>();
		for (const tile of worldTiles ?? []) {
			map.set(tileKey(tile.x, tile.y), tile);
		}
		return map;
	}, [worldTiles]);
	const fogOpacities = useMemo(
		() => computeFogOpacities(terrain, anchor, ringRadius, worldByKey),
		[terrain, anchor, ringRadius, worldByKey],
	);

	if (terrain.length === 0) {
		// Seed not loaded yet, or an ungenerated (out-of-window) chunk — solid fog.
		const fog = [];
		for (let ty = 0; ty < 12; ty++) {
			for (let tx = 0; tx < 12; tx++) {
				const wx = chunkX * 12 + tx;
				const wy = chunkY * 12 + ty;
				const { left, top } = tileToIso(wx, wy, ISO);
				fog.push(
					<div
						key={`${wx},${wy}`}
						className="absolute"
						style={{
							left,
							top,
							width: ISO.tileWidth,
							height: ISO.tileHeight,
							zIndex: zIndexFor(wx, wy, "tile", ISO),
							clipPath: DIAMOND_CLIP,
							background: FOG_COLOR,
						}}
					/>,
				);
			}
		}
		return <>{fog}</>;
	}

	return (
		<>
			{terrain.map((tile) => (
				<IsoTile
					key={tileKey(tile.x, tile.y)}
					terrain={tile}
					worldTile={worldByKey.get(tileKey(tile.x, tile.y))}
					anchor={anchor}
					ringRadius={ringRadius}
					showInfo={showInfo}
					fogOpacity={fogOpacities.get(tileKey(tile.x, tile.y)) ?? 0}
				/>
			))}
		</>
	);
});

export function TileLayer({
	chunks,
	anchor,
	seed,
	ringRadius = VILLAGE_RING_RADIUS,
	showInfo = false,
}: TileLayerProps) {
	return (
		<>
			{chunks.map((chunk) => (
				<ChunkView
					key={chunkKey(chunk)}
					chunkX={chunk.chunkX}
					chunkY={chunk.chunkY}
					anchor={anchor}
					ringRadius={ringRadius}
					showInfo={showInfo}
					seed={seed}
				/>
			))}
		</>
	);
}
