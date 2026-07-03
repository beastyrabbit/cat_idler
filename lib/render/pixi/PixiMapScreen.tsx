"use client";

/**
 * PixiJS renderer spike — a side-by-side alternative to the DOM `MapScreen`,
 * mounted at /game/pixi. Renders the SAME live world (SSE dashboard via
 * `useGameDashboard`, chunks via `/api/game/chunks`) with the SAME projection
 * math, into a single WebGL canvas driven by `pixi-viewport`.
 *
 * The point of the spike (docs/ENGINE_FRONTEND.md §2): stay correct and 60fps at
 * FAR zoom-out, where the DOM renderer's node explosion + oversized compositing
 * layer stalls for seconds. Feature parity with the DOM HUD/zones is explicitly
 * out of scope; terrain, fence, buildings, cats, fog, and pan/zoom are in.
 *
 * This module is browser-only and must be loaded behind `dynamic(..., { ssr:false })`.
 */

import { Application } from "pixi.js";
import { Viewport } from "pixi-viewport";
import { useEffect, useRef, useState } from "react";
import {
	CHUNK_MAX,
	CHUNK_MIN,
	ISO,
	ISO_CONTENT,
} from "@/components/map/constants";
import { useGameDashboard } from "@/hooks/useGameDashboard";
import { tileDiamondCenter, visibleChunksIso } from "@/lib/game/isoProjection";
import { chunkKey } from "@/lib/game/mapView";
import { shrineWorldPosition } from "@/lib/game/villageLayout";
import { ChunkStore } from "./chunkStore";
import {
	LOD_THRESHOLD,
	PixiScene,
	type SceneBuilding,
	type SceneCat,
} from "./scene";
import { loadMapTextures } from "./textures";

const MIN_SCALE = 0.03;
const MAX_SCALE = 1.6;
const INITIAL_SCALE = 0.45;

interface HudStats {
	fps: number;
	scale: number;
	band: "close" | "overview";
	loadedChunks: number;
}

