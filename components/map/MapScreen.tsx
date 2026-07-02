"use client";

import { useCallback, useMemo, useRef, useState } from "react";

import { MapViewport, type ViewportView } from "@/components/ui/MapViewport";
import { formatDuration, useGameDashboard } from "@/hooks/useGameDashboard";
import { tileDiamondCenter, visibleChunksIso } from "@/lib/game/isoProjection";
import { type ChunkCoord, chunkKey } from "@/lib/game/mapView";
import { shrineWorldPosition } from "@/lib/game/villageLayout";
import { BuildingLayer } from "./BuildingLayer";
import { CatLayer } from "./CatLayer";
import { CHUNK_MAX, CHUNK_MIN, ISO, ISO_CONTENT } from "./constants";
import { TileLayer } from "./TileLayer";

const JOB_LABELS: Record<string, string> = {
	supply_food: "Supply Food",
	supply_water: "Supply Water",
	leader_plan_hunt: "Plan Hunt",
	hunt_expedition: "Hunt Expedition",
	leader_plan_house: "Plan House",
	build_house: "Build House",
	ritual: "Shrine Ritual",
};

const STATUS_STYLES: Record<string, string> = {
	starting: "bg-sky-100 text-sky-800",
	thriving: "bg-emerald-100 text-emerald-800",
	struggling: "bg-amber-100 text-amber-900",
	dead: "bg-red-100 text-red-800",
};

/** Parchment panel + wooden border, Travian-style. */
const PANEL =
	"rounded-lg border-2 border-[#5d4024] bg-[#f3e6c8] shadow-[0_2px_8px_rgba(0,0,0,0.45)]";
const PANEL_HEADING =
	"mb-2 font-serif text-xs font-bold uppercase tracking-wider text-[#6b4a2a]";
const WOOD_BUTTON =
	"rounded border border-[#3f2c17] bg-gradient-to-b from-[#8a6136] to-[#6b4a2a] px-2 py-1.5 text-xs font-bold text-amber-50 shadow hover:from-[#9a7146] hover:to-[#7b5a3a] disabled:opacity-50";

const INITIAL_CHUNKS: ChunkCoord[] = [];
for (let y = -1; y <= 1; y++) {
	for (let x = -1; x <= 1; x++) {
		INITIAL_CHUNKS.push({ chunkX: x, chunkY: y });
	}
}

