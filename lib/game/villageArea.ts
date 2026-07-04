/**
 * Organic village area (pure, deterministic, no DB / no terrain / no renderer
 * imports). Task #31 stage 1.
 *
 * ── The model ────────────────────────────────────────────────────────────────
 * The village is no longer a fixed square ring. It is a SET of claimed tiles
 * (`VillageArea`, a set of "x,y" keys) that grows ONE tile at a time, and the
 * fence is derived from the *actual shape* of that set — not from a radius.
 *
 * The fence lives on the BOUNDARY EDGES of the claimed set: every edge between a
 * claimed tile and a non-claimed (or water) orthogonal neighbour is a fence
 * segment. The boundary of a connected region of grid cells is always a single
 * closed loop, so this is closed by construction for any shape (square, L, T,
 * blob) with no special corner handling. It also maps exactly onto the sprite
 * vocabulary the renderer actually has (`components/map/constants.ts`), which is
 * only FENCE_X (an east–west rail) and FENCE_Y (a north–south rail) plus a gate
 * — there is NO corner sprite. In this edge model a corner is simply where a
 * horizontal edge meets a vertical edge on the same tile: two ordinary segments,
 * so `fence-x` + `fence-y` render the L exactly like the old ring's corners do.
 *
 *   - a tile's N or S edge is horizontal  → axis "x" → FENCE_X sprite
 *   - a tile's E or W edge is vertical     → axis "y" → FENCE_Y sprite
 *
 * `fenceMaskAt` gives the autotile bitmask per boundary tile (which of its four
 * sides are fence edges), the fence analogue of `roadSpriteFor`'s neighbour mask;
 * `fencePerimeter` flattens the whole boundary into individual segments with the
 * gate flagged.
 *
 * ── For the stage-2 wirer ────────────────────────────────────────────────────
 * This module is deliberately free of side effects so the tick/pathfinding/
 * render wiring can consume it without touching this file. To integrate:
 *   - CLEARING / render ring: replace the Chebyshev `villageRingRadius` fence with
 *     `fencePerimeter(area, gate)`; each segment renders FENCE_X (N/S) or FENCE_Y
 *     (E/W) on its tile's edge, the gate segment renders the gate sprite. Ground
 *     under `isInsideVillage` is cleared grass.
 *   - nextBuildingSite: pick free interior tiles from the area (a claimed tile
 *     with no building) instead of concentric rings; call `expandVillage` when
 *     `shouldExpand` fires to claim one more tile first.
 *   - PATHFINDING blocking: the fence blocks *crossing a boundary edge* except at
 *     the gate. Use `fenceBlocksMove(from, to, area, gate)` (or its parts,
 *     `fenceEdgeBetween` + `perimeterBlocks`) in place of the old "step onto a
 *     ring tile" test. `isInsideVillage` answers containment.
 * The persisted representation is just the list of claimed tiles (`toTiles` /
 * `fromTiles`); everything else is derived.
 */

export interface Pos {
	x: number;
	y: number;
}

/** A claimed village as a set of "x,y" tile keys. Derived, not persisted; the
 * DB stores the tile list (see {@link toTiles} / {@link fromTiles}). */
export type VillageArea = ReadonlySet<string>;

/** Which side of a tile an edge sits on. N/S are horizontal, E/W vertical. */
export type Side = "N" | "E" | "S" | "W";

/** Autotile direction bits for a tile's four sides (mirrors ROAD_DIR's shape). */
export const FENCE_DIR = { N: 1, E: 2, S: 4, W: 8 } as const;

/** Tile-space delta for each side (y increases going south, x going east). */
export const SIDE_DELTA: Record<Side, Pos> = {
	N: { x: 0, y: -1 },
	E: { x: 1, y: 0 },
	S: { x: 0, y: 1 },
	W: { x: -1, y: 0 },
};

const SIDES: Side[] = ["N", "E", "S", "W"];

/** Horizontal edges (N/S) render along the x-axis; vertical edges (E/W) along y. */
export function sideAxis(side: Side): "x" | "y" {
	return side === "N" || side === "S" ? "x" : "y";
}

export function key(x: number, y: number): string {
	return `${x},${y}`;
}
export function keyOf(pos: Pos): string {
	return key(pos.x, pos.y);
}
export function posOf(k: string): Pos {
	const [x, y] = k.split(",").map(Number);
	return { x, y };
}

