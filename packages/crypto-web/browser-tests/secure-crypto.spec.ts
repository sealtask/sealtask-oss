import { expect, test } from '@playwright/test'

test('runs StrongBox, HPKE, and authenticated invite acceptance in secure Chromium', async ({
  page,
}) => {
  const workerUrls: string[] = []
  const wasmResponses: Array<{ status: number; url: string }> = []

  page.on('worker', (worker) => {
    workerUrls.push(worker.url())
  })
  page.on('response', (response) => {
    if (response.url().includes('.wasm')) {
      wasmResponses.push({
        status: response.status(),
        url: response.url(),
      })
    }
  })

  await page.goto('/browser-tests/')
  await expect(page.locator('html')).toHaveAttribute(
    'data-crypto-test-ready',
    'true',
  )

  const result = await page.evaluate(async () => window.runSecureCryptoFlow())

  expect(result).toEqual({
    acceptedListKeyMatches: true,
    hpkeRoundTrip: 'real worker hpke',
    inviteAuthentication: 'authenticated',
    invitePreviewTitle: 'Secure Chromium invitation',
    isSecureContext: true,
    passwordV1WrappingKeyMatches: true,
    strongBoxRoundTrip: 'module worker and canonical wasm',
  })
  expect(workerUrls.some((url) => url.includes('strong-box.worker'))).toBe(true)
  expect(
    wasmResponses.some(
      ({ status, url }) =>
        status === 200 && url.includes('strong_box_wasm_bg'),
    ),
  ).toBe(true)
  expect(
    wasmResponses.some(
      ({ status, url }) => status === 200 && url.includes('argon2'),
    ),
  ).toBe(true)
})
