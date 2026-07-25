import { encode as cborEncode } from 'cbor-x'

import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import type { AttachmentRef } from './attachment'
import {
  computeSchemaHash,
  decodeAndValidatePayloadBytes,
  validatePayloadBytes,
  type PayloadValidationDependencies,
} from './payload-validation'
import type {
  PayloadRichText,
  RichTextBlock,
  TextMark,
  TextMarkType,
  TextSpan,
} from './rich-text'
import { parseSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import type { ValidatedSealedBlobPayload } from './types'

export type {
  RichTextBlock,
  TextMark,
  TextMarkType,
  TextSpan,
}
export type TaskPayloadRichText = PayloadRichText

export type ChecklistItemPayload = {
  id: string
  title: string
  is_done: boolean
  completed_at?: number | null
  assignee_user_ids?: string[]
}

export type TaskPayloadBody = {
  title: string
  rich_text?: TaskPayloadRichText | null
  checklist?: ChecklistItemPayload[]
  attachments?: AttachmentRef[]
  references?: unknown[]
  mentions?: string[]
  client_meta?: Record<string, unknown>
  recurrence_state?: Record<string, unknown> | null
}

export type TaskPayloadEnvelope = {
  kind: 'task'
  version: number
  body: TaskPayloadBody
}

const TASK_PAYLOAD_CONTEXT = new TextEncoder().encode('worklist.task.v1')

export function buildTaskPayloadEnvelope(
  body: TaskPayloadBody,
  version = 1,
): TaskPayloadEnvelope {
  return { kind: 'task', version, body }
}

export async function encryptTaskPayload(params: {
  envelope: TaskPayloadEnvelope
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<ValidatedSealedBlobPayload> {
  const plaintext = toUint8Array(cborEncode(params.envelope))
  validatePayloadBytes(plaintext, 'task', params.validation)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const [schemaHash, ciphertext] = await Promise.all([
    computeSchemaHash(plaintext),
    bridge.encrypt({
      key: params.listKey,
      context: TASK_PAYLOAD_CONTEXT,
      plaintext,
    }),
  ])
  return {
    ...toSealedBlob({ version: SEALED_PAYLOAD_VERSION, ciphertext }),
    schemaHash,
  }
}

export async function decryptTaskPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<TaskPayloadEnvelope> {
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: TASK_PAYLOAD_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  const envelope = decodeAndValidatePayloadBytes(
    plaintext,
    'task',
    params.validation,
  )
  return {
    ...envelope,
    body: envelope.body as TaskPayloadBody,
  }
}
