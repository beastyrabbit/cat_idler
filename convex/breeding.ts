/**
 * Breeding System
 *
 * Handles cat breeding and birth.
 */

import { mutation, query, internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";

/**
 * Check if two cats can breed (internal).
 */
export const canBreed = internalMutation({
  args: {
    cat1Id: v.id("cats"),
    cat2Id: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const cat1 = await ctx.db.get(args.cat1Id);
    const cat2 = await ctx.db.get(args.cat2Id);

    if (!cat1 || !cat2) return false;
    if (cat1.deathTime || cat2.deathTime) return false;
    if (cat1.colonyId !== cat2.colonyId) return false;
    if (cat1.isPregnant || cat2.isPregnant) return false;
    if (cat1._id === cat2._id) return false;

    // Check if they're related (same parent)
    const isRelated =
      (cat1.parentIds[0] && cat1.parentIds[0] === cat2.parentIds[0]) ||
      (cat1.parentIds[0] && cat1.parentIds[0] === cat2.parentIds[1]) ||
      (cat1.parentIds[1] && cat1.parentIds[1] === cat2.parentIds[0]) ||
      (cat1.parentIds[1] && cat1.parentIds[1] === cat2.parentIds[1]);

    return !isRelated;
  },
});

/**
 * Attempt breeding between two cats.
 */
export const attemptBreeding = mutation({
  args: {
    cat1Id: v.id("cats"),
    cat2Id: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const cat1 = await ctx.db.get(args.cat1Id);
    const cat2 = await ctx.db.get(args.cat2Id);

    if (!cat1 || !cat2) {
      throw new Error("Cats not found");
    }

    // Check if can breed
    const canBreedResult = await ctx.runMutation(internal.breeding.canBreed, {
      cat1Id: args.cat1Id,
      cat2Id: args.cat2Id,
    });

    if (!canBreedResult) {
      throw new Error("Cats cannot breed");
    }

    // 30% base chance, increased by fertility blessing
    const baseChance = 0.3;
    const chance = baseChance; // TODO: Add fertility blessing modifier

    if (Math.random() < chance) {
      // Success - make one cat pregnant
      const pregnantCat = Math.random() < 0.5 ? cat1 : cat2;
      const pregnancyDuration = 2 * 60 * 60 * 1000; // 2 hours
      const dueTime = Date.now() + pregnancyDuration;

      await ctx.db.patch(pregnantCat._id, {
        isPregnant: true,
        pregnancyDueTime: dueTime,
      });

      await ctx.runMutation(internal.events.logEventInternal, {
        colonyId: cat1.colonyId,
        type: "breeding",
        message: `${cat1.name} and ${cat2.name} are expecting kittens!`,
        involvedCatIds: [cat1._id, cat2._id],
      });

      return { success: true, dueTime };
    }

    return { success: false };
  },
});

/**
 * Process births (called by game tick).
 */
export const processBirths = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const allCats = await ctx.db
      .query("cats")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("isPregnant"), true))
      .collect();

    for (const cat of allCats) {
      if (cat.pregnancyDueTime && cat.pregnancyDueTime <= now) {
        // Give birth
        const partnerId = cat.parentIds[0] || cat.parentIds[1];
        const partner = partnerId ? await ctx.db.get(partnerId) : null;

        // Generate kitten stats (average of parents with some variation)
        const baseStats = {
          attack: Math.floor(
            (cat.stats.attack + (partner?.stats.attack || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          defense: Math.floor(
            (cat.stats.defense + (partner?.stats.defense || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          hunting: Math.floor(
            (cat.stats.hunting + (partner?.stats.hunting || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          medicine: Math.floor(
            (cat.stats.medicine + (partner?.stats.medicine || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          cleaning: Math.floor(
            (cat.stats.cleaning + (partner?.stats.cleaning || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          building: Math.floor(
            (cat.stats.building + (partner?.stats.building || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          leadership: Math.floor(
            (cat.stats.leadership + (partner?.stats.leadership || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
          vision: Math.floor(
            (cat.stats.vision + (partner?.stats.vision || 50)) / 2 +
              (Math.random() - 0.5) * 20
          ),
        };

        // Clamp stats to 0-100
        Object.keys(baseStats).forEach((key) => {
          baseStats[key as keyof typeof baseStats] = Math.max(
            0,
            Math.min(100, baseStats[key as keyof typeof baseStats])
          );
        });

        const kittenName = `Kitten ${Math.floor(Math.random() * 1000)}`;

        const kittenId = await ctx.runMutation(internal.cats.createCatInternal, {
          colonyId: args.colonyId,
          name: kittenName,
          stats: baseStats,
          parentIds: [cat._id, partnerId || null],
        });

        // Reset pregnancy
        await ctx.db.patch(cat._id, {
          isPregnant: false,
          pregnancyDueTime: null,
        });

        await ctx.runMutation(internal.events.logEventInternal, {
          colonyId: args.colonyId,
          type: "birth",
          message: `${cat.name} gave birth to ${kittenName}!`,
          involvedCatIds: [cat._id, kittenId],
        });
      }
    }
  },
});



