import { describe, expect, it } from 'vitest'

import { decodeBase64, encodeBase64 } from './base64'
import {
  constantTimeEquals,
  toArrayBuffer,
  toUint8Array,
  zeroBytes,
} from './bytes'
import { hkdfExpand } from './hkdf'
import { hmacSha256 } from './hmac'
import { randomBytes } from './random'
import { clampScalar } from './x25519'
import { x25519ScalarMult, x25519ScalarMultBase } from './x25519-fallback'

describe('runtime primitives', () => {
  it('preserves byte-copy and zeroization semantics', () => {
    const source = new Uint8Array([1, 2, 3])
    const copied = toUint8Array(source)
    source.fill(9)

    expect(copied).toEqual(new Uint8Array([1, 2, 3]))
    expect(new Uint8Array(toArrayBuffer(copied))).toEqual(copied)
    expect(constantTimeEquals(copied, new Uint8Array([1, 2, 3]))).toBe(true)
    expect(constantTimeEquals(copied, new Uint8Array([1, 2]))).toBe(false)
    zeroBytes(copied)
    expect(copied).toEqual(new Uint8Array(3))
  })

  it('round-trips unpadded URL-safe base64', () => {
    const bytes = new Uint8Array([251, 255, 239, 1])
    const encoded = encodeBase64(bytes)

    expect(encoded).not.toMatch(/=$/)
    expect(decodeBase64(encoded.replaceAll('+', '-').replaceAll('/', '_'))).toEqual(bytes)
  })

  it('uses WebCrypto secure randomness', () => {
    expect(randomBytes(32)).toHaveLength(32)
    expect(() => randomBytes(0)).toThrow(/greater than zero/i)
  })

  it('matches the RFC 5869 HKDF-SHA256 vector', async () => {
    const parent = new Uint8Array(22).fill(0x0b)
    const salt = Uint8Array.from([
      0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
      0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
    ])
    const info = Uint8Array.from([
      0xf0, 0xf1, 0xf2, 0xf3, 0xf4,
      0xf5, 0xf6, 0xf7, 0xf8, 0xf9,
    ])

    const derived = await hkdfExpand({ parent, salt, info, length: 32 })
    expect(bytesToHex(derived)).toBe(
      '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf',
    )
  })

  it('matches the RFC 4231 HMAC-SHA256 vector', async () => {
    const mac = await hmacSha256(
      new Uint8Array(20).fill(0x0b),
      new TextEncoder().encode('Hi There'),
    )
    expect(bytesToHex(mac)).toBe(
      'b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7',
    )
  })

  it('matches the production X25519 fallback reference vector', () => {
    const alice = clampScalar(
      hexToBytes('a546e36b389a1a706c147f1bf9d53e9798d7051fa25610c7d11d69860e652720'),
    )
    const bob = clampScalar(
      hexToBytes('4b66e9d4d1b40b172b928b2cc76c32101cf71f0f2c1942b47eb18ae0428d4230'),
    )
    const alicePublic = hexToBytes(
      'c0be51d006467bc40bf2d1f6fb9fa7e75b8a79e6ad741f096079b6abcc8ca676',
    )
    const bobPublic = hexToBytes(
      'a29ed84b9258f532dad080e90bab35643350970db7fbcf84557265b8d2af4e19',
    )
    const shared = hexToBytes(
      '2acc94fb2a20e487d2b253a0e7075c27e30250ec294900f8760ab515f805250e',
    )

    expect(x25519ScalarMultBase(alice)).toEqual(alicePublic)
    expect(x25519ScalarMultBase(bob)).toEqual(bobPublic)
    expect(x25519ScalarMult(alice, bobPublic)).toEqual(shared)
    expect(x25519ScalarMult(bob, alicePublic)).toEqual(shared)
  })
})

function bytesToHex(bytes: Uint8Array) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

function hexToBytes(hex: string) {
  return Uint8Array.from(
    { length: hex.length / 2 },
    (_, index) => Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16),
  )
}
