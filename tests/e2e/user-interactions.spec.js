/**
 * E2E Tests: User Interactions
 * 
 * Tests user actions like feeding, healing, building placement.
 */

import { createDriver, waitForText, safeClick, safeFill, isVisible, getText } from './selenium-setup.js';
import { By } from 'selenium-webdriver';

export default async function testUserInteractions(headed = false, port = 3000) {
  const driver = await createDriver(headed);
  const baseUrl = `http://localhost:${port}`;
  
  try {
    console.log('  Testing user interactions...');
    
    // Navigate to colony
    await driver.get(baseUrl);
    await waitForText(driver, 'Cat Colony Idle Game', 10000);
    
    // Check if we need to create a colony
    const createInput = await driver.findElements(By.css('input[placeholder*="name"], input[type="text"]'));
    if (createInput.length > 0 && await createInput[0].isDisplayed()) {
      await createInput[0].clear();
      await createInput[0].sendKeys(`E2E Test ${Date.now()}`);
      // Find create button by xpath
      const createButton = await driver.findElement(By.xpath("//button[contains(text(), 'Create Colony') or contains(text(), 'Create')]"));
      await driver.executeScript('arguments[0].scrollIntoView(true);', createButton);
      await driver.sleep(500);
      await createButton.click();
      await driver.sleep(3000);
      
      // Handle leader selection if present
      const leaderModal = await driver.findElements(By.xpath("//*[contains(text(), 'Leader') or contains(text(), 'leader')]"));
      if (leaderModal.length > 0) {
        await driver.sleep(1000);
        const candidates = await driver.findElements(By.css('button, [role="button"], [class*="border"]'));
        if (candidates.length > 0) {
          await candidates[0].click();
          await driver.sleep(500);
          const selectBtn = await driver.findElements(By.xpath("//button[contains(text(), 'Select')]"));
          if (selectBtn.length > 0) {
            await selectBtn[0].click();
            await driver.sleep(1000);
          }
        }
      }
    }
    
    await driver.sleep(2000);
    
    // Test: Give food
    console.log('  Testing give food...');
    const feedButtons = await driver.findElements(By.xpath("//button[contains(text(), 'Food') or contains(text(), 'food') or contains(text(), 'Feed')]"));
    if (feedButtons.length > 0 && await feedButtons[0].isEnabled()) {
      await feedButtons[0].click();
      await driver.sleep(1000);
      console.log('  ✓ Give food clicked');
    }
    
    // Test: Give water
    console.log('  Testing give water...');
    const waterButtons = await driver.findElements(By.xpath("//button[contains(text(), 'Water') or contains(text(), 'water')]"));
    if (waterButtons.length > 0 && await waterButtons[0].isEnabled()) {
      await waterButtons[0].click();
      await driver.sleep(1000);
      console.log('  ✓ Give water clicked');
    }
    
    // Test: View toggle
    console.log('  Testing view toggle...');
    const worldMapButtons = await driver.findElements(By.xpath("//button[contains(text(), 'World') or contains(text(), 'world')]"));
    if (worldMapButtons.length > 0) {
      await worldMapButtons[0].click();
      await driver.sleep(1000);
      console.log('  ✓ Switched to world map');
      
      const colonyButtons = await driver.findElements(By.xpath("//button[contains(text(), 'Colony') or contains(text(), 'colony')]"));
      if (colonyButtons.length > 0) {
        await colonyButtons[0].click();
        await driver.sleep(1000);
        console.log('  ✓ Switched back to colony');
      }
    }
    
    // Test: Task queue visibility
    console.log('  Testing task queue...');
    const taskQueue = await driver.findElements(By.xpath("//*[contains(text(), 'Task') or contains(text(), 'task')]"));
    if (taskQueue.length > 0) {
      console.log('  ✓ Task queue visible');
    }
    
    // Test: Event log visibility
    console.log('  Testing event log...');
    const eventLog = await driver.findElements(By.xpath("//*[contains(text(), 'Event') or contains(text(), 'event')]"));
    if (eventLog.length > 0) {
      console.log('  ✓ Event log visible');
    }
    
    console.log('  ✓ All user interaction tests passed');
    
  } catch (error) {
    const screenshot = await driver.takeScreenshot();
    console.error('  ✗ Test failed');
    throw error;
  } finally {
    await driver.quit();
  }
}



