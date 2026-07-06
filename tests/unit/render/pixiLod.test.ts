import { describe, expect, it } from "vitest";

import { LOD_THRESHOLD, lodBandForScale } from "@/lib/render/pixi/lod";

describe("pixi LOD", () => {
	it("keeps the threshold inclusive for close rendering", () => {
		expect(lodBandForScale(LOD_THRESHOLD)).toBe("close");
		expect(lodBandForScale(LOD_THRESHOLD + 0.001)).toBe("close");
	});

	it("switches below the threshold to chunk overview rendering", () => {
		expect(lodBandForScale(LOD_THRESHOLD - 0.001)).toBe("overview");
		expect(lodBandForScale(0.03)).toBe("overview");
	});
});