export function fromTiles(tiles: readonly Pos[]): VillageArea {
	return new Set(tiles.map(keyOf));
}
export function toTiles(area: VillageArea): Pos[] {
	// Stable order (row-major) so callers/tests are deterministic.
	return [...area].map(posOf).sort((a, b) => a.y - b.y || a.x - b.x);
}

/** Whether a tile is claimed village ground. */
export function isInsideVillage(pos: Pos, area: VillageArea): boolean {
	return area.has(keyOf(pos));
}

// --- Fence perimeter (boundary edges) --------------------------------------

/**
 * Autotile bitmask of which of a tile's four sides are fence edges — i.e. the
 * orthogonal neighbour on that side is NOT claimed village ground. Non-boundary
 * (fully interior) tiles return 0. This is the fence analogue of the neighbour
 * mask `roadSpriteFor` consumes. A tile only ever borders open ground/water on a
 * side, so no `isWater` is needed here: an edge exists wherever the neighbour is
 * simply not part of the area (water is never claimed — see {@link expandVillage}).
 */
export function fenceMaskAt(pos: Pos, area: VillageArea): number {
	if (!isInsideVillage(pos, area)) {
		return 0;
	}
	let mask = 0;
	for (const side of SIDES) {
		const d = SIDE_DELTA[side];
		if (!isInsideVillage({ x: pos.x + d.x, y: pos.y + d.y }, area)) {
			mask |= FENCE_DIR[side];
		}
	}
	return mask;
}

export interface FenceSegment {
	/** The claimed tile this edge belongs to. */
	x: number;
	y: number;
	/** Which edge of the tile the fence sits on. */
	side: Side;
	/** Render axis: "x" → FENCE_X sprite, "y" → FENCE_Y sprite. */
	axis: "x" | "y";
	/** True for the single gate segment (renders the gate sprite, passable). */
	gate: boolean;
}

/**
 * Every fence segment around the claimed shape, one per boundary edge, in a
 * deterministic order (row-major by tile, then N,E,S,W). If a `gate` edge is
 * given, the matching segment is flagged `gate: true`. The result is a closed
 * loop for any connected area.
 */
export function fencePerimeter(
	area: VillageArea,
	gate?: GatePlacement | null,
): FenceSegment[] {
	const out: FenceSegment[] = [];
	for (const pos of toTiles(area)) {
		const mask = fenceMaskAt(pos, area);
		for (const side of SIDES) {
			if ((mask & FENCE_DIR[side]) === 0) {
				continue;
			}
			out.push({
				x: pos.x,
				y: pos.y,
				side,
				axis: sideAxis(side),
				gate:
					gate != null &&
					gate.x === pos.x &&
					gate.y === pos.y &&
					gate.side === side,
			});
		}
	}
	return out;
}

/** Count of fence segments — handy for tests and HUD ("fence length"). */
export function perimeterLength(area: VillageArea): number {
	let n = 0;
	for (const pos of area) {
		n = n + popcount(fenceMaskAt(posOf(pos), area));
	}
	return n;
}

function popcount(mask: number): number {
	let n = 0;
	let m = mask;
	while (m) {
		n += m & 1;
		m >>= 1;
	}
	return n;
}

// --- Gate ------------------------------------------------------------------

export interface GatePlacement {
	x: number;
	y: number;
	side: Side;
}

export interface GateOptions {
	/**
	 * Path wear on the OUTSIDE tile just beyond a boundary edge; the gate opens
	 * onto the most-trodden corridor. Higher = busier. Optional.
	 */
	outsideWear?: (outside: Pos) => number;
	/**
	 * Fallback bias direction (the shrine→river axis) as a unit-ish delta; the
	 * gate faces the boundary edge most aligned with it. Optional; defaults to
	 * south (matching the historical fixed gate).
	 */
	axisBias?: Pos;
}

/**
 * Choose the gate: the boundary edge opening onto the most-worn outside corridor
 * (`outsideWear`), falling back to the edge whose outward direction best aligns
 * with `axisBias` (the shrine→river axis), and finally to the southernmost edge
 * — the historical gate side — so the result is always defined for a non-empty
 * area. Deterministic: ties break by the same row-major segment order as
 * {@link fencePerimeter}.
 */
