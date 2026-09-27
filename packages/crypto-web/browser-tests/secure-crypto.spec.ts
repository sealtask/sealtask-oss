import { expect, test } from '@playwright/test'
import { createHash, generateKeyPairSync, sign } from 'node:crypto'

test('verifies independently encoded timestamped witness signatures in Chromium', async ({ page }) => {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519')
  const key = Buffer.from(publicKey.export({ format: 'jwk' }).x!, 'base64url')
  const name = 'independent.example/w1'
  const logOrigin = 'sealtask.com/transparency/v1'
  const body = `${logOrigin}\n1913\n${Buffer.alloc(32, 7).toString('base64')}\n`
  const timestamp = Math.floor(Date.now() / 1000)
  const id = createHash('sha256').update(Buffer.concat([Buffer.from(`${name}\n`), Buffer.from([4]), key])).digest().subarray(0, 4)
  const bytes = Buffer.alloc(76)
  bytes.set(id)
  bytes.writeBigUInt64BE(BigInt(timestamp), 4)
  bytes.set(sign(null, Buffer.from(`cosignature/v1\ntime ${timestamp}\n${body}`), privateKey), 12)
  const input = { note: `${body}\n— ${name} ${bytes.toString('base64')}\n`, name, logOrigin, publicKey: key.toString('base64') }
  await page.goto('/browser-tests/')
  await expect(page.locator('html')).toHaveAttribute('data-crypto-test-ready', 'true')
  expect(await page.evaluate(async (value) => window.verifyWitnessCheckpoint(value), input)).toBe(1913)
  await expect(page.evaluate(async (value) => window.verifyWitnessCheckpoint(value), { ...input, note: input.note.replace('\n1913\n', '\n1914\n') })).rejects.toThrow(/signature/)
})

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
