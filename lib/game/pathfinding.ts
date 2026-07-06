/**
 * A* pathfinding over the world grid — pure, deterministic, side-effect free.
 *
 * Movement used to be a straight one-axis-at-a-time L-walk with a single
 * hard-coded "go through the south gate" waypoint. That special case is gone:
 * walkability is now a real cost model and A* *always* runs, so terrain shapes
 * every route. Water and the fence (off its one gate) are impassable; a built
 * road is the cheapest ground, a worn trail cheaper than open field, and forest
 * is dear — pushing through trees costs several times an open step, so cats
 * visibly skirt the woods and funnel onto roads, and a leader-paved road earns
 * its keep by pulling traffic onto it.
 *
 * On open ground every monotone route ties on cost, so the search breaks ties
 * toward the classic x-before-y L (see {@link X_FIRST_BIAS} below): ordinary trips
 * wear the same trail the old L-walk did, and only real terrain — a river, the
 * palisade, a stand of trees — bends the path. Node expansion is capped so a
 * pathological search can never blow up a tick; on failure the caller falls
 * back to the straight walk. Every returned route is a strict run of
 * 4-neighbour steps (start first, goal last), so the tick wears every tile.
 */

import { elevationBlocksStep, stairBridgesStep } from "./elevation";
import type { WorldPos } from "./movement";
import {
	fenceBlocksMove,
	type GatePlacement,
	type VillageArea,
} from "./villageArea";

/**
 * The world as walkability: what a mover cannot enter and what it costs to
 * step onto a tile. Both are pure functions of tile coordinates so the same
 * grid drives cats and raiders and stays trivially testable.
 */
export interface WalkGrid {
	/** A mover cannot step onto this tile (river water, or fence off-gate). */
	isBlocked(x: number, y: number): boolean;
	/**
	 * Relative cost to enter this tile: built road < worn trail < open ground <
	 * forest < dense woods. Never below {@link MIN_STEP_COST} (which keeps the
	 * A* heuristic admissible).
	 */
	cost(x: number, y: number): number;
	/**
	 * Terrain floor of a tile (optional). Inert while the world renders flat —
	 * kept as a seam so elevation can be reintroduced via {@link cliffBlocksStep}
	 * without rethreading the grid. Absent → flat world.
	 */
	heightAt?(x: number, y: number): number;
	/** Whether a tile carries a staircase. Inert while the world is flat. */
	hasStair?(x: number, y: number): boolean;
	/**
	 * Whether the village palisade blocks the step from (fx,fy) to (tx,ty) — the
	 * organic fence blocks crossing a claimed-area boundary edge except at the
	 * gate. An EDGE test (not a tile test) so the fence follows the actual shape.
	 * Absent when the grid was built from the legacy square ring (which blocks via
	 * {@link isBlocked} instead).
	 */
	fenceBlocksStep?(fx: number, fy: number, tx: number, ty: number): boolean;
}

/**
 * Whether a cliff blocks the step between two adjacent tiles.
 *
 * A supplied height field makes floor-changing edges impassable unless a stair
 * sits on one endpoint and the step changes exactly one floor. Without
 * `heightAt` the world remains flat and this seam is inert.
 */
export function cliffBlocksStep(
	grid: WalkGrid,
	ax: number,
	ay: number,
	bx: number,
	by: number,
): boolean {
	return elevationBlocksStep(grid, ax, ay, bx, by);
}

/** Extra effort for taking a visible stair between floors. */
export const STAIR_STEP_COST = 1.2;

function elevationStepCost(
	grid: WalkGrid,
	ax: number,
	ay: number,
	bx: number,
	by: number,
): number {
	return stairBridgesStep(grid, ax, ay, bx, by) ? STAIR_STEP_COST : 0;
}

/** Cost to enter a tile carrying a leader-paved road — the cheapest ground. */
export const ROAD_COST = 0.4;
/**
 * Cost to enter a worn natural path: a trail trodden to road grade (pathWear at
 * or past the render threshold) or a pre-worn overlay (game trail / old road).
 * Cheaper than open ground, dearer than a built road.
 */
export const WORN_PATH_COST = 0.6;
/** Default cost for ordinary open ground (field, plains, bare rock). */
export const OPEN_COST = 1;
/** Cost to push through forest — several open steps, so cats skirt the trees. */
export const FOREST_COST = 4;
/** Cost to push through dense woods — dearer still than ordinary forest. */
export const DENSE_WOODS_COST = 8;
/**
 * Cheapest a single step can cost (a road tile), so `manhattan * MIN_STEP_COST`
 * never overestimates the true remaining cost and the A* heuristic stays
 * admissible.
 */
