import { getDb } from "../db/client";
import { ensureGlobalState, workerTick } from "../server/game";

const tickMs = Number(process.env.WORKER_TICK_MS ?? 1000);
const db = getDb();

let running = false;

function runTick() {
	if (running) {
		return;
	}

	running = true;
	const start = Date.now();
	try {
		workerTick(db);
	} catch (error) {
		console.error("[worker] tick failed:", error);
	} finally {
		const duration = Date.now() - start;
		if (duration > tickMs) {
			console.warn(
				`[worker] tick took ${duration}ms (exceeds ${tickMs}ms interval)`,
			);
		}
		running = false;
	}
}

function main() {
	console.log("[worker] starting");
	console.log(`[worker] db: ${process.env.GAME_DB_PATH ?? "data/game.db"}`);
	console.log(`[worker] tick every ${tickMs}ms`);

	ensureGlobalState(db);
	runTick();

	const interval = setInterval(runTick, tickMs);

	const shutdown = () => {
		clearInterval(interval);
		process.exit(0);
	};

	process.on("SIGINT", shutdown);
	process.on("SIGTERM", shutdown);
}

main();
