"use client";

import { memo, useEffect, useState } from "react";
import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import type { ChunkCoord } from "@/lib/game/mapView";
import { chunkKey } from "@/lib/game/mapView";
import type { WorldTile } from "@/types/game";
import {
	DIAMOND_CLIP,
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	GATE_SPRITE,
	ISO,
	ROAD_SPRITE,
	TILE_SPRITES,
	VILLAGE_RING_RADIUS,
	WATER_SPRITE,
} from "./constants";

interface TileLayerProps {
	chunks: ChunkCoord[];
	anchor: { x: number; y: number };
	/** Info mode: draw resource markers on rich tiles. */
	showInfo?: boolean;
}

// Terrain is near-static, but pathWear (roads, fog reveal) evolves —
// cache chunks and refresh them once a minute.
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

/** Tiles within this distance of the village are always revealed (~10x10). */
const VILLAGE_VISION_RADIUS = 5.5;

/** Resource markers only appear on notably rich tiles (biome max ~60/25). */
const RICH_FOOD = 35;
const RICH_HERBS = 12;

function isExplored(
	tile: WorldTile,
	anchor: { x: number; y: number },
): boolean {
	if (tile.pathWear > 62) return true;
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	return Math.sqrt(dx * dx + dy * dy) < VILLAGE_VISION_RADIUS;
}

function hasWater(tile: WorldTile): boolean {
	return tile.type === "river" || tile.resources.water > 0;
}

/** Chebyshev distance from the village anchor. */
function villageDistance(
	tile: WorldTile,
	anchor: { x: number; y: number },
): number {
	return Math.max(Math.abs(tile.x - anchor.x), Math.abs(tile.y - anchor.y));
}

/**
 * Fence sprite for a village-ring tile: fences follow the edge they sit
 * on, the south side gets an open gate, water gaps stay open.
 */
function ringSprites(
	tile: WorldTile,
	anchor: { x: number; y: number },
): Array<{ src: string; ox: number; oy: number }> {
	if (villageDistance(tile, anchor) !== VILLAGE_RING_RADIUS || hasWater(tile)) {
		return [];
	}
	const dx = tile.x - anchor.x;
	const dy = tile.y - anchor.y;
	if (dx === 0 && dy === VILLAGE_RING_RADIUS) {
		return [{ src: GATE_SPRITE, ox: 0, oy: 0 }];
	}
	const onRow = Math.abs(dy) === VILLAGE_RING_RADIUS;
	const onColumn = Math.abs(dx) === VILLAGE_RING_RADIUS;
	// Corners: shift each direction half a tile toward its edge so the
	// two rails meet in an L instead of crossing in an X.
	if (onRow && onColumn) {
		const sx = Math.sign(dx);
		const sy = Math.sign(dy);
		return [
			{ src: FENCE_X_SPRITE, ox: -sx * 64, oy: -sx * 32 },
			{ src: FENCE_Y_SPRITE, ox: sy * 64, oy: -sy * 32 },
		];
	}
	// Rows (north/south edges) run along x; columns along y.
	return onRow
		? [{ src: FENCE_X_SPRITE, ox: 0, oy: 0 }]
		: [{ src: FENCE_Y_SPRITE, ox: 0, oy: 0 }];
}

/**
 * Ground inside the embankment is cleared for construction — render it as
 * open grass so buildings and cats aren't hidden behind biome trees.
 */
function isVillageClearing(
	tile: WorldTile,
	anchor: { x: number; y: number },
): boolean {
	return villageDistance(tile, anchor) < VILLAGE_RING_RADIUS && !hasWater(tile);
}

