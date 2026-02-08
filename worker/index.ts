import { ConvexHttpClient } from "convex/browser";

import { api } from "../convex/_generated/api";

const convexUrl = process.env.CONVEX_URL || process.env.NEXT_PUBLIC_CONVEX_URL;
const tickMs = Number(process.env.WORKER_TICK_MS ?? 1000);

if (!convexUrl) {
  throw new Error(
    "Set CONVEX_URL or NEXT_PUBLIC_CONVEX_URL before running worker.",
  );
}

const client = new ConvexHttpClient(convexUrl);
const anyApi = api as any;

let running = false;

async function runTick() {
  if (running) {
    return;
  }

  running = true;
  const start = Date.now();
  try {
    await client.mutation(anyApi.game.workerTick, {});
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

async function main() {
  console.log("[worker] starting");
  console.log(`[worker] convex: ${convexUrl}`);
  console.log(`[worker] tick every ${tickMs}ms`);

  await client.mutation(anyApi.game.ensureGlobalState, {});
  await runTick();

  const interval = setInterval(() => {
    void runTick();
  }, tickMs);

  const shutdown = () => {
    clearInterval(interval);
    process.exit(0);
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
}

void main();
