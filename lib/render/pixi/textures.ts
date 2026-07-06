/**
 * Texture loading for the PixiJS map spike.
 *
 * All map art is the same Kenney "Isometric Miniature" PNG set the DOM renderer
 * uses (see `components/map/constants.ts`), loaded once into Pixi `Texture`s with
 * `nearest` scaling so the pixel art stays crisp at every zoom — the doc's key
 * reason to prefer Pixi over CSS, which blurs on scale.
 *
 * This module is browser-only (Pixi needs WebGL); it is imported behind an
 * `ssr:false` dynamic boundary so it never runs during Next's server render.
 */

import { Assets, type Texture } from "pixi.js";
import {
	BUILDING_SPRITE_FALLBACK,
	BUILDING_SPRITES,
	BRIDGE_SPRITES,
	FENCE_X_SPRITE,
	FENCE_Y_SPRITE,
	GATE_SPRITE,
	STUMP_SPRITE,
	TILE_SPRITES,
	WATER_SPRITE,
} from "@/components/map/constants";

/** The cat spritesheet (8 facings x 4 walk frames, 32x32 cells). */
export const CAT_SHEET_URL = "/images/cats/cat-sheet.png";

/** Every sprite URL the map can draw, de-duplicated. */
export function allSpriteUrls(): string[] {
	const urls = new Set<string>();
	for (const entry of Object.values(TILE_SPRITES)) {
		urls.add(entry.src);
		if (entry.base) {
			urls.add(entry.base);
		}
	}
	for (const src of Object.values(BUILDING_SPRITES)) {
		urls.add(src);
	}
	for (const src of Object.values(BRIDGE_SPRITES)) {
		urls.add(src);
	}
	urls.add(BUILDING_SPRITE_FALLBACK);
	urls.add(FENCE_X_SPRITE);
	urls.add(FENCE_Y_SPRITE);
	urls.add(GATE_SPRITE);
	urls.add(WATER_SPRITE);
	urls.add(STUMP_SPRITE);
	urls.add(CAT_SHEET_URL);
	return [...urls];
}

/**
 * Load every map texture and return a lookup by URL. Textures are set to
 * `nearest` scaling (crisp pixel art). Missing files resolve to a 1x1
 * transparent texture rather than throwing, so one absent sprite never blanks
 * the whole map during the spike.
 */
export async function loadMapTextures(): Promise<Map<string, Texture>> {
	const urls = allSpriteUrls();
	const byUrl = new Map<string, Texture>();
	const results = await Promise.allSettled(
		urls.map(async (url) => [url, await Assets.load<Texture>(url)] as const),
	);
	for (const result of results) {
		if (result.status === "fulfilled") {
			const [url, texture] = result.value;
			texture.source.scaleMode = "nearest";
			byUrl.set(url, texture);
		}
	}
	return byUrl;
}
