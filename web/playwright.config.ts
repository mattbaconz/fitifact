import { defineConfig, devices } from "@playwright/test";

const desktop = { viewport: { width: 1280, height: 900 } };
const mobile = { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true };

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  retries: 0,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "npm run build && npm run preview -- --host 127.0.0.1",
    port: 4173,
    reuseExistingServer: false,
    timeout: 180_000,
  },
  projects: [
    { name: "chromium-desktop", use: { ...devices["Desktop Chrome"], ...desktop } },
    { name: "firefox-desktop", use: { ...devices["Desktop Firefox"], ...desktop } },
    { name: "webkit-desktop", use: { ...devices["Desktop Safari"], ...desktop } },
    { name: "chromium-mobile", use: { ...devices["Pixel 7"], ...mobile } },
    { name: "firefox-mobile", use: { ...devices["Desktop Firefox"], ...mobile } },
    { name: "webkit-mobile", use: { ...devices["iPhone 14"], ...mobile } },
  ],
});
