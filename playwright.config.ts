import { defineConfig, devices } from '@playwright/test';

// E2E tests run against the production build served by `astro preview`,
// driving real Chromium so the full UI -> Web Worker -> download pipeline
// is exercised exactly as a user would.
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  // Each test spins up Chromium + pdf.js/wasm workers; cap concurrency so the
  // single preview server and CPU aren't thrashed (avoids flaky timeouts).
  workers: 2,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  timeout: 60000,
  use: {
    baseURL: 'http://localhost:4321',
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run build && npm run preview',
    url: 'http://localhost:4321',
    reuseExistingServer: !process.env.CI,
    timeout: 180000,
  },
});
