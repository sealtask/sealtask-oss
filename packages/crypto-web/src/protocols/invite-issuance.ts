import { encode as cborEncode } from 'cbor-x'

import { encodeBase64 } from '../runtime/base64'
import { toArrayBuffer, toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { hkdfExpand } from '../runtime/hkdf'
import {
  computeKeyFingerprint,
  encodeHpkeEnvelope,
  hpkeSeal,
  type HpkeSealParams,
  type HpkeSealResult,
} from '../runtime/hpke'
import { randomBytes } from '../runtime/random'
import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  buildInvitePreviewAuthMessageFromPackageBody,
  createInvitePreviewAuthenticator,
  type InvitePreviewAuthenticator,
  type InvitePreviewAuthenticatorInput,
  type InviteProtocolVersion,
  type InviteRole,
} from '../trust/invite-auth'
import { toSealedBlob } from './sealed-blob'
import type { SealedBlobPayload } from './types'

export const INVITE_MEMBER_CONTEXT = new TextEncoder().encode(
  'worklist.invite.member',
)

export type InviteIssuanceErrorCode =
  | 'access_end_invalid'
  | 'invite_preview_metadata_malformed'
  | 'legacy_invite_deadline_required'
  | 'membership_id_required'
  | 'reservation_revision_required'
  | 'unsupported_invite_protocol'
  | 'user_id_required'

export class InviteIssuanceError extends Error {
  readonly code: InviteIssuanceErrorCode

  constructor(code: InviteIssuanceErrorCode, message: string) {
    super(message)
    this.name = 'InviteIssuanceError'
    this.code = code
  }
}

export type InvitePackageInviter = {
  id: string
  name?: string | null
  email?: string | null
}

export type NormalizedInvitePackageInviter = {
  id: string
  name: string | null
  email: string | null
}

export type RegisteredInviteTarget = {
  id: string
  email: string
  name: string
  membershipRole: InviteRole
  expiresAt: Date | null
  kind: 'registered'
  userId: string
}

export type InvitePackageMetadata = {
  kind: 'work_list.invite.package'
  version: InviteProtocolVersion
  body: {
    work_list_id: string
    membership_id: string
    title: string
    invitee: {
      user_id: string
      email: string
      name: string
      role: InviteRole
    }
    inviter: NormalizedInvitePackageInviter | null
    authenticator: InvitePreviewAuthenticator | null
    issued_at: string
    expires_at: string | null
    reservation_revision?: number
  }
}

export type InviteMemberEnvelopePayload = {
  kind: 'work_list.invite.member'
  version: 1
  body: {
    work_list_id: string
    membership_id: string
    user_id: string
    email: string
    key: string
    salt_member: string
    invite_package_digest: string
    role: InviteRole
    issued_at: string
  }
}

/**
 * Structural request payload for the final invitation issuance API. The proof
 * is generic so applications can pass their generated transparency DTO
 * without making the public crypto package depend on generated API types.
 */
export type InviteMemberPayload<InviteKeyProof = unknown> = {
  email: string
  role: InviteRole
  membershipId: string
  workListKeyCiphertext: string
  recipientCiphertext: string
  invitePackageCiphertext: string
  saltMember: string
  expiresAt: string | null
  payloadBindingKey: string
  inviteKeyProof: InviteKeyProof
  inviteProtocolVersion: InviteProtocolVersion
  reservationRevision?: number
}

export type InviteIssuanceDependencies = {
  createAuthenticator?: typeof createInvitePreviewAuthenticator
  fingerprint?: typeof computeKeyFingerprint
  hpkeSeal?: (
    params: HpkeSealParams,
  ) => Promise<HpkeSealResult>
  now?: () => Date
  randomBytes?: (length: number) => Uint8Array
  error?: (
    code: InviteIssuanceErrorCode,
    defaultMessage: string,
  ) => Error
}

export type PreparedCreateInvitePayloadParams<
  InviteKeyProof = unknown,
> = {
  bridge: StrongBoxBridge
  workListId: string
  listKey: Uint8Array
  bindingKeyB64: string
  recipientPublicKey: Uint8Array
  target: RegisteredInviteTarget
  listTitle: string
  membershipId: string
  expiresAt: Date | null
  inviteKeyProof: InviteKeyProof
  inviteProtocolVersion?: InviteProtocolVersion
  reservationRevision?: number
  inviter: NormalizedInvitePackageInviter
  inviterAuth: InvitePreviewAuthenticatorInput
  dependencies?: InviteIssuanceDependencies
}

