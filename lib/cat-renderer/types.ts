/**
 * Cat Renderer Types
 * 
 * Types for the external cat renderer API from beastyrabbit.com
 */

export interface CatRenderParams {
  spriteNumber: number;
  params: Record<string, unknown>;
  collectLayers?: boolean;
  includeLayerImages?: boolean;
}

export interface RendererResponse {
  image: string; // base64 data URL
  meta?: {
    started_at: number;
    finished_at: number;
    duration_ms: number;
    memory_pressure: boolean;
  };
}

export interface CatSpriteParams {
  spriteNumber: number;
  peltName: string;
  colour: string;
  tint: string;
  skinColour: string;
  eyeColour: string;
  eyeColour2?: string;
  shading?: boolean;
  reverse?: boolean;
  isTortie?: boolean;
  tortie?: Array<{
    mask: string;
    pattern: string;
    colour: string;
  }>;
  whitePatches?: string;
  whitePatchesTint?: string;
  points?: string;
  vitiligo?: string;
  accessories?: string[];
  scars?: string[];
  [key: string]: unknown;
}

