import { defineConfig } from '@playwright/test';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const installedChromium = join(homedir(), '.cache/ms-playwright/chromium-1243/chrome-linux64/chrome');
export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  timeout: 45_000,
  expect: { timeout: 10_000 },
  outputDir: process.env.E2E_OUTPUT_DIR || '/tmp/syco23-catalogue-verification/e2e-results',
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:15173',
    viewport: { width: 1440, height: 1000 },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    launchOptions: existsSync(installedChromium) ? { executablePath: installedChromium } : {},
  },
  webServer: [
    { command: 'node --import ./server/node_modules/tsx/dist/loader.mjs tests/e2e/serve-api.mts', url: 'http://localhost:18787/api/live', reuseExistingServer: false, timeout: 45_000 },
    { command: 'pnpm --dir client dev --host localhost --port 15173 --strictPort', url: 'http://localhost:15173', reuseExistingServer: false, timeout: 45_000, env: { MIXSETS_API_PROXY: 'http://127.0.0.1:18787' } },
  ],
});
