import { describe, expect, it } from 'vitest';

import { configForPreset, presetFromTimeScale } from '@/lib/game/testAcceleration';

describe('test acceleration presets', () => {
  it('returns expected config for each preset', () => {
    expect(configForPreset('off')).toEqual({
      timeScale: 1,
      resourceDecayMultiplier: 1,
      resilienceHoursOverride: null,
      criticalMsOverride: 300000,
    });

    expect(configForPreset('fast')).toEqual({
      timeScale: 20,
      resourceDecayMultiplier: 20,
      resilienceHoursOverride: 0.05,
      criticalMsOverride: 20000,
    });

    expect(configForPreset('turbo')).toEqual({
      timeScale: 120,
      resourceDecayMultiplier: 120,
      resilienceHoursOverride: 0,
      criticalMsOverride: 5000,
    });
  });

  it('infers preset from time scale', () => {
    expect(presetFromTimeScale(undefined)).toBe('off');
    expect(presetFromTimeScale(1)).toBe('off');
    expect(presetFromTimeScale(20)).toBe('fast');
    expect(presetFromTimeScale(120)).toBe('turbo');
  });
});
