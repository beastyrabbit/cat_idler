"use client";

/**
 * Tile test-fitting bench (dev-only QA page).
 *
 * Renders scripted compositions with the *real* game renderer geometry
 * (`ISO`, `tileToIso`) and sprite tables/constants so path autotiling, fence
 * corners, the village clearing, water seams and chopped stumps can be checked
 * side by side at close zoom — far easier than hunting the cases in the live
 * world. Point at a wrong connection here and it maps straight to a case.
 *
 * Route: /dev/fit  (not linked from the game; safe to delete)
 */

import {
	BUILDING_SPRITES,
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	GATE_SPRITE,
	ISO,
	ROAD_DIR,
	roadSpriteFor,
	STUMP_SPRITE,
	TILE_SPRITES,
	WATER_SPRITE,
} from "@/components/map/constants";
import { ringSprites } from "@/components/map/TileLayer";
import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import {
	type FenceSegment,
	fencePerimeter,
	fromTiles,
	gatePlacement,
	type Pos,
} from "@/lib/game/villageArea";
import type { WorldTile } from "@/types/game";

const GRASS = TILE_SPRITES.field.src;
const { E, W, N, S } = ROAD_DIR;

type Placement = {
	x: number;
	y: number;
	src: string;
	ox?: number;
	oy?: number;
};

/**
 * A scaled isometric board. Auto-fits the placements' bounding box (each
 * diamond plus `rise` px of headroom above it for trees/fences/buildings) into
 * `width`, so every scene frames itself. Sprites are placed exactly as the live
 * IsoTile does: at `(left, top - surfaceOffset)`, 256x512.
 */
function Board({
	tiles,
	width = 300,
	pad = 10,
	rise = ISO.surfaceOffset,
}: {
	tiles: Placement[];
	width?: number;
	pad?: number;
	/** How much sky above each diamond top to keep (for trees/fences/buildings). */
	rise?: number;
}) {
	let minL = Infinity;
	let maxL = -Infinity;
	let minT = Infinity;
	let maxT = -Infinity;
	for (const t of tiles) {
		const { left, top } = tileToIso(t.x, t.y, ISO);
		const l = left + (t.ox ?? 0);
		const oy = t.oy ?? 0;
		minL = Math.min(minL, l);
		maxL = Math.max(maxL, l + ISO.tileWidth);
		minT = Math.min(minT, top - rise + oy);
		maxT = Math.max(maxT, top + ISO.tileHeight + oy);
	}
	const contentW = maxL - minL + pad * 2;
	const contentH = maxT - minT + pad * 2;
	const scale = width / contentW;
	return (
		<div
			style={{
				position: "relative",
				width,
				height: contentH * scale,
				overflow: "hidden",
				background: "#141c12",
				borderRadius: 6,
			}}
		>
			<div
				style={{
					position: "absolute",
					left: pad,
					top: pad,
					width: maxL - minL,
					height: maxT - minT,
					transform: `scale(${scale})`,
					transformOrigin: "top left",
				}}
			>
				{tiles.map((t, i) => {
					const { left, top } = tileToIso(t.x, t.y, ISO);
					return (
						<img
							key={`${t.x},${t.y},${t.src},${i}`}
							src={t.src}
							alt=""
							draggable={false}
							style={{
								position: "absolute",
								left: left + (t.ox ?? 0) - minL,
								top: top - ISO.surfaceOffset + (t.oy ?? 0) - minT,
								width: ISO.tileWidth,
								height: ISO.imageHeight,
								maxWidth: "none",
								zIndex: zIndexFor(t.x, t.y, "object", ISO),
								pointerEvents: "none",
								userSelect: "none",
							}}
						/>
					);
				})}
			</div>
		</div>
	);
}

/** A titled case card. */
function Case({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<figure style={{ margin: 0 }}>
			{children}
			<figcaption
				style={{
					marginTop: 4,
					fontSize: 12,
					color: "#cbd5c0",
					textAlign: "center",
					fontFamily: "monospace",
				}}
			>
				{title}
			</figcaption>
		</figure>
	);
}

/** Fill a Chebyshev square of grass around an anchor. */
function grassSquare(
	anchor: { x: number; y: number },
	radius: number,
): Placement[] {
	const out: Placement[] = [];
	for (let dy = -radius; dy <= radius; dy++) {
		for (let dx = -radius; dx <= radius; dx++) {
			out.push({ x: anchor.x + dx, y: anchor.y + dy, src: GRASS });
		}
	}
	return out;
}

