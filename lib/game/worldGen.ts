/**
 * World Generation System
 *
 * Procedural generation of infinite world using Voronoi-based biomes,
 * rivers, and paths. Generates 12x12 chunks deterministically.
 */

import { hashSeed, createSeededRandom, noise2D, fractalNoise2D } from './noise'
import {
  BiomeType,
  OverlayFeature,
  BIOME_PROPERTIES,
  OVERLAY_FEATURE_PROPERTIES,
  calculateDangerLevel,
} from './biomes'
import type { WorldTile } from '@/types/game'

const CHUNK_SIZE = 12

export interface ChunkCoords {
  chunkX: number
  chunkY: number
}

export interface TileCoords {
  x: number
  y: number
}

/**
 * Convert world tile coordinates to chunk coordinates
 */
export function tileToChunk(x: number, y: number): ChunkCoords {
  return {
    chunkX: Math.floor(x / CHUNK_SIZE),
    chunkY: Math.floor(y / CHUNK_SIZE),
  }
}

/**
 * Convert chunk coordinates to tile coordinates (top-left corner)
 */
export function chunkToTile(chunkX: number, chunkY: number): TileCoords {
  return {
    x: chunkX * CHUNK_SIZE,
    y: chunkY * CHUNK_SIZE,
  }
}

/**
 * Voronoi cell for biome placement
 */
interface VoronoiCell {
  x: number
  y: number
  biome: BiomeType
}

/**
 * Generate Voronoi cells for biome placement
 * Uses seeded random to place biome centers
 */
function generateVoronoiCells(
  seed: number,
  minX: number,
  minY: number,
  maxX: number,
  maxY: number,
  density: number = 0.1
): VoronoiCell[] {
  const cells: VoronoiCell[] = []
  const rng = createSeededRandom(hashSeed(seed, 'voronoi'))
  
  const width = maxX - minX
  const height = maxY - minY
  const numCells = Math.floor(width * height * density)
  
  const biomeTypes: BiomeType[] = [
    'oak_forest',
    'pine_forest',
    'jungle',
    'dead_forest',
    'mountains',
    'swamp',
    'desert',
    'tundra',
    'meadow',
  ]
  
  for (let i = 0; i < numCells; i++) {
    const x = minX + rng.next() * width
    const y = minY + rng.next() * height
    const biomeIndex = rng.int(0, biomeTypes.length - 1)
    
    cells.push({
      x,
      y,
      biome: biomeTypes[biomeIndex],
    })
  }
  
  return cells
}

/**
 * Find the nearest Voronoi cell to a point
 */
function findNearestCell(x: number, y: number, cells: VoronoiCell[]): VoronoiCell {
  let nearest = cells[0]
  let minDist = Infinity
  
  for (const cell of cells) {
    const dx = x - cell.x
    const dy = y - cell.y
    const dist = dx * dx + dy * dy
    
    if (dist < minDist) {
      minDist = dist
      nearest = cell
    }
  }
  
  return nearest
}

/**
 * Check if a tile is on a biome boundary (for river generation)
 */
function isBiomeBoundary(
  x: number,
  y: number,
  cells: VoronoiCell[],
  threshold: number = 0.3
): boolean {
  const nearest = findNearestCell(x, y, cells)
  const secondNearest = cells
    .filter(c => c !== nearest)
    .reduce((closest, cell) => {
      const dx1 = x - nearest.x
      const dy1 = y - nearest.y
      const dist1 = dx1 * dx1 + dy1 * dy1
      
      const dx2 = x - cell.x
      const dy2 = y - cell.y
      const dist2 = dx2 * dx2 + dy2 * dy2
      
      return dist2 < dist1 ? cell : closest
    }, cells[0])
  
  const dist1 = Math.sqrt((x - nearest.x) ** 2 + (y - nearest.y) ** 2)
  const dist2 = Math.sqrt((x - secondNearest.x) ** 2 + (y - secondNearest.y) ** 2)
  
  return Math.abs(dist1 - dist2) < threshold
}

/**
 * Check if a tile should have a river
 */
function shouldHaveRiver(
  x: number,
  y: number,
  seed: number,
  cells: VoronoiCell[]
): boolean {
  // Rivers flow along biome boundaries
  if (isBiomeBoundary(x, y, cells)) {
    const noise = noise2D(x, y, hashSeed(seed, 'rivers'), 0.05)
    return noise > 0.4 // 60% chance on boundaries
  }
  
  // Rivers can also cut through biomes (less common)
  const noise = noise2D(x, y, hashSeed(seed, 'rivers_through'), 0.02)
  return noise > 0.85 // 15% chance
  
  // Check for orthogonal river connections (no diagonals)
  // This is handled in the river generation logic
}

/**
 * Check if a tile should have a path
 */
function shouldHavePath(
  x: number,
  y: number,
  seed: number,
  pathType: 'ancient_road' | 'game_trail' | 'trade_route'
): boolean {
  const noise = noise2D(x, y, hashSeed(seed, pathType), 0.03)
  
  // Different thresholds for different path types
  const thresholds = {
    ancient_road: 0.92, // Rare
    game_trail: 0.75, // Common in forests
    trade_route: 0.88, // Uncommon
  }
  
  return noise > thresholds[pathType]
}

/**
 * Determine overlay feature for a tile
 */
