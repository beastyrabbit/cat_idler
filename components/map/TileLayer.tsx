"use client";

import { memo, useEffect, useMemo, useState } from "react";
import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import type { ChunkCoord } from "@/lib/game/mapView";
import { chunkKey } from "@/lib/game/mapView";
import {
	buildOrganicVillageView,
	computeFogBrightness,
	computeRoadSprites,
	fenceSprites,
	type GatePlacement,
	isExplored,
	type OrganicVillageView,
	tileGround,
} from "@/lib/render/mapTileVisual";
import type { WorldTile } from "@/types/game";
import {
	DIAMOND_CLIP,
	FOG_SHADES,
	ISO,
	VILLAGE_RING_RADIUS,
} from "./constants";

export { ringSprites } from "@/lib/render/mapTileVisual";

interface TileLayerProps {
	chunks: ChunkCoord[];
	anchor: { x: number; y: number };
	/** Fence/clearing ring radius (grows as the village fills). */
	ringRadius?: number;
	/** Organic claimed village footprint; when present it supersedes the ring. */
	claimedTiles?: Array<{ x: number; y: number }>;
	/** Organic gate edge derived by the server for the claimed footprint. */
	villageGate?: GatePlacement | null;
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

/** Resource markers only appear on notably rich tiles (biome max ~60/25). */
const RICH_FOOD = 35;
const RICH_HERBS = 12;

/** Deepest fog shade — used for ungenerated (out-of-window) chunks only. */
const SOLID_FOG = FOG_SHADES[FOG_SHADES.length - 1];

const IsoTile = memo(function IsoTile({
	tile,
	anchor,
	ringRadius,
	village,
	showInfo,
	fogDim,
	roadSprite,
}: {
	tile: WorldTile;
	anchor: { x: number; y: number };
	ringRadius: number;
	village: OrganicVillageView | null;
	showInfo: boolean;
	/** Brightness (0..1) an unexplored tile's terrain is dimmed to; 1 when explored. */
	fogDim: number;
	/** Oriented road sprite when this tile is a road, else undefined. */
	roadSprite?: { src: string; filter?: string };
}) {
	const { left, top } = tileToIso(tile.x, tile.y, ISO);
	const explored = isExplored(tile, anchor, ringRadius, village);
	const tileZ = zIndexFor(tile.x, tile.y, "tile", ISO);
	const objectZ = zIndexFor(tile.x, tile.y, "object", ISO);
	const dim = explored ? 1 : fogDim;

	// Water wins over everything. Explored ground then layers on roads, the
	// cleared village grass, and felled-forest stumps; unexplored ground shows
	// only its bare terrain, dimmed by `dim` (the land, unlit) so fog never
	// reads as a missing tile.
	const sprite: { src: string; filter?: string; base?: string } | undefined =
		tileGround(
			tile,
			anchor,
			ringRadius,
			village,
			explored ? roadSprite : undefined,
		);
	const title = `${tile.type.replaceAll("_", " ")} (${tile.x}, ${tile.y})`;
	// Standalone tree sprites declare a grass `base` underlay; water/road/path
	// sprites carry their own ground and have none.
	const baseSprite = sprite?.base;
	// Compose the sprite's own biome tint with the fog dimming (CSS filters
	// multiply, so a dim brightness stacks onto any existing tint).
	const dimFilter = dim < 1 ? `brightness(${dim})` : undefined;
	const spriteFilter =
		[sprite?.filter, dimFilter].filter(Boolean).join(" ") || undefined;

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
						// Unknown tile type: a flat diamond. Dim it too when fogged.
						background: dim < 1 ? SOLID_FOG : "#8aa37b",
					}}
				/>
			) : (
				<>
					{baseSprite && (
						<img
							src={baseSprite}
							alt=""
							draggable={false}
							className="pointer-events-none absolute select-none"
							style={{
								left,
								top: top - ISO.surfaceOffset,
								width: ISO.tileWidth,
								height: ISO.imageHeight,
								zIndex: tileZ,
								filter: dimFilter,
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
							filter: spriteFilter,
						}}
					/>
				</>
			)}

			{/* Palisade fence around the claimed village footprint. */}
			{explored &&
				fenceSprites(tile, anchor, ringRadius, village).map((fence) => (
					<img
						key={
							"key" in fence && typeof fence.key === "string"
								? fence.key
								: `${fence.src}-${fence.ox}-${fence.oy}`
						}
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
			{explored &&
				showInfo &&
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
	ringRadius,
	village,
	showInfo,
}: {
	chunkX: number;
	chunkY: number;
	anchor: { x: number; y: number };
	ringRadius: number;
	village: OrganicVillageView | null;
	showInfo: boolean;
}) {
	const tiles = useChunkTiles(chunkX, chunkY);
	const fogDims = useMemo(
		() =>
			tiles ? computeFogBrightness(tiles, anchor, ringRadius, village) : null,
		[tiles, anchor, ringRadius, village],
	);
	const roadSprites = useMemo(
		() =>
			tiles ? computeRoadSprites(tiles, anchor, ringRadius, village) : null,
		[tiles, anchor, ringRadius, village],
	);

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
							background: SOLID_FOG,
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
					ringRadius={ringRadius}
					village={village}
					showInfo={showInfo}
					fogDim={fogDims?.get(tile._id) ?? 1}
					roadSprite={roadSprites?.get(tile._id)}
				/>
			))}
		</>
	);
});

export function TileLayer({
	chunks,
	anchor,
	ringRadius = VILLAGE_RING_RADIUS,
	claimedTiles,
	villageGate,
	showInfo = false,
}: TileLayerProps) {
	const village = useMemo(
		() => buildOrganicVillageView(claimedTiles, villageGate),
		[claimedTiles, villageGate],
	);

	return (
		<>
			{chunks.map((chunk) => (
				<ChunkView
					key={chunkKey(chunk)}
					chunkX={chunk.chunkX}
					chunkY={chunk.chunkY}
					anchor={anchor}
					ringRadius={ringRadius}
					village={village}
					showInfo={showInfo}
				/>
			))}
		</>
	);
}
