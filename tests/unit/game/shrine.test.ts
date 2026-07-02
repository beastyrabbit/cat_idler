import { describe, expect, it } from "vitest";

import {
	type Carrying,
	DEPOSIT_GRACE_MS,
	DEPOSIT_RADIUS,
	isAtShrine,
	shouldDeposit,
	shouldForceDeposit,
} from "@/lib/game/shrine";

const SHRINE = { x: 6, y: 6 };

function carrying(jobEndedAt: number): Carrying {
	return { kind: "food", amount: 10, jobEndedAt };
}

describe("shrine deposits", () => {
	describe("isAtShrine", () => {
		it("accepts the shrine tile and its ring", () => {
			expect(isAtShrine({ x: 6, y: 6 }, SHRINE)).toBe(true);
			expect(isAtShrine({ x: 6 + DEPOSIT_RADIUS, y: 6 }, SHRINE)).toBe(true);
			expect(isAtShrine({ x: 5, y: 7 }, SHRINE)).toBe(true);
		});

		it("rejects tiles beyond the deposit radius", () => {
			expect(isAtShrine({ x: 6 + DEPOSIT_RADIUS + 1, y: 6 }, SHRINE)).toBe(
				false,
			);
			expect(isAtShrine({ x: 20, y: 20 }, SHRINE)).toBe(false);
		});
	});

	describe("shouldForceDeposit", () => {
		it("triggers exactly at the grace boundary", () => {
			const c = carrying(1_000);
			expect(shouldForceDeposit(c, 1_000 + DEPOSIT_GRACE_MS - 1)).toBe(false);
			expect(shouldForceDeposit(c, 1_000 + DEPOSIT_GRACE_MS)).toBe(true);
		});
	});

	describe("shouldDeposit", () => {
		it("deposits on arrival even within the grace window", () => {
			expect(
				shouldDeposit(carrying(1_000), { x: 6, y: 5 }, SHRINE, 2_000),
			).toBe(true);
		});

		it("waits while traveling inside the grace window", () => {
			expect(
				shouldDeposit(carrying(1_000), { x: 15, y: 6 }, SHRINE, 2_000),
			).toBe(false);
		});

		it("force-credits a straggler after the grace window", () => {
			expect(
				shouldDeposit(
					carrying(1_000),
					{ x: 15, y: 6 },
					SHRINE,
					1_000 + DEPOSIT_GRACE_MS,
				),
			).toBe(true);
		});
	});
});
