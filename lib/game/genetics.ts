/**
 * Cat Genetics System
 * 
 * Handles trait inheritance from parents to kittens.
 * Major traits (pelt, color, eyes) are inherited, minor traits have variation.
 */

import type { CatSpriteParams } from '@/lib/cat-renderer/types';

export interface GeneticTraits {
  peltName: string;
  colour: string;
  eyeColour: string;
  skinColour: string;
  isTortie: boolean;
  tint: string;
  whitePatches?: string;
  points?: string;
}

/**
 * Extract genetic traits from sprite params
 */
export function extractGeneticTraits(params: CatSpriteParams | null): GeneticTraits | null {
  if (!params) return null;
  
  return {
    peltName: params.peltName || 'Tabby',
    colour: params.colour || 'GRAY',
    eyeColour: params.eyeColour || 'YELLOW',
    skinColour: params.skinColour || 'PINK',
    isTortie: params.isTortie || false,
    tint: params.tint || 'none',
    whitePatches: params.whitePatches,
    points: params.points,
  };
}

/**
 * Inherit traits from two parents
 */
export function inheritTraits(
  parent1Traits: GeneticTraits | null,
  parent2Traits: GeneticTraits | null
): GeneticTraits {
  // If no parents, return random (for founder cats)
  if (!parent1Traits && !parent2Traits) {
    return generateRandomTraits();
  }

  // If only one parent, inherit from that parent with some mutation
  if (!parent1Traits && parent2Traits) {
    return inheritWithMutation(parent2Traits);
  }
  if (parent1Traits && !parent2Traits) {
    return inheritWithMutation(parent1Traits);
  }

  // Both parents exist - inherit from one randomly (50/50)
  const traits: GeneticTraits = {
    peltName: Math.random() < 0.5 ? parent1Traits!.peltName : parent2Traits!.peltName,
    colour: Math.random() < 0.5 ? parent1Traits!.colour : parent2Traits!.colour,
    eyeColour: Math.random() < 0.5 ? parent1Traits!.eyeColour : parent2Traits!.eyeColour,
    skinColour: Math.random() < 0.5 ? parent1Traits!.skinColour : parent2Traits!.skinColour,
    isTortie: determineTortieInheritance(parent1Traits!.isTortie, parent2Traits!.isTortie),
    tint: Math.random() < 0.5 ? parent1Traits!.tint : parent2Traits!.tint,
    whitePatches: inheritWhitePatches(parent1Traits!.whitePatches, parent2Traits!.whitePatches),
    points: inheritPoints(parent1Traits!.points, parent2Traits!.points),
  };

  return traits;
}

/**
 * Generate random traits for founder cats
 */
function generateRandomTraits(): GeneticTraits {
  const pelts = ['Tabby', 'Ticked', 'Mackerel', 'Classic', 'Sokoke', 'Speckled', 'Rosette', 'Smoke'];
  const colours = ['BLACK', 'WHITE', 'GINGER', 'GRAY', 'BROWN', 'CREAM', 'ORANGE', 'DARKGRAY'];
  const eyeColours = ['YELLOW', 'AMBER', 'HAZEL', 'GREEN', 'BLUE', 'DARKBLUE', 'GRAY'];
  const skinColours = ['BLACK', 'PINK', 'DARKBROWN', 'BROWN', 'LIGHTBROWN'];
  const tints = ['none', 'REDYELLOW', 'BLUE', 'PURPLE', 'GREEN'];

  return {
    peltName: pelts[Math.floor(Math.random() * pelts.length)],
    colour: colours[Math.floor(Math.random() * colours.length)],
    eyeColour: eyeColours[Math.floor(Math.random() * eyeColours.length)],
    skinColour: skinColours[Math.floor(Math.random() * skinColours.length)],
    isTortie: Math.random() < 0.3,
    tint: tints[Math.floor(Math.random() * tints.length)],
    whitePatches: Math.random() < 0.3 ? 'LITTLE' : undefined,
    points: Math.random() < 0.1 ? 'POINTED' : undefined,
  };
}

/**
 * Inherit from single parent with mutation chance
 */
