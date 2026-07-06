import { getDb } from "../db/client";
import { ensureGlobalState, workerTick } from "../server/game";

const tickMs = Number(process.env.WORKER_TICK_MS ?? 1000);
// A broken DB (corruption, disk full, schema drift) fails every tick; exit
// after this many consecutive failures so a supervisor restart is visible
// instead of a silent spin.
const MAX_CONSECUTIVE_FAILURES = 30;
const db = getDb();

let running = false;
let consecutiveFailures = 0;

function runTick() {
	if (running) {
		return;
	}

	running = true;
	const start = Date.now();
	try {
		workerTick(db);
		consecutiveFailures = 0;
	} catch (error) {
		consecutiveFailures += 1;
		console.error("[worker] tick failed:", error);
		if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
			console.error(
				`[worker] ${consecutiveFailures} consecutive tick failures — exiting`,
			);
			process.exit(1);
		}
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
