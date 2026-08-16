import { describe, expect, it } from 'vitest'

import {
  readStrictCborHeader,
  readStrictCborText,
  readStrictCborTextMapKeys,
  skipStrictCborItem,
} from './strict-cbor'

describe('strict CBOR reader', () => {
  it('reads bounded definite-length headers without imposing a byte encoding', () => {
    expect(
      readStrictCborHeader(Uint8Array.of(0x1a, 0x00, 0x00, 0x01, 0x00), 0),
    ).toEqual({ majorType: 0, argument: 256, nextOffset: 5 })
    expect(
      readStrictCborHeader(
        Uint8Array.of(
          0x1b,
          0x00,
          0x1f,
          0xff,
          0xff,
          0xff,
          0xff,
          0xff,
          0xff,
        ),
        0,
      ),
    ).toEqual({
      majorType: 0,
      argument: Number.MAX_SAFE_INTEGER,
      nextOffset: 9,
    })

    // The protocol accepts a compatible writer's non-minimal definite form.
    expect(readStrictCborHeader(Uint8Array.of(0x18, 0x01), 0)).toEqual({
      majorType: 0,
      argument: 1,
      nextOffset: 2,
    })
  })

  it('rejects truncated, indefinite, reserved, and unsafe headers', () => {
    expect(() => readStrictCborHeader(Uint8Array.of(0x00), -1)).toThrow(
      'CBOR offset must be a non-negative safe integer',
    )
    expect(() => readStrictCborHeader(Uint8Array.of(0x00), 0.5)).toThrow(
      'CBOR offset must be a non-negative safe integer',
    )
    expect(() => readStrictCborHeader(new Uint8Array(), 0)).toThrow(
      'truncated CBOR header',
    )
    expect(() => readStrictCborHeader(Uint8Array.of(0x18), 0)).toThrow(
      'unsupported or truncated CBOR argument',
    )
    expect(() => readStrictCborHeader(Uint8Array.of(0x1c), 0)).toThrow(
      'unsupported or truncated CBOR argument',
    )
    expect(() => readStrictCborHeader(Uint8Array.of(0x1f), 0)).toThrow(
      'unsupported or truncated CBOR argument',
    )
    expect(() =>
      readStrictCborHeader(
        Uint8Array.of(
          0x1b,
          0x00,
          0x20,
          0x00,
          0x00,
          0x00,
          0x00,
          0x00,
          0x00,
        ),
        0,
      ),
    ).toThrow('CBOR argument exceeds the safe integer range')
  })

  it('reads only complete, valid UTF-8 text keys', () => {
    expect(readStrictCborText(Uint8Array.of(0x62, 0x6f, 0x6b), 0)).toEqual({
      value: 'ok',
      nextOffset: 3,
    })
    expect(() => readStrictCborText(Uint8Array.of(0x41, 0x01), 0)).toThrow(
      'CBOR map key is not text',
    )
    expect(() => readStrictCborText(Uint8Array.of(0x62, 0x6f), 0)).toThrow(
      'truncated CBOR text',
    )
    expect(() => readStrictCborText(Uint8Array.of(0x61, 0xff), 0)).toThrow()
  })

  it('walks only the strict definite CBOR subset', () => {
    const value = Uint8Array.of(
      0xa1,
      0x61,
      0x61,
      0x82,
      0x01,
      0x20,
    )
    expect(skipStrictCborItem(value, 0, { maxDepth: 2 })).toBe(
      value.byteLength,
    )
    expect(skipStrictCborItem(Uint8Array.of(0x20), 0, { maxDepth: 0 })).toBe(1)
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0x42, 0x01), 0, { maxDepth: 0 }),
    ).toThrow('truncated CBOR string')
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0x9f, 0x01, 0xff), 0, { maxDepth: 0 }),
    ).toThrow('unsupported or truncated CBOR argument')
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0xc0, 0x01), 0, { maxDepth: 0 }),
    ).toThrow('CBOR tags are not supported')
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0xf9, 0x3c, 0x00), 0, { maxDepth: 0 }),
    ).toThrow('CBOR simple values and floats are not supported')
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0xf4), 0, { maxDepth: 0 }),
    ).toThrow('CBOR simple values and floats are not supported')
  })

  it('applies the caller-owned nesting policy at the boundary', () => {
    expect(
      skipStrictCborItem(nestedArrays(2), 0, { maxDepth: 2 }),
    ).toBe(3)
    expect(() =>
      skipStrictCborItem(nestedArrays(3), 0, { maxDepth: 2 }),
    ).toThrow('CBOR nesting is too deep')
    expect(() =>
      skipStrictCborItem(Uint8Array.of(0x00), 0, { maxDepth: -1 }),
    ).toThrow('CBOR max depth must be a non-negative safe integer')
  })

  it('returns unique text map keys only when the entire map is consumed', () => {
    const mapWithNestedValue = Uint8Array.of(
      0xa2,
      0x61,
      0x61,
      0x01,
      0x61,
      0x62,
      0x82,
      0x02,
      0x03,
    )
    expect(readStrictCborTextMapKeys(mapWithNestedValue, { maxDepth: 2 }))
      .toEqual(['a', 'b'])
    expect(() =>
      readStrictCborTextMapKeys(mapWithNestedValue, { maxDepth: 1 }),
    ).toThrow('CBOR nesting is too deep')
    expect(() =>
      readStrictCborTextMapKeys(Uint8Array.of(0xa1, 0x61, 0x61, 0x01), {
        maxDepth: 0,
      }),
    ).toThrow('CBOR nesting is too deep')
    expect(() =>
      readStrictCborTextMapKeys(
        Uint8Array.of(0xa2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02),
        { maxDepth: 1 },
      ),
    ).toThrow('duplicate top-level map key')
    expect(() =>
      readStrictCborTextMapKeys(Uint8Array.of(0xa1, 0x01, 0x01), {
        maxDepth: 1,
      }),
    ).toThrow('CBOR map key is not text')
    expect(() =>
      readStrictCborTextMapKeys(Uint8Array.of(0xa1, 0x61, 0x61, 0x01, 0x00), {
        maxDepth: 1,
      }),
    ).toThrow('trailing bytes')
  })
})

function nestedArrays(depth: number): Uint8Array {
  return Uint8Array.of(...Array<number>(depth).fill(0x81), 0x00)
}
