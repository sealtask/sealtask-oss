import { transparencyUserIdToBytes } from './transparency-user-id'
import {
  concatBytes,
  encodeUint32,
  encodeUint64,
  sha256,
} from './transparency-wire'

const encoder = new TextEncoder()
const STATEMENT_DOMAIN = encoder.encode('worklist.transparency.v1')
const LEAF_PREFIX = 0x00
const NODE_PREFIX = 0x01

export async function computeStatementDigest(params: {
  userId: string
  generation: number
  inviteKey: Uint8Array
}): Promise<Uint8Array> {
  const userBytes = transparencyUserIdToBytes(params.userId)
  const generationBytes = encodeUint64(params.generation, 'generation')
  const keyLengthBytes = encodeUint32(params.inviteKey.length, 'invite_public_key.length')
  return sha256(
    concatBytes(STATEMENT_DOMAIN, userBytes, generationBytes, keyLengthBytes, params.inviteKey),
    'WebCrypto digest API is unavailable in this environment',
  )
}

export async function hashTransparencyLeaf(content: Uint8Array): Promise<Uint8Array> {
  const prefixed = new Uint8Array(1 + content.length)
  prefixed[0] = LEAF_PREFIX
  prefixed.set(content, 1)
  return sha256(prefixed, 'WebCrypto digest API is unavailable in this environment')
}

export async function hashTransparencyNode(
  left: Uint8Array,
  right: Uint8Array,
): Promise<Uint8Array> {
  const prefixed = new Uint8Array(1 + left.length + right.length)
  prefixed[0] = NODE_PREFIX
  prefixed.set(left, 1)
  prefixed.set(right, 1 + left.length)
  return sha256(prefixed, 'WebCrypto digest API is unavailable in this environment')
}

export async function reconstructInclusionRoot(
  leafHash: Uint8Array,
  index: number,
  size: number,
  proof: readonly Uint8Array[],
): Promise<Uint8Array> {
  if (!Number.isSafeInteger(size) || size <= 0) {
    throw new Error('inclusion proof cannot target an empty or unsafe tree size')
  }
  if (!Number.isSafeInteger(index) || index < 0 || index >= size) {
    throw new Error('inclusion proof leaf index is out of range for the claimed tree size')
  }

  let proofIndex = 0
  const consumeProofHash = (): Uint8Array => {
    const value = proof[proofIndex]
    if (!value) {
      throw new Error('inclusion proof is missing a required sibling hash')
    }
    requireHash(value, `inclusion proof sibling ${proofIndex}`)
    proofIndex += 1
    return value
  }

  const rebuild = async (
    subtreeSize: number,
    subtreeIndex: number,
    currentHash: Uint8Array,
  ): Promise<Uint8Array> => {
    if (subtreeSize === 1) {
      return currentHash
    }
    const split = largestPowerOfTwoLessThan(subtreeSize)
    const rightSize = subtreeSize - split
    if (subtreeIndex < split) {
      const leftHash = await rebuild(split, subtreeIndex, currentHash)
      return hashTransparencyNode(leftHash, consumeProofHash())
    }
    const rightHash = await rebuild(rightSize, subtreeIndex - split, currentHash)
    return hashTransparencyNode(consumeProofHash(), rightHash)
  }

  requireHash(leafHash, 'leaf hash')
  const root = await rebuild(size, index, leafHash)
  if (proofIndex !== proof.length) {
    throw new Error('inclusion proof has unused hashes')
  }
  return root
}

export async function verifyConsistencyProof(
  prefixSize: number,
  totalSize: number,
  proof: readonly Uint8Array[],
): Promise<{ prefixRoot: Uint8Array; fullRoot: Uint8Array }> {
  if (
    !Number.isSafeInteger(prefixSize)
    || !Number.isSafeInteger(totalSize)
    || prefixSize <= 0
    || prefixSize > totalSize
  ) {
    throw new Error('consistency proof prefix must be within safe tree bounds')
  }
  if (prefixSize === totalSize) {
    if (proof.length !== 1) {
      throw new Error('consistency proof must contain exactly one hash when prefix equals total size')
    }
    requireHash(proof[0], 'consistency proof root')
    return { prefixRoot: proof[0].slice(), fullRoot: proof[0].slice() }
  }

  let proofIndex = 0
  const consumeProofHash = (): Uint8Array => {
    const value = proof[proofIndex]
    if (!value) {
      throw new Error('consistency proof is missing a required hash')
    }
    requireHash(value, `consistency proof hash ${proofIndex}`)
    proofIndex += 1
    return value
  }

  const recurse = async (
    size: number,
    prefix: number,
  ): Promise<{ prefixRoot: Uint8Array; subtreeRoot: Uint8Array }> => {
    if (prefix === size) {
      const hash = consumeProofHash()
      return { prefixRoot: hash, subtreeRoot: hash }
    }
    const split = largestPowerOfTwoLessThan(size)
    const rightSize = size - split
    if (prefix <= split) {
      const { prefixRoot, subtreeRoot: leftHash } = await recurse(split, prefix)
      return {
        prefixRoot,
        subtreeRoot: await hashTransparencyNode(leftHash, consumeProofHash()),
      }
    }
    const { prefixRoot: rightPrefixRoot, subtreeRoot: rightHash } =
      await recurse(rightSize, prefix - split)
    const leftHash = consumeProofHash()
    return {
      prefixRoot: await hashTransparencyNode(leftHash, rightPrefixRoot),
      subtreeRoot: await hashTransparencyNode(leftHash, rightHash),
    }
  }

  const result = await recurse(totalSize, prefixSize)
  if (proofIndex !== proof.length) {
    throw new Error('consistency proof has unused hashes')
  }
  return { prefixRoot: result.prefixRoot, fullRoot: result.subtreeRoot }
}

function largestPowerOfTwoLessThan(size: number): number {
  let power = 1
  while (power * 2 < size) {
    power *= 2
  }
  return power
}

function requireHash(value: Uint8Array, field: string): void {
  if (value.length !== 32) {
    throw new Error(`${field} must be 32 bytes`)
  }
}
