"use client";

import { Application, Rectangle } from "pixi.js";
import { Viewport } from "pixi-viewport";
import { useEffect, useRef } from "react";
import {
	CHUNK_MAX,
	CHUNK_MIN,
	ISO,
	ISO_CONTENT,
} from "@/components/map/constants";
import {
	isoToTile,
	tileDiamondCenter,
	visibleChunksIso,
} from "@/lib/game/isoProjection";
import { chunkKey } from "@/lib/game/mapView";
import type { GatePlacement } from "@/lib/game/villageArea";
import { shrineWorldPosition } from "@/lib/game/villageLayout";
import { ChunkStore } from "./chunkStore";
import {
	LOD_THRESHOLD,
	PixiScene,
	type SceneBuilding,
	type SceneCat,
	type SceneRaider,
	type SceneZone,
} from "./scene";
import { loadMapTextures } from "./textures";
import { buildOrganicVillageView, type OrganicVillageView } from "./tileVisual";

const MIN_SCALE = 0.03;
const MAX_SCALE = 1.6;
const INITIAL_SCALE = 0.45;

export interface PixiViewportStats {
	fps: number;
	scale: number;
	band: "close" | "overview";
	loadedChunks: number;
}

interface PixiViewportLayerProps {
	cats: SceneCat[];
	buildings: SceneBuilding[];
	anchor: { x: number; y: number };
	villageRadius: number;
	claimedTiles?: Array<{ x: number; y: number }>;
	villageGate?: GatePlacement | null;
	zones: SceneZone[];
	now: number;
	raiders: SceneRaider[];
	showInfo: boolean;
	leaderId: string | null;
	selectedCatId: string | null;
	draftCorner: { x: number; y: number } | null;
	onSelectCat: (catId: string) => void;
	onRemoveZone: (zoneId: string) => void;
	onMapTileClick: (tile: { x: number; y: number }) => void;
	onStats?: (stats: PixiViewportStats) => void;
}

interface SceneData {
	cats: SceneCat[];
	buildings: SceneBuilding[];
	anchor: { x: number; y: number };
	villageRadius: number;
	village: OrganicVillageView | null;
	villageKey: string;
	zones: SceneZone[];
	now: number;
	raiders: SceneRaider[];
	showInfo: boolean;
	leaderId: string | null;
	selectedCatId: string | null;
	draftCorner: { x: number; y: number } | null;
	onSelectCat: (catId: string) => void;
	onRemoveZone: (zoneId: string) => void;
	onMapTileClick: (tile: { x: number; y: number }) => void;
}

function villageShapeKey(
	claimedTiles: Array<{ x: number; y: number }> | undefined,
	gate: GatePlacement | null | undefined,
): string {
	const tilesKey =
		claimedTiles?.map((tile) => `${tile.x},${tile.y}`).join(";") ?? "";
	const gateKey = gate ? `${gate.x},${gate.y},${gate.side}` : "";
	return `${tilesKey}|${gateKey}`;
}

