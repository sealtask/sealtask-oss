import { beforeEach, describe, expect, it, vi } from 'vitest'

import hpkeVectors from '../../test/fixtures/hpke-vectors.json'
import {
  decodeHpkeEnvelope,
  encodeHpkeEnvelope,
  hpkeOpen,
  hpkeSeal,
} from './hpke'
import { clampScalar, derivePublicKey } from './x25519'

const bridgeMocks = vi.hoisted(() => ({
  getStrongBoxBridge: vi.fn(),
}))

vi.mock('./strong-box', () => ({
  getStrongBoxBridge: bridgeMocks.getStrongBoxBridge,
}))

beforeEach(() => {
  bridgeMocks.getStrongBoxBridge
    .mockReset()
    .mockRejectedValue(new Error('WASM bridge unavailable in fallback tests'))
})

describe('HPKE compatibility implementation', () => {
  it('matches RFC 9180 Appendix A.2 exactly in the JS fallback', async () => {
    const vector = hpkeVectors.rfc9180_appendix_a2
    const senderPrivateKey = hexBytes(vector.sender_private_key)
    const recipientPrivateKey = hexBytes(vector.recipient_private_key)
    const recipientPublicKey = hexBytes(vector.recipient_public_key)
    const info = hexBytes(vector.info)
    const aad = hexBytes(vector.aad)
    const plaintext = hexBytes(vector.plaintext)
    const randomSpy = mockRandomBytes(senderPrivateKey)

    try {
      const sealed = await hpkeSeal({ recipientPublicKey, info, aad, plaintext })
      const envelopeBytes = encodeHpkeEnvelope(sealed)
      const envelope = decodeHpkeEnvelope(envelopeBytes)

      expect(sealed.enc).toEqual(hexBytes(vector.enc))
      expect(sealed.nonce).toEqual(hexBytes(vector.nonce))
      expect(sealed.ciphertext).toEqual(hexBytes(vector.ciphertext))
      expect(Array.from(envelopeBytes)).toEqual(Array.from(hexBytes(vector.envelope)))
      expect(envelope).toMatchObject({
        version: 1,
        suite: vector.suite,
      })

      await expect(
        hpkeOpen({
          recipientPrivateKey,
          info,
          aad,
          envelope,
        }),
      ).resolves.toEqual(plaintext)
    } finally {
      randomSpy.mockRestore()
    }
  })

  it.each([
    ['browser fallback', hpkeVectors.legacy_javascript],
    ['Rust/WASM bridge', hpkeVectors.legacy_wasm],
  ])('opens the historical %s 0x0010 dialect without consulting WASM', async (_, vector) => {
    const inputs = hpkeVectors.rfc9180_appendix_a2
    const envelopeBytes = hexBytes(vector.envelope)
    const envelope = decodeHpkeEnvelope(envelopeBytes)

    expect(envelope.suite).toEqual(vector.suite)
    expect(envelope.enc).toEqual(hexBytes(vector.enc))
    expect(envelope.ciphertext).toEqual(hexBytes(vector.ciphertext))
    await expect(
      hpkeOpen({
        recipientPrivateKey: hexBytes(inputs.recipient_private_key),
        info: hexBytes(inputs.info),
        aad: hexBytes(inputs.aad),
        envelope: envelopeBytes,
      }),
    ).resolves.toEqual(hexBytes(inputs.plaintext))
    expect(bridgeMocks.getStrongBoxBridge).not.toHaveBeenCalled()
  })

  it('round-trips new envelopes using KEM 0x0020', async () => {
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
        kem: 0x0020,
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
    ).rejects.toThrow('ChaCha20-Poly1305 authentication failed.')
  })
})

function hexBytes(value: string): Uint8Array {
  if (value.length % 2 !== 0 || !/^[0-9a-f]*$/i.test(value)) {
    throw new Error('Fixture contains invalid hexadecimal data.')
  }
  return Uint8Array.from(value.match(/.{2}/g) ?? [], (byte) => Number.parseInt(byte, 16))
}

function mockRandomBytes(bytes: Uint8Array) {
  return vi.spyOn(globalThis.crypto, 'getRandomValues').mockImplementation((target) => {
    if (!(target instanceof Uint8Array) || target.length !== bytes.length) {
      throw new Error('Unexpected random byte request in HPKE vector test.')
    }
    target.set(bytes)
    return target
  })
}
