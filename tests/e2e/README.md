# E2E Tests with Selenium

These tests run in a real browser and test the actual GUI using Selenium WebDriver.

## Running E2E Tests

```bash
# Run all E2E tests (headless)
bun run test:e2e

# Run in headed mode (see browser)
bun run test:e2e:headed
```

## Prerequisites

1. **Start Convex backend:**
   ```bash
   bun run convex:dev
   ```

2. **Frontend will start automatically** when you run E2E tests (configured in `run-tests.js`)

3. **Chrome/Chromium must be installed** on your system

## Test Files

- `colony-lifecycle.spec.js` - Tests colony creation and basic flow
- `user-interactions.spec.js` - Tests user actions (feed, heal, etc.)
- `building-placement.spec.js` - Tests building selection and placement
- `resource-bars.spec.js` - Tests resource bar display and updates
- `navigation.spec.js` - Tests page navigation

## Writing New Tests

```javascript
import { createDriver, waitForText, safeClick } from './selenium-setup.js';

export default async function testMyFeature(headed = false) {
  const driver = await createDriver(headed);
  
  try {
    await driver.get('http://localhost:3000');
    await waitForText(driver, 'Cat Colony Idle Game');
    // Your test code
  } finally {
    await driver.quit();
  }
}
```

## Debugging

- Use `bun run test:e2e:headed` to see the browser
- Screenshots are saved on failure
- Check console output for detailed error messages



