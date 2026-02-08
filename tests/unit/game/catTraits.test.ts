import { describe, expect, it } from 'vitest';

import { summarizeCatTraits } from '@/lib/game/catTraits';

describe('cat trait summary', () => {
  it('returns useful defaults when sprite params are missing', () => {
    expect(summarizeCatTraits(null)).toEqual({
      lineage: 'Unknown lineage',
      coat: 'Unknown coat',
      eyes: 'Unknown eyes',
      markings: 'None',
    });
  });

  it('extracts main lineage and attributes from sprite params', () => {
    const summary = summarizeCatTraits({
      peltName: 'Tabby',
      colour: 'GRAY',
      tint: 'BLUE',
      eyeColour: 'GREEN',
      whitePatches: 'LITTLE',
      points: 'POINTED',
      isTortie: true,
    });

    expect(summary.lineage).toBe('Tabby');
    expect(summary.coat).toBe('GRAY · BLUE');
    expect(summary.eyes).toBe('GREEN');
    expect(summary.markings).toContain('Tortie');
    expect(summary.markings).toContain('White LITTLE');
    expect(summary.markings).toContain('Points POINTED');
  });

  it('shows heterochromia eye pair when second eye exists', () => {
    const summary = summarizeCatTraits({
      peltName: 'Smoke',
      colour: 'BLACK',
      eyeColour: 'BLUE',
      eyeColour2: 'YELLOW',
    });

    expect(summary.eyes).toBe('BLUE/YELLOW');
  });
});
