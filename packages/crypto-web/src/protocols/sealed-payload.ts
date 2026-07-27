import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'

export type SealedPayload<V extends number = number> = {
  version: V
  ciphertext: Uint8Array
}

type SealedPayloadRecord = {
  version: number
  ciphertext: unknown
}

type DefaultSealedPayloadVersion = typeof SEALED_PAYLOAD_VERSION
type AllowedVersions<V extends number = number> = readonly V[]

function assertSealedPayload(value: unknown): asserts value is SealedPayloadRecord {
  if (
    !value ||
    typeof value !== 'object' ||
    typeof (value as SealedPayloadRecord).version !== 'number' ||
    !(value as SealedPayloadRecord).ciphertext
  ) {
    throw new Error('Invalid sealed payload structure')
  }
}

export function parseSealedPayloadBytes(bytes: Uint8Array): SealedPayload {
  if (!isCborMap(bytes)) {
    throw new Error('Invalid sealed payload structure')
  }
  const decoded = cborDecode(bytes) as unknown
  assertSealedPayload(decoded)

  const ciphertext = decoded.ciphertext
  const normalizedCiphertext = Array.isArray(ciphertext)
    ? new Uint8Array(ciphertext)
    : ciphertext
  if (!(normalizedCiphertext instanceof Uint8Array)) {
    throw new Error('Ciphertext must be an Array or Uint8Array')
  }

  return {
    version: decoded.version,
    ciphertext: normalizedCiphertext,
  }
}

function isCborMap(bytes: Uint8Array): boolean {
  return bytes.length > 0 && (bytes[0] & 0xe0) === 0xa0
}

export function parseSealedPayload(
  base64Value: string,
): SealedPayload<DefaultSealedPayloadVersion>
export function parseSealedPayload<V extends number>(
  base64Value: string,
  allowedVersions: AllowedVersions<V>,
): SealedPayload<V>
export function parseSealedPayload(
  base64Value: string,
  allowedVersions?: AllowedVersions,
): SealedPayload {
  const bytes = decodeBase64(base64Value)
  const payload = parseSealedPayloadBytes(bytes)
  if (allowedVersions) {
    validateSealedPayload(payload, allowedVersions)
  } else {
    validateSealedPayload(payload)
  }
  return payload
}

export function parseStrictSealedPayload(
  base64Value: string,
): SealedPayload<DefaultSealedPayloadVersion> {
  const bytes = decodeBase64(base64Value)
  assertExactSealedPayloadMap(bytes)
  return parseSealedPayload(base64Value)
}

export function validateSealedPayload(
  payload: SealedPayload,
): asserts payload is SealedPayload<DefaultSealedPayloadVersion>
export function validateSealedPayload<V extends number>(
  payload: SealedPayload,
  allowedVersions: AllowedVersions<V>,
): asserts payload is SealedPayload<V>
export function validateSealedPayload(
  payload: SealedPayload,
  allowedVersions: AllowedVersions = [SEALED_PAYLOAD_VERSION],
): void {
  if (!allowedVersions.includes(payload.version)) {
    throw new Error(`Unsupported sealed payload version: ${payload.version}`)
  }
  if (!(payload.ciphertext instanceof Uint8Array) || payload.ciphertext.length === 0) {
    throw new Error('Ciphertext must be a non-empty Uint8Array')
  }
}

export function serializeSealedPayload<V extends number>(
  payload: SealedPayload<V>,
  allowedVersions?: AllowedVersions<V>,
): Uint8Array {
  if (allowedVersions) {
    validateSealedPayload(payload, allowedVersions)
  } else {
    validateSealedPayload(payload)
  }
  return cborEncode(payload)
}

export function serializeSealedPayloadBase64<V extends number>(
  payload: SealedPayload<V>,
  allowedVersions?: AllowedVersions<V>,
): string {
  return encodeBase64(serializeSealedPayload(payload, allowedVersions))
}

