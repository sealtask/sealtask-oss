import { encodeBase64 } from '../runtime/base64'
import { zeroBytes } from '../runtime/bytes'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'

const COMMITMENT_KEY_INFO =
  'sealtask.note-create.semantic-commitment.key.v1'
const COMMITMENT_PAYLOAD_DOMAIN =
  'sealtask.note-create.semantic-commitment.payload.v1'
const encoder = new TextEncoder()

export type NoteCreateSemanticPlan = {
  title: string
  content: unknown
  mentions: unknown[]
  attachments: unknown[]
  clientMeta: Record<string, unknown>
  isPrivate: boolean
}

/**
 * Commits to readable note semantics without exposing them to the API.
 *
 * Property order is frozen here rather than inherited from a caller-owned
 * object, keeping retries stable while encryption uses fresh nonces.
 */
export async function computeNoteCreateSemanticCommitment(params: {
  listKey: Uint8Array
  plan: NoteCreateSemanticPlan
}): Promise<string> {
  const canonical = JSON.stringify({
    title: params.plan.title,
    content: params.plan.content,
    mentions: params.plan.mentions,
    attachments: params.plan.attachments,
    clientMeta: params.plan.clientMeta,
    isPrivate: params.plan.isPrivate,
  })
  const message = encoder.encode(
    `${COMMITMENT_PAYLOAD_DOMAIN}\0${canonical}`,
  )
  let commitmentKey: Uint8Array | undefined
  let commitment: Uint8Array | undefined
  try {
    commitmentKey = await hkdfExpand({
      parent: params.listKey,
      info: COMMITMENT_KEY_INFO,
      length: 32,
    })
    commitment = await hmacSha256(
      commitmentKey,
      message,
      'WebCrypto HMAC-SHA256 is unavailable for note creation',
    )
    return encodeBase64(commitment)
  } finally {
    zeroBytes(message)
    zeroBytes(commitment)
    zeroBytes(commitmentKey)
  }
}
