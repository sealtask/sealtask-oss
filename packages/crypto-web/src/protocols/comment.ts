import { encode as cborEncode } from 'cbor-x'

import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  decodeAndValidatePayloadBytes,
  validatePayloadBytes,
  type PayloadValidationDependencies,
} from './payload-validation'
import { parseSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import type { TaskPayloadRichText } from './task'
import type { SealedBlobPayload } from './types'

const COMMENT_PAYLOAD_CONTEXT = new TextEncoder().encode('worklist.comment.v1')

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
  const plaintext = toUint8Array(cborEncode(params.envelope))
  validatePayloadBytes(plaintext, 'comment', params.validation)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: COMMENT_PAYLOAD_CONTEXT,
    plaintext,
  })
  return toSealedBlob({ version: SEALED_PAYLOAD_VERSION, ciphertext })
}

export async function decryptCommentPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<CommentPayloadEnvelope> {
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: COMMENT_PAYLOAD_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  const envelope = decodeAndValidatePayloadBytes(
    plaintext,
    'comment',
    params.validation,
  )
  return {
    ...envelope,
    body: envelope.body as CommentPayloadBody,
  }
}