export function gatePlacement(
	area: VillageArea,
	opts: GateOptions = {},
): GatePlacement | null {
	const segments = fencePerimeter(area);
	if (segments.length === 0) {
		return null;
	}
	const bias = opts.axisBias ?? SIDE_DELTA.S;
	const centroid = areaCentroid(area);
	let best: FenceSegment | null = null;
	let bestScore = Number.NEGATIVE_INFINITY;
	let bestDist = Number.POSITIVE_INFINITY;
	for (const seg of segments) {
		const d = SIDE_DELTA[seg.side];
		const outside = { x: seg.x + d.x, y: seg.y + d.y };
		const score = opts.outsideWear
			? opts.outsideWear(outside)
			: // Alignment with the bias axis (dot product).
				d.x * sign(bias.x) + d.y * sign(bias.y);
		// Tie-break toward the CENTRE of the chosen edge (nearest the centroid), so
		// a symmetric footprint gates from the middle of its south side (the
		// historical gate) rather than a corner. Deterministic: exact ties fall to
		// the earlier row-major segment.
		const dist = (seg.x - centroid.x) ** 2 + (seg.y - centroid.y) ** 2;
		if (score > bestScore || (score === bestScore && dist < bestDist)) {
			bestScore = score;
			bestDist = dist;
			best = seg;
		}
	}
	return best ? { x: best.x, y: best.y, side: best.side } : null;
}

function sign(n: number): number {
	return n > 0 ? 1 : n < 0 ? -1 : 0;
}

// --- Pathfinding blocking (edge crossings) ---------------------------------

/**
 * The boundary edge crossed when stepping between two orthogonally-adjacent
 * tiles, or null if the move doesn't cross the fence (both inside, both outside,
 * or not adjacent). The edge is expressed as the INSIDE tile's segment.
 */
export function fenceEdgeBetween(
	from: Pos,
	to: Pos,
	area: VillageArea,
): FenceSegment | null {
	const dx = to.x - from.x;
	const dy = to.y - from.y;
	if (Math.abs(dx) + Math.abs(dy) !== 1) {
		return null; // not orthogonally adjacent
	}
	const fromIn = isInsideVillage(from, area);
	const toIn = isInsideVillage(to, area);
	if (fromIn === toIn) {
		return null; // both inside or both outside — no crossing
	}
	const inside = fromIn ? from : to;
	const side: Side =
		dx === 1
			? fromIn
				? "E"
				: "W"
			: dx === -1
				? fromIn
					? "W"
					: "E"
				: dy === 1
					? fromIn
						? "S"
						: "N"
					: fromIn
						? "N"
						: "S";
	return {
		x: inside.x,
		y: inside.y,
		side,
		axis: sideAxis(side),
		gate: false,
	};
}

/**
 * Whether a fence segment blocks passage. The palisade blocks EVERYTHING except
 * the single gate segment. (`perimeterBlocks(segment)` per the stage-1 brief.)
 */
export function perimeterBlocks(segment: FenceSegment): boolean {
	return !segment.gate;
}

/**
 * Whether the fence blocks a step from `from` to `to`: true when the move
 * crosses a boundary edge that isn't the gate. Both `from`/`to` inside, both
 * outside, or a non-adjacent move never block. This is the primitive the stage-2
 * pathfinding uses in place of the old "cannot step onto a ring tile" test.
 */
export function fenceBlocksMove(
	from: Pos,
	to: Pos,
	area: VillageArea,
	gate?: GatePlacement | null,
): boolean {
	const edge = fenceEdgeBetween(from, to, area);
	if (!edge) {
		return false;
	}
	const isGate =
		gate != null &&
		gate.x === edge.x &&
		gate.y === edge.y &&
		gate.side === edge.side;
	return perimeterBlocks({ ...edge, gate: isGate });
}

// --- Growth: claim one tile at a time --------------------------------------

export interface ExpandOptions {
	/** A tile is unclaimable if it is water (rivers/ponds). Injected so this
	 * module stays free of terrain imports. Defaults to "no water". */
	isWater?: (pos: Pos) => boolean;
	/** Deterministic 0..1 source; used only to break exact ties so growth looks
	 * organic while staying reproducible. Defaults to a fixed choice. */
	rng?: () => number;
}

