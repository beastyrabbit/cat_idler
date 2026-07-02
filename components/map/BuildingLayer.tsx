"use client";

import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import { BUILDING_SPRITE_FALLBACK, BUILDING_SPRITES, ISO } from "./constants";

export interface MapBuilding {
	_id: string;
	type: string;
	level: number;
	constructionProgress: number;
	worldPosition: { x: number; y: number };
}

interface BuildingLayerProps {
	buildings: MapBuilding[];
}

export function BuildingLayer({ buildings }: BuildingLayerProps) {
	return (
		<>
			{buildings.map((building) => {
				const { x, y } = building.worldPosition;
				const { left, top } = tileToIso(x, y, ISO);
				const zIndex = zIndexFor(x, y, "object", ISO);
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
									top: top - 120,
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
								top: top - ISO.surfaceOffset,
								width: ISO.tileWidth,
								height: ISO.imageHeight,
								zIndex,
								opacity: underConstruction ? 0.45 : 1,
							}}
						/>

						{building.level > 1 && (
							<span
								className="pointer-events-none absolute rounded-full bg-black/70 px-2 py-0.5 text-xs font-bold text-amber-200"
								style={{
									left: left + ISO.tileWidth / 2 - 20,
									top: top - 40,
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
									top: top + ISO.tileHeight / 2 - 4,
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
