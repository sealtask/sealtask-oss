import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import {
  constantTimeEquals,
  toArrayBuffer,
  toUint8Array,
  zeroBytes,
} from '../runtime/bytes'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import {
  computeKeyFingerprint,
  hpkeOpen,
} from '../runtime/hpke'
import type { StrongBoxBridge } from '../runtime/strong-box'
import { deriveInviteKeyPair } from '../protocols/invite-key'
import { parseSealedPayload } from '../protocols/sealed-payload'
import {
  sealWorkListKeyForOwner,
  type ProjectKeyWriteFormat,
} from '../protocols/work-list'
import {
  INVITE_PREVIEW_AUTH_KIND,
  INVITE_PREVIEW_AUTH_SCHEME,
  buildInvitePreviewAuthMessageFromPackageBody,
  isSupportedInvitePreviewAuthVersion,
  verifyInvitePreviewAuthenticator,
  type InvitePreviewAuthenticator,
  type InvitePreviewAuthSignedBody,
  type InviteProtocolVersion,
  type InviteRole,
} from './invite-auth'

const encoder = new TextEncoder()
const INVITE_MEMBER_CONTEXT = encoder.encode('worklist.invite.member')
const MEMBERSHIP_PROOF_LABEL = 'accept:'
const AUTH_DIGEST_BYTES = 32
const PUBLIC_KEY_BYTES = 32
const VERIFIED_INVITE_AUTHORITY = Symbol('VerifiedPendingInvitation')

export type PendingInvitationCrypto = {
  membershipId: string
  workListId: string
  role: string
  invitationState?: string | null
  expiresAt?: string | null
  inviteProtocolVersion?: InviteProtocolVersion
  reservationRevision?: number
  workListKeyCiphertext: string
  invitePackageCiphertext?: string | null
  recipientCiphertext?: string | null
  saltMember?: string | null
}

export type InviteKeyResolver = (params: {
  userId: string
  generation: number
}) => Promise<{
  invitePublicKey: Uint8Array
  generation: number
  identityPublicKey: Uint8Array
  authorization: 'owner-authorized'
}>

export type PendingInvitationPreview =
  | {
    status: 'decrypted'
    title: string
    inviterName: string | null
    inviterEmail: string | null
    role: InviteRole
    expiresAt: string | null
  }
  | { status: 'locked' }
  | {
    status: 'unavailable'
    reason: 'legacy' | 'missing_package' | 'awaiting_issuance' | 'verification_unavailable'
  }
  | { status: 'verification_failed' }

export type PendingInvitationPreviewEntry = {
  fingerprint: string
  preview: PendingInvitationPreview
}

export type PendingInvitationAcceptanceBlockReason =
  | 'verification_failed'
  | 'missing_package'
  | 'verification_unavailable'
  | 'legacy'

export type InviteAuthenticationLevel = 'authenticated' | 'legacy'

export type InviteAcceptancePayload = {
  workListKeyCiphertext: string
  membershipProof: string
  reservationRevision: number
}

export type InviteVerificationErrorCode =
  | 'awaiting_issuance'
  | 'missing_artifacts'
  | 'missing_package'
  | 'verification_unavailable'
  | 'authentication_required'
  | 'authentication_failed'
  | 'verification_failed'
  | 'verified_invite_consumed'
  | 'invalid_verified_invite'

export class InviteVerificationError extends Error {
  readonly code: InviteVerificationErrorCode

  constructor(code: InviteVerificationErrorCode, message: string, options?: ErrorOptions) {
    super(message, options)
    this.name = 'InviteVerificationError'
    this.code = code
  }
}

type RecoveredInvite = {
  membershipId: string
  workListId: string
  reservationRevision?: number
  listKey: Uint8Array
  memberEnvelopeKey: Uint8Array
  authentication: InviteAuthenticationLevel
  preview: Extract<PendingInvitationPreview, { status: 'decrypted' }> | null
}

/**
 * A one-use capability issued only after all recipient/member bindings and the
 * inviter authenticator have verified. Its constructor is private at the type
 * level and also guarded by a module-private runtime token.
 */
export class VerifiedPendingInvitation {
  readonly membershipId: string
  readonly workListId: string
  readonly authentication = 'authenticated' as const
  #listKey: Uint8Array | null
  #memberEnvelopeKey: Uint8Array | null
  #reservationRevision: number

  private constructor(
    authority: symbol,
    recovered: RecoveredInvite & { reservationRevision: number },
  ) {
    if (authority !== VERIFIED_INVITE_AUTHORITY) {
      throw new InviteVerificationError(
        'invalid_verified_invite',
        'Verified invitations can only be created by the trust verifier.',
      )
    }
    this.membershipId = recovered.membershipId
    this.workListId = recovered.workListId
    this.#reservationRevision = recovered.reservationRevision
    this.#listKey = recovered.listKey
    this.#memberEnvelopeKey = recovered.memberEnvelopeKey
  }