export const MIN_STEP_COST = ROAD_COST;
/**
 * Tie-breaking weight that nudges equal-cost routes toward the x-before-y L.
 * Folded into g as a per-step penalty for stepping in y before x is aligned, it
 * makes that L the unique cheapest route on open ground while staying far too
 * small (times any realistic path length) to override a real cost difference
 * between two routes — so it only decides genuine ties.
 */
const X_FIRST_BIAS = 1e-6;

export interface FindPathOptions {
	/**
	 * Hard cap on A* node expansions. A search that hits it returns null and the
	 * caller falls back to the straight walk — this is the per-tick safety valve
	 * that keeps an accelerated tick bounded no matter how tangled the terrain.
	 */
	maxExpansions?: number;
	/**
	 * Tiles of slack added around the start/goal bounding box the search may
	 * roam into. Detours around a river or the fence need room to swing wide;
	 * beyond this margin the search gives up (null → straight-walk fallback).
	 */
	margin?: number;
}

const DEFAULT_MAX_EXPANSIONS = 6000;
const DEFAULT_MARGIN = 16;

/** 4-directional neighbours, x before y so ties resolve like the L-walk. */
const NEIGHBOURS: ReadonlyArray<readonly [number, number]> = [
	[1, 0],
	[-1, 0],
	[0, 1],
	[0, -1],
];

function manhattan(ax: number, ay: number, bx: number, by: number): number {
	return Math.abs(ax - bx) + Math.abs(ay - by);
}

/**
 * Binary min-heap over `{ key, f, seq }` records, ordered by f-score and then by
 * insertion sequence. The `seq` tie-break makes pops fully deterministic: two
 * nodes with an identical f come out in the order they went in, so the same grid
 * always yields byte-identical routes run to run (no reliance on heap-swap luck).
 */
class MinHeap {
	private items: Array<{ key: number; f: number; seq: number }> = [];
	private counter = 0;

	get size(): number {
		return this.items.length;
	}

	/** True when `a` should sort before `b` (lower f, ties broken by insertion). */
	private before(
		a: { f: number; seq: number },
		b: { f: number; seq: number },
	): boolean {
		return a.f < b.f || (a.f === b.f && a.seq < b.seq);
	}

	push(key: number, f: number): void {
		const items = this.items;
		items.push({ key, f, seq: this.counter++ });
		let i = items.length - 1;
		while (i > 0) {
			const parent = (i - 1) >> 1;
			if (!this.before(items[i], items[parent])) {
				break;
			}
			[items[parent], items[i]] = [items[i], items[parent]];
			i = parent;
		}
	}

	pop(): { key: number; f: number } | undefined {
		const items = this.items;
		const top = items[0];
		const last = items.pop();
		if (items.length > 0 && last) {
			items[0] = last;
			let i = 0;
			for (;;) {
				const l = 2 * i + 1;
				const r = 2 * i + 2;
				let smallest = i;
				if (l < items.length && this.before(items[l], items[smallest])) {
					smallest = l;
				}
				if (r < items.length && this.before(items[r], items[smallest])) {
					smallest = r;
				}
				if (smallest === i) {
					break;
				}
				[items[smallest], items[i]] = [items[i], items[smallest]];
				i = smallest;
			}
		}
		return top;
	}
}

/** Minimal tile shape the walkability grid reads. */
export interface WalkTile {
	x: number;
	y: number;
	type: string;
	overlayFeature?: string | null;
	resources?: { water?: number } | null;
	pathWear: number;
}

export interface ColonyGridParams {
	/** Every known colony tile; unknown tiles default to open walkable ground. */
	tiles: WalkTile[];
	/** Village centre (Chebyshev origin for the legacy fence ring). */
	anchor: WorldPos;
	/** Chebyshev radius of the legacy palisade ring around the anchor. Ignored
	 * when {@link area} is given (the organic fence follows the claimed shape). */
	ringRadius: number;
	/** The single tile in the legacy fence ring a mover may pass through. */
	gate: WorldPos;
	/**
	 * The organic claimed village area (lib/game/villageArea.ts). When given, the
	 * palisade is derived from this shape's boundary edges and blocks crossings
	 * (except the {@link areaGate}) via `fenceBlocksStep`, superseding the square
	 * ring. Omit to keep the legacy Chebyshev-ring behaviour.
	 */
	area?: VillageArea;
	/** The gate edge for {@link area} — the one boundary crossing that's open. */
	areaGate?: GatePlacement | null;
	/**
	 * Optional terrain height/stair field (from `terrainGen`). When supplied,
	 * cliffs (a 2+ floor drop) block movement unless a staircase bridges them.
	 */
	terrain?: {
		heightAt(x: number, y: number): number;
		hasStair(x: number, y: number): boolean;
	};
}

