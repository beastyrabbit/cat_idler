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
	destination?: { x: number; y: number } | null;
	carrying?: { kind: "food" | "blessings"; amount: number } | null;
	specialization?: "hunter" | "architect" | "ritualist" | null;
}

/** Job hats overlaid on the animated sprite. */
const HAT_SPRITES: Record<string, string> = {
	hunter: "/images/cats/hat-hunter.png",
	architect: "/images/cats/hat-architect.png",
	ritualist: "/images/cats/hat-ritualist.png",
};
const CAT_SHEET = "/images/cats/cat-sheet.png";

// Last rendered tile per cat — big jumps (teleports, speed-ups) snap
// instead of gliding across the whole map.
const lastRendered = new Map<string, { x: number; y: number }>();

/**
 * The sheet is a 360-degree turn: 8 direction groups x 4 walk frames,
 * ordered S, SW, W, NW, N, NE, E, SE. Pick the group whose facing
 * matches the screen-space movement vector.
 */
function directionGroup(dx: number, dy: number): number {
	// World axes on screen: +x runs SE, +y runs SW.
	const sx = dx - dy;
	const sy = (dx + dy) / 2;
	const angle = (Math.atan2(sy, sx) * 180) / Math.PI;
	return Math.round(((((angle - 90) % 360) + 360) % 360) / 45) % 8;
}

/** Floating tool while a cat works, by job (fallback: trade). */
const WORK_ICONS: Record<string, string> = {
	hunt_expedition: "🏹",
	build_house: "🪓",
	ritual: "🔮",
	hunter: "🏹",
	architect: "🪓",
	ritualist: "🔮",
};

interface CatLayerProps {
	cats: MapCat[];
	leaderId: string | null;
	onSelect?: (catId: string) => void;
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

export function CatLayer({ cats, leaderId, onSelect }: CatLayerProps) {
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
				const moving =
					cat.activity === "traveling" || cat.activity === "returning";
				const prev = lastRendered.get(cat._id);
				const jumped =
					prev != null &&
					(Math.abs(worldPos.x - prev.x) > 4 ||
						Math.abs(worldPos.y - prev.y) > 4);
				lastRendered.set(cat._id, { x: worldPos.x, y: worldPos.y });
				const working = cat.activity === "working";
				const group =
					moving && cat.destination
						? directionGroup(
								cat.destination.x - worldPos.x,
								cat.destination.y - worldPos.y,
							)
						: 0;
				const badge =
					(cat.activity ? ACTIVITY_BADGES[cat.activity] : null) ??
					(cat.currentTask ? TASK_BADGES[cat.currentTask] : null);
				const isLeader = cat._id === leaderId;

				return (
					<button
						type="button"
						key={cat._id}
						onClick={() => onSelect?.(cat._id)}
						className={`absolute flex w-24 cursor-pointer flex-col items-center border-0 bg-transparent p-0 ${
							jumped ? "" : "transition-transform duration-1000 ease-linear"
						}`}
						style={{
							left: 0,
							top: 0,
							zIndex,
							transform: `translate(${center.x + offset.x - 48}px, ${
								center.y + offset.y - 40
							}px)`,
						}}
						title={`${cat.name}${isLeader ? " (leader)" : ""} — click for details`}
					>
						<div className="relative drop-shadow-md">
							{isLeader && (
								<span className="absolute -top-4 left-1/2 -translate-x-1/2 text-base">
									👑
								</span>
							)}
							<div
								role="img"
								aria-label={cat.name}
								className={
									working
										? "cat-sprite cat-sprite-spin"
										: moving
											? "cat-sprite cat-sprite-walk"
											: "cat-sprite"
								}
								style={
									{
										backgroundImage: `url(${CAT_SHEET})`,
										transform: "scale(1.4)",
										"--sheet-off": `${-group * 128}px`,
									} as React.CSSProperties
								}
							/>
							{/* Silhouette ghost so cats read through buildings */}
							<div
								aria-hidden
								className="cat-sprite pointer-events-none absolute left-0 top-0"
								style={{
									backgroundImage: `url(${CAT_SHEET})`,
									transform: "scale(1.4)",
									filter: "brightness(0)",
									opacity: 0.28,
									zIndex: 99_990,
									backgroundPositionX: `${-group * 128}px`,
								}}
							/>
							{working && (
								<span className="cat-work-icon" aria-hidden>
									<span className="cat-work-icon-face">
										{(cat.currentTask && WORK_ICONS[cat.currentTask]) ||
											(cat.specialization && WORK_ICONS[cat.specialization]) ||
											"🏹"}
									</span>
									<span className="cat-work-icon-face cat-work-icon-cross">
										{(cat.currentTask && WORK_ICONS[cat.currentTask]) ||
											(cat.specialization && WORK_ICONS[cat.specialization]) ||
											"🏹"}
									</span>
								</span>
							)}
							{cat.specialization && HAT_SPRITES[cat.specialization] && (
								<img
									src={HAT_SPRITES[cat.specialization]}
									alt=""
									draggable={false}
									className="pointer-events-none absolute left-0 top-0 h-8 w-8"
									style={{
										imageRendering: "pixelated",
										transform: "scale(1.4)",
									}}
								/>
							)}
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
					</button>
				);
			})}
		</>
	);
}
