import { afterEach, describe, expect, it } from "vitest";

import {
	clientIpFromHeaders,
	subscriberHashFromHeaders,
	subscriberHashForIp,
} from "@/server/voterIdentity";

describe("voter identity subscriber hash", () => {
	const originalTrustedHeader = process.env.TRUSTED_SUBSCRIBER_IP_HEADER;

	afterEach(() => {
		if (originalTrustedHeader == null) {
			delete process.env.TRUSTED_SUBSCRIBER_IP_HEADER;
		} else {
			process.env.TRUSTED_SUBSCRIBER_IP_HEADER = originalTrustedHeader;
		}
	});

	it("does not trust client-controlled forwarded-for by default", () => {
		const headers = new Headers({ "x-forwarded-for": "203.0.113.10" });

		expect(clientIpFromHeaders(headers)).toBe("unknown");
		expect(subscriberHashFromHeaders(headers)).toBe(
			subscriberHashForIp("unknown"),
		);
	});

	it("uses trusted platform IP headers", () => {
		const headers = new Headers({
			"cf-connecting-ip": "203.0.113.11",
			"x-forwarded-for": "198.51.100.99",
		});

		expect(clientIpFromHeaders(headers)).toBe("203.0.113.11");
	});

	it("allows forwarded-for only when explicitly configured as trusted", () => {
		process.env.TRUSTED_SUBSCRIBER_IP_HEADER = "x-forwarded-for";
		const headers = new Headers({
			"x-forwarded-for": "203.0.113.12, 198.51.100.1",
		});

		expect(clientIpFromHeaders(headers)).toBe("203.0.113.12");
	});
});
