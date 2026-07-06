import type { WorldPos } from "./movement";

/** Tile-space direction bits shared by elevation pathing and flat affordances. */
export const ELEVATION_DIR = { E: 1, W: 2, N: 4, S: 8 } as const;

type ElevationDirection = keyof typeof ELEVATION_DIR;

const DIRECTIONS: ReadonlyArray<{
	dir: ElevationDirection;
	dx: number;
	dy: number;
}> = [
	{ dir: "E", dx: 1, dy: 0 },
	{ dir: "W", dx: -1, dy: 0 },
	{ dir: "N", dx: 0, dy: -1 },
	{ dir: "S", dx: 0, dy: 1 },
];

export interface ElevationField {
	heightAt?(x: number, y: number): number;
	hasStair?(x: number, y: number): boolean;
}

function floorAt(field: ElevationField, x: number, y: number): number {
	return field.heightAt?.(x, y) ?? 0;
}

/** A stair on either endpoint bridges a single-floor step between the tiles. */
export function stairBridgesStep(
	field: ElevationField,
	ax: number,
	ay: number,
	bx: number,
	by: number,
): boolean {
	if (!field.heightAt || !field.hasStair) {
		return false;
	}
	if (Math.abs(floorAt(field, ax, ay) - floorAt(field, bx, by)) !== 1) {
		return false;
	}
	return field.hasStair(ax, ay) || field.hasStair(bx, by);
}

/**
 * Elevation blocks every floor-changing edge unless a stair bridges exactly one
 * floor. Without a height sampler the world remains flat and never blocks.
 */
export function elevationBlocksStep(
	field: ElevationField,
	ax: number,
	ay: number,
	bx: number,
	by: number,
): boolean {
	if (!field.heightAt) {
		return false;
	}
	const delta = Math.abs(floorAt(field, ax, ay) - floorAt(field, bx, by));
	if (delta === 0) {
		return false;
	}
	return !stairBridgesStep(field, ax, ay, bx, by);
}

/**
 * Mask of blocked downhill edges around one tile. Rendering this on the higher
 * tile creates a flat ridge line wherever pathing would reject the crossing.
 */
export function blockedElevationEdgeMask(
	pos: WorldPos,
	field: ElevationField,
): number {
	if (!field.heightAt) {
		return 0;
	}
	const here = floorAt(field, pos.x, pos.y);
	let mask = 0;
	for (const { dir, dx, dy } of DIRECTIONS) {
		const there = floorAt(field, pos.x + dx, pos.y + dy);
		if (
			there < here &&
			elevationBlocksStep(field, pos.x, pos.y, pos.x + dx, pos.y + dy)
		) {
			mask |= ELEVATION_DIR[dir];
		}
	}
	return mask;
}

/** Mask of stair-connected edges around one tile, used by the flat hatch mark. */
export function stairElevationEdgeMask(
	pos: WorldPos,
	field: ElevationField,
): number {
	if (!field.heightAt || !field.hasStair?.(pos.x, pos.y)) {
		return 0;
	}
	let mask = 0;
	for (const { dir, dx, dy } of DIRECTIONS) {
		if (stairBridgesStep(field, pos.x, pos.y, pos.x + dx, pos.y + dy)) {
			mask |= ELEVATION_DIR[dir];
		}
	}
	return mask;
}
