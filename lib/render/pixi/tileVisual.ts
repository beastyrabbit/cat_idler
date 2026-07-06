import { ISO } from "@/components/map/constants";

export * from "@/lib/render/mapTileVisual";

/** Sprite draw box: full 256x512 canvas at (left, top - surfaceOffset). */
export const SPRITE_W = ISO.tileWidth;
export const SPRITE_H = ISO.imageHeight;
export const SPRITE_TOP_OFFSET = ISO.surfaceOffset;