  static async verify(params: VerifyInviteForAcceptanceParams): Promise<VerifiedPendingInvitation> {
    const recovered = await recoverAndVerifyInvite(params)
    const reservationRevision = recovered.reservationRevision
    if (
      recovered.authentication !== 'authenticated'
      || reservationRevision === undefined
    ) {
      zeroBytes(recovered.listKey)
      zeroBytes(recovered.memberEnvelopeKey)
      throw new InviteVerificationError(
        'authentication_required',
        'Only version-two invitations with verified sender authentication can be accepted.',
      )
    }
    return new VerifiedPendingInvitation(VERIFIED_INVITE_AUTHORITY, {
      ...recovered,
      reservationRevision,
    })
  }

  dispose(): void {
    zeroBytes(this.#listKey)
    zeroBytes(this.#memberEnvelopeKey)
    this.#listKey = null
    this.#memberEnvelopeKey = null
  }

  async buildAcceptance(
    authority: symbol,
    params: {
      dataKey: Uint8Array
      strongBox: StrongBoxBridge
      projectKeyWriteFormat?: ProjectKeyWriteFormat
    },
  ): Promise<InviteAcceptancePayload> {
    if (authority !== VERIFIED_INVITE_AUTHORITY) {
      throw new InviteVerificationError(
        'invalid_verified_invite',
        'Invite acceptance requires a capability issued by the trust verifier.',
      )
    }
    const listKey = this.#listKey
    const memberEnvelopeKey = this.#memberEnvelopeKey
    if (!listKey || !memberEnvelopeKey) {
      throw new InviteVerificationError(
        'verified_invite_consumed',
        'The verified invitation has already been consumed or disposed.',
      )
    }
    // Take exclusive ownership before the first async suspension so concurrent
    // callers cannot both consume the same verified capability.
    this.#listKey = null
    this.#memberEnvelopeKey = null

    try {
      const membershipPayload = await sealWorkListKeyForOwner({
        listKey,
        dataKey: params.dataKey,
        strongBox: params.strongBox,
        projectKeyWriteFormat: params.projectKeyWriteFormat,
      })
      return {
        workListKeyCiphertext: membershipPayload.base64,
        membershipProof: await computeMembershipProof(
          memberEnvelopeKey,
          this.membershipId,
        ),
        reservationRevision: this.#reservationRevision,
      }
    } finally {
      zeroBytes(listKey)
      zeroBytes(memberEnvelopeKey)
    }
  }
}

export type VerifyInviteForAcceptanceParams = {
  invitation: PendingInvitationCrypto
  dataKey: Uint8Array
  userId: string
  strongBox: StrongBoxBridge
  resolveInviteKey?: InviteKeyResolver
}

export function verifyInviteForAcceptance(
  params: VerifyInviteForAcceptanceParams,
): Promise<VerifiedPendingInvitation> {
  return VerifiedPendingInvitation.verify(params)
}

export async function buildInviteAcceptancePayload(params: {
  verifiedInvite: VerifiedPendingInvitation
  dataKey: Uint8Array
  strongBox: StrongBoxBridge
  projectKeyWriteFormat?: ProjectKeyWriteFormat
}): Promise<InviteAcceptancePayload> {
  if (!(params.verifiedInvite instanceof VerifiedPendingInvitation)) {
    throw new InviteVerificationError(
      'invalid_verified_invite',
      'Invite acceptance requires a capability issued by the trust verifier.',
    )
  }
  if (params.verifiedInvite.authentication !== 'authenticated') {
    params.verifiedInvite.dispose()
    throw new InviteVerificationError(
      'authentication_required',
      'Invitations without verified sender authentication cannot be accepted.',
    )
  }
  return params.verifiedInvite.buildAcceptance(VERIFIED_INVITE_AUTHORITY, {
    dataKey: params.dataKey,
    strongBox: params.strongBox,
    projectKeyWriteFormat: params.projectKeyWriteFormat,
  })
}

export async function decryptPendingInvitationPreview(params: {
  invitation: PendingInvitationCrypto
  dataKey: Uint8Array | null
  userId: string | null | undefined
  strongBox: StrongBoxBridge
  resolveInviteKey?: InviteKeyResolver
}): Promise<PendingInvitationPreview> {
  if (!isInvitationReady(resolvePendingInvitationState(params.invitation))) {
    return { status: 'unavailable', reason: 'awaiting_issuance' }
  }
  if (!params.dataKey || !params.userId) {
    return { status: 'locked' }
  }

  let recovered: RecoveredInvite | null = null
  try {
    recovered = await recoverAndVerifyInvite({
      invitation: params.invitation,
      dataKey: params.dataKey,
      userId: params.userId,
      strongBox: params.strongBox,
      resolveInviteKey: params.resolveInviteKey,
    })
    if (recovered.authentication === 'legacy' || !recovered.preview) {
      return { status: 'unavailable', reason: 'legacy' }
    }
    return recovered.preview
  } catch (error) {
    if (error instanceof InviteVerificationError) {
      if (error.code === 'missing_package') {
        return { status: 'unavailable', reason: 'missing_package' }
      }
      if (error.code === 'verification_unavailable') {
        return { status: 'unavailable', reason: 'verification_unavailable' }
      }
      if (error.code === 'awaiting_issuance') {
        return { status: 'unavailable', reason: 'awaiting_issuance' }
      }
    }
    return { status: 'verification_failed' }
  } finally {
    zeroBytes(recovered?.listKey)
    zeroBytes(recovered?.memberEnvelopeKey)
  }
}

export function computePendingInvitationPreviewFingerprint(
  invitation: PendingInvitationCrypto,
): string {
  return JSON.stringify([
    invitation.membershipId,
    invitation.workListId,
    invitation.role,
    invitation.invitationState,
    invitation.expiresAt ?? null,
    invitation.inviteProtocolVersion ?? 1,
    invitation.reservationRevision ?? null,
    invitation.workListKeyCiphertext,
    invitation.invitePackageCiphertext ?? null,
    invitation.recipientCiphertext ?? null,
    invitation.saltMember ?? null,
  ])
}

export function isPendingInvitationPreviewVerificationPending(
  invitation: PendingInvitationCrypto,
  preview: PendingInvitationPreview | undefined,
): boolean {
  return !preview && Boolean(
    isInvitationReady(resolvePendingInvitationState(invitation))
    && invitation.recipientCiphertext
    && invitation.saltMember,
  )
}

export function getPendingInvitationAcceptanceBlockReason(
  preview: PendingInvitationPreview | undefined,
): PendingInvitationAcceptanceBlockReason | null {
  if (preview?.status === 'verification_failed') {
    return 'verification_failed'
  }
  if (
    preview?.status === 'unavailable'
    && (
      preview.reason === 'missing_package'
      || preview.reason === 'verification_unavailable'
      || preview.reason === 'legacy'
    )
  ) {
    return preview.reason
  }
  return null
}

export function canAcceptPendingInvitationAfterPreview(
  invitation: PendingInvitationCrypto,
  preview: PendingInvitationPreview | undefined,
): boolean {
  if (
    !isInvitationReady(resolvePendingInvitationState(invitation))
    || !invitation.recipientCiphertext
    || !invitation.saltMember
    || !preview
    || invitation.inviteProtocolVersion !== 2
    || !Number.isSafeInteger(invitation.reservationRevision)
    || invitation.reservationRevision! < 0
  ) {
    return false
  }
  return preview.status === 'decrypted' && Boolean(invitation.invitePackageCiphertext)
}

export function resolvePendingInvitationState(
  invitation: Pick<
    PendingInvitationCrypto,
    'invitationState' | 'recipientCiphertext' | 'saltMember'
  >,
): 'awaiting_issuance' | 'ready' {
  if (
    invitation.invitationState === 'ready'
    || invitation.invitationState === 'awaiting_issuance'
  ) {
    return invitation.invitationState
  }
  return invitation.recipientCiphertext && invitation.saltMember
    ? 'ready'
    : 'awaiting_issuance'
}

export function isInvitationReady(
  state: 'awaiting_issuance' | 'ready' | null | undefined,
): boolean {
  return state === 'ready'
}

export function normalizeMembershipRole(role: string): InviteRole {
  if (role === 'member' || role === 'admin') {
    return role
  }
  throw new InviteVerificationError(
    'verification_failed',
    `Unsupported membership role: ${role}`,
  )
}

export function encodeRecipientBindingContext(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  keyFingerprintB64: string
}): Uint8Array {
  return encodeCbor({
    kind: 'work_list.invite.binding',
    version: 1,
    body: {
      work_list_id: params.workListId,
      membership_id: params.membershipBindingId,
      role: params.role,
      key_fingerprint: params.keyFingerprintB64,
    },
  })
}

export function encodeInvitePackageBindingContext(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  keyFingerprintB64: string
  expiresAt: string | null
  inviteProtocolVersion?: InviteProtocolVersion
  reservationRevision?: number
}): Uint8Array {
  const protocol = validateInviteProtocolMetadata({
    inviteProtocolVersion: params.inviteProtocolVersion,
    reservationRevision: params.reservationRevision,
    expiresAt: params.expiresAt,
  })
  return encodeCbor({
    kind: 'work_list.invite.package.binding',
    version: protocol.inviteProtocolVersion,
    body: {
      work_list_id: params.workListId,
      membership_id: params.membershipBindingId,
      role: params.role,
      key_fingerprint: params.keyFingerprintB64,
      expires_at: params.expiresAt,
      ...(protocol.inviteProtocolVersion === 2
        ? { reservation_revision: protocol.reservationRevision }
        : {}),
    },
  })
}

