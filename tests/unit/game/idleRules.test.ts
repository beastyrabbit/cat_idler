import { describe, expect, it } from 'vitest';

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
} from '@/lib/game/idleRules';
import type { UpgradeLevels } from '@/lib/game/idleEngine';

const baseUpgrades: UpgradeLevels = {
  click_power: 0,
  supply_speed: 0,
  hunt_mastery: 0,
  build_mastery: 0,
  ritual_mastery: 0,
  resilience: 0,
};

describe('idle rules', () => {
  it('detects strategic conflicts correctly', () => {
    expect(hasConflictingStrategicJob('leader_plan_hunt', [{ kind: 'hunt_expedition' }])).toBe(true);
    expect(hasConflictingStrategicJob('leader_plan_house', [{ kind: 'build_house' }])).toBe(true);
    expect(hasConflictingStrategicJob('ritual', [{ kind: 'ritual' }])).toBe(true);
    expect(hasConflictingStrategicJob('leader_plan_hunt', [{ kind: 'supply_food' }])).toBe(false);
  });

  it('auto-queues hunt/build only when resources low and no conflicts', () => {
    expect(shouldAutoQueueHunt(11, [])).toBe(true);
    expect(shouldAutoQueueHunt(12, [])).toBe(false);
    expect(shouldAutoQueueHunt(3, [{ kind: 'leader_plan_hunt' }])).toBe(false);

    expect(shouldAutoQueueBuild(7, [])).toBe(true);
    expect(shouldAutoQueueBuild(8, [])).toBe(false);
    expect(shouldAutoQueueBuild(2, [{ kind: 'build_house' }])).toBe(false);
  });

  it('starts ritual only when requested, stable, and conflict-free', () => {
    expect(shouldStartRitual(null, { food: 20, water: 20 }, [])).toBe(false);
    expect(shouldStartRitual(Date.now(), { food: 15, water: 20 }, [])).toBe(false);
    expect(shouldStartRitual(Date.now(), { food: 20, water: 20 }, [{ kind: 'ritual' }])).toBe(false);
    expect(shouldStartRitual(Date.now(), { food: 20, water: 20 }, [])).toBe(true);
  });

  it('computes consumption with resilience impact', () => {
    const base = consumptionForTick(5, 600, baseUpgrades);
    const resilient = consumptionForTick(5, 600, { ...baseUpgrades, resilience: 4 });

    expect(base.foodUse).toBeGreaterThan(resilient.foodUse);
    expect(base.waterUse).toBeGreaterThan(resilient.waterUse);
  });

  it('maps supply levels to colony status bands', () => {
    expect(nextColonyStatus({ food: 1, water: 2, herbs: 3 })).toBe('struggling');
    expect(nextColonyStatus({ food: 30, water: 30, herbs: 20 })).toBe('thriving');
    expect(nextColonyStatus({ food: 10, water: 10, herbs: 10 })).toBe('starting');
  });

  it('tracks critical unattended state and reset threshold', () => {
    expect(shouldTrackCritical({ food: 0, water: 10 }, 5, 4)).toBe(true);
    expect(shouldTrackCritical({ food: 0, water: 10 }, 3, 4)).toBe(false);

    const now = Date.now();
    expect(shouldResetFromCritical(now - 5 * 60 * 1000, now)).toBe(true);
    expect(shouldResetFromCritical(now - 4 * 60 * 1000, now)).toBe(false);
  });

  it('validates ritual request freshness window', () => {
    const now = Date.now();
    expect(ritualRequestIsFresh(now - 1000, now)).toBe(true);
    expect(ritualRequestIsFresh(now - 13 * 60 * 60 * 1000, now)).toBe(false);
  });
});