/** Build a fake WorldTile just rich enough for ringSprites. */
function fakeTile(x: number, y: number): WorldTile {
	return {
		_id: `${x},${y}`,
		x,
		y,
		type: "field",
		resources: { food: 0, herbs: 0, water: 0 },
		maxResources: { food: 0, herbs: 0 },
		dangerLevel: 0,
		pathWear: 0,
		lastDepleted: 0,
	} as unknown as WorldTile;
}

// --- Scene builders ---------------------------------------------------------

/** (a) Fence ring with all four corners + a south gate. */
function fenceRingTiles(): Placement[] {
	const anchor = { x: 3, y: 3 };
	const ringRadius = 2;
	const tiles: Placement[] = grassSquare(anchor, ringRadius);
	for (let dy = -ringRadius; dy <= ringRadius; dy++) {
		for (let dx = -ringRadius; dx <= ringRadius; dx++) {
			const x = anchor.x + dx;
			const y = anchor.y + dy;
			for (const f of ringSprites(fakeTile(x, y), anchor, ringRadius)) {
				tiles.push({ x, y, src: f.src, ox: f.ox, oy: f.oy });
			}
		}
	}
	return tiles;
}

/** Single-tile path case: grass + the autotiled path for a neighbour mask. */
function pathCase(mask: number): Placement[] {
	return [
		{ x: 0, y: 0, src: GRASS },
		{ x: 0, y: 0, src: roadSpriteFor(mask) },
	];
}

const PATH_CASES: Array<{ title: string; mask: number }> = [
	{ title: "straight E–W", mask: E | W },
	{ title: "straight N–S", mask: N | S },
	{ title: "corner E+N", mask: E | N },
	{ title: "corner E+S", mask: E | S },
	{ title: "corner W+N", mask: W | N },
	{ title: "corner W+S", mask: W | S },
	{ title: "end E", mask: E },
	{ title: "end W", mask: W },
	{ title: "end N", mask: N },
	{ title: "end S", mask: S },
	{ title: "T (E+W+N)", mask: E | W | N },
	{ title: "crossing (4-way)", mask: E | W | N | S },
	{ title: "clearing (lone)", mask: 0 },
];

/** (c) Mini village block: clearing with shrine + den + storage. */
function villageBlockTiles(): Placement[] {
	const anchor = { x: 2, y: 2 };
	const tiles = grassSquare(anchor, 2);
	tiles.push({ x: 2, y: 2, src: BUILDING_SPRITES.shrine });
	tiles.push({ x: 1, y: 3, src: BUILDING_SPRITES.den });
	tiles.push({ x: 3, y: 1, src: BUILDING_SPRITES.food_storage });
	return tiles;
}

/** (d) Water/grass seam: a diagonal ribbon of water through grass. */
function waterSeamTiles(): Placement[] {
	const anchor = { x: 2, y: 2 };
	const tiles = grassSquare(anchor, 2);
	for (const [x, y] of [
		[1, 3],
		[2, 2],
		[3, 1],
	] as const) {
		tiles.push({ x, y, src: WATER_SPRITE });
	}
	return tiles;
}

// --- Organic village (task #31 stage 1) -------------------------------------

/** Tile just outside a claimed tile across the given edge — where the rail sits. */
const OUTSIDE_OF: Record<FenceSegment["side"], { dx: number; dy: number }> = {
	N: { dx: 0, dy: -1 },
	S: { dx: 0, dy: 1 },
	E: { dx: 1, dy: 0 },
	W: { dx: -1, dy: 0 },
};

/** Render an organic claimed shape: cleared grass, its auto-generated fence
 * (FENCE_X on N/S edges, FENCE_Y on E/W), and the gate segment. Rails are seated
 * on the OUTSIDE-neighbour tile centred (ox/oy 0), the exact seating the live
 * renderer's `ringSprites` uses for a straight run, so the fence hugs the shape
 * on the ground instead of floating. */
function organicVillageTiles(tiles: Pos[]): Placement[] {
	const area = fromTiles(tiles);
	const gate = gatePlacement(area);
	const out: Placement[] = [];
	// Cleared ground under every claimed tile.
	for (const t of tiles) {
		out.push({ x: t.x, y: t.y, src: GRASS });
	}
	// Fence rails on every boundary edge; the gate segment swaps in the gate.
	for (const seg of fencePerimeter(area, gate)) {
		const o = OUTSIDE_OF[seg.side];
		const src = seg.gate
			? GATE_SPRITE
			: seg.axis === "x"
				? FENCE_X_SPRITE
				: FENCE_Y_SPRITE;
		out.push({ x: seg.x + o.dx, y: seg.y + o.dy, src });
	}
	return out;
}

