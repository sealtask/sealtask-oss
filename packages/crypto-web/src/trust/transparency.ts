import { decodeBase64 } from '../runtime/base64'
import {
  computeStatementDigest,
  hashTransparencyLeaf,
  hashTransparencyNode,
  reconstructInclusionRoot,
  verifyConsistencyProof,
} from './transparency-proofs'
import {
  computeOwnerAuthorizedStatementDigest,
  verifyOwnerAuthorizedTransparencyStatement,
} from './transparency-authorization'
import { requireCanonicalTransparencyUserId } from './transparency-user-id'

export type TransparencyAuthorizationStatement = {
  userId: string
  generation: number
  invitePublicKey: string
  statementDigest: string
  protocolVersion: number
  identityPublicKey?: string | null
  previousStatementDigest?: string | null
  ownerSignature?: string | null
  createdAt?: string
}

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
    protocolVersion?: number
    identityPublicKey?: string | null
    previousStatementDigest?: string | null
    ownerSignature?: string | null
  }
  authorizationChain?: TransparencyAuthorizationStatement[]
  directorySnapshot?: TransparencyAuthorizationStatement[] | null
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
  trustedConsistencyProof?: {
    fromSize: number
    hashes: string[]
  } | null
  identityInclusionProof?: {
    statementDigest: string
    leafHash: string
    leafIndex: number
    hashes: string[]
  } | null
}

export type TransparencyCheckpoint = {
  size: number
  hash: Uint8Array
}

export interface TransparencyCheckpointStore {
  load(): TransparencyCheckpoint | null
  save(
    checkpoint: TransparencyCheckpoint,
    expectedPrevious: TransparencyCheckpoint | null,
  ): Promise<void>
}

export interface TransparencyIdentityStore {
  load(userId: string): Uint8Array | null
  save(userId: string, identityPublicKey: Uint8Array): Promise<void>
}

export type FetchTransparencyProof = (params: {
  userId: string
  generation?: number
  fromSize: number
  trustedFromSize?: number
}) => Promise<TransparencyProofBundle>

export type PublishTransparencyProof = (params: {
  invitePublicKey: string
  generation: number
  consistencyBaseSize: number
  trustedConsistencyBaseSize: number | null
  protocolVersion: 2
  identityPublicKey: string
  previousStatementDigest: string | null
  ownerSignature: string
}) => Promise<TransparencyProofBundle>

export type InviteKeyVerification = {
  proof: TransparencyProofBundle
  invitePublicKey: Uint8Array
  generation: number
  identityPublicKey: Uint8Array
  authorization: 'owner-authorized'
}

export type LegacyInviteKeyInspection = {
  proof: TransparencyProofBundle
  invitePublicKey: Uint8Array
  generation: number
  identityPublicKey: null
  authorization: 'legacy'
}

export class TransparencyBaseMismatchError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'TransparencyBaseMismatchError'
  }
}

export class InvalidTransparencyCheckpointError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options)
    this.name = 'InvalidTransparencyCheckpointError'
  }
}

export class LegacyTransparencyStatementError extends Error {
  constructor(message = 'Legacy invite-key statements are inspection-only and cannot authenticate invitations.') {
    super(message)
    this.name = 'LegacyTransparencyStatementError'
  }
}

export type TransparencyClientOptions = {
  checkpointStore: TransparencyCheckpointStore
  identityStore: TransparencyIdentityStore
  fetchProof: FetchTransparencyProof
  publishProof?: PublishTransparencyProof
  trustedCheckpoint?: TransparencyCheckpoint
  allowUnanchoredBootstrap?: boolean
}

export class TransparencyClient {
  readonly #checkpointStore: TransparencyCheckpointStore
  readonly #identityStore: TransparencyIdentityStore
  readonly #fetchProof: FetchTransparencyProof
  readonly #publishProof: PublishTransparencyProof | undefined
  readonly #trustedCheckpoint: TransparencyCheckpoint | null
  #transitionQueue: Promise<void> = Promise.resolve()

  constructor(options: TransparencyClientOptions) {
    const retiredOptions = options as TransparencyClientOptions & {
      recoverFromGenesis?: unknown
      isStaleCheckpointError?: unknown
    }
    if (
      Object.prototype.hasOwnProperty.call(retiredOptions, 'recoverFromGenesis')
      || Object.prototype.hasOwnProperty.call(retiredOptions, 'isStaleCheckpointError')
    ) {
      throw new Error(
        'Automatic transparency checkpoint recovery is no longer supported.',
      )
    }
    if (options.trustedCheckpoint && options.allowUnanchoredBootstrap === true) {
      throw new Error(
        'Transparency clients cannot combine a trusted checkpoint with unanchored bootstrap.',
      )
    }
    if (!options.trustedCheckpoint && options.allowUnanchoredBootstrap !== true) {
      throw new Error(
        'Transparency clients require a trusted checkpoint; unanchored bootstrap must be explicit.',
      )
    }
    if (options.trustedCheckpoint) {
      validateCheckpoint(options.trustedCheckpoint, 'trusted transparency checkpoint')
    }
    this.#checkpointStore = options.checkpointStore
    this.#identityStore = options.identityStore
    this.#fetchProof = options.fetchProof
    this.#publishProof = options.publishProof
    this.#trustedCheckpoint = cloneCheckpoint(options.trustedCheckpoint ?? null)
  }

