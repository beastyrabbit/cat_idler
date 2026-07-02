export type TestAccelerationPreset = 'off' | 'fast' | 'turbo' | 'hyper' | 'ludicrous';

export interface TestAccelerationConfig {
  timeScale: number;
  resourceDecayMultiplier: number;
  resilienceHoursOverride: number | null;
  criticalMsOverride: number;
}

export function configForPreset(preset: TestAccelerationPreset): TestAccelerationConfig {
  if (preset === 'off') {
    return {
      timeScale: 1,
      resourceDecayMultiplier: 1,
      resilienceHoursOverride: null,
      criticalMsOverride: 5 * 60 * 1000,
    };
  }

  if (preset === 'fast') {
    return {
      timeScale: 20,
      resourceDecayMultiplier: 20,
      resilienceHoursOverride: 0.05,
      criticalMsOverride: 20_000,
    };
  }

  if (preset === 'turbo') {
    return {
      timeScale: 120,
      resourceDecayMultiplier: 120,
      resilienceHoursOverride: 0,
      criticalMsOverride: 5_000,
    };
  }

  if (preset === 'hyper') {
    // Watch the colony live: jobs/movement at 100x, but decay stays
    // survivable so it can actually expand instead of starving.
    return {
      timeScale: 100,
      resourceDecayMultiplier: 20,
      resilienceHoursOverride: 0,
      criticalMsOverride: 10_000,
    };
  }

  return {
    timeScale: 10_000,
    resourceDecayMultiplier: 20,
    resilienceHoursOverride: 0,
    criticalMsOverride: 10_000,
  };
}

export function presetFromTimeScale(timeScale: number | null | undefined): TestAccelerationPreset {
  const scale = timeScale ?? 1;

  if (scale >= 10_000) {
    return 'ludicrous';
  }
  if (scale >= 120 && scale < 10_000 && scale !== 100) {
    return 'turbo';
  }
  if (scale === 100) {
    return 'hyper';
  }
  if (scale >= 20) {
    return 'fast';
  }
  return 'off';
}
