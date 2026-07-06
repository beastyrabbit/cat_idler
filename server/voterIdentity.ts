import { createHash } from "node:crypto";

const DEFAULT_SUBSCRIBER_HASH_SALT = "catford-examiner-salt-2026";
const DEFAULT_TRUSTED_IP_HEADERS = [
	"cf-connecting-ip",
	"true-client-ip",
	"x-real-ip",
];

function headerValue(headers: Headers, name: string): string | null {
	const value = headers.get(name);
	const first = value?.split(",")[0]?.trim();
	return first || null;
}

export function clientIpFromHeaders(headers: Headers): string {
	const configuredHeader = process.env.TRUSTED_SUBSCRIBER_IP_HEADER?.trim();
	if (configuredHeader) {
		return headerValue(headers, configuredHeader) ?? "unknown";
	}

	for (const name of DEFAULT_TRUSTED_IP_HEADERS) {
		const ip = headerValue(headers, name);
		if (ip) {
			return ip;
		}
	}

	return "unknown";
}

export function subscriberHashForIp(ip: string): string {
	const salt = process.env.SUBSCRIBER_HASH_SALT ?? DEFAULT_SUBSCRIBER_HASH_SALT;
	return createHash("sha256")
		.update(ip)
		.update(salt)
		.digest("hex")
		.slice(0, 16);
}

export function subscriberHashFromHeaders(headers: Headers): string {
	return subscriberHashForIp(clientIpFromHeaders(headers));
}
