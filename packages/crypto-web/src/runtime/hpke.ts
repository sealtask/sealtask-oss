import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import {
  constantTimeEquals,
  toArrayBuffer,
  toUint8Array,
  zeroBytes,
} from './bytes'
import { hmacSha256 } from './hmac'
import { randomBytes } from './random'
import { getStrongBoxBridge, type StrongBoxBridge } from './strong-box'
import { clampScalar, derivePublicKey, deriveSharedSecret } from './x25519'

const textEncoder = new TextEncoder()

const HASH_LENGTH = 32
const KEY_LENGTH = 32
const NONCE_LENGTH = 12
const MODE_BASE = 0x00
const KEM_ID = 0x0010
const KDF_ID = 0x0001
const AEAD_ID = 0x0003

const LABEL_PREFIX = textEncoder.encode('HPKE-v1')
const SUITE_ID = concatBytes(
  textEncoder.encode('HPKE'),
  i2osp(KEM_ID, 2),
  i2osp(KDF_ID, 2),
  i2osp(AEAD_ID, 2),
)

export type HpkeSealParams = {
  recipientPublicKey: Uint8Array
  info: Uint8Array
  aad: Uint8Array
  plaintext: Uint8Array
}

export type HpkeSealResult = {
  enc: Uint8Array
  ciphertext: Uint8Array
  nonce?: Uint8Array
}

export type HpkeEnvelope = {
  version: number
  suite: {
    kem: number
    kdf: number
    aead: number
    mode: number
  }
  enc: Uint8Array
  ciphertext: Uint8Array
}

export type HpkeOpenParams = {
  recipientPrivateKey: Uint8Array
  info: Uint8Array
  aad: Uint8Array
  envelope: Uint8Array | HpkeEnvelope
}

export async function hpkeSeal(params: HpkeSealParams): Promise<HpkeSealResult> {
  const bridge = await maybeGetHpkeBridge()
  if (bridge?.hpkeEncap) {
    const result = await bridge.hpkeEncap(params)
    return { enc: result.enc, ciphertext: result.ciphertext, nonce: result.nonce }
  }
  return hpkeSealPure(params)
}

export function encodeHpkeEnvelope(result: HpkeSealResult): Uint8Array {
  const payload = {
    version: 1,
    suite: {
      kem: KEM_ID,
      kdf: KDF_ID,
      aead: AEAD_ID,
      mode: MODE_BASE,
    },
    enc: result.enc,
    ciphertext: result.ciphertext,
  }
  return toUint8Array(cborEncode(payload))
}

export function decodeHpkeEnvelope(bytes: Uint8Array): HpkeEnvelope {
  const decoded = cborDecode(bytes) as Partial<HpkeEnvelope>
  return normalizeHpkeEnvelope(decoded)
}

export async function hpkeOpen(params: HpkeOpenParams): Promise<Uint8Array> {
  const bridge = await maybeGetHpkeBridge()
  if (bridge?.hpkeDecap) {
    const envelope =
      params.envelope instanceof Uint8Array
        ? decodeHpkeEnvelope(params.envelope)
        : normalizeHpkeEnvelope(params.envelope)

    return bridge.hpkeDecap({
      recipientPrivateKey: params.recipientPrivateKey,
      info: params.info,
      aad: params.aad,
      enc: envelope.enc,
      ciphertext: envelope.ciphertext,
    })
  }

  return hpkeOpenPure(params)
}

