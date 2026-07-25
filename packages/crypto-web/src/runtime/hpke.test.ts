import { describe, expect, it } from 'vitest'

import {
  decodeHpkeEnvelope,
  encodeHpkeEnvelope,
  hpkeOpen,
  hpkeSeal,
} from './hpke'
import { clampScalar, derivePublicKey } from './x25519'

describe('HPKE compatibility implementation', () => {
  it('round-trips the current envelope through the JS fallback', async () => {
    const recipientPrivateKey = clampScalar(
      Uint8Array.from({ length: 32 }, (_, index) => index + 7),
    )
    const recipientPublicKey = await derivePublicKey(recipientPrivateKey)
    const info = new TextEncoder().encode('hpke-binding-info')
    const aad = new TextEncoder().encode('hpke-aad')
    const plaintext = new TextEncoder().encode('current SealTask HPKE payload')

    const sealed = await hpkeSeal({ recipientPublicKey, info, aad, plaintext })
    const envelope = decodeHpkeEnvelope(encodeHpkeEnvelope(sealed))
    const opened = await hpkeOpen({
      recipientPrivateKey,
      info,
      aad,
      envelope,
    })

    expect(opened).toEqual(plaintext)
    expect(envelope).toMatchObject({
      version: 1,
      suite: {
        kem: 0x0010,
        kdf: 0x0001,
        aead: 0x0003,
        mode: 0,
      },
    })
  })

  it('rejects a modified authenticated payload', async () => {
    const recipientPrivateKey = clampScalar(new Uint8Array(32).fill(17))
    const recipientPublicKey = await derivePublicKey(recipientPrivateKey)
    const info = new Uint8Array([1, 2])
    const aad = new Uint8Array([3, 4])
    const sealed = await hpkeSeal({
      recipientPublicKey,
      info,
      aad,
      plaintext: new Uint8Array([5, 6, 7]),
    })
    sealed.ciphertext[sealed.ciphertext.length - 1] ^= 0xff

    await expect(
      hpkeOpen({
        recipientPrivateKey,
        info,
        aad,
        envelope: decodeHpkeEnvelope(encodeHpkeEnvelope(sealed)),
      }),
    ).rejects.toThrow()
  })
})
