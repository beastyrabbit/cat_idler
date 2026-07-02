/**
 * Player-painted map zones (pure rules).
 *
 * Players steer the colony by marking rectangles: `gather` doubles a
 * tile's appeal, `avoid` excludes it — unless a critical need leaves the
 * cats no other option.
 */

export interface ZoneRect {
	x1: number;
	y1: number;
	x2: number;
	y2: number;
}

export interface Zone extends ZoneRect {
	kind: "avoid" | "gather";
}

export const ZONE_MAX_PER_PLAYER = 2;
export const ZONE_MAX_EDGE = 8;
export const ZONE_MIN_DURATION_MS = 10 * 60 * 1000;
export const ZONE_MAX_DURATION_MS = 2 * 3600 * 1000;
export const GATHER_MULTIPLIER = 2;

/** Order two corners into a normalized rect (integer tiles). */
export function normalizeRect(
	a: { x: number; y: number },
	b: { x: number; y: number },
): ZoneRect {
	return {
		x1: Math.min(Math.round(a.x), Math.round(b.x)),
		y1: Math.min(Math.round(a.y), Math.round(b.y)),
		x2: Math.max(Math.round(a.x), Math.round(b.x)),
		y2: Math.max(Math.round(a.y), Math.round(b.y)),
	};
}

/** Inclusive-edge containment. */
export function isInZone(
	pos: { x: number; y: number },
	zone: ZoneRect,
): boolean {
	return (
		pos.x >= zone.x1 && pos.x <= zone.x2 && pos.y >= zone.y1 && pos.y <= zone.y2
	);
}

/** Returns a user-facing error, or null when the zone is acceptable. */
export function validateZone(
	rect: ZoneRect,
	durationMs: number,
	activePlayerZones: number,
): string | null {
	if (activePlayerZones >= ZONE_MAX_PER_PLAYER) {
		return `You already have ${ZONE_MAX_PER_PLAYER} active zones`;
	}
	if (
		rect.x2 - rect.x1 + 1 > ZONE_MAX_EDGE ||
		rect.y2 - rect.y1 + 1 > ZONE_MAX_EDGE
	) {
		return `Zones are limited to ${ZONE_MAX_EDGE}x${ZONE_MAX_EDGE} tiles`;
	}
	if (durationMs < ZONE_MIN_DURATION_MS || durationMs > ZONE_MAX_DURATION_MS) {
		return "Zone duration must be between 10 minutes and 2 hours";
	}
	return null;
}

/**
 * Zone-adjusted appeal of a tile. Gather zones double the base score;
 * avoid zones zero it unless `critical` (a need leaves no choice).
 */
export function scoreTileWithZones(
	baseScore: number,
	pos: { x: number; y: number },
	zones: Zone[],
	critical = false,
): number {
	let score = baseScore;
	for (const zone of zones) {
		if (!isInZone(pos, zone)) {
			continue;
		}
		if (zone.kind === "avoid" && !critical) {
			return 0;
		}
		if (zone.kind === "gather") {
			score *= GATHER_MULTIPLIER;
		}
	}
	return score;
}

/** Wander/journey candidates with avoid-zone tiles removed. */
export function filterTargetsByZones<T extends { x: number; y: number }>(
	targets: T[],
	zones: Zone[],
	critical = false,
): T[] {
	if (critical) {
		return targets;
	}
	const avoids = zones.filter((zone) => zone.kind === "avoid");
	if (avoids.length === 0) {
		return targets;
	}
	return targets.filter(
		(target) => !avoids.some((zone) => isInZone(target, zone)),
	);
}

/**
 * Weighted pick over candidates: gather-zone tiles appear twice, avoid
 * tiles are gone. Falls back to the raw list when zones filter out
 * everything (critical-need behavior).
 */
export function pickTargetWithZones<T extends { x: number; y: number }>(
	targets: T[],
	zones: Zone[],
	roll: number,
): T | null {
	if (targets.length === 0) {
		return null;
	}
	const allowed = filterTargetsByZones(targets, zones);
	const pool = allowed.length > 0 ? allowed : targets;
	const weighted: T[] = [];
	for (const target of pool) {
		weighted.push(target);
		if (
			zones.some((zone) => zone.kind === "gather" && isInZone(target, zone))
		) {
			weighted.push(target);
		}
	}
	const clamped = Math.min(Math.max(roll, 0), 0.999999);
	return weighted[Math.floor(clamped * weighted.length)];
}
