import { decode as cborDecode, encode as cborEncode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { toUint8Array, zeroBytes } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import {
  encodeHpkeEnvelope,
  hpkeSeal,
} from '../runtime/hpke'
import type { StrongBoxBridge } from '../runtime/strong-box'
import { deriveInviteKeyPair } from '../protocols/invite-key'
import {
  parseSealedPayload,
  serializeSealedPayloadBase64,
} from '../protocols/sealed-payload'
import {
  buildInvitePreviewAuthMessageFromPackageBody,
  createInvitePreviewAuthenticator,
} from './invite-auth'
import {
  VerifiedPendingInvitation,
  buildInviteAcceptancePayload,
  canAcceptPendingInvitationAfterPreview,
  computeInvitePackageDigestB64,
  decryptPendingInvitationPreview,
  deriveMemberEnvelopeKey,
  encodeInvitePackageBindingContext,
  encodeRecipientBindingContext,
  getPendingInvitationAcceptanceBlockReason,
  verifyInviteForAcceptance,
  type InviteKeyResolver,
  type PendingInvitationCrypto,
} from './invite-verification'

const USER_ID = 'invitee-user'
const INVITER_ID = 'inviter-user'
const WORK_LIST_ID = 'work-list-1'
const MEMBERSHIP_ID = 'membership-1'
const INVITER_GENERATION = 7
const DATA_KEY = new Uint8Array(32).fill(0x31)
const INVITER_DATA_KEY = new Uint8Array(32).fill(0x72)
const INVITER_IDENTITY_PUBLIC_KEY = new Uint8Array(32).fill(0x91)
const LIST_KEY = new Uint8Array(32).fill(0xa4)
const SALT = new Uint8Array(32).fill(0x5c)

const strongBox: StrongBoxBridge = {
  async encrypt({ plaintext }) {
    return plaintext.slice()
  },
  async decrypt({ ciphertext }) {
    return ciphertext.slice()
  },
}

describe('authenticated invitation verification and acceptance', () => {
  it('verifies an authenticated preview and mints a one-use acceptance capability', async () => {
    const fixture = await buildInvitationFixture()

    const preview = await decryptPendingInvitationPreview({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })
    expect(preview).toEqual({
      status: 'decrypted',
      title: 'Authenticated project',
      inviterName: 'Inviter',
      inviterEmail: 'inviter@example.com',
      role: 'member',
      expiresAt: null,
    })
    expect(canAcceptPendingInvitationAfterPreview(fixture.invitation, preview)).toBe(true)

    const verifiedInvite = await verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })
    expect(verifiedInvite.authentication).toBe('authenticated')
    expect(verifiedInvite.membershipId).toBe(MEMBERSHIP_ID)

    const acceptance = await buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: DATA_KEY,
      strongBox,
    })
    expect(decodeAcceptedProjectKeyPlaintext(acceptance.workListKeyCiphertext)).toEqual(LIST_KEY)
    expect(decodeBase64(acceptance.membershipProof)).toHaveLength(32)
    expect(acceptance.reservationRevision).toBe(4)
    await expect(buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: DATA_KEY,
      strongBox,
    })).rejects.toMatchObject({ code: 'verified_invite_consumed' })
  })

  it('writes a v2 project-key envelope only when the rollout gate explicitly opts in', async () => {
    const fixture = await buildInvitationFixture()
    const verifiedInvite = await verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })

    const acceptance = await buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: DATA_KEY,
      strongBox,
      projectKeyWriteFormat: 'v2-envelope',
    })

    expect(decodeAcceptedProjectKeyEnvelope(acceptance.workListKeyCiphertext)).toEqual({
      kind: 'sealtask-project-key',
      version: 2,
      key: LIST_KEY,
    })
  })

  it('atomically consumes the acceptance capability before asynchronous encryption', async () => {
    const fixture = await buildInvitationFixture()
    let blockEncryption = false
    let releaseEncryption!: () => void
    const encryptionGate = new Promise<void>((resolve) => {
      releaseEncryption = resolve
    })
    const deferredStrongBox: StrongBoxBridge = {
      async encrypt({ plaintext }) {
        if (blockEncryption) {
          await encryptionGate
        }
        return plaintext.slice()
      },
      async decrypt({ ciphertext }) {
        return ciphertext.slice()
      },
    }
    const verifiedInvite = await verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox: deferredStrongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })
    blockEncryption = true

    const first = buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: DATA_KEY,
      strongBox: deferredStrongBox,
    })
    await expect(buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: DATA_KEY,
      strongBox: deferredStrongBox,
    })).rejects.toMatchObject({ code: 'verified_invite_consumed' })

    releaseEncryption()
    await expect(first).resolves.toEqual({
      workListKeyCiphertext: expect.any(String),
      membershipProof: expect.any(String),
      reservationRevision: 4,
    })
  })

  it('rejects a forged object and a direct constructor call at the acceptance boundary', async () => {
    await expect(buildInviteAcceptancePayload({
      verifiedInvite: {} as VerifiedPendingInvitation,
      dataKey: DATA_KEY,
      strongBox,
    })).rejects.toMatchObject({ code: 'invalid_verified_invite' })

    expect(() => Reflect.construct(
      VerifiedPendingInvitation,
      [Symbol('forged'), {
        membershipId: MEMBERSHIP_ID,
        workListId: WORK_LIST_ID,
        listKey: LIST_KEY,
        memberEnvelopeKey: LIST_KEY,
        authentication: 'authenticated',
        preview: null,
      }],
    )).toThrow(/only be created by the trust verifier/i)
  })

  it('rejects a capability whose authentication marker is downgraded before acceptance', async () => {
    const fixture = await buildInvitationFixture()
    const verifiedInvite = await verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })

    try {
      expect(Reflect.set(verifiedInvite, 'authentication', 'legacy')).toBe(true)
      await expect(buildInviteAcceptancePayload({
        verifiedInvite,
        dataKey: DATA_KEY,
        strongBox,
      })).rejects.toMatchObject({ code: 'authentication_required' })
      expect(Reflect.set(verifiedInvite, 'authentication', 'authenticated')).toBe(true)
      await expect(buildInviteAcceptancePayload({
        verifiedInvite,
        dataKey: DATA_KEY,
        strongBox,
      })).rejects.toMatchObject({ code: 'verified_invite_consumed' })
    } finally {
      verifiedInvite.dispose()
    }
  })

  it('rejects an authenticator-tampered package even when every digest is self-consistent', async () => {
    const fixture = await buildInvitationFixture({ tamperAuthenticatorMac: true })

    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })).rejects.toMatchObject({
      code: 'authentication_failed',
    })
    await expect(decryptPendingInvitationPreview({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })).resolves.toEqual({ status: 'verification_failed' })
  })

  it('rejects a transparency-resolved inviter key that does not own the authenticator', async () => {
    const fixture = await buildInvitationFixture()
    const wrongPair = await deriveInviteKeyPair({
      dataKey: new Uint8Array(32).fill(0xee),
      userId: INVITER_ID,
    })
    const wrongResolver: InviteKeyResolver = async () => ({
      invitePublicKey: wrongPair.publicKey,
      generation: INVITER_GENERATION,
      identityPublicKey: INVITER_IDENTITY_PUBLIC_KEY,
      authorization: 'owner-authorized',
    })
    try {
      await expect(verifyInviteForAcceptance({
        invitation: fixture.invitation,
        dataKey: DATA_KEY,
        userId: USER_ID,
        strongBox,
        resolveInviteKey: wrongResolver,
      })).rejects.toMatchObject({ code: 'authentication_failed' })
    } finally {
      zeroBytes(wrongPair.privateKey)
      zeroBytes(wrongPair.publicKey)
    }
  })

  it('rejects a resolver result that is not owner-authorized', async () => {
    const fixture = await buildInvitationFixture()
    const unauthenticatedResolver = async () => {
      const resolved = await fixture.resolveInviteKey({
        userId: INVITER_ID,
        generation: INVITER_GENERATION,
      })
      return {
        ...resolved,
        authorization: 'legacy' as const,
      }
    }

    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: unauthenticatedResolver as unknown as InviteKeyResolver,
    })).rejects.toMatchObject({ code: 'authentication_failed' })
  })

  it('rejects malformed owner identity evidence from the resolver', async () => {
    const fixture = await buildInvitationFixture()
    const malformedResolver = async () => {
      const resolved = await fixture.resolveInviteKey({
        userId: INVITER_ID,
        generation: INVITER_GENERATION,
      })
      return {
        ...resolved,
        identityPublicKey: new Uint8Array(31),
      }
    }

    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: malformedResolver,
    })).rejects.toMatchObject({ code: 'authentication_failed' })
  })

  it('distinguishes unavailable inviter verification from failed authentication', async () => {
    const fixture = await buildInvitationFixture()
    const unavailableResolver: InviteKeyResolver = async () => {
      throw new Error('offline')
    }

    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: unavailableResolver,
    })).rejects.toMatchObject({ code: 'verification_unavailable' })
    await expect(decryptPendingInvitationPreview({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: unavailableResolver,
    })).resolves.toEqual({
      status: 'unavailable',
      reason: 'verification_unavailable',
    })
  })

  it('classifies authenticator-less packages as legacy and rejects normal acceptance', async () => {
    const fixture = await buildInvitationFixture({ includeAuthenticator: false })
    const preview = await decryptPendingInvitationPreview({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
    })
    expect(preview).toEqual({ status: 'unavailable', reason: 'legacy' })
    expect(canAcceptPendingInvitationAfterPreview(fixture.invitation, preview)).toBe(false)
    expect(getPendingInvitationAcceptanceBlockReason(preview)).toBe('legacy')
    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
    })).rejects.toMatchObject({ code: 'authentication_required' })
  })

  it('classifies genuinely digest-less package-less artifacts as legacy but rejects acceptance', async () => {
    const legacy = await buildInvitationFixture({ includePackage: false })
    const preview = await decryptPendingInvitationPreview({
      invitation: legacy.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
    })
    expect(preview).toEqual({ status: 'unavailable', reason: 'legacy' })
    expect(canAcceptPendingInvitationAfterPreview(legacy.invitation, preview)).toBe(false)
    expect(getPendingInvitationAcceptanceBlockReason(preview)).toBe('legacy')
    await expect(verifyInviteForAcceptance({
      invitation: legacy.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
    })).rejects.toMatchObject({ code: 'authentication_required' })

    const packageBound = await buildInvitationFixture()
    const missingPackage: PendingInvitationCrypto = {
      ...packageBound.invitation,
      invitePackageCiphertext: null,
    }
    await expect(verifyInviteForAcceptance({
      invitation: missingPackage,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: packageBound.resolveInviteKey,
    })).rejects.toMatchObject({ code: 'missing_package' })
  })

  it('rejects a member-envelope package digest omitted from an otherwise bound invite', async () => {
    const fixture = await buildInvitationFixture({ omitMemberDigest: true })

    await expect(verifyInviteForAcceptance({
      invitation: fixture.invitation,
      dataKey: DATA_KEY,
      userId: USER_ID,
      strongBox,
      resolveInviteKey: fixture.resolveInviteKey,
    })).rejects.toMatchObject({ code: 'verification_failed' })
  })
})

