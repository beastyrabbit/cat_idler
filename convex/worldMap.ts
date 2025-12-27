/**
 * World Map Operations
 *
 * Convex mutations and queries for world tile management.
 * Uses chunk-based procedural generation (12x12 tiles per chunk).
 */

import { query, mutation, internalMutation, MutationCtx } from "./_generated/server";
import { Id, Doc } from "./_generated/dataModel";
import { internal } from "./_generated/api";
import { v } from "convex/values";
import { generateChunk, getColonyPosition, tileToChunk } from "../lib/game/worldGen";

// Helper type for inserting world tiles
type WorldTileInsert = Omit<Doc<"worldTiles">, "_id" | "_creationTime">;

const CHUNK_SIZE = 12;

/**
 * Get all world tiles for a colony (for backward compatibility).
 * Note: This loads all tiles, which may be slow for large explored areas.
 * Consider using getChunkTiles instead.
 */
export const getWorldTiles = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("worldTiles")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();
  },
});

/**
 * Get tiles for a specific chunk.
 */
export const getChunkTiles = query({
  args: {
    colonyId: v.id("colonies"),
    chunkX: v.number(),
    chunkY: v.number(),
  },
  handler: async (ctx, args) => {
    const minX = args.chunkX * CHUNK_SIZE;
    const minY = args.chunkY * CHUNK_SIZE;
    const maxX = minX + CHUNK_SIZE;
    const maxY = minY + CHUNK_SIZE;

    const tiles = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) =>
        q.and(
          q.gte(q.field("x"), minX),
          q.lt(q.field("x"), maxX),
          q.gte(q.field("y"), minY),
          q.lt(q.field("y"), maxY)
        )
      )
      .collect();

    return tiles;
  },
});

/**
 * Get tile at specific coordinates.
 * Generates chunk if it doesn't exist.
 */
export const getTileAt = query({
  args: {
    colonyId: v.id("colonies"),
    x: v.number(),
    y: v.number(),
  },
  handler: async (ctx, args) => {
    // Try to get existing tile
    const existing = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony_position", (q) =>
        q.eq("colonyId", args.colonyId).eq("x", args.x).eq("y", args.y)
      )
      .first();

    if (existing) {
      return existing;
    }

    // Tile doesn't exist, need to generate chunk
    // But queries can't mutate, so return null and let caller handle it
    return null;
  },
});

/**
 * Ensure a chunk exists (generate if needed) - public mutation.
 */
export const ensureChunk = mutation({
  args: {
    colonyId: v.id("colonies"),
    chunkX: v.number(),
    chunkY: v.number(),
  },
  handler: async (ctx, args) => {
    await ensureChunkHandler(ctx, args);
  },
});

/**
 * Ensure a chunk exists (generate if needed) - internal mutation.
 */
export const ensureChunkInternal = internalMutation({
  args: {
    colonyId: v.id("colonies"),
    chunkX: v.number(),
    chunkY: v.number(),
  },
  handler: async (ctx, args) => {
    await ensureChunkHandler(ctx, args);
  },
});

/**
 * Shared handler for chunk generation.
 */
async function ensureChunkHandler(
  ctx: MutationCtx,
  args: { colonyId: Id<"colonies">; chunkX: number; chunkY: number }
) {
    // Check if chunk already exists
    const minX = args.chunkX * CHUNK_SIZE;
    const minY = args.chunkY * CHUNK_SIZE;
    const existing = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) =>
        q.and(
          q.gte(q.field("x"), minX),
          q.lt(q.field("x"), minX + CHUNK_SIZE),
          q.gte(q.field("y"), minY),
          q.lt(q.field("y"), minY + CHUNK_SIZE)
        )
      )
      .first();

    if (existing) {
      return; // Chunk already exists
    }

    // Get colony to get world seed
    const colony = await ctx.db.get(args.colonyId);
    if (!colony) {
      throw new Error("Colony not found");
    }

    const worldSeed = colony.worldSeed ?? Date.now();
    const colonyPos = getColonyPosition();

    // Generate chunk
    const tiles = generateChunk(
      args.chunkX,
      args.chunkY,
      worldSeed,
      colonyPos.x,
      colonyPos.y
    );

  // Insert tiles
  for (const tile of tiles) {
    const tileData: WorldTileInsert = {
      ...tile,
      colonyId: args.colonyId,
    };
    await ctx.db.insert("worldTiles", tileData);
  }
}

/**
 * Initialize world map for a colony (3x3 chunks = 36x36 tiles).
 */
export const initializeWorldMap = mutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    await initializeWorldMapHandler(ctx, args);
  },
});

