import { Decoder, Encoder } from 'cbor-x'

import { decodeBase64 } from '../runtime/base64'
import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import {
  getStrongBoxBridge,
  type StrongBoxBridge,
} from '../runtime/strong-box'
import { parseStrictSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import type { SealedBlobPayload } from './types'

export const WORK_LIST_EXTERNAL_REFERENCES_CONTEXT =
  new TextEncoder().encode(
    'worklist.work_list_external_references.v1',
  )
export const TASK_EXTERNAL_REFERENCES_CONTEXT =
  new TextEncoder().encode('worklist.task_external_references.v1')
export const EXTERNAL_REFERENCES_VERSION = 1
export const EXTERNAL_REFERENCE_ITEMS_MAX = 32
export const EXTERNAL_REFERENCE_LABEL_MAX_BYTES = 64
export const EXTERNAL_REFERENCE_VALUE_MAX_BYTES = 256
export const EXTERNAL_REFERENCE_SYSTEM_MAX_BYTES = 128
export const EXTERNAL_REFERENCES_SEALED_PAYLOAD_MAX_BYTES =
  32 * 1024

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
const WORK_LIST_ENVELOPE_KEYS = [
  'items',
  'kind',
  'version',
  'work_list_id',
] as const
const TASK_ENVELOPE_KEYS = [
  'items',
  'kind',
  'task_id',
  'version',
  'work_list_id',
] as const
const REQUIRED_ITEM_KEYS = ['label', 'value'] as const
const OPTIONAL_ITEM_KEYS = ['label', 'system', 'value'] as const
const cborEncoder = new Encoder({
  useRecords: false,
  variableMapSize: true,
  tagUint8Array: false,
})
const cborDecoder = new Decoder({ mapsAsObjects: true })

export type ExternalReferenceItem = {
  label: string
  value: string
  system?: string
}

export type WorkListExternalReferences = {
  kind: 'work_list_external_references'
  version: typeof EXTERNAL_REFERENCES_VERSION
  workListId: string
  items: ExternalReferenceItem[]
}

export type TaskExternalReferences = {
  kind: 'task_external_references'
  version: typeof EXTERNAL_REFERENCES_VERSION
  workListId: string
  taskId: string
  items: ExternalReferenceItem[]
}

type WorkListExternalReferencesWire = {
  kind: 'work_list_external_references'
  version: typeof EXTERNAL_REFERENCES_VERSION
  work_list_id: string
  items: ExternalReferenceItem[]
}

type TaskExternalReferencesWire = {
  kind: 'task_external_references'
  version: typeof EXTERNAL_REFERENCES_VERSION
  work_list_id: string
  task_id: string
  items: ExternalReferenceItem[]
}

export function buildWorkListExternalReferences(params: {
  workListId: string
  items: readonly ExternalReferenceItem[]
}): WorkListExternalReferences {
  return {
    kind: 'work_list_external_references',
    version: EXTERNAL_REFERENCES_VERSION,
    workListId: requireCanonicalUuid(
      params.workListId,
      'workListId',
    ),
    items: validateExternalReferenceItems(params.items),
  }
}

export function buildTaskExternalReferences(params: {
  workListId: string
  taskId: string
  items: readonly ExternalReferenceItem[]
}): TaskExternalReferences {
  return {
    kind: 'task_external_references',
    version: EXTERNAL_REFERENCES_VERSION,
    workListId: requireCanonicalUuid(
      params.workListId,
      'workListId',
    ),
    taskId: requireCanonicalUuid(params.taskId, 'taskId'),
    items: validateExternalReferenceItems(params.items),
  }
}

export function encodeWorkListExternalReferences(
  envelope: WorkListExternalReferences,
): Uint8Array {
  const normalized = buildWorkListExternalReferences(envelope)
  const wire: WorkListExternalReferencesWire = {
    kind: normalized.kind,
    version: normalized.version,
    work_list_id: normalized.workListId,
    items: normalized.items,
  }
  return toUint8Array(cborEncoder.encode(wire))
}

export function encodeTaskExternalReferences(
  envelope: TaskExternalReferences,
): Uint8Array {
  const normalized = buildTaskExternalReferences(envelope)
  const wire: TaskExternalReferencesWire = {
    kind: normalized.kind,
    version: normalized.version,
    work_list_id: normalized.workListId,
    task_id: normalized.taskId,
    items: normalized.items,
  }
  return toUint8Array(cborEncoder.encode(wire))
}

export function decodeWorkListExternalReferences(params: {
  plaintext: Uint8Array
  expectedWorkListId: string
}): WorkListExternalReferences {
  const expectedWorkListId = requireCanonicalUuid(
    params.expectedWorkListId,
    'expectedWorkListId',
  )
  const decoded = decodeStrictCborEnvelope(params.plaintext)
  if (
    !hasOnlyKeys(decoded, WORK_LIST_ENVELOPE_KEYS) ||
    decoded.kind !== 'work_list_external_references' ||
    decoded.version !== EXTERNAL_REFERENCES_VERSION ||
    typeof decoded.work_list_id !== 'string' ||
    !Array.isArray(decoded.items)
  ) {
    throw new Error(
      'Work list external references envelope is invalid',
    )
  }
  const envelope = buildWorkListExternalReferences({
    workListId: decoded.work_list_id,
    items: decodeExternalReferenceItems(decoded.items),
  })
  if (envelope.workListId !== expectedWorkListId) {
    throw new Error('Work list external references identity mismatch')
  }
  return envelope
}

export function decodeTaskExternalReferences(params: {
  plaintext: Uint8Array
  expectedWorkListId: string
  expectedTaskId: string
}): TaskExternalReferences {
  const expectedWorkListId = requireCanonicalUuid(
    params.expectedWorkListId,
    'expectedWorkListId',
  )
  const expectedTaskId = requireCanonicalUuid(
    params.expectedTaskId,
    'expectedTaskId',
  )
  const decoded = decodeStrictCborEnvelope(params.plaintext)
  if (
    !hasOnlyKeys(decoded, TASK_ENVELOPE_KEYS) ||
    decoded.kind !== 'task_external_references' ||
    decoded.version !== EXTERNAL_REFERENCES_VERSION ||
    typeof decoded.work_list_id !== 'string' ||
    typeof decoded.task_id !== 'string' ||
    !Array.isArray(decoded.items)
  ) {
    throw new Error('Task external references envelope is invalid')
  }
  const envelope = buildTaskExternalReferences({
    workListId: decoded.work_list_id,
    taskId: decoded.task_id,
    items: decodeExternalReferenceItems(decoded.items),
  })
  if (envelope.workListId !== expectedWorkListId) {
    throw new Error('Task external references project identity mismatch')
  }
  if (envelope.taskId !== expectedTaskId) {
    throw new Error('Task external references task identity mismatch')
  }
  return envelope
}

export async function encryptWorkListExternalReferences(params: {
  envelope: WorkListExternalReferences
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  return encryptExternalReferences({
    plaintext: encodeWorkListExternalReferences(params.envelope),
    context: WORK_LIST_EXTERNAL_REFERENCES_CONTEXT,
    listKey: params.listKey,
    strongBox: params.strongBox,
  })
}

export async function encryptTaskExternalReferences(params: {
  envelope: TaskExternalReferences
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  return encryptExternalReferences({
    plaintext: encodeTaskExternalReferences(params.envelope),
    context: TASK_EXTERNAL_REFERENCES_CONTEXT,
    listKey: params.listKey,
    strongBox: params.strongBox,
  })
}

export async function decryptWorkListExternalReferences(params: {
  ciphertext: string
  listKey: Uint8Array
  expectedWorkListId: string
  strongBox?: StrongBoxBridge
}): Promise<WorkListExternalReferences> {
  const plaintext = await decryptExternalReferences({
    ciphertext: params.ciphertext,
    context: WORK_LIST_EXTERNAL_REFERENCES_CONTEXT,
    listKey: params.listKey,
    strongBox: params.strongBox,
  })
  return decodeWorkListExternalReferences({
    plaintext,
    expectedWorkListId: params.expectedWorkListId,
  })
}

export async function decryptTaskExternalReferences(params: {
  ciphertext: string
  listKey: Uint8Array
  expectedWorkListId: string
  expectedTaskId: string
  strongBox?: StrongBoxBridge
}): Promise<TaskExternalReferences> {
  const plaintext = await decryptExternalReferences({
    ciphertext: params.ciphertext,
    context: TASK_EXTERNAL_REFERENCES_CONTEXT,
    listKey: params.listKey,
    strongBox: params.strongBox,
  })
  return decodeTaskExternalReferences({
    plaintext,
    expectedWorkListId: params.expectedWorkListId,
    expectedTaskId: params.expectedTaskId,
  })
}

async function encryptExternalReferences(params: {
  plaintext: Uint8Array
  context: Uint8Array
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: params.context,
    plaintext: params.plaintext,
  })
  const sealed = toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
  if (
    sealed.bytes.byteLength >
    EXTERNAL_REFERENCES_SEALED_PAYLOAD_MAX_BYTES
  ) {
    throw new Error('External references ciphertext is too large')
  }
  return sealed
}

async function decryptExternalReferences(params: {
  ciphertext: string
  context: Uint8Array
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<Uint8Array> {
  const serialized = decodeBase64(params.ciphertext)
  if (
    serialized.byteLength === 0 ||
    serialized.byteLength >
      EXTERNAL_REFERENCES_SEALED_PAYLOAD_MAX_BYTES
  ) {
    throw new Error('External references ciphertext has an invalid size')
  }
  const sealed = parseStrictSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  return bridge.decrypt({
    key: params.listKey,
    context: params.context,
    ciphertext: sealed.ciphertext,
  })
}

function decodeExternalReferenceItems(
  values: unknown[],
): ExternalReferenceItem[] {
  return values.map((value) => {
    if (
      !isRecord(value) ||
      (!hasOnlyKeys(value, REQUIRED_ITEM_KEYS) &&
        !hasOnlyKeys(value, OPTIONAL_ITEM_KEYS)) ||
      typeof value.label !== 'string' ||
      typeof value.value !== 'string' ||
      (value.system !== undefined && typeof value.system !== 'string')
    ) {
      throw new Error('External reference item is invalid')
    }
    return {
      label: value.label,
      value: value.value,
      ...(typeof value.system === 'string'
        ? { system: value.system }
        : {}),
    }
  })
}

function validateExternalReferenceItems(
  values: readonly ExternalReferenceItem[],
): ExternalReferenceItem[] {
  if (!Array.isArray(values) || values.length > EXTERNAL_REFERENCE_ITEMS_MAX) {
    throw new Error(
      `External references cannot contain more than ${EXTERNAL_REFERENCE_ITEMS_MAX} items`,
    )
  }
  return values.map((value) => {
    if (!isRecord(value)) {
      throw new Error('External reference item is invalid')
    }
    const label = requireExternalReferenceText(
      value.label,
      'label',
      EXTERNAL_REFERENCE_LABEL_MAX_BYTES,
    )
    const referenceValue = requireExternalReferenceText(
      value.value,
      'value',
      EXTERNAL_REFERENCE_VALUE_MAX_BYTES,
    )
    const system =
      value.system === undefined
        ? undefined
        : requireExternalReferenceText(
            value.system,
            'system',
            EXTERNAL_REFERENCE_SYSTEM_MAX_BYTES,
          )
    return {
      label,
      value: referenceValue,
      ...(system === undefined ? {} : { system }),
    }
  })
}

function requireExternalReferenceText(
  value: unknown,
  field: string,
  maxBytes: number,
): string {
  if (
    typeof value !== 'string' ||
    value.length === 0 ||
    new TextEncoder().encode(value).byteLength > maxBytes ||
    /^\p{White_Space}|\p{White_Space}$/u.test(value) ||
    /[\u0000-\u001f\u007f-\u009f]/u.test(value)
  ) {
    throw new Error(`External reference ${field} is invalid`)
  }
  return value
}

function requireCanonicalUuid(value: string, field: string): string {
  if (typeof value !== 'string' || !UUID_PATTERN.test(value)) {
    throw new Error(`${field} must be a canonical UUID`)
  }
  return value
}

function decodeStrictCborEnvelope(
  plaintext: Uint8Array,
): Record<string, unknown> {
  assertStrictCborTextKeyMaps(plaintext)
  let decoded: unknown
  try {
    decoded = cborDecoder.decode(plaintext)
  } catch {
    throw new Error('External references plaintext is not valid CBOR')
  }
  if (!isRecord(decoded)) {
    throw new Error('External references envelope is invalid')
  }
  return decoded
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}

function hasOnlyKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
): boolean {
  const actual = Object.keys(value).sort()
  const sortedExpected = [...expected].sort()
  return (
    actual.length === sortedExpected.length &&
    actual.every((key, index) => key === sortedExpected[index])
  )
}

type CborHeader = {
  majorType: number
  argument: number
  nextOffset: number
}

function assertStrictCborTextKeyMaps(bytes: Uint8Array): void {
  try {
    const nextOffset = scanCborItem(bytes, 0, 0)
    if (nextOffset !== bytes.byteLength) {
      throw new Error('trailing bytes')
    }
  } catch {
    throw new Error(
      'External references plaintext is not valid strict CBOR',
    )
  }
}

function scanCborItem(
  bytes: Uint8Array,
  offset: number,
  depth: number,
): number {
  if (depth > 16) {
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
        nextOffset = scanCborItem(bytes, nextOffset, depth + 1)
      }
      return nextOffset
    }
    case 5: {
      const seen = new Set<string>()
      let nextOffset = header.nextOffset
      for (let index = 0; index < header.argument; index += 1) {
        const key = readCborText(bytes, nextOffset)
        if (seen.has(key.value)) {
          throw new Error('duplicate CBOR map key')
        }
        seen.add(key.value)
        nextOffset = scanCborItem(bytes, key.nextOffset, depth + 1)
      }
      return nextOffset
    }
    case 6:
      return scanCborItem(bytes, header.nextOffset, depth + 1)
    default:
      throw new Error('unsupported CBOR major type')
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
