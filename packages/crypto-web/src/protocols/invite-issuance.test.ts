import { decode as cborDecode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { toArrayBuffer } from '../runtime/bytes'
import {
  computeKeyFingerprint,
  hpkeOpen,
  type HpkeSealParams,
} from '../runtime/hpke'
import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  buildInvitePreviewAuthMessageFromPackageBody,
  verifyInvitePreviewAuthenticator,
  type InvitePreviewAuthenticator,
} from '../trust/invite-auth'
import {
  createInvitePayloadFromPreparedMetadata,
  deriveMemberEnvelopeKey,
  encodeInvitePackageBindingContext,
  encodeRecipientBindingContext,
  INVITE_MEMBER_CONTEXT,
  normalizeInvitePackageInviter,
} from './invite-issuance'
import { deriveInviteKeyPair } from './invite-key'
import { parseSealedPayload } from './sealed-payload'

const decoder = new TextDecoder()
const FIXED_NOW = new Date('2026-07-25T12:00:00.000Z')

const identityBridge: StrongBoxBridge = {
  async encrypt({ plaintext }) {
    return plaintext.slice()
  },
  async decrypt({ ciphertext }) {
    return ciphertext.slice()
  },
}

describe('invite issuance protocol', () => {
  it('builds recipient-decryptable, mutually digest-bound v2 artifacts', async () => {
    const [inviterKeys, recipientKeys] = await Promise.all([
      deriveInviteKeyPair({
        dataKey: new Uint8Array(32).fill(3),
        userId: 'inviter-user',
      }),
      deriveInviteKeyPair({
        dataKey: new Uint8Array(32).fill(9),
        userId: 'invitee-user',
      }),
    ])
    const listKey = new Uint8Array(32).fill(5)
    const salt = new Uint8Array(32).fill(11)
    const proof = { statement: { userId: 'invitee-user' } }
    const payload = await createInvitePayloadFromPreparedMetadata({
      bridge: identityBridge,
      workListId: 'list-123',
      listKey,
      bindingKeyB64: encodeBase64(new Uint8Array(32).fill(7)),
      recipientPublicKey: recipientKeys.publicKey,
      target: {
        kind: 'registered',
        id: 'invitee',
        userId: 'invitee-user',
        email: 'invitee@example.com',
        name: 'Invitee',
        membershipRole: 'member',
        expiresAt: null,
      },
      listTitle: 'Launch Readiness',
      membershipId: 'membership-123',
      inviter: normalizeInvitePackageInviter({
        id: 'inviter-user',
        name: 'Owner',
        email: 'owner@example.com',
      }),
      inviterAuth: {
        ...inviterKeys,
        generation: 4,
      },
      expiresAt: null,
      inviteKeyProof: proof,
      inviteProtocolVersion: 2,
      reservationRevision: 7,
      dependencies: {
        now: () => FIXED_NOW,
        randomBytes: () => salt.slice(),
      },
    })

    expect(payload).toMatchObject({
      email: 'invitee@example.com',
      role: 'member',
      membershipId: 'membership-123',
      expiresAt: null,
      inviteProtocolVersion: 2,
      reservationRevision: 7,
      inviteKeyProof: proof,
    })

    const fingerprintB64 = encodeBase64(
      await computeKeyFingerprint(recipientKeys.publicKey),
    )
    const packageContext = encodeInvitePackageBindingContext({
      workListId: 'list-123',
      membershipBindingId: 'membership-123',
      role: 'member',
      keyFingerprintB64: fingerprintB64,
      expiresAt: null,
      inviteProtocolVersion: 2,
      reservationRevision: 8,
    })
    const packageSealed = parseSealedPayload(
      payload.invitePackageCiphertext,
    )
    const packagePlaintext = await hpkeOpen({
      recipientPrivateKey: recipientKeys.privateKey,
      info: packageContext,
      aad: packageContext,
      envelope: packageSealed.ciphertext,
    })
    const invitePackage = cborDecode(packagePlaintext) as {
      kind: string
      version: number
      body: {
        membership_id: string
        reservation_revision: number
        authenticator: InvitePreviewAuthenticator
        [key: string]: unknown
      }
    }
    expect(invitePackage).toMatchObject({
      kind: 'work_list.invite.package',
      version: 2,
      body: {
        membership_id: 'membership-123',
        reservation_revision: 8,
        expires_at: null,
      },
    })
    const authMessage = buildInvitePreviewAuthMessageFromPackageBody({
      signedBody: invitePackage.body,
      role: 'member',
      packageVersion: 2,
    })
    expect(authMessage).not.toBeNull()
    await expect(
      verifyInvitePreviewAuthenticator({
        message: authMessage!,
        authenticator: invitePackage.body.authenticator,
        recipientPrivateKey: recipientKeys.privateKey,
        recipientPublicKey: recipientKeys.publicKey,
        inviterPublicKey: inviterKeys.publicKey,
      }),
    ).resolves.toBe(true)

    const expectedDigest = encodeBase64(
      new Uint8Array(
        await crypto.subtle.digest(
          'SHA-256',
          toArrayBuffer(decodeBase64(payload.invitePackageCiphertext)),
        ),
      ),
    )
    const recipientContext = encodeRecipientBindingContext({
      workListId: 'list-123',
      membershipBindingId: 'membership-123',
      role: 'member',
      keyFingerprintB64: fingerprintB64,
    })
    const recipientPlaintext = await hpkeOpen({
      recipientPrivateKey: recipientKeys.privateKey,
      info: recipientContext,
      aad: recipientContext,
      envelope: parseSealedPayload(payload.recipientCiphertext)
        .ciphertext,
    })
    expect(cborDecode(recipientPlaintext)).toMatchObject({
      kind: 'work_list.invite.recipient',
      version: 1,
      body: {
        key: encodeBase64(listKey),
        invite_package_digest: expectedDigest,
      },
    })

    const memberEnvelopeKey = await deriveMemberEnvelopeKey({
      listKey,
      userId: 'invitee-user',
      salt,
    })
    const memberPayload = cborDecode(
      parseSealedPayload(payload.workListKeyCiphertext).ciphertext,
    )
    expect(memberPayload).toMatchObject({
      kind: 'work_list.invite.member',
      version: 1,
      body: {
        key: encodeBase64(memberEnvelopeKey),
        salt_member: encodeBase64(salt),
        invite_package_digest: expectedDigest,
      },
    })
  })

  it('uses frozen package/member contexts with injected crypto dependencies', async () => {
    const hpkeCalls: HpkeSealParams[] = []
    const memberContexts: string[] = []
    const bridge: StrongBoxBridge = {
      async encrypt({ context, plaintext }) {
        memberContexts.push(decoder.decode(context))
        return plaintext.slice()
      },
      async decrypt({ ciphertext }) {
        return ciphertext.slice()
      },
    }
    await createInvitePayloadFromPreparedMetadata({
      bridge,
      workListId: 'list',
      listKey: new Uint8Array(32).fill(1),
      bindingKeyB64: 'binding',
      recipientPublicKey: new Uint8Array(32).fill(2),
      target: {
        kind: 'registered',
        id: 'target',
        userId: 'user',
        email: 'user@example.com',
        name: '',
        membershipRole: 'admin',
        expiresAt: null,
      },
      listTitle: 'Project',
      membershipId: 'membership',
      expiresAt: null,
      inviteKeyProof: {},
      inviteProtocolVersion: 2,
      reservationRevision: 0,
      inviter: { id: 'owner', name: null, email: null },
      inviterAuth: {
        privateKey: new Uint8Array(32).fill(3),
        publicKey: new Uint8Array(32).fill(4),
        generation: 0,
      },
      dependencies: {
        now: () => FIXED_NOW,
        randomBytes: () => new Uint8Array(32).fill(5),
        fingerprint: async () => new Uint8Array(32).fill(6),
        createAuthenticator: async () => ({
          kind: 'work_list.invite.preview_auth',
          version: 2,
          body: {
            scheme: 'x25519-hkdf-sha256-hmac-sha256',
            inviter_user_id: 'owner',
            inviter_key_generation: 0,
            inviter_key_fingerprint: 'inviter',
            recipient_key_fingerprint: 'recipient',
            mac: 'mac',
          },
        }),
        hpkeSeal: async (params) => {
          hpkeCalls.push(params)
          return {
            enc: new Uint8Array(32).fill(7),
            ciphertext: new Uint8Array([8, 9]),
          }
        },
      },
    })

    expect(hpkeCalls).toHaveLength(2)
    expect(hpkeCalls[0].info).toEqual(hpkeCalls[0].aad)
    expect(hpkeCalls[1].info).toEqual(hpkeCalls[1].aad)
    expect(cborDecode(hpkeCalls[0].info)).toMatchObject({
      kind: 'work_list.invite.package.binding',
      version: 2,
      body: { reservation_revision: 1 },
    })
    expect(cborDecode(hpkeCalls[1].info)).toMatchObject({
      kind: 'work_list.invite.binding',
      version: 1,
    })
    expect(memberContexts).toEqual([
      decoder.decode(INVITE_MEMBER_CONTEXT),
    ])
  })

  it('normalizes inviter display fields and enforces v1/v2 metadata', async () => {
    expect(
      normalizeInvitePackageInviter({
        id: 'owner',
        name: '   ',
        email: 'owner@example.com',
      }),
    ).toEqual({
      id: 'owner',
      name: null,
      email: 'owner@example.com',
    })
    expect(() =>
      encodeInvitePackageBindingContext({
        workListId: 'list',
        membershipBindingId: 'membership',
        role: 'member',
        keyFingerprintB64: 'fingerprint',
        expiresAt: null,
      }),
    ).toThrow('finite invitation deadline')
    expect(() =>
      encodeInvitePackageBindingContext({
        workListId: 'list',
        membershipBindingId: 'membership',
        role: 'member',
        keyFingerprintB64: 'fingerprint',
        expiresAt: null,
        inviteProtocolVersion: 2,
        reservationRevision: Number.MAX_SAFE_INTEGER,
      }),
    ).toThrow('reservation revision')
  })
})
