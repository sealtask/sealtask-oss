import { describe, expect, it, vi } from 'vitest'

import type { StrongBoxBridge } from './strong-box-types'
import {
  detectCryptoGateIssue,
  supportsWasmReferenceTypes,
  verifyStrongBoxRoundTrip,
} from './capabilities'

describe('browser crypto capabilities', () => {
  it('reports unavailable browser capabilities in gate order', () => {
    expect(
      detectCryptoGateIssue({
        isSecureContext: false,
        crypto: {} as Crypto,
      }),
    ).toEqual({ type: 'insecure-context' })
    expect(
      detectCryptoGateIssue({
        isSecureContext: true,
        crypto: undefined,
      }),
    ).toEqual({ type: 'missing-subtle' })
    expect(
      detectCryptoGateIssue({
        isSecureContext: true,
        crypto: { subtle: {} } as Crypto,
        WebAssembly: { validate: () => false },
      }),
    ).toEqual({ type: 'missing-wasm-reference-types' })
  })

  it('passes the externref probe to WebAssembly.validate', () => {
    const validate = vi.fn(() => true)
    expect(supportsWasmReferenceTypes({ validate })).toBe(true)
    expect(validate).toHaveBeenCalledWith(expect.any(Uint8Array))
  })

  it('verifies a StrongBox encrypt/decrypt round trip', async () => {
    const bridge: StrongBoxBridge = {
      encrypt: vi.fn(async ({ plaintext }) => plaintext.slice()),
      decrypt: vi.fn(async ({ ciphertext }) => ciphertext.slice()),
    }
    await expect(verifyStrongBoxRoundTrip(bridge)).resolves.toBeUndefined()
  })

  it('rejects a mismatched StrongBox round trip', async () => {
    const bridge: StrongBoxBridge = {
      encrypt: vi.fn(async ({ plaintext }) => plaintext.slice()),
      decrypt: vi.fn(async () => new Uint8Array([0])),
    }
    await expect(verifyStrongBoxRoundTrip(bridge)).rejects.toThrow(
      'StrongBox local round trip failed',
    )
  })
})