type CborHeader = {
  majorType: number
  argument: number
  nextOffset: number
}

function assertExactSealedPayloadMap(bytes: Uint8Array): void {
  try {
    const header = readCborHeader(bytes, 0)
    if (header.majorType !== 5 || header.argument !== 2) {
      throw new Error('sealed payload must be a two-field map')
    }
    const seen = new Set<string>()
    let offset = header.nextOffset
    for (let index = 0; index < header.argument; index += 1) {
      const key = readCborText(bytes, offset)
      if (
        (key.value !== 'version' && key.value !== 'ciphertext') ||
        seen.has(key.value)
      ) {
        throw new Error('sealed payload map keys are invalid')
      }
      seen.add(key.value)
      offset = skipCborItem(bytes, key.nextOffset, 0)
    }
    if (
      offset !== bytes.byteLength ||
      !seen.has('version') ||
      !seen.has('ciphertext')
    ) {
      throw new Error('sealed payload map is incomplete')
    }
  } catch {
    throw new Error('Invalid strict sealed payload structure')
  }
}

function readCborHeader(bytes: Uint8Array, offset: number): CborHeader {
  if (offset >= bytes.byteLength) {
    throw new Error('truncated CBOR header')
  }
  const initial = bytes[offset]
  const majorType = initial >>> 5
  const additional = initial & 0x1f
  if (additional < 24) {
    return { majorType, argument: additional, nextOffset: offset + 1 }
  }
  const argumentBytes =
    additional === 24
      ? 1
      : additional === 25
        ? 2
        : additional === 26
          ? 4
          : additional === 27
            ? 8
            : 0
  if (argumentBytes === 0 || offset + 1 + argumentBytes > bytes.byteLength) {
    throw new Error('unsupported or truncated CBOR argument')
  }
  let argument = 0n
  for (let index = 0; index < argumentBytes; index += 1) {
    argument =
      (argument << 8n) | BigInt(bytes[offset + 1 + index])
  }
  if (argument > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error('CBOR argument exceeds the safe integer range')
  }
  return {
    majorType,
    argument: Number(argument),
    nextOffset: offset + 1 + argumentBytes,
  }
}

function readCborText(
  bytes: Uint8Array,
  offset: number,
): { value: string; nextOffset: number } {
  const header = readCborHeader(bytes, offset)
  if (header.majorType !== 3) {
    throw new Error('CBOR map key is not text')
  }
  const nextOffset = header.nextOffset + header.argument
  if (nextOffset > bytes.byteLength) {
    throw new Error('truncated CBOR text')
  }
  return {
    value: new TextDecoder('utf-8', { fatal: true }).decode(
      bytes.subarray(header.nextOffset, nextOffset),
    ),
    nextOffset,
  }
}

function skipCborItem(
  bytes: Uint8Array,
  offset: number,
  depth: number,
): number {
  if (depth > 8) {
    throw new Error('CBOR nesting is too deep')
  }
  const header = readCborHeader(bytes, offset)
  switch (header.majorType) {
    case 0:
    case 1:
    case 7:
      return header.nextOffset
    case 2:
    case 3: {
      const nextOffset = header.nextOffset + header.argument
      if (nextOffset > bytes.byteLength) {
        throw new Error('truncated CBOR string')
      }
      return nextOffset
    }
    case 4: {
      let nextOffset = header.nextOffset
      for (let index = 0; index < header.argument; index += 1) {
        nextOffset = skipCborItem(bytes, nextOffset, depth + 1)
      }
      return nextOffset
    }
    case 5: {
      let nextOffset = header.nextOffset
      for (let index = 0; index < header.argument * 2; index += 1) {
        nextOffset = skipCborItem(bytes, nextOffset, depth + 1)
      }
      return nextOffset
    }
    case 6:
      return skipCborItem(bytes, header.nextOffset, depth + 1)
    default:
      throw new Error('unsupported CBOR major type')
  }
}
