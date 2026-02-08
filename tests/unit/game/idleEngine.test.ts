import { describe, expect, it } from 'vitest';

import {
  applyClickBoostSeconds,
  getDurationSeconds,
  getHuntReward,
  getResilienceHours,
  getUpgradeCost,
  nextSpecialization,
  type UpgradeLevels,
} from '@/lib/game/idleEngine';

const baseUpgrades: UpgradeLevels = {
  click_power: 0,
  supply_speed: 0,
  hunt_mastery: 0,
  build_mastery: 0,
  ritual_mastery: 0,
  resilience: 0,
};

describe('idle engine', () => {
  it('applies click boost diminishing returns tiers', () => {
    expect(applyClickBoostSeconds(1, 0)).toBe(10);
    expect(applyClickBoostSeconds(31, 0)).toBe(5);
    expect(applyClickBoostSeconds(61, 0)).toBe(2);
  });

  it('reduces hunt duration for hunter specialization', () => {
    const normal = getDurationSeconds('hunt_expedition', null, baseUpgrades);
    const specialized = getDurationSeconds('hunt_expedition', 'hunter', baseUpgrades);
    expect(specialized).toBeLessThan(normal);
  });

  it('increases hunt rewards for high-xp hunter and upgrades', () => {
    const noBonus = getHuntReward(40, null, 0, baseUpgrades);
    const withBonus = getHuntReward(40, 'hunter', 35, {
      ...baseUpgrades,
      hunt_mastery: 2,
    });
    expect(withBonus).toBeGreaterThan(noBonus);
  });

  it('computes resilience hours with cap', () => {
    expect(getResilienceHours(baseUpgrades, 0)).toBe(2);
    expect(getResilienceHours({ ...baseUpgrades, resilience: 3 }, 1)).toBe(26);
    expect(getResilienceHours({ ...baseUpgrades, resilience: 20 }, 20)).toBe(96);
  });

  it('scales upgrade cost by current level', () => {
    expect(getUpgradeCost(5, 0)).toBe(5);
    expect(getUpgradeCost(5, 2)).toBe(15);
  });

  it('unlocks specialization after threshold', () => {
    expect(nextSpecialization('hunter', 9, null)).toBeNull();
    expect(nextSpecialization('hunter', 10, null)).toBe('hunter');
    expect(nextSpecialization('hunter', 40, 'architect')).toBe('architect');
  });
});
