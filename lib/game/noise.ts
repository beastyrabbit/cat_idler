/**
 * Seeded Noise Utilities
 *
 * Simple pseudo-random number generation and noise functions for procedural generation.
 * All functions are deterministic based on seed values.
 */

/**
 * Simple seeded random number generator (Linear Congruential Generator)
 */
class SeededRandom {
  private seed: number;

  constructor(seed: number) {
    this.seed = seed;
  }

  /**
   * Generate next random number in sequence (0 to 1)
   */
  next(): number {
    this.seed = (this.seed * 1664525 + 1013904223) % 2 ** 32;
    return (this.seed >>> 0) / 2 ** 32;
  }

  /**
   * Generate random integer in range [min, max]
   */
  int(min: number, max: number): number {
    return Math.floor(this.next() * (max - min + 1)) + min;
  }

  /**
   * Generate random float in range [min, max)
   */
  float(min: number, max: number): number {
    return this.next() * (max - min) + min;
  }
}

/**
 * Create a seeded random generator from a seed value
 */
export function createSeededRandom(seed: number): SeededRandom {
  return new SeededRandom(seed);
}

/**
 * Hash function to combine multiple values into a single seed
 */
export function hashSeed(...values: (number | string)[]): number {
  let hash = 0;
  for (const value of values) {
    const str = String(value);
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
      hash = hash & hash; // Convert to 32-bit integer
    }
  }
  return Math.abs(hash);
}

/**
 * Simple 2D noise function (value noise)
 * Returns a value between 0 and 1 based on coordinates
 */
export function noise2D(
  x: number,
  y: number,
  seed: number,
  scale: number = 1.0
): number {
  const fx = Math.floor(x * scale);
  const fy = Math.floor(y * scale);
  
  // Get random values for corners
  const rng = createSeededRandom(hashSeed(seed, fx, fy));
  const n00 = rng.next();
  const rng2 = createSeededRandom(hashSeed(seed, fx + 1, fy));
  const n10 = rng2.next();
  const rng3 = createSeededRandom(hashSeed(seed, fx, fy + 1));
  const n01 = rng3.next();
  const rng4 = createSeededRandom(hashSeed(seed, fx + 1, fy + 1));
  const n11 = rng4.next();
  
  // Interpolate
  const dx = (x * scale) - fx;
  const dy = (y * scale) - fy;
  
  const nx0 = n00 * (1 - dx) + n10 * dx;
  const nx1 = n01 * (1 - dx) + n11 * dx;
  
  return nx0 * (1 - dy) + nx1 * dy;
}

/**
 * Fractal noise (multiple octaves)
 */
export function fractalNoise2D(
  x: number,
  y: number,
  seed: number,
  octaves: number = 4,
  persistence: number = 0.5,
  scale: number = 1.0
): number {
  let value = 0;
  let amplitude = 1;
  let frequency = scale;
  let maxValue = 0;

  for (let i = 0; i < octaves; i++) {
    value += noise2D(x, y, seed + i, frequency) * amplitude;
    maxValue += amplitude;
    amplitude *= persistence;
    frequency *= 2;
  }

  return value / maxValue;
}

