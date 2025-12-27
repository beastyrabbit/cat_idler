/**
 * Event Log Operations
 *
 * Convex mutations and queries for event logging.
 * TASK: CONVEX-007
 */

import { query, mutation, internalMutation } from "./_generated/server";
import { v } from "convex/values";

/**
 * Get events for a colony (most recent first).
 */
export const getEventsByColony = query({
  args: {
    colonyId: v.id("colonies"),
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const events = await ctx.db
      .query("events")
      .withIndex("by_colony_time", (q) => q.eq("colonyId", args.colonyId))
      .order("desc")
      .take(args.limit ?? 100);
    return events;
  },
});

/**
 * Get events involving a specific cat.
 */
export const getEventsByCat = query({
  args: { catId: v.id("cats") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("events")
      .withIndex("by_cat", (q) => q.eq("catId", args.catId))
      .order("desc")
      .collect();
  },
});

/**
 * Log an event.
 */
export const logEvent = mutation({
  args: {
    colonyId: v.id("colonies"),
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
    involvedCatIds: v.optional(v.array(v.id("cats"))),
    metadata: v.optional(v.any()),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("events", {
      colonyId: args.colonyId,
      catId: args.involvedCatIds?.[0],
      timestamp: Date.now(),
      type: args.type,
      message: args.message,
      involvedCatIds: args.involvedCatIds ?? [],
      metadata: args.metadata ?? {},
    });
  },
});

/**
 * Log an event (internal).
 */
export const logEventInternal = internalMutation({
  args: {
    colonyId: v.id("colonies"),
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
    involvedCatIds: v.optional(v.array(v.id("cats"))),
    metadata: v.optional(v.any()),
  },
  handler: async (ctx, args) => {
    await ctx.db.insert("events", {
      colonyId: args.colonyId,
      catId: args.involvedCatIds?.[0],
      timestamp: Date.now(),
      type: args.type,
      message: args.message,
      involvedCatIds: args.involvedCatIds ?? [],
      metadata: args.metadata ?? {},
    });
  },
});



