/**
 * E2E Tests: User Interactions
 *
 * Tests current map UI interactions and signed action buttons.
 */

import { By } from "selenium-webdriver";
import {
	getGameDashboard,
	openGamePage,
	waitForBodyText,
	waitForPathname,
} from "./helpers.js";
import { createDriver } from "./selenium-setup.js";

export default async function testUserInteractions(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing user interactions...");

		await openGamePage(driver, baseUrl);

		for (const label of ["Supply food", "Supply water", "Plan hunt"]) {
			const button = await driver.findElement(
				By.xpath(`//button[contains(normalize-space(), '${label}')]`),
			);
			if (!(await button.isDisplayed())) {
				throw new Error(`${label} button is not visible.`);
			}
		}
		console.log("  ✓ Primary action buttons are visible");

		const supplyFoodButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Supply food')]"),
		);
		if (!(await supplyFoodButton.isEnabled())) {
			throw new Error("Supply food button should be available.");
		}
		const beforeDashboard = await getGameDashboard(baseUrl);
		const beforeJobIds = new Set(
			(beforeDashboard.jobs ?? []).map((job) => job._id),
		);
		await supplyFoodButton.click();
		await waitForPathname(driver, "/game");
		await waitForBodyText(driver, "COLONY WORK");
		await driver.wait(
			async () => {
				const buttons = await driver.findElements(
					By.xpath("//button[contains(normalize-space(), 'Supply food')]"),
				);
				return buttons.length > 0 && (await buttons[0].isEnabled());
			},
			10000,
			"Supply food button did not become available after the action request.",
		);
		await driver.wait(
			async () => {
				const afterDashboard = await getGameDashboard(baseUrl);
				return (afterDashboard.jobs ?? []).some(
					(job) => job.kind === "supply_food" && !beforeJobIds.has(job._id),
				);
			},
			10000,
			"Supply food click did not create a new signed job.",
		);
		console.log("  ✓ Supply food action can be clicked");

		const upgradesButton = await driver.findElement(
			By.xpath("//button[contains(normalize-space(), 'Upgrades')]"),
		);
		await upgradesButton.click();
		await waitForBodyText(driver, "click power");
		await upgradesButton.click();
		await waitForBodyText(driver, "LEND A PAW");
		console.log("  ✓ Upgrades drawer toggles");

		console.log("  ✓ All user interaction tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