function inheritWithMutation(parentTraits: GeneticTraits): GeneticTraits {
  const mutationChance = 0.1; // 10% chance to mutate each trait

  const pelts = ['Tabby', 'Ticked', 'Mackerel', 'Classic', 'Sokoke', 'Speckled', 'Rosette', 'Smoke'];
  const colours = ['BLACK', 'WHITE', 'GINGER', 'GRAY', 'BROWN', 'CREAM', 'ORANGE', 'DARKGRAY'];
  const eyeColours = ['YELLOW', 'AMBER', 'HAZEL', 'GREEN', 'BLUE', 'DARKBLUE', 'GRAY'];
  const skinColours = ['BLACK', 'PINK', 'DARKBROWN', 'BROWN', 'LIGHTBROWN'];
  const tints = ['none', 'REDYELLOW', 'BLUE', 'PURPLE', 'GREEN'];

  return {
    peltName: Math.random() < mutationChance
      ? pelts[Math.floor(Math.random() * pelts.length)]
      : parentTraits.peltName,
    colour: Math.random() < mutationChance
      ? colours[Math.floor(Math.random() * colours.length)]
      : parentTraits.colour,
    eyeColour: Math.random() < mutationChance
      ? eyeColours[Math.floor(Math.random() * eyeColours.length)]
      : parentTraits.eyeColour,
    skinColour: Math.random() < mutationChance
      ? skinColours[Math.floor(Math.random() * skinColours.length)]
      : parentTraits.skinColour,
    isTortie: Math.random() < mutationChance
      ? !parentTraits.isTortie
      : parentTraits.isTortie,
    tint: Math.random() < mutationChance
      ? tints[Math.floor(Math.random() * tints.length)]
      : parentTraits.tint,
    whitePatches: inheritWhitePatches(parentTraits.whitePatches, undefined),
    points: inheritPoints(parentTraits.points, undefined),
  };
}

/**
 * Determine tortie inheritance
 * Tortie is more likely if either parent is tortie
 */
function determineTortieInheritance(parent1IsTortie: boolean, parent2IsTortie: boolean): boolean {
  if (parent1IsTortie && parent2IsTortie) {
    return Math.random() < 0.7; // 70% chance if both parents
  }
  if (parent1IsTortie || parent2IsTortie) {
    return Math.random() < 0.4; // 40% chance if one parent
  }
  return Math.random() < 0.1; // 10% chance if neither parent
}

/**
 * Inherit white patches with variation
 * 40% inherit from one parent, 30% mutate, 30% none
 */
function inheritWhitePatches(
  parent1Patches: string | undefined,
  parent2Patches: string | undefined
): string | undefined {
  const roll = Math.random();
  
  if (roll < 0.4) {
    // Inherit from one parent
    const source = Math.random() < 0.5 ? parent1Patches : parent2Patches;
    return source;
  }
  
  if (roll < 0.7) {
    // Mutate - random white patch
    const patches = ['LITTLE', 'LIGHTTUXEDO', 'TUXEDO', 'FANCY', 'EXTRA', 'POINTMARK'];
    return patches[Math.floor(Math.random() * patches.length)];
  }
  
  // None
  return undefined;
}

/**
 * Inherit points (colorpoint pattern)
 */
function inheritPoints(parent1Points: string | undefined, parent2Points: string | undefined): string | undefined {
  const roll = Math.random();
  
  if (roll < 0.4) {
    // Inherit from one parent
    const source = Math.random() < 0.5 ? parent1Points : parent2Points;
    return source;
  }
  
  if (roll < 0.5) {
    // Mutate - random points
    const points = ['POINTED', 'MINK', 'SEPIA'];
    return points[Math.floor(Math.random() * points.length)];
  }
  
  // None
  return undefined;
}

/**
 * Convert genetic traits to sprite params
 */
export function traitsToSpriteParams(traits: GeneticTraits, baseParams?: Partial<CatSpriteParams>): CatSpriteParams {
  const validSprites = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18];
  const spriteNumber = baseParams?.spriteNumber ?? validSprites[Math.floor(Math.random() * validSprites.length)];

  const params: CatSpriteParams = {
    spriteNumber,
    peltName: traits.peltName,
    colour: traits.colour,
    tint: traits.tint,
    skinColour: traits.skinColour,
    eyeColour: traits.eyeColour,
    shading: baseParams?.shading ?? Math.random() < 0.5,
    reverse: baseParams?.reverse ?? Math.random() < 0.1,
    isTortie: traits.isTortie,
    accessories: baseParams?.accessories ?? [],
    scars: baseParams?.scars ?? [],
  };

  // Add heterochromia (5% chance)
  if (Math.random() < 0.05) {
    const eyeColours = ['YELLOW', 'AMBER', 'HAZEL', 'GREEN', 'BLUE', 'DARKBLUE', 'GRAY'];
    params.eyeColour2 = eyeColours[Math.floor(Math.random() * eyeColours.length)];
  }

  // Add white patches if inherited
  if (traits.whitePatches) {
    params.whitePatches = traits.whitePatches;
  }

  // Add points if inherited
  if (traits.points) {
    params.points = traits.points;
  }

  // Add tortie patterns if tortie
  if (traits.isTortie) {
    const colours = ['BLACK', 'WHITE', 'GINGER', 'GRAY', 'BROWN', 'CREAM', 'ORANGE'];
    const pelts = ['Tabby', 'Ticked', 'Mackerel', 'Classic', 'Sokoke'];
    params.tortie = [
      {
        mask: 'ONE',
        pattern: pelts[Math.floor(Math.random() * pelts.length)],
        colour: colours[Math.floor(Math.random() * colours.length)],
      },
    ];
    params.tortieMask = 'ONE';
    params.tortiePattern = params.tortie[0].pattern;
    params.tortieColour = params.tortie[0].colour;
  }

  return params;
}

