import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { randomBytes } from '../runtime/random'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
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
  TextSpan,
} from './rich-text'
import {
  parseSealedPayload,
  serializeSealedPayloadBase64,
} from './sealed-payload'
import type { SealedBlobPayload } from './types'

const encoder = new TextEncoder()
const NOTE_PAYLOAD_DOMAIN = {
  kind: 'note',
  context: encoder.encode('worklist.note.v1'),
} as const
const NOTE_KEY_CONTEXT = encoder.encode('worklist.note.key.v1')

export type { RichTextBlock, TextMark, TextSpan }
export type NotePayloadRichText = PayloadRichText

export type HistoricalNoteAttachmentRef = {
  id: string
  name: string
  size: number
  mime_type?: string
}

export type NoteAttachmentRef =
  | AttachmentRef
  | HistoricalNoteAttachmentRef

export type NotePayloadBody = {
  title: string
  content: NotePayloadRichText
  mentions?: string[]
  attachments?: NoteAttachmentRef[]
  client_meta?: Record<string, unknown>
}

export type NotePayloadEnvelope = {
  kind: 'note'
  version: number
  body: NotePayloadBody
}

export function buildNotePayloadEnvelope(
  body: NotePayloadBody,
  version = 1,
): NotePayloadEnvelope {
  return { kind: 'note', version, body }
}

export async function encryptNotePayload(params: {
  envelope: NotePayloadEnvelope
  noteKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<SealedBlobPayload> {
  return sealPayloadEnvelope(
    params.envelope,
    params.noteKey,
    NOTE_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
}

export async function decryptNotePayload(params: {
  ciphertext: string
  noteKey: Uint8Array
  strongBox?: StrongBoxBridge
  validation?: PayloadValidationDependencies
}): Promise<NotePayloadEnvelope> {
  const envelope = await openPayloadEnvelope(
    params.ciphertext,
    params.noteKey,
    NOTE_PAYLOAD_DOMAIN,
    params.strongBox,
    params.validation,
  )
  return {
    ...envelope,
    body: envelope.body as NotePayloadBody,
  }
}

export function generateNoteKey(): Uint8Array {
  return randomBytes(32)
}

export async function encryptNoteKey(
  noteKey: Uint8Array,
  dataKey: Uint8Array,
  strongBox?: StrongBoxBridge,
): Promise<string> {
  const bridge = strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: dataKey,
    context: NOTE_KEY_CONTEXT,
    plaintext: noteKey,
  })
  return serializeSealedPayloadBase64({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
}

export async function decryptNoteKey(
  noteKeyCiphertext: string,
  dataKey: Uint8Array,
  strongBox?: StrongBoxBridge,
): Promise<Uint8Array> {
  const sealed = parseSealedPayload(noteKeyCiphertext)
  const bridge = strongBox ?? (await getStrongBoxBridge())
  return bridge.decrypt({
    key: dataKey,
    context: NOTE_KEY_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
}
