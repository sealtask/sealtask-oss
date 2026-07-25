import { encode as cborEncode } from 'cbor-x'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { constantTimeEquals, toUint8Array, zeroBytes } from '../runtime/bytes'
import { hkdfExpand } from '../runtime/hkdf'
import { hmacSha256 } from '../runtime/hmac'
import { computeKeyFingerprint } from '../runtime/hpke'
import { deriveSharedSecret } from '../runtime/x25519'

export const INVITE_PREVIEW_AUTH_KIND = 'work_list.invite.preview_auth'
export const LEGACY_INVITE_PREVIEW_AUTH_VERSION = 1
export const INVITE_PREVIEW_AUTH_VERSION = 2
export const INVITE_PREVIEW_AUTH_SCHEME = 'x25519-hkdf-sha256-hmac-sha256'

const AUTH_MESSAGE_KIND = 'work_list.invite.preview_auth.message'
const AUTH_KEY_KIND = 'work_list.invite.preview_auth.key'

export type InviteRole = 'member' | 'admin'
export type InviteProtocolVersion = 1 | 2

export type InvitePreviewAuthenticatorVersion =
  | typeof LEGACY_INVITE_PREVIEW_AUTH_VERSION
  | typeof INVITE_PREVIEW_AUTH_VERSION

export type InvitePreviewAuthenticator = {
  kind: typeof INVITE_PREVIEW_AUTH_KIND
  version: InvitePreviewAuthenticatorVersion
  body: {
    scheme: typeof INVITE_PREVIEW_AUTH_SCHEME
    inviter_user_id: string
    inviter_key_generation: number
    inviter_key_fingerprint: string
    recipient_key_fingerprint: string
    mac: string
  }
}

export type InvitePreviewAuthSignedBody = Readonly<Record<string, unknown>>

type ValidatedInvitePreviewAuthSignedBody = InvitePreviewAuthSignedBody & {
  work_list_id: string
  membership_id: string
}

export type InvitePreviewAuthMessage = {
  packageVersion: InviteProtocolVersion
  packageBody: ValidatedInvitePreviewAuthSignedBody
  role: InviteRole
}

export type InvitePreviewAuthenticatorInput = {
  privateKey: Uint8Array
  publicKey: Uint8Array
  generation: number
}

export type InvitePreviewMacParams = {
  message: InvitePreviewAuthMessage
  privateKey: Uint8Array
  peerPublicKey: Uint8Array
  inviterUserId: string
  inviterKeyGeneration: number
  inviterKeyFingerprintB64: string
  recipientKeyFingerprintB64: string
}

type InvitePreviewAuthProtocol = {
  invite_protocol_version?: 2
  role: InviteRole
  inviter_user_id: string
  inviter_key_generation: number
  inviter_key_fingerprint: string
  recipient_key_fingerprint: string
}

export function zeroInvitePreviewAuthenticatorInput(
  auth?: InvitePreviewAuthenticatorInput | null,
): void {
  zeroBytes(auth?.privateKey)
  zeroBytes(auth?.publicKey)
}

export function buildInvitePreviewAuthMessageFromPackageBody(params: {
  signedBody: InvitePreviewAuthSignedBody
  role: InviteRole
  packageVersion?: InviteProtocolVersion
}): InvitePreviewAuthMessage | null {
  const { signedBody, role, packageVersion = 1 } = params
  const workListId = signedBody.work_list_id
  const membershipId = signedBody.membership_id
  if (
    typeof workListId !== 'string'
    || typeof membershipId !== 'string'
    || typeof signedBody.title !== 'string'
    || (signedBody.expires_at !== null && typeof signedBody.expires_at !== 'string')
    || (packageVersion !== 1 && packageVersion !== 2)
  ) {
    return null
  }
  const { authenticator: _authenticator, ...packageBody } = signedBody
  return {
    packageVersion,
    packageBody: {
      ...packageBody,
      work_list_id: workListId,
      membership_id: membershipId,
    },
    role,
  }
}

