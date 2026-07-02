/**
 * Signed session identity (server-issued).
 *
 * Client-generated sessionIds are trivially forgeable, so any identity-bearing
 * action (votes, vote-kicks, zone limits, boosts, purchases, unlocks) could be
 * sybil'd by minting fresh sessionIds. To close that, the server signs each
 * sessionId with an HMAC and rejects actions whose signature doesn't verify.
 *
 * These are pure functions (crypto only, no I/O). Secret resolution and the
 * one-time dev-fallback warning live in the server layer (`server/players.ts`).
 */

import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";

/** HMAC-SHA256 of the sessionId, hex-encoded. */
export function signSession(sessionId: string, secret: string): string {
	return createHmac("sha256", secret).update(sessionId).digest("hex");
}

/**
 * Verify a signature against the expected HMAC using a constant-time compare.
 * Returns false for missing/malformed signatures rather than throwing.
 */
export function verifySession(
	sessionId: string,
	sig: string | null | undefined,
	secret: string,
): boolean {
	if (typeof sig !== "string" || sig.length === 0) {
		return false;
	}
	const expected = signSession(sessionId, secret);
	// timingSafeEqual requires equal-length buffers; a length mismatch is
	// already a definite non-match, so short-circuit before the compare.
	if (expected.length !== sig.length) {
		return false;
	}
	try {
		return timingSafeEqual(
			Buffer.from(expected, "hex"),
			Buffer.from(sig, "hex"),
		);
	} catch {
		// Non-hex signature — Buffer.from yields a shorter buffer and throws.
		return false;
	}
}

/** Mint a fresh, server-generated sessionId for brand-new clients. */
export function issueSessionId(): string {
	return `session_${randomUUID()}`;
}
