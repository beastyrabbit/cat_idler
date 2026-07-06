/**
 * PixiJS map scene for the renderer spike.
 *
 * Draws the live world into a `pixi-viewport` using the SAME projection math and
 * art the DOM renderer uses. The scene is imperative and framework-free; the
 * React shell (PixiMapScreen) owns the Application/Viewport lifecycle and feeds
 * this scene data.
 *
 * LOD (the actual fix for the DOM renderer's far-zoom collapse, §2 of
 * docs/ENGINE_FRONTEND.md):
 *   - Close band (scale >= LOD_THRESHOLD): individual tile/fence/building sprites,
 *     mounted per chunk and unmounted when the chunk leaves the padded viewport,
 *     so the live sprite count tracks what's on screen, not the whole map. Pixi
 *     batches these into a few GPU draw calls.
 *   - Overview band (scale < LOD_THRESHOLD): one flat diamond per loaded chunk,
 *     tinted by its dominant biome — 625 quads instead of 90k nodes. This is what
 *     keeps the whole-map overview at 60fps where the DOM renderer stalls.
 */

import {
	Container,
	Graphics,
	Rectangle,
	Sprite,
	Texture,
	type Ticker,
} from "pixi.js";
import type { Viewport } from "pixi-viewport";
import {
	BUILDING_SPRITE_FALLBACK,
	BUILDING_SPRITES,
	ISO,
	TILE_COLORS,
} from "@/components/map/constants";
import {
	tileDiamondCenter,
	tileToIso,
	zIndexFor,
} from "@/lib/game/isoProjection";
import { getLifeStage } from "@/lib/game/lifeSim";
import { colonyToWorld } from "@/lib/game/villageLayout";
import type { WorldTile } from "@/types/game";
import { CAT_SHEET_URL } from "./textures";
import {
	computeFogBrightness,
	fenceSprites,
	isExplored,
	type OrganicVillageView,
	SPRITE_H,
	SPRITE_TOP_OFFSET,
	SPRITE_W,
	tileGround,
} from "./tileVisual";
import { tileTint } from "./tint";

/** Below this viewport scale the scene switches to per-chunk overview quads. */
export const LOD_THRESHOLD = 0.2;

/** Chunks this many tiles outside the viewport stay mounted (pan hysteresis). */
const MOUNT_MARGIN_TILES = 14;

/** Cat facings on the sheet: 8 groups of 4 frames, 32x32 cells. */
const CAT_CELL = 32;
const CAT_FRAMES = 4;
const CAT_WALK_FPS = 6;

/** Minimal cat shape the scene reads (subset of the dashboard row). */
export interface SceneCat {
	_id: string;
	position: { map: "colony" | "world"; x: number; y: number };
	activity?: "idle" | "traveling" | "working" | "returning" | null;
	destination?: { x: number; y: number } | null;
	ageHours?: number;
}

export interface SceneBuilding {
	_id: string;
	type: string;
	constructionProgress: number;
	worldPosition: { x: number; y: number };
}

interface CatView {
	container: Container;
	sprite: Sprite;
	texture: Texture;
	cur: { x: number; y: number };
	target: { x: number; y: number };
	group: number;
	frame: number;
	frameClock: number;
	walking: boolean;
	scale: number;
}

/** Parse a `#rrggbb` string to a 0xRRGGBB number (fallback mid-grey). */
function hexColor(hex: string | undefined): number {
	if (!hex) return 0x8aa37b;
	return Number.parseInt(hex.replace("#", ""), 16);
}

export class PixiScene {
	private readonly viewport: Viewport;
	private readonly textures: Map<string, Texture>;

	/** Close-band terrain/fence/building sprites, globally z-sorted. */
	private readonly groundLayer = new Container();
	/** Overview-band per-chunk quads. */
	private readonly overviewLayer = new Container();
	/** Cats, above ground, interpolated every frame. */
	private readonly catLayer = new Container();

	private readonly mountedChunks = new Map<string, Sprite[]>();
	private readonly overviewQuads = new Map<string, Graphics>();
	private readonly cats = new Map<string, CatView>();
	private readonly spritePool: Sprite[] = [];

