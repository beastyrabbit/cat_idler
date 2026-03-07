#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PORTLESS_PREFIX = "cat-idler-";
const PORTLESS_PROXY_PORT = 1355;
const MAX_DNS_LABEL_LENGTH = 63;
const HASH_LENGTH = 8;
const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPO_ROOT = resolve(dirname(SCRIPT_PATH), "..");

function sanitizeLabelPart(value) {
	return value
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/-+/g, "-")
		.replace(/^-|-$/g, "");
}

function getBranchName() {
	const result = spawnSync("git", ["branch", "--show-current"], {
		cwd: REPO_ROOT,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "ignore"],
	});

	if (result.status !== 0) {
		return "";
	}

	return result.stdout.trim();
}

function truncateWithHash(value, maxLength) {
	if (value.length <= maxLength) {
		return value;
	}

	const hash = createHash("sha1")
		.update(value)
		.digest("hex")
		.slice(0, HASH_LENGTH);
	const truncatedLength = maxLength - HASH_LENGTH - 1;
	const truncated = value
		.slice(0, Math.max(truncatedLength, 1))
		.replace(/-+$/g, "");

	return `${truncated}-${hash}`;
}

function spawnManagedProcess(command, args) {
	const child = spawn(command, args, {
		cwd: REPO_ROOT,
		stdio: "inherit",
		env: process.env,
	});

	child.on("exit", (code, signal) => {
		if (signal) {
			process.kill(process.pid, signal);
			return;
		}

		process.exit(code ?? 1);
	});

	child.on("error", (error) => {
		console.error(`Failed to start dev server: ${error.message}`);
		process.exit(1);
	});

	for (const signal of ["SIGINT", "SIGTERM"]) {
		process.on(signal, () => {
			if (!child.killed) {
				child.kill(signal);
			}
		});
	}

	return child;
}

export function getPortlessName() {
	const override = sanitizeLabelPart(process.env.PORTLESS_NAME ?? "");
	if (override) {
		return truncateWithHash(override, MAX_DNS_LABEL_LENGTH);
	}

	const branchName = sanitizeLabelPart(getBranchName());
	const worktreeName = sanitizeLabelPart(basename(REPO_ROOT));
	const variablePart = branchName || worktreeName || "dev";
	const maxVariableLength = MAX_DNS_LABEL_LENGTH - PORTLESS_PREFIX.length;

	return `${PORTLESS_PREFIX}${truncateWithHash(variablePart, maxVariableLength)}`;
}

export function getPortlessBaseUrl() {
	return `http://${getPortlessName()}.localhost:${PORTLESS_PROXY_PORT}`;
}

function ensurePortlessInstalled() {
	const result = spawnSync("portless", ["--version"], {
		cwd: REPO_ROOT,
		stdio: "ignore",
	});

	if (result.status === 0) {
		return;
	}

	console.error(
		"Portless is not available on PATH. Install or enable the global `portless` CLI before running this command.",
	);
	process.exit(1);
}

export function spawnPortlessNextDev(extraArgs = []) {
	if (process.env.PORTLESS === "skip") {
		console.log(
			"Portless disabled via PORTLESS=skip; starting raw Next.js dev on the default local port.",
		);
		return spawnManagedProcess("bunx", ["next", "dev", ...extraArgs]);
	}

	ensurePortlessInstalled();

	const portlessName = getPortlessName();
	const baseUrl = getPortlessBaseUrl();
	console.log(`Portless URL: ${baseUrl}`);

	return spawnManagedProcess("portless", [
		portlessName,
		"next",
		"dev",
		...extraArgs,
	]);
}

function runCli() {
	const [mode, ...args] = process.argv.slice(2);

	if (mode === "dev") {
		spawnPortlessNextDev(args);
		return;
	}

	if (mode === "url") {
		ensurePortlessInstalled();
		console.log(getPortlessBaseUrl());
		return;
	}

	console.error("Usage: node scripts/portless.mjs <dev|url>");
	process.exit(1);
}

if (import.meta.url === `file://${process.argv[1]}`) {
	runCli();
}
