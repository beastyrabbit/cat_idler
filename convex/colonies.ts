/**
 * Colony CRUD Operations
 *
 * Convex mutations and queries for colony management.
 * TASK: CONVEX-001
 */

import { query, mutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";
import { inheritTraits, traitsToSpriteParams } from "../lib/game/genetics";

/**
 * Get a colony by ID.
 */
export const getColony = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db.get(args.colonyId);
  },
});

/**
 * Get the active (non-dead) colony, or the most recent one.
 */
export const getActiveColony = query({
  args: {},
  handler: async (ctx) => {
    // Get all colonies, find first non-dead
    const allColonies = await ctx.db
      .query("colonies")
      .withIndex("by_created", (q) => q)
      .order("desc")
      .collect();
    
    // Find first non-dead colony
    for (const colony of allColonies) {
      if (colony.status !== "dead") {
        return colony;
      }
    }
    
    // If all are dead, return the most recent
    return allColonies[0] ?? null;
  },
});

/**
 * Get all colonies.
 */
export const getAllColonies = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db
      .query("colonies")
      .withIndex("by_created", (q) => q)
      .order("desc")
      .collect();
  },
});

/**
 * Create a new colony.
 */
export const createColony = mutation({
  args: {
    name: v.string(),
    leaderId: v.union(v.id("cats"), v.null()),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const worldSeed = now; // Use creation time as seed for deterministic generation
    const colonyId = await ctx.db.insert("colonies", {
      name: args.name,
      leaderId: args.leaderId,
      status: "starting",
      resources: {
        food: 10,
        water: 10,
        herbs: 5,
        materials: 0,
        blessings: 0,
      },
      gridSize: 3,
      createdAt: now,
      lastTick: now,
      lastAttack: now,
      worldSeed,
    });

    // Initialize colony immediately
    // Create 5 starting cats with random sprite params (founder cats)
    const catNames = ["Whiskers", "Shadow", "Luna", "Max", "Bella"];
    for (let i = 0; i < 5; i++) {
      const stats = {
        attack: 30 + Math.floor(Math.random() * 40),
        defense: 30 + Math.floor(Math.random() * 40),
        hunting: 30 + Math.floor(Math.random() * 40),
        medicine: 20 + Math.floor(Math.random() * 30),
        cleaning: 30 + Math.floor(Math.random() * 40),
        building: 20 + Math.floor(Math.random() * 30),
        leadership: 20 + Math.floor(Math.random() * 40),
        vision: 30 + Math.floor(Math.random() * 40),
      };

      // Generate random sprite params for founder cats
      const traits = inheritTraits(null, null);
      const spriteParams = traitsToSpriteParams(traits);

      await ctx.db.insert("cats", {
        colonyId,
        name: catNames[i] || `Cat ${i + 1}`,
        parentIds: [null, null],
        birthTime: Date.now(),
        deathTime: null,
        stats,
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
    }

    // Initialize world map (16x16 grid)
    await ctx.runMutation(internal.worldMap.initializeWorldMapInternal, {
      colonyId,
    });

    // Create starting den at center
    const gridSize = 3;
    const center = Math.floor(gridSize / 2);
    await ctx.db.insert("buildings", {
      colonyId,
      type: "den",
      level: 0,
      position: { x: center, y: center },
      constructionProgress: 100, // Complete immediately
    });

    return colonyId;
  },
});

/**
 * Update colony resources.
 */
export const updateColonyResources = mutation({
  args: {
    colonyId: v.id("colonies"),
    resources: v.object({
      food: v.number(),
      water: v.number(),
      herbs: v.number(),
      materials: v.number(),
      blessings: v.number(),
    }),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.colonyId, {
      resources: args.resources,
    });
  },
});

/**
 * Set colony leader.
 */
export const setColonyLeader = mutation({
  args: {
    colonyId: v.id("colonies"),
    catId: v.union(v.id("cats"), v.null()),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.colonyId, {
      leaderId: args.catId,
    });
  },
});

/**
 * Update colony status.
 */
export const updateColonyStatus = mutation({
  args: {
    colonyId: v.id("colonies"),
    status: v.union(
      v.literal("starting"),
      v.literal("thriving"),
      v.literal("struggling"),
      v.literal("dead")
    ),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.colonyId, {
      status: args.status,
    });
  },
});

/**
 * Update last tick timestamp.
 */
export const updateLastTick = mutation({
  args: {
    colonyId: v.id("colonies"),
    lastTick: v.number(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.colonyId, {
      lastTick: args.lastTick,
    });
  },
});



