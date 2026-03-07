/**
 * E2E Tests: User Interactions
 *
 * Tests colony tab switching and direct user action effects.
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

export default async function testUserInteractions(
	headed = false,
	baseUrl = process.env.TEST_BASE_URL ?? "http://localhost:3000",
) {
	const driver = await createDriver(headed);

	try {
		console.log("  Testing user interactions...");

		const colony = await ensureGlobalColony({ ensureLeader: true });
		await driver.get(getColonyPageUrl(baseUrl, colony._id));
		await waitForPathname(
			driver,
			(pathname) => pathname === `/colony/${colony._id}`,
		);
		await waitForBodyText(driver, "COZY COLONY");

		const worldTab = await driver.findElement(
			By.xpath("//*[@role='tab' and normalize-space()='World']"),
		);
		await worldTab.click();
		await driver.wait(
			async () => {
				return (await worldTab.getAttribute("data-state")) === "active";
			},
			5000,
			"World tab did not become active.",
		);

		const colonyTab = await driver.findElement(
			By.xpath("//*[@role='tab' and normalize-space()='Colony']"),
		);
		await colonyTab.click();
		await driver.wait(
			async () => {
				return (await colonyTab.getAttribute("data-state")) === "active";
			},
			5000,
			"Colony tab did not become active again.",
		);
		console.log("  ✓ World and colony tab switching works");

		const actionsTab = await driver.findElement(
			By.xpath("//*[@role='tab' and normalize-space()='Actions']"),
		);
		await actionsTab.click();
		await waitForBodyText(driver, "🍖 Give Food (+1)");
		await waitForBodyText(driver, "💧 Give Water (+1)");

		const beforeFood = await readResourceFraction(driver, "Food");
		const beforeWater = await readResourceFraction(driver, "Water");

		const feedButton = await driver.findElement(
			By.xpath("//button[contains(., 'Give Food (+1)')]"),
		);
		await feedButton.click();
		await driver.wait(
			async () => {
				const currentFood = await readResourceFraction(driver, "Food");
				return currentFood.value > beforeFood.value;
			},
			10000,
			"Food did not increase after clicking Give Food.",
		);
		await driver.wait(
			async () => !(await feedButton.isEnabled()),
			5000,
			"Give Food button did not enter cooldown after use.",
		);

		const waterButton = await driver.findElement(
			By.xpath("//button[contains(., 'Give Water (+1)')]"),
		);
		await waterButton.click();
		await driver.wait(
			async () => {
				const currentWater = await readResourceFraction(driver, "Water");
				return currentWater.value > beforeWater.value;
			},
			10000,
			"Water did not increase after clicking Give Water.",
		);
		await driver.wait(
			async () => !(await waterButton.isEnabled()),
			5000,
			"Give Water button did not enter cooldown after use.",
		);

		console.log(
			"  ✓ Colony action buttons update resources and enter cooldown",
		);
		console.log("  ✓ All user interaction tests passed");
	} catch (error) {
		console.error("  ✗ Test failed");
		throw error;
	} finally {
		await driver.quit();
	}
}