/** A tile carries drawable water (river channel or a resource pool). */
function tileIsWater(tile: WalkTile): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		(tile.resources?.water ?? 0) > 0
	);
}

/** Pathwear at or above which a trodden trail renders (and costs) as a road. */
const ROAD_WEAR_THRESHOLD = 70;

/** Overlay features that are pre-worn natural paths (game trails, old roads). */
const NATURAL_PATH_OVERLAYS = new Set([
	"game_trail",
	"ancient_road",
	"trade_route",
]);

/**
 * Cost to enter a tile, from its terrain and how trodden it is:
 * built road < worn trail < open ground < forest < dense woods. Unknown tiles
 * (never-seen frontier) are treated as open ground.
 */
function tileCost(tile: WalkTile | undefined): number {
	if (!tile) {
		return OPEN_COST;
	}
	if (tile.overlayFeature === "road_built") {
		return ROAD_COST;
	}
	if (
		tile.pathWear >= ROAD_WEAR_THRESHOLD ||
		(tile.overlayFeature != null &&
			NATURAL_PATH_OVERLAYS.has(tile.overlayFeature))
	) {
		return WORN_PATH_COST;
	}
	if (tile.type === "dense_woods") {
		return DENSE_WOODS_COST;
	}
	if (tile.type === "forest") {
		return FOREST_COST;
	}
	return OPEN_COST;
}

/**
 * Walkability for the colony's world: rivers block, the palisade blocks every
 * ring tile but the gate, roads and worn trails are cheap (so cats drift onto
 * them and wear them deeper) while forest is dear (so they skirt the trees).
 * Tiles the colony has never seen are treated as open ground at normal cost, so
 * a route out to a far frontier still plans.
 */
export function buildColonyWalkGrid(params: ColonyGridParams): WalkGrid {
	const { anchor, ringRadius, gate } = params;
	const byKey = new Map<number, WalkTile>();
	// Pack coords into one integer key; offset keeps negatives non-colliding.
	const OFFSET = 1 << 15;
	const packKey = (x: number, y: number): number =>
		(x + OFFSET) * (1 << 16) + (y + OFFSET);
	for (const tile of params.tiles) {
		byKey.set(packKey(tile.x, tile.y), tile);
	}

	// Organic palisade: the fence follows the claimed shape's boundary edges and
	// blocks crossings (except the gate) as an EDGE test. Falls back to the legacy
	// Chebyshev ring (a tile test in isBlocked) when no area is supplied.
	const { area, areaGate } = params;
	const onFence = (x: number, y: number): boolean =>
		Math.max(Math.abs(x - anchor.x), Math.abs(y - anchor.y)) === ringRadius;
	const isGate = (x: number, y: number): boolean =>
		x === gate.x && y === gate.y;

	const { terrain } = params;
	return {
		isBlocked(x, y) {
			// Legacy ring only blocks via a tile test; the organic fence uses
			// fenceBlocksStep instead, so skip the ring test when an area is given.
			if (!area && onFence(x, y) && !isGate(x, y)) {
				return true;
			}
			const tile = byKey.get(packKey(x, y));
			return tile ? tileIsWater(tile) : false;
		},
		cost(x, y) {
			return tileCost(byKey.get(packKey(x, y)));
		},
		fenceBlocksStep: area
			? (fx, fy, tx, ty) =>
					fenceBlocksMove({ x: fx, y: fy }, { x: tx, y: ty }, area, areaGate)
			: undefined,
		heightAt: terrain ? (x, y) => terrain.heightAt(x, y) : undefined,
		hasStair: terrain ? (x, y) => terrain.hasStair(x, y) : undefined,
	};
}

/**
 * Cheapest walkable route from `start` to `goal` as a list of integer tiles,
 * start first and goal last, or `null` when no route fits inside the search
 * budget. The start and goal tiles are always enterable (a cat can always
 * leave where it stands and reach where it was sent — e.g. a water tile a
 * fetch job targets), so only the tiles *between* them respect `isBlocked`.
 *
 * The cost search always runs so terrain actually shapes the route — there is
 * no straight-line fast path that would blind the cat to a cheaper road or a
 * costly wood on the way. The only shortcuts are the degenerate ones a search
 * would waste work on: standing still, and a single adjacent step. On uniform
 * open ground the x-before-y bias reproduces the old straight L, so trodden
 * trails there are unchanged.
 */
