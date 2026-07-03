"use client";

import {
	elevationOffset,
	tileDiamondCenter,
	zIndexFor,
} from "@/lib/game/isoProjection";
import { terrainHeightAt, WORLD_TERRAIN_OPTIONS } from "@/lib/game/terrainGen";
import { ISO } from "./constants";

export interface MapRaider {
	_id: string;
	position: { x: number; y: number };
	hp: number;
	strength: number;
	status: "advancing" | "engaging" | "retreating" | "dead";
}

/** Dark recolor of the cat sheet — the raiders read as a menacing warband. */
const RAIDER_SHEET = "/images/cats/raider-sheet.png";

/** Small stable offset so co-located raiders don't stack exactly. */
function spread(id: string) {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const ux = ((hash >>> 4) % 100) / 100;
	const uy = ((hash >>> 12) % 100) / 100;
	return {
		x: (ux - 0.5) * ISO.tileWidth * 0.5,
		y: (uy - 0.5) * ISO.tileHeight * 0.45,
	};
}

interface RaiderLayerProps {
	raiders: MapRaider[];
	/** World seed, so a raider rides its tile's terrain floor. */
	seed: number | null;
}

/** Enemy raiders marching on the village, drawn on the world map. */
export function RaiderLayer({ raiders, seed }: RaiderLayerProps) {
	return (
		<>
			{raiders.map((raider) => {
				const center = tileDiamondCenter(
					raider.position.x,
					raider.position.y,
					ISO,
				);
				const height =
					seed === null
						? 0
						: terrainHeightAt(
								Math.round(raider.position.x),
								Math.round(raider.position.y),
								seed,
								WORLD_TERRAIN_OPTIONS,
							);
				const elev = elevationOffset(height);
				const zIndex = zIndexFor(
					raider.position.x,
					raider.position.y,
					"object",
					ISO,
					height,
				);
				const offset = spread(raider._id);
				const hpPct = Math.max(
					0,
					Math.min(100, (raider.hp / Math.max(1, raider.strength)) * 100),
				);
				const engaging = raider.status === "engaging";
				return (
					<div
						key={raider._id}
						className="absolute flex w-16 flex-col items-center"
						style={{
							left: 0,
							top: 0,
							zIndex,
							transform: `translate(${center.x + offset.x - 32}px, ${
								center.y + offset.y - 40 - elev
							}px)`,
						}}
						title={`Raider — ${Math.round(raider.hp)}/${Math.round(raider.strength)} HP`}
					>
						<div className="relative drop-shadow-md">
							<div
								role="img"
								aria-label="raider"
								className={
									engaging
										? "cat-sprite cat-sprite-spin"
										: "cat-sprite cat-sprite-walk"
								}
								style={
									{
										backgroundImage: `url(${RAIDER_SHEET})`,
										transform: "scale(1.35)",
										"--sheet-off": "0px",
									} as React.CSSProperties
								}
							/>
							<span className="absolute -top-3 left-1/2 -translate-x-1/2 text-sm">
								⚔️
							</span>
						</div>
						{/* HP bar so players can see their defense clicks landing. */}
						<span className="mt-0.5 h-1.5 w-10 overflow-hidden rounded-full border border-black/50 bg-black/40">
							<span
								className="block h-full bg-red-600"
								style={{ width: `${hpPct}%` }}
							/>
						</span>
					</div>
				);
			})}
		</>
	);
}