export default function PixiMapScreen() {
	const dashboard = useGameDashboard();
	const { cats, buildings, anchor } = dashboard;

	const hostRef = useRef<HTMLDivElement | null>(null);
	const sceneRef = useRef<PixiScene | null>(null);
	const storeRef = useRef<ChunkStore | null>(null);
	// Latest dashboard data, read by the imperative Pixi loop without re-mounting.
	const dataRef = useRef({
		cats: cats as SceneCat[],
		buildings: buildings as SceneBuilding[],
		anchor,
	});
	dataRef.current = {
		cats: cats as SceneCat[],
		buildings: buildings as SceneBuilding[],
		anchor,
	};

	const [ready, setReady] = useState(false);
	const [hud, setHud] = useState<HudStats>({
		fps: 0,
		scale: INITIAL_SCALE,
		band: "close",
		loadedChunks: 0,
	});

	// One-time Pixi Application + Viewport + scene lifecycle.
	useEffect(() => {
		const host = hostRef.current;
		if (!host) return;
		let disposed = false;
		let app: Application | null = null;
		let viewport: Viewport | null = null;
		let scene: PixiScene | null = null;

		const boot = async () => {
			const application = new Application();
			await application.init({
				resizeTo: host,
				background: "#141c12",
				antialias: false,
				preference: "webgl",
				autoDensity: true,
				resolution: window.devicePixelRatio || 1,
			});
			if (disposed) {
				application.destroy(true);
				return;
			}
			app = application;
			host.appendChild(application.canvas);

			const vp = new Viewport({
				screenWidth: host.clientWidth,
				screenHeight: host.clientHeight,
				worldWidth: ISO_CONTENT.width,
				worldHeight: ISO_CONTENT.height,
				events: application.renderer.events,
				ticker: application.ticker,
			});
			vp.drag().pinch().wheel({ smooth: 3 }).decelerate();
			vp.clampZoom({ minScale: MIN_SCALE, maxScale: MAX_SCALE });
			application.stage.addChild(vp);
			viewport = vp;

			const textures = await loadMapTextures();
			if (disposed) {
				application.destroy(true);
				return;
			}

			scene = new PixiScene(vp, textures);
			sceneRef.current = scene;
			const store = new ChunkStore({
				onLoaded: (key, tiles) => {
					scene?.setChunk(key, tiles);
					viewDirty = true;
				},
			});
			storeRef.current = store;

			// Centre on the shrine at the initial zoom.
			const shrine = shrineWorldPosition();
			const c = tileDiamondCenter(shrine.x, shrine.y, ISO);
			vp.setZoom(INITIAL_SCALE, false);
			vp.moveCenter(c.x, c.y);

			let viewDirty = true;
			const markDirty = () => {
				viewDirty = true;
			};
			vp.on("moved", markDirty);
			vp.on("zoomed", markDirty);
			vp.on("moved-end", markDirty);

			const updateView = () => {
				if (!viewport || !scene || !store) return;
				const visible = visibleChunksIso(
					{
						tx: viewport.x,
						ty: viewport.y,
						scale: viewport.scale.x,
						width: host.clientWidth,
						height: host.clientHeight,
					},
					ISO,
				).filter(
					(ch) =>
						ch.chunkX >= CHUNK_MIN &&
						ch.chunkX <= CHUNK_MAX &&
						ch.chunkY >= CHUNK_MIN &&
						ch.chunkY <= CHUNK_MAX,
				);
				store.ensure(visible);
				// Feed any already-cached tiles straight into the scene.
				for (const ch of visible) {
					const key = chunkKey(ch);
					const tiles = store.get(ch.chunkX, ch.chunkY);
					if (tiles) scene.setChunkIfNew(key, tiles);
				}
				scene.setVillage(dataRef.current.anchor ?? { x: 6, y: 6 }, 4);
				scene.sync(dataRef.current.buildings);
			};

			let hudClock = 0;
			application.ticker.add((ticker) => {
				if (viewDirty) {
					viewDirty = false;
					updateView();
				}
				scene?.updateCats(dataRef.current.cats);
				scene?.tickCats(ticker);
				hudClock += ticker.deltaMS;
				if (hudClock >= 400) {
					hudClock = 0;
					setHud({
						fps: Math.round(application.ticker.FPS),
						scale: viewport?.scale.x ?? 0,
						band:
							(viewport?.scale.x ?? 1) < LOD_THRESHOLD ? "overview" : "close",
						loadedChunks: scene?.loadedChunkCount() ?? 0,
					});
				}
			});

			setReady(true);
			viewDirty = true;
		};

		void boot();

		return () => {
			disposed = true;
			sceneRef.current = null;
			storeRef.current = null;
			scene?.destroy();
			if (app) {
				app.destroy(true, { children: true });
			}
		};
	}, []);

	return (
		<div className="relative h-dvh w-full overflow-hidden bg-[#141c12]">
			<div ref={hostRef} className="absolute inset-0" />

			{!ready && (
				<div className="absolute inset-0 flex items-center justify-center text-amber-100">
					<p className="animate-pulse font-serif text-lg">
						Booting the WebGL renderer…
					</p>
				</div>
			)}

			{/* Minimal spike HUD — perf readout + the comparison link. */}
			<div className="pointer-events-none absolute left-3 top-3 z-10 rounded-md border border-[#5d4024] bg-[#0d130b]/85 px-3 py-2 font-mono text-xs text-amber-100 shadow-lg">
				<div className="font-bold text-amber-300">⚡ PixiJS spike</div>
				<div>fps {hud.fps}</div>
				<div>zoom {hud.scale.toFixed(3)}</div>
				<div>
					LOD{" "}
					<span
						className={
							hud.band === "overview" ? "text-sky-300" : "text-emerald-300"
						}
					>
						{hud.band}
					</span>
				</div>
				<div>chunks {hud.loadedChunks}</div>
				<div>cats {cats.length}</div>
			</div>

			<div className="absolute right-3 top-3 z-10 flex gap-2">
				<a
					href="/game"
					className="rounded-md border border-[#5d4024] bg-[#f3e6c8] px-3 py-1.5 text-sm font-bold text-[#4a3319] shadow hover:bg-amber-100"
					title="Compare against the DOM renderer"
				>
					↔ DOM /game
				</a>
			</div>

			<div className="pointer-events-none absolute bottom-3 left-1/2 z-10 -translate-x-1/2 rounded-md border border-[#5d4024] bg-[#0d130b]/85 px-3 py-1.5 text-center text-[11px] text-amber-100/80 shadow">
				drag to pan · wheel / pinch to zoom · zoom out past{" "}
				{LOD_THRESHOLD.toFixed(2)} for the chunk-LOD overview
			</div>
		</div>
	);
}