  resolveCurrentInviteKey(params: {
    userId: string
  }): Promise<InviteKeyVerification> {
    const userId = requireCanonicalTransparencyUserId(params.userId)
    return this.#serializeTransition(async () => {
      const storedCheckpoint = this.#loadStoredCheckpoint()
      const previousCheckpoint = this.#verificationCheckpoint(storedCheckpoint)
      const requestedBaseSize = previousCheckpoint?.size ?? 0
      const proof = await this.#fetchProof({
        userId,
        generation: undefined,
        fromSize: requestedBaseSize,
        trustedFromSize: this.#trustedProofBaseSize(requestedBaseSize),
      })
      return requireOwnerAuthorizedVerification(await this.#processProof({
        userId,
        proof,
        expectedStoredCheckpoint: storedCheckpoint,
        previousCheckpoint,
        requestedBaseSize,
        allowLegacyTarget: false,
        operation: 'ordinary-current',
      }))
    })
  }

  verifyHistoricalInviteKey(params: {
    userId: string
    generation: number
  }): Promise<InviteKeyVerification> {
    const userId = requireCanonicalTransparencyUserId(params.userId)
    if (!Number.isSafeInteger(params.generation) || params.generation < 0) {
      throw new Error(
        'Historical invite key generation must be a non-negative safe integer.',
      )
    }
    return this.#serializeTransition(async () => {
      const storedCheckpoint = this.#loadStoredCheckpoint()
      const previousCheckpoint = this.#verificationCheckpoint(storedCheckpoint)
      const requestedBaseSize = previousCheckpoint?.size ?? 0
      const proof = await this.#fetchProof({
        userId,
        generation: params.generation,
        fromSize: requestedBaseSize,
        trustedFromSize: this.#trustedProofBaseSize(requestedBaseSize),
      })
      assertProofGenerationMatchesRequest(proof, params.generation)
      return requireOwnerAuthorizedVerification(await this.#processProof({
        userId,
        proof,
        expectedStoredCheckpoint: storedCheckpoint,
        previousCheckpoint,
        requestedBaseSize,
        allowLegacyTarget: false,
        operation: 'ordinary-historical',
      }))
    })
  }

  /**
   * Verifies a legacy statement only so its owner can publish a signed v2
   * successor. Ordinary recipient encryption must use resolveCurrentInviteKey.
   */
  inspectInviteKeyForMigration(params: {
    userId: string
    generation?: number
  }): Promise<InviteKeyVerification | LegacyInviteKeyInspection> {
    const userId = requireCanonicalTransparencyUserId(params.userId)
    return this.#serializeTransition(async () => {
      const storedCheckpoint = this.#loadStoredCheckpoint()
      const previousCheckpoint = this.#verificationCheckpoint(storedCheckpoint)
      const requestedBaseSize = previousCheckpoint?.size ?? 0
      const proof = await this.#fetchProof({
        userId,
        generation: params.generation,
        fromSize: requestedBaseSize,
        trustedFromSize: this.#trustedProofBaseSize(requestedBaseSize),
      })
      assertProofGenerationMatchesRequest(proof, params.generation)
      return this.#processProof({
        userId,
        proof,
        expectedStoredCheckpoint: storedCheckpoint,
        previousCheckpoint,
        requestedBaseSize,
        allowLegacyTarget: true,
        operation: 'inspection',
      })
    })
  }

  ingestCurrentInviteKeyProof(params: {
    userId: string
    proof: TransparencyProofBundle
    requestedBaseSize?: number
  }): Promise<InviteKeyVerification> {
    const userId = requireCanonicalTransparencyUserId(params.userId)
    return this.#serializeTransition(async () => {
      const storedCheckpoint = this.#loadStoredCheckpoint()
      const previousCheckpoint = this.#verificationCheckpoint(storedCheckpoint)
      return requireOwnerAuthorizedVerification(await this.#processProof({
        userId,
        proof: params.proof,
        expectedStoredCheckpoint: storedCheckpoint,
        previousCheckpoint,
        requestedBaseSize: params.requestedBaseSize ?? previousCheckpoint?.size ?? 0,
        allowLegacyTarget: false,
        operation: 'ordinary-current',
      }))
    })
  }

  publishInviteKey(params: {
    userId: string
    invitePublicKey: string
    generation: number
    protocolVersion: 2
    identityPublicKey: string
    previousStatementDigest: string | null
    ownerSignature: string
  }): Promise<InviteKeyVerification> {
    const userId = requireCanonicalTransparencyUserId(params.userId)
    return this.#serializeTransition(async () => {
      if (!this.#publishProof) {
        throw new Error('Transparency publishing requires an injected publish transport.')
      }
      if (!Number.isSafeInteger(params.generation) || params.generation < 0) {
        throw new Error('Invite key generation must be a non-negative safe integer.')
      }

      const storedCheckpoint = this.#loadStoredCheckpoint()
      const previousCheckpoint = this.#verificationCheckpoint(storedCheckpoint)
      const requestedBaseSize = previousCheckpoint?.size ?? 0
      const proof = await this.#publishProof({
        invitePublicKey: params.invitePublicKey,
        generation: params.generation,
        consistencyBaseSize: requestedBaseSize,
        trustedConsistencyBaseSize:
          this.#trustedProofBaseSize(requestedBaseSize) ?? null,
        protocolVersion: params.protocolVersion,
        identityPublicKey: params.identityPublicKey,
        previousStatementDigest: params.previousStatementDigest,
        ownerSignature: params.ownerSignature,
      })
      assertPublicationResponseMatchesRequest({
        proof,
        userId,
        publication: params,
      })
      return requireOwnerAuthorizedVerification(await this.#processProof({
        userId,
        proof,
        expectedStoredCheckpoint: storedCheckpoint,
        previousCheckpoint,
        requestedBaseSize,
        allowLegacyTarget: false,
        operation: 'publication',
      }))
    })
  }

  getCheckpoint(): TransparencyCheckpoint | null {
    return this.#effectiveCheckpoint(this.#loadStoredCheckpoint())
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
    expectedStoredCheckpoint: TransparencyCheckpoint | null
    previousCheckpoint: TransparencyCheckpoint | null
    requestedBaseSize: number
    allowLegacyTarget: boolean
    operation:
      | 'ordinary-current'
      | 'ordinary-historical'
      | 'inspection'
      | 'publication'
  }): Promise<InviteKeyVerification | LegacyInviteKeyInspection> {
    const verified = await verifyInviteKeyTransparencyProof({
      ...params,
      trustedCheckpoint: this.#trustedCheckpoint,
      requireCurrentDirectory: params.operation === 'ordinary-current',
    })
    if (verified.authorization === 'owner-authorized') {
      if (params.operation === 'inspection') {
        this.#assertPinnedIdentityIfPresent(
          params.userId,
          verified.identityPublicKey,
        )
      } else {
        await this.#assertOrPinIdentity(
          params.userId,
          verified.identityPublicKey,
          verified.firstOwnerAuthorizedLeafIndex,
          params.operation,
        )
      }
    }
    await this.#checkpointStore.save(
      {
        size: verified.logRootSize,
        hash: verified.logRootHash,
      },
      params.expectedStoredCheckpoint,
    )
    const common = {
      proof: params.proof,
      invitePublicKey: verified.invitePublicKey,
      generation: verified.generation,
    }
    return verified.authorization === 'owner-authorized'
      ? {
          ...common,
          identityPublicKey: verified.identityPublicKey,
          authorization: verified.authorization,
        }
      : {
          ...common,
          identityPublicKey: null,
          authorization: verified.authorization,
        }
  }

  #loadStoredCheckpoint(): TransparencyCheckpoint | null {
    const checkpoint = this.#checkpointStore.load()
    if (checkpoint) {
      validateCheckpoint(checkpoint, 'stored transparency checkpoint')
    }
    return cloneCheckpoint(checkpoint)
  }

  #effectiveCheckpoint(
    checkpoint: TransparencyCheckpoint | null,
  ): TransparencyCheckpoint | null {
    if (!this.#trustedCheckpoint) {
      return cloneCheckpoint(checkpoint)
    }
    if (!checkpoint || checkpoint.size < this.#trustedCheckpoint.size) {
      return cloneCheckpoint(this.#trustedCheckpoint)
    }
    if (
      checkpoint.size === this.#trustedCheckpoint.size
      && !bytesEqual(checkpoint.hash, this.#trustedCheckpoint.hash)
    ) {
      throw new InvalidTransparencyCheckpointError(
        'Stored transparency checkpoint conflicts with the independently trusted root.',
      )
    }
    return cloneCheckpoint(checkpoint)
  }

  #verificationCheckpoint(
    checkpoint: TransparencyCheckpoint | null,
  ): TransparencyCheckpoint | null {
    if (
      checkpoint
      && this.#trustedCheckpoint
      && checkpoint.size === this.#trustedCheckpoint.size
      && !bytesEqual(checkpoint.hash, this.#trustedCheckpoint.hash)
    ) {
      throw new InvalidTransparencyCheckpointError(
        'Stored transparency checkpoint conflicts with the independently trusted root.',
      )
    }
    return cloneCheckpoint(checkpoint ?? this.#trustedCheckpoint)
  }

  #trustedProofBaseSize(requestedBaseSize: number): number | undefined {
    return this.#trustedCheckpoint
      && requestedBaseSize !== this.#trustedCheckpoint.size
      ? this.#trustedCheckpoint.size
      : undefined
  }

  #assertPinnedIdentityIfPresent(
    userId: string,
    identityPublicKey: Uint8Array,
  ): void {
    const pinned = this.#identityStore.load(userId)
    if (
      pinned
      && (pinned.length !== 32 || !bytesEqual(pinned, identityPublicKey))
    ) {
      throw new Error(
        'Transparency owner identity changed; the invite key was not accepted.',
      )
    }
  }

  async #assertOrPinIdentity(
    userId: string,
    identityPublicKey: Uint8Array,
    firstOwnerAuthorizedLeafIndex: number,
    operation: 'ordinary-current' | 'ordinary-historical' | 'publication',
  ): Promise<void> {
    const pinned = this.#identityStore.load(userId)
    if (pinned) {
      if (pinned.length !== 32 || !bytesEqual(pinned, identityPublicKey)) {
        throw new Error(
          'Transparency owner identity changed; the invite key was not accepted.',
        )
      }
      return
    }
    if (
      operation !== 'publication'
      && this.#trustedCheckpoint
      && firstOwnerAuthorizedLeafIndex >= this.#trustedCheckpoint.size
    ) {
      throw new Error(
        'The transparency owner identity is not covered by the independently trusted checkpoint.',
      )
    }
    await this.#identityStore.save(userId, identityPublicKey.slice())
    const persisted = this.#identityStore.load(userId)
    if (!persisted || !bytesEqual(persisted, identityPublicKey)) {
      throw new Error('Transparency owner identity could not be durably pinned.')
    }
  }
}