	private tilesByChunk = new Map<string, WorldTile[]>();
	private anchor = { x: 6, y: 6 };
	private ringRadius = 4;
	private village: OrganicVillageView | null = null;
	private band: "close" | "overview" = "close";

	constructor(viewport: Viewport, textures: Map<string, Texture>) {
		this.viewport = viewport;
		this.textures = textures;
		this.groundLayer.sortableChildren = true;
		this.catLayer.sortableChildren = true;
		this.groundLayer.cullableChildren = true;
		this.viewport.addChild(this.overviewLayer);
		this.viewport.addChild(this.groundLayer);
		this.viewport.addChild(this.catLayer);
	}

	setVillage(
		anchor: { x: number; y: number },
		ringRadius: number,
		village: OrganicVillageView | null = null,
	): void {
		this.anchor = anchor;
		this.ringRadius = ringRadius;
		this.village = village;
	}

	/** Replace the tile data for a chunk (from the chunk store). */
	setChunk(key: string, tiles: WorldTile[]): void {
		this.tilesByChunk.set(key, tiles);
		// A reloaded chunk may carry fresh fog/roads — rebuild it if mounted.
		if (this.mountedChunks.has(key)) {
			this.unmountChunk(key);
		}
		this.overviewQuads.get(key)?.destroy();
		this.overviewQuads.delete(key);
	}

	/**
	 * Adopt a chunk's tiles only if they are new to the scene (a different array
	 * than the one already held). Lets the view loop feed already-cached chunks
	 * without thrashing mounted sprites every frame.
	 */
	setChunkIfNew(key: string, tiles: WorldTile[]): void {
		if (this.tilesByChunk.get(key) === tiles) {
			return;
		}
		this.setChunk(key, tiles);
	}

	/** Number of chunks whose tiles the scene currently holds. */
	loadedChunkCount(): number {
		return this.tilesByChunk.size;
	}

	/** Rebuild mounted tile visuals after village/fog geometry changes. */
	invalidateTileVisuals(): void {
		for (const key of [...this.mountedChunks.keys()]) {
			this.unmountChunk(key);
		}
		for (const quad of this.overviewQuads.values()) {
			quad.destroy();
		}
		this.overviewQuads.clear();
	}

	private texture(url: string): Texture {
		return this.textures.get(url) ?? Texture.EMPTY;
	}

	private acquireSprite(): Sprite {
		const s = this.spritePool.pop();
		if (s) {
			s.visible = true;
			s.alpha = 1;
			s.tint = 0xffffff;
			return s;
		}
		return new Sprite();
	}

	private releaseSprite(s: Sprite): void {
		s.parent?.removeChild(s);
		this.spritePool.push(s);
	}

	/** The viewport's visible world rectangle, padded for tall sprites. */
	private paddedBounds(): Rectangle {
		const b = this.viewport.getVisibleBounds();
		const padX = ISO.tileWidth + MOUNT_MARGIN_TILES * ISO.tileWidth;
		const padTop = SPRITE_TOP_OFFSET + MOUNT_MARGIN_TILES * ISO.tileHeight;
		const padBottom = ISO.tileHeight + MOUNT_MARGIN_TILES * ISO.tileHeight;
		return new Rectangle(
			b.x - padX,
			b.y - padTop,
			b.width + padX * 2,
			b.height + padTop + padBottom,
		);
	}

	/**
	 * Reconcile the scene to the current viewport. Call on view change, chunk
	 * load, or data update. Picks the LOD band and mounts/unmounts accordingly.
	 */
	sync(buildings: SceneBuilding[]): void {
		const scale = this.viewport.scale.x;
		const band = scale < LOD_THRESHOLD ? "overview" : "close";
		if (band !== this.band) {
			this.band = band;
			this.groundLayer.visible = band === "close";
			this.catLayer.visible = band === "close";
			this.overviewLayer.visible = band === "overview";
		}
		if (band === "overview") {
			this.syncOverview();
		} else {
			this.syncClose(buildings);
		}
	}

