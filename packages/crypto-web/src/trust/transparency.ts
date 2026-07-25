import { decodeBase64 } from '../runtime/base64'
import {
  computeStatementDigest,
  hashTransparencyLeaf,
  reconstructInclusionRoot,
  verifyConsistencyProof,
} from './transparency-proofs'

export type TransparencyProofBundle = {
  statement: {
    userId: string
    generation: number
    invitePublicKey: string
    statementDigest: string
    leafHash: string
    leafIndex: number
    sequence?: number
    createdAt?: string
  }
  logRoot: {
    size: number
    hash?: string | null
    createdAt?: string | null
  }
  inclusionProof: string[]
  consistencyProof: {
    fromSize: number
    hashes: string[]
  }
}

export type TransparencyCheckpoint = {
  size: number
  hash: Uint8Array
}

export interface TransparencyCheckpointStore {
  load(): TransparencyCheckpoint | null
  save(checkpoint: TransparencyCheckpoint): void
  clear(): void
}

export type FetchTransparencyProof = (params: {
  userId: string
  generation?: number
  fromSize: number
}) => Promise<TransparencyProofBundle>

export type PublishTransparencyProof = (params: {
  invitePublicKey: string
  generation: number
  consistencyBaseSize: number
}) => Promise<TransparencyProofBundle>

export type InviteKeyVerification = {
  proof: TransparencyProofBundle
  invitePublicKey: Uint8Array
  generation: number
}

export class TransparencyBaseMismatchError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'TransparencyBaseMismatchError'
  }
}

export class StaleTransparencyCheckpointError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options)
    this.name = 'StaleTransparencyCheckpointError'
  }
}

export type TransparencyClientOptions = {
  checkpointStore: TransparencyCheckpointStore
  fetchProof: FetchTransparencyProof
  publishProof?: PublishTransparencyProof
  /**
   * Preserves the shipped recovery behavior. A stale transport response or
   * proof/base mismatch clears the checkpoint and retries once from genesis.
   * Disable this when a deployment requires strict checkpoint pinning.
   */
  recoverFromGenesis?: boolean
  isStaleCheckpointError?: (error: unknown) => boolean
}

export class TransparencyClient {
  readonly #checkpointStore: TransparencyCheckpointStore
  readonly #fetchProof: FetchTransparencyProof
  readonly #publishProof: PublishTransparencyProof | undefined
  readonly #recoverFromGenesis: boolean
  readonly #isStaleCheckpointError: (error: unknown) => boolean
  #transitionQueue: Promise<void> = Promise.resolve()

  constructor(options: TransparencyClientOptions) {
    this.#checkpointStore = options.checkpointStore
    this.#fetchProof = options.fetchProof
    this.#publishProof = options.publishProof
    this.#recoverFromGenesis = options.recoverFromGenesis ?? true
    this.#isStaleCheckpointError =
      options.isStaleCheckpointError
      ?? ((error) => error instanceof StaleTransparencyCheckpointError)
  }

  fetchInviteKey(params: {
    userId: string
    generation?: number
  }): Promise<InviteKeyVerification> {
    return this.#serializeTransition(async () => {
      let previousCheckpoint = this.#loadValidCheckpoint()
      let requestedBaseSize = previousCheckpoint?.size ?? 0
      let genesisRetryConsumed = false

      for (;;) {
        let proof: TransparencyProofBundle
        try {
          proof = await this.#fetchProof({
            userId: params.userId,
            generation: params.generation,
            fromSize: requestedBaseSize,
          })
        } catch (error) {
          if (
            this.#recoverFromGenesis
            && !genesisRetryConsumed
            && requestedBaseSize !== 0
            && this.#isStaleCheckpointError(error)
          ) {
            this.#checkpointStore.clear()
            previousCheckpoint = null
            requestedBaseSize = 0
            genesisRetryConsumed = true
            continue
          }
          throw error
        }

