/**
 * In-memory sliding-window rate limiter.
 *
 * Cheap abuse guard for the actions route: each key (a player sessionId) may
 * perform at most `max` actions within `windowMs`. State is per-process and
 * intentionally ephemeral — this is a spam brake, not a security boundary.
 */

export class RateLimiter {
	private readonly hits = new Map<string, number[]>();

	constructor(
		private readonly max: number,
		private readonly windowMs: number,
	) {}

	/**
	 * Record an attempt for `key`. Returns true if allowed, false if the key
	 * has already hit `max` attempts inside the current window.
	 */
	check(key: string, now: number = Date.now()): boolean {
		const recent = (this.hits.get(key) ?? []).filter(
			(t) => now - t < this.windowMs,
		);
		if (recent.length >= this.max) {
			this.hits.set(key, recent);
			return false;
		}
		recent.push(now);
		this.hits.set(key, recent);
		return true;
	}

	/** Drop keys with no attempts left in the window (bounds memory). */
	prune(now: number = Date.now()): void {
		for (const [key, times] of this.hits) {
			const recent = times.filter((t) => now - t < this.windowMs);
			if (recent.length === 0) {
				this.hits.delete(key);
			} else {
				this.hits.set(key, recent);
			}
		}
	}
}
