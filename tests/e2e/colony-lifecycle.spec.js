/**
 * E2E Tests: Colony Lifecycle
 *
 * Tests the active colony flow from the shared game page into the colony view.
 */

import { By } from "selenium-webdriver";
import {
	ensureGlobalColony,
	getColonyPageUrl,
	waitForBodyText,
	waitForPathname,
} from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testColonyLifecycle(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);
	const expectedOrigin = new URL(baseUrl).origin;

	try {
		console.log("  Testing colony lifecycle...");

		const colony = await ensureGlobalColony({ ensureLeader: true });
		await driver.get(getColonyPageUrl(baseUrl, colony._id));
		await waitForPathname(
			driver,
			(pathname) => pathname === `/colony/${colony._id}`,
		);
		await waitForBodyText(driver, "COZY COLONY");

		const colonyName = await driver.findElement(By.css("h1")).getText();
		if (colonyName !== colony.name) {
			throw new Error(
				`Colony page loaded unexpected colony name. Expected ${colony.name}, got ${colonyName}`,
			);
		}
		const colonyUrl = new URL(await driver.getCurrentUrl());
		if (
			colonyUrl.origin !== expectedOrigin ||
			colonyUrl.pathname !== `/colony/${colony._id}`
		) {
			throw new Error(
				`Expected colony route on ${expectedOrigin}, got ${colonyUrl}`,
			);
		}

		for (const label of [
			"Status:",
			"Cats",
			"Colony",
			"World",
			"Build",
			"Tasks",
			"Actions",
			"Events",
		]) {
			const matches = await driver.findElements(
				By.xpath(`//*[contains(normalize-space(), '${label}')]`),
			);
			if (matches.length === 0) {
				throw new Error(`Colony page missing expected control: ${label}`);
			}
		}
		console.log("  ✓ Colony page structure is ready");

		console.log("  ✓ All colony lifecycle tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
