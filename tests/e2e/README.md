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

2. **Frontend will start automatically** through Portless when you run E2E tests.
   ```bash
   bun run dev:url
   ```
   The runner uses `TEST_BASE_URL` if provided, otherwise it targets the same computed `http://<name>.localhost:1355` URL as `bun run dev`.

3. **Chrome/Chromium must be installed** on your system

4. Optional escape hatches:
   ```bash
   TEST_BASE_URL=http://my-custom-name.localhost:1355 bun run test:e2e
   PORTLESS=skip bun run dev
   ```

## Test Files

- `colony-lifecycle.spec.js` - Tests colony creation and basic flow
- `user-interactions.spec.js` - Tests user actions (feed, heal, etc.)
- `building-placement.spec.js` - Tests building selection and placement
- `resource-bars.spec.js` - Tests resource bar display and updates
- `navigation.spec.js` - Tests page navigation

## Writing New Tests

```javascript
import { createDriver, waitForText, safeClick } from './selenium-setup.js';

export default async function testMyFeature(
  headed = false,
  baseUrl = process.env.TEST_BASE_URL ?? 'http://localhost:3000'
) {
  const driver = await createDriver(headed);
  
  try {
    await driver.get(baseUrl);
    await waitForText(driver, 'Cat Colony Idle Game');
    // Your test code
  } finally {
    await driver.quit();
  }
}
```

## Debugging

- Use `bun run test:e2e:headed` to see the browser
- Run `bun run dev:url` to confirm the computed Portless target
- Screenshots are saved on failure
- Check console output for detailed error messages


