/**
 * Selenium WebDriver Setup
 * 
 * Creates and configures a Selenium WebDriver instance.
 */

import { Builder, By, until } from 'selenium-webdriver';
import chrome from 'selenium-webdriver/chrome.js';

export async function createDriver(headed = false) {
  const options = new chrome.Options();
  
  if (!headed) {
    options.addArguments('--headless');
  }
  
  options.addArguments('--no-sandbox');
  options.addArguments('--disable-dev-shm-usage');
  options.addArguments('--disable-gpu');
  options.addArguments('--window-size=1920,1080');

  const driver = await new Builder()
    .forBrowser('chrome')
    .setChromeOptions(options)
    .build();

  return driver;
}

export async function waitForElement(driver, selector, timeout = 10000) {
  return await driver.wait(
    until.elementLocated(By.css(selector)),
    timeout
  );
}

export async function waitForText(driver, text, timeout = 10000) {
  return await driver.wait(
    until.elementLocated(By.xpath(`//*[contains(text(), '${text}')]`)),
    timeout
  );
}

export async function safeClick(driver, selector, timeout = 10000) {
  const element = await waitForElement(driver, selector, timeout);
  await driver.executeScript('arguments[0].scrollIntoView(true);', element);
  await driver.sleep(500); // Small delay for scroll
  await element.click();
}

export async function safeFill(driver, selector, text, timeout = 10000) {
  const element = await waitForElement(driver, selector, timeout);
  await element.clear();
  await element.sendKeys(text);
}

export async function getText(driver, selector, timeout = 10000) {
  const element = await waitForElement(driver, selector, timeout);
  return await element.getText();
}

export async function isVisible(driver, selector, timeout = 5000) {
  try {
    const element = await driver.wait(
      until.elementLocated(By.css(selector)),
      timeout
    );
    return await element.isDisplayed();
  } catch {
    return false;
  }
}

export async function isTextVisible(driver, text, timeout = 5000) {
  try {
    await driver.wait(
      until.elementLocated(By.xpath(`//*[contains(text(), '${text}')]`)),
      timeout
    );
    return true;
  } catch {
    return false;
  }
}



