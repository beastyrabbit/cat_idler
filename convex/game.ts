import { mutation, query } from './_generated/server';
import { v } from 'convex/values';

import {
  applyClickBoostSeconds,
  getScaledDurationSeconds,
  getHuntReward,
  getResilienceHours,
  getUpgradeCost,
  nextSpecialization,
  type UpgradeLevels,
  type JobKind,
  type CatSpecialization,
} from '../lib/game/idleEngine';
import {
  consumptionForTick,
  hasConflictingStrategicJob,
  nextColonyStatus,
  ritualRequestIsFresh,
  shouldAutoQueueBuild,
  shouldAutoQueueHunt,
  shouldResetFromCritical,
  shouldStartRitual,
  shouldTrackCritical,
} from '../lib/game/idleRules';
import { inheritTraits, traitsToSpriteParams } from '../lib/game/genetics';
import { configForPreset } from '../lib/game/testAcceleration';

const UPGRADE_DEFAULTS = [
  { key: 'click_power', maxLevel: 20, baseCost: 2, description: 'Increase click speed-up power.' },
  { key: 'supply_speed', maxLevel: 10, baseCost: 3, description: 'Reduce player supply action time.' },
  { key: 'hunt_mastery', maxLevel: 10, baseCost: 5, description: 'Improve hunting speed and yield.' },
  { key: 'build_mastery', maxLevel: 10, baseCost: 5, description: 'Improve planning and build speed.' },
  { key: 'ritual_mastery', maxLevel: 10, baseCost: 6, description: 'Improve ritual cadence and timing.' },
  { key: 'resilience', maxLevel: 10, baseCost: 7, description: 'Survive unattended for longer.' },
] as const;

type UpgradeKey = (typeof UPGRADE_DEFAULTS)[number]['key'];

type Ctx = any;

interface RuntimeConfig {
  timeScale: number;
  resourceDecayMultiplier: number;
  resilienceHoursOverride: number | null;
  criticalMsOverride: number;
}

function getRuntimeConfig(colony: any): RuntimeConfig {
  return {
    timeScale: Math.max(1, colony.testTimeScale ?? 1),
    resourceDecayMultiplier: Math.max(1, colony.testResourceDecayMultiplier ?? 1),
    resilienceHoursOverride:
      typeof colony.testResilienceHoursOverride === 'number' ? colony.testResilienceHoursOverride : null,
    criticalMsOverride: Math.max(1_000, colony.testCriticalMsOverride ?? 5 * 60 * 1000),
  };
}

function randomStat(min: number, max: number): number {
  return min + Math.floor(Math.random() * (max - min + 1));
}

function starterNames(): string[] {
  return ['Whiskers', 'Shadow', 'Luna', 'Max', 'Bella'];
}

async function createStarterCats(ctx: Ctx, colonyId: any) {
  const names = starterNames();
  for (let i = 0; i < 5; i += 1) {
    await ctx.db.insert('cats', {
      colonyId,
      name: names[i] ?? `Cat ${i + 1}`,
      parentIds: [null, null],
      birthTime: Date.now(),
      deathTime: null,
      stats: {
        attack: randomStat(30, 60),
        defense: randomStat(30, 60),
        hunting: randomStat(30, 60),
        medicine: randomStat(20, 50),
        cleaning: randomStat(25, 55),
        building: randomStat(20, 50),
        leadership: randomStat(20, 60),
        vision: randomStat(30, 60),
      },
      needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
      currentTask: null,
      position: { map: 'colony', x: 1, y: 1 },
      isPregnant: false,
      pregnancyDueTime: null,
      spriteParams: traitsToSpriteParams(inheritTraits(null, null)) as Record<string, unknown>,
      specialization: null,
      roleXp: {
        hunter: 0,
        architect: 0,
        ritualist: 0,
      },
    });
  }
}

