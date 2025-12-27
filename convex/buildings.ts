/**
 * Building Operations
 *
 * Convex mutations and queries for building management.
 * TASK: CONVEX-003
 */

import { query, mutation, internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";

/**
 * Get all buildings in a colony.
 */
export const getBuildingsByColony = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("buildings")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();
  },
});

/**
 * Get building at a specific position.
 */
export const getBuildingAt = query({
  args: {
    colonyId: v.id("colonies"),
    x: v.number(),
    y: v.number(),
  },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("buildings")
      .withIndex("by_colony_position", (q) =>
        q.eq("colonyId", args.colonyId).eq("position.x", args.x).eq("position.y", args.y)
      )
      .first();
  },
});

/**
 * Place a new building (starts construction).
 */
export const placeBuilding = mutation({
  args: {
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
    position: v.object({
      x: v.number(),
      y: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    const buildingId = await ctx.db.insert("buildings", {
      colonyId: args.colonyId,
      type: args.type,
      level: 0,
      position: args.position,
      constructionProgress: 0,
    });
    return buildingId;
  },
});

/**
 * Place a new building (internal).
 */
export const placeBuildingInternal = internalMutation({
  args: {
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
    position: v.object({
      x: v.number(),
      y: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    const buildingId = await ctx.db.insert("buildings", {
      colonyId: args.colonyId,
      type: args.type,
      level: 0,
      position: args.position,
      constructionProgress: 0,
    });
    return buildingId;
  },
});

/**
 * Progress construction on a building.
 */
export const progressConstruction = mutation({
  args: {
    buildingId: v.id("buildings"),
    amount: v.number(),
  },
  handler: async (ctx, args) => {
    const building = await ctx.db.get(args.buildingId);
    if (!building) {
      throw new Error("Building not found");
    }

    const newProgress = Math.min(100, building.constructionProgress + args.amount);
    await ctx.db.patch(args.buildingId, {
      constructionProgress: newProgress,
    });
  },
});

/**
 * Upgrade a building (increase level).
 */
export const upgradeBuilding = mutation({
  args: {
    buildingId: v.id("buildings"),
  },
  handler: async (ctx, args) => {
    const building = await ctx.db.get(args.buildingId);
    if (!building) {
      throw new Error("Building not found");
    }

    const newLevel = Math.min(3, building.level + 1);
    await ctx.db.patch(args.buildingId, {
      level: newLevel,
    });
  },
});

/**
 * Complete construction (set progress to 100).
 */
export const completeConstruction = mutation({
  args: {
    buildingId: v.id("buildings"),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.buildingId, {
      constructionProgress: 100,
    });
  },
});

/**
 * Process building passive effects (called by game tick).
 */
export const processBuildingEffects = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    const buildings = await ctx.db
      .query("buildings")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("constructionProgress"), 100))
      .collect();

    const colony = await ctx.db.get(args.colonyId);
    if (!colony) return;

    let foodBonus = 0;
    let herbsBonus = 0;

    for (const building of buildings) {
      switch (building.type) {
        case "mouse_farm":
          // +2 food per hour (per tick = +0.033 food)
          foodBonus += 0.033 * (building.level + 1);
          break;
        case "herb_garden":
          // +1 herb per hour (per tick = +0.017 herbs)
          herbsBonus += 0.017 * (building.level + 1);
          break;
      }
    }

    if (foodBonus > 0 || herbsBonus > 0) {
      await ctx.db.patch(args.colonyId, {
        resources: {
          ...colony.resources,
          food: Math.min(1000, colony.resources.food + Math.floor(foodBonus)),
          herbs: Math.min(500, colony.resources.herbs + Math.floor(herbsBonus)),
        },
      });
    }
  },
});