        try {
          return await this.#processProof({
            userId: params.userId,
            proof,
            previousCheckpoint,
            requestedBaseSize,
          })
        } catch (error) {
          if (
            this.#recoverFromGenesis
            && !genesisRetryConsumed
            && error instanceof TransparencyBaseMismatchError
          ) {
            this.#checkpointStore.clear()
            previousCheckpoint = null
            requestedBaseSize = 0
            genesisRetryConsumed = true
            continue
          }
          throw error
        }
      }
    })
  }

  ingestInviteKeyProof(params: {
    userId: string
    proof: TransparencyProofBundle
    requestedBaseSize?: number
  }): Promise<InviteKeyVerification> {
    return this.#serializeTransition(async () => {
      const previousCheckpoint = this.#loadValidCheckpoint()
      return this.#processProof({
        userId: params.userId,
        proof: params.proof,
        previousCheckpoint,
        requestedBaseSize: params.requestedBaseSize ?? previousCheckpoint?.size ?? 0,
      })
    })
  }

  publishInviteKey(params: {
    userId: string
    invitePublicKey: string
    generation: number
  }): Promise<InviteKeyVerification> {
    return this.#serializeTransition(async () => {
      if (!this.#publishProof) {
        throw new Error('Transparency publishing requires an injected publish transport.')
      }
      if (!Number.isSafeInteger(params.generation) || params.generation < 0) {
        throw new Error('Invite key generation must be a non-negative safe integer.')
      }

      let previousCheckpoint = this.#loadValidCheckpoint()
      let requestedBaseSize = previousCheckpoint?.size ?? 0
      let genesisRetryConsumed = false
      for (;;) {
        try {
          const proof = await this.#publishProof({
            invitePublicKey: params.invitePublicKey,
            generation: params.generation,
            consistencyBaseSize: requestedBaseSize,
          })
          return await this.#processProof({
            userId: params.userId,
            proof,
            previousCheckpoint,
            requestedBaseSize,
          })
        } catch (error) {
          if (
            this.#recoverFromGenesis
            && !genesisRetryConsumed
            && requestedBaseSize !== 0
            && this.#isStaleCheckpointError(error)
          ) {
            this.#checkpointStore.clear()
            previousCheckpoint = null
            requestedBaseSize = 0
            genesisRetryConsumed = true
            continue
          }
          throw error
        }
      }
    })
  }

  clearCheckpoint(): Promise<void> {
    return this.#serializeTransition(async () => {
      this.#checkpointStore.clear()
    })
  }

  getCheckpoint(): TransparencyCheckpoint | null {
    return this.#loadValidCheckpoint()
  }

  #serializeTransition<T>(transition: () => Promise<T>): Promise<T> {
    const next = this.#transitionQueue.then(transition, transition)
    this.#transitionQueue = next.then(
      () => undefined,
      () => undefined,
    )
    return next
  }

  async #processProof(params: {
    userId: string
    proof: TransparencyProofBundle
    previousCheckpoint: TransparencyCheckpoint | null
    requestedBaseSize: number
  }): Promise<InviteKeyVerification> {
    const verified = await verifyInviteKeyTransparencyProof(params)
    this.#checkpointStore.save({
      size: verified.logRootSize,
      hash: verified.logRootHash,
    })
    return {
      proof: params.proof,
      invitePublicKey: verified.invitePublicKey,
      generation: verified.generation,
    }
  }

  #loadValidCheckpoint(): TransparencyCheckpoint | null {
    const checkpoint = this.#checkpointStore.load()
    if (!checkpoint) {
      return null
    }
    if (
      !Number.isSafeInteger(checkpoint.size)
      || checkpoint.size <= 0
      || !(checkpoint.hash instanceof Uint8Array)
      || checkpoint.hash.length !== 32
    ) {
      this.#checkpointStore.clear()
      return null
    }
    return {
      size: checkpoint.size,
      hash: checkpoint.hash.slice(),
    }
  }
}