export function isSupportedInvitePreviewAuthVersion(
  version: number,
): version is InvitePreviewAuthenticatorVersion {
  return version === LEGACY_INVITE_PREVIEW_AUTH_VERSION
    || version === INVITE_PREVIEW_AUTH_VERSION
}

export async function createInvitePreviewAuthenticator(params: {
  message: InvitePreviewAuthMessage
  inviterAuth: InvitePreviewAuthenticatorInput
  recipientPublicKey: Uint8Array
}): Promise<InvitePreviewAuthenticator> {
  const inviterUserId = packageBodyInviterUserId(params.message)
  if (!inviterUserId) {
    throw new Error('Invite preview authentication requires an inviter user id.')
  }
  if (
    !Number.isSafeInteger(params.inviterAuth.generation)
    || params.inviterAuth.generation < 0
  ) {
    throw new Error('Invite preview authentication requires a valid inviter key generation.')
  }
  const [inviterKeyFingerprint, recipientKeyFingerprint] = await Promise.all([
    computeKeyFingerprint(params.inviterAuth.publicKey),
    computeKeyFingerprint(params.recipientPublicKey),
  ])
  const inviterKeyFingerprintB64 = encodeBase64(inviterKeyFingerprint)
  const recipientKeyFingerprintB64 = encodeBase64(recipientKeyFingerprint)
  const mac = await computeInvitePreviewMac({
    message: params.message,
    privateKey: params.inviterAuth.privateKey,
    peerPublicKey: params.recipientPublicKey,
    inviterUserId,
    inviterKeyGeneration: params.inviterAuth.generation,
    inviterKeyFingerprintB64,
    recipientKeyFingerprintB64,
  })

  return {
    kind: INVITE_PREVIEW_AUTH_KIND,
    version: INVITE_PREVIEW_AUTH_VERSION,
    body: {
      scheme: INVITE_PREVIEW_AUTH_SCHEME,
      inviter_user_id: inviterUserId,
      inviter_key_generation: params.inviterAuth.generation,
      inviter_key_fingerprint: inviterKeyFingerprintB64,
      recipient_key_fingerprint: recipientKeyFingerprintB64,
      mac: encodeBase64(mac),
    },
  }
}

export async function verifyInvitePreviewAuthenticator(params: {
  message: InvitePreviewAuthMessage
  authenticator: InvitePreviewAuthenticator
  recipientPrivateKey: Uint8Array
  recipientPublicKey: Uint8Array
  inviterPublicKey: Uint8Array
}): Promise<boolean> {
  try {
    const { authenticator, message } = params
    const { body } = authenticator
    if (
      authenticator.kind !== INVITE_PREVIEW_AUTH_KIND
      || !isSupportedInvitePreviewAuthVersion(authenticator.version)
      || body.scheme !== INVITE_PREVIEW_AUTH_SCHEME
    ) {
      return false
    }
    const inviterUserId = packageBodyInviterUserId(message)
    if (!inviterUserId || body.inviter_user_id !== inviterUserId) {
      return false
    }
    if (!Number.isSafeInteger(body.inviter_key_generation) || body.inviter_key_generation < 0) {
      return false
    }

    const [inviterKeyFingerprint, recipientKeyFingerprint] = await Promise.all([
      computeKeyFingerprint(params.inviterPublicKey),
      computeKeyFingerprint(params.recipientPublicKey),
    ])
    if (body.inviter_key_fingerprint !== encodeBase64(inviterKeyFingerprint)) {
      return false
    }
    if (body.recipient_key_fingerprint !== encodeBase64(recipientKeyFingerprint)) {
      return false
    }

    const providedMac = decodeBase64(body.mac)
    try {
      const expectedMac = await computeInvitePreviewMac(
        {
          message,
          privateKey: params.recipientPrivateKey,
          peerPublicKey: params.inviterPublicKey,
          inviterUserId: body.inviter_user_id,
          inviterKeyGeneration: body.inviter_key_generation,
          inviterKeyFingerprintB64: body.inviter_key_fingerprint,
          recipientKeyFingerprintB64: body.recipient_key_fingerprint,
        },
        authenticator.version,
      )
      try {
        return constantTimeEquals(providedMac, expectedMac)
      } finally {
        zeroBytes(expectedMac)
      }
    } finally {
      zeroBytes(providedMac)
    }
  } catch {
    return false
  }
}

