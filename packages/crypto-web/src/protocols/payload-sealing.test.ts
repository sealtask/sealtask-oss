import { encode as cborEncode } from 'cbor-x'
import { describe, expect, it, vi } from 'vitest'

import {
  constantTimeEquals,
  toUint8Array,
} from '../runtime/bytes'
import { encodeBase64 } from '../runtime/base64'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import type {
  StrongBoxBridge,
  StrongBoxDecryptInput,
  StrongBoxEncryptInput,
} from '../runtime/strong-box'
import {
  openPayloadEnvelope,
  sealPayloadEnvelope,
} from './payload-sealing'
import { serializeSealedPayloadBase64 } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import { decryptCommentPayload } from './comment'
import {
  buildTaskPayloadEnvelope,
  decryptTaskPayload,
  encryptTaskPayload,
} from './task'

const TASK_DOMAIN = {
  kind: 'task',
  context: new TextEncoder().encode('worklist.task.v1'),
} as const
const OTHER_TASK_CONTEXT = {
  kind: 'task',
  context: new TextEncoder().encode('worklist.task.other.v1'),
} as const
const key = new Uint8Array(32).fill(7)
const otherKey = new Uint8Array(32).fill(8)
const taskEnvelope = {
  kind: 'task' as const,
  version: 1,
  body: { title: 'Ship release' },
}

if (false) {
  void sealPayloadEnvelope(taskEnvelope, key, {
    // @ts-expect-error The envelope, validator kind, and StrongBox context must share one domain.
    kind: 'comment',
    context: new Uint8Array(),
  })
}

function identityBridge(): StrongBoxBridge {
  return {
    async encrypt({ plaintext }) {
      return plaintext.slice()
    },
    async decrypt({ ciphertext }) {
      return ciphertext.slice()
    },
  }
}

function bindingBridge(): StrongBoxBridge {
  const frames = new Map<string, {
    key: Uint8Array
    context: Uint8Array
    plaintext: Uint8Array
  }>()
  let nextFrameId = 0

  return {
    async encrypt(input) {
      nextFrameId += 1
      const ciphertext = new Uint8Array([nextFrameId])
      frames.set(encodeBase64(ciphertext), {
        key: input.key.slice(),
        context: input.context.slice(),
        plaintext: input.plaintext.slice(),
      })
      return ciphertext
    },
    async decrypt(input) {
      const frame = frames.get(encodeBase64(input.ciphertext))
      if (
        !frame ||
        !constantTimeEquals(frame.key, input.key) ||
        !constantTimeEquals(frame.context, input.context)
      ) {
        throw new Error('StrongBox key or context mismatch')
      }
      return frame.plaintext.slice()
    },
  }
}

describe('structured payload sealing core', () => {
  it('preserves the existing CBOR and sealed-blob wire path', async () => {
    const plaintext = toUint8Array(cborEncode(taskEnvelope))

    const sealed = await sealPayloadEnvelope(
      taskEnvelope,
      key,
      TASK_DOMAIN,
      identityBridge(),
    )

    expect(sealed).toEqual(toSealedBlob({
      version: SEALED_PAYLOAD_VERSION,
      ciphertext: plaintext,
    }))
    await expect(
      openPayloadEnvelope(
        sealed.base64,
        key,
        TASK_DOMAIN,
        identityBridge(),
      ),
    ).resolves.toEqual(taskEnvelope)
  })

  it('keeps key and context binding explicit at the core boundary', async () => {
    const strongBox = bindingBridge()
    const sealed = await sealPayloadEnvelope(
      taskEnvelope,
      key,
      TASK_DOMAIN,
      strongBox,
    )

    await expect(
      openPayloadEnvelope(sealed.base64, otherKey, TASK_DOMAIN, strongBox),
    ).rejects.toThrow('StrongBox key or context mismatch')
    await expect(
      openPayloadEnvelope(sealed.base64, key, OTHER_TASK_CONTEXT, strongBox),
    ).rejects.toThrow('StrongBox key or context mismatch')
  })

  it('keeps public task and comment wrappers in distinct StrongBox domains', async () => {
    const strongBox = bindingBridge()
    const sealed = await encryptTaskPayload({
      envelope: buildTaskPayloadEnvelope({ title: 'Ship release' }),
      listKey: key,
      strongBox,
    })

    await expect(
      decryptTaskPayload({
        ciphertext: sealed.base64,
        listKey: key,
        strongBox,
      }),
    ).resolves.toEqual(buildTaskPayloadEnvelope({ title: 'Ship release' }))
    await expect(
      decryptTaskPayload({
        ciphertext: sealed.base64,
        listKey: otherKey,
        strongBox,
      }),
    ).rejects.toThrow('StrongBox key or context mismatch')
    await expect(
      decryptCommentPayload({
        ciphertext: sealed.base64,
        listKey: key,
        strongBox,
      }),
    ).rejects.toThrow('StrongBox key or context mismatch')
  })

  it('validates before encryption and after authenticated decryption', async () => {
    const encrypt = vi.fn(async (_input: StrongBoxEncryptInput) =>
      new Uint8Array([1]),
    )
    const decrypt = vi.fn(async (_input: StrongBoxDecryptInput) =>
      toUint8Array(cborEncode({
        kind: 'task',
        version: 1,
        body: { title: 42 },
      })),
    )
    const strongBox: StrongBoxBridge = { encrypt, decrypt }

    await expect(
      sealPayloadEnvelope(
        { kind: 'task', version: 1, body: { title: 42 } },
        key,
        TASK_DOMAIN,
        strongBox,
      ),
    ).rejects.toThrow('title must be a string')
    expect(encrypt).not.toHaveBeenCalled()

    await expect(
      openPayloadEnvelope(
        serializeSealedPayloadBase64({
          version: 1,
          ciphertext: new Uint8Array([1]),
        }),
        key,
        TASK_DOMAIN,
        strongBox,
      ),
    ).rejects.toThrow('title must be a string')
    expect(decrypt).toHaveBeenCalledOnce()
  })

  it('rejects malformed outer payloads before invoking StrongBox', async () => {
    const decrypt = vi.fn(async (_input: StrongBoxDecryptInput) =>
      new Uint8Array(),
    )
    const strongBox: StrongBoxBridge = {
      encrypt: async () => new Uint8Array([1]),
      decrypt,
    }

    await expect(
      openPayloadEnvelope(
        encodeBase64(new Uint8Array([0x80])),
        key,
        TASK_DOMAIN,
        strongBox,
      ),
    ).rejects.toThrow('Invalid sealed payload structure')
    expect(decrypt).not.toHaveBeenCalled()
  })
})