async function hpkeSealPure(params: HpkeSealParams): Promise<HpkeSealResult> {
  if (params.recipientPublicKey.length !== 32) {
    throw new Error('Recipient public key must be 32 bytes for HPKE.')
  }

  const ephemeralSeed = randomBytes(32)
  const privateKey = clampScalar(ephemeralSeed)
  const enc = await derivePublicKey(privateKey)
  const sharedSecret = await deriveSharedSecret({
    privateKey,
    peerPublicKey: params.recipientPublicKey,
  })
  zeroBytes(ephemeralSeed)
  zeroBytes(privateKey)

  if (sharedSecret.every((byte) => byte === 0)) {
    throw new Error('Derived HPKE shared secret is invalid.')
  }

  const kemContext = concatBytes(enc, params.recipientPublicKey)
  const keyScheduleContext = concatBytes(new Uint8Array([MODE_BASE]), kemContext, params.info)
  const secret = await labeledExtract(null, 'secret', sharedSecret)
  zeroBytes(sharedSecret)
  const [key, baseNonce] = await Promise.all([
    labeledExpand(secret, 'key', keyScheduleContext, KEY_LENGTH),
    labeledExpand(secret, 'base_nonce', keyScheduleContext, NONCE_LENGTH),
  ])

  const ciphertext = chacha20Poly1305Seal({
    key,
    nonce: baseNonce,
    aad: params.aad,
    plaintext: params.plaintext,
  })

  const nonceCopy = baseNonce.slice()
  zeroBytes(secret)
  zeroBytes(baseNonce)
  zeroBytes(key)

  return { enc, ciphertext, nonce: nonceCopy }
}

async function hpkeOpenPure(params: HpkeOpenParams): Promise<Uint8Array> {
  if (params.recipientPrivateKey.length !== 32) {
    throw new Error('Recipient private key must be 32 bytes for HPKE.')
  }
  const envelope =
    params.envelope instanceof Uint8Array
      ? decodeHpkeEnvelope(params.envelope)
      : normalizeHpkeEnvelope(params.envelope)

  const recipientPublicKey = await derivePublicKey(params.recipientPrivateKey)
  const sharedSecret = await deriveSharedSecret({
    privateKey: params.recipientPrivateKey,
    peerPublicKey: envelope.enc,
  })
  if (sharedSecret.every((byte) => byte === 0)) {
    zeroBytes(recipientPublicKey)
    zeroBytes(sharedSecret)
    throw new Error('Derived HPKE shared secret is invalid.')
  }

  const kemContext = concatBytes(envelope.enc, recipientPublicKey)
  zeroBytes(recipientPublicKey)
  const keyScheduleContext = concatBytes(new Uint8Array([MODE_BASE]), kemContext, params.info)
  const secret = await labeledExtract(null, 'secret', sharedSecret)
  zeroBytes(sharedSecret)
  const [key, baseNonce] = await Promise.all([
    labeledExpand(secret, 'key', keyScheduleContext, KEY_LENGTH),
    labeledExpand(secret, 'base_nonce', keyScheduleContext, NONCE_LENGTH),
  ])
  zeroBytes(secret)

  const plaintext = chacha20Poly1305Open({
    key,
    nonce: baseNonce,
    aad: params.aad,
    ciphertext: envelope.ciphertext,
  })

  zeroBytes(baseNonce)
  zeroBytes(key)

  return plaintext
}

export async function computeKeyFingerprint(publicKey: Uint8Array): Promise<Uint8Array> {
  if (publicKey.length === 0) {
    throw new Error('Public key is required to compute a fingerprint.')
  }
  return sha256(publicKey)
}

let hpkeBridgePromise: Promise<StrongBoxBridge | null> | null = null

async function maybeGetHpkeBridge(): Promise<StrongBoxBridge | null> {
  if (!hpkeBridgePromise) {
    const pendingBridge = (async () => {
      try {
        const bridge = await getStrongBoxBridge()
        if (typeof bridge.hpkeEncap === 'function' && typeof bridge.hpkeDecap === 'function') {
          return bridge
        }
      } catch {
        // Fall through to the protocol-compatible JS implementation.
      }
      return null
    })()
    hpkeBridgePromise = pendingBridge
    void pendingBridge.then((bridge) => {
      if (bridge === null && hpkeBridgePromise === pendingBridge) {
        hpkeBridgePromise = null
      }
    })
  }
  return hpkeBridgePromise
}

async function labeledExtract(
  salt: Uint8Array | null,
  label: string,
  ikm: Uint8Array,
): Promise<Uint8Array> {
  const labeledIkm = concatBytes(LABEL_PREFIX, SUITE_ID, textEncoder.encode(label), ikm)
  return hkdfExtract(salt, labeledIkm)
}

