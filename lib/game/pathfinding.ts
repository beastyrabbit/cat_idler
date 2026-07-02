/**
 * A* pathfinding over the world grid — pure, deterministic, side-effect free.
 *
 * Movement used to be a straight one-axis-at-a-time L-walk with a single
 * hard-coded "go through the south gate" waypoint. That special case is gone:
 * walkability is now a cost model (water blocks, the fence blocks everywhere
 * but the gate, roads are cheap) and the gate *emerges* as the only opening in
 * the fence, so a cat leaving the clearing routes to it on its own.
 *
 * The trail primitive stays {@link pathTiles}: on open ground the straight
 * x-before-y L is already the cheapest route, so {@link findPath} returns it
 * directly (fast path) and the tick wears exactly those tiles. A* only runs
 * when that L crosses something blocked — a river, the palisade — and then it
 * finds the real detour. Node expansion is capped so a pathological search can
 * never blow up a tick; on failure the caller falls back to the straight walk.
 */

import { pathTiles, type WorldPos } from "./movement";

/**
 * The world as walkability: what a mover cannot enter and what it costs to
 * step onto a tile. Both are pure functions of tile coordinates so the same
 * grid drives cats and raiders and stays trivially testable.
 */
export interface WalkGrid {
	/** A mover cannot step onto this tile (river water, or fence off-gate). */
	isBlocked(x: number, y: number): boolean;
	/** Relative cost to enter this tile — roads < 1, open ground 1. */
	cost(x: number, y: number): number;
}

/** Cheapest a single step can cost (a road tile), so the heuristic stays admissible. */
export const MIN_STEP_COST = 0.5;
/** Cost multiplier for a tile carrying a built road or road-grade wear. */
export const ROAD_COST = 0.5;
/** Default cost for ordinary open ground. */
export const OPEN_COST = 1;

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

/** Binary min-heap over `{ key, f }` records, keyed by the f-score. */
class MinHeap {
	private items: Array<{ key: number; f: number }> = [];

	get size(): number {
		return this.items.length;
	}

	push(key: number, f: number): void {
		const items = this.items;
		items.push({ key, f });
		let i = items.length - 1;
		while (i > 0) {
			const parent = (i - 1) >> 1;
			if (items[parent].f <= items[i].f) {
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
				if (l < items.length && items[l].f < items[smallest].f) {
					smallest = l;
				}
				if (r < items.length && items[r].f < items[smallest].f) {
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
	/** Village centre (Chebyshev origin for the fence ring). */
	anchor: WorldPos;
	/** Chebyshev radius of the palisade ring around the anchor. */
	ringRadius: number;
	/** The single tile in the fence ring a mover may pass through. */
	gate: WorldPos;
}

/** A tile carries drawable water (river channel or a resource pool). */
function tileIsWater(tile: WalkTile): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		(tile.resources?.water ?? 0) > 0
	);
}

/** A tile is road-grade — a built road or wear past the road threshold. */
function tileIsRoad(tile: WalkTile): boolean {
	return tile.overlayFeature === "road_built" || tile.pathWear >= 70;
}

/**
 * Walkability for the colony's world: rivers block, the palisade blocks every
 * ring tile but the gate, and roads are cheap so cats drift onto them and wear
 * them deeper. Tiles the colony has never seen are treated as open ground at
 * normal cost, so a route out to a far frontier still plans.
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

	const onFence = (x: number, y: number): boolean =>
		Math.max(Math.abs(x - anchor.x), Math.abs(y - anchor.y)) === ringRadius;
	const isGate = (x: number, y: number): boolean =>
		x === gate.x && y === gate.y;

	return {
		isBlocked(x, y) {
			if (onFence(x, y) && !isGate(x, y)) {
				return true;
			}
			const tile = byKey.get(packKey(x, y));
			return tile ? tileIsWater(tile) : false;
		},
		cost(x, y) {
			const tile = byKey.get(packKey(x, y));
			return tile && tileIsRoad(tile) ? ROAD_COST : OPEN_COST;
		},
	};
}

/**
 * Cheapest walkable route from `start` to `goal` as a list of integer tiles,
 * start first and goal last, or `null` when no route fits inside the search
 * budget. The start and goal tiles are always enterable (a cat can always
 * leave where it stands and reach where it was sent — e.g. a water tile a
 * fetch job targets), so only the tiles *between* them respect `isBlocked`.
 *
 * Fast path: the straight x-before-y L is the cheapest route whenever nothing
 * blocks it, so it is returned without a search — this both saves the search
 * and keeps trodden trails identical to the old L-walk on open ground.
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

	// Fast path: the straight L only counts as clear if none of its *interior*
	// tiles are blocked (the endpoints are always allowed).
	const straight = pathTiles({ x: sx, y: sy }, { x: gx, y: gy });
	let straightClear = true;
	for (let i = 1; i < straight.length - 1; i += 1) {
		if (grid.isBlocked(straight[i].x, straight[i].y)) {
			straightClear = false;
			break;
		}
	}
	if (straightClear) {
		return straight;
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
			// The goal is always enterable; interior tiles honour blocking.
			const isGoal = nx === gx && ny === gy;
			if (!isGoal && grid.isBlocked(nx, ny)) {
				continue;
			}
			const nk = key(nx, ny);
			if (closed[nk]) {
				continue;
			}
			const tentative = gScore[ck] + grid.cost(nx, ny);
			if (tentative < gScore[nk]) {
				gScore[nk] = tentative;
				cameFrom[nk] = ck;
				open.push(nk, tentative + manhattan(nx, ny, gx, gy) * MIN_STEP_COST);
			}
		}
	}

	return null;
}
