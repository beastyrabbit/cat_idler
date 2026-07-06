/**
 * E2E Tests: Map Controls
 *
 * Tests stable controls on the map-first game screen.
 */

import { By } from "selenium-webdriver";
import { openGamePage, waitForBodyText } from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testBuildingPlacement(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing map controls...");

		await openGamePage(driver, baseUrl);

		const infoButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Info')]"),
		);
		await infoButton.click();
		await driver.wait(
			async () => {
				const className = await infoButton.getAttribute("class");
				return className.includes("bg-amber-400");
			},
			5000,
			"Info button did not enter its active state.",
		);
		console.log("  ✓ Tile info toggle activates");

		const gatherZoneButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Gather zone')]"),
		);
		await gatherZoneButton.click();
		await waitForBodyText(driver, "Click the first corner on the map.");

		const cancelButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Cancel')]"),
		);
		await cancelButton.click();
		await waitForBodyText(driver, "Gather zone");
		console.log("  ✓ Zone draft can be started and cancelled");

		const treeButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Tree')]"),
		);
		await treeButton.click();
		await waitForBodyText(driver, "Upgrade Tree");

		const closeTreeButton = await driver.findElement(
			By.css("button[aria-label='Close upgrade tree']"),
		);
		await closeTreeButton.click();
		await waitForBodyText(driver, "LEND A PAW");
		console.log("  ✓ Upgrade tree opens and closes");

		console.log("  ✓ Map controls tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
