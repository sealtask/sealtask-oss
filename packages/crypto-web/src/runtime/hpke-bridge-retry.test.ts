import { expect, it, vi } from 'vitest'

it('retries the StrongBox HPKE bridge after a transient initialization failure', async () => {
  const wasmResult = {
    enc: new Uint8Array(32).fill(4),
    ciphertext: new Uint8Array([5, 6, 7]),
    nonce: new Uint8Array(12).fill(8),
  }
  const hpkeEncap = vi.fn().mockResolvedValue(wasmResult)
  const getStrongBoxBridge = vi
    .fn()
    .mockRejectedValueOnce(new Error('transient worker failure'))
    .mockResolvedValueOnce({
      encrypt: vi.fn(),
      decrypt: vi.fn(),
      hpkeEncap,
      hpkeDecap: vi.fn(),
    })

  vi.doMock('./strong-box', () => ({ getStrongBoxBridge }))

  const [{ hpkeSeal }, { clampScalar, derivePublicKey }] = await Promise.all([
    import('./hpke'),
    import('./x25519'),
  ])
  const recipientPrivateKey = clampScalar(new Uint8Array(32).fill(11))
  const recipientPublicKey = await derivePublicKey(recipientPrivateKey)
  const params = {
    recipientPublicKey,
    info: new Uint8Array([1]),
    aad: new Uint8Array([2]),
    plaintext: new Uint8Array([3]),
  }

  await hpkeSeal(params)
  await expect(hpkeSeal(params)).resolves.toEqual(wasmResult)
  expect(getStrongBoxBridge).toHaveBeenCalledTimes(2)
  expect(hpkeEncap).toHaveBeenCalledOnce()
})
