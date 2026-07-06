/**
 * Deliberate road building — pure selection of the corridor worth paving.
 *
 * Roads are the visible payoff of routine: where cats tread the same ground day
 * after day the wear climbs, and once the leader can spare the materials it
 * paves that trodden corridor into a permanent road (cheaper to walk, so the
 * route entrenches itself). This module only *chooses* the corridor — the
 * highest cumulative-wear connected run outside the village — so the choice is
 * deterministic and unit-testable; the tick does the actual paving.
 */

import type { WorldPos } from "./movement";
import type { WalkTile } from "./pathfinding";

/** Wear at/above which a tile counts as a trafficked, pave-worthy trail. */
export const ROAD_PAVE_WEAR = 70;

export interface RoadCorridorOptions {
	/** Village centre — only ground *outside* the fence ring is paved. */
	anchor: WorldPos;
	/** Chebyshev radius of the fence; interior is already open clearing. */
	ringRadius: number;
	/** Most tiles to pave in one go (also bounded by available materials). */
	maxTiles: number;
	/** Minimum wear a tile needs before it is worth paving. */
	wearThreshold?: number;
}

function cheb(a: WorldPos, b: WorldPos): number {
	return Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y));
}

function isPaved(tile: WalkTile): boolean {
	return tile.overlayFeature === "road_built";
}

/**
 * The most-trafficked unpaved corridor worth paving right now: starting from
 * the single highest-wear trail tile outside the fence, greedily grow a
 * 4-connected run through the next-highest-wear neighbours until the tile
 * budget is spent or the trail peters out. Returns the corridor's tiles (paving
 * order, highest-wear first) or an empty list when nothing clears the
 * threshold. Deterministic: ties break by a stable coordinate key.
 */
export function selectRoadCorridor(
	tiles: WalkTile[],
	options: RoadCorridorOptions,
): WorldPos[] {
	const { anchor, ringRadius, maxTiles } = options;
	const threshold = options.wearThreshold ?? ROAD_PAVE_WEAR;
	if (maxTiles <= 0) {
		return [];
	}

	const candidates = new Map<string, WalkTile>();
	for (const tile of tiles) {
		if (
			!isPaved(tile) &&
			tile.pathWear >= threshold &&
			cheb(tile, anchor) > ringRadius
		) {
			candidates.set(`${tile.x},${tile.y}`, tile);
		}
	}
	if (candidates.size === 0) {
		return [];
	}

	// Stable ordering: wear desc, then coordinate, so the pick is reproducible.
	const byWear = [...candidates.values()].sort(
		(a, b) => b.pathWear - a.pathWear || a.x - b.x || a.y - b.y,
	);

	const corridor: WorldPos[] = [];
	const used = new Set<string>();
	const start = byWear[0];
	corridor.push({ x: start.x, y: start.y });
	used.add(`${start.x},${start.y}`);

	while (corridor.length < maxTiles) {
		// Best unused candidate 4-adjacent to any corridor tile.
		let best: WalkTile | null = null;
		for (const tile of byWear) {
			const key = `${tile.x},${tile.y}`;
			if (used.has(key)) {
				continue;
			}
			const adjacent = corridor.some(
				(c) => Math.abs(c.x - tile.x) + Math.abs(c.y - tile.y) === 1,
			);
			if (adjacent) {
				best = tile;
				break; // byWear is sorted, so the first adjacent is the best
			}
		}
		if (!best) {
			break;
		}
		corridor.push({ x: best.x, y: best.y });
		used.add(`${best.x},${best.y}`);
	}

	return corridor;
}