export function normalizeInvitePackageInviter(
  inviter: InvitePackageInviter,
  dependencies: Pick<InviteIssuanceDependencies, 'error'> = {},
): NormalizedInvitePackageInviter {
  const id = normalizeNonBlankString(inviter.id)
  if (id === null) {
    throw issuanceError(
      dependencies,
      'invite_preview_metadata_malformed',
      'Invite preview metadata is malformed.',
    )
  }
  return {
    id,
    name: normalizeNonBlankString(inviter.name),
    email: normalizeNonBlankString(inviter.email),
  }
}

export async function createInvitePayloadFromPreparedMetadata<
  InviteKeyProof,
>(
  params: PreparedCreateInvitePayloadParams<InviteKeyProof>,
): Promise<InviteMemberPayload<InviteKeyProof>> {
  const dependencies = params.dependencies ?? {}
  const salt = (dependencies.randomBytes ?? randomBytes)(32)
  if (salt.byteLength !== 32) {
    throw new Error('Invite member salt must contain exactly 32 bytes.')
  }
  const memberEnvelopeKey = await deriveMemberEnvelopeKey({
    listKey: params.listKey,
    userId: params.target.userId,
    salt,
    dependencies,
  })
  const issuedAt = (dependencies.now ?? (() => new Date()))().toISOString()
  const membershipBindingId = params.membershipId
  if (!membershipBindingId) {
    throw issuanceError(
      dependencies,
      'membership_id_required',
      'A membership id is required for invitation HPKE binding.',
    )
  }
  const expiresAt = params.expiresAt?.toISOString() ?? null
  const { inviteProtocolVersion, reservationRevision } =
    validateInviteProtocolMetadata(
      {
        inviteProtocolVersion: params.inviteProtocolVersion,
        reservationRevision: params.reservationRevision,
        expiresAt,
      },
      dependencies,
    )
  const artifactReservationRevision =
    reservationRevision === undefined
      ? undefined
      : reservationRevision + 1
  const invitePackageBody = {
    work_list_id: params.workListId,
    membership_id: membershipBindingId,
    title: params.listTitle,
    invitee: {
      user_id: params.target.userId,
      email: params.target.email,
      name: params.target.name || params.target.email,
      role: params.target.membershipRole,
    },
    inviter: params.inviter,
    issued_at: issuedAt,
    expires_at: expiresAt,
    ...(inviteProtocolVersion === 2
      ? { reservation_revision: artifactReservationRevision }
      : {}),
  } satisfies Omit<InvitePackageMetadata['body'], 'authenticator'>

  const message = buildInvitePreviewAuthMessageFromPackageBody({
    signedBody: invitePackageBody,
    role: params.target.membershipRole,
    packageVersion: inviteProtocolVersion,
  })
  if (!message) {
    throw issuanceError(
      dependencies,
      'invite_preview_metadata_malformed',
      'Invite preview metadata is malformed.',
    )
  }
  const signedInvitePackageBody = message.packageBody as Omit<
    InvitePackageMetadata['body'],
    'authenticator'
  >
  const authenticator = await (
    dependencies.createAuthenticator ??
    createInvitePreviewAuthenticator
  )({
    message,
    inviterAuth: params.inviterAuth,
    recipientPublicKey: params.recipientPublicKey,
  })
  const invitePackageMetadata: InvitePackageMetadata = {
    kind: 'work_list.invite.package',
    version: inviteProtocolVersion,
    body: {
      ...signedInvitePackageBody,
      authenticator,
    },
  }
  const invitePackageCiphertext = await sealInvitePackage({
    workListId: params.workListId,
    membershipBindingId,
    role: params.target.membershipRole,
    recipientPublicKey: params.recipientPublicKey,
    metadata: invitePackageMetadata,
    dependencies,
  })
  const invitePackageDigestB64 = encodeBase64(
    await sha256Bytes(invitePackageCiphertext.bytes),
  )
  const saltMember = encodeBase64(salt)
  const memberEnvelopePayload: InviteMemberEnvelopePayload = {
    kind: 'work_list.invite.member',
    version: 1,
    body: {
      work_list_id: params.workListId,
      membership_id: membershipBindingId,
      user_id: params.target.userId,
      email: params.target.email,
      key: encodeBase64(memberEnvelopeKey),
      salt_member: saltMember,
      invite_package_digest: invitePackageDigestB64,
      role: params.target.membershipRole,
      issued_at: issuedAt,
    },
  }
  const workListKeyCiphertext = await sealInviteMemberEnvelope({
    bridge: params.bridge,
    key: memberEnvelopeKey,
    payload: memberEnvelopePayload,
  })
  const recipientCiphertext = await createRecipientCiphertext({
    workListId: params.workListId,
    membershipBindingId,
    role: params.target.membershipRole,
    listKey: params.listKey,
    recipientPublicKey: params.recipientPublicKey,
    issuedAt,
    invitePackageDigestB64,
    dependencies,
  })

  return {
    email: params.target.email,
    role: params.target.membershipRole,
    membershipId: membershipBindingId,
    workListKeyCiphertext: workListKeyCiphertext.base64,
    recipientCiphertext: recipientCiphertext.base64,
    invitePackageCiphertext: invitePackageCiphertext.base64,
    saltMember,
    expiresAt,
    payloadBindingKey: params.bindingKeyB64,
    inviteKeyProof: params.inviteKeyProof,
    inviteProtocolVersion,
    ...(reservationRevision === undefined
      ? {}
      : { reservationRevision }),
  }
}

