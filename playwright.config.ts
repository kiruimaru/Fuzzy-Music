import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  use: {
    baseURL: "http://127.0.0.1:1420",
    launchOptions: { executablePath: process.env.CHROMIUM_PATH || "/usr/bin/chromium" },
  },
  webServer: {
    command: "npm run dev -- --port 1420 --strictPort",
    url: "http://127.0.0.1:1420",
    reuseExistingServer: true,
  },
});
