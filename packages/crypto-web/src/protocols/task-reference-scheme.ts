import { Decoder, Encoder } from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { randomBytes } from '../runtime/random'
import {
  getStrongBoxBridge,
  type StrongBoxBridge,
} from '../runtime/strong-box'
import { parseStrictSealedPayload } from './sealed-payload'
import { readStrictCborTextMapKeys } from './strict-cbor'
import type { SealedBlobPayload } from './types'

export const TASK_REFERENCE_SCHEME_CONTEXT = new TextEncoder().encode(
  'worklist.task_reference_scheme.v1',
)
export const TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES = 512
export const TASK_REFERENCE_SCHEME_AEAD_CIPHERTEXT_BYTES =
  TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES + 16
export const TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES = 565
export const TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES = 589
export const TASK_REFERENCE_SCHEME_VERSION = 1
export const TASK_REFERENCE_ORDINARY_REVISION_MAX = 32
export const TASK_REFERENCE_REPAIR_REVISION_MAX = 4
export const TASK_REFERENCE_REVISION_MAX =
  TASK_REFERENCE_ORDINARY_REVISION_MAX +
  TASK_REFERENCE_REPAIR_REVISION_MAX
export const TASK_REFERENCE_HISTORY_LIMIT =
  TASK_REFERENCE_REVISION_MAX
export const TASK_REFERENCE_PREFIX_PATTERN = /^[A-Z][A-Z0-9]{1,9}$/
export const TASK_REFERENCE_MINIMUM_DIGITS_MIN = 1
export const TASK_REFERENCE_MINIMUM_DIGITS_MAX = 8
export const TASK_REFERENCE_MAX_SAFE_INTEGER = Number.MAX_SAFE_INTEGER

const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
const TASK_REFERENCE_INPUT_PATTERN =
  /^([a-z][a-z0-9]{1,9})\s*-\s*([0-9]+)$/i
const PROJECT_REFERENCE_NUMBER_INPUT_PATTERN = /^#?\s*([0-9]+)$/
const ENVELOPE_KEYS = [
  'kind',
  'minimum_digits',
  'padding',
  'prefix',
  'revision',
  'scheme_revision_id',
  'separator',
  'version',
  'work_list_id',
] as const
const taskReferenceCborEncoder = new Encoder({
  useRecords: false,
  variableMapSize: true,
  tagUint8Array: false,
})
const taskReferenceCborDecoder = new Decoder({ mapsAsObjects: true })
const TASK_REFERENCE_STRICT_CBOR_POLICY = { maxDepth: 16 } as const

export type TaskReferenceScheme = {
  kind: 'task_reference_scheme'
  version: typeof TASK_REFERENCE_SCHEME_VERSION
  workListId: string
  schemeRevisionId: string
  revision: number
  prefix: string
  separator: '-'
  minimumDigits: number
}

export type TaskReferenceSchemeInput = {
  workListId: string
  schemeRevisionId: string
  revision: number
  prefix: string
  minimumDigits: number
}

export type ParsedTaskReference = {
  prefix: string
  referenceNumber: number
}

type TaskReferenceSchemeEnvelope = {
  kind: 'task_reference_scheme'
  version: typeof TASK_REFERENCE_SCHEME_VERSION
  work_list_id: string
  scheme_revision_id: string
  revision: number
  prefix: string
  separator: '-'
  minimum_digits: number
  padding: Uint8Array
}

export function normalizeTaskReferencePrefix(value: string): string {
  return value.trim().toUpperCase()
}

export function buildTaskReferenceScheme(
  input: TaskReferenceSchemeInput,
): TaskReferenceScheme {
  const scheme: TaskReferenceScheme = {
    kind: 'task_reference_scheme',
    version: TASK_REFERENCE_SCHEME_VERSION,
    workListId: normalizeUuid(input.workListId, 'workListId'),
    schemeRevisionId: normalizeUuid(
      input.schemeRevisionId,
      'schemeRevisionId',
    ),
    revision: requireSchemeRevision(input.revision),
    prefix: normalizeTaskReferencePrefix(input.prefix),
    separator: '-',
    minimumDigits: requireMinimumDigits(input.minimumDigits),
  }
  requirePrefix(scheme.prefix)
  return scheme
}