function assertProofGenerationMatchesRequest(
  proof: TransparencyProofBundle,
  requestedGeneration: number | undefined,
): void {
  if (
    requestedGeneration !== undefined
    && proof.statement.generation !== requestedGeneration
  ) {
    throw new Error(
      'Transparency proof generation does not match the requested invite key generation.',
    )
  }
}

function assertPublicationResponseMatchesRequest(params: {
  proof: TransparencyProofBundle
  userId: string
  publication: {
    invitePublicKey: string
    generation: number
    protocolVersion: 2
    identityPublicKey: string
    previousStatementDigest: string | null
    ownerSignature: string
  }
}): void {
  const target = params.proof.statement
  if (
    target.userId !== params.userId
    || target.generation !== params.publication.generation
    || target.protocolVersion !== params.publication.protocolVersion
  ) {
    throw new Error(
      'Transparency publication response does not identify the requested statement.',
    )
  }

  const fields: ReadonlyArray<{
    field: string
    response: string | null | undefined
    requested: string | null
    length: number
  }> = [
    {
      field: 'invitePublicKey',
      response: target.invitePublicKey,
      requested: params.publication.invitePublicKey,
      length: 32,
    },
    {
      field: 'identityPublicKey',
      response: target.identityPublicKey,
      requested: params.publication.identityPublicKey,
      length: 32,
    },
    {
      field: 'previousStatementDigest',
      response: target.previousStatementDigest,
      requested: params.publication.previousStatementDigest,
      length: 32,
    },
    {
      field: 'ownerSignature',
      response: target.ownerSignature,
      requested: params.publication.ownerSignature,
      length: 64,
    },
  ]
  for (const field of fields) {
    if (field.requested === null || field.response == null) {
      if (field.requested !== null || field.response !== null) {
        throw new Error(
          `Transparency publication response ${field.field} does not match the requested statement.`,
        )
      }
      continue
    }
    const responseBytes = decodeFixedLengthField(
      `invite_key_proof.statement.${field.field}`,
      field.response,
      field.length,
    )
    const requestedBytes = decodeFixedLengthField(
      `publication.${field.field}`,
      field.requested,
      field.length,
    )
    if (!bytesEqual(responseBytes, requestedBytes)) {
      throw new Error(
        `Transparency publication response ${field.field} does not match the requested statement.`,
      )
    }
  }
}

