import { describe, expect, it } from "vitest";

import { ChunkStore } from "@/lib/render/pixi/chunkStore";
import type { WorldTile } from "@/types/game";

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((res) => {
		resolve = res;
	});
	return { promise, resolve };
}

const emptyTiles: WorldTile[] = [];

describe("ChunkStore", () => {
	it("bounds concurrent chunk fetches and drains queued work", async () => {
		const pending = [
			deferred<WorldTile[]>(),
			deferred<WorldTile[]>(),
			deferred<WorldTile[]>(),
		];
		let calls = 0;
		const loaded: string[] = [];
		const store = new ChunkStore({
			maxConcurrent: 2,
			fetcher: () => pending[calls++].promise,
			onLoaded: (key) => loaded.push(key),
		});

		store.ensure([
			{ chunkX: 0, chunkY: 0 },
			{ chunkX: 1, chunkY: 0 },
			{ chunkX: 2, chunkY: 0 },
		]);

		expect(calls).toBe(2);
		expect(store.stats()).toMatchObject({ active: 2, queued: 1 });

		pending[0].resolve(emptyTiles);
		await Promise.resolve();
		await Promise.resolve();

		expect(loaded).toEqual(["0,0"]);
		expect(calls).toBe(3);
		expect(store.stats()).toMatchObject({ active: 2, queued: 0 });

		pending[1].resolve(emptyTiles);
		pending[2].resolve(emptyTiles);
		await Promise.resolve();
		await Promise.resolve();

		expect(loaded).toEqual(["0,0", "1,0", "2,0"]);
		expect(store.stats()).toMatchObject({ active: 0, queued: 0, cached: 3 });
	});

	it("serves fresh cache entries and refreshes stale ones", async () => {
		let now = 1_000;
		let calls = 0;
		const store = new ChunkStore({
			ttlMs: 100,
			now: () => now,
			fetcher: async () => {
				calls += 1;
				return emptyTiles;
			},
		});

		store.ensure([{ chunkX: 0, chunkY: 0 }]);
		await Promise.resolve();
		await Promise.resolve();
		expect(calls).toBe(1);

		now = 1_050;
		store.ensure([{ chunkX: 0, chunkY: 0 }]);
		await Promise.resolve();
		expect(calls).toBe(1);

		now = 1_101;
		store.ensure([{ chunkX: 0, chunkY: 0 }]);
		await Promise.resolve();
		await Promise.resolve();
		expect(calls).toBe(2);
	});
});
