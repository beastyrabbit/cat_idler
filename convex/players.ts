import { mutation, query } from './_generated/server';
import { v } from 'convex/values';

export const upsertPresence = mutation({
  args: {
    sessionId: v.string(),
    nickname: v.string(),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const existing = await ctx.db
      .query('players')
      .withIndex('by_session', (q) => q.eq('sessionId', args.sessionId))
      .first();

    if (existing) {
      await ctx.db.patch(existing._id, {
        nickname: args.nickname,
        lastSeenAt: now,
      });
      return existing._id;
    }

    return await ctx.db.insert('players', {
      sessionId: args.sessionId,
      nickname: args.nickname,
      lastSeenAt: now,
      clickWindowStart: now,
      clicksInWindow: 0,
      lifetimeClicks: 0,
      lifetimeContribution: {
        food: 0,
        water: 0,
        jobsRequested: 0,
        upgradesPurchased: 0,
      },
    });
  },
});

export const getOnlinePlayers = query({
  args: {
    minutes: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const windowMs = (args.minutes ?? 5) * 60 * 1000;
    const cutoff = Date.now() - windowMs;

    return await ctx.db
      .query('players')
      .withIndex('by_last_seen', (q) => q.gte('lastSeenAt', cutoff))
      .collect();
  },
});
