/**
 * E2E Tests: Colony Lifecycle
 *
 * Tests the shared game page loads the current map-first UI.
 */

import { By } from "selenium-webdriver";
import { openGamePage, waitForBodyText } from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testColonyLifecycle(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);
	const expectedOrigin = new URL(baseUrl).origin;

	try {
		console.log("  Testing game map lifecycle...");

		await openGamePage(driver, baseUrl);

		const gameUrl = new URL(await driver.getCurrentUrl());
		if (gameUrl.origin !== expectedOrigin || gameUrl.pathname !== "/game") {
			throw new Error(`Expected /game on ${expectedOrigin}, got ${gameUrl}`);
		}

		for (const label of [
			"Catford",
			"Colony Work",
			"Leadership",
			"Zones",
			"Lend a Paw",
			"Supply food",
			"Supply water",
			"Plan hunt",
			"Upgrades",
		]) {
			const matches = await driver.findElements(
				By.xpath(`//*[contains(normalize-space(), '${label}')]`),
			);
			if (matches.length === 0) {
				throw new Error(`Game page missing expected control: ${label}`);
			}
		}
		await waitForBodyText(driver, "cats");
		console.log("  ✓ Game map structure is ready");

		console.log("  ✓ All game map lifecycle tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