type VerifiedTransparencyProof = {
  invitePublicKey: Uint8Array
  generation: number
  logRootHash: Uint8Array
  logRootSize: number
} & (
  | {
      identityPublicKey: Uint8Array
      authorization: 'owner-authorized'
      firstOwnerAuthorizedLeafIndex: number
    }
  | {
      identityPublicKey: null
      authorization: 'legacy'
    }
)

export async function verifyInviteKeyTransparencyProof(params: {
  proof: TransparencyProofBundle
  userId: string
  previousCheckpoint: TransparencyCheckpoint | null
  requestedBaseSize: number
  allowLegacyTarget?: boolean
  trustedCheckpoint?: TransparencyCheckpoint | null
  requireCurrentDirectory?: boolean
}): Promise<VerifiedTransparencyProof> {
  const { proof, previousCheckpoint, requestedBaseSize } = params
  const userId = requireCanonicalTransparencyUserId(params.userId)
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
  const protocolVersion = proof.statement.protocolVersion ?? 1
  if (
    (protocolVersion === 1 && generation > MAX_LEGACY_GENERATION)
    || (
      protocolVersion === 2
      && generation > MAX_OWNER_AUTHORIZED_GENERATION
    )
  ) {
    throw new Error('invite_key_proof.statement.generation is out of range.')
  }
  if (
    protocolVersion === 1
    && (
      invitePublicKey.length < MIN_LEGACY_INVITE_PUBLIC_KEY_LENGTH
      || invitePublicKey.length > MAX_LEGACY_INVITE_PUBLIC_KEY_LENGTH
    )
  ) {
    throw new Error(
      `invite_key_proof.statement.invitePublicKey must decode to between ${MIN_LEGACY_INVITE_PUBLIC_KEY_LENGTH} and ${MAX_LEGACY_INVITE_PUBLIC_KEY_LENGTH} bytes.`,
    )
  }
  if (protocolVersion === 2 && invitePublicKey.length !== 32) {
    throw new Error(
      'invite_key_proof.statement.invitePublicKey must decode to 32 bytes.',
    )
  }
  let identityPublicKey: Uint8Array | null = null
  let firstOwnerAuthorizedStatementDigest: Uint8Array | null = null
  let authorization: 'owner-authorized' | 'legacy'
  let localDigest: Uint8Array
  if (protocolVersion === 1) {
    if (params.allowLegacyTarget !== true) {
      throw new LegacyTransparencyStatementError()
    }
    requireAbsentOwnerAuthorization(proof.statement, 'invite_key_proof.statement')
    localDigest = await computeStatementDigest({
      userId,
      generation,
      inviteKey: invitePublicKey,
    })
    authorization = 'legacy'
  } else if (protocolVersion === 2) {
    const verifiedAuthorization = await verifyTransparencyAuthorizationChain({
      proof,
      userId,
      targetGeneration: generation,
    })
    localDigest = verifiedAuthorization.statementDigest
    identityPublicKey = verifiedAuthorization.identityPublicKey
    firstOwnerAuthorizedStatementDigest =
      verifiedAuthorization.firstOwnerAuthorizedStatementDigest
    authorization = 'owner-authorized'
  } else {
    throw new Error('invite_key_proof.statement.protocolVersion is unsupported.')
  }
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

  await verifyTrustedConsistency({
    proof,
    trustedCheckpoint: params.trustedCheckpoint ?? null,
    requestedBaseSize,
    logRootSize,
    logRootHash,
  })

  let firstOwnerAuthorizedLeafIndex = -1
  if (authorization === 'owner-authorized') {
    firstOwnerAuthorizedLeafIndex = await verifyIdentityInclusionProof({
      proof,
      firstOwnerAuthorizedStatementDigest:
        firstOwnerAuthorizedStatementDigest!,
      logRootSize,
      logRootHash,
    })
  } else if (proof.identityInclusionProof != null) {
    throw new Error(
      'Legacy invite-key proofs cannot contain an owner-identity inclusion proof.',
    )
  }

  if (params.requireCurrentDirectory === true) {
    const currentHead = await verifyCurrentDirectorySnapshot({
      proof,
      userId,
      logRootSize,
      logRootHash,
      trustedCheckpoint: params.trustedCheckpoint ?? null,
      previousCheckpoint,
    })
    if (
      currentHead.generation !== generation
      || !bytesEqual(currentHead.statementDigest, statementDigest)
    ) {
      throw new Error(
        'Transparency directory snapshot does not select the target user’s current authorized key.',
      )
    }
  }

  const common = {
    invitePublicKey,
    generation,
    logRootHash: logRootHash.slice(),
    logRootSize,
  }
  return authorization === 'owner-authorized'
    ? {
        ...common,
        identityPublicKey: identityPublicKey!,
        authorization,
        firstOwnerAuthorizedLeafIndex,
      }
    : {
        ...common,
        identityPublicKey: null,
        authorization,
      }
}

