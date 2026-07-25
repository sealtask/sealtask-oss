import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { randomBytes } from '../runtime/random'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  MAX_ATTACHMENT_CIPHERTEXT_BYTES,
  MAX_ATTACHMENT_PLAINTEXT_BYTES,
} from './attachment-transport-limits'
import {
  parseSealedPayloadBytes,
  serializeSealedPayload,
} from './sealed-payload'

export {
  MAX_ATTACHMENT_CIPHERTEXT_BYTES,
  MAX_ATTACHMENT_PLAINTEXT_BYTES,
}

const encoder = new TextEncoder()
const ATTACHMENT_BLOB_CONTEXT_LABEL = 'worklist.attachment.blob.v1'
const ATTACHMENT_REF_CONTEXT = encoder.encode('worklist.attachment.ref.v1')
const ATTACHMENT_KEY_BYTES = 32
const ATTACHMENT_BLOB_REF_VERSION = 1

export const ATTACHMENT_BLOB_CONTEXT = encoder.encode(
  ATTACHMENT_BLOB_CONTEXT_LABEL,
)

export type AttachmentBlobRef = {
  version: 1
  ciphertext_bytes: number
  file_key: Uint8Array
  enc_context: string
}

export type AttachmentRef = {
  id: string
  file_name: string
  content_type: string
  size_bytes: number
  blob_key: Uint8Array
  created_by_membership_id?: string
}

export type AttachmentEncryptionResult = {
  ciphertext: Uint8Array
  file_key: Uint8Array
  enc_context: string
}

export async function encryptAttachmentBytes(params: {
  plaintext: Uint8Array
  fileKey?: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<AttachmentEncryptionResult> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const fileKey = params.fileKey ?? randomBytes(ATTACHMENT_KEY_BYTES)
  const ciphertext = await bridge.encrypt({
    key: fileKey,
    context: ATTACHMENT_BLOB_CONTEXT,
    plaintext: toUint8Array(params.plaintext),
  })
  return {
    ciphertext,
    file_key: toUint8Array(fileKey),
    enc_context: ATTACHMENT_BLOB_CONTEXT_LABEL,
  }
}

export async function encodeAttachmentBlobKey(params: {
  listKey: Uint8Array
  blobRef: AttachmentBlobRef
  strongBox?: StrongBoxBridge
}): Promise<Uint8Array> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: ATTACHMENT_REF_CONTEXT,
    plaintext: toUint8Array(cborEncode(params.blobRef)),
  })
  return serializeSealedPayload({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
}

export async function decodeAttachmentBlobKey(params: {
  listKey: Uint8Array
  blobKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<AttachmentBlobRef> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const sealed = parseSealedPayloadBytes(toUint8Array(params.blobKey))
  if (sealed.version !== SEALED_PAYLOAD_VERSION) {
    throw new Error(`Unsupported sealed payload version: ${sealed.version}`)
  }
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: ATTACHMENT_REF_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  const decoded = cborDecode(plaintext) as Partial<AttachmentBlobRef>
  if (
    !decoded ||
    typeof decoded !== 'object' ||
    decoded.version !== ATTACHMENT_BLOB_REF_VERSION ||
    typeof decoded.ciphertext_bytes !== 'number' ||
    !Number.isSafeInteger(decoded.ciphertext_bytes) ||
    decoded.ciphertext_bytes <= 0 ||
    decoded.ciphertext_bytes > MAX_ATTACHMENT_CIPHERTEXT_BYTES ||
    !decoded.file_key
  ) {
    throw new Error('Attachment blob key is invalid')
  }
  return {
    version: ATTACHMENT_BLOB_REF_VERSION,
    ciphertext_bytes: decoded.ciphertext_bytes,
    file_key: toUint8Array(decoded.file_key),
    enc_context:
      typeof decoded.enc_context === 'string'
        ? decoded.enc_context
        : ATTACHMENT_BLOB_CONTEXT_LABEL,
  }
}

export async function decryptAttachmentBytes(params: {
  ciphertext: Uint8Array
  fileKey: Uint8Array
  encContext?: string
  strongBox?: StrongBoxBridge
}): Promise<Uint8Array> {
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.fileKey,
    context: encoder.encode(
      params.encContext ?? ATTACHMENT_BLOB_CONTEXT_LABEL,
    ),
    ciphertext: toUint8Array(params.ciphertext),
  })
  return toUint8Array(plaintext)
}

export function buildAttachmentBlobRef(params: {
  ciphertextBytes: number
  fileKey: Uint8Array
  encContext?: string
}): AttachmentBlobRef {
  if (
    !Number.isSafeInteger(params.ciphertextBytes) ||
    params.ciphertextBytes <= 0 ||
    params.ciphertextBytes > MAX_ATTACHMENT_CIPHERTEXT_BYTES
  ) {
    throw new Error('Attachment ciphertext size is invalid')
  }
  return {
    version: ATTACHMENT_BLOB_REF_VERSION,
    ciphertext_bytes: params.ciphertextBytes,
    file_key: toUint8Array(params.fileKey),
    enc_context: params.encContext ?? ATTACHMENT_BLOB_CONTEXT_LABEL,
  }
}
