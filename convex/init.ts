/**
 * Colony Initialization
 *
 * Helper functions to initialize a new colony with starting cats and world map.
 */

import { internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";

/**
 * Initialize a new colony with starting cats and world map (internal).
 */
export const initializeColony = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    // Create 5 starting cats
    const catNames = ["Whiskers", "Shadow", "Luna", "Max", "Bella"];
    const catIds: string[] = [];

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

      const catId = await ctx.runMutation(internal.cats.createCatInternal, {
        colonyId: args.colonyId,
        name: catNames[i] || `Cat ${i + 1}`,
        stats,
        parentIds: [null, null],
      });

      catIds.push(catId);
    }

    // Initialize world map
    await ctx.runMutation(internal.worldMap.initializeWorldMapInternal, {
      colonyId: args.colonyId,
    });

    // Create starting den at center
    const gridSize = 3; // Starting grid size
    const center = Math.floor(gridSize / 2);
    const denId = await ctx.runMutation(internal.buildings.placeBuildingInternal as any, {
      colonyId: args.colonyId,
      type: "den",
      position: { x: center, y: center },
    });

    // Complete the den immediately
    await ctx.db.patch(denId, {
      constructionProgress: 100,
    });

    return { catIds };
  },
});