	private chunkInBounds(key: string, bounds: Rectangle): boolean {
		const [cx, cy] = key.split(",").map(Number);
		const originX = cx * ISO.chunkSize;
		const originY = cy * ISO.chunkSize;
		// The chunk's iso footprint: use its four corner tiles' diamond centers.
		let minX = Number.POSITIVE_INFINITY;
		let maxX = Number.NEGATIVE_INFINITY;
		let minY = Number.POSITIVE_INFINITY;
		let maxY = Number.NEGATIVE_INFINITY;
		for (const [dx, dy] of [
			[0, 0],
			[ISO.chunkSize - 1, 0],
			[0, ISO.chunkSize - 1],
			[ISO.chunkSize - 1, ISO.chunkSize - 1],
		]) {
			const p = tileToIso(originX + dx, originY + dy, ISO);
			minX = Math.min(minX, p.left);
			maxX = Math.max(maxX, p.left + ISO.tileWidth);
			minY = Math.min(minY, p.top - SPRITE_TOP_OFFSET);
			maxY = Math.max(maxY, p.top + ISO.tileHeight);
		}
		return !(
			maxX < bounds.x ||
			minX > bounds.x + bounds.width ||
			maxY < bounds.y ||
			minY > bounds.y + bounds.height
		);
	}

	private syncClose(buildings: SceneBuilding[]): void {
		const bounds = this.paddedBounds();
		// Unmount chunks that scrolled out.
		for (const key of [...this.mountedChunks.keys()]) {
			if (!this.chunkInBounds(key, bounds)) {
				this.unmountChunk(key);
			}
		}
		// Mount visible chunks that have data.
		for (const [key, tiles] of this.tilesByChunk) {
			if (this.mountedChunks.has(key) || !this.chunkInBounds(key, bounds)) {
				continue;
			}
			this.mountChunk(key, tiles);
		}
		this.syncBuildings(buildings, bounds);
	}

	private mountChunk(key: string, tiles: WorldTile[]): void {
		const fog = computeFogBrightness(
			tiles,
			this.anchor,
			this.ringRadius,
			this.village,
		);
		const sprites: Sprite[] = [];
		for (const tile of tiles) {
			const { left, top } = tileToIso(tile.x, tile.y, ISO);
			const tileZ = zIndexFor(tile.x, tile.y, "tile", ISO);
			const objZ = zIndexFor(tile.x, tile.y, "object", ISO);
			const dim = fog.get(tile._id) ?? 1;
			const ground = tileGround(
				tile,
				this.anchor,
				this.ringRadius,
				this.village,
			);
			// Grass underlay for standalone tree/stump sprites.
			if (ground.base) {
				sprites.push(
					this.placeSprite(
						ground.base,
						left,
						top - SPRITE_TOP_OFFSET,
						tileZ,
						tileTint(undefined, dim),
					),
				);
			}
			sprites.push(
				this.placeSprite(
					ground.src,
					left,
					top - SPRITE_TOP_OFFSET,
					tileZ,
					tileTint(ground.filter, dim),
				),
			);
			// Fence ring — only where the tile is explored (matches TileLayer).
			if (dim >= 1) {
				for (const fence of fenceSprites(
					tile,
					this.anchor,
					this.ringRadius,
					this.village,
				)) {
					sprites.push(
						this.placeSprite(
							fence.src,
							left + fence.ox,
							top - SPRITE_TOP_OFFSET + fence.oy,
							objZ,
							0xffffff,
						),
					);
				}
			}
		}
		this.mountedChunks.set(key, sprites);
	}

	private placeSprite(
		url: string,
		x: number,
		y: number,
		z: number,
		tint: number,
	): Sprite {
		const s = this.acquireSprite();
		s.texture = this.texture(url);
		s.width = SPRITE_W;
		s.height = SPRITE_H;
		s.position.set(x, y);
		s.zIndex = z;
		s.tint = tint;
		this.groundLayer.addChild(s);
		return s;
	}

