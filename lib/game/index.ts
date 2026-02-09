/**
 * Game Logic Module
 *
 * This module exports all pure game logic functions.
 * These functions have NO side effects and NO database access.
 * They take inputs and return outputs - perfect for unit testing.
 */

// Needs system (hunger, thirst, rest, health)
export * from './needs'

// Age and life stage calculations
export * from './age'

// Skill learning system
export * from './skills'

// Combat resolution
export * from './combat'

// Cat AI autonomous behavior
export * from './catAI'

// Task assignment
export * from './tasks'

// Path system
export * from './paths'

// World resource management
export * from './worldResources'

// World generation
export * from './worldGen'
export * from './biomes'
export * from './noise'
export * from './seededRng'
export * from './policy'
export * from './survival'
export * from './housePlanner'

