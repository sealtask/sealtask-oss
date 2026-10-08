import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'

const encoder = new TextEncoder()
const DOMAIN = 'worklist.project-duplication.local-attempt.v1'

async function storageKey(dataKey: Uint8Array, userId: string): Promise<Uint8Array> {
  if (dataKey.byteLength !== 32) throw new Error('Invalid project-copy storage key')
  return hkdfExpand({ parent: dataKey, info: `${DOMAIN}.key\0${userId}`, length: 32 })
}

export async function projectDuplicationStorageSlot(dataKey: Uint8Array, userId: string, sourceId: string): Promise<string> {
  const key = await storageKey(dataKey, userId)
  try {
    return `${userId}:${encodeBase64(await hmacSha256(key, encoder.encode(sourceId), 'Project copy storage requires HMAC'))}`
  } finally { key.fill(0) }
}

export async function sealProjectDuplicationAttempt(params: {
  dataKey: Uint8Array; userId: string; header: string; plaintext: string; strongBox?: StrongBoxBridge
}): Promise<string> {
  const key = await storageKey(params.dataKey, params.userId)
  const plaintext = encoder.encode(params.plaintext)
  try {
    const bridge = params.strongBox ?? await getStrongBoxBridge()
    return encodeBase64(await bridge.encrypt({ key, context: encoder.encode(`${DOMAIN}\0${params.header}`), plaintext }))
  } finally { key.fill(0); plaintext.fill(0) }
}

export async function openProjectDuplicationAttempt(params: {
  dataKey: Uint8Array; userId: string; header: string; ciphertext: string; strongBox?: StrongBoxBridge
}): Promise<string> {
  const key = await storageKey(params.dataKey, params.userId)
  let plaintext: Uint8Array | undefined
  try {
    const bridge = params.strongBox ?? await getStrongBoxBridge()
    plaintext = await bridge.decrypt({ key, context: encoder.encode(`${DOMAIN}\0${params.header}`), ciphertext: decodeBase64(params.ciphertext) })
    return new TextDecoder('utf-8', { fatal: true }).decode(plaintext)
  } finally { key.fill(0); plaintext?.fill(0) }
}
