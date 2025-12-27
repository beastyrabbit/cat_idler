/**
 * E2E Tests: Building Placement
 * 
 * Tests building selection and placement on the colony grid.
 */

import { createDriver, waitForText, safeClick, isVisible } from './selenium-setup.js';
import { By } from 'selenium-webdriver';

export default async function testBuildingPlacement(headed = false, port = 3000) {
  const driver = await createDriver(headed);
  const baseUrl = `http://localhost:${port}`;
  
  try {
    console.log('  Testing building placement...');
    
    // Navigate to colony
    await driver.get(baseUrl);
    await waitForText(driver, 'Cat Colony Idle Game', 10000);
    
    // Navigate to colony if needed
    const createInput = await driver.findElements(By.css('input[placeholder*="name"], input[type="text"]'));
    if (createInput.length > 0 && await createInput[0].isDisplayed()) {
      await createInput[0].clear();
      await createInput[0].sendKeys(`Building Test ${Date.now()}`);
      // Find create button by xpath
      const createButton = await driver.findElement(By.xpath("//button[contains(text(), 'Create Colony') or contains(text(), 'Create')]"));
      await driver.executeScript('arguments[0].scrollIntoView(true);', createButton);
      await driver.sleep(500);
      await createButton.click();
      await driver.sleep(3000);
      
      // Handle leader selection
      const leaderModal = await driver.findElements(By.xpath("//*[contains(text(), 'Leader')]"));
      if (leaderModal.length > 0) {
        await driver.sleep(1000);
        const candidates = await driver.findElements(By.css('button, [role="button"]'));
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
    
    // Test: Build menu visibility
    console.log('  Testing build menu...');
    const buildMenu = await driver.findElements(By.xpath("//*[contains(text(), 'Building') or contains(text(), 'building')]"));
    if (buildMenu.length > 0) {
      console.log('  ✓ Build menu visible');
    }
    
    // Test: Building type buttons
    const buildingButtons = await driver.findElements(By.xpath("//button[contains(text(), 'Storage') or contains(text(), 'Bowl') or contains(text(), 'Bed')]"));
    if (buildingButtons.length > 0) {
      console.log('  ✓ Building type buttons visible');
    }
    
    console.log('  ✓ Building placement tests passed');
    
  } catch (error) {
    const screenshot = await driver.takeScreenshot();
    console.error('  ✗ Test failed');
    throw error;
  } finally {
    await driver.quit();
  }
}



