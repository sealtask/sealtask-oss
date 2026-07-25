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
