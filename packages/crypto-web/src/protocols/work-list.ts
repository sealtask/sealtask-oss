import {
  decode as cborDecode,
  encode as cborEncode,
  Encoder,
} from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { toUint8Array, zeroBytes } from '../runtime/bytes'
import {
  KEY_SIZE_BYTES,
  SEALED_PAYLOAD_VERSION,
} from '../runtime/constants'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  decodeAndValidatePayloadBytes,
  validatePayloadBytes,
  type PayloadValidationDependencies,
} from './payload-validation'
import {
  PROJECT_EMOJI_POLICY,
  type ProjectEmojiPolicy,
} from './project-emoji'
import { parseSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import type { SealedBlobPayload } from './types'

const encoder = new TextEncoder()
const strictUtf8Decoder = new TextDecoder('utf-8', { fatal: true })
const projectKeyEnvelopeEncoder = new Encoder({
  tagUint8Array: false,
  useRecords: false,
})
const PROJECT_KEY_ENVELOPE_KIND = 'sealtask-project-key'
const PROJECT_KEY_ENVELOPE_VERSION = 2
const PROJECT_KEY_CBOR_MAX_BYTES = 512
const WORK_LIST_PAYLOAD_CONTEXT = encoder.encode('worklist.work_list.v1')
const WORK_LIST_MEMBERSHIP_CONTEXT = encoder.encode('worklist.membership')
const TEXT_VALUE_CONTEXTS = {
  workListTitle: encoder.encode('worklist.work_list.title.v1'),
  workListDescription: encoder.encode(
    'worklist.work_list.description.v1',
  ),
  taskTitle: encoder.encode('worklist.task.title.v1'),
  noteTitle: encoder.encode('worklist.note.title.v1'),
} as const

export type TextValueContext = keyof typeof TEXT_VALUE_CONTEXTS

/**
 * Plaintext encoding used inside a membership-key StrongBox ciphertext.
 *
 * `legacy-raw` remains the safe default until deployments have excluded every
 * previously shipped client that interprets decrypted membership plaintext as
 * the project key directly. `v2-envelope` must only be selected after that
 * fleet-wide rollout gate has been confirmed.
 */
export type ProjectKeyWriteFormat = 'legacy-raw' | 'v2-envelope'

export type WorkListPayloadEnvelope = {
  kind: 'work_list'
  version: number
  body: Record<string, unknown>
}

export async function deriveWorkListKey(params: {
  dataKey: Uint8Array
  workListId: string
}): Promise<Uint8Array> {
  return hkdfExpand({
    parent: params.dataKey,
    info: `worklist:${params.workListId}`,
  })
}

export async function derivePayloadBindingKey(params: {
  listKey: Uint8Array
}): Promise<Uint8Array> {
  return hkdfExpand({
    parent: params.listKey,
    info: 'member:payload-binding',
  })
}

export async function resolveWorkListKey(params: {
  workListId: string
  membershipCiphertext: string | null
  dataKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<Uint8Array> {
  if (
    params.membershipCiphertext &&
    params.membershipCiphertext.trim().length > 0
  ) {
    return decryptWorkListKeyCiphertext({
      ciphertext: params.membershipCiphertext,
      dataKey: params.dataKey,
      strongBox: params.strongBox,
    })
  }
  return deriveWorkListKey({
    dataKey: params.dataKey,
    workListId: params.workListId,
  })
}

export async function decryptWorkListPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<WorkListPayloadEnvelope> {
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: WORK_LIST_PAYLOAD_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  const envelope = decodeAndValidatePayloadBytes(
    plaintext,
    'work_list',
    params.validation,
  )
  return {
    kind: envelope.kind,
    version: envelope.version,
    body: envelope.body as Record<string, unknown>,
  }
}

export async function encryptWorkListPayload(params: {
  envelope: WorkListPayloadEnvelope
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<SealedBlobPayload> {
  const plaintext = toUint8Array(cborEncode(params.envelope))
  validatePayloadBytes(plaintext, 'work_list', params.validation)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: WORK_LIST_PAYLOAD_CONTEXT,
    plaintext,
  })
  return toSealedBlob({ version: SEALED_PAYLOAD_VERSION, ciphertext })
}

export async function sealWorkListKeyForOwner(params: {
  listKey: Uint8Array
  dataKey: Uint8Array
  strongBox?: StrongBoxBridge
  projectKeyWriteFormat?: ProjectKeyWriteFormat
}): Promise<SealedBlobPayload> {
  const projectKey = requireProjectKey(params.listKey)
  let encodedEnvelope: Uint8Array | undefined
  let plaintext: Uint8Array | undefined
  try {
    const bridge = params.strongBox ?? (await getStrongBoxBridge())
    const writeFormat = params.projectKeyWriteFormat ?? 'legacy-raw'
    if (writeFormat === 'legacy-raw') {
      plaintext = projectKey
    } else if (writeFormat === 'v2-envelope') {
      encodedEnvelope = projectKeyEnvelopeEncoder.encode({
        kind: PROJECT_KEY_ENVELOPE_KIND,
        version: PROJECT_KEY_ENVELOPE_VERSION,
        key: projectKey,
      })
      plaintext = toUint8Array(encodedEnvelope)
    } else {
      throw new Error('Unsupported project key write format')
    }
    const ciphertext = await bridge.encrypt({
      key: params.dataKey,
      context: WORK_LIST_MEMBERSHIP_CONTEXT,
      plaintext,
    })
    return toSealedBlob({
      version: SEALED_PAYLOAD_VERSION,
      ciphertext,
    })
  } finally {
    zeroBytes(projectKey)
    zeroBytes(plaintext)
    zeroBytes(encodedEnvelope)
  }
}

export async function decryptWorkListKeyCiphertext(params: {
  ciphertext: string
  dataKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<Uint8Array> {
  if (!params.ciphertext) {
    throw new Error('Project key ciphertext is required')
  }
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.dataKey,
    context: WORK_LIST_MEMBERSHIP_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  try {
    return extractWorkListKeyFromPlaintext(plaintext)
  } finally {
    zeroBytes(plaintext)
  }
}

export async function sealTextValue(params: {
  value: string
  key: Uint8Array
  context: TextValueContext
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = toUint8Array(cborEncode({ value: params.value }))
  const ciphertext = await bridge.encrypt({
    key: params.key,
    context: TEXT_VALUE_CONTEXTS[params.context],
    plaintext,
  })
  return toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
}

export async function decryptTextValue(params: {
  ciphertext: string
  key: Uint8Array
  context: TextValueContext
  strongBox?: StrongBoxBridge
}): Promise<string> {
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.key,
    context: TEXT_VALUE_CONTEXTS[params.context],
    ciphertext: sealed.ciphertext,
  })
  const decoded = cborDecode(plaintext) as unknown
  if (
    !isRecord(decoded) ||
    !hasExactlyKeys(decoded, ['value']) ||
    typeof decoded.value !== 'string'
  ) {
    throw new Error('Invalid text value payload')
  }
  return decoded.value
}

export async function sealOptionalText(params: {
  value: string | null | undefined
  key: Uint8Array
  context: TextValueContext
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload | null> {
  if (
    params.value === null ||
    params.value === undefined ||
    params.value.trim().length === 0
  ) {
    return null
  }
  return sealTextValue({
    value: params.value,
    key: params.key,
    context: params.context,
    strongBox: params.strongBox,
  })
}

export async function computePayloadProof(params: {
  ciphertext: Uint8Array
  bindingKey: Uint8Array
}): Promise<string> {
  const mac = await hmacSha256(
    toUint8Array(params.bindingKey),
    toUint8Array(params.ciphertext),
    'WebCrypto HMAC is unavailable',
  )
  return encodeBase64(mac)
}

export function clonePayloadBody(
  envelope: WorkListPayloadEnvelope,
): Record<string, unknown> {
  return isRecord(envelope?.body) ? structuredClone(envelope.body) : {}
}

export function rebuildEnvelope(
  body: Record<string, unknown>,
  version = 1,
): WorkListPayloadEnvelope {
  return { kind: 'work_list', version, body }
}

export function rebuildDecryptedEnvelopeForWrite(
  original: WorkListPayloadEnvelope,
  nextBody: Record<string, unknown>,
  projectEmoji: ProjectEmojiPolicy = PROJECT_EMOJI_POLICY,
): WorkListPayloadEnvelope {
  const body = structuredClone(nextBody)
  const originalTheme = isRecord(original.body.theme)
    ? original.body.theme
    : null
  const nextTheme = isRecord(body.theme) ? body.theme : null
  const originalEmoji = originalTheme?.emoji
  if (
    nextTheme &&
    nextTheme.emoji === originalEmoji &&
    projectEmoji.isLegacyReadable(originalEmoji) &&
    projectEmoji.getValidationError(originalEmoji) !== null
  ) {
    delete nextTheme.emoji
  }
  return rebuildEnvelope(body, original.version ?? 1)
}

function extractWorkListKeyFromPlaintext(
  plaintext: Uint8Array,
): Uint8Array {
  // A legacy raw key can itself be valid CBOR. In particular, a key
  // beginning with 0x58 0x1e looks like a 30-byte CBOR byte string, so raw
  // 32-byte plaintext must win before attempting any structural decode.
  if (plaintext.length === KEY_SIZE_BYTES) {
    return toUint8Array(plaintext)
  }
  if (plaintext.length > PROJECT_KEY_CBOR_MAX_BYTES) {
    throw new Error('Invalid project key payload')
  }

  let projectKey: Uint8Array
  try {
    projectKey = decodeProjectKeyCbor(plaintext)
  } catch {
    throw new Error('Invalid project key payload')
  }
  try {
    return requireProjectKey(projectKey)
  } finally {
    zeroBytes(projectKey)
  }
}

type ProjectKeyCborValue =
  | { kind: 'bytes'; value: Uint8Array; tagged: boolean }
  | { kind: 'legacy-array'; value: Uint8Array; tagged: false }
  | { kind: 'text'; value: string; tagged: false }

function decodeProjectKeyCbor(plaintext: Uint8Array): Uint8Array {
  const reader = new ProjectKeyCborReader(plaintext)
  if (
    reader.nextMajorType() === 2
    || reader.nextMajorType() === 4
    || reader.nextIsTag(64)
  ) {
    const value = reader.readProjectKeyValue()
    reader.requireEnd()
    if (value.kind === 'text') {
      throw new Error('Invalid project key payload')
    }
    return value.value
  }

  const entryCount = reader.readMapLength()
  const fields = new Map<string, string | number | ProjectKeyCborValue>()
  for (let entriesRead = 0; entriesRead < entryCount; entriesRead += 1) {
    if (entriesRead >= 3) {
      throw new Error('Invalid project key envelope')
    }
    const name = reader.readText()
    if (fields.has(name)) {
      throw new Error('Invalid project key envelope')
    }
    switch (name) {
      case 'kind':
        fields.set(name, reader.readText())
        break
      case 'version':
        fields.set(name, reader.readUnsignedInteger())
        break
      case 'key':
        fields.set(name, reader.readProjectKeyValue())
        break
      default:
        throw new Error('Invalid project key envelope')
    }
  }
  reader.requireEnd()

  if (fields.size === 1 && fields.has('key')) {
    const key = fields.get('key')
    if (!key || typeof key === 'string' || typeof key === 'number') {
      throw new Error('Invalid project key payload')
    }
    if (key.kind === 'text') {
      return decodeLegacyProjectKeyString(key.value)
    }
    return key.value
  }

  if (fields.size !== 3) {
    throw new Error('Invalid project key payload')
  }
  const kind = fields.get('kind')
  const version = fields.get('version')
  const key = fields.get('key')
  if (
    kind !== PROJECT_KEY_ENVELOPE_KIND ||
    version !== PROJECT_KEY_ENVELOPE_VERSION ||
    !key ||
    typeof key === 'string' ||
    typeof key === 'number' ||
    key.kind !== 'bytes' ||
    key.tagged
  ) {
    throw new Error('Invalid project key envelope')
  }
  return key.value
}

class ProjectKeyCborReader {
  readonly #bytes: Uint8Array
  #offset = 0

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes
  }

  nextMajorType(): number {
    return this.#peekByte() >>> 5
  }

  nextIsTag(tag: number): boolean {
    const start = this.#offset
    try {
      const header = this.#readHeader()
      return header.majorType === 6 && header.argument === tag
    } finally {
      this.#offset = start
    }
  }

  readMapLength(): number {
    const header = this.#readHeader()
    if (header.majorType !== 5 || header.argument === null) {
      throw new Error('Invalid project key payload')
    }
    return header.argument
  }

  readText(): string {
    const header = this.#readHeader()
    if (header.majorType !== 3 || header.argument === null) {
      throw new Error('Invalid project key payload')
    }
    const bytes = this.#readBytes(header.argument)
    return strictUtf8Decoder.decode(bytes)
  }

  readUnsignedInteger(): number {
    const header = this.#readHeader()
    if (header.majorType !== 0 || header.argument === null) {
      throw new Error('Invalid project key payload')
    }
    return header.argument
  }

  readProjectKeyValue(): ProjectKeyCborValue {
    let tagged = false
    if (this.nextIsTag(64)) {
      const tag = this.#readHeader()
      if (tag.majorType !== 6 || tag.argument !== 64) {
        throw new Error('Invalid project key payload')
      }
      tagged = true
    }
    const header = this.#readHeader()
    if (header.argument === null) {
      throw new Error('Invalid project key payload')
    }
    if (header.majorType === 2) {
      return {
        kind: 'bytes',
        value: this.#readBytes(header.argument),
        tagged,
      }
    }
    if (header.majorType === 3 && !tagged) {
      return {
        kind: 'text',
        value: strictUtf8Decoder.decode(this.#readBytes(header.argument)),
        tagged: false,
      }
    }
    if (
      header.majorType === 4
      && !tagged
      && header.argument === KEY_SIZE_BYTES
    ) {
      const value = new Uint8Array(KEY_SIZE_BYTES)
      try {
        for (let index = 0; index < value.length; index += 1) {
          const byte = this.readUnsignedInteger()
          if (byte > 0xff) {
            throw new Error('Invalid project key payload')
          }
          value[index] = byte
        }
      } catch (error) {
        zeroBytes(value)
        throw error
      }
      return {
        kind: 'legacy-array',
        value,
        tagged: false,
      }
    }
    throw new Error('Invalid project key payload')
  }

  requireEnd(): void {
    if (this.#offset !== this.#bytes.length) {
      throw new Error('Invalid project key payload')
    }
  }

  #readHeader(): { majorType: number; argument: number | null } {
    const initial = this.#readByte()
    const majorType = initial >>> 5
    const additional = initial & 0x1f
    if (additional < 24) {
      return { majorType, argument: additional }
    }
    if (additional === 31) {
      return { majorType, argument: null }
    }
    const byteCount =
      additional === 24
        ? 1
        : additional === 25
          ? 2
          : additional === 26
            ? 4
            : additional === 27
              ? 8
              : 0
    if (byteCount === 0) {
      throw new Error('Invalid CBOR header')
    }
    let value = 0n
    for (let index = 0; index < byteCount; index += 1) {
      value = (value << 8n) | BigInt(this.#readByte())
    }
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new Error('CBOR value is too large')
    }
    return { majorType, argument: Number(value) }
  }

  #readBytes(length: number): Uint8Array {
    const end = this.#offset + length
    if (!Number.isSafeInteger(length) || length < 0 || end > this.#bytes.length) {
      throw new Error('Invalid CBOR length')
    }
    // Return a view into the decrypted plaintext so error paths do not leave
    // detached secret copies. Successful callers make their own intrinsic
    // copy before the outer plaintext cleanup runs.
    const value = this.#bytes.subarray(this.#offset, end)
    this.#offset = end
    return value
  }

  #peekByte(): number {
    if (this.#offset >= this.#bytes.length) {
      throw new Error('Unexpected end of CBOR data')
    }
    return this.#bytes[this.#offset]!
  }

  #readByte(): number {
    const value = this.#peekByte()
    this.#offset += 1
    return value
  }
}

function requireProjectKey(key: Uint8Array): Uint8Array {
  if (key.length !== KEY_SIZE_BYTES) {
    throw new Error(
      `Project key must be exactly ${KEY_SIZE_BYTES} bytes`,
    )
  }
  return toUint8Array(key)
}

function decodeLegacyProjectKeyString(value: string): Uint8Array {
  const normalized = value
    .replace(/[ \t\n\v\f\r]/g, '')
    .replaceAll('-', '+')
    .replaceAll('_', '/')
  if (normalized.length === 0 || /\s/u.test(normalized)) {
    throw new Error('Invalid project key payload')
  }
  const decoded = decodeBase64(normalized)
  let isCanonical = false
  try {
    const canonicalUnpadded = encodeBase64(decoded)
    const canonicalPadded = canonicalUnpadded.padEnd(
      canonicalUnpadded.length + ((4 - (canonicalUnpadded.length % 4)) % 4),
      '=',
    )
    isCanonical =
      normalized === canonicalUnpadded || normalized === canonicalPadded
    if (!isCanonical) {
      throw new Error('Invalid project key payload')
    }
    return decoded
  } finally {
    if (!isCanonical) {
      zeroBytes(decoded)
    }
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value)
  )
}

function hasExactlyKeys(
  value: Record<string, unknown>,
  expectedKeys: string[],
): boolean {
  const actualKeys = Object.keys(value)
  if (actualKeys.length !== expectedKeys.length) return false
  const expected = new Set(expectedKeys)
  return actualKeys.every((key) => expected.has(key))
}
