import { decodeBase64 } from '../src/runtime/base64'
import { zeroBytes } from '../src/runtime/bytes'
import { deriveKeyFromPassword } from '../src/runtime/argon2'
import { createStrongBoxBridge } from '../src/runtime/strong-box'
import {
  createInvitePayloadFromPreparedMetadata,
  normalizeInvitePackageInviter,
} from '../src/protocols/invite-issuance'
import { deriveInviteKeyPair } from '../src/protocols/invite-key'
import { decryptWorkListKeyCiphertext } from '../src/protocols/work-list'
import {
  buildInviteAcceptancePayload,
  decryptPendingInvitationPreview,
  verifyInviteForAcceptance,
  type PendingInvitationCrypto,
} from '../src/trust/invite-verification'
import passwordV1Fixture from '../test/fixtures/password-v1.json'

const encoder = new TextEncoder()
const decoder = new TextDecoder()

type SecureCryptoFlowResult = {
  acceptedListKeyMatches: boolean
  hpkeRoundTrip: string
  inviteAuthentication: string
  invitePreviewTitle: string
  isSecureContext: boolean
  passwordV1WrappingKeyMatches: boolean
  strongBoxRoundTrip: string
}

declare global {
  interface Window {
    runSecureCryptoFlow: () => Promise<SecureCryptoFlowResult>
  }
}

