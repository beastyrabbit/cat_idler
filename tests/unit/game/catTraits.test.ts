import { describe, expect, it } from 'vitest';

import { summarizeCatIdentity, summarizeCatTraits } from '@/lib/game/catTraits';

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

  it('builds full identity details including species and extras', () => {
    const identity = summarizeCatIdentity({
      spriteNumber: 12,
      peltName: 'Classic',
      colour: 'BROWN',
      skinColour: 'PINK',
      eyeColour: 'HAZEL',
      accessories: ['feather', 'leaf'],
      scars: ['ear nick'],
    });

    expect(identity.species).toBe('Domestic Cat');
    expect(identity.sprite).toBe('#12');
    expect(identity.lineage).toBe('Classic');
    expect(identity.skin).toBe('PINK');
    expect(identity.accessories).toBe('feather, leaf');
    expect(identity.scars).toBe('ear nick');
  });

  it('uses fallbacks for missing identity values', () => {
    const identity = summarizeCatIdentity(null);

    expect(identity.species).toBe('Domestic Cat');
    expect(identity.sprite).toBe('Unknown');
    expect(identity.skin).toBe('Unknown');
    expect(identity.accessories).toBe('None');
    expect(identity.scars).toBe('None');
  });
});
