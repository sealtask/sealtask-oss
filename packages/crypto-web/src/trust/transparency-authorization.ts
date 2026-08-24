import { constantTimeEquals } from '../runtime/bytes'
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
import {
  concatBytes,
  encodeUint32,
  encodeUint64,
  sha256,
} from './transparency-wire'

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

  return sha256(
    concatBytes(
      STATEMENT_V2_DOMAIN,
      userBytes,
      generationBytes,
      encodeUint32(params.invitePublicKey.length, 'invite_public_key.length'),
      params.invitePublicKey,
      params.identityPublicKey,
      Uint8Array.of(params.previousStatementDigest === null ? 0 : 1),
      params.previousStatementDigest ?? new Uint8Array(0),
    ),
    'WebCrypto subtle API is unavailable in this environment',
  )
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
  if (!constantTimeEquals(expectedDigest, statement.statementDigest)) {
    return false
  }

  return verifyAsync(
    statement.ownerSignature,
    statement.statementDigest,
    statement.identityPublicKey,
    { zip215: false },
  )
}

function requireExactBytes(value: Uint8Array, length: number, field: string): void {
  if (!(value instanceof Uint8Array) || value.length !== length) {
    throw new Error(`${field} must be exactly ${length} bytes`)
  }
}