async function ensureAliveCatDefaults(ctx: Ctx, colonyId: any) {
  const aliveCats = await ctx.db
    .query('cats')
    .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colonyId))
    .filter((q: any) => q.eq(q.field('deathTime'), null))
    .collect();

  for (const cat of aliveCats) {
    const patch: Record<string, unknown> = {};
    let needsPatch = false;

    if (!cat.spriteParams) {
      patch.spriteParams = traitsToSpriteParams(inheritTraits(null, null)) as Record<string, unknown>;
      needsPatch = true;
    }

    if (!cat.roleXp) {
      patch.roleXp = { hunter: 0, architect: 0, ritualist: 0 };
      needsPatch = true;
    }

    if (!('specialization' in cat)) {
      patch.specialization = null;
      needsPatch = true;
    }

    if (needsPatch) {
      await ctx.db.patch(cat._id, patch);
    }
  }
}

async function getGlobalColony(ctx: Ctx) {
  return await ctx.db
    .query('colonies')
    .withIndex('by_is_global', (q: any) => q.eq('isGlobal', true))
    .first();
}

async function getUpgradeRows(ctx: Ctx, colonyId: any) {
  return await ctx.db
    .query('globalUpgrades')
    .withIndex('by_colony', (q: any) => q.eq('colonyId', colonyId))
    .collect();
}

function upgradesToLevels(rows: any[]): UpgradeLevels {
  const map: Record<string, number> = {};
  for (const row of rows) {
    map[row.key] = row.level;
  }
  return {
    click_power: map.click_power ?? 0,
    supply_speed: map.supply_speed ?? 0,
    hunt_mastery: map.hunt_mastery ?? 0,
    build_mastery: map.build_mastery ?? 0,
    ritual_mastery: map.ritual_mastery ?? 0,
    resilience: map.resilience ?? 0,
  };
}

async function ensureGlobalUpgrades(ctx: Ctx, colonyId: any) {
  const existing = await getUpgradeRows(ctx, colonyId);
  const existingKeys = new Set(existing.map((u: any) => u.key));

  for (const upgrade of UPGRADE_DEFAULTS) {
    if (existingKeys.has(upgrade.key)) {
      continue;
    }

    await ctx.db.insert('globalUpgrades', {
      colonyId,
      key: upgrade.key,
      level: 0,
      maxLevel: upgrade.maxLevel,
      baseCost: upgrade.baseCost,
      description: upgrade.description,
    });
  }
}

async function ensureGlobalColony(ctx: Ctx) {
  const now = Date.now();
  let colony = await getGlobalColony(ctx);

  if (!colony) {
    const colonyId = await ctx.db.insert('colonies', {
      name: 'Global Cat Colony',
      leaderId: null,
      status: 'starting',
      resources: {
        food: 24,
        water: 24,
        herbs: 8,
        materials: 0,
        blessings: 0,
      },
      gridSize: 3,
      createdAt: now,
      lastTick: now,
      lastAttack: now,
      worldSeed: now,
      isGlobal: true,
      runNumber: 1,
      runStartedAt: now,
      lastPlayerActivityAt: now,
      lastResetAt: now,
      automationTier: 0,
      globalUpgradePoints: 0,
      ritualRequestedAt: null,
      criticalSince: null,
      testTimeScale: 1,
      testResourceDecayMultiplier: 1,
      testResilienceHoursOverride: null,
      testCriticalMsOverride: 5 * 60 * 1000,
    });

    await createStarterCats(ctx, colonyId);
    await ensureGlobalUpgrades(ctx, colonyId);

    colony = await ctx.db.get(colonyId);
  } else {
    await ensureGlobalUpgrades(ctx, colony._id);

    if (typeof colony.runNumber !== 'number') {
      await ctx.db.patch(colony._id, {
        runNumber: 1,
        runStartedAt: colony.runStartedAt ?? now,
        lastPlayerActivityAt: colony.lastPlayerActivityAt ?? now,
        lastResetAt: colony.lastResetAt ?? now,
        automationTier: colony.automationTier ?? 0,
        globalUpgradePoints: colony.globalUpgradePoints ?? 0,
        ritualRequestedAt: colony.ritualRequestedAt ?? null,
        criticalSince: colony.criticalSince ?? null,
        testTimeScale: colony.testTimeScale ?? 1,
        testResourceDecayMultiplier: colony.testResourceDecayMultiplier ?? 1,
        testResilienceHoursOverride: colony.testResilienceHoursOverride ?? null,
        testCriticalMsOverride: colony.testCriticalMsOverride ?? 5 * 60 * 1000,
      });
      colony = await ctx.db.get(colony._id);
    }
  }

  const aliveCats = await ctx.db
    .query('cats')
    .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colony._id))
    .filter((q: any) => q.eq(q.field('deathTime'), null))
    .collect();

  if (aliveCats.length === 0) {
    await createStarterCats(ctx, colony._id);
  }

  await ensureAliveCatDefaults(ctx, colony._id);

  return await ctx.db.get(colony._id);
}

