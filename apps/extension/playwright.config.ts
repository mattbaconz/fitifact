import { defineConfig } from "@playwright/test";

// Each test launches Chromium with the unpacked build in `dist` (run `npm run build` first).
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  workers: 2,
  retries: 0,
  timeout: 120_000,
  reporter: "line",
  use: { trace: "retain-on-failure" },
});