async function verifyTrustedConsistency(params: {
  proof: TransparencyProofBundle
  trustedCheckpoint: TransparencyCheckpoint | null
  requestedBaseSize: number
  logRootSize: number
  logRootHash: Uint8Array
}): Promise<void> {
  const supplied = params.proof.trustedConsistencyProof
  if (!params.trustedCheckpoint) {
    if (supplied != null) {
      throw new Error(
        'Unanchored transparency proofs cannot supply a trusted consistency proof.',
      )
    }
    return
  }
  if (params.logRootSize < params.trustedCheckpoint.size) {
    throw new TransparencyBaseMismatchError(
      'Transparency log size predates the independently trusted checkpoint.',
    )
  }
  if (params.requestedBaseSize === params.trustedCheckpoint.size) {
    if (supplied != null) {
      throw new Error(
        'Transparency proof redundantly supplied a trusted consistency path.',
      )
    }
    return
  }
  if (!supplied) {
    throw new TransparencyBaseMismatchError(
      'Transparency proof is missing continuity from the independently trusted checkpoint.',
    )
  }
  const fromSize = ensureSafeInteger(
    supplied.fromSize,
    'invite_key_proof.trustedConsistencyProof.fromSize',
  )
  if (fromSize !== params.trustedCheckpoint.size) {
    throw new TransparencyBaseMismatchError(
      'Transparency trusted consistency proof uses the wrong checkpoint size.',
    )
  }
  const hashes = supplied.hashes.map((hash, index) =>
    decodeHashField(
      `invite_key_proof.trustedConsistencyProof.hashes[${index}]`,
      hash,
    ))
  if (fromSize === params.logRootSize) {
    if (
      hashes.length !== 0
      || !bytesEqual(params.trustedCheckpoint.hash, params.logRootHash)
    ) {
      throw new TransparencyBaseMismatchError(
        'Transparency proof conflicts with the independently trusted checkpoint.',
      )
    }
    return
  }
  if (hashes.length === 0) {
    throw new Error(
      'Transparency trusted consistency proof is missing required hashes.',
    )
  }
  const { prefixRoot, fullRoot } = await verifyConsistencyProof(
    fromSize,
    params.logRootSize,
    hashes,
  )
  if (!bytesEqual(prefixRoot, params.trustedCheckpoint.hash)) {
    throw new TransparencyBaseMismatchError(
      'Transparency proof does not extend the independently trusted checkpoint.',
    )
  }
  if (!bytesEqual(fullRoot, params.logRootHash)) {
    throw new Error(
      'Transparency trusted consistency proof does not reconstruct the current log root.',
    )
  }
}

async function verifyIdentityInclusionProof(params: {
  proof: TransparencyProofBundle
  firstOwnerAuthorizedStatementDigest: Uint8Array
  logRootSize: number
  logRootHash: Uint8Array
}): Promise<number> {
  const supplied = params.proof.identityInclusionProof
  if (!supplied) {
    throw new Error(
      'Owner-authorized invite keys require an identity-statement inclusion proof.',
    )
  }
  const statementDigest = decodeHashField(
    'invite_key_proof.identityInclusionProof.statementDigest',
    supplied.statementDigest,
  )
  if (
    !bytesEqual(
      statementDigest,
      params.firstOwnerAuthorizedStatementDigest,
    )
  ) {
    throw new Error(
      'Transparency identity inclusion proof does not select the first owner-authorized statement.',
    )
  }
  const leafHash = decodeHashField(
    'invite_key_proof.identityInclusionProof.leafHash',
    supplied.leafHash,
  )
  const expectedLeafHash = await hashTransparencyLeaf(statementDigest)
  if (!bytesEqual(leafHash, expectedLeafHash)) {
    throw new Error(
      'Transparency identity inclusion proof leaf does not match its statement digest.',
    )
  }
  const leafIndex = ensureSafeInteger(
    supplied.leafIndex,
    'invite_key_proof.identityInclusionProof.leafIndex',
  )
  if (leafIndex < 0 || leafIndex >= params.logRootSize) {
    throw new Error(
      'Transparency identity inclusion proof leaf index exceeds the log size.',
    )
  }
  const hashes = supplied.hashes.map((hash, index) =>
    decodeHashField(
      `invite_key_proof.identityInclusionProof.hashes[${index}]`,
      hash,
    ))
  const reconstructedRoot = await reconstructInclusionRoot(
    leafHash,
    leafIndex,
    params.logRootSize,
    hashes,
  )
  if (!bytesEqual(reconstructedRoot, params.logRootHash)) {
    throw new Error(
      'Transparency identity inclusion proof does not match the current log root.',
    )
  }
  return leafIndex
}

const MAX_DIRECTORY_SNAPSHOT_STATEMENTS = 20_000
const MAX_LEGACY_GENERATION = 0x7fff_ffff
const MAX_OWNER_AUTHORIZED_GENERATION = 9_999
const MIN_LEGACY_INVITE_PUBLIC_KEY_LENGTH = 32
const MAX_LEGACY_INVITE_PUBLIC_KEY_LENGTH = 512

