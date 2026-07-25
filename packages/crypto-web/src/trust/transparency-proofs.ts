const encoder = new TextEncoder()
const STATEMENT_DOMAIN = encoder.encode('worklist.transparency.v1')
const LEAF_PREFIX = 0x00
const NODE_PREFIX = 0x01

export async function computeStatementDigest(params: {
  userId: string
  generation: number
  inviteKey: Uint8Array
}): Promise<Uint8Array> {
  const userBytes = uuidToBytes(params.userId)
  const generationBytes = encodeUint64(params.generation, 'generation')
  const keyLengthBytes = encodeUint32(params.inviteKey.length, 'invite_public_key.length')
  return sha256(
    concatBytes(STATEMENT_DOMAIN, userBytes, generationBytes, keyLengthBytes, params.inviteKey),
  )
}

export async function hashTransparencyLeaf(content: Uint8Array): Promise<Uint8Array> {
  const prefixed = new Uint8Array(1 + content.length)
  prefixed[0] = LEAF_PREFIX
  prefixed.set(content, 1)
  return sha256(prefixed)
}

export async function hashTransparencyNode(
  left: Uint8Array,
  right: Uint8Array,
): Promise<Uint8Array> {
  const prefixed = new Uint8Array(1 + left.length + right.length)
  prefixed[0] = NODE_PREFIX
  prefixed.set(left, 1)
  prefixed.set(right, 1 + left.length)
  return sha256(prefixed)
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

function uuidToBytes(value: string): Uint8Array {
  const normalized = value.replace(/-/g, '').trim().toLowerCase()
  if (!/^[0-9a-f]{32}$/.test(normalized)) {
    throw new Error('user_id must be a valid UUID string')
  }
  const bytes = new Uint8Array(16)
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(normalized.slice(index * 2, index * 2 + 2), 16)
  }
  return bytes
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

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const output = new Uint8Array(parts.reduce((total, part) => total + part.length, 0))
  let offset = 0
  for (const part of parts) {
    output.set(part, offset)
    offset += part.length
  }
  return output
}

function largestPowerOfTwoLessThan(size: number): number {
  let power = 1
  while (power * 2 < size) {
    power *= 2
  }
  return power
}

async function sha256(input: Uint8Array): Promise<Uint8Array> {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error('WebCrypto digest API is unavailable in this environment')
  }
  const digest = await subtle.digest('SHA-256', toArrayBuffer(input))
  return new Uint8Array(digest)
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

function requireHash(value: Uint8Array, field: string): void {
  if (value.length !== 32) {
    throw new Error(`${field} must be 32 bytes`)
  }
}