export function findPath(
	start: WorldPos,
	goal: WorldPos,
	grid: WalkGrid,
	options: FindPathOptions = {},
): WorldPos[] | null {
	const sx = Math.round(start.x);
	const sy = Math.round(start.y);
	const gx = Math.round(goal.x);
	const gy = Math.round(goal.y);

	if (sx === gx && sy === gy) {
		return [{ x: sx, y: sy }];
	}

	// Cheap shortcut for an adjacent goal when the direct edge is legal. If an
	// edge blocker stands between start and goal, fall through to A* so a nearby
	// stair or gate can still provide a valid detour.
	if (manhattan(sx, sy, gx, gy) === 1) {
		if (
			!cliffBlocksStep(grid, sx, sy, gx, gy) &&
			!(grid.fenceBlocksStep?.(sx, sy, gx, gy) ?? false)
		) {
			return [
				{ x: sx, y: sy },
				{ x: gx, y: gy },
			];
		}
	}

	const maxExpansions = options.maxExpansions ?? DEFAULT_MAX_EXPANSIONS;
	const margin = options.margin ?? DEFAULT_MARGIN;
	const minX = Math.min(sx, gx) - margin;
	const maxX = Math.max(sx, gx) + margin;
	const minY = Math.min(sy, gy) - margin;
	const maxY = Math.max(sy, gy) + margin;
	const width = maxX - minX + 1;

	// Nodes are keyed by a flat index into the bounding box so state lives in
	// plain typed arrays — no per-node object churn inside the hot loop.
	const key = (x: number, y: number): number => (y - minY) * width + (x - minX);
	const size = width * (maxY - minY + 1);
	const gScore = new Float64Array(size).fill(Number.POSITIVE_INFINITY);
	const cameFrom = new Int32Array(size).fill(-1);
	const closed = new Uint8Array(size);

	const startKey = key(sx, sy);
	gScore[startKey] = 0;
	const open = new MinHeap();
	open.push(startKey, manhattan(sx, sy, gx, gy) * MIN_STEP_COST);

	const goalKey = key(gx, gy);
	let expansions = 0;

	while (open.size > 0) {
		const current = open.pop();
		if (!current) {
			break;
		}
		const ck = current.key;
		if (closed[ck]) {
			continue;
		}
		closed[ck] = 1;

		if (ck === goalKey) {
			// Reconstruct start→goal.
			const path: WorldPos[] = [];
			let node = ck;
			while (node !== -1) {
				const px = (node % width) + minX;
				const py = Math.floor(node / width) + minY;
				path.push({ x: px, y: py });
				node = cameFrom[node];
			}
			path.reverse();
			return path;
		}

		expansions += 1;
		if (expansions > maxExpansions) {
			return null;
		}

		const cx = (ck % width) + minX;
		const cy = Math.floor(ck / width) + minY;
		for (const [dx, dy] of NEIGHBOURS) {
			const nx = cx + dx;
			const ny = cy + dy;
			if (nx < minX || nx > maxX || ny < minY || ny > maxY) {
				continue;
			}
			// The goal is always enterable; interior tiles honour blocking. A
			// cliff face blocks every step (even into the goal) unless a stair
			// bridges it — an unreachable mesa fails to a straight-walk fallback.
			const isGoal = nx === gx && ny === gy;
			if (!isGoal && grid.isBlocked(nx, ny)) {
				continue;
			}
			if (cliffBlocksStep(grid, cx, cy, nx, ny)) {
				continue;
			}
			// The palisade blocks crossing the claimed-area boundary except at the
			// gate — an edge test, so it holds even into the goal (a cat must leave
			// through the gate, not vault the fence to reach a spot just outside).
			if (grid.fenceBlocksStep?.(cx, cy, nx, ny)) {
				continue;
			}
			const nk = key(nx, ny);
			if (closed[nk]) {
				continue;
			}
			// A vanishing penalty for stepping in y before x is aligned biases the
			// search toward finishing the x-leg first: on open ground that makes the
			// x-before-y L the unique cheapest route, while the penalty is far too
			// small to ever outweigh a real terrain cost difference.
			const prematureY = dy !== 0 && cx !== gx ? X_FIRST_BIAS : 0;
			const tentative =
				gScore[ck] +
				grid.cost(nx, ny) +
				elevationStepCost(grid, cx, cy, nx, ny) +
				prematureY;
			if (tentative < gScore[nk]) {
				gScore[nk] = tentative;
				cameFrom[nk] = ck;
				open.push(nk, tentative + manhattan(nx, ny, gx, gy) * MIN_STEP_COST);
			}
		}
	}

	return null;
}
