/**
 * Tint helpers for the PixiJS map spike.
 *
 * The DOM renderer tints reused biome sprites and dims fog with CSS `filter`
 * strings. Pixi has no CSS filters; a per-sprite `tint` (a multiply colour) is
 * the cheap GPU-batched equivalent. We faithfully reproduce the `brightness(n)`
 * component (biome darkening + fog dimming, which is what the eye reads as
 * shape-preserving fog) and approximate `saturate`/`hue-rotate` as identity —
 * an acceptable spike gap for a handful of biomes (jungle/swamp/enemy). A real
 * cutover would use a pooled `ColorMatrixFilter` per distinct filter string.
 */

/** Parse the `brightness(n)` factor out of a CSS filter string (default 1). */
export function brightnessOf(filter: string | undefined): number {
	if (!filter) {
		return 1;
	}
	const match = filter.match(/brightness\(\s*([0-9.]+)\s*\)/);
	return match ? Number.parseFloat(match[1]) : 1;
}

/**
 * A grey multiply-tint (0xRRGGBB, equal channels) for an overall brightness in
 * [0,1]. `sprite.tint = greyTint(b)` darkens the sprite to `b` of its value,
 * matching CSS `brightness(b)`.
 */
export function greyTint(brightness: number): number {
	const channel = Math.max(0, Math.min(255, Math.round(brightness * 255)));
	return (channel << 16) | (channel << 8) | channel;
}

/**
 * Combined tint for a tile sprite: its biome `filter` brightness times the fog
 * dim (1 when explored). Both are multiplicative, exactly as stacked CSS
 * filters multiply, so a fogged jungle reads as a dim jungle.
 */
export function tileTint(filter: string | undefined, fogDim: number): number {
	return greyTint(brightnessOf(filter) * fogDim);
}