type FixtureOptions = {
  includeAuthenticator?: boolean
  includePackage?: boolean
  tamperAuthenticatorMac?: boolean
  omitMemberDigest?: boolean
}

async function buildInvitationFixture(options: FixtureOptions = {}): Promise<{
  invitation: PendingInvitationCrypto
  resolveInviteKey: InviteKeyResolver
}> {
  const includePackage = options.includePackage ?? true
  const recipientPair = await deriveInviteKeyPair({ dataKey: DATA_KEY, userId: USER_ID })
  const inviterPair = await deriveInviteKeyPair({
    dataKey: INVITER_DATA_KEY,
    userId: INVITER_ID,
  })
  const resolvedInviterPublicKey = inviterPair.publicKey.slice()
  try {
    const packageBody = {
      work_list_id: WORK_LIST_ID,
      membership_id: MEMBERSHIP_ID,
      title: 'Authenticated project',
      invitee: {
        user_id: USER_ID,
        email: 'invitee@example.com',
        name: 'Invitee',
        role: 'member' as const,
      },
      inviter: {
        id: INVITER_ID,
        name: 'Inviter',
        email: 'inviter@example.com',
      },
      issued_at: '2026-07-25T12:00:00.000Z',
      expires_at: null,
      reservation_revision: 4,
    }
    const authMessage = buildInvitePreviewAuthMessageFromPackageBody({
      signedBody: packageBody,
      role: 'member',
      packageVersion: 2,
    })
    if (!authMessage) {
      throw new Error('fixture auth message did not validate')
    }
    const authenticator = options.includeAuthenticator === false
      ? null
      : await createInvitePreviewAuthenticator({
        message: authMessage,
        inviterAuth: {
          privateKey: inviterPair.privateKey,
          publicKey: inviterPair.publicKey,
          generation: INVITER_GENERATION,
        },
        recipientPublicKey: recipientPair.publicKey,
      })
    if (authenticator && options.tamperAuthenticatorMac) {
      const mac = decodeBase64(authenticator.body.mac)
      mac[0] ^= 0x80
      authenticator.body.mac = encodeBase64(mac)
      zeroBytes(mac)
    }

    const packageCiphertext = includePackage
      ? await sealPackage({
        recipientPublicKey: recipientPair.publicKey,
        packageBody: {
          ...packageBody,
          authenticator,
        },
      })
      : null
    const packageDigest = packageCiphertext
      ? await computeInvitePackageDigestB64(packageCiphertext)
      : null
    const memberEnvelopeKey = await deriveMemberEnvelopeKey({
      listKey: LIST_KEY,
      userId: USER_ID,
      salt: SALT,
    })
    try {
      const memberEnvelope = {
        kind: 'work_list.invite.member',
        version: 1,
        body: {
          work_list_id: WORK_LIST_ID,
          membership_id: MEMBERSHIP_ID,
          user_id: USER_ID,
          key: encodeBase64(memberEnvelopeKey),
          salt_member: encodeBase64(SALT),
          ...(packageDigest && !options.omitMemberDigest
            ? { invite_package_digest: packageDigest }
            : {}),
          role: 'member',
          issued_at: '2026-07-25T12:00:00.000Z',
        },
      }
      const memberCiphertext = await strongBox.encrypt({
        key: memberEnvelopeKey,
        context: new TextEncoder().encode('worklist.invite.member'),
        plaintext: toUint8Array(cborEncode(memberEnvelope)),
      })
      const recipientCiphertext = await sealRecipient({
        recipientPublicKey: recipientPair.publicKey,
        packageDigest,
      })
      const invitation: PendingInvitationCrypto = {
        membershipId: MEMBERSHIP_ID,
        workListId: WORK_LIST_ID,
        role: 'member',
        invitationState: 'ready',
        expiresAt: null,
        inviteProtocolVersion: 2,
        reservationRevision: 4,
        workListKeyCiphertext: serializeSealedPayloadBase64({
          version: SEALED_PAYLOAD_VERSION,
          ciphertext: memberCiphertext,
        }),
        recipientCiphertext,
        invitePackageCiphertext: packageCiphertext,
        saltMember: encodeBase64(SALT),
      }
      return {
        invitation,
        resolveInviteKey: async ({ userId, generation }) => {
          if (userId !== INVITER_ID || generation !== INVITER_GENERATION) {
            throw new Error('unexpected inviter key lookup')
          }
          return {
            invitePublicKey: resolvedInviterPublicKey.slice(),
            generation: INVITER_GENERATION,
            identityPublicKey: INVITER_IDENTITY_PUBLIC_KEY.slice(),
            authorization: 'owner-authorized',
          }
        },
      }
    } finally {
      zeroBytes(memberEnvelopeKey)
    }
  } finally {
    zeroBytes(recipientPair.privateKey)
    zeroBytes(recipientPair.publicKey)
    zeroBytes(inviterPair.privateKey)
    zeroBytes(inviterPair.publicKey)
  }
}

