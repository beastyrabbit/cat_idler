#!/usr/bin/env node

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
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { getPortlessBaseUrl } from "../../scripts/portless.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = join(__dirname, "../..");
const isHeaded = process.env.HEADED === "true";
const baseUrl = process.env.TEST_BASE_URL || getPortlessBaseUrl();
const maxWait = 60000;

let devServer = null;
let serverStartedByUs = false;

async function isServerReady() {
	try {
		const response = await fetch(baseUrl);
		return response.ok;
	} catch {
		return false;
	}
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
	console.log("Starting Next.js dev server...");
	devServer = spawn("bun", ["run", "dev"], {
		cwd: repoRoot,
		stdio: "inherit",
	});
	serverStartedByUs = true;

	try {
		await waitForServerReady();
		await runTests();
	} catch (error) {
		console.error(`✗ ${error.message}`);
		stopDevServer();
		process.exit(1);
	}
}
