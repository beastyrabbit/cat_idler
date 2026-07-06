/**
 * Cat movement simulation — pure functions driven by workerTick.
 *
 * Movement is cosmetic in this phase: the economy stays on job timers,
 * cats just visibly travel to where their job happens. Positions are
 * world-tile coordinates and may be fractional between hops; the map UI
 * interpolates the 1Hz updates with CSS transitions.
 */

export interface WorldPos {
	x: number;
	y: number;
}

/** Base walking speed. Test time-scale multiplies elapsed seconds. */
export const MOVE_SPEED_TILES_PER_SEC = 0.5;

/** Idle cats meander within this Chebyshev radius of the village anchor. */
export const WANDER_RADIUS = 3;

/**
 * Explorers pick their way carefully through the fog: while traveling to a
 * frontier tile they move at this fraction of normal speed, so scouting a
 * wide 5x5 reveal is a real time investment rather than a free sprint.
 */
export const EXPLORE_SPEED_FACTOR = 0.35;

/** Fallback hunt range (Chebyshev tiles from the anchor). */
const HUNT_RANGE_MIN = 8;
const HUNT_RANGE_MAX = 14;

export interface MovementStep {
	position: WorldPos;
	arrived: boolean;
}

/**
 * Advance a position toward a destination with a movement budget of
 * `speed * elapsedSec` tiles, spent greedily on the x axis first, then y.
 * Never overshoots.
 */
export function advanceMovement(
	position: WorldPos,
	destination: WorldPos,
	elapsedSec: number,
	speed: number = MOVE_SPEED_TILES_PER_SEC,
): MovementStep {
	const budget = Math.max(0, elapsedSec) * speed;
	let { x, y } = position;

	// Strictly 4-directional: one axis per step, x before y — cats hop
	// tile to tile instead of cutting corners.
	const dx = destination.x - x;
	if (dx !== 0) {
		x += Math.sign(dx) * Math.min(Math.abs(dx), budget);
	} else {
		const dy = destination.y - y;
		y += Math.sign(dy) * Math.min(Math.abs(dy), budget);
	}

	return {
		position: { x, y },
		arrived: x === destination.x && y === destination.y,
	};
}

/**
 * Integer tiles a strict 4-directional, x-before-y walk crosses going from
 * `from` to `to`, inclusive of both rounded endpoints. This mirrors
 * {@link advanceMovement}'s one-axis-at-a-time rule, so the returned tiles form
 * a straight run (single axis), an L (both axes), or a single tile (zero-length
 * hop). Fractional coordinates are rounded onto the tile grid.
 *
 * This is the shared source of truth for "which tiles did this cat step on":
 * the movement tick wears and reveals exactly these tiles, and future enemies
 * decide whether they can intercept a traveler by testing membership here.
 */
export function pathTiles(from: WorldPos, to: WorldPos): WorldPos[] {
	const startX = Math.round(from.x);
	const startY = Math.round(from.y);
	const endX = Math.round(to.x);
	const endY = Math.round(to.y);

	const tiles: WorldPos[] = [];
	// The row at startY, from startX to endX (x first).
	const stepX = Math.sign(endX - startX);
	for (let x = startX; ; x += stepX) {
		tiles.push({ x, y: startY });
		if (x === endX) {
			break;
		}
	}
	// Then the column at endX, from startY to endY — the corner tile
	// (endX, startY) is already the last of the row, so start one step in.
	const stepY = Math.sign(endY - startY);
	if (stepY !== 0) {
		for (let y = startY + stepY; ; y += stepY) {
			tiles.push({ x: endX, y });
			if (y === endY) {
				break;
			}
		}
	}
	return tiles;
}

export interface PathWalk {
	/** Final (possibly fractional) position where the budget ran out. */
	position: WorldPos;
	/** True when the destination itself was reached this walk. */
	arrived: boolean;
	/** Integer tiles the walk crossed, start tile first (deduped, in order). */
	tiles: WorldPos[];
}

/**
 * Walk `from` toward `destination` (through optional `waypoints`, e.g. the
 * village gate) spending a whole tile budget on a strict one-axis-at-a-time,
 * x-before-y 4-directional path.
 *
 * Unlike {@link advanceMovement} — which moves one axis per call and discards
 * any leftover budget — this consumes the entire budget within a single call,
 * turning corners and clearing waypoints as it goes. That is what lets an
 * accelerated tick actually walk every tile of a long journey instead of
 * teleporting one leg per render: the cat's final position is still wherever
 * the budget ran out, but {@link PathWalk.tiles} records the full trail so it
 * can be worn, revealed, and (later) intercepted.
 */
