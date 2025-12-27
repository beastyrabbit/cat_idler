/**
 * Convex Database Schema
 *
 * Defines all database tables for the Cat Colony Idle Game.
 * See docs/plan.md for detailed field descriptions.
 */

import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  // ============================================================================
  // Colonies
  // ============================================================================
  colonies: defineTable({
    name: v.string(),
    leaderId: v.union(v.id("cats"), v.null()),
    status: v.union(
      v.literal("starting"),
      v.literal("thriving"),
      v.literal("struggling"),
      v.literal("dead")
    ),
    resources: v.object({
      food: v.number(),
      water: v.number(),
      herbs: v.number(),
      materials: v.number(),
      blessings: v.number(),
    }),
    gridSize: v.number(), // 3, 5, or 8
    createdAt: v.number(),
    lastTick: v.number(),
    lastAttack: v.number(),
    worldSeed: v.optional(v.number()), // Seed for procedural world generation
  })
    .index("by_status", ["status"])
    .index("by_created", ["createdAt"]),

  // ============================================================================
  // Cats
  // ============================================================================
  cats: defineTable({
    colonyId: v.id("colonies"),
    name: v.string(),
    parentIds: v.array(v.union(v.id("cats"), v.null())), // [mom, dad]
    birthTime: v.number(),
    deathTime: v.union(v.number(), v.null()),
    stats: v.object({
      attack: v.number(),
      defense: v.number(),
      hunting: v.number(),
      medicine: v.number(),
      cleaning: v.number(),
      building: v.number(),
      leadership: v.number(),
      vision: v.number(),
    }),
    needs: v.object({
      hunger: v.number(),
      thirst: v.number(),
      rest: v.number(),
      health: v.number(),
    }),
    currentTask: v.union(
      v.literal("hunt"),
      v.literal("gather_herbs"),
      v.literal("fetch_water"),
      v.literal("clean"),
      v.literal("build"),
      v.literal("guard"),
      v.literal("heal"),
      v.literal("kitsit"),
      v.literal("explore"),
      v.literal("patrol"),
      v.literal("teach"),
      v.literal("rest"),
      v.null()
    ),
    position: v.object({
      map: v.union(v.literal("colony"), v.literal("world")),
      x: v.number(),
      y: v.number(),
    }),
    isPregnant: v.boolean(),
    pregnancyDueTime: v.union(v.number(), v.null()),
    spriteParams: v.union(v.any(), v.null()),
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_alive", ["colonyId", "deathTime"]),

  // ============================================================================
  // Buildings
  // ============================================================================
  buildings: defineTable({
    colonyId: v.id("colonies"),
    type: v.union(
      v.literal("den"),
      v.literal("food_storage"),
      v.literal("water_bowl"),
      v.literal("beds"),
      v.literal("herb_garden"),
      v.literal("nursery"),
      v.literal("elder_corner"),
      v.literal("walls"),
      v.literal("mouse_farm")
    ),
    level: v.number(), // 0-3
    position: v.object({
      x: v.number(),
      y: v.number(),
    }),
    constructionProgress: v.number(), // 0-100
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_position", ["colonyId", "position.x", "position.y"]),

  // ============================================================================
  // World Tiles
  // ============================================================================
  worldTiles: defineTable({
    colonyId: v.id("colonies"),
    x: v.number(),
    y: v.number(),
    type: v.union(
      v.literal("field"),
      v.literal("forest"),
      v.literal("dense_woods"),
      v.literal("river"),
      v.literal("enemy_territory"),
      v.literal("oak_forest"),
      v.literal("pine_forest"),
      v.literal("jungle"),
      v.literal("dead_forest"),
      v.literal("mountains"),
      v.literal("swamp"),
      v.literal("desert"),
      v.literal("tundra"),
      v.literal("meadow"),
      v.literal("cave_entrance"),
      v.literal("enemy_lair")
    ),
    resources: v.object({
      food: v.number(),
      herbs: v.number(),
      water: v.number(),
    }),
    maxResources: v.object({
      food: v.number(),
      herbs: v.number(),
    }),
    dangerLevel: v.number(), // 0-100
    pathWear: v.number(), // 0-100
    lastDepleted: v.number(),
    overlayFeature: v.optional(
      v.union(
        v.literal("river"),
        v.literal("ancient_road"),
        v.literal("game_trail"),
        v.literal("trade_route"),
        v.null()
      )
    ),
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_position", ["colonyId", "x", "y"]),

  // ============================================================================
  // Tasks
  // ============================================================================
  tasks: defineTable({
    colonyId: v.id("colonies"),
    type: v.union(
      v.literal("hunt"),
      v.literal("gather_herbs"),
      v.literal("fetch_water"),
      v.literal("clean"),
      v.literal("build"),
      v.literal("guard"),
      v.literal("heal"),
      v.literal("kitsit"),
      v.literal("explore"),
      v.literal("patrol"),
      v.literal("teach"),
      v.literal("rest")
    ),
    priority: v.number(),
    assignedCatId: v.union(v.id("cats"), v.null()),
    assignmentCountdown: v.number(), // Seconds until auto-assign
    isOptimalAssignment: v.boolean(),
    progress: v.number(), // 0-100
    createdAt: v.number(),
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_assigned", ["colonyId", "assignedCatId"]),

  // ============================================================================
  // Encounters
  // ============================================================================
  encounters: defineTable({
    colonyId: v.id("colonies"),
    catId: v.id("cats"),
    type: v.union(
      v.literal("predator"),
      v.literal("rival"),
      v.literal("injury"),
      v.literal("discovery")
    ),
    enemyType: v.union(
      v.literal("fox"),
      v.literal("hawk"),
      v.literal("badger"),
      v.literal("bear"),
      v.literal("rival_cat"),
      v.null()
    ),
    position: v.object({
      x: v.number(),
      y: v.number(),
    }),
    clicksNeeded: v.number(),
    clicksReceived: v.number(),
    createdAt: v.number(),
    expiresAt: v.number(),
    resolved: v.boolean(),
    outcome: v.union(
      v.literal("pending"),
      v.literal("user_win"),
      v.literal("cat_win"),
      v.literal("cat_lose"),
      v.null()
    ),
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_active", ["colonyId", "resolved"])
    .index("by_cat", ["catId"]),

  // ============================================================================
  // Event Log
  // ============================================================================
  events: defineTable({
    colonyId: v.id("colonies"),
    // Primary cat for indexing (in addition to involvedCatIds).
    // This enables efficient "events for a cat" queries without indexing arrays.
    catId: v.optional(v.id("cats")),
    timestamp: v.number(),
    type: v.union(
      v.literal("birth"),
      v.literal("death"),
      v.literal("intruder_attack"),
      v.literal("intruder_defeated"),
      v.literal("breeding"),
      v.literal("leader_change"),
      v.literal("task_complete"),
      v.literal("building_complete"),
      v.literal("user_fed"),
      v.literal("user_healed"),
      v.literal("cat_joined"),
      v.literal("cat_left"),
      v.literal("discovery")
    ),
    message: v.string(),
    involvedCatIds: v.array(v.id("cats")),
    metadata: v.any(), // Flexible object for event-specific data
  })
    .index("by_colony", ["colonyId"])
    .index("by_colony_time", ["colonyId", "timestamp"])
    .index("by_cat", ["catId"]),
});