export function MapScreen() {
	const {
		dashboard,
		colony,
		jobs,
		upgrades,
		cats,
		buildings,
		anchor,
		ritualPoints,
		statusTone,
		now,
		busyAction,
		error,
		connectionLost,
		submitJob,
		onBoostJob,
		onBuyUpgrade,
		onSetAcceleration,
		onAdvanceTime,
		showTestControls,
		accelerationPreset,
		onlineCount,
	} = useGameDashboard();

	const [chunks, setChunks] = useState<ChunkCoord[]>(INITIAL_CHUNKS);
	const [showUpgrades, setShowUpgrades] = useState(false);
	const chunksKeyRef = useRef("");

	const onViewChange = useCallback((view: ViewportView) => {
		const visible = visibleChunksIso(view, ISO).filter(
			(chunk) =>
				chunk.chunkX >= CHUNK_MIN &&
				chunk.chunkX <= CHUNK_MAX &&
				chunk.chunkY >= CHUNK_MIN &&
				chunk.chunkY <= CHUNK_MAX,
		);
		const key = visible.map(chunkKey).join("|");
		if (key !== chunksKeyRef.current) {
			chunksKeyRef.current = key;
			setChunks(visible);
		}
	}, []);

	const shrineCenter = useMemo(() => {
		const shrine = shrineWorldPosition();
		return tileDiamondCenter(shrine.x, shrine.y, ISO);
	}, []);

	if (dashboard === undefined) {
		return (
			<div className="flex h-dvh items-center justify-center bg-[#141c12] text-amber-100">
				<p className="animate-pulse font-serif text-lg">
					Preparing Global Colony…
				</p>
			</div>
		);
	}

	if (!colony) {
		return (
			<div className="flex h-dvh items-center justify-center bg-[#141c12] text-amber-100">
				<p className="font-serif text-lg">Waking up the colony…</p>
			</div>
		);
	}

	const resources = colony.resources;

	return (
		<div className="relative h-dvh overflow-hidden bg-[#141c12]">
			{/* Map fills the screen */}
			<div className="absolute inset-0">
				<MapViewport
					contentSize={ISO_CONTENT}
					height="100%"
					initialScale={0.45}
					minScale={0.08}
					maxScale={1.4}
					initialCenter={shrineCenter}
					onViewChange={onViewChange}
				>
					<div
						className="relative"
						style={{ width: ISO_CONTENT.width, height: ISO_CONTENT.height }}
					>
						<TileLayer chunks={chunks} anchor={anchor} />
						<BuildingLayer buildings={buildings} />
						<CatLayer cats={cats} leaderId={colony.leaderId ?? null} />
					</div>
				</MapViewport>
			</div>

			{/* Top HUD bar */}
			<header className="pointer-events-none absolute inset-x-0 top-0 z-20 flex flex-wrap items-center gap-2 border-b-2 border-[#3f2c17] bg-gradient-to-b from-[#4a3319]/95 to-[#3a2712]/95 p-2 shadow-lg backdrop-blur-sm">
				<h1 className="pointer-events-auto px-2 font-serif text-lg font-black tracking-wide text-amber-100">
					🐾 Catford
				</h1>

				<div className="pointer-events-auto flex items-center gap-3 rounded-md border border-[#5d4024] bg-[#f3e6c8] px-4 py-1.5 text-sm font-bold text-[#4a3319] shadow-inner">
					<span title="Food">🍖 {Math.floor(resources.food)}</span>
					<span title="Water">💧 {Math.floor(resources.water)}</span>
					<span title="Herbs">🌿 {Math.floor(resources.herbs)}</span>
					<span title="Materials">🪵 {Math.floor(resources.materials)}</span>
					<span title="Ritual points" className="text-amber-700">
						✨ {ritualPoints}
					</span>
				</div>

				<span
					className={`pointer-events-auto rounded-full px-3 py-1 text-xs font-bold uppercase tracking-wide shadow ${STATUS_STYLES[statusTone]}`}
				>
					{statusTone}
				</span>

				<span className="pointer-events-auto rounded-full border border-[#5d4024] bg-[#f3e6c8] px-3 py-1 text-xs font-semibold text-[#4a3319] shadow">
					👥 {onlineCount} online · 🐈 {cats.length} cats
				</span>

				<div className="flex-1" />

				<a
					href="/game/newspaper"
					className="pointer-events-auto rounded-md border border-[#5d4024] bg-[#f3e6c8] px-3 py-1.5 font-serif text-sm font-bold text-[#4a3319] shadow hover:bg-amber-100"
					title="Read The Catford Examiner"
				>
					📰 Examiner
				</a>

				{connectionLost && (
					<span className="pointer-events-auto rounded-lg bg-slate-800 px-3 py-1 text-xs font-semibold text-amber-300 shadow">
						⚠ Connection lost — reconnecting…
					</span>
				)}

				{error && (
					<span className="pointer-events-auto rounded-lg bg-red-700/90 px-3 py-1 text-xs font-semibold text-white shadow">
						{error}
					</span>
				)}
			</header>

			{/* Right action panel */}
			<aside className="absolute bottom-3 right-3 top-16 z-20 flex w-72 max-w-[80vw] flex-col gap-2 overflow-hidden">
				<div className={`flex-1 overflow-y-auto p-3 ${PANEL}`}>
					<h3 className={PANEL_HEADING}>Colony Work</h3>
					{jobs.length === 0 ? (
						<p className="text-sm text-[#6b4a2a]/70">Nothing in progress.</p>
					) : (
						<ul className="space-y-2">
							{jobs.map((job: any) => {
								const remaining = job.endsAt - now;
								const active = job.status === "active";
								return (
									<li
										key={job._id}
										className="rounded border border-[#5d4024]/40 bg-[#e9d9b4] p-2"
									>
										<div className="flex items-center justify-between gap-2">
											<span className="text-sm font-bold text-[#4a3319]">
												{JOB_LABELS[job.kind] ?? job.kind}
											</span>
											<span className="text-xs text-[#6b4a2a]">
												{active ? formatDuration(remaining) : "queued"}
											</span>
										</div>
										{active && (
											<button
												type="button"
												onClick={() => onBoostJob(job._id)}
												disabled={busyAction === job._id}
												className="mt-1.5 w-full rounded border border-amber-700 bg-gradient-to-b from-amber-400 to-amber-500 px-2 py-1 text-xs font-bold text-[#3a2712] shadow hover:from-amber-300 hover:to-amber-400 disabled:opacity-50"
											>
												🐾 Boost (-10s)
											</button>
										)}
									</li>
								);
							})}
						</ul>
					)}
				</div>

				<div className={`p-3 ${PANEL}`}>
					<h3 className={PANEL_HEADING}>Lend a Paw</h3>
					<div className="grid grid-cols-2 gap-1.5">
						<button
							type="button"
							onClick={() => submitJob("supply_food")}
							disabled={busyAction === "supply_food"}
							className={WOOD_BUTTON}
						>
							🍖 Supply food
						</button>
						<button
							type="button"
							onClick={() => submitJob("supply_water")}
							disabled={busyAction === "supply_water"}
							className={WOOD_BUTTON}
						>
							💧 Supply water
						</button>
						<button
							type="button"
							onClick={() => submitJob("leader_plan_hunt")}
							disabled={busyAction === "leader_plan_hunt"}
							className={WOOD_BUTTON}
						>
							🎯 Plan hunt
						</button>
						<button
							type="button"
							onClick={() => submitJob("leader_plan_house")}
							disabled={busyAction === "leader_plan_house"}
							className={WOOD_BUTTON}
						>
							🏠 Plan house
						</button>
						<button
							type="button"
							onClick={() => submitJob("ritual")}
							disabled={busyAction === "ritual"}
							className={`col-span-2 ${WOOD_BUTTON} !from-red-800 !to-red-900 hover:!from-red-700 hover:!to-red-800`}
						>
							⛩️ Request shrine ritual
						</button>
						<button
							type="button"
							onClick={() => setShowUpgrades((v) => !v)}
							className={`col-span-2 ${WOOD_BUTTON}`}
						>
							✨ Upgrades ({ritualPoints} pts)
						</button>
					</div>

					{showUpgrades && (
						<ul className="mt-2 space-y-1.5 border-t border-[#5d4024]/40 pt-2">
							{upgrades.map((upgrade: any) => {
								const maxed = upgrade.level >= upgrade.maxLevel;
								const cost = upgrade.baseCost * (upgrade.level + 1);
								return (
									<li
										key={upgrade.key}
										className="flex items-center justify-between gap-2 text-xs font-semibold text-[#4a3319]"
									>
										<span>
											{upgrade.key.replaceAll("_", " ")}{" "}
											<span className="text-[#6b4a2a]/70">
												({upgrade.level}/{upgrade.maxLevel})
											</span>
										</span>
										<button
											type="button"
											onClick={() => onBuyUpgrade(upgrade.key)}
											disabled={
												maxed || busyAction === `upgrade:${upgrade.key}`
											}
											className="rounded border border-amber-700 bg-gradient-to-b from-amber-400 to-amber-500 px-2 py-0.5 font-bold text-[#3a2712] shadow hover:from-amber-300 disabled:opacity-40"
										>
											{maxed ? "MAX" : `${cost} ✨`}
										</button>
									</li>
								);
							})}
						</ul>
					)}
				</div>

				{showTestControls && (
					<div className="rounded-lg border-2 border-purple-900 bg-purple-950/90 p-3 text-white shadow-md backdrop-blur">
						<h3 className="mb-2 text-xs font-bold uppercase tracking-wider text-purple-300">
							Test Controls ({accelerationPreset})
						</h3>
						<div className="grid grid-cols-3 gap-1.5 text-xs">
							{(["off", "fast", "turbo"] as const).map((preset) => (
								<button
									key={preset}
									type="button"
									onClick={() => onSetAcceleration(preset)}
									className={`rounded px-2 py-1 font-semibold ${
										accelerationPreset === preset
											? "bg-purple-400 text-black"
											: "bg-white/10 hover:bg-white/20"
									}`}
								>
									{preset}
								</button>
							))}
							<button
								type="button"
								onClick={() => onAdvanceTime(300)}
								className="rounded bg-white/10 px-2 py-1 font-semibold hover:bg-white/20"
							>
								+5m
							</button>
							<button
								type="button"
								onClick={() => onAdvanceTime(3600)}
								className="rounded bg-white/10 px-2 py-1 font-semibold hover:bg-white/20"
							>
								+1h
							</button>
						</div>
					</div>
				)}
			</aside>
		</div>
	);
}
