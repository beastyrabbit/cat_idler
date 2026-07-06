"use client";

import { useCallback, useMemo, useRef, useState } from "react";

import { MapViewport, type ViewportView } from "@/components/ui/MapViewport";
import { formatDuration, useGameDashboard } from "@/hooks/useGameDashboard";
import {
	isoToTile,
	tileDiamondCenter,
	visibleChunksIso,
} from "@/lib/game/isoProjection";
import { getLifeStage, tradeLevel } from "@/lib/game/lifeSim";
import { type ChunkCoord, chunkKey } from "@/lib/game/mapView";
import { shrineWorldPosition } from "@/lib/game/villageLayout";
import type { LifeStage } from "@/types/game";
import { BuildingLayer } from "./BuildingLayer";
import { CatLayer } from "./CatLayer";
import { CHUNK_MAX, CHUNK_MIN, ISO, ISO_CONTENT } from "./constants";
import { RaiderLayer } from "./RaiderLayer";
import { TileLayer } from "./TileLayer";
import { TreePanel } from "./TreePanel";
import { ZoneLayer } from "./ZoneLayer";

const JOB_LABELS: Record<string, string> = {
	supply_food: "Supply Food",
	supply_water: "Supply Water",
	leader_plan_hunt: "Plan Hunt",
	hunt_expedition: "Hunt Expedition",
	fish: "Fishing",
	leader_plan_house: "Plan House",
	build_house: "Build House",
	ritual: "Shrine Ritual",
	quarry: "Quarry Expedition",
	explore: "Scouting",
};

/** Per-cat role experience carried on the dashboard row. */
type TradeXp = {
	hunter: number;
	architect: number;
	ritualist: number;
	warrior?: number;
};

