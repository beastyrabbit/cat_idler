/**
 * E2E Tests: Resource Bars
 *
 * Tests resource display and values in the game HUD.
 */

import { readHudResource, openGamePage } from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testResourceBars(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing resource HUD...");

		await openGamePage(driver, baseUrl);

		for (const label of ["Food", "Water", "Herbs", "Materials", "Refined"]) {
			const fraction = await readHudResource(driver, label);
			if (!Number.isFinite(fraction.value) || !Number.isFinite(fraction.max)) {
				throw new Error(
					`Invalid resource fraction for ${label}: ${fraction.text}`,
				);
			}

			if (fraction.max <= 0) {
				throw new Error(`${label} progress bar max must be positive.`);
			}
		}

		console.log("  ✓ Resource HUD values are visible");
		console.log("  ✓ Resource HUD tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
