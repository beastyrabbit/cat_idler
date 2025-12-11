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

// TODO: Export other modules as they are implemented
// export * from './skills'
// export * from './combat'
// export * from './catAI'
// export * from './tasks'
// export * from './paths'
// export * from './worldResources'

