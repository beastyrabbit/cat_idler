/**
 * Terrain Generation Core (Isometric Nature pack)
 *
 * Pure, deterministic terrain generation for the map-first overhaul described in
 * `docs/TERRAIN_DESIGN.md`. This module owns the *abstract role* layer only: it
 * emits height levels, cliff/stair auto-tiling roles, biome roles, decoration
 * roles and river segments. It does **not** know sprite filenames — the opaque
 * `naturePack_NNN_R` numbering is resolved later (via `/dev/tiles` annotations)
 * by mapping the roles emitted here onto pack sprites + rotations.
 *
 * Everything is a pure function of `(worldCoords, seed, opts)`:
 *   - Same inputs always produce the same output.
 *   - Height and biome fields are sampled from *world* coordinates, so chunk
 *     borders line up exactly (no seams).
 *
 * There are NO side effects and NO imports from `worldGen.ts` — the noise
 * approach is copied from `lib/game/noise.ts` and kept self-contained so this
 * generator is independent of the legacy Voronoi world generator.
 */

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Chunk edge length in tiles (matches the map viewport's 12x12 chunking). */
export const TERRAIN_CHUNK_SIZE = 12;

/** Highest floor index; heights are quantized to 0..DEFAULT_MAX_HEIGHT. */
export const DEFAULT_MAX_HEIGHT = 3;

// ---------------------------------------------------------------------------
// Directions
// ---------------------------------------------------------------------------

export type Direction = "N" | "E" | "S" | "W";

/** Iteration order is fixed so bitmasks and facings are deterministic. */
export const DIRECTIONS: readonly Direction[] = ["N", "E", "S", "W"];

/** 4-bit edge-mask bit per direction (N=1, E=2, S=4, W=8). */
const DIR_BIT: Record<Direction, number> = { N: 1, E: 2, S: 4, W: 8 };

const DIR_VEC: Record<Direction, { dx: number; dy: number }> = {
	N: { dx: 0, dy: -1 },
	E: { dx: 1, dy: 0 },
	S: { dx: 0, dy: 1 },
	W: { dx: -1, dy: 0 },
};

const OPPOSITE: Record<Direction, Direction> = {
	N: "S",
	E: "W",
	S: "N",
	W: "E",
};

/** Adjacent direction pairs and their outer-corner names, clockwise. */
const CORNER_PAIRS: ReadonlyArray<readonly [Direction, Direction, string]> = [
	["N", "E", "NE"],
	["E", "S", "SE"],
	["S", "W", "SW"],
	["W", "N", "NW"],
];

// ---------------------------------------------------------------------------
// Role vocabulary (what a renderer later maps to sprites + rotations)
// ---------------------------------------------------------------------------

export type BiomeRole =
	| "lowland"
	| "grassland"
	| "forest"
	| "rocky"
	| "highland";

/** Base cliff shape before compass orientation. */
export type CliffBase = "edge" | "corner" | "ridge" | "spur" | "pillar";

export interface FlatTerrainRole {
	kind: "flat";
}

export interface CliffTerrainRole {
	kind: "cliff";
	/** Bitmask of orthogonal neighbors that are strictly lower (N=1,E=2,S=4,W=8). */
	edges: number;
	/** Base shape (used to pick the sprite family). */
	base: CliffBase;
	/**
	 * Oriented role string a renderer keys off, e.g. `edge-N`, `corner-NE`,
	 * `ridge-NS`, `spur-W`, `pillar`.
	 */
	variant: string;
	/** Primary downhill facing (compass), or null for a pillar. */
	facing: Direction | null;
	/**
	 * Deepest single drop to a lower orthogonal neighbor (in floors, >= 1). A
	 * renderer stacks this many one-floor cliff blocks so a multi-floor drop
	 * reads as one continuous wall down to the lowest neighbor.
	 */
	maxDrop: number;
}

export type TerrainRole = FlatTerrainRole | CliffTerrainRole;

export type RockSize = "small" | "medium" | "large";

export type DecorationRole =
	| { kind: "tree"; species: number }
	| { kind: "rock"; size: RockSize; resource: boolean };

export type RiverSegment = "start" | "straight" | "bend" | "end";

export interface RiverRole {
	kind: "river";
	segment: RiverSegment;
	/** Where the water comes from (points upstream); null at the source. */
	inDir: Direction | null;
	/** Where the water flows to (points downstream); null at the mouth. */
	outDir: Direction | null;
	/** Convenience facing: outflow for start/straight/bend, inflow for end. */
	facing: Direction;
}

export interface StairsRole {
	kind: "stairs";
	/** Descent direction — the tile one step this way is exactly one floor lower. */
	facing: Direction;
}

export interface TerrainTile {
	x: number;
	y: number;
	/** Continuous elevation in [0,1) (kept for biome moisture/renderer blends). */
	elevation: number;
	/** Continuous moisture in [0,1). */
	moisture: number;
	/** Quantized floor level 0..maxHeight. */
	height: number;
	biome: BiomeRole;
	terrain: TerrainRole;
	river?: RiverRole;
	stairs?: StairsRole;
	decoration?: DecorationRole;
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface TerrainOptions {
	/** Highest floor index (levels 0..maxHeight). Default 3. */
	maxHeight?: number;
	/** Base spatial frequency of the elevation field. Default 0.08. */
	heightScale?: number;
	/** Fractal octaves for elevation. Default 4. */
	octaves?: number;
	/** Persistence (amplitude falloff per octave). Default 0.5. */
	persistence?: number;
	/** Spatial frequency of the moisture field. Default 0.06. */
	moistureScale?: number;
	/** Village anchor; a flat plateau is guaranteed around it. Default {0,0}. */
	villageAnchor?: { x: number; y: number };
	/** Chebyshev radius of the guaranteed flat plateau. Default 4. */
	plateauRadius?: number;
	/** Floor level the plateau sits on. Default 1. */
	plateauHeight?: number;
	/** Minimum contiguous cliff-edge run length to place a stair. Default 3. */
	minRunForStair?: number;
	/** Region cell size (tiles) for river source selection. Default 24. */
	regionSize?: number;
	/** Rivers sourced per region cell. Default 1. */
	riversPerRegion?: number;
	/** Maximum river path length in tiles. Default 36. */
	maxRiverLength?: number;
	/** Elevation a river source must exceed (in [0,1)). Default 0.6. */
	riverSourceMinElevation?: number;
	/** If true, river tiles report height 0 (carved). Default false — see note. */
	carveRivers?: boolean;
	/** If true, scatter tree/rock decoration on flat tiles. Default true. */
	decorate?: boolean;
}

/**
 * Terrain options shared by the server world generator (`server/worldMap.ts`)
 * and the client renderer (`components/map/TileLayer.tsx`) so gameplay tiles and
 * on-screen sprites are sampled from the *exact same* height/biome field — no
 * drift between what a cat can walk and what a player sees. The village anchor
 * (world 6,6, see `getColonyPosition`) gets a flat plateau; rivers steer clear
 * of it. Anything not set here uses the module defaults.
 */
export const WORLD_TERRAIN_OPTIONS: TerrainOptions = {
	villageAnchor: { x: 6, y: 6 },
	plateauRadius: 8,
	plateauHeight: 1,
};

interface ResolvedOptions extends Required<TerrainOptions> {}

function resolveOptions(opts: TerrainOptions): ResolvedOptions {
	return {
		maxHeight: opts.maxHeight ?? DEFAULT_MAX_HEIGHT,
		heightScale: opts.heightScale ?? 0.08,
		octaves: opts.octaves ?? 4,
		persistence: opts.persistence ?? 0.5,
		moistureScale: opts.moistureScale ?? 0.06,
		villageAnchor: opts.villageAnchor ?? { x: 0, y: 0 },
		plateauRadius: opts.plateauRadius ?? 4,
		plateauHeight: opts.plateauHeight ?? 1,
		minRunForStair: opts.minRunForStair ?? 3,
		regionSize: opts.regionSize ?? 24,
		riversPerRegion: opts.riversPerRegion ?? 1,
		maxRiverLength: opts.maxRiverLength ?? 36,
		riverSourceMinElevation: opts.riverSourceMinElevation ?? 0.6,
		carveRivers: opts.carveRivers ?? false,
		decorate: opts.decorate ?? true,
	};
}

// ---------------------------------------------------------------------------
// Self-contained seeded value noise (approach copied from lib/game/noise.ts)
// ---------------------------------------------------------------------------

/** Deterministic string/number hash → non-negative 32-bit-ish integer. */
function hashSeed(...values: (number | string)[]): number {
	let hash = 0;
	for (const value of values) {
		const str = String(value);
		for (let i = 0; i < str.length; i++) {
			hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
			hash = hash & hash;
		}
	}
	return Math.abs(hash);
}

/** Deterministic per-lattice-point value in [0,1). */
function latticeValue(seed: number, ix: number, iy: number): number {
	let h = hashSeed(seed, ix, iy) >>> 0;
	h = (h ^ (h >>> 13)) >>> 0;
	h = Math.imul(h, 1274126177) >>> 0;
	h = (h ^ (h >>> 16)) >>> 0;
	return h / 4294967296;
}

/** Smoothstep fade for value-noise interpolation. */
function fade(t: number): number {
	return t * t * (3 - 2 * t);
}

/** Single-octave bilinear value noise, in [0,1). */
function valueNoise(x: number, y: number, seed: number, scale: number): number {
	const sx = x * scale;
	const sy = y * scale;
	const x0 = Math.floor(sx);
	const y0 = Math.floor(sy);
	const fx = fade(sx - x0);
	const fy = fade(sy - y0);

	const n00 = latticeValue(seed, x0, y0);
	const n10 = latticeValue(seed, x0 + 1, y0);
	const n01 = latticeValue(seed, x0, y0 + 1);
	const n11 = latticeValue(seed, x0 + 1, y0 + 1);

	const nx0 = n00 + (n10 - n00) * fx;
	const nx1 = n01 + (n11 - n01) * fx;
	return nx0 + (nx1 - nx0) * fy;
}

/** Fractal (multi-octave) value noise, in [0,1). */
function fractalNoise(
	x: number,
	y: number,
	seed: number,
	octaves: number,
	persistence: number,
	scale: number,
): number {
	let value = 0;
	let amplitude = 1;
	let frequency = scale;
	let maxValue = 0;
	for (let i = 0; i < octaves; i++) {
		value += valueNoise(x, y, seed + i * 1013, frequency) * amplitude;
		maxValue += amplitude;
		amplitude *= persistence;
		frequency *= 2;
	}
	return value / maxValue;
}

// ---------------------------------------------------------------------------
// Height + moisture fields (world-coordinate based → cross-chunk consistent)
// ---------------------------------------------------------------------------

/** Continuous elevation in [0,1) at a world tile. */
export function terrainElevationAt(
	x: number,
	y: number,
	seed: number,
	opts: TerrainOptions = {},
): number {
	const o = resolveOptions(opts);
	return fractalNoise(x, y, seed, o.octaves, o.persistence, o.heightScale);
}

/** Continuous moisture in [0,1) at a world tile. */
export function terrainMoistureAt(
	x: number,
	y: number,
	seed: number,
	opts: TerrainOptions = {},
): number {
	const o = resolveOptions(opts);
	return fractalNoise(x, y, seed ^ 0x9e3779b9, 3, 0.5, o.moistureScale);
}

/** Chebyshev distance from the village anchor. */
function anchorChebyshev(x: number, y: number, o: ResolvedOptions): number {
	return Math.max(
		Math.abs(x - o.villageAnchor.x),
		Math.abs(y - o.villageAnchor.y),
	);
}

function isInPlateau(x: number, y: number, o: ResolvedOptions): boolean {
	return anchorChebyshev(x, y, o) <= o.plateauRadius;
}

function heightWith(
	x: number,
	y: number,
	seed: number,
	o: ResolvedOptions,
): number {
	if (isInPlateau(x, y, o)) {
		return o.plateauHeight;
	}
	const e = fractalNoise(x, y, seed, o.octaves, o.persistence, o.heightScale);
	const level = Math.floor(e * (o.maxHeight + 1));
	return Math.max(0, Math.min(o.maxHeight, level));
}

/** Quantized floor level 0..maxHeight at a world tile (plateau-aware). */
export function terrainHeightAt(
	x: number,
	y: number,
	seed: number,
	opts: TerrainOptions = {},
): number {
	return heightWith(x, y, seed, resolveOptions(opts));
}

/** Whether a world tile carries a staircase (pathing's only cliff crossing). */
export function terrainStairAt(
	x: number,
	y: number,
	seed: number,
	opts: TerrainOptions = {},
): boolean {
	return deriveStairs(x, y, seed, resolveOptions(opts)) !== undefined;
}

// ---------------------------------------------------------------------------
// Cliff autotiling (bitmask → oriented role)
// ---------------------------------------------------------------------------

/**
 * Classify a tile's terrain role from its height and its four orthogonal
 * neighbor heights. Pure — the fixture tests drive this directly.
 */
export function classifyCliff(
	center: number,
	neighbors: Record<Direction, number>,
): TerrainRole {
	let edges = 0;
	const lower: Direction[] = [];
	for (const dir of DIRECTIONS) {
		if (neighbors[dir] < center) {
			edges |= DIR_BIT[dir];
			lower.push(dir);
		}
	}
	if (edges === 0) {
		return { kind: "flat" };
	}
	const maxDrop = center - Math.min(...lower.map((d) => neighbors[d]));
	return maskToCliff(edges, lower, maxDrop);
}

function maskToCliff(
	edges: number,
	lower: Direction[],
	maxDrop: number,
): CliffTerrainRole {
	const count = lower.length;

	if (count === 1) {
		const facing = lower[0];
		return {
			kind: "cliff",
			edges,
			base: "edge",
			variant: `edge-${facing}`,
			facing,
			maxDrop,
		};
	}

	if (count === 4) {
		return {
			kind: "cliff",
			edges,
			base: "pillar",
			variant: "pillar",
			facing: null,
			maxDrop,
		};
	}

	if (count === 2) {
		const [a, b] = lower;
		// Opposite pair → ridge (a straight spine, lower on two facing sides).
		if (OPPOSITE[a] === b) {
			const axis = a === "N" || a === "S" ? "NS" : "EW";
			const facing: Direction = axis === "NS" ? "N" : "E";
			return {
				kind: "cliff",
				edges,
				base: "ridge",
				variant: `ridge-${axis}`,
				facing,
				maxDrop,
			};
		}
		// Adjacent pair → outer corner.
		for (const [d1, d2, name] of CORNER_PAIRS) {
			if ((a === d1 && b === d2) || (a === d2 && b === d1)) {
				return {
					kind: "cliff",
					edges,
					base: "corner",
					variant: `corner-${name}`,
					facing: d1,
					maxDrop,
				};
			}
		}
	}

	// count === 3 → spur: cliff on three sides, connected on the single higher side.
	const higher = DIRECTIONS.find((d) => !lower.includes(d)) ?? null;
	return {
		kind: "cliff",
		edges,
		base: "spur",
		variant: higher ? `spur-${higher}` : "spur",
		facing: higher,
		maxDrop,
	};
}

function terrainRoleAt(
	x: number,
	y: number,
	seed: number,
	o: ResolvedOptions,
): TerrainRole {
	const center = heightWith(x, y, seed, o);
	const neighbors = {} as Record<Direction, number>;
	for (const dir of DIRECTIONS) {
		const v = DIR_VEC[dir];
		neighbors[dir] = heightWith(x + v.dx, y + v.dy, seed, o);
	}
	return classifyCliff(center, neighbors);
}

// ---------------------------------------------------------------------------
// Stairs (place on straight single-floor cliff runs)
// ---------------------------------------------------------------------------

/**
 * If this tile is a single-floor `edge` cliff, return its descent direction,
 * else null. A stair edge drops exactly one floor so it can carry a staircase.
 */
function stairEdgeDir(
	x: number,
	y: number,
	seed: number,
	o: ResolvedOptions,
): Direction | null {
	const role = terrainRoleAt(x, y, seed, o);
	if (role.kind !== "cliff" || role.base !== "edge" || role.facing === null) {
		return null;
	}
	const v = DIR_VEC[role.facing];
	const center = heightWith(x, y, seed, o);
	const below = heightWith(x + v.dx, y + v.dy, seed, o);
	return center - below === 1 ? role.facing : null;
}

/** Perpendicular scan directions for a given descent facing. */
function perpAxis(facing: Direction): { neg: Direction; pos: Direction } {
	// N/S descents run along the E-W axis; E/W descents along the N-S axis.
	if (facing === "N" || facing === "S") {
		return { neg: "W", pos: "E" };
	}
	return { neg: "N", pos: "S" };
}

const MAX_RUN_SCAN = 64;

function deriveStairs(
	x: number,
	y: number,
	seed: number,
	o: ResolvedOptions,
): StairsRole | undefined {
	const facing = stairEdgeDir(x, y, seed, o);
	if (!facing) {
		return undefined;
	}
	const { neg, pos } = perpAxis(facing);
	const negV = DIR_VEC[neg];
	const posV = DIR_VEC[pos];

	// Walk to the run anchor (first same-facing edge tile scanning `neg`).
	let ax = x;
	let ay = y;
	for (let i = 0; i < MAX_RUN_SCAN; i++) {
		if (stairEdgeDir(ax + negV.dx, ay + negV.dy, seed, o) === facing) {
			ax += negV.dx;
			ay += negV.dy;
		} else {
			break;
		}
	}

	// Count the run length forward from the anchor along `pos`.
	let length = 1;
	let cx = ax + posV.dx;
	let cy = ay + posV.dy;
	for (let i = 0; i < MAX_RUN_SCAN; i++) {
		if (stairEdgeDir(cx, cy, seed, o) === facing) {
			length++;
			cx += posV.dx;
			cy += posV.dy;
		} else {
			break;
		}
	}

	if (length < o.minRunForStair) {
		return undefined;
	}

	// One stair per run, placed at its midpoint (deterministic).
	const chosenIndex = Math.floor((length - 1) / 2);
	const index = (x - ax) * posV.dx + (y - ay) * posV.dy;
	return index === chosenIndex ? { kind: "stairs", facing } : undefined;
}

// ---------------------------------------------------------------------------
// Rivers (per-region source, steepest-descent path, oriented segments)
// ---------------------------------------------------------------------------

interface RiverPathTile {
	x: number;
	y: number;
	inDir: Direction | null;
	outDir: Direction | null;
}

/** Pick river source tiles for one region cell (highest local elevation). */
export function regionRiverSources(
	regionX: number,
	regionY: number,
	seed: number,
	opts: TerrainOptions = {},
): { x: number; y: number }[] {
	const o = resolveOptions(opts);
	const baseSeed = hashSeed(seed, "river", regionX, regionY);
	const originX = regionX * o.regionSize;
	const originY = regionY * o.regionSize;
	const sources: { x: number; y: number }[] = [];

	for (let n = 0; n < o.riversPerRegion; n++) {
		// Sample a handful of candidates, keep the highest that clears threshold.
		let best: { x: number; y: number; e: number } | null = null;
		for (let s = 0; s < 8; s++) {
			const r1 = latticeValue(baseSeed + n * 131, s * 2, 0);
			const r2 = latticeValue(baseSeed + n * 131, s * 2 + 1, 1);
			const x = originX + Math.floor(r1 * o.regionSize);
			const y = originY + Math.floor(r2 * o.regionSize);
			if (isInPlateau(x, y, o)) {
				continue;
			}
			const e = fractalNoise(
				x,
				y,
				seed,
				o.octaves,
				o.persistence,
				o.heightScale,
			);
			if (!best || e > best.e) {
				best = { x, y, e };
			}
		}
		if (best && best.e >= o.riverSourceMinElevation) {
			sources.push({ x: best.x, y: best.y });
		}
	}
	return sources;
}

/**
 * Trace a river from a source via steepest descent on the continuous elevation
 * field. Strictly monotonic (each step is strictly lower), so it never loops.
 * Returns null if the source has no downhill neighbor (path too short to draw).
 */
export function traceRiver(
	sx: number,
	sy: number,
	seed: number,
	opts: TerrainOptions = {},
): RiverPathTile[] | null {
	const o = resolveOptions(opts);
	const path: RiverPathTile[] = [{ x: sx, y: sy, inDir: null, outDir: null }];

	let cx = sx;
	let cy = sy;
	let cElev = fractalNoise(
		cx,
		cy,
		seed,
		o.octaves,
		o.persistence,
		o.heightScale,
	);

	for (let step = 0; step < o.maxRiverLength; step++) {
		let bestDir: Direction | null = null;
		let bestElev = cElev;
		for (const dir of DIRECTIONS) {
			const v = DIR_VEC[dir];
			const nx = cx + v.dx;
			const ny = cy + v.dy;
			if (isInPlateau(nx, ny, o)) {
				continue; // rivers never enter the village plateau
			}
			const e = fractalNoise(
				nx,
				ny,
				seed,
				o.octaves,
				o.persistence,
				o.heightScale,
			);
			if (e < bestElev) {
				bestElev = e;
				bestDir = dir;
			}
		}
		if (!bestDir) {
			break; // reached a local minimum (mouth)
		}
		const v = DIR_VEC[bestDir];
		path[path.length - 1].outDir = bestDir;
		cx += v.dx;
		cy += v.dy;
		cElev = bestElev;
		path.push({ x: cx, y: cy, inDir: OPPOSITE[bestDir], outDir: null });
	}

	return path.length >= 2 ? path : null;
}

/** Classify a river path tile into an oriented segment role. */
export function classifyRiverSegment(tile: RiverPathTile): RiverRole {
	const { inDir, outDir } = tile;
	if (inDir === null && outDir !== null) {
		return { kind: "river", segment: "start", inDir, outDir, facing: outDir };
	}
	if (outDir === null && inDir !== null) {
		return { kind: "river", segment: "end", inDir, outDir, facing: inDir };
	}
	if (inDir !== null && outDir !== null) {
		// Straight when the outflow continues opposite the inflow, else a bend.
		const segment: RiverSegment =
			outDir === OPPOSITE[inDir] ? "straight" : "bend";
		return { kind: "river", segment, inDir, outDir, facing: outDir };
	}
	// Degenerate single-tile river; treat as a start with no flow.
	return { kind: "river", segment: "start", inDir, outDir, facing: "N" };
}

const RIVER_KEY = (x: number, y: number) => `${x},${y}`;

/** Collect all river path tiles that fall inside the given chunk. */
function collectRiverTiles(
	chunkX: number,
	chunkY: number,
	seed: number,
	o: ResolvedOptions,
): Map<string, RiverPathTile> {
	const originX = chunkX * TERRAIN_CHUNK_SIZE;
	const originY = chunkY * TERRAIN_CHUNK_SIZE;
	const reach = o.maxRiverLength;

	const regionMinX = Math.floor((originX - reach) / o.regionSize);
	const regionMaxX = Math.floor(
		(originX + TERRAIN_CHUNK_SIZE + reach) / o.regionSize,
	);
	const regionMinY = Math.floor((originY - reach) / o.regionSize);
	const regionMaxY = Math.floor(
		(originY + TERRAIN_CHUNK_SIZE + reach) / o.regionSize,
	);

	const result = new Map<string, RiverPathTile>();
	for (let rx = regionMinX; rx <= regionMaxX; rx++) {
		for (let ry = regionMinY; ry <= regionMaxY; ry++) {
			const sources = regionRiverSources(rx, ry, seed, o);
			for (const src of sources) {
				const path = traceRiver(src.x, src.y, seed, o);
				if (!path) {
					continue;
				}
				for (const t of path) {
					if (
						t.x >= originX &&
						t.x < originX + TERRAIN_CHUNK_SIZE &&
						t.y >= originY &&
						t.y < originY + TERRAIN_CHUNK_SIZE
					) {
						result.set(RIVER_KEY(t.x, t.y), t);
					}
				}
			}
		}
	}
	return result;
}

// ---------------------------------------------------------------------------
// Biomes + decoration
// ---------------------------------------------------------------------------

export function classifyBiome(
	height: number,
	maxHeight: number,
	moisture: number,
): BiomeRole {
	if (height <= 0) {
		return "lowland";
	}
	if (height >= maxHeight) {
		return "highland";
	}
	if (moisture > 0.6) {
		return "forest";
	}
	if (moisture < 0.33) {
		return "rocky";
	}
	return "grassland";
}

interface DecorDensity {
	tree: number;
	rock: number;
}

const BIOME_DECOR: Record<BiomeRole, DecorDensity> = {
	lowland: { tree: 0.05, rock: 0.02 },
	grassland: { tree: 0.08, rock: 0.03 },
	forest: { tree: 0.45, rock: 0.05 },
	rocky: { tree: 0.03, rock: 0.35 },
	highland: { tree: 0.02, rock: 0.15 },
};

function deriveDecoration(
	x: number,
	y: number,
	seed: number,
	biome: BiomeRole,
): DecorationRole | undefined {
	const density = BIOME_DECOR[biome];
	const roll = latticeValue(hashSeed(seed, "decor", x, y), 0, 0);

	if (roll < density.tree) {
		const speciesRoll = latticeValue(hashSeed(seed, "species", x, y), 1, 1);
		return { kind: "tree", species: Math.floor(speciesRoll * 4) };
	}
	if (roll < density.tree + density.rock) {
		const sizeRoll = latticeValue(hashSeed(seed, "rock", x, y), 2, 2);
		const size: RockSize =
			sizeRoll < 0.5 ? "small" : sizeRoll < 0.85 ? "medium" : "large";
		const resourceRoll = latticeValue(hashSeed(seed, "ore", x, y), 3, 3);
		return { kind: "rock", size, resource: resourceRoll < 0.4 };
	}
	return undefined;
}

// ---------------------------------------------------------------------------
// Chunk assembly
// ---------------------------------------------------------------------------

/**
 * Generate a full 12x12 terrain chunk of abstract roles.
 *
 * Deterministic: identical `(chunkX, chunkY, seed, opts)` always yields an
 * identical array. Cross-chunk consistent: height/moisture/cliff/stair fields
 * are sampled from world coordinates, so a shared border column is identical
 * from either chunk.
 *
 * River limitation: rivers are sourced *per region cell* and traced by steepest
 * descent, so a path is only guaranteed continuous within `maxRiverLength` of
 * its source. A chunk enumerates every region within `maxRiverLength`, so any
 * river passing through it is found; but a river does not connect across region
 * cells whose sources are farther than that reach.
 */
export function generateTerrainChunk(
	chunkX: number,
	chunkY: number,
	seed: number,
	opts: TerrainOptions = {},
): TerrainTile[] {
	const o = resolveOptions(opts);
	const originX = chunkX * TERRAIN_CHUNK_SIZE;
	const originY = chunkY * TERRAIN_CHUNK_SIZE;
	const rivers = collectRiverTiles(chunkX, chunkY, seed, o);

	const tiles: TerrainTile[] = [];
	for (let ly = 0; ly < TERRAIN_CHUNK_SIZE; ly++) {
		for (let lx = 0; lx < TERRAIN_CHUNK_SIZE; lx++) {
			const x = originX + lx;
			const y = originY + ly;

			const elevation = fractalNoise(
				x,
				y,
				seed,
				o.octaves,
				o.persistence,
				o.heightScale,
			);
			const moisture = terrainMoistureAt(x, y, seed, o);
			let height = heightWith(x, y, seed, o);
			const terrain = terrainRoleAt(x, y, seed, o);

			const riverTile = rivers.get(RIVER_KEY(x, y));
			const river = riverTile ? classifyRiverSegment(riverTile) : undefined;
			if (river && o.carveRivers && !isInPlateau(x, y, o)) {
				height = 0;
			}

			const stairs = deriveStairs(x, y, seed, o);
			const biome = classifyBiome(height, o.maxHeight, moisture);

			const decoration =
				o.decorate && terrain.kind === "flat" && !river && !stairs
					? deriveDecoration(x, y, seed, biome)
					: undefined;

			const tile: TerrainTile = {
				x,
				y,
				elevation,
				moisture,
				height,
				biome,
				terrain,
			};
			if (river) tile.river = river;
			if (stairs) tile.stairs = stairs;
			if (decoration) tile.decoration = decoration;
			tiles.push(tile);
		}
	}
	return tiles;
}
