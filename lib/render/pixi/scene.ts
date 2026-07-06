import {
	Container,
	Graphics,
	Rectangle,
	Sprite,
	Text,
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
import { lodBandForScale, LOD_THRESHOLD } from "./lod";
import { CAT_SHEET_URL, RAIDER_SHEET_URL } from "./textures";
import {
	computeFogBrightness,
	computeRoadSprites,
	fenceSprites,
	isExplored,
	type OrganicVillageView,
	SPRITE_H,
	SPRITE_TOP_OFFSET,
	SPRITE_W,
	tileGround,
} from "./tileVisual";
import { tileTint } from "./tint";

export { LOD_THRESHOLD } from "./lod";

const MOUNT_MARGIN_TILES = 14;
const CAT_CELL = 32;
const CAT_FRAMES = 4;
const CAT_WALK_FPS = 6;
const JUMP_SNAP_TILES = 3;
const RICH_FOOD = 35;
const RICH_HERBS = 12;

const ZONE_COLOR = {
	avoid: 0xdc3c3c,
	gather: 0x3cb45a,
} as const;

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

const WORK_ICONS: Record<string, string> = {
	hunt_expedition: "🏹",
	build_house: "🪓",
	ritual: "🔮",
	hunter: "🏹",
	architect: "🪓",
	ritualist: "🔮",
	warrior: "⚔️",
	train_warrior: "⚔️",
};

const STAGE_GLYPH: Record<string, string | undefined> = {
	kitten: "🍼",
	elder: "🧓",
};

export interface SceneCat {
	_id: string;
	name: string;
	position: { map: "colony" | "world"; x: number; y: number };
	currentTask: string | null;
	activity?: "idle" | "traveling" | "working" | "returning" | null;
	destination?: { x: number; y: number } | null;
	carrying?: { kind: "food" | "blessings"; amount: number } | null;
	specialization?: "hunter" | "architect" | "ritualist" | "warrior" | null;
	ageHours?: number;
}

export interface SceneBuilding {
	_id: string;
	type: string;
	level: number;
	constructionProgress: number;
	worldPosition: { x: number; y: number };
}

export interface SceneZone {
	_id: string;
	kind: "avoid" | "gather";
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	expiresAt: number;
}

export interface SceneRaider {
	_id: string;
	position: { x: number; y: number };
	hp: number;
	strength: number;
	status: "advancing" | "engaging" | "retreating" | "dead";
}

export interface SceneSyncOptions {
	buildings: SceneBuilding[];
	zones: SceneZone[];
	raiders: SceneRaider[];
	showInfo: boolean;
	now: number;
	leaderId: string | null;
	selectedCatId: string | null;
	draftCorner: { x: number; y: number } | null;
	onSelectCat?: (catId: string) => void;
	onRemoveZone?: (zoneId: string) => void;
}

interface CatView {
	container: Container;
	sprite: Sprite;
	texture: Texture;
	label: Text;
	badge: Text;
	carry: Text;
	leader: Text;
	work: Text;
	selection: Graphics;
	cur: { x: number; y: number };
	target: { x: number; y: number };
	group: number;
	frame: number;
	frameClock: number;
	walking: boolean;
	scale: number;
}

interface RaiderView {
	container: Container;
	sprite: Sprite;
	texture: Texture;
	icon: Text;
	hpBack: Graphics;
	hpFill: Graphics;
}

type ChunkRenderable = Sprite | Text;

function hexColor(hex: string | undefined): number {
	if (!hex) return 0x8aa37b;
	return Number.parseInt(hex.replace("#", ""), 16);
}

function spreadOffset(id: string, xScale = 0.55, yScale = 0.5) {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const ux = ((hash >>> 4) % 100) / 100;
	const uy = ((hash >>> 12) % 100) / 100;
	return {
		x: (ux - 0.5) * ISO.tileWidth * xScale,
		y: (uy - 0.5) * ISO.tileHeight * yScale,
	};
}

function makeText(text: string, size: number, fill = 0xffffff): Text {
	const t = new Text({
		text,
		style: {
			fontFamily: "Arial, sans-serif",
			fontSize: size,
			fontWeight: "700",
			fill,
			stroke: { color: 0x000000, width: 3 },
		},
	});
	t.anchor.set(0.5);
	return t;
}

function drawDiamond(
	g: Graphics,
	x: number,
	y: number,
	color: number,
	alpha: number,
) {
	g.clear();
	g.poly([
		x + ISO.tileWidth / 2,
		y,
		x + ISO.tileWidth,
		y + ISO.tileHeight / 2,
		x + ISO.tileWidth / 2,
		y + ISO.tileHeight,
		x,
		y + ISO.tileHeight / 2,
	]).fill({ color, alpha });
}

export class PixiScene {
	private readonly viewport: Viewport;
	private readonly textures: Map<string, Texture>;

	private readonly overviewLayer = new Container();
	private readonly groundLayer = new Container();
	private readonly zoneLayer = new Container();
	private readonly buildingLayer = new Container();
	private readonly catLayer = new Container();
	private readonly raiderLayer = new Container();

	private readonly mountedChunks = new Map<string, ChunkRenderable[]>();
	private readonly overviewQuads = new Map<string, Graphics>();
	private readonly cats = new Map<string, CatView>();
	private readonly raiders = new Map<string, RaiderView>();
	private readonly spritePool: Sprite[] = [];
	private readonly zoneGraphics: Array<Graphics | Text> = [];
	private readonly buildingSprites: Sprite[] = [];
	private readonly buildingOverlays: Array<Graphics | Text> = [];

	private tilesByChunk = new Map<string, WorldTile[]>();
	private anchor = { x: 6, y: 6 };
	private ringRadius = 4;
	private village: OrganicVillageView | null = null;
	private band = lodBandForScale(1);
	private showInfo = false;

	constructor(viewport: Viewport, textures: Map<string, Texture>) {
		this.viewport = viewport;
		this.textures = textures;
		this.groundLayer.sortableChildren = true;
		this.buildingLayer.sortableChildren = true;
		this.catLayer.sortableChildren = true;
		this.raiderLayer.sortableChildren = true;
		this.groundLayer.cullableChildren = true;
		this.viewport.addChild(this.overviewLayer);
		this.viewport.addChild(this.groundLayer);
		this.viewport.addChild(this.zoneLayer);
		this.viewport.addChild(this.buildingLayer);
		this.viewport.addChild(this.catLayer);
		this.viewport.addChild(this.raiderLayer);
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

	setChunk(key: string, tiles: WorldTile[]): void {
		this.tilesByChunk.set(key, tiles);
		if (this.mountedChunks.has(key)) {
			this.unmountChunk(key);
		}
		this.overviewQuads.get(key)?.destroy();
		this.overviewQuads.delete(key);
	}

	setChunkIfNew(key: string, tiles: WorldTile[]): void {
		if (this.tilesByChunk.get(key) === tiles) return;
		this.setChunk(key, tiles);
	}

	loadedChunkCount(): number {
		return this.tilesByChunk.size;
	}

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
			s.filters = null;
			return s;
		}
		return new Sprite();
	}

	private releaseSprite(s: Sprite): void {
		s.parent?.removeChild(s);
		this.spritePool.push(s);
	}

	private releaseChunkRenderable(renderable: ChunkRenderable): void {
		if (renderable instanceof Sprite) {
			this.releaseSprite(renderable);
			return;
		}
		renderable.parent?.removeChild(renderable);
		renderable.destroy();
	}

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

	sync(options: SceneSyncOptions): void {
		if (options.showInfo !== this.showInfo) {
			this.showInfo = options.showInfo;
			this.invalidateTileVisuals();
		}
		const nextBand = lodBandForScale(this.viewport.scale.x);
		if (nextBand !== this.band) {
			this.band = nextBand;
			const close = nextBand === "close";
			this.groundLayer.visible = close;
			this.zoneLayer.visible = close;
			this.buildingLayer.visible = close;
			this.catLayer.visible = close;
			this.raiderLayer.visible = close;
			this.overviewLayer.visible = !close;
		}
		if (this.band === "overview") {
			this.syncOverview();
			return;
		}
		this.syncClose(options);
	}

	private chunkInBounds(key: string, bounds: Rectangle): boolean {
		const [cx, cy] = key.split(",").map(Number);
		const originX = cx * ISO.chunkSize;
		const originY = cy * ISO.chunkSize;
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

	private syncClose(options: SceneSyncOptions): void {
		const bounds = this.paddedBounds();
		for (const key of [...this.mountedChunks.keys()]) {
			if (!this.chunkInBounds(key, bounds)) {
				this.unmountChunk(key);
			}
		}
		for (const [key, tiles] of this.tilesByChunk) {
			if (this.mountedChunks.has(key) || !this.chunkInBounds(key, bounds)) {
				continue;
			}
			this.mountChunk(key, tiles);
		}
		this.syncZones(
			options.zones,
			options.now,
			options.draftCorner,
			options.onRemoveZone,
		);
		this.syncBuildings(options.buildings, bounds);
		this.updateRaiders(options.raiders);
	}

	private mountChunk(key: string, tiles: WorldTile[]): void {
		const fog = computeFogBrightness(
			tiles,
			this.anchor,
			this.ringRadius,
			this.village,
		);
		const roads = computeRoadSprites(
			tiles,
			this.anchor,
			this.ringRadius,
			this.village,
		);
		const sprites: ChunkRenderable[] = [];
		for (const tile of tiles) {
			const { left, top } = tileToIso(tile.x, tile.y, ISO);
			const tileZ = zIndexFor(tile.x, tile.y, "tile", ISO);
			const objZ = zIndexFor(tile.x, tile.y, "object", ISO);
			const dim = fog.get(tile._id) ?? 1;
			const explored = dim >= 1;
			const ground = tileGround(
				tile,
				this.anchor,
				this.ringRadius,
				this.village,
				explored ? roads.get(tile._id) : undefined,
			);
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
			if (explored) {
				const marker = this.infoMarkerFor(tile);
				if (marker) {
					marker.position.set(left + ISO.tileWidth / 2, top + ISO.tileHeight / 2);
					marker.zIndex = objZ;
					this.groundLayer.addChild(marker);
					sprites.push(marker);
				}
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
			this.releaseChunkRenderable(s);
		}
		this.mountedChunks.delete(key);
	}

	private infoMarkerFor(tile: WorldTile): Text | null {
		if (
			!this.showInfo ||
			(tile.resources.food < RICH_FOOD && tile.resources.herbs < RICH_HERBS)
		) {
			return null;
		}
		const text = [
			tile.resources.food >= RICH_FOOD ? "🍖" : "",
			tile.resources.herbs >= RICH_HERBS ? "🌿" : "",
		].join("");
		return makeText(text, 16);
	}

	private syncBuildings(buildings: SceneBuilding[], bounds: Rectangle): void {
		for (const s of this.buildingSprites) this.releaseSprite(s);
		this.buildingSprites.length = 0;
		for (const overlay of this.buildingOverlays) overlay.destroy();
		this.buildingOverlays.length = 0;

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
			const z = zIndexFor(b.worldPosition.x, b.worldPosition.y, "object", ISO);
			if (b.type === "shrine") {
				const glow = new Graphics()
					.circle(left + ISO.tileWidth / 2, top - 28, 88)
					.fill({ color: 0xf6c453, alpha: 0.2 });
				glow.zIndex = z - 1;
				this.groundLayer.addChild(glow);
				this.buildingOverlays.push(glow);
			}
			const url = BUILDING_SPRITES[b.type] ?? BUILDING_SPRITE_FALLBACK;
			const s = this.acquireSprite();
			s.texture = this.texture(url);
			s.width = SPRITE_W;
			s.height = SPRITE_H;
			s.position.set(left, top - SPRITE_TOP_OFFSET);
			s.zIndex = z;
			s.alpha = b.constructionProgress < 100 ? 0.45 : 1;
			this.groundLayer.addChild(s);
			this.buildingSprites.push(s);
			if (b.level > 1) {
				const label = makeText(`Lv ${b.level}`, 12, 0xffd580);
				label.position.set(left + ISO.tileWidth / 2, top - 38);
				label.zIndex = z + 1;
				this.buildingLayer.addChild(label);
				this.buildingOverlays.push(label);
			}
			if (b.constructionProgress < 100) {
				const back = new Graphics()
					.roundRect(
						left + ISO.tileWidth / 2 - 48,
						top + ISO.tileHeight / 2 - 4,
						96,
						8,
						4,
					)
					.fill({ color: 0x000000, alpha: 0.5 });
				const fill = new Graphics()
					.roundRect(
						left + ISO.tileWidth / 2 - 48,
						top + ISO.tileHeight / 2 - 4,
						(96 * Math.max(0, Math.min(100, b.constructionProgress))) / 100,
						8,
						4,
					)
					.fill({ color: 0xf6c453, alpha: 0.95 });
				back.zIndex = z + 2;
				fill.zIndex = z + 3;
				this.buildingLayer.addChild(back, fill);
				this.buildingOverlays.push(back, fill);
			}
		}
	}

	private syncZones(
		zones: SceneZone[],
		now: number,
		draftCorner: { x: number; y: number } | null,
		onRemove?: (zoneId: string) => void,
	): void {
		for (const g of this.zoneGraphics) g.destroy();
		this.zoneGraphics.length = 0;
		for (const zone of zones) {
			const minutesLeft = Math.max(
				0,
				Math.ceil((zone.expiresAt - now) / 60_000),
			);
			for (let y = zone.y1; y <= zone.y2; y++) {
				for (let x = zone.x1; x <= zone.x2; x++) {
					const { left, top } = tileToIso(x, y, ISO);
					const g = new Graphics();
					drawDiamond(g, left, top, ZONE_COLOR[zone.kind], 0.35);
					g.zIndex = zIndexFor(x, y, "object", ISO);
					g.eventMode = "static";
					g.cursor = "pointer";
					g.on("pointertap", () => onRemove?.(zone._id));
					this.zoneLayer.addChild(g);
					this.zoneGraphics.push(g);
				}
			}
			const labelPos = tileDiamondCenter(zone.x1, zone.y1, ISO);
			const label = makeText(
				`${zone.kind === "avoid" ? "🚫" : "📍"} ${minutesLeft}m`,
				13,
			);
			label.position.set(labelPos.x, labelPos.y - 14);
			label.zIndex = zIndexFor(zone.x1, zone.y1, "object", ISO) + 1;
			this.zoneLayer.addChild(label);
			this.zoneGraphics.push(label as unknown as Graphics);
		}
		if (draftCorner) {
			const { left, top } = tileToIso(draftCorner.x, draftCorner.y, ISO);
			const g = new Graphics();
			drawDiamond(g, left, top, 0xfac83c, 0.6);
			g.zIndex = zIndexFor(draftCorner.x, draftCorner.y, "object", ISO) + 2;
			this.zoneLayer.addChild(g);
			this.zoneGraphics.push(g);
		}
	}

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
		const counts = new Map<string, number>();
		let explored = 0;
		for (const t of tiles) {
			if (isExplored(t, this.anchor, this.ringRadius, this.village)) {
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
		const c = (dx: number, dy: number) =>
			tileDiamondCenter(originX + dx, originY + dy, ISO);
		const nw = c(0, 0);
		const ne = c(ISO.chunkSize - 1, 0);
		const se = c(ISO.chunkSize - 1, ISO.chunkSize - 1);
		const sw = c(0, ISO.chunkSize - 1);
		const g = new Graphics();
		g.poly([nw.x, nw.y, ne.x, ne.y, se.x, se.y, sw.x, sw.y]).fill({
			color,
			alpha: explored > 0 ? 1 : 0.9,
		});
		g.zIndex = originX + originY;
		this.overviewLayer.addChild(g);
		return g;
	}

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
				view = this.createCat(world, cat._id);
				this.cats.set(cat._id, view);
			}
			view.target = world;
			if (
				Math.abs(view.target.x - view.cur.x) > JUMP_SNAP_TILES ||
				Math.abs(view.target.y - view.cur.y) > JUMP_SNAP_TILES
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
		for (const [id, view] of this.cats) {
			if (!seen.has(id)) {
				view.container.destroy({ children: true });
				view.texture.destroy();
				this.cats.delete(id);
			}
		}
	}

	setCatOverlays(
		cats: SceneCat[],
		leaderId: string | null,
		selectedCatId: string | null,
		onSelect?: (catId: string) => void,
	): void {
		for (const cat of cats) {
			const view = this.cats.get(cat._id);
			if (!view) continue;
			const stage = getLifeStage(cat.ageHours ?? 24);
			const stageGlyph = STAGE_GLYPH[stage] ?? "";
			view.label.text = `${stageGlyph ? `${stageGlyph} ` : ""}${cat.name}`;
			view.badge.text =
				(cat.activity ? ACTIVITY_BADGES[cat.activity] : undefined) ??
				(cat.currentTask ? TASK_BADGES[cat.currentTask] : undefined) ??
				"";
			view.carry.text = cat.carrying
				? cat.carrying.kind === "food"
					? "🎒"
					: "✨"
				: "";
			view.work.text =
				cat.activity === "working"
					? (cat.currentTask && WORK_ICONS[cat.currentTask]) ||
						(cat.specialization && WORK_ICONS[cat.specialization]) ||
						"🏹"
					: "";
			view.leader.text = cat._id === leaderId ? "👑" : "";
			view.selection.visible = cat._id === selectedCatId;
			view.container.eventMode = "static";
			view.container.cursor = "pointer";
			view.container.removeAllListeners("pointertap");
			view.container.on("pointertap", () => onSelect?.(cat._id));
		}
	}

	private createCat(world: { x: number; y: number }, id: string): CatView {
		const sheet = this.texture(CAT_SHEET_URL);
		const texture = new Texture({
			source: sheet.source,
			frame: new Rectangle(0, 0, CAT_CELL, CAT_CELL),
		});
		const sprite = new Sprite(texture);
		sprite.anchor.set(0.5, 0.9);
		const container = new Container();
		const selection = new Graphics()
			.ellipse(0, -4, 34, 16)
			.stroke({ width: 3, color: 0xfacc15, alpha: 0.9 });
		selection.visible = false;
		const label = makeText("", 11);
		label.position.set(0, 16);
		const badge = makeText("", 15);
		badge.position.set(24, -42);
		const carry = makeText("", 16);
		carry.position.set(-24, -34);
		const leader = makeText("", 16);
		leader.position.set(0, -54);
		const work = makeText("", 16);
		work.position.set(0, -42);
		container.addChild(selection, sprite, label, badge, carry, leader, work);
		container.position.set(world.x, world.y);
		container.hitArea = new Rectangle(-32, -60, 64, 84);
		this.catLayer.addChild(container);
		const offset = spreadOffset(id);
		container.pivot.set(-offset.x, -offset.y);
		return {
			container,
			sprite,
			texture,
			label,
			badge,
			carry,
			leader,
			work,
			selection,
			cur: { ...world },
			target: { ...world },
			group: 0,
			frame: 0,
			frameClock: 0,
			walking: false,
			scale: 1,
		};
	}

	tickCats(ticker: Ticker): void {
		if (this.band !== "close") return;
		const dt = ticker.deltaMS / 1000;
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

	private updateRaiders(raiders: SceneRaider[]): void {
		const seen = new Set<string>();
		for (const raider of raiders) {
			if (raider.status === "dead") continue;
			seen.add(raider._id);
			let view = this.raiders.get(raider._id);
			if (!view) {
				view = this.createRaider(raider._id);
				this.raiders.set(raider._id, view);
			}
			const center = tileDiamondCenter(
				raider.position.x,
				raider.position.y,
				ISO,
			);
			const offset = spreadOffset(raider._id, 0.5, 0.45);
			view.container.position.set(center.x + offset.x, center.y + offset.y);
			view.container.zIndex = zIndexFor(
				raider.position.x,
				raider.position.y,
				"object",
				ISO,
			);
			const hpPct = Math.max(
				0,
				Math.min(1, raider.hp / Math.max(1, raider.strength)),
			);
			view.hpFill
				.clear()
				.roundRect(-20, 18, 40 * hpPct, 5, 3)
				.fill(0xdc2626);
			view.icon.text = raider.status === "engaging" ? "⚔️" : "!";
		}
		for (const [id, view] of this.raiders) {
			if (!seen.has(id)) {
				view.container.destroy({ children: true });
				view.texture.destroy();
				this.raiders.delete(id);
			}
		}
	}

	private createRaider(id: string): RaiderView {
		const sheet = this.texture(RAIDER_SHEET_URL);
		const texture = new Texture({
			source: sheet.source,
			frame: new Rectangle(0, 0, CAT_CELL, CAT_CELL),
		});
		const sprite = new Sprite(texture);
		sprite.anchor.set(0.5, 0.9);
		sprite.width = CAT_CELL * 1.9;
		sprite.height = CAT_CELL * 1.9;
		const container = new Container();
		const icon = makeText("⚔️", 15);
		icon.position.set(0, -48);
		const hpBack = new Graphics()
			.roundRect(-20, 18, 40, 5, 3)
			.fill({ color: 0x000000, alpha: 0.55 });
		const hpFill = new Graphics();
		container.addChild(sprite, icon, hpBack, hpFill);
		this.raiderLayer.addChild(container);
		return { container, sprite, texture, icon, hpBack, hpFill };
	}

	destroy(): void {
		for (const view of this.cats.values()) view.texture.destroy();
		for (const view of this.raiders.values()) view.texture.destroy();
		this.cats.clear();
		this.raiders.clear();
		this.groundLayer.destroy({ children: true });
		this.overviewLayer.destroy({ children: true });
		this.zoneLayer.destroy({ children: true });
		this.buildingLayer.destroy({ children: true });
		this.catLayer.destroy({ children: true });
		this.raiderLayer.destroy({ children: true });
	}
}

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
