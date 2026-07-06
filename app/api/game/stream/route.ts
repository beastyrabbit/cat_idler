/**
 * Real-time dashboard stream (SSE).
 *
 * Pushes the full dashboard payload once per second — the worker ticks the
 * simulation at the same cadence, so every frame reflects a fresh tick.
 * The worker runs in a separate process, so updates are read from SQLite
 * rather than an in-memory bus.
 *
 * On a dashboard read failure the stream sends an `error` event and
 * closes; the client's EventSource reconnects and the hook surfaces
 * staleness via `connectionLost`.
 */

import { getDb } from "@/db/client";
import { ensureGlobalState, getGlobalDashboard } from "@/server/game";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const PUSH_INTERVAL_MS = 1000;

export async function GET(request: Request) {
	let db: ReturnType<typeof getDb>;
	try {
		db = getDb();
		ensureGlobalState(db); // transactional bootstrap
	} catch (err) {
		console.error("[stream] bootstrap failed:", err);
		return new Response("Game backend unavailable", { status: 503 });
	}

	const encoder = new TextEncoder();

	const stream = new ReadableStream({
		start(controller) {
			let closed = false;

			const close = () => {
				if (closed) return;
				closed = true;
				clearInterval(interval);
				try {
					controller.close();
				} catch {
					// already closed by the runtime
				}
			};

			const send = () => {
				if (closed) return;
				try {
					const dashboard = getGlobalDashboard(db);
					controller.enqueue(
						encoder.encode(
							`event: dashboard\ndata: ${JSON.stringify(dashboard)}\n\n`,
						),
					);
				} catch (err) {
					if (closed) {
						// enqueue raced a client disconnect — not a data failure
						return;
					}
					console.error("[stream] dashboard push failed:", err);
					try {
						controller.enqueue(
							encoder.encode(`event: error\ndata: "dashboard_failed"\n\n`),
						);
					} catch {
						// stream already torn down
					}
					close();
				}
			};

			const interval = setInterval(send, PUSH_INTERVAL_MS);
			send();

			request.signal.addEventListener("abort", close);
		},
	});

	return new Response(stream, {
		headers: {
			"Content-Type": "text/event-stream",
			"Cache-Control": "no-cache, no-transform",
			Connection: "keep-alive",
		},
	});
}