async function labeledExpand(
  prk: Uint8Array,
  label: string,
  info: Uint8Array,
  length: number,
): Promise<Uint8Array> {
  const labeledInfo = concatBytes(
    i2osp(length, 2),
    LABEL_PREFIX,
    SUITE_ID,
    textEncoder.encode(label),
    info,
  )
  return hkdfExpand(prk, labeledInfo, length)
}

async function hkdfExtract(salt: Uint8Array | null, ikm: Uint8Array): Promise<Uint8Array> {
  const keyMaterial = salt && salt.length > 0 ? salt : new Uint8Array(HASH_LENGTH)
  return hmacSha256(keyMaterial, ikm, 'WebCrypto subtle API is unavailable for HPKE.')
}

async function hkdfExpand(prk: Uint8Array, info: Uint8Array, length: number): Promise<Uint8Array> {
  const blocks = Math.ceil(length / HASH_LENGTH)
  const result = new Uint8Array(blocks * HASH_LENGTH)
  let previous: Uint8Array<ArrayBufferLike> = new Uint8Array(0)

  for (let counter = 1; counter <= blocks; counter += 1) {
    const buffer = new Uint8Array(previous.length + info.length + 1)
    buffer.set(previous, 0)
    buffer.set(info, previous.length)
    buffer[buffer.length - 1] = counter

    const block = await hmacSha256(prk, buffer, 'WebCrypto subtle API is unavailable for HPKE.')
    result.set(block, (counter - 1) * HASH_LENGTH)
    previous = block
  }

  return result.slice(0, length)
}

async function sha256(data: Uint8Array): Promise<Uint8Array> {
  const subtle = getSubtle()
  const digest = await subtle.digest('SHA-256', toArrayBuffer(data))
  return new Uint8Array(digest)
}

function getSubtle(): SubtleCrypto {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error('WebCrypto subtle API is unavailable for HPKE.')
  }
  return subtle
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0)
  const result = new Uint8Array(total)
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.length
  }
  return result
}

function i2osp(value: number, length: number): Uint8Array {
  if (value < 0 || value >= 1 << (length * 8)) {
    throw new Error(`Value ${value} does not fit in ${length} bytes.`)
  }
  const bytes = new Uint8Array(length)
  for (let index = length - 1; index >= 0; index -= 1) {
    bytes[index] = value & 0xff
    value >>>= 8
  }
  return bytes
}

function ensureUint8Array(value: unknown, field: string): Uint8Array {
  if (value instanceof Uint8Array) {
    return value
  }
  if (Array.isArray(value)) {
    return Uint8Array.from(value)
  }
  throw new Error(`HPKE envelope ${field} must be a byte array.`)
}

function normalizeHpkeEnvelope(envelope: HpkeEnvelope | Partial<HpkeEnvelope>): HpkeEnvelope {
  if (!envelope || typeof envelope !== 'object') {
    throw new Error('HPKE envelope is malformed.')
  }
  const { version, suite } = envelope
  if (version !== 1) {
    throw new Error(`Unsupported HPKE envelope version: ${String(version)}`)
  }
  if (!suite) {
    throw new Error('HPKE envelope missing suite definition.')
  }
  if (
    suite.kem !== KEM_ID ||
    suite.kdf !== KDF_ID ||
    suite.aead !== AEAD_ID ||
    suite.mode !== MODE_BASE
  ) {
    throw new Error('HPKE envelope uses an unsupported ciphersuite.')
  }
  return {
    version,
    suite,
    enc: ensureUint8Array(envelope.enc, 'enc'),
    ciphertext: ensureUint8Array(envelope.ciphertext, 'ciphertext'),
  }
}