export function encodeRecipientBindingContext(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  keyFingerprintB64: string
}): Uint8Array {
  return toUint8Array(
    cborEncode({
      kind: 'work_list.invite.binding',
      version: 1,
      body: {
        work_list_id: params.workListId,
        membership_id: params.membershipBindingId,
        role: params.role,
        key_fingerprint: params.keyFingerprintB64,
      },
    }),
  )
}

export function encodeInvitePackageBindingContext(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  keyFingerprintB64: string
  expiresAt: string | null
  inviteProtocolVersion?: InviteProtocolVersion
  reservationRevision?: number
  dependencies?: Pick<InviteIssuanceDependencies, 'error'>
}): Uint8Array {
  const { inviteProtocolVersion, reservationRevision } =
    validateInviteProtocolMetadata(
      {
        inviteProtocolVersion: params.inviteProtocolVersion,
        reservationRevision: params.reservationRevision,
        expiresAt: params.expiresAt,
      },
      params.dependencies,
    )
  return toUint8Array(
    cborEncode({
      kind: 'work_list.invite.package.binding',
      version: inviteProtocolVersion,
      body: {
        work_list_id: params.workListId,
        membership_id: params.membershipBindingId,
        role: params.role,
        key_fingerprint: params.keyFingerprintB64,
        expires_at: params.expiresAt,
        ...(inviteProtocolVersion === 2
          ? { reservation_revision: reservationRevision }
          : {}),
      },
    }),
  )
}

export function encodeRecipientPlaintext(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  listKey: Uint8Array
  issuedAt: string
  invitePackageDigestB64: string
}): Uint8Array {
  return toUint8Array(
    cborEncode({
      kind: 'work_list.invite.recipient',
      version: 1,
      body: {
        work_list_id: params.workListId,
        membership_id: params.membershipBindingId,
        role: params.role,
        key: encodeBase64(params.listKey),
        invite_package_digest: params.invitePackageDigestB64,
        issued_at: params.issuedAt,
      },
    }),
  )
}

export async function deriveMemberEnvelopeKey(params: {
  listKey: Uint8Array
  userId: string
  salt: Uint8Array
  dependencies?: Pick<InviteIssuanceDependencies, 'error'>
}): Promise<Uint8Array> {
  if (!params.userId) {
    throw issuanceError(
      params.dependencies,
      'user_id_required',
      'A user identifier is required to derive membership keys.',
    )
  }
  return hkdfExpand({
    parent: params.listKey,
    info: `member:${params.userId}:${encodeBase64(params.salt)}`,
  })
}

export async function sealInviteMemberEnvelope(params: {
  bridge: StrongBoxBridge
  key: Uint8Array
  payload: InviteMemberEnvelopePayload
}): Promise<SealedBlobPayload> {
  const ciphertext = await params.bridge.encrypt({
    key: params.key,
    context: INVITE_MEMBER_CONTEXT,
    plaintext: toUint8Array(cborEncode(params.payload)),
  })
  return toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
}