async function verifyCurrentDirectorySnapshot(params: {
  proof: TransparencyProofBundle
  userId: string
  logRootSize: number
  logRootHash: Uint8Array
  trustedCheckpoint: TransparencyCheckpoint | null
  previousCheckpoint: TransparencyCheckpoint | null
}): Promise<{
  generation: number
  statementDigest: Uint8Array
}> {
  const snapshot = params.proof.directorySnapshot
  if (!Array.isArray(snapshot)) {
    throw new Error(
      'Current invite-key resolution requires a complete transparency directory snapshot.',
    )
  }
  if (
    snapshot.length !== params.logRootSize
    || snapshot.length === 0
    || snapshot.length > MAX_DIRECTORY_SNAPSHOT_STATEMENTS
  ) {
    throw new Error(
      'Transparency directory snapshot length does not match the bounded log root.',
    )
  }

  const statementsByUser =
    new Map<string, TransparencyAuthorizationStatement[]>()
  const storedDigests: Uint8Array[] = []
  const leafHashes: Uint8Array[] = []
  for (let index = 0; index < snapshot.length; index += 1) {
    const statement = snapshot[index]
    if (!statement) {
      throw new Error(
        `Transparency directory snapshot is missing statement ${index}.`,
      )
    }
    const userId = requireCanonicalTransparencyUserId(
      statement.userId,
      `invite_key_proof.directorySnapshot[${index}].userId`,
    )
    const generation = ensureSafeInteger(
      statement.generation,
      `invite_key_proof.directorySnapshot[${index}].generation`,
    )
    const protocolVersion = ensureSafeInteger(
      statement.protocolVersion,
      `invite_key_proof.directorySnapshot[${index}].protocolVersion`,
    )
    const generationIsValid = protocolVersion === 1
      ? generation >= 0 && generation <= MAX_LEGACY_GENERATION
      : protocolVersion === 2
        ? generation >= 0 && generation <= MAX_OWNER_AUTHORIZED_GENERATION
        : false
    if (!generationIsValid) {
      throw new Error(
        protocolVersion === 1 || protocolVersion === 2
          ? `Transparency directory generation is out of range at index ${index}.`
          : `Transparency directory protocol version is unsupported at index ${index}.`,
      )
    }
    const digest = decodeHashField(
      `invite_key_proof.directorySnapshot[${index}].statementDigest`,
      statement.statementDigest,
    )
    storedDigests.push(digest)
    leafHashes.push(await hashTransparencyLeaf(digest))
    const chain = statementsByUser.get(userId) ?? []
    chain.push(statement)
    statementsByUser.set(userId, chain)
  }

  for (const [userId, globalChain] of statementsByUser) {
    let ownerAuthorizationStarted = false
    for (const statement of globalChain) {
      if (statement.protocolVersion === 2) {
        ownerAuthorizationStarted = true
      } else if (statement.protocolVersion === 1 && ownerAuthorizationStarted) {
        throw new Error(
          `Legacy transparency statement follows owner authorization for ${userId}.`,
        )
      }
    }

    const chain = [...globalChain].sort(compareTransparencyGenerations)
    statementsByUser.set(userId, chain)
    const target = chain.at(-1)!
    const targetGeneration = target.generation
    if (target.protocolVersion === 2) {
      await verifyTransparencyAuthorizationStatements({
        chain,
        userId,
        targetGeneration,
      })
    } else {
      let previousGeneration: number | null = null
      for (let index = 0; index < chain.length; index += 1) {
        const statement = chain[index]
        const generation = ensureSafeInteger(
          statement.generation,
          `invite_key_proof.directorySnapshot.${userId}[${index}].generation`,
        )
        if (
          statement.protocolVersion !== 1
          || generation < 0
          || generation > MAX_LEGACY_GENERATION
          || (previousGeneration !== null && generation <= previousGeneration)
        ) {
          throw new Error(
            `Legacy transparency directory chain is malformed for ${userId}.`,
          )
        }
        requireAbsentOwnerAuthorization(
          statement,
          `invite_key_proof.directorySnapshot.${userId}[${index}]`,
        )
        const invitePublicKey = decodeBoundedLengthField(
          `invite_key_proof.directorySnapshot.${userId}[${index}].invitePublicKey`,
          statement.invitePublicKey,
          MIN_LEGACY_INVITE_PUBLIC_KEY_LENGTH,
          MAX_LEGACY_INVITE_PUBLIC_KEY_LENGTH,
        )
        const expectedDigest = await computeStatementDigest({
          userId,
          generation,
          inviteKey: invitePublicKey,
        })
        const storedDigest = decodeHashField(
          `invite_key_proof.directorySnapshot.${userId}[${index}].statementDigest`,
          statement.statementDigest,
        )
        if (!bytesEqual(expectedDigest, storedDigest)) {
          throw new Error(
            `Legacy transparency directory digest mismatch for ${userId}.`,
          )
        }
        previousGeneration = generation
      }
    }
  }

  const fullRoot = await computeTransparencyTreeRoot(leafHashes)
  if (!bytesEqual(fullRoot, params.logRootHash)) {
    throw new Error(
      'Transparency directory snapshot does not reconstruct the current log root.',
    )
  }
  for (const checkpoint of [
    params.trustedCheckpoint,
    params.previousCheckpoint,
  ]) {
    if (!checkpoint) {
      continue
    }
    if (checkpoint.size > leafHashes.length) {
      throw new TransparencyBaseMismatchError(
        'Transparency directory snapshot predates a required checkpoint.',
      )
    }
    const prefixRoot = await computeTransparencyTreeRoot(
      leafHashes.slice(0, checkpoint.size),
    )
    if (!bytesEqual(prefixRoot, checkpoint.hash)) {
      throw new TransparencyBaseMismatchError(
        'Transparency directory snapshot does not contain a required checkpoint prefix.',
      )
    }
  }

  const targetChain = statementsByUser.get(params.userId)
  if (!targetChain || targetChain.length === 0) {
    throw new Error(
      'Transparency directory snapshot does not contain the invite target.',
    )
  }
  const target = targetChain.at(-1)!
  const targetIndex = snapshot.indexOf(target)
  return {
    generation: target.generation,
    statementDigest: storedDigests[targetIndex],
  }
}

async function computeTransparencyTreeRoot(
  leaves: readonly Uint8Array[],
): Promise<Uint8Array> {
  if (leaves.length === 0) {
    throw new Error('Transparency tree cannot be reconstructed from no leaves.')
  }
  if (leaves.length === 1) {
    return leaves[0].slice()
  }
  const split = largestPowerOfTwoLessThan(leaves.length)
  const [left, right] = await Promise.all([
    computeTransparencyTreeRoot(leaves.slice(0, split)),
    computeTransparencyTreeRoot(leaves.slice(split)),
  ])
  return hashTransparencyNode(left, right)
}

function largestPowerOfTwoLessThan(size: number): number {
  let power = 1
  while (power * 2 < size) {
    power *= 2
  }
  return power
}

async function verifyTransparencyAuthorizationChain(params: {
  proof: TransparencyProofBundle
  userId: string
  targetGeneration: number
}): Promise<{
  statementDigest: Uint8Array
  identityPublicKey: Uint8Array
  firstOwnerAuthorizedStatementDigest: Uint8Array
}> {
  const chain = params.proof.authorizationChain
  if (!Array.isArray(chain) || chain.length === 0) {
    throw new Error('Owner-authorized invite keys require a complete authorization chain.')
  }
  const orderedChain = [...chain].sort(compareTransparencyGenerations)
  const verified = await verifyTransparencyAuthorizationStatements({
    chain: orderedChain,
    userId: params.userId,
    targetGeneration: params.targetGeneration,
  })
  assertTargetMatchesAuthorizationChain(
    params.proof.statement,
    orderedChain.at(-1)!,
  )
  return verified
}