function chacha20Poly1305Seal(params: {
  key: Uint8Array
  nonce: Uint8Array
  aad: Uint8Array
  plaintext: Uint8Array
}): Uint8Array {
  const { key, nonce, aad, plaintext } = params
  if (key.length !== KEY_LENGTH) {
    throw new Error('ChaCha20-Poly1305 key must be 32 bytes.')
  }
  if (nonce.length !== NONCE_LENGTH) {
    throw new Error('ChaCha20-Poly1305 nonce must be 12 bytes.')
  }

  const polyKey = chacha20Block(key, 0, nonce).slice(0, 32)
  const ciphertext = chacha20Xor(key, nonce, 1, plaintext)
  const tag = poly1305Authenticate(polyKey, aad, ciphertext)
  const sealed = new Uint8Array(ciphertext.length + tag.length)
  sealed.set(ciphertext, 0)
  sealed.set(tag, ciphertext.length)
  return sealed
}

function chacha20Poly1305Open(params: {
  key: Uint8Array
  nonce: Uint8Array
  aad: Uint8Array
  ciphertext: Uint8Array
}): Uint8Array {
  const { key, nonce, aad, ciphertext } = params
  if (ciphertext.length < 16) {
    throw new Error('ChaCha20-Poly1305 ciphertext must include an authentication tag.')
  }
  if (key.length !== KEY_LENGTH) {
    throw new Error('ChaCha20-Poly1305 key must be 32 bytes.')
  }
  if (nonce.length !== NONCE_LENGTH) {
    throw new Error('ChaCha20-Poly1305 nonce must be 12 bytes.')
  }

  const tagOffset = ciphertext.length - 16
  const payload = ciphertext.slice(0, tagOffset)
  const tag = ciphertext.slice(tagOffset)
  const polyKey = chacha20Block(key, 0, nonce).slice(0, 32)
  const expectedTag = poly1305Authenticate(polyKey, aad, payload)
  if (!constantTimeEquals(expectedTag, tag)) {
    throw new Error('ChaCha20-Poly1305 authentication failed.')
  }

  return chacha20Xor(key, nonce, 1, payload)
}

function chacha20Xor(
  key: Uint8Array,
  nonce: Uint8Array,
  counter: number,
  plaintext: Uint8Array,
): Uint8Array {
  const output = new Uint8Array(plaintext.length)
  const block = new Uint8Array(64)
  let ctr = counter >>> 0

  for (let offset = 0; offset < plaintext.length; offset += 64) {
    const keystream = chacha20Block(key, ctr, nonce, block)
    ctr = (ctr + 1) >>> 0
    const chunk = Math.min(64, plaintext.length - offset)
    for (let i = 0; i < chunk; i += 1) {
      output[offset + i] = plaintext[offset + i] ^ keystream[i]
    }
  }

  return output
}

function chacha20Block(
  key: Uint8Array,
  counter: number,
  nonce: Uint8Array,
  buffer?: Uint8Array,
): Uint8Array {
  const state = new Uint32Array(16)
  state[0] = 0x61707865
  state[1] = 0x3320646e
  state[2] = 0x79622d32
  state[3] = 0x6b206574

  for (let i = 0; i < 8; i += 1) {
    state[4 + i] = readUint32LE(key, i * 4)
  }

  state[12] = counter >>> 0
  state[13] = readUint32LE(nonce, 0)
  state[14] = readUint32LE(nonce, 4)
  state[15] = readUint32LE(nonce, 8)

  const working = state.slice()

  for (let round = 0; round < 10; round += 1) {
    quarterRound(working, 0, 4, 8, 12)
    quarterRound(working, 1, 5, 9, 13)
    quarterRound(working, 2, 6, 10, 14)
    quarterRound(working, 3, 7, 11, 15)
    quarterRound(working, 0, 5, 10, 15)
    quarterRound(working, 1, 6, 11, 12)
    quarterRound(working, 2, 7, 8, 13)
    quarterRound(working, 3, 4, 9, 14)
  }

  for (let i = 0; i < 16; i += 1) {
    working[i] = (working[i] + state[i]) >>> 0
  }

  const output = buffer ?? new Uint8Array(64)
  for (let i = 0; i < 16; i += 1) {
    writeUint32LE(output, i * 4, working[i])
  }
  return output
}

