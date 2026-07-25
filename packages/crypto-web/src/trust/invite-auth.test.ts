import { decode as cborDecode } from 'cbor-x'
import { afterAll, describe, expect, it } from 'vitest'

import { zeroBytes } from '../runtime/bytes'
import { deriveInviteKeyPair } from '../protocols/invite-key'
import {
  INVITE_PREVIEW_AUTH_VERSION,
  LEGACY_INVITE_PREVIEW_AUTH_VERSION,
  buildInvitePreviewAuthMessageFromPackageBody,
  createInvitePreviewAuthenticator,
  encodeInvitePreviewMacMessage,
  verifyInvitePreviewAuthenticator,
  type InvitePreviewAuthenticator,
  type InvitePreviewMacParams,
} from './invite-auth'

const inviter = await deriveInviteKeyPair({
  dataKey: new Uint8Array(32).fill(0x19),
  userId: 'inviter-user',
})
const recipient = await deriveInviteKeyPair({
  dataKey: new Uint8Array(32).fill(0x83),
  userId: 'recipient-user',
})

afterAll(() => {
  zeroBytes(inviter.privateKey)
  zeroBytes(inviter.publicKey)
  zeroBytes(recipient.privateKey)
  zeroBytes(recipient.publicKey)
})

describe('invite preview authenticator wire protocol', () => {
  it('preserves unknown package fields and omits only the authenticator', () => {
    const message = buildInvitePreviewAuthMessageFromPackageBody({
      signedBody: {
        work_list_id: 'work-list-1',
        membership_id: 'membership-1',
        title: 'Project',
        expires_at: null,
        unknown_top_level: { nested: ['kept', 42] },
        authenticator: { server_supplied: 'must-not-be-signed-twice' },
      },
      role: 'member',
      packageVersion: 2,
    })

    expect(message?.packageBody).toEqual({
      work_list_id: 'work-list-1',
      membership_id: 'membership-1',
      title: 'Project',
      expires_at: null,
      unknown_top_level: { nested: ['kept', 42] },
    })
  })

  it('creates and verifies the current authenticator version', async () => {
    const message = requireMessage()
    const authenticator = await createInvitePreviewAuthenticator({
      message,
      inviterAuth: {
        privateKey: inviter.privateKey.slice(),
        publicKey: inviter.publicKey.slice(),
        generation: 4,
      },
      recipientPublicKey: recipient.publicKey.slice(),
    })

    expect(authenticator.version).toBe(INVITE_PREVIEW_AUTH_VERSION)
    await expect(verifyInvitePreviewAuthenticator({
      message,
      authenticator,
      recipientPrivateKey: recipient.privateKey.slice(),
      recipientPublicKey: recipient.publicKey.slice(),
      inviterPublicKey: inviter.publicKey.slice(),
    })).resolves.toBe(true)
  })

  it('does not trial-verify a current MAC after relabeling it as legacy', async () => {
    const message = requireMessage()
    const current = await createInvitePreviewAuthenticator({
      message,
      inviterAuth: {
        privateKey: inviter.privateKey.slice(),
        publicKey: inviter.publicKey.slice(),
        generation: 4,
      },
      recipientPublicKey: recipient.publicKey.slice(),
    })
    const relabeled: InvitePreviewAuthenticator = {
      ...current,
      version: LEGACY_INVITE_PREVIEW_AUTH_VERSION,
      body: { ...current.body },
    }

    await expect(verifyInvitePreviewAuthenticator({
      message,
      authenticator: relabeled,
      recipientPrivateKey: recipient.privateKey.slice(),
      recipientPublicKey: recipient.publicKey.slice(),
      inviterPublicKey: inviter.publicKey.slice(),
    })).resolves.toBe(false)
  })

  it('keeps signed package data separate from protocol-owned fields in CBOR', () => {
    const message = requireMessage()
    const encoded = encodeInvitePreviewMacMessage({
      message,
      privateKey: inviter.privateKey,
      peerPublicKey: recipient.publicKey,
      inviterUserId: 'inviter-user',
      inviterKeyGeneration: 4,
      inviterKeyFingerprintB64: 'inviter-fingerprint',
      recipientKeyFingerprintB64: 'recipient-fingerprint',
    } satisfies InvitePreviewMacParams)
    const decoded = cborDecode(encoded) as {
      body: {
        package: Record<string, unknown>
        protocol: Record<string, unknown>
      }
    }

    expect(decoded.body.package).toMatchObject({
      work_list_id: 'work-list-1',
      role: 'untrusted-package-field',
    })
    expect(decoded.body.protocol).toMatchObject({
      invite_protocol_version: 2,
      role: 'member',
      inviter_user_id: 'inviter-user',
    })
  })
})

function requireMessage() {
  const message = buildInvitePreviewAuthMessageFromPackageBody({
    signedBody: {
      work_list_id: 'work-list-1',
      membership_id: 'membership-1',
      title: 'Project',
      expires_at: null,
      inviter: { id: 'inviter-user' },
      role: 'untrusted-package-field',
    },
    role: 'member',
    packageVersion: 2,
  })
  if (!message) {
    throw new Error('test message did not validate')
  }
  return message
}
