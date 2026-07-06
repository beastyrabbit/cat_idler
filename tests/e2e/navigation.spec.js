/**
 * E2E Tests: Navigation
 *
 * Tests navigation between pages and views.
 */

import { By } from "selenium-webdriver";
import {
	ensureGamePageReady,
	ensureGlobalGameState,
	waitForBodyText,
	waitForPathname,
} from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testNavigation(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);
	const expectedOrigin = new URL(baseUrl).origin;

	try {
		console.log("  Testing navigation...");

		await ensureGlobalGameState(baseUrl);
		await driver.get(baseUrl);
		await waitForPathname(driver, "/game");
		await ensureGamePageReady(driver);

		const gameUrl = new URL(await driver.getCurrentUrl());
		if (gameUrl.origin !== expectedOrigin || gameUrl.pathname !== "/game") {
			throw new Error(
				`Expected root navigation to land on /game, got ${gameUrl}`,
			);
		}
		console.log("  ✓ Root route redirects to /game");

		await driver.get(new URL("/game", baseUrl).toString());
		await waitForPathname(driver, "/game");
		await ensureGamePageReady(driver);
		const gameHeading = await driver.findElements(
			By.xpath("//*[contains(normalize-space(), 'Catford')]"),
		);
		if (gameHeading.length === 0) {
			throw new Error("Game route did not render the map HUD heading.");
		}
		console.log("  ✓ Game route loads on the expected origin");

		const examinerLink = await driver.findElement(
			By.xpath("//a[contains(normalize-space(), 'Examiner')]"),
		);
		await examinerLink.click();
		await waitForPathname(driver, "/game/newspaper");
		await waitForBodyText(driver, "BACK TO THE VILLAGE MAP");

		await driver.navigate().back();
		await waitForPathname(driver, "/game");
		await ensureGamePageReady(driver);

		const backUrl = new URL(await driver.getCurrentUrl());
		if (backUrl.origin !== expectedOrigin || backUrl.pathname !== "/game") {
			throw new Error(
				`Expected browser back to return to /game on ${expectedOrigin}, got ${backUrl}`,
			);
		}
		console.log("  ✓ Browser back returns to the game route");

		console.log("  ✓ All navigation tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
