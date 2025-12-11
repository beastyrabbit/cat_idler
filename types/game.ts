/**
 * Game Type Definitions
 * 
 * This file contains all TypeScript types for the Cat Colony Idle Game.
 * See docs/plan.md for detailed explanations of each type.
 */

// =============================================================================
// Core ID Types (Convex will generate these, but we define for reference)
// =============================================================================

export type Id<T extends string> = string & { __tableName: T }

// =============================================================================
// Enums and Literal Types
// =============================================================================

export type ColonyStatus = 'starting' | 'thriving' | 'struggling' | 'dead'

export type LifeStage = 'kitten' | 'young' | 'adult' | 'elder'

export type MapType = 'colony' | 'world'

export type BuildingType =
  | 'den'
  | 'food_storage'
  | 'water_bowl'
  | 'beds'
  | 'herb_garden'
  | 'nursery'
  | 'elder_corner'
  | 'walls'
  | 'mouse_farm'

export type TileType =
  | 'field'
  | 'forest'
  | 'dense_woods'
  | 'river'
  | 'enemy_territory'

export type TaskType =
  | 'hunt'
  | 'gather_herbs'
  | 'fetch_water'
  | 'clean'
  | 'build'
  | 'guard'
  | 'heal'
  | 'kitsit'
  | 'explore'
  | 'patrol'
  | 'teach'
  | 'rest'

export type EncounterType = 'predator' | 'rival' | 'injury' | 'discovery'

export type EnemyType = 'fox' | 'hawk' | 'badger' | 'bear' | 'rival_cat'

export type EncounterOutcome = 'pending' | 'user_win' | 'cat_win' | 'cat_lose'

export type EventType =
  | 'birth'
  | 'death'
  | 'intruder_attack'
  | 'intruder_defeated'
  | 'breeding'
  | 'leader_change'
  | 'task_complete'
  | 'building_complete'
  | 'user_fed'
  | 'user_healed'
  | 'cat_joined'
  | 'cat_left'
  | 'discovery'

// =============================================================================
// Value Objects (no ID, embedded in other objects)
// =============================================================================

export interface Resources {
  food: number
  water: number
  herbs: number
  materials: number
  blessings: number
}

export interface CatStats {
  attack: number
  defense: number
  hunting: number
  medicine: number
  cleaning: number
  building: number
  leadership: number
  vision: number
}

export interface CatNeeds {
  hunger: number
  thirst: number
  rest: number
  health: number
}

export interface Position {
  map: MapType
  x: number
  y: number
}

export interface TileResources {
  food: number
  herbs: number
  water: number
}

// =============================================================================
// Entity Interfaces (stored in database with IDs)
// =============================================================================

export interface Colony {
  _id: Id<'colonies'>
  name: string
  leaderId: Id<'cats'> | null
  status: ColonyStatus
  resources: Resources
  gridSize: number
  createdAt: number
  lastTick: number
  lastAttack: number
}

export interface Cat {
  _id: Id<'cats'>
  colonyId: Id<'colonies'>
  name: string
  parentIds: [Id<'cats'> | null, Id<'cats'> | null]
  birthTime: number
  deathTime: number | null
  stats: CatStats
  needs: CatNeeds
  currentTask: TaskType | null
  position: Position
  isPregnant: boolean
  pregnancyDueTime: number | null
  spriteParams: Record<string, unknown> | null
}

export interface Building {
  _id: Id<'buildings'>
  colonyId: Id<'colonies'>
  type: BuildingType
  level: number
  position: { x: number; y: number }
  constructionProgress: number
}

export interface WorldTile {
  _id: Id<'worldTiles'>
  colonyId: Id<'colonies'>
  x: number
  y: number
  type: TileType
  resources: TileResources
  maxResources: { food: number; herbs: number }
  dangerLevel: number
  pathWear: number
  lastDepleted: number
}