/**
 * Initialize world map for a colony (internal).
 */
export const initializeWorldMapInternal = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    await initializeWorldMapHandler(ctx, args);
  },
});

/**
 * Shared handler for world map initialization.
 */
async function initializeWorldMapHandler(
  ctx: MutationCtx,
  args: { colonyId: Id<"colonies"> }
) {
  // Get or create world seed
  const colony = await ctx.db.get(args.colonyId);
  if (!colony) {
    throw new Error("Colony not found");
  }

  let worldSeed = colony.worldSeed;
  if (!worldSeed) {
    worldSeed = Date.now();
    await ctx.db.patch(args.colonyId, { worldSeed });
  }

  const colonyPos = getColonyPosition();

  // Generate 3x3 chunks around center (chunk 0,0)
  // Chunks: (-1,-1) to (1,1)
  for (let chunkY = -1; chunkY <= 1; chunkY++) {
    for (let chunkX = -1; chunkX <= 1; chunkX++) {
      // Check if chunk already exists
      const minX = chunkX * CHUNK_SIZE;
      const minY = chunkY * CHUNK_SIZE;
      const existing = await ctx.db
        .query("worldTiles")
        .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
        .filter((q) =>
          q.and(
            q.gte(q.field("x"), minX),
            q.lt(q.field("x"), minX + CHUNK_SIZE),
            q.gte(q.field("y"), minY),
            q.lt(q.field("y"), minY + CHUNK_SIZE)
          )
        )
        .first();

      if (existing) {
        continue; // Chunk already exists
      }

      // Generate chunk
      const tiles = generateChunk(
        chunkX,
        chunkY,
        worldSeed,
        colonyPos.x,
        colonyPos.y
      );

      // Insert tiles
      for (const tile of tiles) {
        const tileData: WorldTileInsert = {
          ...tile,
          colonyId: args.colonyId,
        };
        await ctx.db.insert("worldTiles", tileData);
      }
    }
  }
}

/**
 * Harvest resources from a tile.
 */
export const harvestTile = mutation({
  args: {
    tileId: v.id("worldTiles"),
    harvestType: v.union(v.literal("food"), v.literal("herbs"), v.literal("water")),
    catSkill: v.number(),
  },
  handler: async (ctx, args) => {
    const tile = await ctx.db.get(args.tileId);
    if (!tile) {
      throw new Error("Tile not found");
    }

    // Calculate harvest amount: 1 + (skill / 50)
    const harvestAmount = 1 + Math.floor(args.catSkill / 50);
    const currentAmount = tile.resources[args.harvestType];
    const harvested = Math.min(harvestAmount, currentAmount);

    const newResources = {
      ...tile.resources,
      [args.harvestType]: currentAmount - harvested,
    };

    await ctx.db.patch(args.tileId, {
      resources: newResources,
      lastDepleted: newResources[args.harvestType] === 0 ? Date.now() : tile.lastDepleted,
    });

    return harvested;
  },
});

/**
 * Add path wear to a tile (internal).
 * Also ensures the tile exists (generates chunk if needed).
 */
export const addPathWear = internalMutation({
  args: {
    colonyId: v.id("colonies"),
    x: v.number(),
    y: v.number(),
    amount: v.number(),
  },
  handler: async (ctx, args) => {
    // Get or create tile
    let tile = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony_position", (q) =>
        q.eq("colonyId", args.colonyId).eq("x", args.x).eq("y", args.y)
      )
      .first();

    if (!tile) {
      // Generate chunk containing this tile
      const chunk = tileToChunk(args.x, args.y);
      await ctx.runMutation(internal.worldMap.ensureChunkInternal, {
        colonyId: args.colonyId,
        chunkX: chunk.chunkX,
        chunkY: chunk.chunkY,
      });

      // Get the tile again
      tile = await ctx.db
        .query("worldTiles")
        .withIndex("by_colony_position", (q) =>
          q.eq("colonyId", args.colonyId).eq("x", args.x).eq("y", args.y)
        )
        .first();

      if (!tile) {
        throw new Error("Failed to generate tile");
      }
    }

    const newWear = Math.min(100, tile.pathWear + args.amount);
    await ctx.db.patch(tile._id, {
      pathWear: newWear,
    });
  },
});

/**
 * Regenerate tiles for a single colony (internal).
 */