export default function PixiViewportLayer(props: PixiViewportLayerProps) {
	const { onStats } = props;
	const hostRef = useRef<HTMLDivElement | null>(null);
	const sceneRef = useRef<PixiScene | null>(null);
	const storeRef = useRef<ChunkStore | null>(null);
	const syncSceneDataRef = useRef<(() => void) | null>(null);
	const refreshVisibleChunksRef = useRef<(() => void) | null>(null);
	const dataRef = useRef<SceneData>({
		cats: [],
		buildings: [],
		anchor: { x: 6, y: 6 },
		villageRadius: 4,
		village: null,
		villageKey: "",
		zones: [],
		now: 0,
		raiders: [],
		showInfo: false,
		leaderId: null,
		selectedCatId: null,
		draftCorner: null,
		onSelectCat: () => undefined,
		onRemoveZone: () => undefined,
		onMapTileClick: () => undefined,
	});

	useEffect(() => {
		const villageKey = villageShapeKey(props.claimedTiles, props.villageGate);
		const previous = dataRef.current;
		dataRef.current = {
			cats: props.cats,
			buildings: props.buildings,
			anchor: props.anchor,
			villageRadius: props.villageRadius,
			village: buildOrganicVillageView(props.claimedTiles, props.villageGate),
			villageKey,
			zones: props.zones,
			now: props.now,
			raiders: props.raiders,
			showInfo: props.showInfo,
			leaderId: props.leaderId,
			selectedCatId: props.selectedCatId,
			draftCorner: props.draftCorner,
			onSelectCat: props.onSelectCat,
			onRemoveZone: props.onRemoveZone,
			onMapTileClick: props.onMapTileClick,
		};
		if (
			previous.anchor.x !== props.anchor.x ||
			previous.anchor.y !== props.anchor.y ||
			previous.villageRadius !== props.villageRadius ||
			previous.villageKey !== villageKey
		) {
			sceneRef.current?.invalidateTileVisuals();
			refreshVisibleChunksRef.current?.();
		}
		syncSceneDataRef.current?.();
	}, [props]);

	useEffect(() => {
		const host = hostRef.current;
		if (!host) return;
		let disposed = false;
		let app: Application | null = null;
		let viewport: Viewport | null = null;
		let scene: PixiScene | null = null;
		let resizeObserver: ResizeObserver | null = null;
		let handleResize: (() => void) | null = null;

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
			vp.eventMode = "static";
			vp.hitArea = new Rectangle(0, 0, ISO_CONTENT.width, ISO_CONTENT.height);
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
					syncSceneDataRef.current?.();
					viewDirty = true;
				},
			});
			storeRef.current = store;

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
			vp.on("pointertap", (event) => {
				if (!dataRef.current.draftCorner && !dataRef.current.onMapTileClick) {
					return;
				}
				const world = vp.toWorld(event.global.x, event.global.y);
				const tile = isoToTile(world.x, world.y, ISO);
				dataRef.current.onMapTileClick({
					x: Math.round(tile.x),
					y: Math.round(tile.y),
				});
			});

			handleResize = () => {
				vp.resize(
					host.clientWidth,
					host.clientHeight,
					ISO_CONTENT.width,
					ISO_CONTENT.height,
				);
				vp.hitArea = new Rectangle(0, 0, ISO_CONTENT.width, ISO_CONTENT.height);
				markDirty();
			};
			if (typeof ResizeObserver !== "undefined") {
				resizeObserver = new ResizeObserver(handleResize);
				resizeObserver.observe(host);
			}
			window.addEventListener("resize", handleResize);

			const syncSceneData = () => {
				if (!scene) return;
				const data = dataRef.current;
				scene.setVillage(data.anchor, data.villageRadius, data.village);
				scene.sync({
					buildings: data.buildings,
					zones: data.zones,
					raiders: data.raiders,
					showInfo: data.showInfo,
					now: data.now,
					leaderId: data.leaderId,
					selectedCatId: data.selectedCatId,
					draftCorner: data.draftCorner,
					onSelectCat: data.onSelectCat,
					onRemoveZone: data.onRemoveZone,
				});
				scene.setCatOverlays(
					data.cats,
					data.leaderId,
					data.selectedCatId,
					data.onSelectCat,
				);
			};
			syncSceneDataRef.current = syncSceneData;

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
				for (const ch of visible) {
					const key = chunkKey(ch);
					const tiles = store.get(ch.chunkX, ch.chunkY);
					if (tiles) scene.setChunkIfNew(key, tiles);
				}
				syncSceneData();
			};
			refreshVisibleChunksRef.current = updateView;

			let hudClock = 0;
			application.ticker.add((ticker) => {
				if (viewDirty) {
					viewDirty = false;
					updateView();
				}
				const data = dataRef.current;
				scene?.updateCats(data.cats);
				scene?.setCatOverlays(
					data.cats,
					data.leaderId,
					data.selectedCatId,
					data.onSelectCat,
				);
				scene?.tickCats(ticker);
				hudClock += ticker.deltaMS;
				if (hudClock >= 400) {
					hudClock = 0;
					onStats?.({
						fps: Math.round(application.ticker.FPS),
						scale: viewport?.scale.x ?? 0,
						band:
							(viewport?.scale.x ?? 1) < LOD_THRESHOLD ? "overview" : "close",
						loadedChunks: scene?.loadedChunkCount() ?? 0,
					});
				}
			});

			viewDirty = true;
		};

		void boot();

		return () => {
			disposed = true;
			sceneRef.current = null;
			storeRef.current = null;
			syncSceneDataRef.current = null;
			refreshVisibleChunksRef.current = null;
			resizeObserver?.disconnect();
			if (handleResize) window.removeEventListener("resize", handleResize);
			scene?.destroy();
			app?.destroy(true, { children: true });
		};
	}, [onStats]);

	return <div ref={hostRef} className="absolute inset-0" />;
}