export async function encryptTaskReferenceScheme(params: {
  scheme: TaskReferenceScheme
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<SealedBlobPayload> {
  const scheme = buildTaskReferenceScheme(params.scheme)
  const plaintext = encodeFixedSizeEnvelope(scheme)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: TASK_REFERENCE_SCHEME_CONTEXT,
    plaintext,
  })
  if (ciphertext.byteLength !== TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES) {
    throw new Error('Task reference scheme ciphertext has an invalid size')
  }
  const bytes = toUint8Array(taskReferenceCborEncoder.encode({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  }))
  if (bytes.byteLength !== TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES) {
    throw new Error('Task reference scheme sealed payload has an invalid size')
  }
  return { bytes, base64: encodeBase64(bytes) }
}

export async function decryptTaskReferenceScheme(params: {
  ciphertext: string
  listKey: Uint8Array
  expectedWorkListId: string
  expectedSchemeRevisionId: string
  expectedRevision: number
  strongBox?: StrongBoxBridge
}): Promise<TaskReferenceScheme> {
  if (
    decodeBase64(params.ciphertext).byteLength !==
    TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES
  ) {
    throw new Error('Task reference scheme sealed payload has an invalid size')
  }
  const sealed = parseStrictSealedPayload(params.ciphertext)
  if (
    sealed.ciphertext.byteLength !==
    TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES
  ) {
    throw new Error('Task reference scheme ciphertext has an invalid size')
  }
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: TASK_REFERENCE_SCHEME_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  return decodeTaskReferenceSchemePlaintext({
    plaintext,
    expectedWorkListId: params.expectedWorkListId,
    expectedSchemeRevisionId: params.expectedSchemeRevisionId,
    expectedRevision: params.expectedRevision,
  })
}

export function decodeTaskReferenceSchemePlaintext(params: {
  plaintext: Uint8Array
  expectedWorkListId: string
  expectedSchemeRevisionId: string
  expectedRevision: number
}): TaskReferenceScheme {
  const scheme = decodeFixedSizeEnvelope(params.plaintext)
  const expectedWorkListId = normalizeUuid(
    params.expectedWorkListId,
    'expectedWorkListId',
  )
  const expectedSchemeRevisionId = normalizeUuid(
    params.expectedSchemeRevisionId,
    'expectedSchemeRevisionId',
  )
  const expectedRevision = requireSchemeRevision(params.expectedRevision)

  if (scheme.workListId !== expectedWorkListId) {
    throw new Error('Task reference scheme project identity mismatch')
  }
  if (scheme.schemeRevisionId !== expectedSchemeRevisionId) {
    throw new Error('Task reference scheme revision identity mismatch')
  }
  if (scheme.revision !== expectedRevision) {
    throw new Error('Task reference scheme revision mismatch')
  }
  return scheme
}

export function formatTaskReference(
  scheme: TaskReferenceScheme,
  referenceNumber: number,
): string {
  const number = requirePositiveSafeInteger(
    referenceNumber,
    'referenceNumber',
  )
  const minimumDigits = requireMinimumDigits(scheme.minimumDigits)
  requirePrefix(scheme.prefix)
  return `${scheme.prefix}-${String(number).padStart(minimumDigits, '0')}`
}

export function formatTaskReferenceAliases(params: {
  current: TaskReferenceScheme
  historical?: readonly TaskReferenceScheme[]
  referenceNumber: number
}): string[] {
  const result: string[] = []
  const seen = new Set<string>()
  for (const scheme of [params.current, ...(params.historical ?? [])]) {
    const formatted = formatTaskReference(scheme, params.referenceNumber)
    if (!seen.has(formatted)) {
      seen.add(formatted)
      result.push(formatted)
    }
  }
  return result
}

export function parseTaskReference(
  value: string,
): ParsedTaskReference | null {
  const match = TASK_REFERENCE_INPUT_PATTERN.exec(value.trim())
  if (!match) return null
  const prefix = normalizeTaskReferencePrefix(match[1])
  if (!TASK_REFERENCE_PREFIX_PATTERN.test(prefix)) return null
  const referenceNumber = parsePositiveSafeInteger(match[2])
  return referenceNumber === null ? null : { prefix, referenceNumber }
}

export function parseProjectReferenceNumber(value: string): number | null {
  const match = PROJECT_REFERENCE_NUMBER_INPUT_PATTERN.exec(value.trim())
  return match ? parsePositiveSafeInteger(match[1]) : null
}

function encodeFixedSizeEnvelope(
  scheme: TaskReferenceScheme,
): Uint8Array {
  let paddingLength = 0
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const candidate = encodeEnvelope(scheme, new Uint8Array(paddingLength))
    const adjustment =
      TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES - candidate.byteLength
    if (adjustment === 0) {
      const encoded = encodeEnvelope(scheme, randomBytes(paddingLength))
      if (encoded.byteLength !== TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES) {
        throw new Error('Task reference scheme padding length is unstable')
      }
      return encoded
    }
    paddingLength += adjustment
    if (paddingLength <= 0) break
  }
  throw new Error('Task reference scheme exceeds its fixed plaintext size')
}

