export interface SeededRoll {
  value: number;
  nextSeed: number;
}

// Linear Congruential Generator (Numerical Recipes variant, period 2^32)
const MODULUS = 0x1_0000_0000;
const MULTIPLIER = 1664525;
const INCREMENT = 1013904223;

export function normalizeSeed(seed: number): number {
  if (!Number.isFinite(seed)) {
    return 1;
  }

  return Math.max(1, Math.floor(Math.abs(seed)) >>> 0);
}

export function rollSeeded(seed: number): SeededRoll {
  const normalized = normalizeSeed(seed);
  const nextSeed = (normalized * MULTIPLIER + INCREMENT) >>> 0;
  return {
    value: nextSeed / MODULUS,
    nextSeed,
  };
}