export function walkPath(
	from: WorldPos,
	destination: WorldPos,
	budgetTiles: number,
	waypoints: WorldPos[] = [],
): PathWalk {
	const stops = [...waypoints, destination];
	let budget = Math.max(0, budgetTiles);
	let x = from.x;
	let y = from.y;

	const tiles: WorldPos[] = [];
	const seen = new Set<string>();
	const record = (segFrom: WorldPos, segTo: WorldPos) => {
		for (const tile of pathTiles(segFrom, segTo)) {
			const key = `${tile.x},${tile.y}`;
			if (!seen.has(key)) {
				seen.add(key);
				tiles.push(tile);
			}
		}
	};
	record({ x, y }, { x, y }); // the starting tile always counts as trodden

	let arrived = false;
	for (let i = 0; i < stops.length; i++) {
		const stop = stops[i];
		// x axis first, then y — one axis at a time so cats never cut corners.
		const dx = stop.x - x;
		if (dx !== 0 && budget > 0) {
			const move = Math.min(Math.abs(dx), budget);
			const nextX = x + Math.sign(dx) * move;
			record({ x, y }, { x: nextX, y });
			budget -= move;
			x = nextX;
		}
		const dy = stop.y - y;
		if (dy !== 0 && budget > 0) {
			const move = Math.min(Math.abs(dy), budget);
			const nextY = y + Math.sign(dy) * move;
			record({ x, y }, { x, y: nextY });
			budget -= move;
			y = nextY;
		}

		if (x !== stop.x || y !== stop.y) {
			break; // budget spent before reaching this stop — stop here
		}
		if (i === stops.length - 1) {
			arrived = true; // reached the destination itself
		}
		// A waypoint was cleared; continue toward the next stop with what's left.
	}

	return { position: { x, y }, arrived, tiles };
}

/**
 * Pick an idle-wander tile near the anchor from two seeded rolls.
 * Integer tiles so wandering cats settle on tile centers.
 */
export function pickWanderTarget(
	anchor: WorldPos,
	roll1: number,
	roll2: number,
): WorldPos {
	const span = WANDER_RADIUS * 2 + 1;
	return {
		x: anchor.x - WANDER_RADIUS + Math.floor(roll1 * span),
		y: anchor.y - WANDER_RADIUS + Math.floor(roll2 * span),
	};
}

export interface JobDestinationContext {
	anchor: WorldPos;
	shrine: WorldPos;
	/** Known food-rich tiles outside the village (may be empty). */
	foodTiles: WorldPos[];
	/** Walkable shoreline tiles next to water, suitable for fishing. */
	fishingSites?: WorldPos[];
	/** Seeded roll used for any random choice. */
	roll: number;
	/** Construction site for build jobs (colony-translated to world). */
	site?: WorldPos;
	/** Village expansion tile selected by the leader. */
	expansionSite?: WorldPos;
	/** Nearest explored stone tile for a quarry expedition. */
	quarrySite?: WorldPos;
	/** Nearest explored water tile for a water-fetch expedition. */
	waterSite?: WorldPos;
	/** Frontier tile an explore job is dispatched to reveal. */
	exploreSite?: WorldPos;
}

/**
 * Where a cat physically goes for a job. Returns null for jobs with no
 * travel component (player supply actions, leader planning).
 */
export function destinationForJob(
	kind: string,
	context: JobDestinationContext,
): WorldPos | null {
	switch (kind) {
		case "ritual":
			return { ...context.shrine };
		case "build_house":
			return context.site ? { ...context.site } : { ...context.anchor };
		case "expand_village":
			return context.expansionSite ? { ...context.expansionSite } : null;
		case "quarry":
			return context.quarrySite ? { ...context.quarrySite } : null;
		case "fetch_water":
			return context.waterSite ? { ...context.waterSite } : null;
		case "fish": {
			const sites = context.fishingSites ?? [];
			if (sites.length === 0) {
				return null;
			}
			const clamped = Math.min(Math.max(context.roll, 0), 0.999999);
			const index = Math.floor(clamped * sites.length);
			return { ...sites[index] };
		}
		case "explore":
			return context.exploreSite ? { ...context.exploreSite } : null;
		case "hunt_expedition": {
			if (context.foodTiles.length > 0) {
				const clamped = Math.min(Math.max(context.roll, 0), 0.999999);
				const index = Math.floor(clamped * context.foodTiles.length);
				return { ...context.foodTiles[index] };
			}
			// No known food nearby — strike out in a roll-chosen direction.
			const angle = context.roll * Math.PI * 2;
			const range =
				HUNT_RANGE_MIN + context.roll * (HUNT_RANGE_MAX - HUNT_RANGE_MIN);
			return {
				x: Math.round(context.anchor.x + Math.cos(angle) * range),
				y: Math.round(context.anchor.y + Math.sin(angle) * range),
			};
		}
		default:
			return null;
	}
}