async function chooseLeader(ctx: Ctx, colonyId: any) {
  const aliveCats = await ctx.db
    .query('cats')
    .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colonyId))
    .filter((q: any) => q.eq(q.field('deathTime'), null))
    .collect();

  if (aliveCats.length === 0) {
    return null;
  }

  let best = aliveCats[0];
  for (const cat of aliveCats) {
    if (cat.stats.leadership > best.stats.leadership) {
      best = cat;
    }
  }
  return best;
}

async function selectBestCat(ctx: Ctx, colonyId: any, specialization: CatSpecialization) {
  const aliveCats = await ctx.db
    .query('cats')
    .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colonyId))
    .filter((q: any) => q.eq(q.field('deathTime'), null))
    .collect();

  if (aliveCats.length === 0) {
    return null;
  }

  const preferred = aliveCats.filter((cat: any) => (cat.specialization ?? null) === specialization);
  const pool = preferred.length > 0 ? preferred : aliveCats;

  let best = pool[0];
  for (const cat of pool) {
    if (specialization === 'hunter' && cat.stats.hunting > best.stats.hunting) {
      best = cat;
    }
    if (specialization === 'architect' && cat.stats.building > best.stats.building) {
      best = cat;
    }
    if (specialization === 'ritualist' && cat.stats.leadership > best.stats.leadership) {
      best = cat;
    }
  }

  return best;
}

async function logEvent(ctx: Ctx, colonyId: any, type: any, message: string, involvedCatIds: any[] = [], metadata: any = {}) {
  await ctx.db.insert('events', {
    colonyId,
    catId: involvedCatIds[0],
    timestamp: Date.now(),
    type,
    message,
    involvedCatIds,
    metadata,
  });
}

async function queueJob(
  ctx: Ctx,
  colonyId: any,
  kind: JobKind,
  requestedByType: 'player' | 'leader' | 'system',
  upgrades: UpgradeLevels,
  runtime: RuntimeConfig,
  requestedByPlayerId: any,
  assignedCat: any,
  metadata: any = {},
) {
  const specialization: CatSpecialization = assignedCat?.specialization ?? null;
  const duration = getScaledDurationSeconds(kind, specialization, upgrades, runtime.timeScale);
  const now = Date.now();

  const jobId = await ctx.db.insert('jobs', {
    colonyId,
    kind,
    status: 'queued',
    requestedByType,
    requestedByPlayerId: requestedByPlayerId ?? undefined,
    assignedCatId: assignedCat?._id ?? null,
    baseDurationSec: duration,
    speedMultiplier: 1,
    yieldMultiplier: 1,
    clickTimeReducedSec: 0,
    createdAt: now,
    startedAt: now,
    endsAt: now + duration * 1000,
    metadata,
  });

  await logEvent(ctx, colonyId, 'job_queued', `Queued ${kind.replace(/_/g, ' ')}`, assignedCat ? [assignedCat._id] : [], {
    jobId,
    kind,
  });

  return jobId;
}

