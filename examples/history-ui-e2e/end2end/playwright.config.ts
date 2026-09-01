import { defineConfig, devices } from "@playwright/test";

const headed = !!process.env.PW_HEADED;

export default defineConfig({
  testDir: "./tests",
  timeout: 300_000,
  expect: { timeout: 60_000 },
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://localhost:3000",
    actionTimeout: 60_000,
    navigationTimeout: 120_000,
    ...devices["Desktop Chrome"],
    ...(headed
      ? {
          launchOptions: {
            slowMo: Number(process.env.PW_SLOW_MO ?? 250),
          },
        }
      : {}),
  },
});
