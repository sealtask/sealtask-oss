import { describe, expect, it } from 'vitest'

import { KEY_SIZE_BYTES } from '../runtime/constants'
import type { OpaqueExportKey } from '../runtime/opaque'
import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  createDataKeyCiphertext,
  createDataKeyCiphertextWithOpaqueExportKey,
  createRecoveryDataKeyCiphertext,
  DATA_KEY_SALT_BYTES,
  decryptDataKeyCiphertext,
  decryptDataKeyCiphertextWithOpaqueExportKey,
  decryptRecoveryDataKeyCiphertext,
  getDataKeyCiphertextVersion,
} from './data-key'
import { deriveInviteKeyPair } from './invite-key'
import {
  formatRecoveryKey,
  parseRecoveryKey,
  RecoveryKeyFormatError,
} from './recovery-key'

const TAG_LENGTH = 16

const deterministicBox: StrongBoxBridge = {
  async encrypt({ key, context, plaintext }) {
    return concat(computeTag(key, context), xorWithKey(plaintext, key))
  },
  async decrypt({ key, context, ciphertext }) {
    if (ciphertext.length <= TAG_LENGTH) {
      throw new Error('ciphertext too short')
    }
    const expected = computeTag(key, context)
    if (
      ciphertext
        .slice(0, TAG_LENGTH)
        .some((byte, index) => byte !== expected[index])
    ) {
      throw new Error('authentication failed')
    }
    return xorWithKey(ciphertext.slice(TAG_LENGTH), key)
  },
}

const derivePasswordKey = async (
  password: string,
  salt: Uint8Array,
) => {
  const key = new Uint8Array(KEY_SIZE_BYTES)
  for (let index = 0; index < key.length; index += 1) {
    key[index] =
      salt[index % salt.length] ^
      (password.charCodeAt(index % password.length) & 0xff)
  }
  return key
}

const deriveExportKey = async (
  _exportKey: OpaqueExportKey,
  info: string,
) => {
  const key = new Uint8Array(KEY_SIZE_BYTES)
  for (let index = 0; index < key.length; index += 1) {
    key[index] = (info.charCodeAt(index % info.length) + index) & 0xff
  }
  return key
}

describe('data-key wrapping protocols', () => {
  it('round-trips legacy password wrapping and rejects the wrong password', async () => {
    const salt = new Uint8Array(DATA_KEY_SALT_BYTES).fill(7)
    const dataKey = new Uint8Array(KEY_SIZE_BYTES).fill(3)
    const wrapped = await createDataKeyCiphertext({
      password: 'correct horse battery staple',
      salt,
      dataKey,
      strongBox: deterministicBox,
      deriveKey: derivePasswordKey,
    })
    await expect(
      decryptDataKeyCiphertext({
        password: 'correct horse battery staple',
        ciphertext: wrapped.ciphertext,
        strongBox: deterministicBox,
        deriveKey: derivePasswordKey,
      }),
    ).resolves.toEqual({ dataKey, salt })
    await expect(
      decryptDataKeyCiphertext({
        password: 'wrong',
        ciphertext: wrapped.ciphertext,
        strongBox: deterministicBox,
        deriveKey: derivePasswordKey,
      }),
    ).rejects.toThrow('authentication failed')
  })

  it('round-trips OPAQUE and recovery wrappers with distinct domains', async () => {
    const exportKey = new Uint8Array(64).fill(4)
    const dataKey = new Uint8Array(KEY_SIZE_BYTES).fill(9)
    const opaque = await createDataKeyCiphertextWithOpaqueExportKey({
      exportKey,
      dataKey,
      strongBox: deterministicBox,
      deriveExportKey,
    })
    const recovery = await createRecoveryDataKeyCiphertext({
      recoveryExportKey: exportKey,
      dataKey,
      strongBox: deterministicBox,
      deriveExportKey,
    })

    expect(getDataKeyCiphertextVersion(opaque.ciphertext)).toBe(2)
    await expect(
      decryptDataKeyCiphertextWithOpaqueExportKey({
        exportKey,
        ciphertext: opaque.ciphertext,
        strongBox: deterministicBox,
        deriveExportKey,
      }),
    ).resolves.toEqual({ dataKey })
    await expect(
      decryptRecoveryDataKeyCiphertext({
        recoveryExportKey: exportKey,
        ciphertext: recovery.ciphertext,
        strongBox: deterministicBox,
        deriveExportKey,
      }),
    ).resolves.toEqual({ dataKey })
    expect(opaque.ciphertext).not.toBe(recovery.ciphertext)
  })
})

describe('invite and recovery key protocols', () => {
  it('derives deterministic user-scoped X25519 key pairs', async () => {
    const dataKey = new Uint8Array(32).fill(0x2a)
    const first = await deriveInviteKeyPair({
      dataKey,
      userId: 'user-a',
    })
    const retry = await deriveInviteKeyPair({
      dataKey,
      userId: 'user-a',
    })
    const otherUser = await deriveInviteKeyPair({
      dataKey,
      userId: 'user-b',
    })

    expect(first).toEqual(retry)
    expect(first.privateKey).toHaveLength(32)
    expect(first.publicKey).toHaveLength(32)
    expect(otherUser.privateKey).not.toEqual(first.privateKey)
    expect(first.privateKey[0] & 7).toBe(0)
    expect(first.privateKey[31] & 0x80).toBe(0)
    expect(first.privateKey[31] & 0x40).toBe(0x40)
  })

  it('formats, normalizes, and parses recovery keys', () => {
    const recoveryKeyId =
      '018F5B74-2F8A-7A51-8F4F-6F1F2C3B4A5D'
    const secret =
      'abcDEF123_-abcDEF123_-abcDEF123_-abcDEF123_'
    expect(
      parseRecoveryKey(formatRecoveryKey(recoveryKeyId, secret)),
    ).toEqual({
      recoveryKeyId: recoveryKeyId.toLowerCase(),
      secret,
    })
    expect(() => parseRecoveryKey('not-a-key')).toThrow(
      RecoveryKeyFormatError,
    )
  })
})

function concat(left: Uint8Array, right: Uint8Array): Uint8Array {
  const result = new Uint8Array(left.length + right.length)
  result.set(left)
  result.set(right, left.length)
  return result
}

function xorWithKey(data: Uint8Array, key: Uint8Array): Uint8Array {
  return data.map((byte, index) => byte ^ key[index % key.length])
}

function computeTag(
  key: Uint8Array,
  context: Uint8Array,
): Uint8Array {
  const tag = new Uint8Array(TAG_LENGTH)
  for (let index = 0; index < tag.length; index += 1) {
    tag[index] =
      key[index % key.length] ^
      context[index % context.length] ^
      0xa5
  }
  return tag
}