async function resetGlobalRun(ctx: Ctx, colony: any, reason: string) {
  const now = Date.now();

  const activePlayers = await ctx.db
    .query('players')
    .withIndex('by_last_seen', (q: any) => q.gte('lastSeenAt', now - 5 * 60 * 1000))
    .collect();

  await ctx.db.insert('runHistory', {
    colonyId: colony._id,
    runNumber: colony.runNumber ?? 1,
    startedAt: colony.runStartedAt ?? colony.createdAt,
    endedAt: now,
    durationSec: Math.max(1, Math.floor((now - (colony.runStartedAt ?? colony.createdAt)) / 1000)),
    reason,
    finalResources: colony.resources,
    activePlayers: activePlayers.length,
  });

  const statuses: Array<'queued' | 'active' | 'completed' | 'failed' | 'cancelled'> = [
    'queued',
    'active',
    'completed',
    'failed',
    'cancelled',
  ];
  for (const status of statuses) {
    const jobs = await ctx.db
      .query('jobs')
      .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', status))
      .collect();
    for (const job of jobs) {
      await ctx.db.delete(job._id);
    }
  }

  const aliveCats = await ctx.db
    .query('cats')
    .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colony._id))
    .filter((q: any) => q.eq(q.field('deathTime'), null))
    .collect();

  if (aliveCats.length === 0) {
    await createStarterCats(ctx, colony._id);
  } else {
    for (const cat of aliveCats) {
      await ctx.db.patch(cat._id, {
        needs: { hunger: 100, thirst: 100, rest: 100, health: 100 },
        currentTask: null,
        position: { map: 'colony', x: 1, y: 1 },
      });
    }
  }

  await ctx.db.patch(colony._id, {
    status: 'starting',
    resources: {
      food: 24,
      water: 24,
      herbs: 8,
      materials: 0,
      blessings: colony.resources.blessings,
    },
    runNumber: (colony.runNumber ?? 1) + 1,
    runStartedAt: now,
    lastResetAt: now,
    lastTick: now,
    criticalSince: null,
    ritualRequestedAt: null,
  });

  await logEvent(ctx, colony._id, 'run_reset', `The colony collapsed and started run ${(colony.runNumber ?? 1) + 1}.`, [], {
    reason,
  });
}

export const ensureGlobalState = mutation({
  args: {},
  handler: async (ctx) => {
    const colony = await ensureGlobalColony(ctx);
    return colony?._id;
  },
});

export const setTestAcceleration = mutation({
  args: {
    preset: v.union(v.literal('off'), v.literal('fast'), v.literal('turbo')),
  },
  handler: async (ctx, args) => {
    const colony = await ensureGlobalColony(ctx);
    if (!colony) {
      throw new Error('Global colony unavailable');
    }

    const config = configForPreset(args.preset);
    await ctx.db.patch(colony._id, {
      testTimeScale: config.timeScale,
      testResourceDecayMultiplier: config.resourceDecayMultiplier,
      testResilienceHoursOverride: config.resilienceHoursOverride,
      testCriticalMsOverride: config.criticalMsOverride,
    });
    return { preset: args.preset };
  },
});

export const getGlobalDashboard = query({
  args: {},
  handler: async (ctx) => {
    const colony = await getGlobalColony(ctx);
    if (!colony) {
      return null;
    }

    const now = Date.now();

    const cats = await ctx.db
      .query('cats')
      .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colony._id))
      .filter((q: any) => q.eq(q.field('deathTime'), null))
      .collect();

    const jobsQueued = await ctx.db
      .query('jobs')
      .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'queued'))
      .collect();
    const jobsActive = await ctx.db
      .query('jobs')
      .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'active'))
      .collect();

    const jobs = [...jobsActive, ...jobsQueued].sort((a, b) => a.endsAt - b.endsAt);

    const events = await ctx.db
      .query('events')
      .withIndex('by_colony_time', (q: any) => q.eq('colonyId', colony._id))
      .order('desc')
      .take(30);

    const upgrades = await getUpgradeRows(ctx, colony._id);

    const onlinePlayers = await ctx.db
      .query('players')
      .withIndex('by_last_seen', (q: any) => q.gte('lastSeenAt', now - 5 * 60 * 1000))
      .collect();

    const leader = colony.leaderId ? cats.find((cat: any) => cat._id === colony.leaderId) ?? null : null;

    return {
      now,
      colony,
      leader,
      cats: cats.sort((a: any, b: any) => b.stats.leadership - a.stats.leadership),
      jobs,
      upgrades: upgrades.sort((a: any, b: any) => a.key.localeCompare(b.key)),
      events,
      onlineCount: onlinePlayers.length,
    };
  },
});