export function encodeInvitePreviewMacMessage(params: InvitePreviewMacParams): Uint8Array {
  const protocolBody = buildInvitePreviewAuthProtocolBody(params)
  return toUint8Array(cborEncode({
    kind: AUTH_MESSAGE_KIND,
    version: INVITE_PREVIEW_AUTH_VERSION,
    body: {
      package: params.message.packageBody,
      protocol: protocolBody,
    },
  }))
}

async function computeInvitePreviewMac(
  params: InvitePreviewMacParams,
  authenticatorVersion: InvitePreviewAuthenticatorVersion = INVITE_PREVIEW_AUTH_VERSION,
): Promise<Uint8Array> {
  const sharedSecret = await deriveSharedSecret({
    privateKey: params.privateKey,
    peerPublicKey: params.peerPublicKey,
  })
  if (sharedSecret.every((byte) => byte === 0)) {
    zeroBytes(sharedSecret)
    throw new Error('Invite preview authentication shared secret is invalid.')
  }

  let macKey: Uint8Array | null = null
  try {
    macKey = await hkdfExpand({
      parent: sharedSecret,
      info: encodeMacKeyContext(params, authenticatorVersion),
    })
    return hmacSha256(
      macKey,
      authenticatorVersion === LEGACY_INVITE_PREVIEW_AUTH_VERSION
        ? encodeLegacyInvitePreviewMacMessage(params)
        : encodeInvitePreviewMacMessage(params),
      'WebCrypto API is unavailable for invite preview authentication.',
    )
  } finally {
    zeroBytes(sharedSecret)
    zeroBytes(macKey)
  }
}

function encodeMacKeyContext(
  params: InvitePreviewMacParams,
  authenticatorVersion: InvitePreviewAuthenticatorVersion,
): Uint8Array {
  return toUint8Array(cborEncode({
    kind: AUTH_KEY_KIND,
    version: authenticatorVersion,
    body: {
      scheme: INVITE_PREVIEW_AUTH_SCHEME,
      work_list_id: params.message.packageBody.work_list_id,
      membership_id: params.message.packageBody.membership_id,
      inviter_user_id: params.inviterUserId,
      inviter_key_generation: params.inviterKeyGeneration,
      inviter_key_fingerprint: params.inviterKeyFingerprintB64,
      recipient_key_fingerprint: params.recipientKeyFingerprintB64,
    },
  }))
}

function encodeLegacyInvitePreviewMacMessage(params: InvitePreviewMacParams): Uint8Array {
  return toUint8Array(cborEncode({
    kind: AUTH_MESSAGE_KIND,
    version: LEGACY_INVITE_PREVIEW_AUTH_VERSION,
    body: {
      ...params.message.packageBody,
      ...buildInvitePreviewAuthProtocolBody(params),
    },
  }))
}

function buildInvitePreviewAuthProtocolBody(
  params: InvitePreviewMacParams,
): InvitePreviewAuthProtocol {
  return {
    ...(params.message.packageVersion === 2 ? { invite_protocol_version: 2 as const } : {}),
    role: params.message.role,
    inviter_user_id: params.inviterUserId,
    inviter_key_generation: params.inviterKeyGeneration,
    inviter_key_fingerprint: params.inviterKeyFingerprintB64,
    recipient_key_fingerprint: params.recipientKeyFingerprintB64,
  }
}

function packageBodyInviterUserId(message: InvitePreviewAuthMessage): string | null {
  const inviter = message.packageBody.inviter
  if (typeof inviter !== 'object' || inviter === null) {
    return null
  }
  return normalizeNonBlankString(Reflect.get(inviter, 'id'))
}

function normalizeNonBlankString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value : null
}
