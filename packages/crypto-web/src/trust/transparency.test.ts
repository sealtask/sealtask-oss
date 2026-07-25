import { describe, expect, it, vi } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import {
  computeStatementDigest,
  hashTransparencyLeaf,
  hashTransparencyNode,
} from './transparency-proofs'
import {
  MemoryTransparencyCheckpointStore,
  StaleTransparencyCheckpointError,
  TransparencyClient,
  type FetchTransparencyProof,
  type TransparencyProofBundle,
} from './transparency'

const ALICE_ID = '11111111-1111-1111-1111-111111111111'
const BOB_ID = '22222222-2222-2222-2222-222222222222'

describe('transparency proof wire compatibility', () => {
  it('matches the server statement and RFC6962-style leaf hash bytes', async () => {
    const inviteKey = createInviteKeyBytes(0x51)
    const digest = await computeStatementDigest({
      userId: ALICE_ID,
      generation: 3,
      inviteKey,
    })
    const leaf = await hashTransparencyLeaf(digest)

    expect(toHex(digest)).toBe(
      '1986143200531eea3e062a8dd14d8c91e5924751df71e5d10c25a90c53df7323',
    )
    expect(toHex(leaf)).toBe(
      '245d0390672b6979c291ca91dfd09d3e08465fac88a79ae6a149610f53d6b754',
    )
  })
})

