/**
 * Chunk tile store for the PixiJS map spike.
 *
 * Wraps `GET /api/game/chunks` with two things the DOM renderer lacked and that
 * caused its far-zoom "request storm" (§2.3 of docs/ENGINE_FRONTEND.md):
 *   - a cache with a TTL, so a chunk is fetched at most once per `ttlMs`, and
 *   - a bounded fetch queue, so panning to a wide view enqueues hundreds of
 *     chunks but only `maxConcurrent` are ever in flight, instead of firing all
 *     of them at once through the single better-sqlite3 route.
 *
 * Framework-agnostic (no React) so the Pixi scene can drive it directly.
 */

import { chunkKey } from "@/lib/game/mapView";
import type { WorldTile } from "@/types/game";

interface CacheEntry {
	tiles: WorldTile[];
	fetchedAt: number;
}

export interface ChunkStoreOptions {
	ttlMs?: number;
	maxConcurrent?: number;
	now?: () => number;
	fetcher?: (chunkX: number, chunkY: number) => Promise<WorldTile[]>;
	/** Called whenever a chunk's tiles arrive or refresh (key = "cx,cy"). */
	onLoaded?: (key: string, tiles: WorldTile[]) => void;
}

export class ChunkStore {
	private readonly cache = new Map<string, CacheEntry>();
	private readonly queue: Array<{ chunkX: number; chunkY: number }> = [];
	private readonly inFlight = new Set<string>();
	private active = 0;

	private readonly ttlMs: number;
	private readonly maxConcurrent: number;
	private readonly now: () => number;
	private readonly fetcher: (
		chunkX: number,
		chunkY: number,
	) => Promise<WorldTile[]>;
	private readonly onLoaded?: (key: string, tiles: WorldTile[]) => void;

	constructor(options: ChunkStoreOptions = {}) {
		this.ttlMs = options.ttlMs ?? 60_000;
		this.maxConcurrent = options.maxConcurrent ?? 6;
		this.now = options.now ?? Date.now;
		this.fetcher = options.fetcher ?? defaultFetchChunk;
		this.onLoaded = options.onLoaded;
	}

	/** Cached tiles for a chunk, or null if not loaded yet. */
	get(chunkX: number, chunkY: number): WorldTile[] | null {
		return this.cache.get(chunkKey({ chunkX, chunkY }))?.tiles ?? null;
	}

	/**
	 * Ensure the given chunks are loaded (or refreshed past the TTL). Stale/new
	 * chunks are enqueued; the queue drains at `maxConcurrent`. Chunks not in the
	 * request set are left alone — they stay cached for the next pan.
	 */
	ensure(coords: Array<{ chunkX: number; chunkY: number }>): void {
		const now = this.now();
		for (const coord of coords) {
			const key = chunkKey(coord);
			if (this.inFlight.has(key)) {
				continue;
			}
			const cached = this.cache.get(key);
			if (cached && now - cached.fetchedAt <= this.ttlMs) {
				continue;
			}
			if (this.queue.some((q) => chunkKey(q) === key)) {
				continue;
			}
			this.queue.push(coord);
		}
		this.pump();
	}

	private pump(): void {
		while (this.active < this.maxConcurrent && this.queue.length > 0) {
			const coord = this.queue.shift();
			if (!coord) {
				break;
			}
			void this.fetchChunk(coord);
		}
	}

	private async fetchChunk(coord: {
		chunkX: number;
		chunkY: number;
	}): Promise<void> {
		const key = chunkKey(coord);
		this.inFlight.add(key);
		this.active += 1;
		try {
			const tiles = await this.fetcher(coord.chunkX, coord.chunkY);
			this.cache.set(key, { tiles, fetchedAt: this.now() });
			this.onLoaded?.(key, tiles);
		} catch (err) {
			console.warn(`[pixi] chunk ${key} fetch failed:`, err);
		} finally {
			this.inFlight.delete(key);
			this.active -= 1;
			this.pump();
		}
	}

	stats(): {
		cached: number;
		queued: number;
		inFlight: number;
		active: number;
	} {
		return {
			cached: this.cache.size,
			queued: this.queue.length,
			inFlight: this.inFlight.size,
			active: this.active,
		};
	}
}

async function defaultFetchChunk(
	chunkX: number,
	chunkY: number,
): Promise<WorldTile[]> {
	const res = await fetch(`/api/game/chunks?x=${chunkX}&y=${chunkY}`);
	const data = res.ok ? await res.json() : null;
	return (data?.tiles ?? []) as WorldTile[];
}
