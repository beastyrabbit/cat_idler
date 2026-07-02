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
	let budget = Math.max(0, elapsedSec) * speed;
	let { x, y } = position;

	const dx = destination.x - x;
	const stepX = Math.sign(dx) * Math.min(Math.abs(dx), budget);
	x += stepX;
	budget -= Math.abs(stepX);

	const dy = destination.y - y;
	const stepY = Math.sign(dy) * Math.min(Math.abs(dy), budget);
	y += stepY;

	return {
		position: { x, y },
		arrived: x === destination.x && y === destination.y,
	};
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
	/** Seeded roll used for any random choice. */
	roll: number;
	/** Construction site for build jobs (colony-translated to world). */
	site?: WorldPos;
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