async function sealPackage(params: {
  recipientPublicKey: Uint8Array
  packageBody: Record<string, unknown>
}): Promise<string> {
  const fingerprint = await crypto.subtle.digest(
    'SHA-256',
    params.recipientPublicKey.slice().buffer,
  )
  const binding = encodeInvitePackageBindingContext({
    workListId: WORK_LIST_ID,
    membershipBindingId: MEMBERSHIP_ID,
    role: 'member',
    keyFingerprintB64: encodeBase64(new Uint8Array(fingerprint)),
    expiresAt: null,
    inviteProtocolVersion: 2,
    reservationRevision: 4,
  })
  const sealed = await hpkeSeal({
    recipientPublicKey: params.recipientPublicKey,
    info: binding,
    aad: binding,
    plaintext: toUint8Array(cborEncode({
      kind: 'work_list.invite.package',
      version: 2,
      body: params.packageBody,
    })),
  })
  return serializeSealedPayloadBase64({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext: encodeHpkeEnvelope(sealed),
  })
}

async function sealRecipient(params: {
  recipientPublicKey: Uint8Array
  packageDigest: string | null
}): Promise<string> {
  const fingerprint = await crypto.subtle.digest(
    'SHA-256',
    params.recipientPublicKey.slice().buffer,
  )
  const binding = encodeRecipientBindingContext({
    workListId: WORK_LIST_ID,
    membershipBindingId: MEMBERSHIP_ID,
    role: 'member',
    keyFingerprintB64: encodeBase64(new Uint8Array(fingerprint)),
  })
  const sealed = await hpkeSeal({
    recipientPublicKey: params.recipientPublicKey,
    info: binding,
    aad: binding,
    plaintext: toUint8Array(cborEncode({
      kind: 'work_list.invite.recipient',
      version: 1,
      body: {
        work_list_id: WORK_LIST_ID,
        membership_id: MEMBERSHIP_ID,
        role: 'member',
        key: encodeBase64(LIST_KEY),
        ...(params.packageDigest ? { invite_package_digest: params.packageDigest } : {}),
        issued_at: '2026-07-25T12:00:00.000Z',
      },
    })),
  })
  return serializeSealedPayloadBase64({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext: encodeHpkeEnvelope(sealed),
  })
}

function decodeAcceptedProjectKeyEnvelope(ciphertext: string): unknown {
  const sealed = parseSealedPayload(ciphertext)
  return cborDecode(sealed.ciphertext)
}

function decodeAcceptedProjectKeyPlaintext(ciphertext: string): Uint8Array {
  return parseSealedPayload(ciphertext).ciphertext
}
