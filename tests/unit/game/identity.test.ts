import { describe, expect, it } from "vitest";

import {
	issueSessionId,
	signSession,
	verifySession,
} from "@/lib/game/identity";

const SECRET = "test-secret-abc";

describe("signSession", () => {
	it("is deterministic for the same session + secret", () => {
		expect(signSession("session_a", SECRET)).toBe(
			signSession("session_a", SECRET),
		);
	});

	it("produces a different signature for a different session", () => {
		expect(signSession("session_a", SECRET)).not.toBe(
			signSession("session_b", SECRET),
		);
	});

	it("produces a different signature under a different secret", () => {
		expect(signSession("session_a", SECRET)).not.toBe(
			signSession("session_a", "other-secret"),
		);
	});
});

describe("verifySession", () => {
	it("accepts a signature the server issued", () => {
		const sig = signSession("session_a", SECRET);
		expect(verifySession("session_a", sig, SECRET)).toBe(true);
	});

	it("rejects a signature for a tampered sessionId", () => {
		// Attacker keeps a valid signature but swaps the sessionId (sybil attempt).
		const sig = signSession("session_a", SECRET);
		expect(verifySession("session_forged", sig, SECRET)).toBe(false);
	});

	it("rejects a forged/garbage signature", () => {
		expect(verifySession("session_a", "deadbeef", SECRET)).toBe(false);
		expect(verifySession("session_a", "not-hex-!!", SECRET)).toBe(false);
	});

	it("rejects a missing signature", () => {
		expect(verifySession("session_a", null, SECRET)).toBe(false);
		expect(verifySession("session_a", undefined, SECRET)).toBe(false);
		expect(verifySession("session_a", "", SECRET)).toBe(false);
	});

	it("rejects a signature made under a different secret", () => {
		const sig = signSession("session_a", "rotated-secret");
		expect(verifySession("session_a", sig, SECRET)).toBe(false);
	});

	it("upgrades a legacy unsigned session once it is signed", () => {
		// A pre-HMAC client stored only a sessionId (no signature) — it fails
		// verification until presence signs it, after which it verifies.
		const legacy = "session_legacy_localstorage";
		expect(verifySession(legacy, null, SECRET)).toBe(false);
		const upgraded = signSession(legacy, SECRET);
		expect(verifySession(legacy, upgraded, SECRET)).toBe(true);
	});
});

describe("issueSessionId", () => {
	it("mints unique, prefixed session ids", () => {
		const a = issueSessionId();
		const b = issueSessionId();
		expect(a).toMatch(/^session_/);
		expect(a).not.toBe(b);
	});
});
