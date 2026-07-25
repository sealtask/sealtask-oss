import { encodeBase64 } from '../runtime/base64'
import { serializeSealedPayload, type SealedPayload } from './sealed-payload'
import type { SealedBlobPayload } from './types'

export function toSealedBlob(payload: SealedPayload): SealedBlobPayload {
  const bytes = serializeSealedPayload(payload)
  return {
    bytes,
    base64: encodeBase64(bytes),
  }
}