/**
 * Pick the single next tile to claim, or null if the area can't grow (no dry,
 * adjacent tile). Candidates are the unclaimed, non-water tiles orthogonally
 * adjacent to the area. Each is scored by how much it FILLS IN the shape — its
 * count of already-claimed neighbours over the full 8-neighbourhood — so growth
 * prefers concavities and stays compact/convex-ish rather than sprouting spurs.
 * Ties break toward the tile nearest the area's centroid, then (if an `rng` is
 * given) by a seeded roll for organic variety, then by stable row-major order.
 */
export function expandVillage(
	area: VillageArea,
	opts: ExpandOptions = {},
): Pos | null {
	const isWater = opts.isWater ?? (() => false);
	if (area.size === 0) {
		return null;
	}
	// Candidate frontier: unclaimed, dry, orthogonally adjacent to the area.
	const seen = new Set<string>();
	const candidates: Pos[] = [];
	for (const pos of area) {
		const p = posOf(pos);
		for (const side of SIDES) {
			const d = SIDE_DELTA[side];
			const c = { x: p.x + d.x, y: p.y + d.y };
			const ck = keyOf(c);
			if (seen.has(ck) || isInsideVillage(c, area) || isWater(c)) {
				continue;
			}
			seen.add(ck);
			candidates.push(c);
		}
	}
	if (candidates.length === 0) {
		return null;
	}
	const centroid = areaCentroid(area);
	let best: Pos | null = null;
	let bestKey: [number, number, number] = [
		Number.NEGATIVE_INFINITY,
		Number.NEGATIVE_INFINITY,
		Number.NEGATIVE_INFINITY,
	];
	for (const c of candidates) {
		const fill = claimedNeighbours8(c, area); // 0..8, higher = more compact
		const dist2 = (c.x - centroid.x) ** 2 + (c.y - centroid.y) ** 2;
		const roll = opts.rng ? opts.rng() : 0;
		// Sort key: maximise fill, minimise distance, then the roll.
		const cand: [number, number, number] = [fill, -dist2, roll];
		if (compareKey(cand, bestKey) > 0) {
			bestKey = cand;
			best = c;
		}
	}
	return best;
}

function compareKey(
	a: [number, number, number],
	b: [number, number, number],
): number {
	for (let i = 0; i < 3; i++) {
		if (a[i] !== b[i]) {
			return a[i] > b[i] ? 1 : -1;
		}
	}
	return 0;
}

function claimedNeighbours8(pos: Pos, area: VillageArea): number {
	let n = 0;
	for (let dy = -1; dy <= 1; dy++) {
		for (let dx = -1; dx <= 1; dx++) {
			if (dx === 0 && dy === 0) {
				continue;
			}
			if (isInsideVillage({ x: pos.x + dx, y: pos.y + dy }, area)) {
				n++;
			}
		}
	}
	return n;
}

function areaCentroid(area: VillageArea): Pos {
	let sx = 0;
	let sy = 0;
	for (const k of area) {
		const p = posOf(k);
		sx += p.x;
		sy += p.y;
	}
	return { x: sx / area.size, y: sy / area.size };
}

// --- Growth trigger --------------------------------------------------------

/** Interior tiles kept free of buildings before the leader claims more ground —
 * a small working margin so there's always somewhere to break the next build. */
export const FREE_TILE_FLOOR = 2;
/** Above this many cats per claimed tile the village feels cramped and grows,
 * even if a build slot is technically still free. */
export const CROWDING_PER_TILE = 1.5;

/**
 * Whether the leader should claim one more tile before its next build. Expand
 * when the free buildable interior runs low (fewer than {@link FREE_TILE_FLOOR}
 * un-built claimed tiles) OR when the population outgrows the footprint
 * ({@link CROWDING_PER_TILE} cats per claimed tile). Pure and monotone in each
 * input so it is easy to reason about and boundary-test.
 */
export function shouldExpand(
	population: number,
	claimedCount: number,
	buildingCount: number,
): boolean {
	if (claimedCount <= 0) {
		return true;
	}
	const freeTiles = claimedCount - buildingCount;
	if (freeTiles < FREE_TILE_FLOOR) {
		return true;
	}
	return population > claimedCount * CROWDING_PER_TILE;
}
