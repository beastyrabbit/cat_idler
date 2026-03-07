import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { ConvexHttpClient } from "convex/browser";
import { By } from "selenium-webdriver";

import { api } from "../../convex/_generated/api.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, "../..");
const gameReadyMarkers = [
	"Shared Idle World",
	"Global Colony Not Ready",
	"Preparing Global Colony...",
];
const loadedGameMarkers = [
	"Shared Idle World",
	"Food",
	"Water",
	"Request Hunt (plan + expedition)",
	"Ritual Points",
];

let loadedEnv = false;
let convexClient = null;

function parseEnvFile(content) {
	for (const rawLine of content.split("\n")) {
		const line = rawLine.trim();
		if (!line || line.startsWith("#")) {
			continue;
		}

		const separator = line.indexOf("=");
		if (separator === -1) {
			continue;
		}

		const key = line.slice(0, separator).trim();
		if (!key || process.env[key]) {
			continue;
		}

		const rawValue = line.slice(separator + 1).trim();
		const quoted =
			(rawValue.startsWith('"') && rawValue.endsWith('"')) ||
			(rawValue.startsWith("'") && rawValue.endsWith("'"));
		process.env[key] = quoted ? rawValue.slice(1, -1) : rawValue;
	}
}

function loadLocalEnv() {
	if (loadedEnv) {
		return;
	}

	for (const fileName of [".env.local", ".env"]) {
		const filePath = resolve(repoRoot, fileName);
		if (existsSync(filePath)) {
			parseEnvFile(readFileSync(filePath, "utf8"));
		}
	}

	loadedEnv = true;
}

function getConvexUrl() {
	loadLocalEnv();

	const convexUrl =
		process.env.CONVEX_URL ?? process.env.NEXT_PUBLIC_CONVEX_URL;
	if (!convexUrl) {
		throw new Error(
			"Missing CONVEX_URL or NEXT_PUBLIC_CONVEX_URL for E2E setup.",
		);
	}

	return convexUrl;
}

function getConvexClient() {
	if (!convexClient) {
		convexClient = new ConvexHttpClient(getConvexUrl());
	}

	return convexClient;
}

export async function waitForBodyText(driver, text, timeout = 15000) {
	await driver.wait(
		async () => {
			const bodyText = await driver.findElement(By.css("body")).getText();
			return bodyText.includes(text);
		},
		timeout,
		`Timed out waiting for text: ${text}`,
	);
}

export async function waitForAnyBodyText(driver, texts, timeout = 15000) {
	await driver.wait(
		async () => {
			const bodyText = await driver.findElement(By.css("body")).getText();
			return texts.some((text) => bodyText.includes(text));
		},
		timeout,
		`Timed out waiting for one of: ${texts.join(", ")}`,
	);
}

export async function waitForPathname(driver, matcher, timeout = 15000) {
	await driver.wait(
		async () => {
			const pathname = new URL(await driver.getCurrentUrl()).pathname;
			if (typeof matcher === "string") {
				return pathname === matcher;
			}

			if (matcher instanceof RegExp) {
				return matcher.test(pathname);
			}

			return matcher(pathname);
		},
		timeout,
		"Timed out waiting for expected pathname.",
	);
}

export async function ensureGamePageReady(driver) {
	await waitForAnyBodyText(driver, gameReadyMarkers);

	const initializeButtons = await driver.findElements(
		By.xpath("//button[normalize-space()='Initialize Colony']"),
	);
	if (
		initializeButtons.length > 0 &&
		(await initializeButtons[0].isDisplayed())
	) {
		await initializeButtons[0].click();
	}

	await waitForAnyBodyText(driver, loadedGameMarkers);
}

export async function openGamePage(driver, baseUrl) {
	await driver.get(new URL("/game", baseUrl).toString());
	await waitForPathname(driver, "/game");
	await ensureGamePageReady(driver);
}

export function getColonyPageUrl(baseUrl, colonyId) {
	return new URL(`/colony/${colonyId}`, baseUrl).toString();
}

async function getGlobalColony(client) {
	const colonies = await client.query(api.colonies.getAllColonies, {});
	return colonies.find((colony) => colony.isGlobal) ?? null;
}

export async function ensureGlobalColony({
	minimumMaterials = 0,
	ensureLeader = true,
} = {}) {
	const client = getConvexClient();

	await client.mutation(api.game.ensureGlobalState, {});
	let colony = await getGlobalColony(client);
	if (!colony) {
		throw new Error("No global colony was available after ensureGlobalState.");
	}

	if (minimumMaterials > 0 && colony.resources.materials < minimumMaterials) {
		await client.mutation(api.colonies.updateColonyResources, {
			colonyId: colony._id,
			resources: {
				...colony.resources,
				materials: minimumMaterials,
			},
		});
		colony = await getGlobalColony(client);
	}

	if (ensureLeader && !colony.leaderId) {
		const cats = await client.query(api.cats.getAliveCats, {
			colonyId: colony._id,
		});
		if (!cats[0]) {
			throw new Error("Active colony has no living cats to assign as leader.");
		}

		await client.mutation(api.colonies.setColonyLeader, {
			colonyId: colony._id,
			catId: cats[0]._id,
		});
		colony = await getGlobalColony(client);
	}

	return colony;
}

export async function readResourceFraction(driver, label) {
	const row = await driver.findElement(
		By.xpath(
			`//span[normalize-space()='${label}']/ancestor::div[contains(@class, 'justify-between')][1]`,
		),
	);
	const text = await row.getText();
	const match = text.match(/(-?\d+(?:\.\d+)?)\s*\/\s*(\d+(?:\.\d+)?)/);
	if (!match) {
		throw new Error(`Could not parse resource fraction for ${label}: ${text}`);
	}

	return {
		text,
		value: Number(match[1]),
		max: Number(match[2]),
	};
}
