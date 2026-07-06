import { describe, expect, it } from "vitest";

import {
	HUNT_TRIP_COUNT,
	remainingYield,
	splitYield,
	tripDueAt,
} from "@/lib/game/trips";

describe("trips", () => {
	describe("splitYield", () => {
		it("splits an even total into equal shares", () => {
			expect(splitYield(30, 3, 0)).toBe(10);
			expect(splitYield(30, 3, 1)).toBe(10);
			expect(splitYield(30, 3, 2)).toBe(10);
		});

		it("distributes remainders to the earliest trips, summing exactly", () => {
			const shares = [0, 1, 2].map((i) => splitYield(31, 3, i));
			expect(shares).toEqual([11, 10, 10]);
			expect(shares.reduce((a, b) => a + b, 0)).toBe(31);

			const shares2 = [0, 1, 2].map((i) => splitYield(29, 3, i));
			expect(shares2.reduce((a, b) => a + b, 0)).toBe(29);
		});

		it("handles tripCount 1 (everything in one haul)", () => {
			expect(splitYield(17, 1, 0)).toBe(17);
		});

		it("handles zero and tiny totals", () => {
			expect(splitYield(0, 3, 0)).toBe(0);
			const tiny = [0, 1, 2].map((i) => splitYield(2, 3, i));
			expect(tiny.reduce((a, b) => a + b, 0)).toBe(2);
		});
	});

	describe("remainingYield", () => {
		it("is the total before any trips", () => {
			expect(remainingYield(30, 3, 0)).toBe(30);
		});

		it("subtracts exactly the shares already hauled", () => {
			expect(remainingYield(31, 3, 1)).toBe(31 - 11);
			expect(remainingYield(31, 3, 2)).toBe(31 - 11 - 10);
		});

		it("never goes negative past the last trip", () => {
			expect(remainingYield(30, 3, 3)).toBe(0);
			expect(remainingYield(30, 3, 5)).toBe(0);
		});
	});

	describe("tripDueAt", () => {
		const startedAt = 1_000_000;
		const endsAt = startedAt + 9_000;

		it("spaces mid-trips evenly across the job duration", () => {
			expect(tripDueAt(startedAt, endsAt, 1, 3)).toBe(startedAt + 3_000);
			expect(tripDueAt(startedAt, endsAt, 2, 3)).toBe(startedAt + 6_000);
		});

		it("handles durations that do not divide evenly", () => {
			const due = tripDueAt(startedAt, startedAt + 10_000, 1, 3);
			expect(due).toBeGreaterThan(startedAt);
			expect(due).toBeLessThan(startedAt + 10_000);
		});

		it("defaults to the standard trip count", () => {
			expect(tripDueAt(startedAt, endsAt, 1)).toBe(
				tripDueAt(startedAt, endsAt, 1, HUNT_TRIP_COUNT),
			);
		});
	});
});
