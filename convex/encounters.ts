/**
 * Encounter Operations
 *
 * Convex mutations and queries for encounter management.
 * TASK: CONVEX-006
 */

import { query, mutation, internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";
import { getClicksNeeded } from "../lib/game/combat";
import { ENEMY_STATS } from "../types/game";

/**
 * Get active encounters for a colony.
 */
export const getActiveEncounters = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("encounters")
      .withIndex("by_colony_active", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("resolved"), false))
      .collect();
  },
});

/**
 * Get encounter for a specific cat.
 */
export const getEncounterByCat = query({
  args: { catId: v.id("cats") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("encounters")
      .withIndex("by_cat", (q) => q.eq("catId", args.catId))
      .filter((q) => q.eq(q.field("resolved"), false))
      .first();
  },
});

/**
 * Create a new encounter.
 */
export const createEncounter = mutation({
  args: {
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
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const encounterId = await ctx.db.insert("encounters", {
      colonyId: args.colonyId,
      catId: args.catId,
      type: args.type,
      enemyType: args.enemyType,
      position: args.position,
      clicksNeeded: args.clicksNeeded,
      clicksReceived: 0,
      createdAt: now,
      expiresAt: now + 30000, // 30 seconds default
      resolved: false,
      outcome: "pending",
    });
    return encounterId;
  },
});

/**
 * Add clicks to an encounter (user helping).
 */
export const addClicks = mutation({
  args: {
    encounterId: v.id("encounters"),
    clicks: v.number(),
  },
  handler: async (ctx, args) => {
    const encounter = await ctx.db.get(args.encounterId);
    if (!encounter) {
      throw new Error("Encounter not found");
    }

    const newClicks = Math.min(
      encounter.clicksNeeded,
      encounter.clicksReceived + args.clicks
    );

    await ctx.db.patch(args.encounterId, {
      clicksReceived: newClicks,
    });

    // Auto-resolve if clicks reached
    if (newClicks >= encounter.clicksNeeded) {
      await ctx.db.patch(args.encounterId, {
        resolved: true,
        outcome: "user_win",
      });
    }
  },
});

/**
 * Resolve an encounter (auto-resolve or combat result).
 */
export const resolveEncounter = mutation({
  args: {
    encounterId: v.id("encounters"),
    won: v.boolean(),
    loot: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const encounter = await ctx.db.get(args.encounterId);
    if (!encounter) {
      throw new Error("Encounter not found");
    }

    await ctx.db.patch(args.encounterId, {
      resolved: true,
      outcome: args.won ? "cat_win" : "cat_lose",
    });

    // If lost, add a scar (50% chance)
    if (!args.won) {
      await ctx.runMutation(internal.discoveries.addCombatScar, {
        catId: encounter.catId,
      });
    }

    // If won and loot provided, add to colony resources
    if (args.won && args.loot) {
      const colony = await ctx.db.get(encounter.colonyId);
      if (colony) {
        await ctx.db.patch(encounter.colonyId, {
          resources: {
            ...colony.resources,
            food: colony.resources.food + args.loot,
          },
        });
      }
    }

    return { won: args.won, loot: args.loot };
  },
});

/**
 * Auto-resolve expired encounters (called by game tick).
 */
export const autoResolveExpired = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const encounters = await ctx.db
      .query("encounters")
      .withIndex("by_colony_active", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) =>
        q.and(
          q.eq(q.field("resolved"), false),
          q.lt(q.field("expiresAt"), now)
        )
      )
      .collect();

    for (const encounter of encounters) {
      // Get cat stats for combat resolution
      const cat = await ctx.db.get(encounter.catId);
      if (!cat) {
        continue;
      }

      // Simple auto-resolve: use cat stats vs enemy
      // In real implementation, would use combat.ts functions
      const catPower = cat.stats.attack + cat.stats.defense;
      const enemyPower = 50; // Default enemy strength
      const won = catPower > enemyPower;

      await ctx.db.patch(encounter._id, {
        resolved: true,
        outcome: won ? "cat_win" : "cat_lose",
      });

      // Apply damage if lost
      if (!won) {
        const damage = 30 + Math.floor(Math.random() * 40); // 30-70 damage
        const newHealth = Math.max(0, cat.needs.health - damage);
        await ctx.db.patch(encounter.catId, {
          needs: {
            ...cat.needs,
            health: newHealth,
          },
        });

        // Add a scar from combat (50% chance)
        if (Math.random() < 0.5) {
          await ctx.runMutation(internal.discoveries.addCombatScar, {
            catId: encounter.catId,
          });
        }
      }
    }
  },
});

/**
 * Check for random encounters (called by game tick).
 */
export const checkRandomEncounters = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    // Get cats on world map
    const catsOnWorld = await ctx.db
      .query("cats")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) =>
        q.and(
          q.eq(q.field("position.map"), "world"),
          q.eq(q.field("deathTime"), null)
        )
      )
      .collect();

    for (const cat of catsOnWorld) {
      // Check if cat already has an encounter
      const existingEncounter = await ctx.db
        .query("encounters")
        .withIndex("by_cat", (q) => q.eq("catId", cat._id))
        .filter((q) => q.eq(q.field("resolved"), false))
        .first();

      if (existingEncounter) continue;

      // Get tile at cat's position
      const tile = await ctx.db
        .query("worldTiles")
        .withIndex("by_colony_position", (q) =>
          q
            .eq("colonyId", args.colonyId)
            .eq("x", cat.position.x)
            .eq("y", cat.position.y)
        )
        .first();

      if (!tile) continue;

      // Calculate encounter chance
      const baseChance = tile.dangerLevel / 100;
      const pathSafety = tile.pathWear > 90 ? 0 : tile.pathWear > 60 ? 0.4 : tile.pathWear > 30 ? 0.25 : 0;
      const visionReduction = cat.stats.vision / 200;
      const encounterChance = baseChance * (1 - pathSafety) * (1 - visionReduction);

      if (Math.random() < encounterChance) {
        // Create encounter
        const enemyTypes: Array<"fox" | "hawk" | "badger" | "bear" | "rival_cat"> = [
          "fox",
          "hawk",
          "badger",
          "bear",
          "rival_cat",
        ];
        const enemyType =
          enemyTypes[Math.floor(Math.random() * enemyTypes.length)];

        const baseClicks = ENEMY_STATS[enemyType].baseClicks;
        const colony = await ctx.db.get(args.colonyId);
        const colonyDefense = 0; // TODO: Calculate from walls
        const clicksNeeded = getClicksNeeded(
          baseClicks,
          colonyDefense,
          cat.stats.vision
        );

        await ctx.db.insert("encounters", {
          colonyId: args.colonyId,
          catId: cat._id,
          type: "predator",
          enemyType,
          position: cat.position,
          clicksNeeded,
          clicksReceived: 0,
          createdAt: Date.now(),
          expiresAt: Date.now() + 30000, // 30 seconds
          resolved: false,
          outcome: "pending",
        });
      }
    }
  },
});



