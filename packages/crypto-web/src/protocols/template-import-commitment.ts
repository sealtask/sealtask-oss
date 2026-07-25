import { encodeBase64 } from '../runtime/base64'
import { zeroBytes } from '../runtime/bytes'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'

const COMMITMENT_KEY_INFO =
  'worklist.template-import.semantic-commitment.key.v1'
const COMMITMENT_PAYLOAD_DOMAIN =
  'worklist.template-import.semantic-commitment.payload.v1'
const LIST_KEY_BYTES = 32
const encoder = new TextEncoder()

/**
 * Commits an import operation to the exact readable project semantics without
 * disclosing them to the API. The domain-separated key is unique to the new
 * project, so equal public templates do not produce a reusable equality
 * oracle across projects.
 */
export async function computeTemplateImportSemanticCommitment(params: {
  listKey: Uint8Array
  plan: unknown
}): Promise<string> {
  if (params.listKey.byteLength !== LIST_KEY_BYTES) {
    throw new Error(
      'Template import list key must contain exactly 32 bytes',
    )
  }

  const canonicalPlan = canonicalizeTemplateImportSemanticPlan(params.plan)
  const message = encoder.encode(
    `${COMMITMENT_PAYLOAD_DOMAIN}\0${canonicalPlan}`,
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
      'WebCrypto HMAC-SHA256 is unavailable for template import',
    )
    return encodeBase64(commitment)
  } finally {
    zeroBytes(message)
    zeroBytes(commitment)
    zeroBytes(commitmentKey)
  }
}

/**
 * Frozen v1 canonical JSON. Arrays retain semantic order; object keys are
 * recursively sorted so equivalent construction order cannot break retries.
 */
export function canonicalizeTemplateImportSemanticPlan(
  value: unknown,
): string {
  return canonicalizeValue(value, new WeakSet<object>())
}

function canonicalizeValue(
  value: unknown,
  stack: WeakSet<object>,
): string {
  if (value === null) return 'null'
  if (typeof value === 'string' || typeof value === 'boolean') {
    return JSON.stringify(value)
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new Error(
        'Template import semantic plan contains a non-finite number',
      )
    }
    return JSON.stringify(value)
  }
  if (Array.isArray(value)) {
    if (stack.has(value)) {
      throw new Error('Template import semantic plan contains a cycle')
    }
    for (let index = 0; index < value.length; index += 1) {
      if (!Object.hasOwn(value, index)) {
        throw new Error(
          'Template import semantic plan contains a sparse or extended array',
        )
      }
    }
    if (Object.keys(value).length !== value.length) {
      throw new Error(
        'Template import semantic plan contains a sparse or extended array',
      )
    }
    stack.add(value)
    try {
      return `[${value
        .map((item) => canonicalizeValue(item, stack))
        .join(',')}]`
    } finally {
      stack.delete(value)
    }
  }
  if (typeof value === 'object') {
    const prototype = Object.getPrototypeOf(value)
    if (prototype !== Object.prototype && prototype !== null) {
      throw new Error(
        'Template import semantic plan contains an unsupported object',
      )
    }
    if (stack.has(value)) {
      throw new Error('Template import semantic plan contains a cycle')
    }
    if (Object.getOwnPropertySymbols(value).length !== 0) {
      throw new Error(
        'Template import semantic plan contains a symbol property',
      )
    }
    const record = value as Record<string, unknown>
    const keys = Object.getOwnPropertyNames(record)
    for (const key of keys) {
      const descriptor = Object.getOwnPropertyDescriptor(record, key)
      if (
        !descriptor?.enumerable ||
        !Object.hasOwn(descriptor, 'value')
      ) {
        throw new Error(
          'Template import semantic plan contains a non-data property',
        )
      }
    }
    stack.add(value)
    try {
      return `{${keys
        .sort()
        .map(
          (key) =>
            `${JSON.stringify(key)}:${canonicalizeValue(record[key], stack)}`,
        )
        .join(',')}}`
    } finally {
      stack.delete(value)
    }
  }
  throw new Error(
    'Template import semantic plan contains an unsupported value',
  )
}