export const requestJob = mutation({
  args: {
    sessionId: v.string(),
    nickname: v.string(),
    kind: v.union(
      v.literal('supply_food'),
      v.literal('supply_water'),
      v.literal('leader_plan_hunt'),
      v.literal('leader_plan_house'),
      v.literal('ritual'),
    ),
  },
  handler: async (ctx, args) => {
    const colony = await ensureGlobalColony(ctx);
    if (!colony) {
      throw new Error('Global colony unavailable');
    }

    const now = Date.now();

    const player = await upsertPlayer(ctx, args.sessionId, args.nickname, now);
    const upgrades = upgradesToLevels(await getUpgradeRows(ctx, colony._id));
    const runtime = getRuntimeConfig(colony);

    // Prevent duplicate strategic jobs.
    if (args.kind !== 'supply_food' && args.kind !== 'supply_water') {
      const existingActive = await ctx.db
        .query('jobs')
        .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'active'))
        .collect();
      const existingQueued = await ctx.db
        .query('jobs')
        .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'queued'))
        .collect();
      const all = [...existingActive, ...existingQueued];
      if (hasConflictingStrategicJob(args.kind as JobKind, all)) {
        throw new Error('That request is already in progress.');
      }
    }

    if (args.kind === 'ritual') {
      const alreadyRequested = ritualRequestIsFresh(colony.ritualRequestedAt, now);
      const activeJobs = await ctx.db
        .query('jobs')
        .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'active'))
        .collect();
      const queuedJobs = await ctx.db
        .query('jobs')
        .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'queued'))
        .collect();
      const activeRitual = [...activeJobs, ...queuedJobs].some((job: any) => job.kind === 'ritual');
      if (alreadyRequested || activeRitual) {
        throw new Error('Ritual request already pending or active.');
      }

      await ctx.db.patch(colony._id, {
        lastPlayerActivityAt: now,
        ritualRequestedAt: now,
      });

      await ctx.db.patch(player._id, {
        lifetimeContribution: {
          ...player.lifetimeContribution,
          jobsRequested: player.lifetimeContribution.jobsRequested + 1,
        },
      });

      await logEvent(ctx, colony._id, 'ritual_ready', `${args.nickname} requested a ritual. Leader will schedule it when conditions are safe.`);

      return { requested: true };
    }

    const jobId = await queueJob(
      ctx,
      colony._id,
      args.kind as JobKind,
      'player',
      upgrades,
      runtime,
      player._id,
      null,
      {},
    );

    await ctx.db.patch(player._id, {
      lifetimeContribution: {
        ...player.lifetimeContribution,
        jobsRequested: player.lifetimeContribution.jobsRequested + 1,
      },
    });

    await ctx.db.patch(colony._id, {
      lastPlayerActivityAt: now,
      ritualRequestedAt: colony.ritualRequestedAt ?? null,
    });

    return jobId;
  },
});