export interface Task {
  _id: Id<'tasks'>
  colonyId: Id<'colonies'>
  type: TaskType
  priority: number
  assignedCatId: Id<'cats'> | null
  assignmentCountdown: number
  isOptimalAssignment: boolean
  progress: number
  createdAt: number
}

export interface Encounter {
  _id: Id<'encounters'>
  colonyId: Id<'colonies'>
  catId: Id<'cats'>
  type: EncounterType
  enemyType: EnemyType | null
  position: { x: number; y: number }
  clicksNeeded: number
  clicksReceived: number
  createdAt: number
  expiresAt: number
  resolved: boolean
  outcome: EncounterOutcome | null
}

export interface EventLog {
  _id: Id<'events'>
  colonyId: Id<'colonies'>
  timestamp: number
  type: EventType
  message: string
  involvedCatIds: Id<'cats'>[]
  metadata: Record<string, unknown>
}

// =============================================================================
// Action Types (for cat AI)
// =============================================================================

export type AutonomousAction =
  | { type: 'eat' }
  | { type: 'drink' }
  | { type: 'sleep'; position: Position }
  | { type: 'return_to_colony' }
  | { type: 'flee'; from: Position }

// =============================================================================
// Combat Types
// =============================================================================

export interface CombatResult {
  won: boolean
  damage: number
}

// =============================================================================
// UI/Component Types
// =============================================================================

export interface CatSpriteProps {
  cat: Cat
  size?: 'small' | 'medium' | 'large'
  animated?: boolean
  onClick?: () => void
}

export interface ResourceBarProps {
  value: number
  max: number
  color?: 'green' | 'blue' | 'red' | 'yellow'
  showLabel?: boolean
  label?: string
}

export interface TaskCardProps {
  task: Task
  assignedCat?: Cat
  onSpeedUp: () => void
  onReassign: () => void
}

export interface EncounterPopupProps {
  encounter: Encounter
  cat: Cat
  onClickDefend: () => void
}

// =============================================================================
// Constants
// =============================================================================

export const NEEDS_DECAY_RATES = {
  hunger: 5,
  thirst: 3,
  rest: 2,
} as const

export const NEEDS_RESTORE_AMOUNTS = {
  eating: 30,
  drinking: 40,
  sleeping: 20,
  sleepingWithBeds: 30,
} as const

export const LIFE_STAGE_HOURS = {
  kitten: [0, 6],
  young: [6, 24],
  adult: [24, 48],
  elder: [48, Infinity],
} as const

export const LEADER_QUALITY = {
  bad: { min: 0, max: 10, time: 30, wrongChance: 0.4 },
  okay: { min: 11, max: 20, time: 20, wrongChance: 0.2 },
  good: { min: 21, max: 30, time: 10, wrongChance: 0.05 },
  great: { min: 31, max: 100, time: 5, wrongChance: 0 },
} as const

export const ENEMY_STATS: Record<EnemyType, { baseClicks: number; damage: [number, number] }> = {
  fox: { baseClicks: 20, damage: [30, 50] },
  hawk: { baseClicks: 15, damage: [20, 40] },
  badger: { baseClicks: 40, damage: [40, 60] },
  bear: { baseClicks: 75, damage: [50, 70] },
  rival_cat: { baseClicks: 30, damage: [30, 50] },
}

export const BUILDING_COSTS: Record<BuildingType, number> = {
  den: 0,
  food_storage: 5,
  water_bowl: 3,
  beds: 8,
  herb_garden: 10,
  nursery: 12,
  elder_corner: 10,
  walls: 15,
  mouse_farm: 25,
}

export const TASK_TO_SKILL: Record<TaskType, keyof CatStats> = {
  hunt: 'hunting',
  gather_herbs: 'medicine',
  fetch_water: 'hunting',
  clean: 'cleaning',
  build: 'building',
  guard: 'defense',
  heal: 'medicine',
  kitsit: 'leadership',
  explore: 'vision',
  patrol: 'attack',
  teach: 'leadership',
  rest: 'defense', // No skill gain for rest, but needs a mapping
}

