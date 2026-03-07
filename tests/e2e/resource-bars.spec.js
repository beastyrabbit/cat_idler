/**
 * E2E Tests: Resource Bars
 *
 * Tests resource bar display and values on the colony page.
 */

import { By } from "selenium-webdriver";
import {
	ensureGlobalColony,
	getColonyPageUrl,
	readResourceFraction,
	waitForBodyText,
	waitForPathname,
} from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testResourceBars(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing resource bars...");

		const colony = await ensureGlobalColony({ ensureLeader: true });
		await driver.get(getColonyPageUrl(baseUrl, colony._id));
		await waitForPathname(
			driver,
			(pathname) => pathname === `/colony/${colony._id}`,
		);
		await waitForBodyText(driver, "COZY COLONY");

		for (const label of ["Food", "Water", "Herbs", "Materials", "Blessings"]) {
			const progressBar = await driver.findElement(
				By.css(`[data-testid="resource-progress"][aria-label="${label}"]`),
			);
			await driver.executeScript(
				"arguments[0].scrollIntoView({block: 'center'});",
				progressBar,
			);

			const fraction = await readResourceFraction(driver, label);
			if (!Number.isFinite(fraction.value) || !Number.isFinite(fraction.max)) {
				throw new Error(
					`Invalid resource fraction for ${label}: ${fraction.text}`,
				);
			}

			if (fraction.max <= 0) {
				throw new Error(`${label} progress bar max must be positive.`);
			}
		}

		console.log("  ✓ Resource bars and values are visible");
		console.log("  ✓ Resource bars tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
