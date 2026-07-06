export type LodBand = "close" | "overview";

/** Below this viewport scale the scene switches to per-chunk overview quads. */
export const LOD_THRESHOLD = 0.2;

export function lodBandForScale(scale: number): LodBand {
	return scale < LOD_THRESHOLD ? "overview" : "close";
}
