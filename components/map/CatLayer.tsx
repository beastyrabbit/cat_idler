"use client";

import { tileDiamondCenter, zIndexFor } from "@/lib/game/isoProjection";
import { colonyToWorld } from "@/lib/game/villageLayout";
import { ISO } from "./constants";

export interface MapCat {
	_id: string;
	name: string;
	position: { map: "colony" | "world"; x: number; y: number };
	currentTask: string | null;
	activity?: "idle" | "traveling" | "working" | "returning" | null;
	carrying?: { kind: "food" | "blessings"; amount: number } | null;
}

interface CatLayerProps {
	cats: MapCat[];
	leaderId: string | null;
}

/** Stable small offset inside a tile so co-located cats don't stack exactly. */
function spreadOffset(id: string) {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const ux = ((hash >>> 4) % 100) / 100;
	const uy = ((hash >>> 12) % 100) / 100;
	return {
		x: (ux - 0.5) * ISO.tileWidth * 0.55,
		y: (uy - 0.5) * ISO.tileHeight * 0.5,
	};
}

const TASK_BADGES: Record<string, string> = {
	hunt: "🎯",
	gather_herbs: "🌿",
	fetch_water: "💧",
	build: "🔨",
	guard: "🛡️",
	heal: "💊",
	explore: "🧭",
	patrol: "👁️",
	rest: "💤",
};

const ACTIVITY_BADGES: Record<string, string> = {
	traveling: "🧭",
	working: "⚒️",
	returning: "🏠",
};

export function CatLayer({ cats, leaderId }: CatLayerProps) {
	return (
		<>
			{cats.map((cat) => {
				const worldPos =
					cat.position.map === "world"
						? { x: cat.position.x, y: cat.position.y }
						: colonyToWorld(cat.position);
				const center = tileDiamondCenter(worldPos.x, worldPos.y, ISO);
				const zIndex = zIndexFor(worldPos.x, worldPos.y, "object", ISO);
				const offset = spreadOffset(cat._id);
				const badge =
					(cat.activity ? ACTIVITY_BADGES[cat.activity] : null) ??
					(cat.currentTask ? TASK_BADGES[cat.currentTask] : null);
				const isLeader = cat._id === leaderId;

				return (
					<div
						key={cat._id}
						className="absolute flex w-24 flex-col items-center transition-transform duration-1000 ease-linear"
						style={{
							left: 0,
							top: 0,
							zIndex,
							transform: `translate(${center.x + offset.x - 48}px, ${
								center.y + offset.y - 40
							}px)`,
						}}
						title={`${cat.name}${isLeader ? " (leader)" : ""}${
							cat.currentTask
								? ` — ${cat.currentTask.replaceAll("_", " ")}`
								: ""
						}`}
					>
						<div className="relative text-3xl drop-shadow-md">
							{isLeader && (
								<span className="absolute -top-4 left-1/2 -translate-x-1/2 text-base">
									👑
								</span>
							)}
							🐈
							{badge && (
								<span className="absolute -right-2 -top-1 text-sm">
									{badge}
								</span>
							)}
							{cat.carrying && (
								<span
									className="absolute -left-3 top-0 text-base"
									title={`Carrying ${Math.round(cat.carrying.amount)} ${cat.carrying.kind}`}
								>
									{cat.carrying.kind === "food" ? "🎒" : "✨"}
								</span>
							)}
						</div>
						<span className="mt-0.5 max-w-full truncate rounded-full bg-black/60 px-1.5 text-[11px] font-semibold text-white">
							{cat.name}
						</span>
					</div>
				);
			})}
		</>
	);
}