export async function verifyInviteKeyTransparencyProof(params: {
  proof: TransparencyProofBundle
  userId: string
  previousCheckpoint: TransparencyCheckpoint | null
  requestedBaseSize: number
}): Promise<{
  invitePublicKey: Uint8Array
  generation: number
  logRootHash: Uint8Array
  logRootSize: number
}> {
  const { proof, userId, previousCheckpoint, requestedBaseSize } = params
  if (!Number.isSafeInteger(requestedBaseSize) || requestedBaseSize < 0) {
    throw new TransparencyBaseMismatchError(
      'Requested transparency base must be a non-negative safe integer.',
    )
  }
  if (proof.statement.userId !== userId) {
    throw new Error('Invite key proof user_id does not match the invite target.')
  }

  const generation = ensureSafeInteger(
    proof.statement.generation,
    'invite_key_proof.statement.generation',
  )
  if (generation < 0) {
    throw new Error('invite_key_proof.statement.generation must be non-negative.')
  }
  const invitePublicKey = decodeBase64Field(
    'invite_key_proof.statement.invitePublicKey',
    proof.statement.invitePublicKey,
  )
  const statementDigest = decodeHashField(
    'invite_key_proof.statement.statementDigest',
    proof.statement.statementDigest,
  )
  const localDigest = await computeStatementDigest({
    userId,
    generation,
    inviteKey: invitePublicKey,
  })
  if (!bytesEqual(statementDigest, localDigest)) {
    throw new Error('Invite key proof digest does not match the statement payload.')
  }

  const leafHash = decodeHashField(
    'invite_key_proof.statement.leafHash',
    proof.statement.leafHash,
  )
  const computedLeaf = await hashTransparencyLeaf(statementDigest)
  if (!bytesEqual(leafHash, computedLeaf)) {
    throw new Error('Invite key proof leaf hash does not match the statement digest.')
  }

  const logRootSize = ensureSafeInteger(
    proof.logRoot.size,
    'invite_key_proof.logRoot.size',
  )
  if (logRootSize <= 0) {
    throw new Error('invite_key_proof.logRoot.size must be positive.')
  }
  const logRootHash = decodeHashField('invite_key_proof.logRoot.hash', proof.logRoot.hash)
  const leafIndex = ensureSafeInteger(
    proof.statement.leafIndex,
    'invite_key_proof.statement.leafIndex',
  )
  if (leafIndex < 0 || leafIndex >= logRootSize) {
    throw new Error('Invite key proof leaf index exceeds the claimed log size.')
  }

  const inclusionProof = proof.inclusionProof.map((hash, index) =>
    decodeHashField(`invite_key_proof.inclusionProof[${index}]`, hash))
  const reconstructedRoot = await reconstructInclusionRoot(
    leafHash,
    leafIndex,
    logRootSize,
    inclusionProof,
  )
  if (!bytesEqual(reconstructedRoot, logRootHash)) {
    throw new Error('Invite key proof inclusion path does not match the log root hash.')
  }

  const fromSize = ensureSafeInteger(
    proof.consistencyProof.fromSize,
    'invite_key_proof.consistencyProof.from_size',
  )
  if (fromSize !== requestedBaseSize) {
    throw new TransparencyBaseMismatchError(
      'Invite key proof base size does not match the requested consistency base.',
    )
  }
  if (fromSize > logRootSize) {
    throw new Error('Invite key proof consistency base exceeds the log size.')
  }
  if (previousCheckpoint && fromSize !== previousCheckpoint.size) {
    throw new TransparencyBaseMismatchError(
      'Invite key proof base size does not match the last verified root size.',
    )
  }
  if (previousCheckpoint && logRootSize < previousCheckpoint.size) {
    throw new TransparencyBaseMismatchError(
      'Transparency log size regressed compared to the last verified root.',
    )
  }

  const consistencyHashes = proof.consistencyProof.hashes.map((hash, index) =>
    decodeHashField(`invite_key_proof.consistencyProof.hashes[${index}]`, hash))
  if (fromSize === 0) {
    if (consistencyHashes.length > 0) {
      throw new Error('Invite key proof cannot include consistency hashes when base size is 0.')
    }
  } else if (fromSize === logRootSize) {
    if (consistencyHashes.length > 0) {
      throw new Error(
        'Invite key proof consistency hashes must be empty when base matches the log size.',
      )
    }
    if (previousCheckpoint && !bytesEqual(previousCheckpoint.hash, logRootHash)) {
      throw new TransparencyBaseMismatchError(
        'Invite key proof log root hash differs from the last verified root.',
      )
    }
  } else {
    if (consistencyHashes.length === 0) {
      throw new Error(
        'Invite key proof is missing consistency hashes for the requested base size.',
      )
    }
    const { prefixRoot, fullRoot } = await verifyConsistencyProof(
      fromSize,
      logRootSize,
      consistencyHashes,
    )
    if (!bytesEqual(fullRoot, logRootHash)) {
      throw new Error(
        'Invite key proof consistency hashes do not reconstruct the log root hash.',
      )
    }
    if (previousCheckpoint && !bytesEqual(prefixRoot, previousCheckpoint.hash)) {
      throw new TransparencyBaseMismatchError(
        'Invite key proof does not extend the last verified log root.',
      )
    }
  }

  return {
    invitePublicKey,
    generation,
    logRootHash: logRootHash.slice(),
    logRootSize,
  }
}

export class MemoryTransparencyCheckpointStore implements TransparencyCheckpointStore {
  #checkpoint: TransparencyCheckpoint | null

  constructor(initialCheckpoint: TransparencyCheckpoint | null = null) {
    this.#checkpoint = cloneCheckpoint(initialCheckpoint)
  }

  load(): TransparencyCheckpoint | null {
    return cloneCheckpoint(this.#checkpoint)
  }

  save(checkpoint: TransparencyCheckpoint): void {
    this.#checkpoint = cloneCheckpoint(checkpoint)
  }

  clear(): void {
    this.#checkpoint = null
  }
}

function cloneCheckpoint(checkpoint: TransparencyCheckpoint | null): TransparencyCheckpoint | null {
  return checkpoint
    ? { size: checkpoint.size, hash: checkpoint.hash.slice() }
    : null
}

function decodeBase64Field(field: string, value: string | null | undefined): Uint8Array {
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`${field} must be a non-empty base64 string.`)
  }
  try {
    return decodeBase64(value)
  } catch (error) {
    throw new Error(`${field} must be valid base64: ${(error as Error).message}`)
  }
}

function decodeHashField(field: string, value: string | null | undefined): Uint8Array {
  const bytes = decodeBase64Field(field, value)
  if (bytes.length !== 32) {
    throw new Error(`${field} must decode to a 32-byte hash.`)
  }
  return bytes
}

function ensureSafeInteger(value: number, field: string): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value)) {
    throw new Error(`${field} must be a safe integer.`)
  }
  return value
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) {
    return false
  }
  let difference = 0
  for (let index = 0; index < left.length; index += 1) {
    difference |= left[index] ^ right[index]
  }
  return difference === 0
}
