import { describe, expect, it } from "vitest";

import { RateLimiter } from "@/lib/game/rateLimiter";

describe("RateLimiter", () => {
	it("allows up to max attempts then blocks", () => {
		const limiter = new RateLimiter(3, 1000);
		expect(limiter.check("a", 0)).toBe(true);
		expect(limiter.check("a", 1)).toBe(true);
		expect(limiter.check("a", 2)).toBe(true);
		expect(limiter.check("a", 3)).toBe(false);
	});

	it("tracks keys independently", () => {
		const limiter = new RateLimiter(1, 1000);
		expect(limiter.check("a", 0)).toBe(true);
		expect(limiter.check("b", 0)).toBe(true);
		expect(limiter.check("a", 0)).toBe(false);
	});

	it("slides the window: old attempts expire", () => {
		const limiter = new RateLimiter(2, 1000);
		expect(limiter.check("a", 0)).toBe(true);
		expect(limiter.check("a", 500)).toBe(true);
		expect(limiter.check("a", 600)).toBe(false);
		// Once the first two attempts age out of the window, budget frees up.
		expect(limiter.check("a", 1600)).toBe(true);
	});

	it("prunes keys with no attempts left in the window", () => {
		const limiter = new RateLimiter(2, 1000);
		limiter.check("a", 0);
		limiter.prune(5000);
		// After pruning, the key starts fresh (full budget available).
		expect(limiter.check("a", 5001)).toBe(true);
		expect(limiter.check("a", 5002)).toBe(true);
		expect(limiter.check("a", 5003)).toBe(false);
	});
});
