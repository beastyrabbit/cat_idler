"use client";

import { memo, useEffect, useState } from "react";
import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import type { ChunkCoord } from "@/lib/game/mapView";
import { chunkKey } from "@/lib/game/mapView";
import type { WorldTile } from "@/types/game";
import {
	DIAMOND_CLIP,
	DIRT_HILL_SPRITE,
	ISO,
	TILE_SPRITES,
	VILLAGE_RING_RADIUS,
} from "./constants";

interface TileLayerProps {
	chunks: ChunkCoord[];
	anchor: { x: number; y: number };
}

// Tiles are immutable in this phase — cache fetched chunks for the session.
const chunkCache = new Map<string, WorldTile[]>();

function useChunkTiles(chunkX: number, chunkY: number): WorldTile[] | null {
	const key = `${chunkX},${chunkY}`;
	const [tiles, setTiles] = useState<WorldTile[] | null>(
		() => chunkCache.get(key) ?? null,
	);

	useEffect(() => {
		// A ChunkView's coordinates never change after mount, so a cached
		// chunk was already picked up by the state initializer above.
		if (chunkCache.has(key)) {
			return;
		}

		let cancelled = false;
		fetch(`/api/game/chunks?x=${chunkX}&y=${chunkY}`)
			.then((response) => (response.ok ? response.json() : null))
			.then((data) => {
				const fetched = (data?.tiles ?? []) as WorldTile[];
				chunkCache.set(key, fetched);
				if (!cancelled) {
					setTiles(fetched);
				}
			})
			.catch((err) => {
				console.warn(`chunk (${chunkX}, ${chunkY}) fetch failed:`, err);
			});

		return () => {
			cancelled = true;
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
	if (tile.pathWear > 0) return true;
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

/** The founding village sits inside a ring of dirt embankments. */
function isVillageRing(
	tile: WorldTile,
	anchor: { x: number; y: number },
): boolean {
	return villageDistance(tile, anchor) === VILLAGE_RING_RADIUS && !hasWater(tile);
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
}: {
	tile: WorldTile;
	anchor: { x: number; y: number };
}) {
	const { left, top } = tileToIso(tile.x, tile.y, ISO);
	const explored = isExplored(tile, anchor);
	const tileZ = zIndexFor(tile.x, tile.y, "tile", ISO);
	const objectZ = zIndexFor(tile.x, tile.y, "object", ISO);

	if (!explored) {
		return (
			<div
				className="absolute bg-slate-900"
				style={{
					left,
					top,
					width: ISO.tileWidth,
					height: ISO.tileHeight,
					zIndex: tileZ,
					clipPath: DIAMOND_CLIP,
				}}
			/>
		);
	}

	const isWater = tile.type === "river";
	const sprite = isVillageClearing(tile, anchor)
		? TILE_SPRITES.field
		: TILE_SPRITES[tile.type];
	const title = `${tile.type.replaceAll("_", " ")} (${tile.x}, ${tile.y})`;

	return (
		<>
			{isWater || !sprite ? (
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
						background: isWater
							? "linear-gradient(160deg, #7cc3ec 0%, #4c94c9 55%, #3a7cae 100%)"
							: "#8aa37b",
						boxShadow: isWater ? "inset 0 0 24px rgba(0,40,80,0.35)" : "none",
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

			{/* Pond/stream overlay on non-water terrain */}
			{!isWater && tile.overlayFeature === "river" && (
				<div
					className="absolute"
					style={{
						left: left + ISO.tileWidth * 0.15,
						top: top + ISO.tileHeight * 0.15,
						width: ISO.tileWidth * 0.7,
						height: ISO.tileHeight * 0.7,
						zIndex: objectZ,
						clipPath: DIAMOND_CLIP,
						background:
							"linear-gradient(160deg, rgba(124,195,236,0.9), rgba(58,124,174,0.9))",
					}}
				/>
			)}

			{/* Dirt embankment around the founding village */}
			{isVillageRing(tile, anchor) && (
				<img
					src={DIRT_HILL_SPRITE}
					alt=""
					draggable={false}
					className="pointer-events-none absolute select-none"
					style={{
						left,
						top: top - ISO.surfaceOffset,
						width: ISO.tileWidth,
						height: ISO.imageHeight,
						zIndex: objectZ,
					}}
				/>
			)}

			{/* Only notably rich tiles get a marker — keeps the map readable */}
			{(tile.resources.food >= RICH_FOOD || tile.resources.herbs >= RICH_HERBS) && (
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
}: {
	chunkX: number;
	chunkY: number;
	anchor: { x: number; y: number };
}) {
	const tiles = useChunkTiles(chunkX, chunkY);

	if (!tiles || tiles.length === 0) {
		// Ungenerated (or still loading) chunk — uncharted territory stays
		// the dark backdrop; no placeholder needed in iso space.
		return null;
	}

	return (
		<>
			{tiles.map((tile) => (
				<IsoTile key={tile._id} tile={tile} anchor={anchor} />
			))}
		</>
	);
});

export function TileLayer({ chunks, anchor }: TileLayerProps) {
	return (
		<>
			{chunks.map((chunk) => (
				<ChunkView
					key={chunkKey(chunk)}
					chunkX={chunk.chunkX}
					chunkY={chunk.chunkY}
					anchor={anchor}
				/>
			))}
		</>
	);
}