const ORGANIC_SHAPES: Array<{ title: string; tiles: Pos[] }> = [
	{
		title: "L-shape + gate",
		tiles: [
			{ x: 0, y: 0 },
			{ x: 1, y: 0 },
			{ x: 0, y: 1 },
			{ x: 1, y: 1 },
			{ x: 2, y: 1 },
			{ x: 0, y: 2 },
			{ x: 1, y: 2 },
			{ x: 2, y: 2 },
		],
	},
	{
		title: "T-shape + gate",
		tiles: [
			{ x: 0, y: 0 },
			{ x: 1, y: 0 },
			{ x: 2, y: 0 },
			{ x: 1, y: 1 },
			{ x: 1, y: 2 },
		],
	},
	{
		title: "diagonal blob + gate",
		tiles: [
			{ x: 0, y: 0 },
			{ x: 1, y: 0 },
			{ x: 1, y: 1 },
			{ x: 2, y: 1 },
			{ x: 2, y: 2 },
			{ x: 3, y: 2 },
			{ x: 3, y: 1 },
		],
	},
	{
		title: "grown clearing (4x3 + spur)",
		tiles: [
			...[0, 1, 2, 3].flatMap((x) => [0, 1, 2].map((y) => ({ x, y }))),
			{ x: 4, y: 1 },
			{ x: 1, y: 3 },
			{ x: 2, y: 3 },
		],
	},
];

/** (e) Chopped stump beside standing forest. */
function stumpTiles(): Placement[] {
	const anchor = { x: 1, y: 1 };
	const tiles = grassSquare(anchor, 1);
	tiles.push({ x: 1, y: 1, src: TILE_SPRITES.forest.src });
	tiles.push({ x: 2, y: 1, src: STUMP_SPRITE });
	tiles.push({ x: 0, y: 1, src: TILE_SPRITES.forest.src });
	return tiles;
}

export default function FitPage() {
	return (
		<main
			style={{
				minHeight: "100vh",
				background: "#0e140c",
				color: "#e8eee0",
				padding: 24,
				fontFamily: "system-ui, sans-serif",
			}}
		>
			<h1 style={{ fontSize: 20, marginBottom: 4 }}>Tile test-fitting bench</h1>
			<p style={{ fontSize: 13, color: "#9fb08c", marginTop: 0 }}>
				Scripted compositions rendered with the live game geometry &amp;
				sprites. Point at a wrong connection to map it to a case. Route:
				/dev/fit
			</p>

			<section style={{ marginTop: 24 }}>
				<h2 style={{ fontSize: 15 }}>Fence ring — 4 corners + gate</h2>
				<Board tiles={fenceRingTiles()} width={420} />
			</section>

			<section style={{ marginTop: 24 }}>
				<h2 style={{ fontSize: 15 }}>
					Organic village — auto-fenced irregular shapes + gate
				</h2>
				<p style={{ fontSize: 12, color: "#9fb08c", marginTop: 0 }}>
					Fence derived from the claimed-tile set via{" "}
					<code>villageArea.fencePerimeter</code> (FENCE_X on N/S edges, FENCE_Y
					on E/W), gate from <code>gatePlacement</code>. The perimeter must be a
					closed loop that hugs the actual shape with exactly one gate.
				</p>
				<div
					style={{
						display: "grid",
						gridTemplateColumns: "repeat(auto-fill, 300px)",
						gap: 16,
						marginTop: 8,
					}}
				>
					{ORGANIC_SHAPES.map((s) => (
						<Case key={s.title} title={s.title}>
							<Board tiles={organicVillageTiles(s.tiles)} width={300} />
						</Case>
					))}
				</div>
			</section>

			<section style={{ marginTop: 24 }}>
				<h2 style={{ fontSize: 15 }}>Path autotiling — every case</h2>
				<div
					style={{
						display: "grid",
						gridTemplateColumns: "repeat(auto-fill, 160px)",
						gap: 16,
						marginTop: 8,
					}}
				>
					{PATH_CASES.map((c) => (
						<Case key={c.title} title={c.title}>
							<Board
								tiles={pathCase(c.mask)}
								width={160}
								rise={ISO.tileHeight}
							/>
						</Case>
					))}
				</div>
			</section>

			<section
				style={{ marginTop: 24, display: "flex", gap: 32, flexWrap: "wrap" }}
			>
				<div>
					<h2 style={{ fontSize: 15 }}>Village block</h2>
					<Board tiles={villageBlockTiles()} width={320} />
				</div>
				<div>
					<h2 style={{ fontSize: 15 }}>Water / grass seam</h2>
					<Board tiles={waterSeamTiles()} width={320} rise={ISO.tileHeight} />
				</div>
				<div>
					<h2 style={{ fontSize: 15 }}>Chopped stump beside forest</h2>
					<Board tiles={stumpTiles()} width={320} />
				</div>
			</section>
		</main>
	);
}