export async function createRecipientCiphertext(
  params: {
    workListId: string
    membershipBindingId: string
    role: InviteRole
    listKey: Uint8Array
    recipientPublicKey: Uint8Array
    issuedAt: string
    invitePackageDigestB64: string
    dependencies?: InviteIssuanceDependencies
  },
): Promise<SealedBlobPayload> {
  const fingerprint = await (
    params.dependencies?.fingerprint ?? computeKeyFingerprint
  )(params.recipientPublicKey)
  const bindingContext = encodeRecipientBindingContext({
    workListId: params.workListId,
    membershipBindingId: params.membershipBindingId,
    role: params.role,
    keyFingerprintB64: encodeBase64(fingerprint),
  })
  const plaintext = encodeRecipientPlaintext(params)
  const hpkeResult = await (
    params.dependencies?.hpkeSeal ?? hpkeSeal
  )({
    recipientPublicKey: params.recipientPublicKey,
    info: bindingContext,
    aad: bindingContext,
    plaintext,
  })
  return toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext: encodeHpkeEnvelope(hpkeResult),
  })
}

export async function sealInvitePackage(params: {
  workListId: string
  membershipBindingId: string
  role: InviteRole
  recipientPublicKey: Uint8Array
  metadata: InvitePackageMetadata
  dependencies?: InviteIssuanceDependencies
}): Promise<SealedBlobPayload> {
  const fingerprint = await (
    params.dependencies?.fingerprint ?? computeKeyFingerprint
  )(params.recipientPublicKey)
  const bindingContext = encodeInvitePackageBindingContext({
    workListId: params.workListId,
    membershipBindingId: params.membershipBindingId,
    role: params.role,
    keyFingerprintB64: encodeBase64(fingerprint),
    expiresAt: params.metadata.body.expires_at,
    inviteProtocolVersion: params.metadata.version,
    reservationRevision: params.metadata.body.reservation_revision,
    dependencies: params.dependencies,
  })
  const hpkeResult = await (
    params.dependencies?.hpkeSeal ?? hpkeSeal
  )({
    recipientPublicKey: params.recipientPublicKey,
    info: bindingContext,
    aad: bindingContext,
    plaintext: toUint8Array(cborEncode(params.metadata)),
  })
  return toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext: encodeHpkeEnvelope(hpkeResult),
  })
}

export function validateInviteProtocolMetadata(
  params: {
    inviteProtocolVersion?: InviteProtocolVersion
    reservationRevision?: number
    expiresAt: string | null
    allowV2NullExpiry?: boolean
  },
  dependencies: Pick<InviteIssuanceDependencies, 'error'> = {},
): {
  inviteProtocolVersion: InviteProtocolVersion
  reservationRevision?: number
} {
  const inviteProtocolVersion = params.inviteProtocolVersion ?? 1
  if (inviteProtocolVersion !== 1 && inviteProtocolVersion !== 2) {
    throw issuanceError(
      dependencies,
      'unsupported_invite_protocol',
      'The invitation protocol version is unsupported.',
    )
  }
  if (inviteProtocolVersion === 1) {
    if (params.expiresAt === null) {
      throw issuanceError(
        dependencies,
        'legacy_invite_deadline_required',
        'Legacy invites require a finite invitation deadline.',
      )
    }
    return { inviteProtocolVersion }
  }
  if (
    !Number.isSafeInteger(params.reservationRevision) ||
    params.reservationRevision! < 0 ||
    params.reservationRevision! >= Number.MAX_SAFE_INTEGER
  ) {
    throw issuanceError(
      dependencies,
      'reservation_revision_required',
      'Invitation protocol v2 requires a valid reservation revision.',
    )
  }
  if (!params.allowV2NullExpiry && params.expiresAt !== null) {
    const expiry = new Date(params.expiresAt)
    if (Number.isNaN(expiry.getTime())) {
      throw issuanceError(
        dependencies,
        'access_end_invalid',
        'The invitation access end is invalid.',
      )
    }
  }
  return {
    inviteProtocolVersion,
    reservationRevision: params.reservationRevision,
  }
}

async function sha256Bytes(bytes: Uint8Array): Promise<Uint8Array> {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error(
      'WebCrypto API is unavailable for invite digest generation.',
    )
  }
  return new Uint8Array(
    await subtle.digest('SHA-256', toArrayBuffer(bytes)),
  )
}

function normalizeNonBlankString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0
    ? value
    : null
}

function issuanceError(
  dependencies:
    | Pick<InviteIssuanceDependencies, 'error'>
    | undefined,
  code: InviteIssuanceErrorCode,
  defaultMessage: string,
): Error {
  return (
    dependencies?.error?.(code, defaultMessage) ??
    new InviteIssuanceError(code, defaultMessage)
  )
}
