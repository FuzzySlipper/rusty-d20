import { defineConfig, devices } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';
import { nxE2EPreset } from '@nx/playwright/preset';

const localPort = process.env['E2E_PORT'] ?? '4317';
const localBaseUrl = `http://127.0.0.1:${localPort}`;
const baseURL = process.env['BASE_URL'] ?? localBaseUrl;
const localSavePath = `target/rusty-d20/e2e-${process.pid}.json`;

const localWebServer = process.env['BASE_URL']
  ? {}
  : {
      webServer: {
        command: `cargo run -p rusty-d20 --bin rusty-d20-host -- --address 127.0.0.1:${localPort} --save-file ${localSavePath}`,
        url: `${localBaseUrl}/healthz`,
        reuseExistingServer: false,
        cwd: workspaceRoot,
        timeout: 180_000,
      },
    };

export default defineConfig({
  ...nxE2EPreset(import.meta.dirname, { testDir: './src' }),
  use: {
    baseURL,
    trace: 'on-first-retry',
  },
  testIgnore: process.env['LIVE_RUN'] === '1' ? [] : ['**/live/**'],
  workers: 1,
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  ...localWebServer,
});
