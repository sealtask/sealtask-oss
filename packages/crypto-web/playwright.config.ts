import { defineConfig, devices } from '@playwright/test'
import { tmpdir } from 'node:os'
import path from 'node:path'

const port = Number(process.env.CRYPTO_WEB_BROWSER_TEST_PORT ?? 4175)
const baseURL = `http://127.0.0.1:${port}`

export default defineConfig({
  testDir: './browser-tests',
  timeout: 120_000,
  expect: {
    timeout: 30_000,
  },
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list']],
  outputDir: path.join(tmpdir(), 'sealtask-crypto-web-playwright'),
  use: {
    baseURL,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  webServer: {
    command: `bunx vite --config browser-tests/vite.config.ts --host 127.0.0.1 --port ${port} --strictPort`,
    url: `${baseURL}/browser-tests/`,
    timeout: 120_000,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
  },
  projects: [
    {
      name: 'secure-chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})
