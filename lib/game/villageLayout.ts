/**
 * Village Layout
 *
 * Pure helpers that map the colony-local building grid onto world tile
 * coordinates and pick construction sites that grow the village outward
 * from the central shrine in concentric rings.
 *
 * Colony-local (0, 0) is the shrine cell and corresponds to the world
 * anchor returned by `getColonyPosition()` in worldGen.
 */

export interface GridPos {
	x: number;
	y: number;
}

/** World tile coordinate of the village center (see worldGen.getColonyPosition). */
export const VILLAGE_ANCHOR: GridPos = { x: 6, y: 6 };

/** Colony-local position of the shrine. */
export const SHRINE_LOCAL: GridPos = { x: 0, y: 0 };

/** Default maximum ring the village can spiral out to. */
export const DEFAULT_MAX_RING = 8;

/** Smallest fence-ring radius — the founding clearing always looks roomy. */
export const VILLAGE_MIN_RING = 4;

export function colonyToWorld(local: GridPos): GridPos {
	return { x: VILLAGE_ANCHOR.x + local.x, y: VILLAGE_ANCHOR.y + local.y };
}

export function worldToColony(world: GridPos): GridPos {
	return { x: world.x - VILLAGE_ANCHOR.x, y: world.y - VILLAGE_ANCHOR.y };
}

export function shrineWorldPosition(): GridPos {
	return colonyToWorld(SHRINE_LOCAL);
}

/**
 * Cells at Chebyshev distance `ring` from the shrine, in deterministic
 * order: top row left-to-right, then the two side columns top-to-bottom,
 * then the bottom row left-to-right.
 */
export function ringCells(ring: number): GridPos[] {
	if (ring <= 0) {
		return [{ ...SHRINE_LOCAL }];
	}

	const cells: GridPos[] = [];
	for (let x = -ring; x <= ring; x++) {
		cells.push({ x, y: -ring });
	}
	for (let y = -ring + 1; y <= ring - 1; y++) {
		cells.push({ x: -ring, y });
		cells.push({ x: ring, y });
	}
	for (let x = -ring; x <= ring; x++) {
		cells.push({ x, y: ring });
	}
	return cells;
}

function posKey(p: GridPos): string {
	return `${p.x},${p.y}`;
}

/**
 * Pick the next free construction site (colony-local coords).
 *
 * Fills the innermost ring that still has free cells; `roll` (0..1)
 * selects among that ring's free cells so growth looks organic while
 * staying deterministic under seeded RNG. Returns null when every ring
 * up to `maxRing` is occupied.
 */
export function nextBuildingSite(
	occupied: GridPos[],
	roll: number,
	maxRing: number = DEFAULT_MAX_RING,
	isBlocked?: (local: GridPos) => boolean,
): GridPos | null {
	const taken = new Set(occupied.map(posKey));
	taken.add(posKey(SHRINE_LOCAL));

	for (let ring = 1; ring <= maxRing; ring++) {
		const free = ringCells(ring).filter(
			(cell) => !taken.has(posKey(cell)) && !(isBlocked?.(cell) ?? false),
		);
		if (free.length === 0) {
			continue;
		}
		const clamped = Math.min(Math.max(roll, 0), 0.999999);
		return free[Math.floor(clamped * free.length)];
	}

	return null;
}

/**
 * Radius (in rings) the village footprint occupies for a building count.
 * Ring r holds 8r buildings; the shrine sits at ring 0.
 */
export function villageRadius(buildingCount: number): number {
	let radius = 1;
	let capacity = 8;
	while (buildingCount > capacity) {
		radius += 1;
		capacity += 8 * radius;
	}
	return radius;
}

/**
 * Radius of the fence/clearing ring that encloses the village.
 *
 * The fence always sits one ring beyond the outermost building ring, so the
 * settlement keeps a clear margin inside the palisade and buildings never
 * land on top of the fence. It steps outward as the inner rings fill, never
 * shrinking below {@link VILLAGE_MIN_RING} so the founding clearing looks
 * roomy from the start.
 */
export function villageRingRadius(buildingCount: number): number {
	return Math.max(VILLAGE_MIN_RING, villageRadius(buildingCount) + 1);
}
