#!/usr/bin/env bun

/**
 * Selenium E2E Test Runner
 *
 * Runs all E2E tests using Selenium WebDriver.
 *
 * Usage:
 *   bun run test:e2e          # Headless
 *   bun run test:e2e:headed   # With browser visible
 */

import { spawn } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  realpathSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { getPortlessBaseUrl } from "../../scripts/portless.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = realpathSync(join(__dirname, "../.."));
const isHeaded = process.env.HEADED === "true";
const configuredBaseUrl = process.env.TEST_BASE_URL || "";
let baseUrl = normalizeBaseUrl(configuredBaseUrl || getPortlessBaseUrl());
const maxWait = 60000;

let devServer = null;
let serverStartedByUs = false;

function normalizeBaseUrl(url) {
  const parsed = new URL(url);
  if (
    !configuredBaseUrl &&
    parsed.protocol === "http:" &&
    parsed.hostname.endsWith(".localhost") &&
    parsed.port === "1355"
  ) {
    parsed.protocol = "https:";
  }
  return parsed.toString().replace(/\/$/, "");
}

function dashboardUrl(url) {
  return new URL("/api/game/dashboard", url).toString();
}

async function isServerReady(url = baseUrl) {
  try {
    const response = await fetch(dashboardUrl(url));
    if (!response.ok) {
      return false;
    }
    const body = await response.json().catch(() => null);
    return Boolean(body?.colony && Array.isArray(body?.cats));
  } catch {
    return false;
  }
}

function readProcParts(pid, file) {
  try {
    return readFileSync(`/proc/${pid}/${file}`, "utf8")
      .split("\0")
      .filter(Boolean);
  } catch {
    return [];
  }
}

function nextDevPortsForThisRepo() {
  if (!existsSync("/proc")) {
    return [];
  }

  const ports = new Set();
  for (const entry of readdirSync("/proc", { withFileTypes: true })) {
    if (!entry.isDirectory() || !/^\d+$/.test(entry.name)) {
      continue;
    }

    let cwd = "";
    try {
      cwd = realpathSync(readlinkSync(`/proc/${entry.name}/cwd`));
    } catch {
      continue;
    }
    if (cwd !== repoRoot) {
      continue;
    }

    const argv = readProcParts(entry.name, "cmdline");
    if (!argv.some((part) => part.includes("next")) || !argv.includes("dev")) {
      continue;
    }

    const portArgIndex = argv.findIndex(
      (part) => part === "-p" || part === "--port",
    );
    const inlinePort = argv
      .find((part) => part.startsWith("--port="))
      ?.slice("--port=".length);
    const port =
      inlinePort ??
      (portArgIndex >= 0 ? argv[portArgIndex + 1] : undefined) ??
      readProcParts(entry.name, "environ")
        .find((part) => part.startsWith("PORT="))
        ?.slice("PORT=".length) ??
      "3000";

    if (/^\d+$/.test(port)) {
      ports.add(Number(port));
    }
  }

  return [...ports].sort((a, b) => a - b);
}

async function findExistingLocalServer() {
  if (configuredBaseUrl) {
    return null;
  }

  for (const port of nextDevPortsForThisRepo()) {
    const candidate = `http://localhost:${port}`;
    if (await isServerReady(candidate)) {
      return candidate;
    }
  }

  return null;
}

function stopDevServer(signal = "SIGTERM") {
  if (!serverStartedByUs || !devServer || devServer.killed) {
    return;
  }

  devServer.kill(signal);
}

async function waitForServerReady() {
  const startedAt = Date.now();

  while (Date.now() - startedAt < maxWait) {
    if (await isServerReady()) {
      console.log(`✓ Dev server is ready at ${baseUrl}`);
      return;
    }

    await new Promise((resolve) => {
      setTimeout(resolve, 1000);
    });
  }

  throw new Error(`Dev server failed to start at ${baseUrl}`);
}

async function runTests() {
  console.log("\nRunning E2E tests with Selenium...\n");

  const testFiles = [
    "./colony-lifecycle.spec.js",
    "./user-interactions.spec.js",
    "./building-placement.spec.js",
    "./resource-bars.spec.js",
    "./navigation.spec.js",
  ];

  let passed = 0;
  let failed = 0;

  process.env.TEST_BASE_URL = baseUrl;

  for (const testFile of testFiles) {
    try {
      const { default: testFn } = await import(testFile);
      await testFn(isHeaded, baseUrl);
      passed++;
      console.log(`✓ ${testFile} passed\n`);
    } catch (error) {
      failed++;
      console.error(`✗ ${testFile} failed:`, error.message);
      if (error.stack) {
        console.error(error.stack);
      }
    }
  }

  console.log(`\n${"=".repeat(50)}`);
  console.log(`Tests: ${passed} passed, ${failed} failed`);
  console.log(`${"=".repeat(50)}\n`);

  stopDevServer();
  process.exit(failed > 0 ? 1 : 0);
}

process.on("SIGINT", () => {
  console.log("\nStopping dev server...");
  stopDevServer("SIGINT");
  process.exit(130);
});

process.on("SIGTERM", () => {
  stopDevServer("SIGTERM");
  process.exit(143);
});

if (await isServerReady()) {
  console.log(`✓ Using existing dev server at ${baseUrl}`);
  await runTests();
} else {
  const existingServer = await findExistingLocalServer();
  if (existingServer) {
    baseUrl = existingServer;
    console.log(`✓ Using existing dev server at ${baseUrl}`);
    await runTests();
  }

  console.log("Starting Next.js dev server...");
  devServer = spawn("bun", ["run", "dev"], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  serverStartedByUs = true;
  let rejectStartupExit;
  const startupExit = new Promise((_, reject) => {
    rejectStartupExit = reject;
  });
  const handleStartupExit = (code, signal) => {
    rejectStartupExit(
      new Error(
        `Dev server exited unexpectedly before becoming ready (${signal ? `signal ${signal}` : `code ${code ?? 1}`})`,
      ),
    );
  };
  devServer.once("exit", handleStartupExit);

  try {
    await Promise.race([waitForServerReady(), startupExit]);
    devServer.off("exit", handleStartupExit);
    await runTests();
  } catch (error) {
    console.error(`✗ ${error.message}`);
    stopDevServer();
    process.exit(1);
  }
}