window.runSecureCryptoFlow = async () => {
  if (!window.isSecureContext) {
    throw new Error('The public browser crypto test requires a secure context.')
  }
  if (!window.crypto?.subtle || typeof window.Worker !== 'function') {
    throw new Error('The public browser crypto test requires WebCrypto and module Workers.')
  }

  const bridge = await createStrongBoxBridge()
  const bridgeWithDispose = bridge as typeof bridge & { dispose?: () => void }
  const inviterDataKey = new Uint8Array(32).fill(0x31)
  const inviteeDataKey = new Uint8Array(32).fill(0x72)
  const listKey = new Uint8Array(32).fill(0xa4)
  const strongBoxKey = new Uint8Array(32).fill(0x19)
  let derivedPasswordWrappingKey: Uint8Array | null = null
  let expectedPasswordWrappingKey: Uint8Array | null = null
  let inviterKeys: Awaited<ReturnType<typeof deriveInviteKeyPair>> | null = null
  let inviteeKeys: Awaited<ReturnType<typeof deriveInviteKeyPair>> | null = null

  try {
    const passwordVector = passwordV1Fixture
    expectedPasswordWrappingKey = decodeBase64(passwordVector.wrappingKeyB64)
    derivedPasswordWrappingKey = await deriveKeyFromPassword(
      passwordVector.password,
      decodeBase64(passwordVector.saltB64),
    )
    const passwordV1WrappingKeyMatches =
      derivedPasswordWrappingKey.length === expectedPasswordWrappingKey.length &&
      derivedPasswordWrappingKey.every(
        (byte, index) => byte === expectedPasswordWrappingKey?.[index],
      )
    if (!passwordV1WrappingKeyMatches) {
      throw new Error(
        'Browser Argon2id did not reproduce the Rust password-v1 wrapping key fixture.',
      )
    }

    const strongBoxContext = encoder.encode('sealtask.crypto-web.browser-test')
    const strongBoxPlaintext = encoder.encode('module worker and canonical wasm')
    const strongBoxCiphertext = await bridge.encrypt({
      key: strongBoxKey,
      context: strongBoxContext,
      plaintext: strongBoxPlaintext,
    })
    const strongBoxRecovered = await bridge.decrypt({
      key: strongBoxKey,
      context: strongBoxContext,
      ciphertext: strongBoxCiphertext,
    })
    const strongBoxRoundTrip = decoder.decode(strongBoxRecovered)
    if (strongBoxRoundTrip !== decoder.decode(strongBoxPlaintext)) {
      throw new Error('StrongBox Worker/WASM round trip did not recover the plaintext.')
    }

    inviteeKeys = await deriveInviteKeyPair({
      dataKey: inviteeDataKey,
      userId: 'invitee-user',
    })
    inviterKeys = await deriveInviteKeyPair({
      dataKey: inviterDataKey,
      userId: 'inviter-user',
    })

    if (!bridge.hpkeEncap || !bridge.hpkeDecap) {
      throw new Error('The StrongBox WASM bridge does not expose HPKE operations.')
    }
    const hpkeInfo = encoder.encode('sealtask.crypto-web.hpke.browser-test')
    const hpkeAad = encoder.encode('secure-chromium')
    const hpkePlaintext = encoder.encode('real worker hpke')
    const hpkeSealed = await bridge.hpkeEncap({
      recipientPublicKey: inviteeKeys.publicKey,
      info: hpkeInfo,
      aad: hpkeAad,
      plaintext: hpkePlaintext,
    })
    const hpkeRecovered = await bridge.hpkeDecap({
      recipientPrivateKey: inviteeKeys.privateKey,
      info: hpkeInfo,
      aad: hpkeAad,
      enc: hpkeSealed.enc,
      ciphertext: hpkeSealed.ciphertext,
    })
    const hpkeRoundTrip = decoder.decode(hpkeRecovered)
    if (hpkeRoundTrip !== decoder.decode(hpkePlaintext)) {
      throw new Error('StrongBox Worker/WASM HPKE did not recover the plaintext.')
    }

    const invitePayload = await createInvitePayloadFromPreparedMetadata({
      bridge,
      workListId: 'work-list-1',
      listKey,
      bindingKeyB64: 'browser-test-binding-key',
      recipientPublicKey: inviteeKeys.publicKey,
      target: {
        kind: 'registered',
        id: 'invitee-membership-target',
        userId: 'invitee-user',
        email: 'invitee@example.test',
        name: 'Invitee',
        membershipRole: 'member',
        expiresAt: null,
      },
      listTitle: 'Secure Chromium invitation',
      membershipId: 'membership-1',
      inviter: normalizeInvitePackageInviter({
        id: 'inviter-user',
        name: 'Inviter',
        email: 'inviter@example.test',
      }),
      inviterAuth: {
        privateKey: inviterKeys.privateKey,
        publicKey: inviterKeys.publicKey,
        generation: 7,
      },
      expiresAt: null,
      inviteKeyProof: {
        kind: 'browser-test-transparency-proof',
      },
      inviteProtocolVersion: 2,
      reservationRevision: 4,
      dependencies: {
        now: () => new Date('2026-07-25T12:00:00.000Z'),
        randomBytes: () => new Uint8Array(32).fill(0x5c),
      },
    })

    const invitation: PendingInvitationCrypto = {
      membershipId: invitePayload.membershipId,
      workListId: 'work-list-1',
      role: invitePayload.role,
      invitationState: 'ready',
      expiresAt: invitePayload.expiresAt,
      inviteProtocolVersion: invitePayload.inviteProtocolVersion,
      reservationRevision:
        invitePayload.reservationRevision === undefined
          ? undefined
          : invitePayload.reservationRevision + 1,
      workListKeyCiphertext: invitePayload.workListKeyCiphertext,
      recipientCiphertext: invitePayload.recipientCiphertext,
      invitePackageCiphertext: invitePayload.invitePackageCiphertext,
      saltMember: invitePayload.saltMember,
    }
    const resolveInviteKey = async ({
      userId,
      generation,
    }: {
      userId: string
      generation: number
    }) => {
      if (userId !== 'inviter-user' || generation !== 7 || !inviterKeys) {
        throw new Error('Unexpected inviter key lookup.')
      }
      return {
        invitePublicKey: inviterKeys.publicKey.slice(),
        generation,
        identityPublicKey: new Uint8Array(32).fill(0x91),
        authorization: 'owner-authorized' as const,
      }
    }

    const preview = await decryptPendingInvitationPreview({
      invitation,
      dataKey: inviteeDataKey,
      userId: 'invitee-user',
      strongBox: bridge,
      resolveInviteKey,
    })
    if (preview.status !== 'decrypted') {
      throw new Error(`Invite preview was not authenticated: ${preview.status}.`)
    }

    const verifiedInvite = await verifyInviteForAcceptance({
      invitation,
      dataKey: inviteeDataKey,
      userId: 'invitee-user',
      strongBox: bridge,
      resolveInviteKey,
    })
    const inviteAuthentication = verifiedInvite.authentication
    const acceptance = await buildInviteAcceptancePayload({
      verifiedInvite,
      dataKey: inviteeDataKey,
      strongBox: bridge,
    })
    const acceptedListKey = await decryptWorkListKeyCiphertext({
      ciphertext: acceptance.workListKeyCiphertext,
      dataKey: inviteeDataKey,
      strongBox: bridge,
    })
    const acceptedListKeyMatches =
      acceptedListKey.length === listKey.length &&
      acceptedListKey.every((byte, index) => byte === listKey[index])
    if (
      !acceptedListKeyMatches ||
      decodeBase64(acceptance.membershipProof).length !== 32
    ) {
      throw new Error('Invite acceptance did not produce the expected encrypted artifacts.')
    }

    return {
      acceptedListKeyMatches,
      hpkeRoundTrip,
      inviteAuthentication,
      invitePreviewTitle: preview.title,
      isSecureContext: window.isSecureContext,
      passwordV1WrappingKeyMatches,
      strongBoxRoundTrip,
    }
  } finally {
    zeroBytes(derivedPasswordWrappingKey)
    zeroBytes(expectedPasswordWrappingKey)
    zeroBytes(inviterKeys?.privateKey)
    zeroBytes(inviterKeys?.publicKey)
    zeroBytes(inviteeKeys?.privateKey)
    zeroBytes(inviteeKeys?.publicKey)
    zeroBytes(inviterDataKey)
    zeroBytes(inviteeDataKey)
    zeroBytes(listKey)
    zeroBytes(strongBoxKey)
    bridgeWithDispose.dispose?.()
  }
}

document.documentElement.dataset.cryptoTestReady = 'true'