export async function deriveMemberEnvelopeKey(params: {
  listKey: Uint8Array
  userId: string
  salt: Uint8Array
}): Promise<Uint8Array> {
  if (!params.userId) {
    throw new InviteVerificationError(
      'verification_failed',
      'A user identifier is required to derive membership keys.',
    )
  }
  return hkdfExpand({
    parent: params.listKey,
    info: `member:${params.userId}:${encodeBase64(params.salt)}`,
  })
}

export async function computeInvitePackageDigestB64(
  invitePackageCiphertext: string,
): Promise<string> {
  const bytes = decodeBase64(invitePackageCiphertext)
  return encodeBase64(await sha256(bytes, 'invite digest verification'))
}

async function recoverAndVerifyInvite(
  params: VerifyInviteForAcceptanceParams,
): Promise<RecoveredInvite> {
  const { invitation, dataKey, userId, strongBox } = params
  if (!isInvitationReady(resolvePendingInvitationState(invitation))) {
    throw new InviteVerificationError(
      'awaiting_issuance',
      'The invitation is still awaiting encrypted issuance.',
    )
  }
  if (!invitation.membershipId || !invitation.workListId) {
    throw new InviteVerificationError(
      'missing_artifacts',
      'The invitation is missing its membership or project identifier.',
    )
  }
  if (!invitation.recipientCiphertext || !invitation.saltMember) {
    throw new InviteVerificationError(
      'missing_artifacts',
      'The invitation is missing the encrypted recipient artifacts.',
    )
  }

  const role = normalizeMembershipRole(invitation.role)
  const expectedPackageDigest = invitation.invitePackageCiphertext
    ? await computeInvitePackageDigestB64(invitation.invitePackageCiphertext)
    : null
  const keyPair = await deriveInviteKeyPair({ dataKey, userId })
  let recipientPlaintext: Uint8Array | null = null
  let memberPlaintext: Uint8Array | null = null
  let packagePlaintext: Uint8Array | null = null
  let salt: Uint8Array | null = null
  let listKey: Uint8Array | null = null
  let memberEnvelopeKey: Uint8Array | null = null
  try {
    const keyFingerprint = await computeKeyFingerprint(keyPair.publicKey)
    const recipientBinding = encodeRecipientBindingContext({
      workListId: invitation.workListId,
      membershipBindingId: invitation.membershipId,
      role,
      keyFingerprintB64: encodeBase64(keyFingerprint),
    })
    const recipientSealed = parseSealedPayload(invitation.recipientCiphertext)
    recipientPlaintext = await hpkeOpen({
      recipientPrivateKey: keyPair.privateKey,
      info: recipientBinding,
      aad: recipientBinding,
      envelope: recipientSealed.ciphertext,
    })
    const recipientEnvelope = parseRecipientEnvelope(cborDecode(recipientPlaintext) as unknown)
    validateRecipientEnvelope({
      envelope: recipientEnvelope,
      invitation,
      role,
    })
    listKey = decodeRequiredKey(
      recipientEnvelope.body.key,
      'Recipient HPKE payload is missing the project key',
    )

    salt = decodeBase64(invitation.saltMember)
    memberEnvelopeKey = await deriveMemberEnvelopeKey({ listKey, userId, salt })
    const memberSealed = parseSealedPayload(invitation.workListKeyCiphertext)
    memberPlaintext = await strongBox.decrypt({
      key: memberEnvelopeKey,
      context: INVITE_MEMBER_CONTEXT,
      ciphertext: memberSealed.ciphertext,
    })
    const memberEnvelope = parseInviteMemberEnvelope(cborDecode(memberPlaintext) as unknown)
    validateMemberEnvelope({
      envelope: memberEnvelope,
      invitation,
      userId,
      memberEnvelopeKey,
    })

    const recipientDigest = recipientEnvelope.body.invite_package_digest
    const memberDigest = memberEnvelope.body.invite_package_digest
    const authentication = validatePackageDigestBinding({
      expectedPackageDigest,
      recipientDigest,
      memberDigest,
      hasPackage: Boolean(invitation.invitePackageCiphertext),
    })

    let preview: Extract<PendingInvitationPreview, { status: 'decrypted' }> | null = null
    let finalAuthentication = authentication
    if (
      invitation.invitePackageCiphertext
      && authentication === 'authenticated'
    ) {
      const protocol = resolveInvitationProtocol(invitation)
      const expectedExpiresAt = protocol.inviteProtocolVersion === 1
        ? canonicalRequiredExpiresAt(invitation.expiresAt ?? null)
        : canonicalOptionalExpiresAt(invitation.expiresAt ?? null)
      const packageBinding = encodeInvitePackageBindingContext({
        workListId: invitation.workListId,
        membershipBindingId: invitation.membershipId,
        role,
        keyFingerprintB64: encodeBase64(keyFingerprint),
        expiresAt: expectedExpiresAt,
        inviteProtocolVersion: protocol.inviteProtocolVersion,
        reservationRevision: protocol.reservationRevision,
      })
      const packageSealed = parseSealedPayload(invitation.invitePackageCiphertext)
      packagePlaintext = await hpkeOpen({
        recipientPrivateKey: keyPair.privateKey,
        info: packageBinding,
        aad: packageBinding,
        envelope: packageSealed.ciphertext,
      })
      const packageEnvelope = parseInvitePackageEnvelope(cborDecode(packagePlaintext) as unknown)
      validatePackageEnvelope({
        envelope: packageEnvelope,
        invitation,
        protocol,
        expectedExpiresAt,
        role,
      })

      const authStatus = await verifyPackageAuthenticator({
        envelope: packageEnvelope,
        packageVersion: protocol.inviteProtocolVersion,
        recipientPrivateKey: keyPair.privateKey,
        recipientPublicKey: keyPair.publicKey,
        role,
        resolveInviteKey: params.resolveInviteKey,
      })
      if (authStatus === 'unavailable') {
        throw new InviteVerificationError(
          'verification_unavailable',
          'The inviter key could not be resolved for authentication.',
        )
      }
      if (authStatus === 'failed') {
        throw new InviteVerificationError(
          'authentication_failed',
          'The invitation package authenticator did not verify.',
        )
      }
      finalAuthentication = authStatus
      if (authStatus === 'authenticated') {
        preview = {
          status: 'decrypted',
          title: packageEnvelope.body.title as string,
          inviterName: normalizeNonBlankString(packageEnvelope.body.inviter?.name),
          inviterEmail: normalizeNonBlankString(packageEnvelope.body.inviter?.email),
          role,
          expiresAt: invitation.expiresAt ?? null,
        }
      }
    }

    const recovered: RecoveredInvite = {
      membershipId: invitation.membershipId,
      workListId: invitation.workListId,
      reservationRevision: resolveInvitationProtocol(invitation).reservationRevision,
      listKey,
      memberEnvelopeKey,
      authentication: finalAuthentication,
      preview,
    }
    listKey = null
    memberEnvelopeKey = null
    return recovered
  } catch (error) {
    if (error instanceof InviteVerificationError) {
      throw error
    }
    throw new InviteVerificationError(
      'verification_failed',
      'The encrypted invitation artifacts could not be verified.',
      { cause: error },
    )
  } finally {
    zeroBytes(keyPair.privateKey)
    zeroBytes(keyPair.publicKey)
    zeroBytes(recipientPlaintext)
    zeroBytes(memberPlaintext)
    zeroBytes(packagePlaintext)
    zeroBytes(salt)
    zeroBytes(listKey)
    zeroBytes(memberEnvelopeKey)
  }
}

