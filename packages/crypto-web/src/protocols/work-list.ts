import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  computeSchemaHash,
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
import type {
  SealedBlobPayload,
  ValidatedSealedBlobPayload,
} from './types'

const encoder = new TextEncoder()
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
}): Promise<ValidatedSealedBlobPayload> {
  const plaintext = toUint8Array(cborEncode(params.envelope))
  validatePayloadBytes(plaintext, 'work_list', params.validation)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const [schemaHash, ciphertext] = await Promise.all([
    computeSchemaHash(plaintext),
    bridge.encrypt({
      key: params.listKey,
      context: WORK_LIST_PAYLOAD_CONTEXT,
      plaintext,
    }),
  ])
  return {
    ...toSealedBlob({ version: SEALED_PAYLOAD_VERSION, ciphertext }),
    schemaHash,
  }
}

export async function sealWorkListKeyForOwner(params: {
  listKey: Uint8Array
  dataKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.dataKey,
    context: WORK_LIST_MEMBERSHIP_CONTEXT,
    plaintext: toUint8Array(params.listKey),
  })
  return toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
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
  return extractWorkListKeyFromPlaintext(plaintext)
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
    !hasOnlyKeys(decoded, ['value']) ||
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
  const envelopeKey = tryDecodeWorkListKeyEnvelope(plaintext)
  if (envelopeKey) return envelopeKey
  if (plaintext.length === 0) {
    throw new Error('Decrypted project key is empty')
  }
  return toUint8Array(plaintext)
}

function tryDecodeWorkListKeyEnvelope(
  plaintext: Uint8Array,
): Uint8Array | null {
  try {
    const decoded = cborDecode(plaintext) as unknown
    if (decoded instanceof Uint8Array) return toUint8Array(decoded)
    if (!isRecord(decoded)) return null
    const key = decoded.key
    if (typeof key === 'string') {
      return toUint8Array(
        decodeBase64(key.trim().replace(/-/g, '+').replace(/_/g, '/')),
      )
    }
    if (key instanceof Uint8Array) return toUint8Array(key)
    if (key instanceof ArrayBuffer) {
      return toUint8Array(new Uint8Array(key))
    }
    return null
  } catch {
    return null
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return (
    typeof value === 'object' &&
    value !== null &&
    !Array.isArray(value)
  )
}

function hasOnlyKeys(
  value: Record<string, unknown>,
  allowedKeys: string[],
): boolean {
  const allowed = new Set(allowedKeys)
  return Object.keys(value).every((key) => allowed.has(key))
}