function encodeEnvelope(
  scheme: TaskReferenceScheme,
  padding: Uint8Array,
): Uint8Array {
  const envelope: TaskReferenceSchemeEnvelope = {
    kind: scheme.kind,
    version: scheme.version,
    work_list_id: scheme.workListId,
    scheme_revision_id: scheme.schemeRevisionId,
    revision: scheme.revision,
    prefix: scheme.prefix,
    separator: scheme.separator,
    minimum_digits: scheme.minimumDigits,
    padding,
  }
  return toUint8Array(taskReferenceCborEncoder.encode(envelope))
}

function decodeFixedSizeEnvelope(
  plaintext: Uint8Array,
): TaskReferenceScheme {
  if (plaintext.byteLength !== TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES) {
    throw new Error('Task reference scheme plaintext has an invalid size')
  }
  assertUniqueTopLevelCborMapKeys(plaintext)
  let decoded: unknown
  try {
    decoded = taskReferenceCborDecoder.decode(plaintext)
  } catch {
    throw new Error('Task reference scheme plaintext is not valid CBOR')
  }
  if (!isRecord(decoded) || !hasOnlyKeys(decoded, ENVELOPE_KEYS)) {
    throw new Error('Task reference scheme envelope is invalid')
  }
  if (!(decoded.padding instanceof Uint8Array)) {
    throw new Error('Task reference scheme padding is invalid')
  }
  if (
    decoded.kind !== 'task_reference_scheme' ||
    decoded.version !== TASK_REFERENCE_SCHEME_VERSION ||
    decoded.separator !== '-'
  ) {
    throw new Error('Task reference scheme kind or version is unsupported')
  }
  if (
    typeof decoded.work_list_id !== 'string' ||
    typeof decoded.scheme_revision_id !== 'string' ||
    typeof decoded.prefix !== 'string' ||
    typeof decoded.revision !== 'number' ||
    typeof decoded.minimum_digits !== 'number'
  ) {
    throw new Error('Task reference scheme fields are invalid')
  }
  return buildTaskReferenceScheme({
    workListId: decoded.work_list_id,
    schemeRevisionId: decoded.scheme_revision_id,
    revision: decoded.revision,
    prefix: decoded.prefix,
    minimumDigits: decoded.minimum_digits,
  })
}

function requirePrefix(value: string): void {
  if (!TASK_REFERENCE_PREFIX_PATTERN.test(value)) {
    throw new Error(
      'Task reference prefix must contain 2 to 10 uppercase ASCII letters or digits and start with a letter',
    )
  }
}

function requireMinimumDigits(value: number): number {
  if (
    !Number.isSafeInteger(value) ||
    value < TASK_REFERENCE_MINIMUM_DIGITS_MIN ||
    value > TASK_REFERENCE_MINIMUM_DIGITS_MAX
  ) {
    throw new Error(
      `Task reference minimumDigits must be between ${TASK_REFERENCE_MINIMUM_DIGITS_MIN} and ${TASK_REFERENCE_MINIMUM_DIGITS_MAX}`,
    )
  }
  return value
}

function requireSchemeRevision(value: number): number {
  if (
    !Number.isSafeInteger(value) ||
    value < 1 ||
    value > TASK_REFERENCE_REVISION_MAX
  ) {
    throw new Error(
      `Task reference scheme revision must be between 1 and ${TASK_REFERENCE_REVISION_MAX}`,
    )
  }
  return value
}

function requirePositiveSafeInteger(
  value: number,
  field: string,
): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${field} must be a positive safe integer`)
  }
  return value
}

function parsePositiveSafeInteger(value: string): number | null {
  if (!/^[0-9]+$/.test(value)) return null
  const normalized = value.replace(/^0+(?=\d)/, '')
  const parsed = Number(normalized)
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null
}

function normalizeUuid(value: string, field: string): string {
  if (
    value !== value.toLowerCase() ||
    !UUID_PATTERN.test(value)
  ) {
    throw new Error(`${field} must be a canonical UUID`)
  }
  return value
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

function assertUniqueTopLevelCborMapKeys(bytes: Uint8Array): void {
  try {
    readStrictCborTextMapKeys(bytes, TASK_REFERENCE_STRICT_CBOR_POLICY)
  } catch {
    throw new Error('Task reference scheme plaintext is not valid strict CBOR')
  }
}