type RecipientEnvelope = {
  kind: 'work_list.invite.recipient'
  version: number | null
  body: {
    work_list_id: string | null
    membership_id: string | null
    role: string | null
    key: string | null
    invite_package_digest: string | null
  }
}

type InviteMemberEnvelope = {
  kind: 'work_list.invite.member'
  version: number | null
  body: {
    work_list_id: string | null
    membership_id: string | null
    key: string | null
    salt_member: string | null
    user_id: string | null
    invite_package_digest: string | null
  }
}

type PackageInviter = {
  id: string | null
  name: string | null
  email: string | null
}

type InvitePackageEnvelope = {
  kind: 'work_list.invite.package'
  version: number | null
  signedBody: InvitePreviewAuthSignedBody
  body: {
    work_list_id: string | null
    membership_id: string | null
    title: unknown
    invitee: { role: string | null } | null
    inviter: PackageInviter | null
    authenticator: InvitePreviewAuthenticator | null
    expires_at: string | null
    reservation_revision: number | null
  }
}

function parseRecipientEnvelope(value: unknown): RecipientEnvelope {
  const envelope = readRecord(value, 'Recipient HPKE payload')
  if (envelope.kind !== 'work_list.invite.recipient') {
    throw new Error('Recipient HPKE payload is malformed')
  }
  const body = readRecord(envelope.body, 'Recipient HPKE payload body')
  return {
    kind: 'work_list.invite.recipient',
    version: readOptionalNumber(envelope.version, 'Recipient HPKE payload version'),
    body: {
      work_list_id: readOptionalString(body.work_list_id, 'Recipient HPKE payload work_list_id'),
      membership_id: readOptionalString(body.membership_id, 'Recipient HPKE payload membership_id'),
      role: readOptionalString(body.role, 'Recipient HPKE payload role'),
      key: readOptionalString(body.key, 'Recipient HPKE payload key'),
      invite_package_digest: readOptionalString(
        body.invite_package_digest,
        'Recipient HPKE payload invite_package_digest',
      ),
    },
  }
}

