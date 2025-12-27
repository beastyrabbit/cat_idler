/**
 * Cat CRUD Operations
 *
 * Convex mutations and queries for cat management.
 * TASK: CONVEX-002
 */

import { query, mutation, internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { inheritTraits, extractGeneticTraits, traitsToSpriteParams } from "../lib/game/genetics";
import type { CatSpriteParams } from "../lib/cat-renderer/types";

/**
 * Get a cat by ID.
 */
export const getCat = query({
  args: { catId: v.id("cats") },
  handler: async (ctx, args) => {
    return await ctx.db.get(args.catId);
  },
});

/**
 * Get all cats in a colony.
 */
export const getCatsByColony = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("cats")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();
  },
});

/**
 * Get alive cats in a colony.
 */
export const getAliveCats = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("cats")
      .withIndex("by_colony_alive", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("deathTime"), null))
      .collect();
  },
});

/**
 * Get dead cats in a colony.
 */
export const getDeadCats = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("cats")
      .withIndex("by_colony_alive", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.neq(q.field("deathTime"), null))
      .collect();
  },
});

/**
 * Create a new cat.
 */
export const createCat = mutation({
  args: {
    colonyId: v.id("colonies"),
    name: v.string(),
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
    parentIds: v.optional(v.array(v.union(v.id("cats"), v.null()))),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    
    // Generate sprite params using genetics
    const spriteParams = await generateSpriteParams(ctx, args.parentIds);
    
    const catId = await ctx.db.insert("cats", {
      colonyId: args.colonyId,
      name: args.name,
      parentIds: args.parentIds ?? [null, null],
      birthTime: now,
      deathTime: null,
      stats: args.stats,
      needs: {
        hunger: 100,
        thirst: 100,
        rest: 100,
        health: 100,
      },
      currentTask: null,
      position: {
        map: "colony",
        x: 1,
        y: 1,
      },
      isPregnant: false,
      pregnancyDueTime: null,
      spriteParams: spriteParams as Record<string, unknown> | null,
    });
    return catId;
  },
});

/**
 * Generate sprite params for a new cat using genetics
 */
async function generateSpriteParams(
  ctx: any,
  parentIds: (string | null)[] | undefined
): Promise<CatSpriteParams | null> {
  // If no parents, generate random traits (founder cat)
  if (!parentIds || parentIds.every((id) => id === null)) {
    const traits = inheritTraits(null, null);
    return traitsToSpriteParams(traits);
  }

  // Get parent traits
  const parent1Id = parentIds[0];
  const parent2Id = parentIds[1];

  const parent1 = parent1Id ? await ctx.db.get(parent1Id) : null;
  const parent2 = parent2Id ? await ctx.db.get(parent2Id) : null;

  const parent1Traits = extractGeneticTraits(
    parent1?.spriteParams as CatSpriteParams | null
  );
  const parent2Traits = extractGeneticTraits(
    parent2?.spriteParams as CatSpriteParams | null
  );

  // Inherit traits from parents
  const inheritedTraits = inheritTraits(parent1Traits, parent2Traits);
  return traitsToSpriteParams(inheritedTraits);
}

/**
 * Create a new cat (internal - for breeding).
 */
export const createCatInternal = internalMutation({
  args: {
    colonyId: v.id("colonies"),
    name: v.string(),
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
    parentIds: v.optional(v.array(v.union(v.id("cats"), v.null()))),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    
    // Generate sprite params using genetics
    const spriteParams = await generateSpriteParams(ctx, args.parentIds);

    const catId = await ctx.db.insert("cats", {
      colonyId: args.colonyId,
      name: args.name,
      parentIds: args.parentIds ?? [null, null],
      birthTime: now,
      deathTime: null,
      stats: args.stats,
      needs: {
        hunger: 100,
        thirst: 100,
        rest: 100,
        health: 100,
      },
      currentTask: null,
      position: {
        map: "colony",
        x: 1,
        y: 1,
      },
      isPregnant: false,
      pregnancyDueTime: null,
      spriteParams: spriteParams as Record<string, unknown> | null,
    });
    return catId;
  },
});

/**
 * Update cat needs.
 */
export const updateCatNeeds = mutation({
  args: {
    catId: v.id("cats"),
    needs: v.object({
      hunger: v.number(),
      thirst: v.number(),
      rest: v.number(),
      health: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.catId, {
      needs: args.needs,
    });
  },
});

/**
 * Update cat position.
 */
export const updateCatPosition = mutation({
  args: {
    catId: v.id("cats"),
    position: v.object({
      map: v.union(v.literal("colony"), v.literal("world")),
      x: v.number(),
      y: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.catId, {
      position: args.position,
    });
  },
});

/**
 * Assign a task to a cat.
 */
export const assignTask = mutation({
  args: {
    catId: v.id("cats"),
    taskType: v.union(
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
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.catId, {
      currentTask: args.taskType,
    });
  },
});

/**
 * Kill a cat (set death time).
 */
export const killCat = mutation({
  args: {
    catId: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    await ctx.db.patch(args.catId, {
      deathTime: now,
      currentTask: null,
    });
  },
});

/**
 * Kill a cat (internal).
 */
export const killCatInternal = internalMutation({
  args: {
    catId: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    await ctx.db.patch(args.catId, {
      deathTime: now,
      currentTask: null,
    });
  },
});

/**
 * Update cat stats (for skill learning).
 */
export const updateCatStats = mutation({
  args: {
    catId: v.id("cats"),
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
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.catId, {
      stats: args.stats,
    });
  },
});

/**
 * Update cat stats (internal).
 */
export const updateCatStatsInternal = internalMutation({
  args: {
    catId: v.id("cats"),
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
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.catId, {
      stats: args.stats,
    });
  },
});



