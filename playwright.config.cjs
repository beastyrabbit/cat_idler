const browserUrl = process.env.LAI_PLAYTEST_BROWSER_URL;

module.exports = {
  testDir: "./tests/browser-playtests",
  testMatch: "**/*.spec.cjs",
  timeout: 30_000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: process.env.CI ? "line" : "list",
  use: {
    baseURL: browserUrl,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
    serviceWorkers: "block",
  },
};