function getOverlayFeature(
  x: number,
  y: number,
  seed: number,
  cells: VoronoiCell[]
): OverlayFeature {
  const rng = createSeededRandom(hashSeed(seed, x, y, 'overlay'))
  
  // Rivers take priority
  if (shouldHaveRiver(x, y, seed, cells)) {
    // Check if this river connects orthogonally (no diagonals)
    const neighbors = [
      { x: x - 1, y },
      { x: x + 1, y },
      { x, y: y - 1 },
      { x, y: y + 1 },
    ]
    
    let hasOrthogonalRiver = false
    for (const neighbor of neighbors) {
      if (shouldHaveRiver(neighbor.x, neighbor.y, seed, cells)) {
        hasOrthogonalRiver = true
        break
      }
    }
    
    // Only place river if it has at least one orthogonal neighbor or random chance
    if (hasOrthogonalRiver || rng.next() < 0.3) {
      return 'river'
    }
  }
  
  // Check for paths (in order of priority)
  if (shouldHavePath(x, y, seed, 'ancient_road')) {
    return 'ancient_road'
  }
  if (shouldHavePath(x, y, seed, 'trade_route')) {
    return 'trade_route'
  }
  if (shouldHavePath(x, y, seed, 'game_trail')) {
    return 'game_trail'
  }
  
  return null
}

/**
 * Generate a single tile
 */
function generateTile(
  x: number,
  y: number,
  seed: number,
  cells: VoronoiCell[],
  colonyX: number,
  colonyY: number
): Omit<WorldTile, '_id' | 'colonyId'> {
  const nearestCell = findNearestCell(x, y, cells)
  const biome = nearestCell.biome
  const overlayFeature = getOverlayFeature(x, y, seed, cells)
  
  const biomeProps = BIOME_PROPERTIES[biome]
  const rng = createSeededRandom(hashSeed(seed, x, y))
  
  // Calculate distance from colony
  const distanceFromColony = Math.sqrt(
    (x - colonyX) ** 2 + (y - colonyY) ** 2
  )
  
  // Generate resources
  let resources = {
    food: rng.int(biomeProps.baseResources.food.min, biomeProps.baseResources.food.max),
    herbs: rng.int(biomeProps.baseResources.herbs.min, biomeProps.baseResources.herbs.max),
    water: biomeProps.baseResources.water,
  }
  
  // Rivers have infinite water
  if (overlayFeature === 'river') {
    resources.water = 999
    resources.food = 0
    resources.herbs = 0
  }
  
  // Calculate danger level
  const dangerLevel = calculateDangerLevel(biome, overlayFeature, distanceFromColony)
  
  // Get initial path wear from overlay feature
  let pathWear = 0
  if (overlayFeature && overlayFeature !== 'river') {
    pathWear = OVERLAY_FEATURE_PROPERTIES[overlayFeature].initialPathWear
  }
  
  // Determine tile type (for backward compatibility)
  let tileType: 'field' | 'forest' | 'dense_woods' | 'river' | 'enemy_territory'
  if (overlayFeature === 'river') {
    tileType = 'river'
  } else if (biome === 'enemy_lair') {
    tileType = 'enemy_territory'
  } else if (biome === 'jungle' || biome === 'dead_forest') {
    tileType = 'dense_woods'
  } else if (biome.includes('forest')) {
    tileType = 'forest'
  } else {
    tileType = 'field'
  }
  
  return {
    x,
    y,
    type: tileType,
    resources,
    maxResources: {
      food: biomeProps.maxResources.food,
      herbs: biomeProps.maxResources.herbs,
    },
    dangerLevel,
    pathWear,
    lastDepleted: 0,
    overlayFeature,
  }
}

/**
 * Generate a chunk (12x12 tiles)
 */
export function generateChunk(
  chunkX: number,
  chunkY: number,
  seed: number,
  colonyX: number,
  colonyY: number
): Omit<WorldTile, '_id' | 'colonyId'>[] {
  const tiles: Omit<WorldTile, '_id' | 'colonyId'>[] = []
  
  // Calculate chunk bounds
  const minX = chunkX * CHUNK_SIZE
  const minY = chunkY * CHUNK_SIZE
  const maxX = minX + CHUNK_SIZE
  const maxY = minY + CHUNK_SIZE
  
  // Expand bounds for Voronoi generation (to avoid edge artifacts)
  const expand = CHUNK_SIZE * 2
  const voronoiMinX = minX - expand
  const voronoiMinY = minY - expand
  const voronoiMaxX = maxX + expand
  const voronoiMaxY = maxY + expand
  
  // Generate Voronoi cells for this region
  const cells = generateVoronoiCells(
    seed,
    voronoiMinX,
    voronoiMinY,
    voronoiMaxX,
    voronoiMaxY
  )
  
  // Generate tiles
  for (let y = minY; y < maxY; y++) {
    for (let x = minX; x < maxX; x++) {
      tiles.push(generateTile(x, y, seed, cells, colonyX, colonyY))
    }
  }
  
  return tiles
}

/**
 * Get colony position (center of starting chunk)
 */
export function getColonyPosition(): TileCoords {
  // Colony is at center of chunk (0, 0)
  // Center of 12x12 chunk is at (6, 6) in tile coordinates
  return { x: 6, y: 6 }
}

