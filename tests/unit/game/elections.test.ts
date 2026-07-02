import { describe, expect, it } from "vitest";

import {
	candidatesFor,
	electionDue,
	electionWinner,
	KICK_THRESHOLD,
	shouldTriggerKick,
	tallyVotes,
} from "@/lib/game/elections";

function cat(id: string, leadership: number) {
	return { _id: id, leadership };
}

describe("elections", () => {
	describe("candidatesFor", () => {
		it("picks the top 5 by leadership", () => {
			const cats = [
				cat("a", 10),
				cat("b", 90),
				cat("c", 50),
				cat("d", 70),
				cat("e", 30),
				cat("f", 80),
				cat("g", 60),
			];
			expect(candidatesFor(cats)).toEqual(["b", "f", "d", "g", "c"]);
		});

		it("excludes barred cats", () => {
			const cats = [cat("a", 90), cat("b", 80), cat("c", 70)];
			expect(candidatesFor(cats, new Set(["a"]))).toEqual(["b", "c"]);
		});

		it("handles fewer than 5 cats", () => {
			expect(candidatesFor([cat("a", 10)])).toEqual(["a"]);
			expect(candidatesFor([])).toEqual([]);
		});
	});

	describe("tallyVotes", () => {
		it("counts votes per candidate", () => {
			const tally = tallyVotes([
				{ playerId: "p1", catId: "a" },
				{ playerId: "p2", catId: "a" },
				{ playerId: "p3", catId: "b" },
			]);
			expect(tally).toEqual({ a: 2, b: 1 });
		});

		it("collapses duplicate players to their latest vote position", () => {
			const tally = tallyVotes([
				{ playerId: "p1", catId: "a" },
				{ playerId: "p1", catId: "b" },
			]);
			expect(tally).toEqual({ b: 1 });
		});
	});

	describe("electionWinner", () => {
		const candidates = [cat("a", 40), cat("b", 90), cat("c", 60)];

		it("picks the candidate with the most votes", () => {
			expect(electionWinner(candidates, { a: 3, b: 1, c: 2 })).toBe("a");
		});

		it("breaks ties by higher leadership", () => {
			expect(electionWinner(candidates, { a: 2, c: 2 })).toBe("c");
		});

		it("falls back to highest leadership with zero votes", () => {
			expect(electionWinner(candidates, {})).toBe("b");
		});

		it("ignores votes for non-candidates", () => {
			expect(electionWinner(candidates, { z: 10, a: 1 })).toBe("a");
		});

		it("returns null with no candidates", () => {
			expect(electionWinner([], { a: 1 })).toBeNull();
		});
	});

	describe("shouldTriggerKick", () => {
		it("requires the full threshold of distinct voters", () => {
			const four = Array.from({ length: KICK_THRESHOLD - 1 }, (_, i) => ({
				playerId: `p${i}`,
				catId: "leader",
			}));
			expect(shouldTriggerKick(four)).toBe(false);

			const five = Array.from({ length: KICK_THRESHOLD }, (_, i) => ({
				playerId: `p${i}`,
				catId: "leader",
			}));
			expect(shouldTriggerKick(five)).toBe(true);
		});

		it("does not count the same player twice", () => {
			const stuffed = Array.from({ length: KICK_THRESHOLD + 3 }, () => ({
				playerId: "same",
				catId: "leader",
			}));
			expect(shouldTriggerKick(stuffed)).toBe(false);
		});
	});

	describe("electionDue", () => {
		const TERM = 24 * 3600 * 1000;

		it("is due when there has never been an election", () => {
			expect(electionDue(null, 1_000_000, TERM)).toBe(true);
		});

		it("is due once the term expires", () => {
			expect(electionDue(1_000, 1_000 + TERM, TERM)).toBe(true);
			expect(electionDue(1_000, 1_000 + TERM - 1, TERM)).toBe(false);
		});
	});
});
