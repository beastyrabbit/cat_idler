import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { By } from "selenium-webdriver";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, "../..");
const loadedGameMarkers = [
	"Catford",
	"Colony Work",
	"Zones",
	"Lend a Paw",
	"Supply food",
];

let loadedEnv = false;

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

export async function postGameAction(baseUrl, payload) {
	const response = await fetch(new URL("/api/game/actions", baseUrl), {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify(payload),
	});
	const result = await response.json().catch(() => null);

	if (!response.ok) {
		const message =
			result && typeof result === "object" && "message" in result
				? result.message
				: `Request failed (${response.status})`;
		throw new Error(`Game action ${payload.action} failed: ${message}`);
	}

	return result;
}

export async function getGameDashboard(baseUrl) {
	const response = await fetch(new URL("/api/game/dashboard", baseUrl));
	const result = await response.json().catch(() => null);

	if (!response.ok || !result) {
		throw new Error(`Dashboard request failed (${response.status})`);
	}

	return result;
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
	await waitForAnyBodyText(driver, loadedGameMarkers);
}

export async function openGamePage(driver, baseUrl) {
	const identity = await ensureGlobalGameState(baseUrl);
	await driver.get(new URL("/game", baseUrl).toString());
	await driver.executeScript(
		`
			localStorage.setItem("cat_idle_session", arguments[0]);
			localStorage.setItem("cat_idle_sig", arguments[1]);
			localStorage.setItem("cat_idle_nickname", arguments[2]);
		`,
		identity.sessionId,
		identity.sig,
		identity.nickname,
	);
	await driver.navigate().refresh();
	await waitForPathname(driver, "/game");
	await ensureGamePageReady(driver);
}

export async function ensureGlobalGameState(baseUrl) {
	loadLocalEnv();
	await postGameAction(baseUrl, { action: "ensure" });
	const nickname = `E2E Cat ${Date.now()}`;
	const presence = await postGameAction(baseUrl, {
		action: "presence",
		nickname,
	});

	if (
		!presence ||
		typeof presence.sessionId !== "string" ||
		typeof presence.sig !== "string"
	) {
		throw new Error("Presence action did not return a signed session.");
	}

	return {
		nickname,
		sessionId: presence.sessionId,
		sig: presence.sig,
	};
}

export async function readHudResource(driver, label) {
	const resource = await driver.findElement(By.css(`span[title^="${label}:"]`));
	const text = await resource.getAttribute("title");
	const match = text.match(/:\s*(-?\d+(?:\.\d+)?)\s*\/\s*(\d+(?:\.\d+)?)/);
	if (!match) {
		throw new Error(`Could not parse resource HUD value for ${label}: ${text}`);
	}

	return {
		text,
		value: Number(match[1]),
		max: Number(match[2]),
	};
}