async function upsertPlayer(ctx: Ctx, sessionId: string, nickname: string, now: number) {
  const existing = await ctx.db
    .query('players')
    .withIndex('by_session', (q: any) => q.eq('sessionId', sessionId))
    .first();

  if (existing) {
    await ctx.db.patch(existing._id, {
      nickname,
      lastSeenAt: now,
    });
    return await ctx.db.get(existing._id);
  }

  const playerId = await ctx.db.insert('players', {
    sessionId,
    nickname,
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

  return await ctx.db.get(playerId);
}

export const clickBoostJob = mutation({
  args: {
    sessionId: v.string(),
    nickname: v.string(),
    jobId: v.id('jobs'),
  },
  handler: async (ctx, args) => {
    const now = Date.now();
    const job = await ctx.db.get(args.jobId);
    if (!job || (job.status !== 'active' && job.status !== 'queued')) {
      throw new Error('This job cannot be boosted.');
    }

    const colony = await ensureGlobalColony(ctx);
    if (!colony || colony._id !== job.colonyId) {
      throw new Error('Invalid colony');
    }

    const player = await upsertPlayer(ctx, args.sessionId, args.nickname, now);
    const upgrades = upgradesToLevels(await getUpgradeRows(ctx, colony._id));

    const inSameWindow = now - player.clickWindowStart < 60_000;
    const clicksInWindow = inSameWindow ? player.clicksInWindow + 1 : 1;
    const windowStart = inSameWindow ? player.clickWindowStart : now;
    const reduceSeconds = applyClickBoostSeconds(clicksInWindow, upgrades.click_power);

    const minEnd = now + 5_000;
    const nextEnd = Math.max(minEnd, job.endsAt - reduceSeconds * 1000);

    await ctx.db.patch(player._id, {
      clickWindowStart: windowStart,
      clicksInWindow,
      lifetimeClicks: player.lifetimeClicks + 1,
    });

    await ctx.db.patch(job._id, {
      endsAt: nextEnd,
      clickTimeReducedSec: (job.clickTimeReducedSec ?? 0) + reduceSeconds,
      status: 'active',
    });

    await ctx.db.patch(colony._id, {
      lastPlayerActivityAt: now,
    });

    return {
      reducedBySec: reduceSeconds,
      newEndsAt: nextEnd,
    };
  },
});

export const purchaseUpgrade = mutation({
  args: {
    sessionId: v.string(),
    nickname: v.string(),
    key: v.union(
      v.literal('click_power'),
      v.literal('supply_speed'),
      v.literal('hunt_mastery'),
      v.literal('build_mastery'),
      v.literal('ritual_mastery'),
      v.literal('resilience'),
    ),
  },
  handler: async (ctx, args) => {
    const colony = await ensureGlobalColony(ctx);
    if (!colony) {
      throw new Error('Global colony unavailable');
    }

    const now = Date.now();
    const player = await upsertPlayer(ctx, args.sessionId, args.nickname, now);

    const upgrade = await ctx.db
      .query('globalUpgrades')
      .withIndex('by_colony_key', (q: any) => q.eq('colonyId', colony._id).eq('key', args.key))
      .first();

    if (!upgrade) {
      throw new Error('Upgrade not found.');
    }

    if (upgrade.level >= upgrade.maxLevel) {
      throw new Error('Upgrade already maxed.');
    }

    const cost = getUpgradeCost(upgrade.baseCost, upgrade.level);
    const points = colony.globalUpgradePoints ?? 0;
    if (points < cost) {
      throw new Error('Not enough ritual points.');
    }

    await ctx.db.patch(upgrade._id, {
      level: upgrade.level + 1,
    });

    await ctx.db.patch(colony._id, {
      globalUpgradePoints: points - cost,
      lastPlayerActivityAt: now,
    });

    await ctx.db.patch(player._id, {
      lifetimeContribution: {
        ...player.lifetimeContribution,
        upgradesPurchased: player.lifetimeContribution.upgradesPurchased + 1,
      },
    });

    await logEvent(ctx, colony._id, 'upgrade_purchased', `${args.nickname} upgraded ${args.key.replace(/_/g, ' ')} to level ${upgrade.level + 1}.`);

    return { level: upgrade.level + 1, remainingPoints: points - cost };
  },
});

export const workerTick = mutation({
  args: {},
  handler: async (ctx) => {
    const colony = await ensureGlobalColony(ctx);
    if (!colony) {
      return { ok: false };
    }

    const now = Date.now();
    const elapsedSec = Math.max(1, Math.floor((now - colony.lastTick) / 1000));

    const upgrades = upgradesToLevels(await getUpgradeRows(ctx, colony._id));
    const runtime = getRuntimeConfig(colony);

    // Ensure best leader.
    const bestLeader = await chooseLeader(ctx, colony._id);
    if (bestLeader && colony.leaderId !== bestLeader._id) {
      await ctx.db.patch(colony._id, { leaderId: bestLeader._id });
      await logEvent(ctx, colony._id, 'leader_change', `${bestLeader.name} is now leading the colony.`, [bestLeader._id]);
    }

    const aliveCats = await ctx.db
      .query('cats')
      .withIndex('by_colony_alive', (q: any) => q.eq('colonyId', colony._id))
      .filter((q: any) => q.eq(q.field('deathTime'), null))
      .collect();

    const { foodUse, waterUse } = consumptionForTick(
      aliveCats.length,
      elapsedSec * runtime.resourceDecayMultiplier,
      upgrades,
    );

    const nextResources = {
      ...colony.resources,
      food: Math.max(0, colony.resources.food - foodUse),
      water: Math.max(0, colony.resources.water - waterUse),
    };

    // Promote queued jobs to active.
    const queuedJobs = await ctx.db
      .query('jobs')
      .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'queued'))
      .collect();

    for (const job of queuedJobs) {
      await ctx.db.patch(job._id, {
        status: 'active',
        startedAt: job.startedAt || now,
      });
    }

    // Auto-plan hunt/build when resources are low.
    const activeJobs = await ctx.db
      .query('jobs')
      .withIndex('by_colony_status', (q: any) => q.eq('colonyId', colony._id).eq('status', 'active'))
      .collect();

    if (shouldAutoQueueHunt(nextResources.food, activeJobs)) {
      await queueJob(
        ctx,
        colony._id,
        'leader_plan_hunt',
        'leader',
        upgrades,
        runtime,
        null,
        await selectBestCat(ctx, colony._id, 'hunter'),
      );
    }

    if (shouldAutoQueueBuild(nextResources.materials, activeJobs)) {
      await queueJob(
        ctx,
        colony._id,
        'leader_plan_house',
        'leader',
        upgrades,
        runtime,
        null,
        await selectBestCat(ctx, colony._id, 'architect'),
      );
    }

    // Ritual from player request only if stable.
    if (shouldStartRitual(colony.ritualRequestedAt, nextResources, activeJobs)) {
      await queueJob(
        ctx,
        colony._id,
        'ritual',
        'leader',
        upgrades,
        runtime,
        null,
        await selectBestCat(ctx, colony._id, 'ritualist'),
      );
      await ctx.db.patch(colony._id, { ritualRequestedAt: null });
      await logEvent(ctx, colony._id, 'ritual_ready', 'Leader approved a ritual window.');
    }

    // Complete due jobs.
    const dueJobs = activeJobs.filter((job: any) => job.endsAt <= now);

    let patchedResources = { ...nextResources };
    let automationTier = colony.automationTier ?? 0;
    let globalUpgradePoints = colony.globalUpgradePoints ?? 0;

    for (const job of dueJobs) {
      const assignedCat = job.assignedCatId ? await ctx.db.get(job.assignedCatId) : null;

      if (job.kind === 'supply_food') {
        patchedResources.food += 8;
        if (job.requestedByPlayerId) {
          const player = await ctx.db.get(job.requestedByPlayerId);
          if (player) {
            await ctx.db.patch(player._id, {
              lifetimeContribution: {
                ...player.lifetimeContribution,
                food: player.lifetimeContribution.food + 8,
              },
            });
          }
        }
      }

      if (job.kind === 'supply_water') {
        patchedResources.water += 8;
        if (job.requestedByPlayerId) {
          const player = await ctx.db.get(job.requestedByPlayerId);
          if (player) {
            await ctx.db.patch(player._id, {
              lifetimeContribution: {
                ...player.lifetimeContribution,
                water: player.lifetimeContribution.water + 8,
              },
            });
          }
        }
      }

      if (job.kind === 'leader_plan_hunt') {
        const hunter = await selectBestCat(ctx, colony._id, 'hunter');
        await queueJob(ctx, colony._id, 'hunt_expedition', 'leader', upgrades, runtime, null, hunter);
      }

      if (job.kind === 'leader_plan_house') {
        const architect = await selectBestCat(ctx, colony._id, 'architect');
        await queueJob(ctx, colony._id, 'build_house', 'leader', upgrades, runtime, null, architect);
      }

      if (job.kind === 'hunt_expedition' && assignedCat) {
        const roleXp = assignedCat.roleXp ?? { hunter: 0, architect: 0, ritualist: 0 };
        const reward = getHuntReward(assignedCat.stats.hunting, assignedCat.specialization ?? null, roleXp.hunter, upgrades);
        patchedResources.food += reward;

        const nextRoleXp = { ...roleXp, hunter: roleXp.hunter + 1 };
        await ctx.db.patch(assignedCat._id, {
          roleXp: nextRoleXp,
          specialization: nextSpecialization('hunter', nextRoleXp.hunter, assignedCat.specialization ?? null),
          stats: {
            ...assignedCat.stats,
            hunting: Math.min(100, assignedCat.stats.hunting + 0.4),
          },
        });
      }

      if (job.kind === 'build_house' && assignedCat) {
        patchedResources.materials += 12;
        automationTier = Math.min(10, automationTier + 0.05);

        const roleXp = assignedCat.roleXp ?? { hunter: 0, architect: 0, ritualist: 0 };
        const nextRoleXp = { ...roleXp, architect: roleXp.architect + 1 };
        await ctx.db.patch(assignedCat._id, {
          roleXp: nextRoleXp,
          specialization: nextSpecialization('architect', nextRoleXp.architect, assignedCat.specialization ?? null),
          stats: {
            ...assignedCat.stats,
            building: Math.min(100, assignedCat.stats.building + 0.4),
          },
        });
      }

      if (job.kind === 'ritual' && assignedCat) {
        globalUpgradePoints += 1 + Math.floor(upgrades.ritual_mastery / 3);

        const roleXp = assignedCat.roleXp ?? { hunter: 0, architect: 0, ritualist: 0 };
        const nextRoleXp = { ...roleXp, ritualist: roleXp.ritualist + 1 };
        await ctx.db.patch(assignedCat._id, {
          roleXp: nextRoleXp,
          specialization: nextSpecialization('ritualist', nextRoleXp.ritualist, assignedCat.specialization ?? null),
        });
      }

      await ctx.db.patch(job._id, {
        status: 'completed',
        completedAt: now,
      });

      await logEvent(
        ctx,
        colony._id,
        'job_completed',
        `Completed ${job.kind.replace(/_/g, ' ')}.`,
        assignedCat ? [assignedCat._id] : [],
      );
    }

    const unattendedHours = (now - (colony.lastPlayerActivityAt ?? now)) / 3_600_000;
    const resilienceHours = runtime.resilienceHoursOverride ?? getResilienceHours(upgrades, automationTier);

    let criticalSince = colony.criticalSince ?? null;
    if (shouldTrackCritical(patchedResources, unattendedHours, resilienceHours)) {
      if (!criticalSince) {
        criticalSince = now;
      }

      // Collapse if critical state lasts 5 minutes after unattended threshold.
      if (shouldResetFromCritical(criticalSince, now, runtime.criticalMsOverride)) {
        await resetGlobalRun(ctx, {
          ...colony,
          resources: patchedResources,
          automationTier,
          runStartedAt: colony.runStartedAt ?? colony.createdAt,
        }, 'unattended-collapse');
        return { ok: true, reset: true };
      }
    } else {
      criticalSince = null;
    }

    const nextStatus = nextColonyStatus(patchedResources);

    await ctx.db.patch(colony._id, {
      resources: patchedResources,
      status: nextStatus,
      automationTier,
      globalUpgradePoints,
      criticalSince,
      lastTick: now,
    });

    return {
      ok: true,
      colonyId: colony._id,
      resources: patchedResources,
      automationTier,
      globalUpgradePoints,
      reset: false,
    };
  },
});
