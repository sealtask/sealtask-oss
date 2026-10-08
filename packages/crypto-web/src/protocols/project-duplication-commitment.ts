import { encodeBase64 } from '../runtime/base64'
import { zeroBytes } from '../runtime/bytes'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import { canonicalizeTemplateImportSemanticPlan } from './template-import-commitment'

const KEY_INFO = 'worklist.project-duplication.semantic-commitment.key.v1'
const PAYLOAD_DOMAIN = 'worklist.project-duplication.semantic-commitment.payload.v1'

/**
 * Frozen canonical JSON: UTF-16 object-key order, ordinary JSON string escaping,
 * no Unicode normalization, ordered arrays, and safe integers only. Restricting
 * numbers avoids cross-language floating-point formatting differences. Strings
 * must contain Unicode scalars so UTF-8 encoding cannot replace lone surrogates.
 * Existing template canonicalization and commitment domains remain unchanged.
 */
export function canonicalizeProjectDuplicationPlan(plan: unknown): string {
  const canonical = canonicalizeTemplateImportSemanticPlan(plan)
  const assertPortable = (value: unknown): void => {
    if (typeof value === 'number' && !Number.isSafeInteger(value)) {
      throw new Error('Project duplication commitment requires safe integer numbers')
    }
    if (typeof value === 'string' && /[\uD800-\uDFFF]/u.test(value)) {
      throw new Error('Project duplication commitment requires Unicode scalar strings')
    }
    if (Array.isArray(value)) value.forEach(assertPortable)
    else if (value !== null && typeof value === 'object') {
      for (const [key, child] of Object.entries(value)) {
        assertPortable(key)
        assertPortable(child)
      }
    }
  }
  assertPortable(plan)
  return canonical
}

export async function computeProjectDuplicationCommitment(params: {
  listKey: Uint8Array
  plan: unknown
}): Promise<string> {
  if (params.listKey.byteLength !== 32) throw new Error('Project duplication key must contain 32 bytes')
  const message = new TextEncoder().encode(`${PAYLOAD_DOMAIN}\0${canonicalizeProjectDuplicationPlan(params.plan)}`)
  let key: Uint8Array | undefined
  let digest: Uint8Array | undefined
  try {
    key = await hkdfExpand({ parent: params.listKey, info: KEY_INFO, length: 32 })
    digest = await hmacSha256(key, message, 'Project duplication requires WebCrypto HMAC-SHA256')
    return encodeBase64(digest)
  } finally {
    zeroBytes(message)
    zeroBytes(key)
    zeroBytes(digest)
  }
}