	private unmountChunk(key: string): void {
		const sprites = this.mountedChunks.get(key);
		if (!sprites) return;
		for (const s of sprites) {
			this.releaseSprite(s);
		}
		this.mountedChunks.delete(key);
	}

	// --- Buildings (close band): rebuilt each sync from the live list. ---
	private buildingSprites: Sprite[] = [];
	private syncBuildings(buildings: SceneBuilding[], bounds: Rectangle): void {
		for (const s of this.buildingSprites) {
			this.releaseSprite(s);
		}
		this.buildingSprites = [];
		for (const b of buildings) {
			const { left, top } = tileToIso(
				b.worldPosition.x,
				b.worldPosition.y,
				ISO,
			);
			if (
				left + SPRITE_W < bounds.x ||
				left > bounds.x + bounds.width ||
				top < bounds.y ||
				top - SPRITE_TOP_OFFSET > bounds.y + bounds.height
			) {
				continue;
			}
			const url = BUILDING_SPRITES[b.type] ?? BUILDING_SPRITE_FALLBACK;
			const s = this.placeSprite(
				url,
				left,
				top - SPRITE_TOP_OFFSET,
				zIndexFor(b.worldPosition.x, b.worldPosition.y, "object", ISO),
				0xffffff,
			);
			s.alpha = b.constructionProgress < 100 ? 0.45 : 1;
			this.buildingSprites.push(s);
		}
	}

	// --- Overview band: one dominant-biome diamond per loaded chunk. ---
	private syncOverview(): void {
		for (const [key, tiles] of this.tilesByChunk) {
			if (this.overviewQuads.has(key)) continue;
			this.overviewQuads.set(key, this.buildOverviewQuad(key, tiles));
		}
	}

	private buildOverviewQuad(key: string, tiles: WorldTile[]): Graphics {
		const [cx, cy] = key.split(",").map(Number);
		const originX = cx * ISO.chunkSize;
		const originY = cy * ISO.chunkSize;
		// Dominant biome among explored tiles → colour; else deep fog.
		const counts = new Map<string, number>();
		let explored = 0;
		for (const t of tiles) {
			if (t.pathWear > 62 || this.villageKnown(t)) {
				counts.set(t.type, (counts.get(t.type) ?? 0) + 1);
				explored += 1;
			}
		}
		let dominant = "field";
		let best = 0;
		for (const [type, n] of counts) {
			if (n > best) {
				best = n;
				dominant = type;
			}
		}
		const color = explored > 0 ? hexColor(TILE_COLORS[dominant]) : 0x141c12;
		const alpha = explored > 0 ? 1 : 0.9;
		// Diamond through the four corner-tile centres.
		const c = (dx: number, dy: number) =>
			tileDiamondCenter(originX + dx, originY + dy, ISO);
		const nw = c(0, 0);
		const ne = c(ISO.chunkSize - 1, 0);
		const se = c(ISO.chunkSize - 1, ISO.chunkSize - 1);
		const sw = c(0, ISO.chunkSize - 1);
		const g = new Graphics();
		g.poly([nw.x, nw.y, ne.x, ne.y, se.x, se.y, sw.x, sw.y]).fill({
			color,
			alpha,
		});
		g.zIndex = originX + originY;
		this.overviewLayer.addChild(g);
		return g;
	}

	private villageKnown(tile: WorldTile): boolean {
		return isExplored(tile, this.anchor, this.ringRadius, this.village);
	}

