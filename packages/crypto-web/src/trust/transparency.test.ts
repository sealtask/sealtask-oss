import { describe, expect, it, vi } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import {
  computeStatementDigest,
  hashTransparencyLeaf,
  hashTransparencyNode,
} from './transparency-proofs'
import {
  createOwnerAuthorizedTransparencyStatement,
} from './transparency-authorization'
import {
  InvalidTransparencyCheckpointError,
  LegacyTransparencyStatementError,
  MemoryTransparencyCheckpointStore,
  MemoryTransparencyIdentityStore,
  TransparencyBaseMismatchError,
  TransparencyClient,
  type FetchTransparencyProof,
  type TransparencyCheckpointStore,
  type TransparencyClientOptions,
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
    const proof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x51) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const store = new MemoryTransparencyCheckpointStore()
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(proof)
    const client = createClient({ checkpointStore: store, fetchProof })

    const verification = await client.resolveCurrentInviteKey({ userId: ALICE_ID })

    expect(verification.generation).toBe(0)
    expect(verification.authorization).toBe('owner-authorized')
    expect(verification.invitePublicKey).toEqual(createInviteKeyBytes(0x51))
    expect(store.load()).toEqual({
      size: 1,
      hash: expect.any(Uint8Array),
    })
    expect(encodeBase64(store.load()!.hash)).toBe(proof.logRoot.hash)
  })

  it('does not advance the checkpoint for a tampered inclusion root', async () => {
    const proof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x31) }],
      targetIndex: 0,
      baseSize: 0,
    })
    proof.logRoot.hash = encodeBase64(new Uint8Array(32).fill(0xff))
    const save = vi.fn()
    const store: TransparencyCheckpointStore = {
      load: () => null,
      save,
    }
    const client = createClient({
      checkpointStore: store,
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/inclusion path does not match/i)
    expect(store.load()).toBeNull()
    expect(save).not.toHaveBeenCalled()
  })

  it('confines legacy statements to the explicit migration inspection API', async () => {
    const proof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 0, inviteKey: createInviteKeyBytes(0x31) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const store = new MemoryTransparencyCheckpointStore()
    const client = createClient({
      checkpointStore: store,
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toBeInstanceOf(LegacyTransparencyStatementError)
    expect(store.load()).toBeNull()

    const inspection = await client.inspectInviteKeyForMigration({ userId: ALICE_ID })
    expect(inspection.authorization).toBe('legacy')
    expect(inspection.identityPublicKey).toBeNull()
    expect(store.load()?.size).toBe(1)
  })

  it('inspects historically valid wide legacy invite keys for migration', async () => {
    const inviteKey = new Uint8Array(512).fill(0x32)
    const proof = await buildProofFixture({
      statements: [{
        userId: ALICE_ID,
        generation: 0x7fff_ffff,
        inviteKey,
      }],
      targetIndex: 0,
      baseSize: 0,
    })
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.inspectInviteKeyForMigration({ userId: ALICE_ID }))
      .resolves.toMatchObject({
        generation: 0x7fff_ffff,
        invitePublicKey: inviteKey,
        authorization: 'legacy',
      })
  })

  it('accepts an owner-authorized rotation after a nonzero legacy generation baseline', async () => {
    const proof = await buildLegacyBaselineOwnerAuthorizedProofFixture(6)
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .resolves.toMatchObject({
        generation: 6,
        authorization: 'owner-authorized',
      })
  })

  it('rejects a generation gap after a nonzero legacy generation baseline', async () => {
    const proof = await buildLegacyBaselineOwnerAuthorizedProofFixture(7)
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/generation gap|contiguous/i)
  })

  it('rejects a legacy publication after owner authorization starts', async () => {
    const proof = await buildLegacyBaselineOwnerAuthorizedProofFixture(6)
    proof.directorySnapshot = [
      proof.directorySnapshot![0],
      proof.directorySnapshot![2],
      proof.directorySnapshot![1],
    ]
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/follows owner authorization|downgrade/i)
  })

  it('does not let a maximum-generation wide legacy key poison another user', async () => {
    const proof = await buildWideLegacyUnrelatedProofFixture()
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: BOB_ID }))
      .resolves.toMatchObject({
        generation: 0,
        authorization: 'owner-authorized',
      })
  })

  it('still requires a pure owner-authorized chain to begin at generation zero', async () => {
    const proof = await buildOwnerAuthorizedProofFixture({
      statements: [{
        userId: ALICE_ID,
        inviteKey: createInviteKeyBytes(0x32),
        generation: 1,
      }],
      targetIndex: 0,
      baseSize: 0,
    })
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/must begin at generation 0/i)
  })

  it('requires an explicit trust decision at construction time', () => {
    expect(() => new TransparencyClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore: new MemoryTransparencyIdentityStore(),
      fetchProof: vi.fn<FetchTransparencyProof>(),
    })).toThrow(/require a trusted checkpoint/i)
  })

  it('uses the independent checkpoint when durable client history is absent', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 0, inviteKey: createInviteKeyBytes(0x71) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondProof = await buildProofFixture({
      statements: [
        { userId: ALICE_ID, generation: 0, inviteKey: createInviteKeyBytes(0x71) },
        { userId: BOB_ID, generation: 0, inviteKey: createInviteKeyBytes(0x72) },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(secondProof)
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof,
      trustedCheckpoint: {
        size: 1,
        hash: decodeProofHash(firstProof.logRoot.hash),
      },
    })

    await client.inspectInviteKeyForMigration({ userId: BOB_ID })

    expect(fetchProof).toHaveBeenCalledWith({
      userId: BOB_ID,
      generation: undefined,
      fromSize: 1,
    })
  })

  it('rejects a different owner identity after the first verified pin', async () => {
    const firstProof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x81) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const forgedProof = await buildOwnerAuthorizedProofFixture({
      statements: [{
        userId: ALICE_ID,
        inviteKey: createInviteKeyBytes(0x82),
        dataKey: new Uint8Array(32).fill(0xee),
      }],
      targetIndex: 0,
      baseSize: 0,
    })
    const identityStore = new MemoryTransparencyIdentityStore()
    const firstClient = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore,
      fetchProof: async () => firstProof,
    })
    const forgedClient = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore,
      fetchProof: async () => forgedProof,
    })

    await firstClient.resolveCurrentInviteKey({ userId: ALICE_ID })
    await expect(forgedClient.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/owner identity changed/i)
  })

  it('rejects a genuine historical key when the directory contains a newer generation', async () => {
    const proof = await buildOwnerAuthorizedChainProofFixture({
      userId: ALICE_ID,
      inviteKeys: [
        createInviteKeyBytes(0x83),
        createInviteKeyBytes(0x84),
      ],
    })
    const historical = proof.authorizationChain![0]
    proof.statement = {
      ...historical,
      sequence: 1,
      leafIndex: 0,
      leafHash: proof.identityInclusionProof!.leafHash,
    }
    proof.authorizationChain = [historical]
    proof.inclusionProof = proof.identityInclusionProof!.hashes.slice()
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/does not select.*current authorized key/i)
  })

  it('pins a first owner identity only when its first v2 leaf is anchor-covered', async () => {
    const anchorProof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x85) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const currentProof = await buildOwnerAuthorizedProofFixture({
      statements: [
        { userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x85) },
        { userId: BOB_ID, inviteKey: createInviteKeyBytes(0x86) },
      ],
      targetIndex: 0,
      baseSize: 1,
    })
    const identityStore = new MemoryTransparencyIdentityStore()
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore,
      fetchProof: async () => currentProof,
      trustedCheckpoint: {
        size: 1,
        hash: decodeProofHash(anchorProof.logRoot.hash),
      },
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .resolves.toMatchObject({ generation: 0 })
    expect(identityStore.load(ALICE_ID)).toEqual(
      decodeProofHash(currentProof.statement.identityPublicKey!),
    )
  })

  it('rejects an unpinned owner identity introduced after the trusted checkpoint', async () => {
    const anchorProof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: BOB_ID, inviteKey: createInviteKeyBytes(0x87) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const currentProof = await buildOwnerAuthorizedProofFixture({
      statements: [
        { userId: BOB_ID, inviteKey: createInviteKeyBytes(0x87) },
        { userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x88) },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const identityStore = new MemoryTransparencyIdentityStore()
    const checkpointStore = new MemoryTransparencyCheckpointStore()
    const client = createClient({
      checkpointStore,
      identityStore,
      fetchProof: async () => currentProof,
      trustedCheckpoint: {
        size: 1,
        hash: decodeProofHash(anchorProof.logRoot.hash),
      },
    })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toThrow(/not covered by the independently trusted checkpoint/i)
    expect(identityStore.load(ALICE_ID)).toBeNull()
    expect(checkpointStore.load()).toBeNull()
  })

  it('does not pin an identity while merely inspecting it for migration', async () => {
    const proof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x89) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const identityStore = new MemoryTransparencyIdentityStore()
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore,
      fetchProof: async () => proof,
    })

    await expect(client.inspectInviteKeyForMigration({ userId: ALICE_ID }))
      .resolves.toMatchObject({ authorization: 'owner-authorized' })
    expect(identityStore.load(ALICE_ID)).toBeNull()
  })

  it('verifies a complete signed rotation chain and rejects a broken link', async () => {
    const proof = await buildOwnerAuthorizedChainProofFixture({
      userId: ALICE_ID,
      inviteKeys: [
        createInviteKeyBytes(0x91),
        createInviteKeyBytes(0x92),
        createInviteKeyBytes(0x93),
      ],
    })
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => proof,
    })

    await expect(client.verifyHistoricalInviteKey({ userId: ALICE_ID, generation: 2 }))
      .resolves.toMatchObject({
        generation: 2,
        authorization: 'owner-authorized',
      })

    const tamperedProof = structuredClone(proof)
    tamperedProof.authorizationChain![1].previousStatementDigest =
      encodeBase64(new Uint8Array(32).fill(0xff))
    const tamperedClient = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof: async () => tamperedProof,
    })
    await expect(tamperedClient.verifyHistoricalInviteKey({
      userId: ALICE_ID,
      generation: 2,
    }))
      .rejects.toThrow(/rotation link mismatch/i)
  })

  it('rejects a valid historical proof for a different requested generation', async () => {
    const proof = await buildOwnerAuthorizedChainProofFixture({
      userId: ALICE_ID,
      inviteKeys: [
        createInviteKeyBytes(0x94),
        createInviteKeyBytes(0x95),
        createInviteKeyBytes(0x96),
      ],
    })
    const checkpointStore = new MemoryTransparencyCheckpointStore()
    const identityStore = new MemoryTransparencyIdentityStore()
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(proof)
    const client = createClient({
      checkpointStore,
      identityStore,
      fetchProof,
    })

    await expect(client.verifyHistoricalInviteKey({
      userId: ALICE_ID,
      generation: 1,
    })).rejects.toThrow(/generation does not match the requested/i)

    expect(fetchProof).toHaveBeenCalledWith({
      userId: ALICE_ID,
      generation: 1,
      fromSize: 0,
    })
    expect(identityStore.load(ALICE_ID)).toBeNull()
    expect(checkpointStore.load()).toBeNull()
  })

  it('rejects a valid migration proof for a different requested generation', async () => {
    const proof = await buildProofFixture({
      statements: [{
        userId: ALICE_ID,
        generation: 3,
        inviteKey: createInviteKeyBytes(0x97),
      }],
      targetIndex: 0,
      baseSize: 0,
    })
    const checkpointStore = new MemoryTransparencyCheckpointStore()
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(proof)
    const client = createClient({ checkpointStore, fetchProof })

    await expect(client.inspectInviteKeyForMigration({
      userId: ALICE_ID,
      generation: 2,
    })).rejects.toThrow(/generation does not match the requested/i)

    expect(fetchProof).toHaveBeenCalledWith({
      userId: ALICE_ID,
      generation: 2,
      fromSize: 0,
    })
    expect(checkpointStore.load()).toBeNull()
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
    const client = createClient({ checkpointStore: store, fetchProof })

    const [alice, bob] = await Promise.all([
      client.inspectInviteKeyForMigration({ userId: ALICE_ID }),
      client.inspectInviteKeyForMigration({ userId: BOB_ID }),
    ])

    expect(alice.generation).toBe(1)
    expect(bob.generation).toBe(2)
    expect(requestedBases).toEqual([0, 1])
    expect(store.load()?.size).toBe(2)
    expect(encodeBase64(store.load()!.hash)).toBe(secondProof.logRoot.hash)
  })

  it('serializes proof ingestion against the checkpoint produced by the prior ingestion', async () => {
    const firstProof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x11) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondProof = await buildOwnerAuthorizedProofFixture({
      statements: [
        { userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x11) },
        { userId: BOB_ID, inviteKey: createInviteKeyBytes(0x22) },
      ],
      targetIndex: 1,
      baseSize: 1,
    })
    const store = new MemoryTransparencyCheckpointStore()
    const client = createClient({
      checkpointStore: store,
      fetchProof: async () => {
        throw new Error('fetch is not used for ingestion')
      },
    })

    await Promise.all([
      client.ingestCurrentInviteKeyProof({
        userId: ALICE_ID,
        proof: firstProof,
        requestedBaseSize: 0,
      }),
      client.ingestCurrentInviteKeyProof({
        userId: BOB_ID,
        proof: secondProof,
        requestedBaseSize: 1,
      }),
    ])

    expect(store.load()?.size).toBe(2)
    expect(encodeBase64(store.load()!.hash)).toBe(secondProof.logRoot.hash)
  })

  it('fails closed and preserves the checkpoint when a proof has an unexpected base', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x41) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const mismatchedProof = await buildProofFixture({
      statements: [
        { userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x41) },
        { userId: BOB_ID, generation: 2, inviteKey: createInviteKeyBytes(0x52) },
      ],
      targetIndex: 1,
      baseSize: 0,
    })
    const checkpoint = {
      size: 1,
      hash: decodeProofHash(firstProof.logRoot.hash),
    }
    const fetchProof = vi.fn<FetchTransparencyProof>().mockResolvedValue(mismatchedProof)
    const store = new MemoryTransparencyCheckpointStore(checkpoint)
    const client = createClient({ checkpointStore: store, fetchProof })

    await expect(client.inspectInviteKeyForMigration({ userId: BOB_ID }))
      .rejects.toBeInstanceOf(TransparencyBaseMismatchError)
    expect(fetchProof).toHaveBeenCalledTimes(1)
    expect(fetchProof).toHaveBeenCalledWith({
      userId: BOB_ID,
      generation: undefined,
      fromSize: 1,
    })
    expect(store.load()).toEqual(checkpoint)
  })

  it('requires continuity from both an older stored root and a newer release anchor', async () => {
    const statements = [
      { userId: ALICE_ID, generation: 0, inviteKey: createInviteKeyBytes(0x42) },
      { userId: BOB_ID, generation: 0, inviteKey: createInviteKeyBytes(0x43) },
      {
        userId: '33333333-3333-3333-3333-333333333333',
        generation: 0,
        inviteKey: createInviteKeyBytes(0x44),
      },
    ]
    const anchorProof = await buildProofFixture({
      statements: statements.slice(0, 2),
      targetIndex: 1,
      baseSize: 0,
    })
    const currentProof = await buildProofFixture({
      statements,
      targetIndex: 2,
      baseSize: 1,
    })
    currentProof.trustedConsistencyProof =
      await buildTrustedConsistencyProofForStatements(statements, 2)
    const divergentStored = {
      size: 1,
      hash: new Uint8Array(32).fill(0xfe),
    }
    const store = new MemoryTransparencyCheckpointStore(divergentStored)
    const fetchProof = vi.fn<FetchTransparencyProof>()
      .mockResolvedValue(currentProof)
    const client = createClient({
      checkpointStore: store,
      fetchProof,
      trustedCheckpoint: {
        size: 2,
        hash: decodeProofHash(anchorProof.logRoot.hash),
      },
    })

    await expect(client.inspectInviteKeyForMigration({
      userId: statements[2].userId,
    })).rejects.toBeInstanceOf(TransparencyBaseMismatchError)
    expect(fetchProof).toHaveBeenCalledWith({
      userId: statements[2].userId,
      generation: undefined,
      fromSize: 1,
      trustedFromSize: 2,
    })
    expect(store.load()).toEqual(divergentStored)
  })

  it('fails closed and preserves the checkpoint when publishing reports a stale base', async () => {
    const firstProof = await buildProofFixture({
      statements: [{ userId: ALICE_ID, generation: 1, inviteKey: createInviteKeyBytes(0x21) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const secondKey = createInviteKeyBytes(0x32)
    const checkpoint = {
      size: 1,
      hash: decodeProofHash(firstProof.logRoot.hash),
    }
    const staleError = new Error('stale base')
    const store = new MemoryTransparencyCheckpointStore(checkpoint)
    const publishProof = vi.fn().mockRejectedValue(staleError)
    const client = createClient({
      checkpointStore: store,
      fetchProof: async () => {
        throw new Error('fetch is not used for publishing')
      },
      publishProof,
    })

    await expect(client.publishInviteKey({
      userId: ALICE_ID,
      invitePublicKey: encodeBase64(secondKey),
      generation: 2,
      protocolVersion: 2,
      identityPublicKey: encodeBase64(new Uint8Array(32).fill(0x41)),
      previousStatementDigest: encodeBase64(new Uint8Array(32).fill(0x42)),
      ownerSignature: encodeBase64(new Uint8Array(64).fill(0x43)),
    })).rejects.toBe(staleError)

    expect(publishProof).toHaveBeenCalledTimes(1)
    expect(publishProof).toHaveBeenCalledWith({
      invitePublicKey: encodeBase64(secondKey),
      generation: 2,
      consistencyBaseSize: 1,
      trustedConsistencyBaseSize: null,
      protocolVersion: 2,
      identityPublicKey: encodeBase64(new Uint8Array(32).fill(0x41)),
      previousStatementDigest: encodeBase64(new Uint8Array(32).fill(0x42)),
      ownerSignature: encodeBase64(new Uint8Array(64).fill(0x43)),
    })
    expect(store.load()).toEqual(checkpoint)
  })

  it('rejects a publication response for a different valid statement before pinning state', async () => {
    const returnedProof = await buildOwnerAuthorizedProofFixture({
      statements: [{ userId: ALICE_ID, inviteKey: createInviteKeyBytes(0x62) }],
      targetIndex: 0,
      baseSize: 0,
    })
    const requested = await createOwnerAuthorizedTransparencyStatement({
      dataKey: new Uint8Array(32).fill(0x7a),
      userId: ALICE_ID,
      generation: 0,
      invitePublicKey: createInviteKeyBytes(0x61),
      previousStatementDigest: null,
    })
    const checkpointStore = new MemoryTransparencyCheckpointStore()
    const identityStore = new MemoryTransparencyIdentityStore()
    const client = createClient({
      checkpointStore,
      identityStore,
      fetchProof: async () => {
        throw new Error('fetch is not used for publishing')
      },
      publishProof: vi.fn().mockResolvedValue(returnedProof),
    })

    await expect(client.publishInviteKey({
      userId: requested.userId,
      invitePublicKey: encodeBase64(requested.invitePublicKey),
      generation: requested.generation,
      protocolVersion: 2,
      identityPublicKey: encodeBase64(requested.identityPublicKey),
      previousStatementDigest: null,
      ownerSignature: encodeBase64(requested.ownerSignature),
    })).rejects.toThrow(/does not match the requested statement/i)

    expect(checkpointStore.load()).toBeNull()
    expect(identityStore.load(ALICE_ID)).toBeNull()
  })

  it.each([
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'.toUpperCase(),
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    'aaaaaaaa-aaaaaaaa-aaaa-aaaa-aaaaaaaa',
  ])('rejects a noncanonical UUID alias before fetching: %s', async (userId) => {
    const fetchProof = vi.fn<FetchTransparencyProof>()
    const client = createClient({
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      fetchProof,
    })

    expect(() => client.resolveCurrentInviteKey({ userId }))
      .toThrow(/canonical lowercase hyphenated UUID/i)
    expect(fetchProof).not.toHaveBeenCalled()
  })

  it('rejects malformed stored checkpoints without deleting the evidence', async () => {
    const fetchProof = vi.fn<FetchTransparencyProof>()
    const store: TransparencyCheckpointStore = {
      load: () => ({ size: 0, hash: new Uint8Array(31) }),
      save: vi.fn(),
    }
    const client = createClient({ checkpointStore: store, fetchProof })

    await expect(client.resolveCurrentInviteKey({ userId: ALICE_ID }))
      .rejects.toBeInstanceOf(InvalidTransparencyCheckpointError)
    expect(fetchProof).not.toHaveBeenCalled()
  })

  it('rejects retired automatic-recovery options', () => {
    const options = {
      checkpointStore: new MemoryTransparencyCheckpointStore(),
      identityStore: new MemoryTransparencyIdentityStore(),
      fetchProof: vi.fn<FetchTransparencyProof>(),
      allowUnanchoredBootstrap: true,
      recoverFromGenesis: false,
    } as unknown as ConstructorParameters<typeof TransparencyClient>[0]

    expect(() => new TransparencyClient(options))
      .toThrow(/automatic transparency checkpoint recovery is no longer supported/i)
  })
})

type StatementFixture = {
  userId: string
  generation: number
  inviteKey: Uint8Array
}

type TestTransparencyClientOptions =
  Omit<TransparencyClientOptions, 'identityStore' | 'allowUnanchoredBootstrap'>
  & {
    identityStore?: MemoryTransparencyIdentityStore
    allowUnanchoredBootstrap?: boolean
  }

function createClient(options: TestTransparencyClientOptions): TransparencyClient {
  const trust = options.trustedCheckpoint
    ? { allowUnanchoredBootstrap: false }
    : { allowUnanchoredBootstrap: options.allowUnanchoredBootstrap ?? true }
  return new TransparencyClient({
    identityStore: options.identityStore ?? new MemoryTransparencyIdentityStore(),
    ...options,
    ...trust,
  })
}

type OwnerAuthorizedStatementFixture = {
  userId: string
  inviteKey: Uint8Array
  dataKey?: Uint8Array
  generation?: number
}

async function buildOwnerAuthorizedProofFixture(params: {
  statements: OwnerAuthorizedStatementFixture[]
  targetIndex: number
  baseSize: number
}): Promise<TransparencyProofBundle> {
  const entries = await Promise.all(
    params.statements.map(async (statement, index) => {
      const generation = statement.generation ?? 0
      const authorization = await createOwnerAuthorizedTransparencyStatement({
        dataKey: statement.dataKey ?? new Uint8Array(32).fill(index + 1),
        userId: statement.userId,
        generation,
        invitePublicKey: statement.inviteKey,
        previousStatementDigest: null,
      })
      return {
        ...statement,
        authorization,
        digest: authorization.statementDigest,
        leafHash: await hashTransparencyLeaf(authorization.statementDigest),
        sequence: index + 1,
      }
    }),
  )
  const tree = await buildTree(entries.map((entry) => entry.leafHash))
  if (!tree) {
    throw new Error('proof fixture requires a non-empty tree')
  }
  const target = entries[params.targetIndex]
  if (!target) {
    throw new Error('targetIndex must reference an existing statement')
  }
  const inclusionProof: Uint8Array[] = []
  collectInclusion(tree, params.targetIndex, 0, inclusionProof)
  const consistencyProof: Uint8Array[] = []
  if (params.baseSize !== 0 && params.baseSize !== entries.length) {
    collectConsistency(tree, params.baseSize, consistencyProof)
  }
  const directorySnapshot = entries.map((entry) => ({
    userId: entry.userId,
    generation: entry.authorization.generation,
    invitePublicKey: encodeBase64(entry.inviteKey),
    statementDigest: encodeBase64(entry.authorization.statementDigest),
    protocolVersion: 2,
    identityPublicKey: encodeBase64(entry.authorization.identityPublicKey),
    previousStatementDigest: null,
    ownerSignature: encodeBase64(entry.authorization.ownerSignature),
    createdAt: '2026-07-26T00:00:00.000Z',
  }))
  const wireAuthorization = directorySnapshot[params.targetIndex]
  return {
    statement: {
      ...wireAuthorization,
      sequence: target.sequence,
      leafIndex: params.targetIndex,
      leafHash: encodeBase64(target.leafHash),
    },
    authorizationChain: [wireAuthorization],
    directorySnapshot,
    logRoot: {
      size: entries.length,
      hash: encodeBase64(tree.hash),
      createdAt: '2026-07-26T00:05:00.000Z',
    },
    inclusionProof: inclusionProof.map(encodeBase64),
    consistencyProof: {
      fromSize: params.baseSize,
      hashes: consistencyProof.map(encodeBase64),
    },
    trustedConsistencyProof: null,
    identityInclusionProof: {
      statementDigest: wireAuthorization.statementDigest,
      leafHash: encodeBase64(target.leafHash),
      leafIndex: params.targetIndex,
      hashes: inclusionProof.map(encodeBase64),
    },
  }
}

async function buildOwnerAuthorizedChainProofFixture(params: {
  userId: string
  inviteKeys: Uint8Array[]
}): Promise<TransparencyProofBundle> {
  if (params.inviteKeys.length === 0) {
    throw new Error('owner authorization chain fixture cannot be empty')
  }
  const dataKey = new Uint8Array(32).fill(0x2a)
  const entries: Array<{
    authorization: Awaited<ReturnType<typeof createOwnerAuthorizedTransparencyStatement>>
    leafHash: Uint8Array
  }> = []
  let previousStatementDigest: Uint8Array | null = null
  for (let generation = 0; generation < params.inviteKeys.length; generation += 1) {
    const authorization = await createOwnerAuthorizedTransparencyStatement({
      dataKey,
      userId: params.userId,
      generation,
      invitePublicKey: params.inviteKeys[generation],
      previousStatementDigest,
    })
    entries.push({
      authorization,
      leafHash: await hashTransparencyLeaf(authorization.statementDigest),
    })
    previousStatementDigest = authorization.statementDigest
  }
  const tree = await buildTree(entries.map((entry) => entry.leafHash))
  if (!tree) {
    throw new Error('failed to build owner authorization chain tree')
  }
  const targetIndex = entries.length - 1
  const target = entries[targetIndex]
  const inclusionProof: Uint8Array[] = []
  collectInclusion(tree, targetIndex, 0, inclusionProof)
  const identityInclusionProof: Uint8Array[] = []
  collectInclusion(tree, 0, 0, identityInclusionProof)
  const authorizationChain = entries.map(({ authorization }, generation) => ({
    userId: authorization.userId,
    generation,
    invitePublicKey: encodeBase64(authorization.invitePublicKey),
    statementDigest: encodeBase64(authorization.statementDigest),
    protocolVersion: 2,
    identityPublicKey: encodeBase64(authorization.identityPublicKey),
    previousStatementDigest:
      authorization.previousStatementDigest === null
        ? null
        : encodeBase64(authorization.previousStatementDigest),
    ownerSignature: encodeBase64(authorization.ownerSignature),
    createdAt: '2026-07-26T00:00:00.000Z',
  }))
  return {
    statement: {
      ...authorizationChain[targetIndex],
      sequence: targetIndex + 1,
      leafIndex: targetIndex,
      leafHash: encodeBase64(target.leafHash),
    },
    authorizationChain,
    directorySnapshot: authorizationChain,
    logRoot: {
      size: entries.length,
      hash: encodeBase64(tree.hash),
      createdAt: '2026-07-26T00:05:00.000Z',
    },
    inclusionProof: inclusionProof.map(encodeBase64),
    consistencyProof: {
      fromSize: 0,
      hashes: [],
    },
    trustedConsistencyProof: null,
    identityInclusionProof: {
      statementDigest: authorizationChain[0].statementDigest,
      leafHash: encodeBase64(entries[0].leafHash),
      leafIndex: 0,
      hashes: identityInclusionProof.map(encodeBase64),
    },
  }
}

async function buildLegacyBaselineOwnerAuthorizedProofFixture(
  ownerGeneration: number,
): Promise<TransparencyProofBundle> {
  const highLegacyGeneration = 5
  const highLegacyInviteKey = new Uint8Array(512).fill(0x34)
  const highLegacyDigest = await computeStatementDigest({
    userId: ALICE_ID,
    generation: highLegacyGeneration,
    inviteKey: highLegacyInviteKey,
  })
  const lowLegacyGeneration = 1
  const lowLegacyInviteKey = createInviteKeyBytes(0x33)
  const lowLegacyDigest = await computeStatementDigest({
    userId: ALICE_ID,
    generation: lowLegacyGeneration,
    inviteKey: lowLegacyInviteKey,
  })
  const ownerAuthorization = await createOwnerAuthorizedTransparencyStatement({
    dataKey: new Uint8Array(32).fill(0x35),
    userId: ALICE_ID,
    generation: ownerGeneration,
    invitePublicKey: createInviteKeyBytes(0x36),
    previousStatementDigest: highLegacyDigest,
  })
  const leafHashes = await Promise.all([
    hashTransparencyLeaf(highLegacyDigest),
    hashTransparencyLeaf(lowLegacyDigest),
    hashTransparencyLeaf(ownerAuthorization.statementDigest),
  ])
  const tree = await buildTree(leafHashes)
  if (!tree) {
    throw new Error('failed to build migrated authorization chain tree')
  }
  const inclusionProof: Uint8Array[] = []
  collectInclusion(tree, 2, 0, inclusionProof)
  const highLegacyStatement = {
    userId: ALICE_ID,
    generation: highLegacyGeneration,
    invitePublicKey: encodeBase64(highLegacyInviteKey),
    statementDigest: encodeBase64(highLegacyDigest),
    protocolVersion: 1,
    identityPublicKey: null,
    previousStatementDigest: null,
    ownerSignature: null,
    createdAt: '2026-07-26T00:00:00.000Z',
  }
  const lowLegacyStatement = {
    userId: ALICE_ID,
    generation: lowLegacyGeneration,
    invitePublicKey: encodeBase64(lowLegacyInviteKey),
    statementDigest: encodeBase64(lowLegacyDigest),
    protocolVersion: 1,
    identityPublicKey: null,
    previousStatementDigest: null,
    ownerSignature: null,
    createdAt: '2026-07-26T00:00:30.000Z',
  }
  const ownerStatement = {
    userId: ALICE_ID,
    generation: ownerGeneration,
    invitePublicKey: encodeBase64(ownerAuthorization.invitePublicKey),
    statementDigest: encodeBase64(ownerAuthorization.statementDigest),
    protocolVersion: 2,
    identityPublicKey: encodeBase64(ownerAuthorization.identityPublicKey),
    previousStatementDigest: encodeBase64(highLegacyDigest),
    ownerSignature: encodeBase64(ownerAuthorization.ownerSignature),
    createdAt: '2026-07-26T00:01:00.000Z',
  }

  return {
    statement: {
      ...ownerStatement,
      sequence: 3,
      leafIndex: 2,
      leafHash: encodeBase64(leafHashes[2]),
    },
    authorizationChain: [
      highLegacyStatement,
      lowLegacyStatement,
      ownerStatement,
    ],
    directorySnapshot: [
      highLegacyStatement,
      lowLegacyStatement,
      ownerStatement,
    ],
    logRoot: {
      size: 3,
      hash: encodeBase64(tree.hash),
      createdAt: '2026-07-26T00:05:00.000Z',
    },
    inclusionProof: inclusionProof.map(encodeBase64),
    consistencyProof: {
      fromSize: 0,
      hashes: [],
    },
    trustedConsistencyProof: null,
    identityInclusionProof: {
      statementDigest: ownerStatement.statementDigest,
      leafHash: encodeBase64(leafHashes[2]),
      leafIndex: 2,
      hashes: inclusionProof.map(encodeBase64),
    },
  }
}

async function buildWideLegacyUnrelatedProofFixture(): Promise<TransparencyProofBundle> {
  const legacyInviteKey = new Uint8Array(512).fill(0x71)
  const legacyDigest = await computeStatementDigest({
    userId: ALICE_ID,
    generation: 0x7fff_ffff,
    inviteKey: legacyInviteKey,
  })
  const ownerAuthorization = await createOwnerAuthorizedTransparencyStatement({
    dataKey: new Uint8Array(32).fill(0x72),
    userId: BOB_ID,
    generation: 0,
    invitePublicKey: createInviteKeyBytes(0x73),
    previousStatementDigest: null,
  })
  const leafHashes = await Promise.all([
    hashTransparencyLeaf(legacyDigest),
    hashTransparencyLeaf(ownerAuthorization.statementDigest),
  ])
  const tree = await buildTree(leafHashes)
  if (!tree) {
    throw new Error('failed to build unrelated legacy directory fixture')
  }
  const inclusionProof: Uint8Array[] = []
  collectInclusion(tree, 1, 0, inclusionProof)
  const legacyStatement = {
    userId: ALICE_ID,
    generation: 0x7fff_ffff,
    invitePublicKey: encodeBase64(legacyInviteKey),
    statementDigest: encodeBase64(legacyDigest),
    protocolVersion: 1,
    identityPublicKey: null,
    previousStatementDigest: null,
    ownerSignature: null,
    createdAt: '2026-07-26T00:00:00.000Z',
  }
  const ownerStatement = {
    userId: BOB_ID,
    generation: 0,
    invitePublicKey: encodeBase64(ownerAuthorization.invitePublicKey),
    statementDigest: encodeBase64(ownerAuthorization.statementDigest),
    protocolVersion: 2,
    identityPublicKey: encodeBase64(ownerAuthorization.identityPublicKey),
    previousStatementDigest: null,
    ownerSignature: encodeBase64(ownerAuthorization.ownerSignature),
    createdAt: '2026-07-26T00:01:00.000Z',
  }

  return {
    statement: {
      ...ownerStatement,
      sequence: 2,
      leafIndex: 1,
      leafHash: encodeBase64(leafHashes[1]),
    },
    authorizationChain: [ownerStatement],
    directorySnapshot: [legacyStatement, ownerStatement],
    logRoot: {
      size: 2,
      hash: encodeBase64(tree.hash),
      createdAt: '2026-07-26T00:05:00.000Z',
    },
    inclusionProof: inclusionProof.map(encodeBase64),
    consistencyProof: {
      fromSize: 0,
      hashes: [],
    },
    trustedConsistencyProof: null,
    identityInclusionProof: {
      statementDigest: ownerStatement.statementDigest,
      leafHash: encodeBase64(leafHashes[1]),
      leafIndex: 1,
      hashes: inclusionProof.map(encodeBase64),
    },
  }
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

async function buildTrustedConsistencyProofForStatements(
  statements: StatementFixture[],
  fromSize: number,
): Promise<{ fromSize: number; hashes: string[] }> {
  const leaves = await Promise.all(statements.map(async (statement) =>
    hashTransparencyLeaf(await computeStatementDigest(statement))))
  const tree = await buildTree(leaves)
  if (!tree) {
    throw new Error('trusted consistency fixture requires statements')
  }
  const hashes: Uint8Array[] = []
  if (fromSize !== tree.size) {
    collectConsistency(tree, fromSize, hashes)
  }
  return {
    fromSize,
    hashes: hashes.map(encodeBase64),
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