function parseInviteMemberEnvelope(value: unknown): InviteMemberEnvelope {
  const envelope = readRecord(value, 'Invitation payload')
  if (envelope.kind !== 'work_list.invite.member') {
    throw new Error('Invitation payload is malformed')
  }
  const body = readRecord(envelope.body, 'Invitation payload body')
  return {
    kind: 'work_list.invite.member',
    version: readOptionalNumber(envelope.version, 'Invitation payload version'),
    body: {
      work_list_id: readOptionalString(body.work_list_id, 'Invitation payload work_list_id'),
      membership_id: readOptionalString(body.membership_id, 'Invitation payload membership_id'),
      key: readOptionalString(body.key, 'Invitation payload key'),
      salt_member: readOptionalString(body.salt_member, 'Invitation payload salt_member'),
      user_id: readOptionalString(body.user_id, 'Invitation payload user_id'),
      invite_package_digest: readOptionalString(
        body.invite_package_digest,
        'Invitation payload invite_package_digest',
      ),
    },
  }
}

function parseInvitePackageEnvelope(value: unknown): InvitePackageEnvelope {
  const envelope = readRecord(value, 'Invite package payload')
  if (envelope.kind !== 'work_list.invite.package') {
    throw new Error('Invite package payload is malformed')
  }
  const body = readRecord(envelope.body, 'Invite package payload body')
  return {
    kind: 'work_list.invite.package',
    version: readOptionalNumber(envelope.version, 'Invite package payload version'),
    signedBody: body,
    body: {
      work_list_id: readOptionalString(body.work_list_id, 'Invite package payload work_list_id'),
      membership_id: readOptionalString(body.membership_id, 'Invite package payload membership_id'),
      title: body.title,
      invitee: parseOptionalInvitee(body.invitee),
      inviter: parseOptionalInviter(body.inviter),
      authenticator: parseOptionalAuthenticator(body.authenticator),
      expires_at: readNullableString(body.expires_at, 'Invite package payload expires_at'),
      reservation_revision: readOptionalNumber(
        body.reservation_revision,
        'Invite package payload reservation_revision',
      ),
    },
  }
}

