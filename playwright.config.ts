import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e', fullyParallel: false, workers: 1, retries: 0,
  timeout: 60000, expect: { timeout: 10000 }, reporter: 'list',
  use: { baseURL: 'http://127.0.0.1:5178', browserName: 'chromium', viewport: { width: 1365, height: 900 }, colorScheme: 'dark', trace: 'off' },
  webServer: { command: 'pnpm --filter @starter/pwa preview', url: 'http://127.0.0.1:5178', reuseExistingServer: false, timeout: 30000 },
});
