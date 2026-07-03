"use client";

import {
	elevationOffset,
	tileToIso,
	zIndexFor,
} from "@/lib/game/isoProjection";
import { terrainHeightAt, WORLD_TERRAIN_OPTIONS } from "@/lib/game/terrainGen";
import {
	ACTOR,
	BUILDING_SPRITE_FALLBACK,
	BUILDING_SPRITES,
	ISO,
} from "./constants";

export interface MapBuilding {
	_id: string;
	type: string;
	level: number;
	constructionProgress: number;
	worldPosition: { x: number; y: number };
}

interface BuildingLayerProps {
	buildings: MapBuilding[];
	/** World seed, so a building sits on its tile's terrain floor. */
	seed: number | null;
}

export function BuildingLayer({ buildings, seed }: BuildingLayerProps) {
	return (
		<>
			{buildings.map((building) => {
				const { x, y } = building.worldPosition;
				const { left, top } = tileToIso(x, y, ISO);
				const height =
					seed === null
						? 0
						: terrainHeightAt(x, y, seed, WORLD_TERRAIN_OPTIONS);
				const elev = elevationOffset(height);
				const zIndex = zIndexFor(x, y, "object", ISO, height);
				const isShrine = building.type === "shrine";
				const underConstruction = building.constructionProgress < 100;
				const sprite =
					BUILDING_SPRITES[building.type] ?? BUILDING_SPRITE_FALLBACK;
				const title = `${building.type.replaceAll("_", " ")} (level ${building.level})`;

				return (
					<div key={building._id} title={title}>
						{isShrine && (
							<div
								className="pointer-events-none absolute rounded-full bg-amber-300/40 blur-xl"
								style={{
									left: left + ISO.tileWidth / 2 - 90,
									top: top - 120 - elev,
									width: 180,
									height: 180,
									zIndex: zIndex - 1,
								}}
							/>
						)}

						<img
							src={sprite}
							alt={building.type}
							draggable={false}
							className="pointer-events-none absolute select-none drop-shadow-lg"
							style={{
								left,
								top: top - ACTOR.surfaceOffset - elev,
								width: ACTOR.width,
								height: ACTOR.height,
								zIndex,
								opacity: underConstruction ? 0.45 : 1,
							}}
						/>

						{building.level > 1 && (
							<span
								className="pointer-events-none absolute rounded-full bg-black/70 px-2 py-0.5 text-xs font-bold text-amber-200"
								style={{
									left: left + ISO.tileWidth / 2 - 20,
									top: top - 40 - elev,
									zIndex,
								}}
							>
								Lv {building.level}
							</span>
						)}

						{underConstruction && (
							<div
								className="pointer-events-none absolute h-2 w-24 overflow-hidden rounded-full border border-black/40 bg-black/40"
								style={{
									left: left + ISO.tileWidth / 2 - 48,
									top: top + ISO.tileHeight / 2 - 4 - elev,
									zIndex,
								}}
							>
								<div
									className="h-full bg-amber-400"
									style={{ width: `${building.constructionProgress}%` }}
								/>
							</div>
						)}
					</div>
				);
			})}
		</>
	);
}