function validateRecipientEnvelope(params: {
  envelope: RecipientEnvelope
  invitation: PendingInvitationCrypto
  role: InviteRole
}): void {
  const { body } = params.envelope
  if (body.work_list_id && body.work_list_id !== params.invitation.workListId) {
    throw new Error('Recipient HPKE payload targets a different project')
  }
  if (body.membership_id && body.membership_id !== params.invitation.membershipId) {
    throw new Error('Recipient HPKE payload belongs to a different membership')
  }
  if (body.role && body.role !== params.role) {
    throw new Error('Recipient HPKE payload role mismatch')
  }
}

function validateMemberEnvelope(params: {
  envelope: InviteMemberEnvelope
  invitation: PendingInvitationCrypto
  userId: string
  memberEnvelopeKey: Uint8Array
}): void {
  const { body } = params.envelope
  if (body.user_id && body.user_id !== params.userId) {
    throw new Error('Invitation does not belong to the current user')
  }
  if (body.work_list_id && body.work_list_id !== params.invitation.workListId) {
    throw new Error('Invitation payload targets a different project')
  }
  if (body.membership_id && body.membership_id !== params.invitation.membershipId) {
    throw new Error('Invitation payload belongs to a different membership')
  }
  if (body.salt_member) {
    const actualSalt = decodeBase64(body.salt_member)
    const expectedSalt = decodeBase64(params.invitation.saltMember!)
    try {
      if (!constantTimeEquals(actualSalt, expectedSalt)) {
        throw new Error('Invitation payload membership salt mismatch')
      }
    } finally {
      zeroBytes(actualSalt)
      zeroBytes(expectedSalt)
    }
  }
  const memberKey = decodeRequiredKey(
    body.key,
    'Invitation payload is missing the member key',
  )
  try {
    if (!constantTimeEquals(memberKey, params.memberEnvelopeKey)) {
      throw new Error('Invitation member key does not match the derived envelope key')
    }
  } finally {
    zeroBytes(memberKey)
  }
}

function validatePackageDigestBinding(params: {
  expectedPackageDigest: string | null
  recipientDigest: string | null
  memberDigest: string | null
  hasPackage: boolean
}): InviteAuthenticationLevel {
  if (!params.hasPackage) {
    if (params.recipientDigest === null && params.memberDigest === null) {
      return 'legacy'
    }
    throw new InviteVerificationError(
      'missing_package',
      'Digest-bound invitation artifacts are missing their encrypted package.',
    )
  }
  if (params.recipientDigest === null && params.memberDigest === null) {
    return 'legacy'
  }
  if (
    !params.expectedPackageDigest
    || params.recipientDigest !== params.expectedPackageDigest
    || params.memberDigest !== params.expectedPackageDigest
  ) {
    throw new InviteVerificationError(
      'verification_failed',
      'Invitation package digests do not match all encrypted artifacts.',
    )
  }
  return 'authenticated'
}

function validatePackageEnvelope(params: {
  envelope: InvitePackageEnvelope
  invitation: PendingInvitationCrypto
  protocol: { inviteProtocolVersion: InviteProtocolVersion; reservationRevision?: number }
  expectedExpiresAt: string | null
  role: InviteRole
}): void {
  const { envelope, invitation, protocol } = params
  if (
    envelope.version !== protocol.inviteProtocolVersion
    || envelope.body.work_list_id !== invitation.workListId
    || envelope.body.membership_id !== invitation.membershipId
    || envelope.body.expires_at !== params.expectedExpiresAt
    || envelope.body.invitee?.role !== params.role
    || typeof envelope.body.title !== 'string'
    || envelope.body.title.trim().length === 0
  ) {
    throw new Error('Invite package payload does not match the invitation record')
  }
  if (
    protocol.inviteProtocolVersion === 2
      ? envelope.body.reservation_revision !== protocol.reservationRevision
      : envelope.body.reservation_revision !== null
  ) {
    throw new Error('Invite package reservation revision does not match')
  }
}