async function verifyTransparencyAuthorizationStatements(params: {
  chain: TransparencyAuthorizationStatement[]
  userId: string
  targetGeneration: number
}): Promise<{
  statementDigest: Uint8Array
  identityPublicKey: Uint8Array
  firstOwnerAuthorizedStatementDigest: Uint8Array
}> {
  const chain = [...params.chain].sort(compareTransparencyGenerations)
  const first = chain[0]
  if (!first) {
    throw new Error('Transparency authorization chain cannot be empty.')
  }
  const firstGeneration = ensureSafeInteger(
    first.generation,
    'invite_key_proof.authorizationChain[0].generation',
  )
  if (
    firstGeneration < 0
    || params.targetGeneration < 0
    || params.targetGeneration > MAX_OWNER_AUTHORIZED_GENERATION
  ) {
    throw new Error('Transparency authorization chain generation is out of range.')
  }

  let previousDigest: Uint8Array | null = null
  let previousGeneration: number | null = null
  let ownerIdentity: Uint8Array | null = null
  let firstOwnerAuthorizedStatementDigest: Uint8Array | null = null
  let ownerAuthorizationStarted = false
  for (let index = 0; index < chain.length; index += 1) {
    const statement = chain[index]
    if (!statement || statement.userId !== params.userId) {
      throw new Error(`Transparency authorization chain user mismatch at generation ${index}.`)
    }
    const generation = ensureSafeInteger(
      statement.generation,
      `invite_key_proof.authorizationChain[${index}].generation`,
    )
    if (previousGeneration !== null && generation <= previousGeneration) {
      throw new Error(
        'Transparency authorization chain generations must be distinct and increasing.',
      )
    }
    const statementDigest = decodeHashField(
      `invite_key_proof.authorizationChain[${index}].statementDigest`,
      statement.statementDigest,
    )
    const protocolVersion = ensureSafeInteger(
      statement.protocolVersion,
      `invite_key_proof.authorizationChain[${index}].protocolVersion`,
    )

    if (protocolVersion === 1) {
      if (generation < 0 || generation > MAX_LEGACY_GENERATION) {
        throw new Error('Legacy transparency generation is out of range.')
      }
      if (ownerAuthorizationStarted) {
        throw new Error('Transparency authorization chains cannot downgrade from v2 to v1.')
      }
      const invitePublicKey = decodeBoundedLengthField(
        `invite_key_proof.authorizationChain[${index}].invitePublicKey`,
        statement.invitePublicKey,
        MIN_LEGACY_INVITE_PUBLIC_KEY_LENGTH,
        MAX_LEGACY_INVITE_PUBLIC_KEY_LENGTH,
      )
      requireAbsentOwnerAuthorization(
        statement,
        `invite_key_proof.authorizationChain[${index}]`,
      )
      const expectedDigest = await computeStatementDigest({
        userId: params.userId,
        generation,
        inviteKey: invitePublicKey,
      })
      if (!bytesEqual(statementDigest, expectedDigest)) {
        throw new Error(`Legacy transparency digest mismatch at generation ${index}.`)
      }
    } else if (protocolVersion === 2) {
      if (generation < 0 || generation > MAX_OWNER_AUTHORIZED_GENERATION) {
        throw new Error('Owner-authorized transparency generation is out of range.')
      }
      const requiredGeneration: number = previousGeneration === null
        ? 0
        : previousGeneration + 1
      if (previousGeneration === null && generation !== 0) {
        throw new Error(
          'An owner-authorized transparency chain without a legacy predecessor must begin at generation 0.',
        )
      }
      if (generation !== requiredGeneration) {
        throw new Error(
          'Owner-authorized transparency suffix contains a generation gap.',
        )
      }
      ownerAuthorizationStarted = true
      const invitePublicKey = decodeFixedLengthField(
        `invite_key_proof.authorizationChain[${index}].invitePublicKey`,
        statement.invitePublicKey,
        32,
      )
      const identityPublicKey = decodeFixedLengthField(
        `invite_key_proof.authorizationChain[${index}].identityPublicKey`,
        statement.identityPublicKey,
        32,
      )
      const ownerSignature = decodeFixedLengthField(
        `invite_key_proof.authorizationChain[${index}].ownerSignature`,
        statement.ownerSignature,
        64,
      )
      const linkedDigest = statement.previousStatementDigest === null
        ? null
        : decodeHashField(
            `invite_key_proof.authorizationChain[${index}].previousStatementDigest`,
            statement.previousStatementDigest,
          )
      if (previousGeneration === null) {
        if (linkedDigest !== null) {
          throw new Error('The initial owner-authorized statement cannot link a predecessor.')
        }
      } else if (!linkedDigest || !previousDigest || !bytesEqual(linkedDigest, previousDigest)) {
        throw new Error(`Transparency rotation link mismatch at generation ${index}.`)
      }
      if (ownerIdentity && !bytesEqual(ownerIdentity, identityPublicKey)) {
        throw new Error('Transparency owner identity changed inside the authorization chain.')
      }
      ownerIdentity ??= identityPublicKey.slice()
      firstOwnerAuthorizedStatementDigest ??= statementDigest.slice()

      const expectedDigest = await computeOwnerAuthorizedStatementDigest({
        userId: params.userId,
        generation,
        invitePublicKey,
        identityPublicKey,
        previousStatementDigest: linkedDigest,
      })
      if (!bytesEqual(statementDigest, expectedDigest)) {
        throw new Error(`Owner-authorized transparency digest mismatch at generation ${index}.`)
      }
      const signatureValid = await verifyOwnerAuthorizedTransparencyStatement({
        protocolVersion: 2,
        userId: params.userId,
        generation,
        invitePublicKey,
        identityPublicKey,
        previousStatementDigest: linkedDigest,
        statementDigest,
        ownerSignature,
      })
      if (!signatureValid) {
        throw new Error(`Owner signature verification failed at generation ${index}.`)
      }
    } else {
      throw new Error(`Unsupported transparency protocol version at generation ${index}.`)
    }
    previousDigest = statementDigest
    previousGeneration = generation
  }

  const target = chain.at(-1)!
  if (!ownerIdentity || target.protocolVersion !== 2) {
    throw new LegacyTransparencyStatementError()
  }
  if (target.generation !== params.targetGeneration) {
    throw new Error(
      'Transparency authorization chain does not reach the requested generation.',
    )
  }
  return {
    statementDigest: previousDigest!,
    identityPublicKey: ownerIdentity,
    firstOwnerAuthorizedStatementDigest:
      firstOwnerAuthorizedStatementDigest!,
  }
}