describe('TransparencyClient', () => {
  it('verifies a proof before advancing the checkpoint', async () => {
    const proof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 3, inviteKey: createInviteKeyBytes(0x51) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const store = new MemoryTransparencyCheckpointStore()
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(proof)
    const client = new TransparencyClient({ checkpointStore: store, fetchProof })

    const verification = await client.fetchInviteKey({ userId: ALICE_ID })

    expect(verification.generation).toBe(3)
    expect(verification.invitePublicKey).toEqual(createInviteKeyBytes(0x51))
    expect(store.load()).toEqual({
      size: 1,
      hash: expect.any(Uint8Array),
    })
    expect(encodeBase64(store.load()!.hash)).toBe(proof.logRoot.hash)
  })

  it('does not advance the checkpoint for a tampered inclusion root', async () => {
    const proof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x31) }],
      targetIndex: 0,
      baseSize: 0,
    })
    proof.logRoot.hash = encodeBase64(new Uint8Array(32).fill(0xff))
    const store = new MemoryTransparencyCheckpointStore()
    const client = new TransparencyClient({
      checkpointStore: store,
      fetchProof: async () => proof,
    })

    await expect(client.fetchInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/inclusion path does not match/i)
    expect(store.load()).toBeNull()
  })

  it('serializes the full load-fetch-verify-save transition for concurrent lookups', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0xa1) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondProof = await buildProofFixture({
      statements: [
        { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0xa1) },
        { userId: BOB_ID, generation: 2, inviteKey: createInviteKeyBytes(0xb2) },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const requestedBases: number[] = []
    const fetchProof: FetchTransparencyProof = async ({ userId, fromSize }) => {
      requestedBases.push(fromSize)
      await Promise.resolve()
      if (userId === ALICE_ID && fromSize === 0) {
        return firstProof
      }
      if (userId === BOB_ID && fromSize === 1) {
        return secondProof
      }
      throw new Error(`unexpected concurrent transition ${userId}:${fromSize}`)
    }
    const store = new MemoryTransparencyCheckpointStore()
    const client = new TransparencyClient({ checkpointStore: store, fetchProof })

    const [alice, bob] = await Promise.all([
      client.fetchInviteKey({ userId: ALICE_ID }),
      client.fetchInviteKey({ userId: BOB_ID }),
    ])

    expect(alice.generation).toBe(1)
    expect(bob.generation).toBe(2)
    expect(requestedBases).toEqual([0, 1])
    expect(store.load()?.size).toBe(2)
    expect(encodeBase64(store.load()!.hash)).toBe(secondProof.logRoot.hash)
  })

  it('serializes proof ingestion against the checkpoint produced by the prior ingestion', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x11) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondProof = await buildProofFixture({
      statements: [
        { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x11) },
        { userId: BOB_ID, generation: 2, inviteKey: createInviteKeyBytes(0x22) },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const store = new MemoryTransparencyCheckpointStore()
    const client = new TransparencyClient({
      checkpointStore: store,
      fetchProof: async () => {
        throw new Error('fetch is not used for ingestion')
      },
    })

    await Promise.all([
      client.ingestInviteKeyProof({
        userId: ALICE_ID,
        proof: firstProof,
        requestedBaseSize: 0,
      }),
      client.ingestInviteKeyProof({
        userId: BOB_ID,
        proof: secondProof,
        requestedBaseSize: 1,
      }),
    ])

    expect(store.load()?.size).toBe(2)
    expect(encodeBase64(store.load()!.hash)).toBe(secondProof.logRoot.hash)
  })

  it('preserves the shipped one-time recovery from an unexpected proof base', async () => {
    const statements = [
      { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x41) },
      { userId: BOB_ID, generation: 2, inviteKey: createInviteKeyBytes(0x52) },
    ]
    const mismatchedProof = await buildProofFixture({
      statements,
      targetIndex: 1,
      baseSize: 1,
    })
    const genesisProof = await buildProofFixture({
      statements,
      targetIndex: 1,
      baseSize: 0,
    })
    const fetchProof = vi
      .fn<FetchTransparencyProof>()
      .mockResolvedValueOnce(mismatchedProof)
      .mockResolvedValueOnce(genesisProof)
    const store = new MemoryTransparencyCheckpointStore()
    const client = new TransparencyClient({ checkpointStore: store, fetchProof })

    await expect(client.fetchInviteKey({ userId: BOB_ID })).resolves.toMatchObject({
      generation: 2,
    })
    expect(fetchProof).toHaveBeenNthCalledWith(1, {
      userId: BOB_ID,
      generation: undefined,
      fromSize: 0,
    })
    expect(fetchProof).toHaveBeenNthCalledWith(2, {
      userId: BOB_ID,
      generation: undefined,
      fromSize: 0,
    })
  })

  it('publishes and ingests under the same serialized checkpoint transition', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x21) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondKey = createInviteKeyBytes(0x32)
    const secondProof = await buildProofFixture({
      statements: [
        { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x21) },
        { userId: ALICE_ID, generation: 2, inviteKey: secondKey },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const store = new MemoryTransparencyCheckpointStore({
      size: 1,
      hash: decodeProofHash(firstProof.logRoot.hash),
    })
    const publishProof = vi.fn()
      .mockRejectedValueOnce(new StaleTransparencyCheckpointError('stale base'))
      .mockImplementationOnce(async () => {
        const genesisProof = await buildProofFixture({
          statements: [
            { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x21) },
            { userId: ALICE_ID, generation: 2, inviteKey: secondKey },
          ],
          targetIndex: 1,
          baseSize: 0,
        })
        return genesisProof
      })
    const client = new TransparencyClient({
      checkpointStore: store,
      fetchProof: async () => secondProof,
      publishProof,
    })

    const result = await client.publishInviteKey({
      userId: ALICE_ID,
      invitePublicKey: encodeBase64(secondKey),
      generation: 2,
    })

    expect(result.invitePublicKey).toEqual(secondKey)
    expect(publishProof).toHaveBeenNthCalledWith(1, {
      invitePublicKey: encodeBase64(secondKey),
      generation: 2,
      consistencyBaseSize: 1,
    })
    expect(publishProof).toHaveBeenNthCalledWith(2, {
      invitePublicKey: encodeBase64(secondKey),
      generation: 2,
      consistencyBaseSize: 0,
    })
    expect(store.load()?.size).toBe(2)
  })
})

type StatementFixture = {
  userId: string
  generation: number
  inviteKey: Uint8Array
}

