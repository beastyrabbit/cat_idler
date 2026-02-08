/**
 * Convex Database Schema
 */

import { defineSchema, defineTable } from 'convex/server';
import { v } from 'convex/values';

export default defineSchema({
  colonies: defineTable({
    name: v.string(),
    leaderId: v.union(v.id('cats'), v.null()),
    status: v.union(
      v.literal('starting'),
      v.literal('thriving'),
      v.literal('struggling'),
      v.literal('dead'),
    ),
    resources: v.object({
      food: v.number(),
      water: v.number(),
      herbs: v.number(),
      materials: v.number(),
      blessings: v.number(),
    }),
    gridSize: v.number(),
    createdAt: v.number(),
    lastTick: v.number(),
    lastAttack: v.number(),
    worldSeed: v.optional(v.number()),

    // Browser-idle v2
    isGlobal: v.optional(v.boolean()),
    runNumber: v.optional(v.number()),
    runStartedAt: v.optional(v.number()),
    lastPlayerActivityAt: v.optional(v.number()),
    lastResetAt: v.optional(v.number()),
    automationTier: v.optional(v.number()),
    globalUpgradePoints: v.optional(v.number()),
    ritualRequestedAt: v.optional(v.union(v.number(), v.null())),
    criticalSince: v.optional(v.union(v.number(), v.null())),
  })
    .index('by_status', ['status'])
    .index('by_created', ['createdAt'])
    .index('by_is_global', ['isGlobal']),

  cats: defineTable({
    colonyId: v.id('colonies'),
    name: v.string(),
    parentIds: v.array(v.union(v.id('cats'), v.null())),
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
      v.literal('hunt'),
      v.literal('gather_herbs'),
      v.literal('fetch_water'),
      v.literal('clean'),
      v.literal('build'),
      v.literal('guard'),
      v.literal('heal'),
      v.literal('kitsit'),
      v.literal('explore'),
      v.literal('patrol'),
      v.literal('teach'),
      v.literal('rest'),
      v.null(),
    ),
    position: v.object({
      map: v.union(v.literal('colony'), v.literal('world')),
      x: v.number(),
      y: v.number(),
    }),
    isPregnant: v.boolean(),
    pregnancyDueTime: v.union(v.number(), v.null()),
    spriteParams: v.union(v.any(), v.null()),

    // Browser-idle v2 specialization
    specialization: v.optional(
      v.union(v.literal('hunter'), v.literal('architect'), v.literal('ritualist'), v.null()),
    ),
    roleXp: v.optional(
      v.object({
        hunter: v.number(),
        architect: v.number(),
        ritualist: v.number(),
      }),
    ),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_alive', ['colonyId', 'deathTime']),

  buildings: defineTable({
    colonyId: v.id('colonies'),
    type: v.union(
      v.literal('den'),
      v.literal('food_storage'),
      v.literal('water_bowl'),
      v.literal('beds'),
      v.literal('herb_garden'),
      v.literal('nursery'),
      v.literal('elder_corner'),
      v.literal('walls'),
      v.literal('mouse_farm'),
    ),
    level: v.number(),
    position: v.object({ x: v.number(), y: v.number() }),
    constructionProgress: v.number(),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_position', ['colonyId', 'position.x', 'position.y']),

  worldTiles: defineTable({
    colonyId: v.id('colonies'),
    x: v.number(),
    y: v.number(),
    type: v.union(
      v.literal('field'),
      v.literal('forest'),
      v.literal('dense_woods'),
      v.literal('river'),
      v.literal('enemy_territory'),
      v.literal('oak_forest'),
      v.literal('pine_forest'),
      v.literal('jungle'),
      v.literal('dead_forest'),
      v.literal('mountains'),
      v.literal('swamp'),
      v.literal('desert'),
      v.literal('tundra'),
      v.literal('meadow'),
      v.literal('cave_entrance'),
      v.literal('enemy_lair'),
    ),
    resources: v.object({ food: v.number(), herbs: v.number(), water: v.number() }),
    maxResources: v.object({ food: v.number(), herbs: v.number() }),
    dangerLevel: v.number(),
    pathWear: v.number(),
    lastDepleted: v.number(),
    overlayFeature: v.optional(
      v.union(
        v.literal('river'),
        v.literal('ancient_road'),
        v.literal('game_trail'),
        v.literal('trade_route'),
        v.null(),
      ),
    ),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_position', ['colonyId', 'x', 'y']),

  tasks: defineTable({
    colonyId: v.id('colonies'),
    type: v.union(
      v.literal('hunt'),
      v.literal('gather_herbs'),
      v.literal('fetch_water'),
      v.literal('clean'),
      v.literal('build'),
      v.literal('guard'),
      v.literal('heal'),
      v.literal('kitsit'),
      v.literal('explore'),
      v.literal('patrol'),
      v.literal('teach'),
      v.literal('rest'),
    ),
    priority: v.number(),
    assignedCatId: v.union(v.id('cats'), v.null()),
    assignmentCountdown: v.number(),
    isOptimalAssignment: v.boolean(),
    progress: v.number(),
    createdAt: v.number(),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_assigned', ['colonyId', 'assignedCatId']),

  encounters: defineTable({
    colonyId: v.id('colonies'),
    catId: v.id('cats'),
    type: v.union(v.literal('predator'), v.literal('rival'), v.literal('injury'), v.literal('discovery')),
    enemyType: v.union(
      v.literal('fox'),
      v.literal('hawk'),
      v.literal('badger'),
      v.literal('bear'),
      v.literal('rival_cat'),
      v.null(),
    ),
    position: v.object({ x: v.number(), y: v.number() }),
    clicksNeeded: v.number(),
    clicksReceived: v.number(),
    createdAt: v.number(),
    expiresAt: v.number(),
    resolved: v.boolean(),
    outcome: v.union(v.literal('pending'), v.literal('user_win'), v.literal('cat_win'), v.literal('cat_lose'), v.null()),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_active', ['colonyId', 'resolved'])
    .index('by_cat', ['catId']),

  events: defineTable({
    colonyId: v.id('colonies'),
    catId: v.optional(v.id('cats')),
    timestamp: v.number(),
    type: v.union(
      v.literal('birth'),
      v.literal('death'),
      v.literal('intruder_attack'),
      v.literal('intruder_defeated'),
      v.literal('breeding'),
      v.literal('leader_change'),
      v.literal('task_complete'),
      v.literal('building_complete'),
      v.literal('user_fed'),
      v.literal('user_healed'),
      v.literal('cat_joined'),
      v.literal('cat_left'),
      v.literal('discovery'),
      v.literal('job_queued'),
      v.literal('job_completed'),
      v.literal('ritual_ready'),
      v.literal('upgrade_purchased'),
      v.literal('run_reset'),
    ),
    message: v.string(),
    involvedCatIds: v.array(v.id('cats')),
    metadata: v.any(),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_time', ['colonyId', 'timestamp'])
    .index('by_cat', ['catId']),

  players: defineTable({
    sessionId: v.string(),
    nickname: v.string(),
    lastSeenAt: v.number(),
    clickWindowStart: v.number(),
    clicksInWindow: v.number(),
    lifetimeClicks: v.number(),
    lifetimeContribution: v.object({
      food: v.number(),
      water: v.number(),
      jobsRequested: v.number(),
      upgradesPurchased: v.number(),
    }),
  })
    .index('by_session', ['sessionId'])
    .index('by_last_seen', ['lastSeenAt']),

  jobs: defineTable({
    colonyId: v.id('colonies'),
    kind: v.union(
      v.literal('supply_food'),
      v.literal('supply_water'),
      v.literal('leader_plan_hunt'),
      v.literal('hunt_expedition'),
      v.literal('leader_plan_house'),
      v.literal('build_house'),
      v.literal('ritual'),
    ),
    status: v.union(
      v.literal('queued'),
      v.literal('active'),
      v.literal('completed'),
      v.literal('failed'),
      v.literal('cancelled'),
    ),
    requestedByType: v.union(v.literal('player'), v.literal('leader'), v.literal('system')),
    requestedByPlayerId: v.optional(v.id('players')),
    assignedCatId: v.union(v.id('cats'), v.null()),
    baseDurationSec: v.number(),
    speedMultiplier: v.number(),
    yieldMultiplier: v.number(),
    clickTimeReducedSec: v.number(),
    createdAt: v.number(),
    startedAt: v.number(),
    endsAt: v.number(),
    completedAt: v.optional(v.number()),
    metadata: v.optional(v.any()),
  })
    .index('by_colony_status', ['colonyId', 'status'])
    .index('by_colony_end', ['colonyId', 'endsAt'])
    .index('by_player', ['requestedByPlayerId']),

  globalUpgrades: defineTable({
    colonyId: v.id('colonies'),
    key: v.union(
      v.literal('click_power'),
      v.literal('supply_speed'),
      v.literal('hunt_mastery'),
      v.literal('build_mastery'),
      v.literal('ritual_mastery'),
      v.literal('resilience'),
    ),
    level: v.number(),
    maxLevel: v.number(),
    baseCost: v.number(),
    description: v.string(),
  })
    .index('by_colony', ['colonyId'])
    .index('by_colony_key', ['colonyId', 'key']),

  runHistory: defineTable({
    colonyId: v.id('colonies'),
    runNumber: v.number(),
    startedAt: v.number(),
    endedAt: v.number(),
    durationSec: v.number(),
    reason: v.string(),
    finalResources: v.object({
      food: v.number(),
      water: v.number(),
      herbs: v.number(),
      materials: v.number(),
      blessings: v.number(),
    }),
    activePlayers: v.number(),
  }).index('by_colony_run', ['colonyId', 'runNumber']),
});
