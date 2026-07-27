import { hkdfExpand } from '../runtime/hkdf'
import {
  getPublicKeyAsync,
  signAsync,
  verifyAsync,
} from '@noble/ed25519'
import {
  compactTransparencyUserId,
  requireCanonicalTransparencyUserId,
  transparencyUserIdToBytes,
} from './transparency-user-id'

const encoder = new TextEncoder()
const STATEMENT_V2_DOMAIN = encoder.encode('worklist.transparency.statement.v2')
const OWNER_KEY_INFO_PREFIX = 'transparency:owner-signing:v2'

export type OwnerAuthorizedTransparencyStatement = {
  protocolVersion: 2
  userId: string
  generation: number
  invitePublicKey: Uint8Array
  identityPublicKey: Uint8Array
  previousStatementDigest: Uint8Array | null
  statementDigest: Uint8Array
  ownerSignature: Uint8Array
}

export async function deriveOwnerTransparencyIdentityPublicKey(params: {
  dataKey: Uint8Array
  userId: string
}): Promise<Uint8Array> {
  const { seed } = await deriveOwnerIdentitySeed(params)
  try {
    const identityPublicKey = await getPublicKeyAsync(seed)
    requireExactBytes(identityPublicKey, 32, 'identity_public_key')
    return identityPublicKey
  } finally {
    seed.fill(0)
  }
}

export async function createOwnerAuthorizedTransparencyStatement(params: {
  dataKey: Uint8Array
  userId: string
  generation: number
  invitePublicKey: Uint8Array
  previousStatementDigest: Uint8Array | null
}): Promise<OwnerAuthorizedTransparencyStatement> {
  const { seed, userId } = await deriveOwnerIdentitySeed(params)
  try {
    const identityPublicKey = await getPublicKeyAsync(seed)
    requireExactBytes(identityPublicKey, 32, 'identity_public_key')

    const statementDigest = await computeOwnerAuthorizedStatementDigest({
      userId,
      generation: params.generation,
      invitePublicKey: params.invitePublicKey,
      identityPublicKey,
      previousStatementDigest: params.previousStatementDigest,
    })
    const ownerSignature = await signAsync(statementDigest, seed)
    requireExactBytes(ownerSignature, 64, 'owner_signature')

    return {
      protocolVersion: 2,
      userId,
      generation: params.generation,
      invitePublicKey: params.invitePublicKey.slice(),
      identityPublicKey,
      previousStatementDigest: params.previousStatementDigest?.slice() ?? null,
      statementDigest,
      ownerSignature,
    }
  } finally {
    seed.fill(0)
  }
}

async function deriveOwnerIdentitySeed(params: {
  dataKey: Uint8Array
  userId: string
}): Promise<{ seed: Uint8Array; userId: string }> {
  requireExactBytes(params.dataKey, 32, 'data_key')
  const userId = requireCanonicalTransparencyUserId(params.userId)
  const seed = await hkdfExpand({
    parent: params.dataKey,
    info: `${OWNER_KEY_INFO_PREFIX}:${compactTransparencyUserId(userId)}`,
  })
  return { seed, userId }
}

export async function computeOwnerAuthorizedStatementDigest(params: {
  userId: string
  generation: number
  invitePublicKey: Uint8Array
  identityPublicKey: Uint8Array
  previousStatementDigest: Uint8Array | null
}): Promise<Uint8Array> {
  const userBytes = transparencyUserIdToBytes(params.userId)
  const generationBytes = encodeUint64(params.generation, 'generation')
  requireExactBytes(params.invitePublicKey, 32, 'invite_public_key')
  requireExactBytes(params.identityPublicKey, 32, 'identity_public_key')
  if (params.previousStatementDigest !== null) {
    requireExactBytes(params.previousStatementDigest, 32, 'previous_statement_digest')
  }

  return sha256(concatBytes(
    STATEMENT_V2_DOMAIN,
    userBytes,
    generationBytes,
    encodeUint32(params.invitePublicKey.length, 'invite_public_key.length'),
    params.invitePublicKey,
    params.identityPublicKey,
    Uint8Array.of(params.previousStatementDigest === null ? 0 : 1),
    params.previousStatementDigest ?? new Uint8Array(0),
  ))
}

export async function verifyOwnerAuthorizedTransparencyStatement(
  statement: OwnerAuthorizedTransparencyStatement,
): Promise<boolean> {
  if (statement.protocolVersion !== 2) {
    return false
  }
  requireExactBytes(statement.statementDigest, 32, 'statement_digest')
  requireExactBytes(statement.ownerSignature, 64, 'owner_signature')
  const expectedDigest = await computeOwnerAuthorizedStatementDigest(statement)
  if (!constantTimeEqual(expectedDigest, statement.statementDigest)) {
    return false
  }

  return verifyAsync(
    statement.ownerSignature,
    statement.statementDigest,
    statement.identityPublicKey,
    { zip215: false },
  )
}

function encodeUint64(value: number, field: string): Uint8Array {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${field} must be a non-negative safe integer`)
  }
  const view = new DataView(new ArrayBuffer(8))
  view.setBigUint64(0, BigInt(value))
  return new Uint8Array(view.buffer)
}

function encodeUint32(value: number, field: string): Uint8Array {
  if (!Number.isSafeInteger(value) || value < 0 || value > 0xffffffff) {
    throw new Error(`${field} must be a non-negative 32-bit integer`)
  }
  const view = new DataView(new ArrayBuffer(4))
  view.setUint32(0, value)
  return new Uint8Array(view.buffer)
}

function concatBytes(...parts: readonly Uint8Array[]): Uint8Array {
  const output = new Uint8Array(parts.reduce((total, part) => total + part.length, 0))
  let offset = 0
  for (const part of parts) {
    output.set(part, offset)
    offset += part.length
  }
  return output
}

async function sha256(input: Uint8Array): Promise<Uint8Array> {
  const digest = await getSubtleCrypto().digest('SHA-256', toArrayBuffer(input))
  return new Uint8Array(digest)
}

function getSubtleCrypto(): SubtleCrypto {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error('WebCrypto subtle API is unavailable in this environment')
  }
  return subtle
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  if (
    bytes.byteOffset === 0
    && bytes.byteLength === bytes.buffer.byteLength
    && bytes.buffer instanceof ArrayBuffer
  ) {
    return bytes.buffer.slice(0)
  }
  return bytes.slice().buffer
}

function requireExactBytes(value: Uint8Array, length: number, field: string): void {
  if (!(value instanceof Uint8Array) || value.length !== length) {
    throw new Error(`${field} must be exactly ${length} bytes`)
  }
}

function constantTimeEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) {
    return false
  }
  let difference = 0
  for (let index = 0; index < left.length; index += 1) {
    difference |= left[index] ^ right[index]
  }
  return difference === 0
}
