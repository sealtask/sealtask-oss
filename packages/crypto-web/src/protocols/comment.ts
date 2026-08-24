import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  openPayloadEnvelope,
  sealPayloadEnvelope,
} from './payload-sealing'
import type { PayloadValidationDependencies } from './payload-validation'
import type { TaskPayloadRichText } from './task'
import type { SealedBlobPayload } from './types'

const COMMENT_PAYLOAD_DOMAIN = {
  kind: 'comment',
  context: new TextEncoder().encode('worklist.comment.v1'),
} as const

export type CommentPayloadBody = {
  content: TaskPayloadRichText
  mentions?: string[]
  attachments?: unknown[]
  client_meta?: Record<string, unknown>
}

export type CommentPayloadEnvelope = {
  kind: 'comment'
  version: number
  body: CommentPayloadBody
}

export function buildCommentPayloadEnvelope(
  body: CommentPayloadBody,
  version = 1,
): CommentPayloadEnvelope {
  return { kind: 'comment', version, body }
}

export async function encryptCommentPayload(params: {
  envelope: CommentPayloadEnvelope
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<SealedBlobPayload> {
  return sealPayloadEnvelope(
    params.envelope,
    params.listKey,
    COMMENT_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
}

export async function decryptCommentPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<CommentPayloadEnvelope> {
  const envelope = await openPayloadEnvelope(
    params.ciphertext,
    params.listKey,
    COMMENT_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
  return {
    ...envelope,
    body: envelope.body as CommentPayloadBody,
  }
}