function assertTargetMatchesAuthorizationChain(
  target: TransparencyProofBundle['statement'],
  chainTarget: TransparencyAuthorizationStatement,
): void {
  const fieldsMatch =
    target.userId === chainTarget.userId
    && target.generation === chainTarget.generation
    && target.invitePublicKey === chainTarget.invitePublicKey
    && target.statementDigest === chainTarget.statementDigest
    && target.protocolVersion === chainTarget.protocolVersion
    && target.identityPublicKey === chainTarget.identityPublicKey
    && target.previousStatementDigest === chainTarget.previousStatementDigest
    && target.ownerSignature === chainTarget.ownerSignature
  if (!fieldsMatch) {
    throw new Error('Transparency proof target does not match its authorization chain.')
  }
}

function requireAbsentOwnerAuthorization(
  statement: {
    identityPublicKey?: string | null
    previousStatementDigest?: string | null
    ownerSignature?: string | null
  },
  field: string,
): void {
  if (
    statement.identityPublicKey != null
    || statement.previousStatementDigest != null
    || statement.ownerSignature != null
  ) {
    throw new Error(`${field} contains owner-authorization fields for a legacy statement.`)
  }
}

function compareTransparencyGenerations(
  left: TransparencyAuthorizationStatement,
  right: TransparencyAuthorizationStatement,
): number {
  return left.generation - right.generation
}

export class MemoryTransparencyCheckpointStore implements TransparencyCheckpointStore {
  #checkpoint: TransparencyCheckpoint | null

  constructor(initialCheckpoint: TransparencyCheckpoint | null = null) {
    this.#checkpoint = cloneCheckpoint(initialCheckpoint)
  }

  load(): TransparencyCheckpoint | null {
    return cloneCheckpoint(this.#checkpoint)
  }

  async save(
    checkpoint: TransparencyCheckpoint,
    expectedPrevious: TransparencyCheckpoint | null,
  ): Promise<void> {
    if (!checkpointsEqual(this.#checkpoint, expectedPrevious)) {
      throw new InvalidTransparencyCheckpointError(
        'Transparency checkpoint changed during proof verification.',
      )
    }
    this.#checkpoint = cloneCheckpoint(checkpoint)
  }

}

export class MemoryTransparencyIdentityStore implements TransparencyIdentityStore {
  readonly #identities = new Map<string, Uint8Array>()

  load(userId: string): Uint8Array | null {
    const canonicalUserId = requireCanonicalTransparencyUserId(userId)
    return this.#identities.get(canonicalUserId)?.slice() ?? null
  }

  async save(userId: string, identityPublicKey: Uint8Array): Promise<void> {
    const canonicalUserId = requireCanonicalTransparencyUserId(userId)
    const existing = this.#identities.get(canonicalUserId)
    if (existing && !bytesEqual(existing, identityPublicKey)) {
      throw new Error(
        'Transparency owner identity changed while it was being pinned.',
      )
    }
    this.#identities.set(canonicalUserId, identityPublicKey.slice())
  }
}

function cloneCheckpoint(checkpoint: TransparencyCheckpoint | null): TransparencyCheckpoint | null {
  return checkpoint
    ? { size: checkpoint.size, hash: checkpoint.hash.slice() }
    : null
}

function checkpointsEqual(
  left: TransparencyCheckpoint | null,
  right: TransparencyCheckpoint | null,
): boolean {
  return (
    left === null
      ? right === null
      : right !== null
        && left.size === right.size
        && bytesEqual(left.hash, right.hash)
  )
}

function requireOwnerAuthorizedVerification(
  verification: InviteKeyVerification | LegacyInviteKeyInspection,
): InviteKeyVerification {
  if (verification.authorization !== 'owner-authorized') {
    throw new LegacyTransparencyStatementError()
  }
  return verification
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
  return decodeFixedLengthField(field, value, 32, 'hash')
}

function decodeFixedLengthField(
  field: string,
  value: string | null | undefined,
  length: number,
  label = 'value',
): Uint8Array {
  const bytes = decodeBase64Field(field, value)
  if (bytes.length !== length) {
    throw new Error(`${field} must decode to a ${length}-byte ${label}.`)
  }
  return bytes
}

function decodeBoundedLengthField(
  field: string,
  value: string | null | undefined,
  minimumLength: number,
  maximumLength: number,
): Uint8Array {
  const bytes = decodeBase64Field(field, value)
  if (bytes.length < minimumLength || bytes.length > maximumLength) {
    throw new Error(
      `${field} must decode to between ${minimumLength} and ${maximumLength} bytes.`,
    )
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

function validateCheckpoint(checkpoint: TransparencyCheckpoint, field: string): void {
  if (
    !Number.isSafeInteger(checkpoint.size)
    || checkpoint.size <= 0
    || !(checkpoint.hash instanceof Uint8Array)
    || checkpoint.hash.length !== 32
  ) {
    throw new InvalidTransparencyCheckpointError(
      `${field} must contain a positive safe tree size and a 32-byte hash; evidence was preserved.`,
    )
  }
}
