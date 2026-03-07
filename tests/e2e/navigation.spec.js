/**
 * E2E Tests: Navigation
 *
 * Tests navigation between pages and views.
 */

import { By } from "selenium-webdriver";
import {
	ensureGamePageReady,
	ensureGlobalColony,
	getColonyPageUrl,
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

		const colony = await ensureGlobalColony({ ensureLeader: true });
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

		await driver.get(getColonyPageUrl(baseUrl, colony._id));
		await waitForPathname(
			driver,
			(pathname) => pathname === `/colony/${colony._id}`,
		);
		await waitForBodyText(driver, "COZY COLONY");

		const colonyUrl = new URL(await driver.getCurrentUrl());
		if (
			colonyUrl.origin !== expectedOrigin ||
			colonyUrl.pathname !== `/colony/${colony._id}`
		) {
			throw new Error(
				`Expected colony route on the same origin, got ${colonyUrl}`,
			);
		}

		const colonyHeadings = await driver.findElements(
			By.xpath("//*[contains(normalize-space(), 'COZY COLONY')]"),
		);
		if (colonyHeadings.length === 0) {
			throw new Error("Colony route did not render its expected heading.");
		}
		console.log("  ✓ Colony route loads on the expected origin");

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