const IsoTile = memo(function IsoTile({
	tile,
	anchor,
	showInfo,
}: {
	tile: WorldTile;
	anchor: { x: number; y: number };
	showInfo: boolean;
}) {
	const { left, top } = tileToIso(tile.x, tile.y, ISO);
	const explored = isExplored(tile, anchor);
	const tileZ = zIndexFor(tile.x, tile.y, "tile", ISO);
	const objectZ = zIndexFor(tile.x, tile.y, "object", ISO);

	if (!explored) {
		// Fog blends into the page backdrop so unexplored land reads as
		// "beyond the known world" instead of a hard navy patch.
		return (
			<div
				className="absolute"
				style={{
					left,
					top,
					width: ISO.tileWidth,
					height: ISO.tileHeight,
					zIndex: tileZ,
					clipPath: DIAMOND_CLIP,
					background: "#182115",
				}}
			/>
		);
	}

	const isWater = tile.type === "river" || tile.overlayFeature === "river";
	// Heavily-trodden ground outside the village becomes a visible road.
	// Worldgen seeds faint trails up to ~60 wear; only genuinely cat-worn
	// routes cross this bar.
	const isRoad =
		!isWater && tile.pathWear >= 70 && !isVillageClearing(tile, anchor);
	const sprite = isWater
		? { src: WATER_SPRITE }
		: isRoad
			? { src: ROAD_SPRITE }
			: isVillageClearing(tile, anchor)
				? TILE_SPRITES.field
				: TILE_SPRITES[tile.type];
	const title = `${tile.type.replaceAll("_", " ")} (${tile.x}, ${tile.y})`;

	return (
		<>
			{!sprite ? (
				<div
					title={title}
					className="absolute"
					style={{
						left,
						top,
						width: ISO.tileWidth,
						height: ISO.tileHeight,
						zIndex: tileZ,
						clipPath: DIAMOND_CLIP,
						background: "#8aa37b",
					}}
				/>
			) : (
				<>
					{sprite.base && (
						<img
							src={sprite.base}
							alt=""
							draggable={false}
							className="pointer-events-none absolute select-none"
							style={{
								left,
								top: top - ISO.surfaceOffset,
								width: ISO.tileWidth,
								height: ISO.imageHeight,
								zIndex: tileZ,
							}}
						/>
					)}
					<img
						src={sprite.src}
						alt=""
						title={title}
						draggable={false}
						className="pointer-events-none absolute select-none"
						style={{
							left,
							top: top - ISO.surfaceOffset,
							width: ISO.tileWidth,
							height: ISO.imageHeight,
							zIndex: tileZ,
							filter: sprite.filter,
						}}
					/>
				</>
			)}

			{/* Fence ring (with a south gate) around the founding village */}
			{ringSprites(tile, anchor).map((fence) => (
				<img
					key={fence.src}
					src={fence.src}
					alt=""
					draggable={false}
					className="pointer-events-none absolute select-none"
					style={{
						left: left + fence.ox,
						top: top - ISO.surfaceOffset + fence.oy,
						width: ISO.tileWidth,
						height: ISO.imageHeight,
						zIndex: objectZ,
					}}
				/>
			))}

			{/* Only notably rich tiles get a marker — keeps the map readable */}
			{showInfo &&
				(tile.resources.food >= RICH_FOOD ||
					tile.resources.herbs >= RICH_HERBS) && (
					<div
						className="pointer-events-none absolute flex gap-1 text-base leading-none drop-shadow"
						style={{
							left: left + ISO.tileWidth / 2 - 16,
							top: top + ISO.tileHeight / 2 - 8,
							zIndex: objectZ,
						}}
					>
						{tile.resources.food >= RICH_FOOD && <span>🍖</span>}
						{tile.resources.herbs >= RICH_HERBS && <span>🌿</span>}
					</div>
				)}
		</>
	);
});

const ChunkView = memo(function ChunkView({
	chunkX,
	chunkY,
	anchor,
	showInfo,
}: {
	chunkX: number;
	chunkY: number;
	anchor: { x: number; y: number };
	showInfo: boolean;
}) {
	const tiles = useChunkTiles(chunkX, chunkY);

	if (!tiles || tiles.length === 0) {
		// Ungenerated (or still loading) chunk — solid fog diamonds.
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
							background: "#182115",
						}}
					/>,
				);
			}
		}
		return <>{fog}</>;
	}

	return (
		<>
			{tiles.map((tile) => (
				<IsoTile
					key={tile._id}
					tile={tile}
					anchor={anchor}
					showInfo={showInfo}
				/>
			))}
		</>
	);
});

export function TileLayer({
	chunks,
	anchor,
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
					showInfo={showInfo}
				/>
			))}
		</>
	);
}