async function verifyPackageAuthenticator(params: {
  envelope: InvitePackageEnvelope
  packageVersion: InviteProtocolVersion
  recipientPrivateKey: Uint8Array
  recipientPublicKey: Uint8Array
  role: InviteRole
  resolveInviteKey?: InviteKeyResolver
}): Promise<'authenticated' | 'legacy' | 'failed' | 'unavailable'> {
  const { authenticator, inviter } = params.envelope.body
  if (!authenticator) {
    return 'legacy'
  }
  if (
    !inviter?.id
    || authenticator.body.inviter_user_id !== inviter.id
    || !Number.isSafeInteger(authenticator.body.inviter_key_generation)
    || authenticator.body.inviter_key_generation < 0
    || !hasValidAuthenticatorDecodedFields(authenticator)
  ) {
    return 'failed'
  }
  if (!await hasMatchingRecipientFingerprint(authenticator, params.recipientPublicKey)) {
    return 'failed'
  }
  const message = buildInvitePreviewAuthMessageFromPackageBody({
    signedBody: params.envelope.signedBody,
    role: params.role,
    packageVersion: params.packageVersion,
  })
  if (!message || !params.resolveInviteKey) {
    return params.resolveInviteKey ? 'failed' : 'unavailable'
  }

  let inviterKey: Awaited<ReturnType<InviteKeyResolver>>
  try {
    inviterKey = await params.resolveInviteKey({
      userId: authenticator.body.inviter_user_id,
      generation: authenticator.body.inviter_key_generation,
    })
  } catch {
    return 'unavailable'
  }
  if (
    inviterKey.generation !== authenticator.body.inviter_key_generation
    || inviterKey.authorization !== 'owner-authorized'
    || !(inviterKey.invitePublicKey instanceof Uint8Array)
    || inviterKey.invitePublicKey.length !== PUBLIC_KEY_BYTES
    || !(inviterKey.identityPublicKey instanceof Uint8Array)
    || inviterKey.identityPublicKey.length !== PUBLIC_KEY_BYTES
  ) {
    return 'failed'
  }
  const verified = await verifyInvitePreviewAuthenticator({
    message,
    authenticator,
    recipientPrivateKey: params.recipientPrivateKey,
    recipientPublicKey: params.recipientPublicKey,
    inviterPublicKey: inviterKey.invitePublicKey,
  })
  return verified ? 'authenticated' : 'failed'
}

function hasValidAuthenticatorDecodedFields(authenticator: InvitePreviewAuthenticator): boolean {
  return hasDecodedBase64Length(authenticator.body.inviter_key_fingerprint, AUTH_DIGEST_BYTES)
    && hasDecodedBase64Length(authenticator.body.recipient_key_fingerprint, AUTH_DIGEST_BYTES)
    && hasDecodedBase64Length(authenticator.body.mac, AUTH_DIGEST_BYTES)
}

async function hasMatchingRecipientFingerprint(
  authenticator: InvitePreviewAuthenticator,
  recipientPublicKey: Uint8Array,
): Promise<boolean> {
  let provided: Uint8Array | null = null
  let expected: Uint8Array | null = null
  try {
    provided = decodeBase64(authenticator.body.recipient_key_fingerprint)
    expected = await computeKeyFingerprint(recipientPublicKey)
    return provided.length === AUTH_DIGEST_BYTES
      && expected.length === AUTH_DIGEST_BYTES
      && constantTimeEquals(provided, expected)
  } catch {
    return false
  } finally {
    zeroBytes(provided)
    zeroBytes(expected)
  }
}

function hasDecodedBase64Length(value: string, expectedLength: number): boolean {
  let decoded: Uint8Array | null = null
  try {
    decoded = decodeBase64(value)
    return decoded.length === expectedLength
  } catch {
    return false
  } finally {
    zeroBytes(decoded)
  }
}

function parseOptionalInvitee(value: unknown): { role: string | null } | null {
  if (value === null || value === undefined) {
    return null
  }
  const invitee = readRecord(value, 'Invite package invitee')
  return { role: readOptionalString(invitee.role, 'Invite package invitee role') }
}

function parseOptionalInviter(value: unknown): PackageInviter | null {
  if (value === null || value === undefined) {
    return null
  }
  const inviter = readRecord(value, 'Invite package inviter')
  return {
    id: readOptionalString(inviter.id, 'Invite package inviter id'),
    name: readOptionalString(inviter.name, 'Invite package inviter name'),
    email: readOptionalString(inviter.email, 'Invite package inviter email'),
  }
}