const STAGE_LABELS: Record<LifeStage, string> = {
	kitten: "🍼 Kitten",
	young: "🐱 Young",
	adult: "🐈 Adult",
	elder: "🧓 Elder",
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
		events,
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
		housing,
		leader,
		election,
		voteKick,
		onCastVote,
		onRequestVoteKick,
		zones,
		onCreateZone,
		onRemoveZone,
		onPlanBuilding,
		onAssignWorker,
		onBuildRoad,
		research,
		onUnlockNode,
		threat,
		raiders,
		onTrainWarrior,
		onDefendRaid,
	} = useGameDashboard();

	const [chunks, setChunks] = useState<ChunkCoord[]>(INITIAL_CHUNKS);
	const [showUpgrades, setShowUpgrades] = useState(false);
	const [showTree, setShowTree] = useState(false);
	const [infoMode, setInfoMode] = useState(false);
	const [selectedCatId, setSelectedCatId] = useState<string | null>(null);
	const [zoneDraft, setZoneDraft] = useState<{
		kind: "avoid" | "gather" | "road";
		durationMs: number;
		cornerA: { x: number; y: number } | null;
	} | null>(null);
	const zoneOverlayRef = useRef<HTMLDivElement | null>(null);
	const chunksKeyRef = useRef("");

	const onZoneOverlayClick = useCallback(
		(e: React.MouseEvent) => {
			const el = zoneOverlayRef.current;
			if (!el || !zoneDraft) {
				return;
			}
			const rect = el.getBoundingClientRect();
			// The content plane is CSS-scaled; undo it before projecting.
			const scale = rect.width / ISO_CONTENT.width;
			const px = (e.clientX - rect.left) / scale;
			const py = (e.clientY - rect.top) / scale;
			const tile = isoToTile(px, py, ISO);
			const corner = { x: Math.round(tile.x), y: Math.round(tile.y) };

			if (!zoneDraft.cornerA) {
				setZoneDraft({ ...zoneDraft, cornerA: corner });
				return;
			}
			if (zoneDraft.kind === "road") {
				void onBuildRoad(zoneDraft.cornerA, corner);
			} else {
				void onCreateZone(
					zoneDraft.kind,
					zoneDraft.cornerA,
					corner,
					zoneDraft.durationMs,
				);
			}
			setZoneDraft(null);
		},
		[zoneDraft, onCreateZone, onBuildRoad],
	);

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
	const villageRadius = dashboard?.villageRadius ?? 4;
	const claimedTiles = dashboard?.claimedTiles;
	const villageGate = dashboard?.villageGate ?? null;
	const caps = dashboard?.storage?.capacities ?? {
		food: dashboard?.storage?.foodCapacity ?? 200,
		water: 200,
		herbs: 100,
		materials: 100,
		refined: 100,
	};
	const resourceBars: Array<{
		icon: string;
		label: string;
		value: number;
		cap: number;
		fill: string;
	}> = [
		{
			icon: "🍖",
			label: "Food",
			value: resources.food,
			cap: caps.food,
			fill: "bg-emerald-600",
		},
		{
			icon: "💧",
			label: "Water",
			value: resources.water,
			cap: caps.water,
			fill: "bg-sky-600",
		},
		{
			icon: "🌿",
			label: "Herbs",
			value: resources.herbs,
			cap: caps.herbs,
			fill: "bg-lime-600",
		},
		{
			icon: "🪵",
			label: "Materials",
			value: resources.materials,
			cap: caps.materials,
			fill: "bg-amber-700",
		},
		{
			icon: "⚙️",
			label: "Refined",
			value: resources.refined ?? 0,
			cap: caps.refined,
			fill: "bg-slate-500",
		},
		{
			icon: "⚔️",
			label: "Weapons",
			value: resources.weapons ?? 0,
			cap: caps.weapons ?? 50,
			fill: "bg-rose-600",
		},
		{
			icon: "🛡️",
			label: "Armor",
			value: resources.armor ?? 0,
			cap: caps.armor ?? 50,
			fill: "bg-zinc-500",
		},
	];

	const THREAT_STYLES: Record<string, string> = {
		calm: "bg-emerald-100 text-emerald-800",
		rising: "bg-amber-100 text-amber-900",
		imminent: "bg-red-200 text-red-900 animate-pulse",
	};
	const THREAT_LABELS: Record<string, string> = {
		calm: "🕊️ Calm",
		rising: "⚠️ Rising",
		imminent: "🔥 Raid imminent",
	};
	const threatBand = (threat?.band ?? "calm") as string;
	const raidActive = Boolean(threat?.raidActive);

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
						className="relative select-none"
						style={{ width: ISO_CONTENT.width, height: ISO_CONTENT.height }}
					>
						<TileLayer
							chunks={chunks}
							anchor={anchor}
							ringRadius={villageRadius}
							claimedTiles={claimedTiles}
							villageGate={villageGate}
							showInfo={infoMode}
						/>
						<ZoneLayer
							zones={zones}
							now={now}
							onRemove={onRemoveZone}
							draftCorner={zoneDraft?.cornerA ?? null}
						/>
						<BuildingLayer buildings={buildings} />
						<CatLayer
							cats={cats}
							leaderId={colony.leaderId ?? null}
							onSelect={setSelectedCatId}
						/>
						<RaiderLayer raiders={raiders} />
						{zoneDraft && (
							// Drawing mode: swallow clicks/pans and pick tile corners.
							// biome-ignore lint/a11y/noStaticElementInteractions lint/a11y/useKeyWithClickEvents: full-map drawing surface; keyboard flow uses the panel buttons
							<div
								ref={zoneOverlayRef}
								className="absolute inset-0 cursor-crosshair"
								style={{ zIndex: 100_000 }}
								onPointerDown={(e) => e.stopPropagation()}
								onClick={onZoneOverlayClick}
							/>
						)}
					</div>
				</MapViewport>
			</div>

			{/* Top HUD bar */}
			<header className="pointer-events-none absolute inset-x-0 top-0 z-20 flex flex-wrap items-center gap-2 border-b-2 border-[#3f2c17] bg-gradient-to-b from-[#4a3319]/95 to-[#3a2712]/95 p-2 shadow-lg backdrop-blur-sm">
				<h1 className="pointer-events-auto px-2 font-serif text-lg font-black tracking-wide text-amber-100">
					🐾 Catford
				</h1>

				<div className="pointer-events-auto flex items-center gap-3 rounded-md border border-[#5d4024] bg-[#f3e6c8] px-4 py-1.5 text-sm font-bold text-[#4a3319] shadow-inner">
					{resourceBars.map((bar) => (
						<span
							key={bar.label}
							title={`${bar.label}: ${Math.floor(bar.value)} / ${bar.cap}${
								bar.label === "Food"
									? ". Leader tithes surplus (20 food or 5 refined = 1 blessing); overflow beyond capacity spoils."
									: ""
							}`}
							className="flex items-center gap-1"
						>
							{bar.icon} {Math.floor(bar.value)}
							<span className="h-2 w-12 overflow-hidden rounded-full border border-[#5d4024]/50 bg-black/10">
								<span
									className={`block h-full ${bar.fill}`}
									style={{
										width: `${Math.min(100, (bar.value / bar.cap) * 100)}%`,
									}}
								/>
							</span>
						</span>
					))}
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

				{housing && (
					<span
						className={`pointer-events-auto rounded-full border border-[#5d4024] px-3 py-1 text-xs font-semibold shadow ${
							housing.pressure >= 1
								? "bg-red-100 text-red-900"
								: housing.pressure >= 0.8
									? "bg-amber-100 text-amber-900"
									: "bg-[#f3e6c8] text-[#4a3319]"
						}`}
						title={`Housing pressure ${Math.round(housing.pressure * 100)}% — village level ${housing.villageLevel}`}
					>
						🏠 {housing.population}/{housing.capacity} · Lv{" "}
						{housing.villageLevel}
					</span>
				)}

				<span
					className={`pointer-events-auto rounded-full border border-[#5d4024] px-3 py-1 text-xs font-bold shadow ${THREAT_STYLES[threatBand]}`}
					title={`Threat: ${threatBand}. Standing guard: ${threat?.warriors ?? 0} warriors · ${threat?.weapons ?? 0}⚔️ ${threat?.armor ?? 0}🛡️ in the armory.`}
				>
					{THREAT_LABELS[threatBand]} · 🗡️ {threat?.warriors ?? 0}
				</span>

				<div className="flex-1" />

				<button
					type="button"
					onClick={() => setShowTree(true)}
					className="pointer-events-auto rounded-md border border-[#5d4024] bg-[#f3e6c8] px-3 py-1.5 text-sm font-bold text-[#4a3319] shadow hover:bg-amber-100"
					title="Open the god/cat upgrade tree"
				>
					🌳 Tree
					{research?.nextTarget && (
						<span className="ml-1 text-[11px] text-emerald-700">
							🔬 {Math.floor(research.researchPoints)}/
							{research.nextTarget.cost}
						</span>
					)}
				</button>

				<button
					type="button"
					onClick={() => setInfoMode((v) => !v)}
					className={`pointer-events-auto rounded-md border border-[#5d4024] px-3 py-1.5 text-sm font-bold shadow ${
						infoMode
							? "bg-amber-400 text-[#3a2712]"
							: "bg-[#f3e6c8] text-[#4a3319] hover:bg-amber-100"
					}`}
					title="Toggle tile info markers (resources)"
				>
					ℹ️ Info
				</button>

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
			<aside className="absolute bottom-3 right-3 top-24 z-10 flex w-72 max-w-[80vw] flex-col gap-2">
				{/* Queue grows with its content, scrolls when it outgrows the column */}
				<div className={`min-h-0 shrink overflow-y-auto p-3 ${PANEL}`}>
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
										{job.clickTimeReducedSec > 0 && (
											<p className="mt-0.5 text-[10px] font-semibold text-emerald-800">
												🐾 boosted {Math.round(job.clickTimeReducedSec)}s total
											</p>
										)}
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

				{/* Leadership: current leader, live election, vote-kick petition */}
				<div className={`p-3 ${PANEL}`}>
					<h3 className={PANEL_HEADING}>Leadership</h3>
					<p className="text-sm font-bold text-[#4a3319]">
						👑 {leader ? leader.name : "No leader yet"}
						{leader && (
							<span className="ml-1 font-normal text-[#6b4a2a]">
								(leadership {Math.round(leader.stats.leadership)})
							</span>
						)}
					</p>

					{election && (
						<div className="mt-2 border-t border-[#5d4024]/40 pt-2">
							<p className="mb-1.5 text-xs font-bold uppercase tracking-wide text-[#6b4a2a]">
								🗳️ Election — closes in{" "}
								{formatDuration(Math.max(0, election.endsAt - now))}
							</p>
							<ul className="space-y-1">
								{election.candidates.map(
									(candidate: {
										_id: string;
										name: string;
										leadership: number;
									}) => (
										<li
											key={candidate._id}
											className="flex items-center justify-between gap-2 text-xs font-semibold text-[#4a3319]"
										>
											<span>
												{candidate.name}{" "}
												<span className="text-[#6b4a2a]/70">
													lead {Math.round(candidate.leadership)} ·{" "}
													{election.tally[candidate._id] ?? 0} vote
													{(election.tally[candidate._id] ?? 0) === 1
														? ""
														: "s"}
												</span>
											</span>
											<button
												type="button"
												onClick={() => onCastVote(election._id, candidate._id)}
												disabled={busyAction === `vote:${election._id}`}
												className="rounded border border-amber-700 bg-gradient-to-b from-amber-400 to-amber-500 px-2 py-0.5 font-bold text-[#3a2712] shadow hover:from-amber-300 disabled:opacity-40"
											>
												Vote
											</button>
										</li>
									),
								)}
							</ul>
						</div>
					)}

					{leader && (
						<div className="mt-2 border-t border-[#5d4024]/40 pt-2">
							{voteKick ? (
								<p className="text-xs font-semibold text-red-900">
									⚖️ Petition to remove {voteKick.targetName}:{" "}
									{voteKick.signatures}/{voteKick.needed} signatures ·{" "}
									{formatDuration(Math.max(0, voteKick.endsAt - now))} left
								</p>
							) : null}
							<button
								type="button"
								onClick={() => onRequestVoteKick()}
								disabled={busyAction === "voteKick"}
								className={`mt-1.5 w-full ${WOOD_BUTTON} !from-red-800 !to-red-900 hover:!from-red-700 hover:!to-red-800`}
							>
								{voteKick ? "✍️ Sign the petition" : "⚖️ Demand a new leader"}
							</button>
						</div>
					)}
				</div>

				{/* Production: workshops and research huts need workers */}
				{buildings.some(
					(b: { type: string; constructionProgress: number }) =>
						(b.type === "workshop" ||
							b.type === "research_hut" ||
							b.type === "smithy") &&
						b.constructionProgress >= 100,
				) && (
					<div className={`p-3 ${PANEL}`}>
						<h3 className={PANEL_HEADING}>Production</h3>
						<ul className="space-y-1.5">
							{buildings
								.filter(
									(b: { type: string; constructionProgress: number }) =>
										(b.type === "workshop" ||
											b.type === "research_hut" ||
											b.type === "smithy") &&
										b.constructionProgress >= 100,
								)
								.map(
									(shop: {
										_id: string;
										type: string;
										worldPosition: { x: number; y: number };
									}) => {
										const worker = cats.find(
											(cat: { assignedBuildingId?: string | null }) =>
												cat.assignedBuildingId === shop._id,
										);
										const label =
											shop.type === "research_hut"
												? "🔬 Research hut"
												: shop.type === "smithy"
													? "🗡️ Smithy"
													: "⚒️ Workshop";
										return (
											<li
												key={shop._id}
												className="flex items-center justify-between gap-2 text-xs font-semibold text-[#4a3319]"
											>
												<span>{label}</span>
												<select
													className="rounded border border-[#5d4024] bg-[#e9d9b4] px-1 py-0.5 text-xs"
													value={worker?._id ?? ""}
													onChange={(e) => {
														const catId = e.target.value;
														if (catId) {
															void onAssignWorker(catId, shop._id);
														} else if (worker) {
															void onAssignWorker(worker._id, null);
														}
													}}
												>
													<option value="">— no worker —</option>
													{cats.map((cat: { _id: string; name: string }) => (
														<option key={cat._id} value={cat._id}>
															{cat.name}
														</option>
													))}
												</select>
											</li>
										);
									},
								)}
						</ul>
					</div>
				)}

				{/* Player zones: steer the cats */}
				<div className={`mt-auto p-3 ${PANEL}`}>
					<h3 className={PANEL_HEADING}>Zones</h3>
					{zoneDraft ? (
						<div className="text-xs font-semibold text-[#4a3319]">
							<p>
								{zoneDraft.cornerA
									? "Click the opposite corner on the map."
									: "Click the first corner on the map."}
							</p>
							<button
								type="button"
								onClick={() => setZoneDraft(null)}
								className={`mt-1.5 w-full ${WOOD_BUTTON}`}
							>
								✖ Cancel
							</button>
						</div>
					) : (
						<div className="grid grid-cols-2 gap-1.5">
							<button
								type="button"
								onClick={() =>
									setZoneDraft({
										kind: "gather",
										durationMs: 30 * 60 * 1000,
										cornerA: null,
									})
								}
								className={WOOD_BUTTON}
								title="Cats prefer gathering here (30 min)"
							>
								📍 Gather zone
							</button>
							<button
								type="button"
								onClick={() =>
									setZoneDraft({
										kind: "avoid",
										durationMs: 30 * 60 * 1000,
										cornerA: null,
									})
								}
								className={WOOD_BUTTON}
								title="Cats stay away from here (30 min)"
							>
								🚫 Avoid zone
							</button>
						</div>
					)}
					{zones.length > 0 && !zoneDraft && (
						<p className="mt-1.5 text-[11px] text-[#6b4a2a]">
							{zones.length} active zone{zones.length === 1 ? "" : "s"} — click
							one on the map to remove it.
						</p>
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
							onClick={() => onPlanBuilding("workshop")}
							disabled={
								busyAction === "build:workshop" ||
								(housing?.villageLevel ?? 1) < 2
							}
							className={WOOD_BUTTON}
							title={
								(housing?.villageLevel ?? 1) < 2
									? "Unlocks at village level 2"
									: "Refines materials into goods (needs a worker)"
							}
						>
							⚒️ Workshop
						</button>
						<button
							type="button"
							onClick={() => onPlanBuilding("field")}
							disabled={
								busyAction === "build:field" || (housing?.villageLevel ?? 1) < 4
							}
							className={WOOD_BUTTON}
							title={
								(housing?.villageLevel ?? 1) < 4
									? "Unlocks at village level 4"
									: "Grows food passively"
							}
						>
							🌾 Field
						</button>
						<button
							type="button"
							onClick={() => onPlanBuilding("research_hut")}
							disabled={
								busyAction === "build:research_hut" ||
								!research?.ownedNodeIds?.includes("research_hut")
							}
							className={WOOD_BUTTON}
							title={
								research?.ownedNodeIds?.includes("research_hut")
									? "Build a research hut; assign a scholar to accrue research"
									: "Unlock the Research Hut node in the tree first"
							}
						>
							🔬 Research hut
						</button>
						<button
							type="button"
							onClick={() => onPlanBuilding("school")}
							disabled={
								busyAction === "build:school" ||
								!research?.ownedNodeIds?.includes("school")
							}
							className={WOOD_BUTTON}
							title={
								research?.ownedNodeIds?.includes("school")
									? "Build a school; kittens trickle in extra research"
									: "Unlock the School node in the tree first"
							}
						>
							🎓 School
						</button>
						<button
							type="button"
							onClick={() => onPlanBuilding("smithy")}
							disabled={
								busyAction === "build:smithy" ||
								!research?.ownedNodeIds?.includes("smithy")
							}
							className={WOOD_BUTTON}
							title={
								research?.ownedNodeIds?.includes("smithy")
									? "Build a smithy; assign a smith to forge weapons and armor"
									: "Unlock the Smithy node in the tree first"
							}
						>
							⚒️ Smithy
						</button>
						<button
							type="button"
							onClick={() => onPlanBuilding("barracks")}
							disabled={
								busyAction === "build:barracks" ||
								!research?.ownedNodeIds?.includes("barracks")
							}
							className={WOOD_BUTTON}
							title={
								research?.ownedNodeIds?.includes("barracks")
									? "Build a barracks so cats can train into warriors"
									: "Unlock the Barracks node in the tree first"
							}
						>
							🏰 Barracks
						</button>
						<button
							type="button"
							onClick={() => onTrainWarrior()}
							disabled={
								busyAction === "trainWarrior" ||
								!buildings.some(
									(b: { type: string; constructionProgress: number }) =>
										b.type === "barracks" && b.constructionProgress >= 100,
								)
							}
							className={`col-span-2 ${WOOD_BUTTON}`}
							title="Train the sturdiest idle cat into a warrior (needs a barracks)"
						>
							🗡️ Train a warrior
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
							{(["off", "fast", "turbo", "hyper", "ludicrous"] as const).map(
								(preset) => (
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
								),
							)}
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

			{/* Selected cat details */}
			{(() => {
				const cat = selectedCatId
					? cats.find((c: { _id: string }) => c._id === selectedCatId)
					: null;
				if (!cat) {
					return null;
				}
				const catAge = (cat as { ageHours?: number }).ageHours ?? 24;
				const stage = getLifeStage(catAge);
				const stageLabel = STAGE_LABELS[stage];
				const roleXp = (cat as { roleXp?: TradeXp | null }).roleXp ?? null;
				const trades: { key: keyof TradeXp; label: string; icon: string }[] = [
					{ key: "hunter", label: "Hunting", icon: "🏹" },
					{ key: "architect", label: "Building", icon: "🪓" },
					{ key: "ritualist", label: "Ritual", icon: "🔮" },
				];
				const learnedTrades = roleXp
					? trades.filter((trade) => (roleXp[trade.key] ?? 0) > 0)
					: [];
				return (
					<div className={`absolute bottom-14 left-3 z-30 w-64 p-3 ${PANEL}`}>
						<div className="mb-1 flex items-center justify-between">
							<h3 className="font-serif text-sm font-black text-[#3a2712]">
								{cat._id === colony.leaderId ? "👑 " : "🐈 "}
								{cat.name}
							</h3>
							<button
								type="button"
								onClick={() => setSelectedCatId(null)}
								className="rounded px-1.5 text-sm font-bold text-[#6b4a2a] hover:bg-black/10"
								aria-label="Close cat details"
							>
								✕
							</button>
						</div>
						<p className="text-xs font-semibold text-[#4a3319]">
							<span className="text-[#6b4a2a]">{stageLabel}</span>
							{cat.specialization && (
								<span className="ml-1 text-[#6b4a2a]">
									· {cat.specialization}
								</span>
							)}
						</p>
						<p className="text-xs font-semibold text-[#4a3319]">
							{cat.activity === "traveling" && "🧭 Traveling"}
							{cat.activity === "working" && "⚒️ Working"}
							{cat.activity === "returning" && "🏠 Heading home"}
							{(cat.activity === "idle" || !cat.activity) &&
								"😽 Idling around the village"}
						</p>
						{learnedTrades.length > 0 && (
							<div className="mt-1 flex flex-wrap gap-x-2 gap-y-0.5 text-[11px] text-[#4a3319]">
								{learnedTrades.map((trade) => (
									<span key={trade.key}>
										{trade.icon} {trade.label} Lv
										{tradeLevel(roleXp?.[trade.key] ?? 0)}
									</span>
								))}
							</div>
						)}
						{cat.carrying && (
							<p className="text-xs text-[#6b4a2a]">
								Carrying {Math.round(cat.carrying.amount)} {cat.carrying.kind}{" "}
								to the shrine
							</p>
						)}
						<div className="mt-1.5 grid grid-cols-2 gap-x-3 gap-y-0.5 text-[11px] text-[#4a3319]">
							<span>🍖 Hunger {Math.round(cat.needs.hunger)}</span>
							<span>💧 Thirst {Math.round(cat.needs.thirst)}</span>
							<span>💤 Rest {Math.round(cat.needs.rest)}</span>
							<span>❤️ Health {Math.round(cat.needs.health)}</span>
						</div>
					</div>
				);
			})()}

			{/* Upgrade tree modal */}
			{showTree && research && (
				<TreePanel
					research={research}
					busyAction={busyAction}
					onUnlockNode={onUnlockNode}
					onClose={() => setShowTree(false)}
				/>
			)}

			{/* Raid alert — a big defense button while a warband is at the gate */}
			{raidActive && (
				<div className="-translate-x-1/2 absolute bottom-24 left-1/2 z-40 flex flex-col items-center gap-1">
					<span className="rounded-full bg-red-800/90 px-3 py-1 font-serif text-xs font-black uppercase tracking-wider text-amber-100 shadow">
						⚔️ Raiders at the gate — {raiders.length} closing in
					</span>
					<button
						type="button"
						onClick={() => onDefendRaid()}
						className="rounded-lg border-2 border-red-950 bg-gradient-to-b from-red-600 to-red-800 px-6 py-3 font-serif text-lg font-black text-amber-50 shadow-lg hover:from-red-500 hover:to-red-700 active:scale-95"
						title="Rally the defense — clicks wear the raiders down"
					>
						🗡️ Defend the village!
					</button>
				</div>
			)}

			{/* News ticker — latest headline, full story in the Examiner */}
			{events.length > 0 && (
				<a
					href="/game/newspaper"
					className="absolute bottom-3 left-3 z-20 flex max-w-[50vw] items-center gap-2 rounded-md border border-[#5d4024] bg-[#f3e6c8]/95 px-3 py-1.5 text-xs font-semibold text-[#4a3319] shadow-md hover:bg-amber-100"
					title="Read the full story in The Catford Examiner"
				>
					<span className="shrink-0 rounded bg-[#4a3319] px-1.5 py-0.5 font-serif text-[10px] font-black uppercase tracking-wider text-amber-100">
						Examiner
					</span>
					<span className="truncate">
						{(events[0] as { message: string }).message}
					</span>
				</a>
			)}
		</div>
	);
}