	// --- Cats: pooled, interpolated toward their live position each frame. ---
	updateCats(cats: SceneCat[]): void {
		const seen = new Set<string>();
		for (const cat of cats) {
			seen.add(cat._id);
			const world =
				cat.position.map === "world"
					? { x: cat.position.x, y: cat.position.y }
					: colonyToWorld(cat.position);
			let view = this.cats.get(cat._id);
			if (!view) {
				view = this.createCat(world);
				this.cats.set(cat._id, view);
			}
			view.target = world;
			// Snap on big jumps (teleport / accelerated multi-tile ticks).
			if (
				Math.abs(view.target.x - view.cur.x) > 3 ||
				Math.abs(view.target.y - view.cur.y) > 3
			) {
				view.cur = { ...world };
			}
			view.walking =
				cat.activity === "traveling" || cat.activity === "returning";
			const legDx = cat.destination ? cat.destination.x - world.x : 0;
			const legDy = cat.destination ? cat.destination.y - world.y : 0;
			view.group = view.walking
				? Math.abs(legDx) > 0.01
					? directionGroup(Math.sign(legDx), 0)
					: directionGroup(0, Math.sign(legDy))
				: 0;
			view.scale = stageScale(getLifeStage(cat.ageHours ?? 24));
		}
		// Drop cats that died / left the roster.
		for (const [id, view] of this.cats) {
			if (!seen.has(id)) {
				view.container.destroy({ children: true });
				view.texture.destroy();
				this.cats.delete(id);
			}
		}
	}

	private createCat(world: { x: number; y: number }): CatView {
		const sheet = this.texture(CAT_SHEET_URL);
		const texture = new Texture({
			source: sheet.source,
			frame: new Rectangle(0, 0, CAT_CELL, CAT_CELL),
		});
		const sprite = new Sprite(texture);
		sprite.anchor.set(0.5, 0.9);
		const container = new Container();
		container.addChild(sprite);
		this.catLayer.addChild(container);
		return {
			container,
			sprite,
			texture,
			cur: { ...world },
			target: { ...world },
			group: 0,
			frame: 0,
			frameClock: 0,
			walking: false,
			scale: 1,
		};
	}

	/** Per-frame cat interpolation + walk animation (installed on the ticker). */
	tickCats(ticker: Ticker): void {
		if (this.band !== "close") return;
		const dt = ticker.deltaMS / 1000;
		// ~1s to close the gap to the live position (matches the 1Hz sim glide).
		const lerp = Math.min(1, dt / 1);
		for (const view of this.cats.values()) {
			view.cur.x += (view.target.x - view.cur.x) * lerp;
			view.cur.y += (view.target.y - view.cur.y) * lerp;
			const center = tileDiamondCenter(view.cur.x, view.cur.y, ISO);
			view.container.position.set(center.x, center.y);
			view.container.zIndex = zIndexFor(
				Math.round(view.cur.x),
				Math.round(view.cur.y),
				"object",
				ISO,
			);
			const px = CAT_CELL * view.scale * 1.4;
			view.sprite.width = px;
			view.sprite.height = px;
			if (view.walking) {
				view.frameClock += dt;
				if (view.frameClock >= 1 / CAT_WALK_FPS) {
					view.frameClock = 0;
					view.frame = (view.frame + 1) % CAT_FRAMES;
				}
			} else {
				view.frame = 0;
			}
			// Pixi v8: mutate the frame rect in place, then refresh UVs.
			const fx = view.group * CAT_FRAMES * CAT_CELL + view.frame * CAT_CELL;
			if (view.texture.frame.x !== fx) {
				view.texture.frame.x = fx;
				view.texture.frame.y = 0;
				view.texture.frame.width = CAT_CELL;
				view.texture.frame.height = CAT_CELL;
				view.texture.updateUvs();
			}
		}
	}

	destroy(): void {
		for (const view of this.cats.values()) {
			view.texture.destroy();
		}
		this.cats.clear();
		this.groundLayer.destroy({ children: true });
		this.overviewLayer.destroy({ children: true });
		this.catLayer.destroy({ children: true });
	}
}

/** Sheet facing group (S, SW, W, NW, N, NE, E, SE) for a screen move vector. */
function directionGroup(dx: number, dy: number): number {
	const sx = dx - dy;
	const sy = (dx + dy) / 2;
	const angle = (Math.atan2(sy, sx) * 180) / Math.PI;
	return Math.round(((((angle - 90) % 360) + 360) % 360) / 45) % 8;
}

function stageScale(stage: string): number {
	if (stage === "kitten") return 0.9;
	if (stage === "young") return 1.15;
	return 1.4;
}