async function buildProofFixture(params: {
  statements: StatementFixture[]
  targetIndex: number
  baseSize: number
}): Promise<TransparencyProofBundle> {
  const entries = await Promise.all(
    params.statements.map(async (statement, index) => {
      const digest = await computeStatementDigest(statement)
      const leafHash = await hashTransparencyLeaf(digest)
      return {
        ...statement,
        digest,
        leafHash,
        sequence: index + 1,
      }
    }),
  )
  const tree = await buildTree(entries.map((entry) => entry.leafHash))
  if (!tree) {
    throw new Error('proof fixture requires a non-empty tree')
  }

  const inclusionProof: Uint8Array[] = []
  collectInclusion(tree, params.targetIndex, 0, inclusionProof)
  const consistencyProof: Uint8Array[] = []
  if (params.baseSize !== 0 && params.baseSize !== entries.length) {
    collectConsistency(tree, params.baseSize, consistencyProof)
  }
  const target = entries[params.targetIndex]
  if (!target) {
    throw new Error('targetIndex must reference an existing statement')
  }
  return {
    statement: {
      sequence: target.sequence,
      leafIndex: params.targetIndex,
      userId: target.userId,
      invitePublicKey: encodeBase64(target.inviteKey),
      generation: target.generation,
      statementDigest: encodeBase64(target.digest),
      leafHash: encodeBase64(target.leafHash),
      createdAt: '2025-11-13T00:00:00.000Z',
    },
    logRoot: {
      size: entries.length,
      hash: encodeBase64(tree.hash),
      createdAt: '2025-11-13T00:05:00.000Z',
    },
    inclusionProof: inclusionProof.map(encodeBase64),
    consistencyProof: {
      fromSize: params.baseSize,
      hashes: consistencyProof.map(encodeBase64),
    },
  }
}

type TreeNode = {
  hash: Uint8Array
  size: number
  left: TreeNode | null
  right: TreeNode | null
}

async function buildTree(leaves: Uint8Array[]): Promise<TreeNode | null> {
  if (leaves.length === 0) {
    return null
  }
  return buildRange(0, leaves.length)

  async function buildRange(start: number, end: number): Promise<TreeNode> {
    if (end - start === 1) {
      return { hash: leaves[start], size: 1, left: null, right: null }
    }
    const split = largestPowerOfTwoLessThan(end - start)
    const left = await buildRange(start, start + split)
    const right = await buildRange(start + split, end)
    return {
      hash: await hashTransparencyNode(left.hash, right.hash),
      size: left.size + right.size,
      left,
      right,
    }
  }
}

function collectInclusion(
  node: TreeNode,
  targetIndex: number,
  offset: number,
  proof: Uint8Array[],
): void {
  if (!node.left || !node.right) {
    return
  }
  const leftEnd = offset + node.left.size
  if (targetIndex < leftEnd) {
    collectInclusion(node.left, targetIndex, offset, proof)
    proof.push(node.right.hash)
  } else {
    collectInclusion(node.right, targetIndex, leftEnd, proof)
    proof.push(node.left.hash)
  }
}

function collectConsistency(node: TreeNode, prefixSize: number, proof: Uint8Array[]): void {
  if (prefixSize <= 0 || prefixSize > node.size) {
    throw new Error('invalid fixture consistency prefix')
  }
  if (prefixSize === node.size) {
    proof.push(node.hash)
    return
  }
  if (!node.left || !node.right) {
    throw new Error('fixture consistency proof cannot descend into a leaf')
  }
  if (prefixSize <= node.left.size) {
    collectConsistency(node.left, prefixSize, proof)
    proof.push(node.right.hash)
  } else {
    collectConsistency(node.right, prefixSize - node.left.size, proof)
    proof.push(node.left.hash)
  }
}

function largestPowerOfTwoLessThan(size: number): number {
  let power = 1
  while (power * 2 < size) {
    power *= 2
  }
  return power
}

function createInviteKeyBytes(seed: number): Uint8Array {
  return Uint8Array.from({ length: 32 }, (_, index) => (seed + index) & 0xff)
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

function decodeProofHash(value: string | null | undefined): Uint8Array {
  if (!value) {
    throw new Error('fixture root hash is missing')
  }
  return Uint8Array.from(Buffer.from(value, 'base64'))
}
