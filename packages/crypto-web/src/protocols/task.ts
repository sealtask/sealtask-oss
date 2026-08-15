import type { StrongBoxBridge } from '../runtime/strong-box'
import type { AttachmentRef } from './attachment'
import {
  openPayloadEnvelope,
  sealPayloadEnvelope,
} from './payload-sealing'
import type { PayloadValidationDependencies } from './payload-validation'
import type {
  PayloadRichText,
  RichTextBlock,
  TextMark,
  TextMarkType,
  TextSpan,
} from './rich-text'
import type { SealedBlobPayload } from './types'

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

const TASK_PAYLOAD_DOMAIN = {
  kind: 'task',
  context: new TextEncoder().encode('worklist.task.v1'),
} as const

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
}): Promise<SealedBlobPayload> {
  return sealPayloadEnvelope(
    params.envelope,
    params.listKey,
    TASK_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
}

export async function decryptTaskPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<TaskPayloadEnvelope> {
  const envelope = await openPayloadEnvelope(
    params.ciphertext,
    params.listKey,
    TASK_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
  return {
    ...envelope,
    body: envelope.body as TaskPayloadBody,
  }
}
