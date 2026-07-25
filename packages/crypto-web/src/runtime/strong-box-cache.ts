import { toArrayBuffer } from './bytes'

type CacheEntry = {
  value: Uint8Array
  expiresAt: number
}

type Zeroize = (value: Uint8Array) => void

const defaultZeroize: Zeroize = (value) => value.fill(0)

export class PlaintextCache {
  private readonly entries = new Map<string, CacheEntry>()
  private readonly maxEntries: number
  private readonly ttlMs: number
  private readonly zeroize: Zeroize

  constructor(maxEntries = 64, ttlMs = 120_000, zeroize: Zeroize = defaultZeroize) {
    this.maxEntries = maxEntries
    this.ttlMs = ttlMs
    this.zeroize = zeroize
  }

  get(key: string): Uint8Array | null {
    const entry = this.entries.get(key)
    if (!entry) {
      return null
    }

    const now = Date.now()
    if (entry.expiresAt <= now) {
      this.deleteEntry(key, entry)
      return null
    }

    this.entries.delete(key)
    this.entries.set(key, { value: entry.value, expiresAt: now + this.ttlMs })
    return entry.value.slice()
  }

  set(key: string, value: Uint8Array) {
    const existing = this.entries.get(key)
    if (existing) {
      this.deleteEntry(key, existing)
    }

    this.entries.set(key, {
      value: value.slice(),
      expiresAt: Date.now() + this.ttlMs,
    })

    if (this.entries.size > this.maxEntries) {
      this.evict()
    }
  }

  clear() {
    for (const entry of this.entries.values()) {
      this.zeroize(entry.value)
    }
    this.entries.clear()
  }

  private deleteEntry(key: string, entry: CacheEntry) {
    this.entries.delete(key)
    this.zeroize(entry.value)
  }

  private evict() {
    const iterator = this.entries.entries().next()
    if (!iterator.done) {
      const [key, entry] = iterator.value
      this.deleteEntry(key, entry)
    }
  }
}

/**
 * Returns a framed SHA-256 cache key, or null when SHA-256 is unavailable.
 *
 * Framing the three variable-length fields prevents a context/ciphertext
 * boundary shift from aliasing a previous authenticated decryption. A failed
 * digest deliberately disables caching instead of retaining raw key material
 * in a JavaScript string.
 */
export async function computeStrongBoxCacheKey(
  key: Uint8Array,
  context: Uint8Array,
  payload: Uint8Array,
  subtle: Pick<SubtleCrypto, 'digest'> | null | undefined = globalThis.crypto?.subtle,
): Promise<string | null> {
  if (!subtle) {
    return null
  }

  const headerBytes = 12
  const framed = new Uint8Array(headerBytes + key.length + context.length + payload.length)
  const view = new DataView(framed.buffer)
  view.setUint32(0, key.length, true)
  view.setUint32(4, context.length, true)
  view.setUint32(8, payload.length, true)
  framed.set(key, headerBytes)
  framed.set(context, headerBytes + key.length)
  framed.set(payload, headerBytes + key.length + context.length)

  try {
    const digest = await subtle.digest('SHA-256', toArrayBuffer(framed))
    return bytesToHex(new Uint8Array(digest))
  } catch {
    return null
  } finally {
    framed.fill(0)
  }
}

function bytesToHex(bytes: Uint8Array) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
