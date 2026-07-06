import type { WorldPos } from "./movement";
import type { GatePlacement, VillageArea } from "./villageArea";
import { fenceBlocksMove } from "./villageArea";

export interface ShorelineTile {
	x: number;
	y: number;
	type: string;
	overlayFeature?: string | null;
	resources?: { water?: number };
}

export interface ShorelineOptions<T extends ShorelineTile> {
	tiles: readonly T[];
	anchor: WorldPos;
	isExplored: (tile: T) => boolean;
	area?: VillageArea;
	gate?: GatePlacement | null;
}

const ORTHOGONAL: ReadonlyArray<readonly [number, number]> = [
	[1, 0],
	[-1, 0],
	[0, 1],
	[0, -1],
];

export function tileHasFishableWater(tile: ShorelineTile): boolean {
	return (
		tile.type === "river" ||
		tile.overlayFeature === "river" ||
		(tile.resources?.water ?? 0) > 0
	);
}

function cheb(anchor: WorldPos, tile: WorldPos): number {
	return Math.max(Math.abs(tile.x - anchor.x), Math.abs(tile.y - anchor.y));
}

/** Walkable explored land tiles orthogonally adjacent to known water. */
export function shorelineFishingSites<T extends ShorelineTile>({
	tiles,
	anchor,
	isExplored,
	area,
	gate,
}: ShorelineOptions<T>): WorldPos[] {
	const byKey = new Map(tiles.map((tile) => [`${tile.x},${tile.y}`, tile]));
	const candidates = new Map<string, WorldPos>();

	for (const water of tiles) {
		if (!tileHasFishableWater(water) || !isExplored(water)) {
			continue;
		}
		for (const [dx, dy] of ORTHOGONAL) {
			const land = byKey.get(`${water.x + dx},${water.y + dy}`);
			if (!land || tileHasFishableWater(land) || !isExplored(land)) {
				continue;
			}
			if (
				area &&
				fenceBlocksMove(
					{ x: land.x, y: land.y },
					{ x: water.x, y: water.y },
					area,
					gate,
				)
			) {
				continue;
			}
			candidates.set(`${land.x},${land.y}`, { x: land.x, y: land.y });
		}
	}

	return [...candidates.values()].sort(
		(a, b) => cheb(anchor, a) - cheb(anchor, b) || a.y - b.y || a.x - b.x,
	);
}