export const regenerateTiles = internalMutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    const tiles = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();

    const now = Date.now();

    for (const tile of tiles) {
      // Skip rivers (infinite water)
      if (tile.type === "river" || tile.overlayFeature === "river") {
        continue;
      }

      // Regenerate if depleted
      if (tile.lastDepleted > 0) {
        const hoursSinceDepletion = (now - tile.lastDepleted) / (1000 * 60 * 60);
        const regenPercent = Math.min(1.0, hoursSinceDepletion * 0.1);
        const finalPercent = hoursSinceDepletion >= 6 ? 1.0 : regenPercent;

        const newResources = {
          food: Math.min(
            tile.maxResources.food,
            Math.floor(tile.resources.food + tile.maxResources.food * finalPercent)
          ),
          herbs: Math.min(
            tile.maxResources.herbs,
            Math.floor(tile.resources.herbs + tile.maxResources.herbs * finalPercent)
          ),
          water: tile.resources.water,
        };

        await ctx.db.patch(tile._id, {
          resources: newResources,
          lastDepleted: newResources.food === tile.maxResources.food && 
                       newResources.herbs === tile.maxResources.herbs ? 0 : tile.lastDepleted,
        });
      }

      // Decay path wear
      if (tile.pathWear > 0) {
        const newWear = Math.max(0, tile.pathWear - 1);
        await ctx.db.patch(tile._id, {
          pathWear: newWear,
        });
      }
    }
  },
});

/**
 * Regenerate tiles for all colonies (called by cron).
 */
export const regenerateTilesForAllColonies = internalMutation({
  args: {},
  handler: async (ctx) => {
    const colonies = await ctx.db.query("colonies").collect();
    for (const colony of colonies) {
      if (colony.status !== "dead") {
        await ctx.runMutation(internal.worldMap.regenerateTiles, {
          colonyId: colony._id,
        });
      }
    }
  },
});

/**
 * Decay paths for all colonies (called by cron).
 */
export const decayPathsForAllColonies = internalMutation({
  args: {},
  handler: async (ctx) => {
    const colonies = await ctx.db.query("colonies").collect();
    for (const colony of colonies) {
      if (colony.status !== "dead") {
        await ctx.runMutation(internal.worldMap.regenerateTiles, {
          colonyId: colony._id,
        });
      }
    }
  },
});

/**
 * Reset world map (for testing/debugging).
 * Generates a new seed and regenerates all chunks.
 */
export const resetWorldMap = mutation({
  args: {
    colonyId: v.id("colonies"),
  },
  handler: async (ctx, args) => {
    // Generate new seed
    const newSeed = Date.now();
    await ctx.db.patch(args.colonyId, { worldSeed: newSeed });

    // Delete all existing tiles
    const existingTiles = await ctx.db
      .query("worldTiles")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();

    for (const tile of existingTiles) {
      await ctx.db.delete(tile._id);
    }

    // Regenerate 3x3 starting chunks
    await initializeWorldMapHandler(ctx, args);
  },
});

/**
 * Expand revealed area (for testing/debugging).
 * Generates chunks in a square around the center.
 */
export const expandRevealedArea = mutation({
  args: {
    colonyId: v.id("colonies"),
    size: v.number(), // Size of square (e.g., 4 = 4x4 chunks)
  },
  handler: async (ctx, args) => {
    const colony = await ctx.db.get(args.colonyId);
    if (!colony) {
      throw new Error("Colony not found");
    }

    const worldSeed = colony.worldSeed ?? Date.now();
    const colonyPos = getColonyPosition();

    const halfSize = Math.floor(args.size / 2);

    // Generate chunks in a square around center (chunk 0,0)
    for (let chunkY = -halfSize; chunkY <= halfSize; chunkY++) {
      for (let chunkX = -halfSize; chunkX <= halfSize; chunkX++) {
        // Check if chunk already exists
        const minX = chunkX * CHUNK_SIZE;
        const minY = chunkY * CHUNK_SIZE;
        const existing = await ctx.db
          .query("worldTiles")
          .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
          .filter((q) =>
            q.and(
              q.gte(q.field("x"), minX),
              q.lt(q.field("x"), minX + CHUNK_SIZE),
              q.gte(q.field("y"), minY),
              q.lt(q.field("y"), minY + CHUNK_SIZE)
            )
          )
          .first();

        if (existing) {
          continue; // Chunk already exists
        }

        // Generate chunk
        const tiles = generateChunk(
          chunkX,
          chunkY,
          worldSeed,
          colonyPos.x,
          colonyPos.y
        );

        // Insert tiles
        for (const tile of tiles) {
          const tileData: WorldTileInsert = {
            ...tile,
            colonyId: args.colonyId,
          };
          await ctx.db.insert("worldTiles", tileData);
        }
      }
    }
  },
});