function quarterRound(state: Uint32Array, a: number, b: number, c: number, d: number) {
  state[a] = (state[a] + state[b]) >>> 0
  state[d] ^= state[a]
  state[d] = rotateLeft(state[d], 16)

  state[c] = (state[c] + state[d]) >>> 0
  state[b] ^= state[c]
  state[b] = rotateLeft(state[b], 12)

  state[a] = (state[a] + state[b]) >>> 0
  state[d] ^= state[a]
  state[d] = rotateLeft(state[d], 8)

  state[c] = (state[c] + state[d]) >>> 0
  state[b] ^= state[c]
  state[b] = rotateLeft(state[b], 7)
}

function rotateLeft(value: number, count: number): number {
  return ((value << count) | (value >>> (32 - count))) >>> 0
}

function readUint32LE(bytes: Uint8Array, offset: number): number {
  return (
    bytes[offset] |
    (bytes[offset + 1] << 8) |
    (bytes[offset + 2] << 16) |
    (bytes[offset + 3] << 24)
  ) >>> 0
}

function writeUint32LE(target: Uint8Array, offset: number, value: number) {
  target[offset] = value & 0xff
  target[offset + 1] = (value >>> 8) & 0xff
  target[offset + 2] = (value >>> 16) & 0xff
  target[offset + 3] = (value >>> 24) & 0xff
}

function poly1305Authenticate(key: Uint8Array, aad: Uint8Array, ciphertext: Uint8Array): Uint8Array {
  if (key.length !== 32) {
    throw new Error('Poly1305 key must be 32 bytes.')
  }
  const r = clampPolyKey(leBytesToBigInt(key, 0, 16))
  const s = leBytesToBigInt(key, 16, 16)
  const prime = (1n << 130n) - 5n
  let acc = 0n

  acc = poly1305Accumulate(acc, r, aad, prime)
  acc = poly1305Pad(acc, r, aad.length, prime)
  acc = poly1305Accumulate(acc, r, ciphertext, prime)
  acc = poly1305Pad(acc, r, ciphertext.length, prime)

  const lengthBlock = new Uint8Array(16)
  writeUint64LE(lengthBlock, 0, BigInt(aad.length))
  writeUint64LE(lengthBlock, 8, BigInt(ciphertext.length))
  acc = poly1305Accumulate(acc, r, lengthBlock, prime)

  const tagValue = (acc + s) % (1n << 128n)
  return bigIntToBytes(tagValue, 16)
}

function clampPolyKey(value: bigint): bigint {
  const mask = BigInt('0x0ffffffc0ffffffc0ffffffc0fffffff')
  return value & mask
}

function poly1305Accumulate(
  acc: bigint,
  r: bigint,
  data: Uint8Array,
  prime: bigint,
): bigint {
  for (let offset = 0; offset < data.length; offset += 16) {
    const chunk = Math.min(16, data.length - offset)
    const blockValue = leBytesToBigInt(data, offset, chunk) + (1n << BigInt(8 * chunk))
    acc = (acc + blockValue) % prime
    acc = (acc * r) % prime
  }
  return acc
}

function poly1305Pad(acc: bigint, r: bigint, length: number, prime: bigint): bigint {
  const remainder = length % 16
  if (remainder === 0) {
    return acc
  }
  const padding = new Uint8Array(16 - remainder)
  return poly1305Accumulate(acc, r, padding, prime)
}

function leBytesToBigInt(bytes: Uint8Array, offset: number, length: number): bigint {
  let value = 0n
  for (let i = 0; i < length; i += 1) {
    value += BigInt(bytes[offset + i] ?? 0) << BigInt(8 * i)
  }
  return value
}

function bigIntToBytes(value: bigint, length: number): Uint8Array {
  const result = new Uint8Array(length)
  for (let i = 0; i < length; i += 1) {
    result[i] = Number((value >> BigInt(8 * i)) & 0xffn)
  }
  return result
}

function writeUint64LE(target: Uint8Array, offset: number, value: bigint) {
  for (let i = 0; i < 8; i += 1) {
    target[offset + i] = Number((value >> BigInt(8 * i)) & 0xffn)
  }
}
