/**
 * E2E Tests: Building Placement
 *
 * Tests building selection and placement on the colony grid.
 */

import { By } from "selenium-webdriver";
import {
	ensureGlobalColony,
	getColonyPageUrl,
	waitForBodyText,
	waitForPathname,
} from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testBuildingPlacement(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing building placement...");

		const colony = await ensureGlobalColony({
			ensureLeader: true,
			minimumMaterials: 3,
		});
		await driver.get(getColonyPageUrl(baseUrl, colony._id));
		await waitForPathname(
			driver,
			(pathname) => pathname === `/colony/${colony._id}`,
		);
		await waitForBodyText(driver, "COZY COLONY");
		await waitForBodyText(driver, "Buildings");

		const waterBowlButton = await driver.findElement(
			By.xpath("//button[.//*[normalize-space()='Water Bowl']]"),
		);
		if (!(await waterBowlButton.isEnabled())) {
			throw new Error("Water Bowl should be buildable after materials top-up.");
		}

		const existingBuildingRows = await driver.findElements(
			By.xpath(
				"//*[normalize-space()='Existing buildings']/following-sibling::div[1]//span[contains(., '(')]",
			),
		);
		await waterBowlButton.click();
		await waitForBodyText(
			driver,
			"Click on the colony grid to place: Water Bowl",
		);

		const placeTargets = await driver.findElements(
			By.xpath("//button[.//*[contains(normalize-space(), 'Place here')]]"),
		);
		if (placeTargets.length === 0) {
			throw new Error("No buildable colony tile was available for placement.");
		}

		const targetTile = placeTargets[0];
		const tileText = await targetTile.getText();
		const coordinateMatch = tileText.match(/(\d+),(\d+)/);
		await driver.executeScript(
			"arguments[0].scrollIntoView({block: 'center'});",
			targetTile,
		);
		await driver.executeScript("arguments[0].click();", targetTile);

		await driver.wait(
			async () => {
				const placementPrompts = await driver.findElements(
					By.xpath(
						"//*[contains(normalize-space(), 'Click on the colony grid to place: Water Bowl')]",
					),
				);
				return placementPrompts.length === 0;
			},
			5000,
			"Placement prompt did not clear after placing the building.",
		);

		await driver.wait(
			async () => {
				const currentBuildingRows = await driver.findElements(
					By.xpath(
						"//*[normalize-space()='Existing buildings']/following-sibling::div[1]//span[contains(., '(')]",
					),
				);
				return currentBuildingRows.length === existingBuildingRows.length + 1;
			},
			10000,
			"Existing buildings list did not grow after placement.",
		);

		if (coordinateMatch) {
			const [, x, y] = coordinateMatch;
			await waitForBodyText(driver, `Water Bowl (${x}, ${y})`);
		}

		console.log("  ✓ Building selection and placement update the colony grid");
		console.log("  ✓ Building placement tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
