import { afterEach, describe, expect, it, vi } from 'vitest'

import { computeStrongBoxCacheKey, PlaintextCache } from './strong-box-cache'

describe('PlaintextCache', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('returns defensive copies and keeps live entries', () => {
    const cache = new PlaintextCache(2, 1_000)
    const plaintext = new Uint8Array([1, 2, 3])

    cache.set('entry', plaintext)
    plaintext.fill(9)

    const first = cache.get('entry')
    expect(first).toEqual(new Uint8Array([1, 2, 3]))
    first?.fill(8)
    expect(cache.get('entry')).toEqual(new Uint8Array([1, 2, 3]))
  })

  it('zeroizes plaintext on replacement, eviction, expiry, and clear', () => {
    vi.useFakeTimers()
    const zeroized: number[][] = []
    const cache = new PlaintextCache(2, 10, (value) => {
      value.fill(0)
      zeroized.push(Array.from(value))
    })

    cache.set('replace', new Uint8Array([1, 2]))
    cache.set('replace', new Uint8Array([3, 4]))
    cache.set('evict', new Uint8Array([5, 6]))
    cache.set('third', new Uint8Array([7, 8]))

    vi.advanceTimersByTime(11)
    expect(cache.get('evict')).toBeNull()
    cache.clear()

    expect(zeroized).toEqual([
      [0, 0],
      [0, 0],
      [0, 0],
      [0, 0],
    ])
  })
})

describe('computeStrongBoxCacheKey', () => {
  it('frames context and ciphertext lengths so boundary shifts cannot alias', async () => {
    const key = new Uint8Array(32).fill(7)
    const first = await computeStrongBoxCacheKey(
      key,
      new Uint8Array([1]),
      new Uint8Array([2, 3]),
    )
    const shifted = await computeStrongBoxCacheKey(
      key,
      new Uint8Array([1, 2]),
      new Uint8Array([3]),
    )

    expect(first).toMatch(/^[0-9a-f]{64}$/)
    expect(shifted).toMatch(/^[0-9a-f]{64}$/)
    expect(first).not.toBe(shifted)
  })

  it('disables caching instead of serializing raw secrets when digest is unavailable', async () => {
    const key = new Uint8Array([0xde, 0xad, 0xbe, 0xef])
    const rejectingDigest = {
      digest: vi.fn(() => Promise.reject(new Error('digest unavailable'))),
    }

    await expect(
      computeStrongBoxCacheKey(
        key,
        new Uint8Array([1]),
        new Uint8Array([2]),
        rejectingDigest,
      ),
    ).resolves.toBeNull()
    await expect(
      computeStrongBoxCacheKey(
        key,
        new Uint8Array([1]),
        new Uint8Array([2]),
        null,
      ),
    ).resolves.toBeNull()
  })
})
