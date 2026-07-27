import { describe, expect, it } from 'vitest'

import {
  computeOwnerAuthorizedStatementDigest,
  createOwnerAuthorizedTransparencyStatement,
  deriveOwnerTransparencyIdentityPublicKey,
  verifyOwnerAuthorizedTransparencyStatement,
} from './transparency-authorization'

const USER_ID = '11111111-1111-1111-1111-111111111111'
const DATA_KEY = Uint8Array.from({ length: 32 }, (_, index) => index + 1)
const INVITE_KEY = new Uint8Array(32).fill(0x51)
const PREVIOUS_DIGEST = new Uint8Array(32).fill(0x37)

describe('owner-authorized transparency statements', () => {
  it('derives deterministic Ed25519 identity material and signs the canonical digest', async () => {
    const first = await createOwnerAuthorizedTransparencyStatement({
      dataKey: DATA_KEY,
      userId: USER_ID,
      generation: 3,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: PREVIOUS_DIGEST,
    })
    const retry = await createOwnerAuthorizedTransparencyStatement({
      dataKey: DATA_KEY,
      userId: USER_ID,
      generation: 3,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: PREVIOUS_DIGEST,
    })

    expect(first.identityPublicKey).toEqual(retry.identityPublicKey)
    await expect(deriveOwnerTransparencyIdentityPublicKey({
      dataKey: DATA_KEY,
      userId: USER_ID,
    })).resolves.toEqual(first.identityPublicKey)
    expect(first.statementDigest).toEqual(retry.statementDigest)
    expect(first.ownerSignature).toEqual(retry.ownerSignature)
    expect(toHex(first.identityPublicKey)).toBe(
      'f2adc3866f72f22a155055fe668f69d2a7c1b310103e6c8f8615739049c30c82',
    )
    expect(toHex(first.statementDigest)).toBe(
      '2837a9ed9a4035edf18279b9203140b430ea30f93e3de52afc6a29e76b9c98a2',
    )
    expect(toHex(first.ownerSignature)).toBe(
      '7d0738e831429c6db6d52242d82255731076a32fed7bbfaf1c700fadfa298d73'
      + 'a74f16e3289e091677924c90499c24427f6ac8fcd7737ad9a365aa6fb19c9704',
    )
    expect(await verifyOwnerAuthorizedTransparencyStatement(first)).toBe(true)
  })

  it('binds every rotation field and rejects tampering', async () => {
    const statement = await createOwnerAuthorizedTransparencyStatement({
      dataKey: DATA_KEY,
      userId: USER_ID,
      generation: 3,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: PREVIOUS_DIGEST,
    })
    const tampered = {
      ...statement,
      generation: statement.generation + 1,
    }

    expect(await verifyOwnerAuthorizedTransparencyStatement(tampered)).toBe(false)
    await expect(computeOwnerAuthorizedStatementDigest({
      ...statement,
      previousStatementDigest: new Uint8Array(31),
    })).rejects.toThrow(/previous_statement_digest must be exactly 32 bytes/)
  })

  it('uses a distinct owner identity for another user or data key', async () => {
    const first = await createOwnerAuthorizedTransparencyStatement({
      dataKey: DATA_KEY,
      userId: USER_ID,
      generation: 0,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: null,
    })
    const otherUser = await createOwnerAuthorizedTransparencyStatement({
      dataKey: DATA_KEY,
      userId: '22222222-2222-2222-2222-222222222222',
      generation: 0,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: null,
    })
    const otherDataKey = await createOwnerAuthorizedTransparencyStatement({
      dataKey: new Uint8Array(32).fill(0xa5),
      userId: USER_ID,
      generation: 0,
      invitePublicKey: INVITE_KEY,
      previousStatementDigest: null,
    })

    expect(first.identityPublicKey).not.toEqual(otherUser.identityPublicKey)
    expect(first.identityPublicKey).not.toEqual(otherDataKey.identityPublicKey)
  })

  it.each([new Uint8Array(31), new Uint8Array(33)])(
    'rejects a non-32-byte account data key',
    async (dataKey) => {
      await expect(createOwnerAuthorizedTransparencyStatement({
        dataKey,
        userId: USER_ID,
        generation: 0,
        invitePublicKey: INVITE_KEY,
        previousStatementDigest: null,
      })).rejects.toThrow(/data_key must be exactly 32 bytes/)
    },
  )
})

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
