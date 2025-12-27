/**
 * Task Queue Operations
 *
 * Convex mutations and queries for task management.
 * TASK: CONVEX-004
 */

import { query, mutation, internalMutation } from "./_generated/server";
import { v } from "convex/values";

/**
 * Get all tasks for a colony.
 */
export const getTasksByColony = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("tasks")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .order("desc")
      .collect();
  },
});

/**
 * Get pending tasks (not yet assigned).
 */
export const getPendingTasks = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("tasks")
      .withIndex("by_colony_assigned", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("assignedCatId"), null))
      .collect();
  },
});

/**
 * Get active tasks (assigned to cats).
 */
export const getActiveTasks = query({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    return await ctx.db
      .query("tasks")
      .withIndex("by_colony_assigned", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.neq(q.field("assignedCatId"), null))
      .collect();
  },
});

/**
 * Create a new task.
 */
export const createTask = mutation({
  args: {
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
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const taskId = await ctx.db.insert("tasks", {
      colonyId: args.colonyId,
      type: args.type,
      priority: args.priority,
      assignedCatId: null,
      assignmentCountdown: 30, // Default countdown
      isOptimalAssignment: false,
      progress: 0,
      createdAt: now,
    });
    return taskId;
  },
});

/**
 * Assign a cat to a task (internal).
 */
export const assignCatToTask = internalMutation({
  args: {
    taskId: v.id("tasks"),
    catId: v.union(v.id("cats"), v.null()),
    isOptimal: v.boolean(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.taskId, {
      assignedCatId: args.catId,
      isOptimalAssignment: args.isOptimal,
      assignmentCountdown: 0, // Assignment complete
    });

    // Keep cat state in sync for UI/AI.
    if (args.catId) {
      const task = await ctx.db.get(args.taskId);
      if (task) {
        await ctx.db.patch(args.catId, { currentTask: task.type });
      }
    }
  },
});

/**
 * Progress a task (internal).
 */
export const progressTask = internalMutation({
  args: {
    taskId: v.id("tasks"),
    amount: v.number(),
  },
  handler: async (ctx, args) => {
    const task = await ctx.db.get(args.taskId);
    if (!task) {
      throw new Error("Task not found");
    }
    
    const newProgress = Math.min(100, task.progress + args.amount);
    await ctx.db.patch(args.taskId, {
      progress: newProgress,
    });
  },
});

/**
 * Complete a task (delete it - internal).
 */
export const completeTask = internalMutation({
  args: {
    taskId: v.id("tasks"),
  },
  handler: async (ctx, args) => {
    const task = await ctx.db.get(args.taskId);
    if (task?.assignedCatId) {
      await ctx.db.patch(task.assignedCatId, { currentTask: null });
    }
    await ctx.db.delete(args.taskId);
  },
});

/**
 * Speed up assignment (user intervention - instant assignment).
 */
export const speedUpAssignment = mutation({
  args: {
    taskId: v.id("tasks"),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.taskId, {
      assignmentCountdown: 0,
    });
  },
});

/**
 * Update assignment countdown.
 */
export const updateAssignmentCountdown = mutation({
  args: {
    taskId: v.id("tasks"),
    countdown: v.number(),
  },
  handler: async (ctx, args) => {
    await ctx.db.patch(args.taskId, {
      assignmentCountdown: args.countdown,
    });
  },
});



