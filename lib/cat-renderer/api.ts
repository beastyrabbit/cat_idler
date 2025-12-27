/**
 * Cat Renderer API Client
 * 
 * Client for the external cat renderer service at web.beastyrabbit.com
 */

import type { CatRenderParams, RendererResponse, CatSpriteParams } from './types';

const RENDERER_BASE_URL = 'https://web.beastyrabbit.com/api/renderer';

/**
 * Render a cat sprite using the external renderer service
 */
export async function renderCat(params: CatSpriteParams): Promise<string> {
  // Extract spriteNumber from params
  const spriteNumber = params.spriteNumber ?? 3; // Default sprite
  
  // Build render payload
  const renderParams: CatRenderParams = {
    spriteNumber,
    params: {
      ...params,
      spriteNumber: undefined, // Remove from params, it's separate
    },
  };

  try {
    const response = await fetch(RENDERER_BASE_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        payload: {
          spriteNumber: renderParams.spriteNumber,
          params: renderParams.params,
        },
      }),
    });

    if (!response.ok) {
      const errorText = await response.text().catch(() => 'Unknown error');
      throw new Error(`Renderer request failed (${response.status}): ${errorText}`);
    }

    const data: RendererResponse = await response.json();
    return data.image;
  } catch (error) {
    console.error('Failed to render cat:', error);
    throw error;
  }
}

/**
 * Generate random sprite parameters for a new cat
 * This is a simplified version - in production you'd want to use the full random generator
 */
export function generateRandomSpriteParams(): CatSpriteParams {
  // Default sprite pool (valid sprites from the renderer)
  const validSprites = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18];
  const spriteNumber = validSprites[Math.floor(Math.random() * validSprites.length)];

  // Basic color options (simplified - full version would use sprite mapper)
  const colours = [
    'BLACK', 'WHITE', 'GINGER', 'GRAY', 'BROWN', 'CREAM', 'ORANGE',
    'DARKGRAY', 'LIGHTGRAY', 'DARKBROWN', 'LIGHTBROWN', 'PALEORANGE',
  ];
  const pelts = [
    'Tortie', 'Calico', 'Tabby', 'Ticked', 'Mackerel', 'Classic', 'Sokoke',
    'Speckled', 'Rosette', 'Smoke', 'Singlestripe', 'Mask', 'Skele',
  ];
  const eyeColours = [
    'YELLOW', 'AMBER', 'HAZEL', 'PALEGREEN', 'GREEN', 'BLUE', 'DARKBLUE',
    'PALEBLUE', 'RED', 'PINK', 'DARKPINK', 'PALEPINK', 'GRAY', 'CYAN',
  ];
  const tints = ['none', 'REDYELLOW', 'BLUE', 'PURPLE', 'GREEN', 'PINK', 'RAINBOW'];
  const skinColours = ['BLACK', 'PINK', 'DARKBROWN', 'BROWN', 'LIGHTBROWN', 'DARK', 'DARK2'];

  const isTortie = Math.random() < 0.3; // 30% chance of tortie

  const params: CatSpriteParams = {
    spriteNumber,
    peltName: pelts[Math.floor(Math.random() * pelts.length)],
    colour: colours[Math.floor(Math.random() * colours.length)],
    tint: tints[Math.floor(Math.random() * tints.length)],
    skinColour: skinColours[Math.floor(Math.random() * skinColours.length)],
    eyeColour: eyeColours[Math.floor(Math.random() * eyeColours.length)],
    shading: Math.random() < 0.5,
    reverse: Math.random() < 0.1,
    isTortie,
    accessories: [],
    scars: [],
  };

  // Add heterochromia (5% chance)
  if (Math.random() < 0.05) {
    params.eyeColour2 = eyeColours[Math.floor(Math.random() * eyeColours.length)];
  }

  // Add white patches (30% chance)
  if (Math.random() < 0.3) {
    params.whitePatches = 'LITTLE';
  }

  // Add tortie patterns if tortie
  if (isTortie) {
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