function parseOptionalAuthenticator(value: unknown): InvitePreviewAuthenticator | null {
  if (value === null || value === undefined) {
    return null
  }
  const authenticator = readRecord(value, 'Invite package authenticator')
  const body = readRecord(authenticator.body, 'Invite package authenticator body')
  const kind = readRequiredString(authenticator.kind, 'Invite package authenticator kind')
  if (kind !== INVITE_PREVIEW_AUTH_KIND) {
    throw new Error('Invite package authenticator kind is unsupported')
  }
  const version = readRequiredNumber(authenticator.version, 'Invite package authenticator version')
  if (!isSupportedInvitePreviewAuthVersion(version)) {
    throw new Error('Invite package authenticator version is unsupported')
  }
  const scheme = readRequiredString(body.scheme, 'Invite package authenticator scheme')
  if (scheme !== INVITE_PREVIEW_AUTH_SCHEME) {
    throw new Error('Invite package authenticator scheme is unsupported')
  }
  return {
    kind,
    version,
    body: {
      scheme,
      inviter_user_id: readRequiredString(
        body.inviter_user_id,
        'Invite package authenticator inviter_user_id',
      ),
      inviter_key_generation: readRequiredNumber(
        body.inviter_key_generation,
        'Invite package authenticator inviter_key_generation',
      ),
      inviter_key_fingerprint: readRequiredString(
        body.inviter_key_fingerprint,
        'Invite package authenticator inviter_key_fingerprint',
      ),
      recipient_key_fingerprint: readRequiredString(
        body.recipient_key_fingerprint,
        'Invite package authenticator recipient_key_fingerprint',
      ),
      mac: readRequiredString(body.mac, 'Invite package authenticator mac'),
    },
  }
}

function resolveInvitationProtocol(invitation: PendingInvitationCrypto): {
  inviteProtocolVersion: InviteProtocolVersion
  reservationRevision?: number
} {
  return validateInviteProtocolMetadata({
    inviteProtocolVersion: invitation.inviteProtocolVersion,
    reservationRevision: invitation.reservationRevision,
    expiresAt: invitation.expiresAt ?? null,
  })
}

function validateInviteProtocolMetadata(params: {
  inviteProtocolVersion?: InviteProtocolVersion
  reservationRevision?: number
  expiresAt: string | null
}): { inviteProtocolVersion: InviteProtocolVersion; reservationRevision?: number } {
  const inviteProtocolVersion = params.inviteProtocolVersion ?? 1
  if (inviteProtocolVersion === 1) {
    if (params.expiresAt === null) {
      throw new Error('Legacy invitations require a finite deadline')
    }
    return { inviteProtocolVersion }
  }
  if (
    inviteProtocolVersion !== 2
    || !Number.isSafeInteger(params.reservationRevision)
    || params.reservationRevision! < 0
  ) {
    throw new Error('Invitation protocol metadata is unsupported or malformed')
  }
  return {
    inviteProtocolVersion,
    reservationRevision: params.reservationRevision,
  }
}

function canonicalOptionalExpiresAt(value: string | null): string | null {
  if (value === null) {
    return null
  }
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    throw new Error('Invitation access end is malformed')
  }
  return date.toISOString()
}

function canonicalRequiredExpiresAt(value: string | null): string {
  const canonical = canonicalOptionalExpiresAt(value)
  if (canonical === null) {
    throw new Error('Legacy invitation is missing its finite expiry')
  }
  return canonical
}

async function computeMembershipProof(
  memberKey: Uint8Array,
  membershipId: string,
): Promise<string> {
  const macBytes = await hmacSha256(
    memberKey,
    encoder.encode(`${MEMBERSHIP_PROOF_LABEL}${membershipId}`),
    'WebCrypto API is unavailable for membership proof generation',
  )
  try {
    return encodeBase64(await sha256(macBytes, 'membership proof generation'))
  } finally {
    zeroBytes(macBytes)
  }
}

async function sha256(bytes: Uint8Array, purpose: string): Promise<Uint8Array> {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error(`WebCrypto API is unavailable for ${purpose}`)
  }
  const digest = await subtle.digest('SHA-256', toArrayBuffer(bytes))
  return new Uint8Array(digest)
}

function decodeRequiredKey(value: string | null, message: string): Uint8Array {
  if (!value) {
    throw new Error(message)
  }
  const key = decodeBase64(value)
  if (key.length === 0) {
    throw new Error(`${message}: decoded key is empty`)
  }
  return key
}

function encodeCbor(value: unknown): Uint8Array {
  return toUint8Array(cborEncode(value))
}

function readRecord(value: unknown, label: string): Record<string, unknown> {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>
  }
  throw new Error(`${label} is malformed`)
}

function readOptionalString(value: unknown, label: string): string | null {
  if (value === null || value === undefined) {
    return null
  }
  if (typeof value === 'string') {
    return value
  }
  throw new Error(`${label} must be a string`)
}

function readNullableString(value: unknown, label: string): string | null {
  if (value === null || typeof value === 'string') {
    return value
  }
  throw new Error(`${label} must be a string or null`)
}

function readOptionalNumber(value: unknown, label: string): number | null {
  if (value === null || value === undefined) {
    return null
  }
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }
  throw new Error(`${label} must be a number`)
}

function readRequiredString(value: unknown, label: string): string {
  const result = readOptionalString(value, label)
  if (result === null || result.trim().length === 0) {
    throw new Error(`${label} is required`)
  }
  return result
}

function readRequiredNumber(value: unknown, label: string): number {
  const result = readOptionalNumber(value, label)
  if (result === null || !Number.isSafeInteger(result)) {
    throw new Error(`${label} must be a safe integer`)
  }
  return result
}

function normalizeNonBlankString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value : null
}
