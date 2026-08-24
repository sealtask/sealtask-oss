import { describe, expect, it } from 'vitest'

import {
  concatBytes,
  encodeUint32,
  encodeUint64,
  sha256,
} from './transparency-wire'

describe('transparency wire primitives', () => {
  it('uses big-endian unsigned integer framing at protocol boundaries', () => {
    expect(toHex(encodeUint32(0xffffffff, 'length'))).toBe('ffffffff')
    expect(toHex(encodeUint64(Number.MAX_SAFE_INTEGER, 'generation'))).toBe(
      '001fffffffffffff',
    )
    expect(() => encodeUint32(0x1_0000_0000, 'length'))
      .toThrow('length must be a non-negative 32-bit integer')
    expect(() => encodeUint64(-1, 'generation'))
      .toThrow('generation must be a non-negative safe integer')
  })

  it('copies concatenated fragments and hashes only a Uint8Array view range', async () => {
    const first = Uint8Array.of(0x01, 0x02)
    const combined = concatBytes(first, Uint8Array.of(0x03))
    first.fill(0xff)

    expect(combined).toEqual(Uint8Array.of(0x01, 0x02, 0x03))

    const surroundingBytes = Uint8Array.of(0xff, 0x61, 0x62, 0x63, 0xff)
    const abc = surroundingBytes.subarray(1, 4)
    expect(toHex(await sha256(abc, 'WebCrypto unavailable'))).toBe(
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
    )
  })
})

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
