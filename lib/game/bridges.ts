import type { WorldPos } from "./movement";
import { findPath, ROAD_COST, type WalkGrid } from "./pathfinding";

export const BRIDGE_MATERIALS_COST = 16;
export const BRIDGE_REFINED_COST = 2;
export const BRIDGE_DETOUR_SAVING_THRESHOLD = 8;

export type BridgeOrientation = "east_west" | "north_south";

export interface BridgeTile {
	x: number;
	y: number;
	type: string;
	overlayFeature?: string | null;
	resources?: { water?: number } | null;
	pathWear: number;
}

export interface ValidBridgePlacement {
	ok: true;
	position: WorldPos;
	orientation: BridgeOrientation;
	banks: [WorldPos, WorldPos];
}

export interface InvalidBridgePlacement {
	ok: false;
	reason: "not_water" | "missing_banks";
}

export type BridgePlacement = ValidBridgePlacement | InvalidBridgePlacement;

export interface BridgeCandidate extends ValidBridgePlacement {
	detourCost: number;
	crossingCost: number;
	saving: number;
	weightedSaving: number;
}

function key(x: number, y: number): string {
	return `${x},${y}`;
}

export function bridgeTileHasWater(tile: BridgeTile | undefined): boolean {
	return Boolean(
		tile &&
		(tile.type === "river" ||
			tile.overlayFeature === "river" ||
			(tile.resources?.water ?? 0) > 0),
	);
}

function landAt(tiles: Map<string, BridgeTile>, x: number, y: number): boolean {
	return tiles.has(key(x, y)) && !bridgeTileHasWater(tiles.get(key(x, y)));
}

function validateBridgePlacementInMap(
	byKey: Map<string, BridgeTile>,
	position: WorldPos,
): BridgePlacement {
	const x = Math.round(position.x);
	const y = Math.round(position.y);
	if (!bridgeTileHasWater(byKey.get(key(x, y)))) {
		return { ok: false, reason: "not_water" };
	}

	const eastWest = landAt(byKey, x - 1, y) && landAt(byKey, x + 1, y);
	const northSouth = landAt(byKey, x, y - 1) && landAt(byKey, x, y + 1);
	if (eastWest) {
		return {
			ok: true,
			position: { x, y },
			orientation: "east_west",
			banks: [
				{ x: x - 1, y },
				{ x: x + 1, y },
			],
		};
	}
	if (northSouth) {
		return {
			ok: true,
			position: { x, y },
			orientation: "north_south",
			banks: [
				{ x, y: y - 1 },
				{ x, y: y + 1 },
			],
		};
	}
	return { ok: false, reason: "missing_banks" };
}

export function validateBridgePlacement(
	tiles: BridgeTile[],
	position: WorldPos,
): BridgePlacement {
	const byKey = new Map(tiles.map((tile) => [key(tile.x, tile.y), tile]));
	return validateBridgePlacementInMap(byKey, position);
}

function pathCost(path: WorldPos[], grid: WalkGrid): number {
	let cost = 0;
	for (let i = 1; i < path.length; i += 1) {
		cost += grid.cost(path[i].x, path[i].y);
	}
	return cost;
}

export function selectBestBridgeCandidate(params: {
	tiles: BridgeTile[];
	grid: WalkGrid;
	existingBridgePositions?: WorldPos[];
	isExplored?: (tile: BridgeTile) => boolean;
}): BridgeCandidate | null {
	const existing = new Set(
		(params.existingBridgePositions ?? []).map((p) =>
			key(Math.round(p.x), Math.round(p.y)),
		),
	);
	const byKey = new Map(
		params.tiles.map((tile) => [key(tile.x, tile.y), tile]),
	);
	let best: BridgeCandidate | null = null;

	for (const tile of params.tiles) {
		if (!bridgeTileHasWater(tile)) {
			continue;
		}
		if (existing.has(key(tile.x, tile.y))) {
			continue;
		}
		if (params.isExplored && !params.isExplored(tile)) {
			continue;
		}
		const placement = validateBridgePlacementInMap(byKey, tile);
		if (!placement.ok) {
			continue;
		}
		const detour = findPath(
			placement.banks[0],
			placement.banks[1],
			params.grid,
			{
				margin: 24,
			},
		);
		if (!detour) {
			continue;
		}
		const detourCost = pathCost(detour, params.grid);
		const crossingCost =
			ROAD_COST + params.grid.cost(placement.banks[1].x, placement.banks[1].y);
		const saving = detourCost - crossingCost;
		const bankWear =
			(tile.pathWear ?? 0) +
			(byKey.get(key(placement.banks[0].x, placement.banks[0].y))?.pathWear ??
				0) +
			(byKey.get(key(placement.banks[1].x, placement.banks[1].y))?.pathWear ??
				0);
		const weightedSaving = saving * (1 + Math.min(200, bankWear) / 200);
		if (weightedSaving <= 0) {
			continue;
		}
		const candidate: BridgeCandidate = {
			...placement,
			detourCost,
			crossingCost,
			saving,
			weightedSaving,
		};
		if (
			!best ||
			candidate.weightedSaving > best.weightedSaving ||
			(candidate.weightedSaving === best.weightedSaving &&
				(candidate.position.x < best.position.x ||
					(candidate.position.x === best.position.x &&
						candidate.position.y < best.position.y)))
		) {
			best = candidate;
		}
	}

	return best;
}
