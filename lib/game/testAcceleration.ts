export type TestAccelerationPreset = 'off' | 'fast' | 'turbo';

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

  return {
    timeScale: 120,
    resourceDecayMultiplier: 120,
    resilienceHoursOverride: 0,
    criticalMsOverride: 5_000,
  };
}

export function presetFromTimeScale(timeScale: number | null | undefined): TestAccelerationPreset {
  if ((timeScale ?? 1) >= 120) {
    return 'turbo';
  }
  if ((timeScale ?? 1) >= 20) {
    return 'fast';
  }
  return 'off';
}
