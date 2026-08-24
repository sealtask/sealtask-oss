import { describe, expect, it, vi } from 'vitest'

import { createStrongBoxBridge } from './strong-box'
import type { StrongBoxWorkerRequest } from './strong-box-worker-protocol'

type PostedRequest = {
  message: StrongBoxWorkerRequest
  transfer: Transferable[]
}

class SynchronousWorker extends EventTarget {
  readonly terminate = vi.fn()
  readonly posted: PostedRequest[] = []

  constructor() {
    super()
    queueMicrotask(() => {
      this.dispatchEvent(new MessageEvent('message', { data: { type: 'ready' } }))
    })
  }

  postMessage(message: StrongBoxWorkerRequest, transfer: Transferable[]) {
    this.posted.push({ message, transfer })
    const result =
      message.op === 'hpke_encap' || message.op === 'hpke_decap'
        ? encodeHpkeResult(message.payload)
        : message.payload.slice(0)
    // Deliberately respond synchronously. The client must register its pending
    // request before postMessage, even though real workers normally respond
    // on a later task.
    this.dispatchEvent(
      new MessageEvent('message', {
        data: {
          type: 'response',
          id: message.id,
          status: 'ok',
          result,
        },
      }),
    )
  }
}

describe('StrongBox worker injection', () => {
  it('sends encrypt fields as copied transferable buffers and handles a synchronous response', async () => {
    const worker = new SynchronousWorker()
    const factory = vi.fn(() => worker as unknown as Worker)
    const bridge = await createStrongBoxBridge({ workerFactory: factory })

    const key = viewOf([0, 1, 2, 0])
    const context = viewOf([0, 3, 4, 0])
    const plaintext = viewOf([0, 5, 6, 7, 0])
    await expect(
      bridge.encrypt({
        key,
        context,
        plaintext,
      }),
    ).resolves.toEqual(plaintext)

    expect(factory).toHaveBeenCalledOnce()
    expect(worker.posted).toHaveLength(1)

    const { message, transfer } = worker.posted[0]
    expect(message).toMatchObject({ type: 'request', id: 0, op: 'encrypt' })
    if (message.op !== 'encrypt') {
      throw new Error('expected an encrypt request')
    }
    expectCopiedBuffer(message.key, key)
    expectCopiedBuffer(message.context, context)
    expectCopiedBuffer(message.payload, plaintext)
    expectTransferList(transfer, [message.key, message.context, message.payload])
  })

  it('sends decrypt ciphertext as the payload transfer', async () => {
    const worker = new SynchronousWorker()
    const bridge = await createStrongBoxBridge({
      workerFactory: () => worker as unknown as Worker,
    })
    const key = viewOf([0, 11, 12, 0])
    const context = viewOf([0, 13, 14, 0])
    const ciphertext = viewOf([0, 15, 16, 17, 0])

    await expect(bridge.decrypt({ key, context, ciphertext })).resolves.toEqual(
      ciphertext,
    )

    const { message, transfer } = worker.posted[0]
    expect(message).toMatchObject({ type: 'request', id: 0, op: 'decrypt' })
    if (message.op !== 'decrypt') {
      throw new Error('expected a decrypt request')
    }
    expectCopiedBuffer(message.key, key)
    expectCopiedBuffer(message.context, context)
    expectCopiedBuffer(message.payload, ciphertext)
    expectTransferList(transfer, [message.key, message.context, message.payload])
  })

  it('sends every HPKE encap field in its transfer list', async () => {
    const worker = new SynchronousWorker()
    const bridge = await createStrongBoxBridge({
      workerFactory: () => worker as unknown as Worker,
    })
    const recipientPublicKey = viewOf([0, 21, 22, 0])
    const info = viewOf([0, 23, 24, 0])
    const aad = viewOf([0, 25, 26, 0])
    const plaintext = viewOf([0, 27, 28, 29, 0])

    if (!bridge.hpkeEncap) {
      throw new Error('expected the worker bridge to support HPKE encap')
    }
    await expect(
      bridge.hpkeEncap({ recipientPublicKey, info, aad, plaintext }),
    ).resolves.toMatchObject({
      nonce: new Uint8Array([1, 2]),
      enc: new Uint8Array([3, 4, 5]),
      ciphertext: plaintext,
    })

    const { message, transfer } = worker.posted[0]
    expect(message).toMatchObject({ type: 'request', id: 0, op: 'hpke_encap' })
    if (message.op !== 'hpke_encap') {
      throw new Error('expected an HPKE encap request')
    }
    expectCopiedBuffer(message.recipientPublicKey, recipientPublicKey)
    expectCopiedBuffer(message.info, info)
    expectCopiedBuffer(message.aad, aad)
    expectCopiedBuffer(message.payload, plaintext)
    expectTransferList(transfer, [
      message.recipientPublicKey,
      message.info,
      message.aad,
      message.payload,
    ])
  })

  it('sends every HPKE decap field in its transfer list', async () => {
    const worker = new SynchronousWorker()
    const bridge = await createStrongBoxBridge({
      workerFactory: () => worker as unknown as Worker,
    })
    const recipientPrivateKey = viewOf([0, 31, 32, 0])
    const info = viewOf([0, 33, 34, 0])
    const aad = viewOf([0, 35, 36, 0])
    const enc = viewOf([0, 37, 38, 0])
    const ciphertext = viewOf([0, 39, 40, 41, 0])

    if (!bridge.hpkeDecap) {
      throw new Error('expected the worker bridge to support HPKE decap')
    }
    await expect(
      bridge.hpkeDecap({ recipientPrivateKey, info, aad, enc, ciphertext }),
    ).resolves.toEqual(ciphertext)

    const { message, transfer } = worker.posted[0]
    expect(message).toMatchObject({ type: 'request', id: 0, op: 'hpke_decap' })
    if (message.op !== 'hpke_decap') {
      throw new Error('expected an HPKE decap request')
    }
    expectCopiedBuffer(message.recipientPrivateKey, recipientPrivateKey)
    expectCopiedBuffer(message.info, info)
    expectCopiedBuffer(message.aad, aad)
    expectCopiedBuffer(message.enc, enc)
    expectCopiedBuffer(message.payload, ciphertext)
    expectTransferList(transfer, [
      message.recipientPrivateKey,
      message.info,
      message.aad,
      message.enc,
      message.payload,
    ])
  })
})

function viewOf(bytes: number[]): Uint8Array {
  return new Uint8Array(bytes).subarray(1, bytes.length - 1)
}

function expectCopiedBuffer(buffer: ArrayBuffer, source: Uint8Array) {
  expect(buffer).not.toBe(source.buffer)
  expect(new Uint8Array(buffer)).toEqual(source)
}

function expectTransferList(
  transfer: Transferable[],
  buffers: ArrayBuffer[],
) {
  expect(transfer).toHaveLength(buffers.length)
  transfer.forEach((value, index) => {
    expect(value).toBe(buffers[index])
  })
}

function encodeHpkeResult(payload: ArrayBuffer): ArrayBuffer {
  const nonce = new Uint8Array([1, 2])
  const enc = new Uint8Array([3, 4, 5])
  const bytes = new Uint8Array(12 + nonce.length + enc.length + payload.byteLength)
  const view = new DataView(bytes.buffer)
  view.setUint32(0, nonce.length, true)
  view.setUint32(4, enc.length, true)
  view.setUint32(8, payload.byteLength, true)
  bytes.set(nonce, 12)
  bytes.set(enc, 12 + nonce.length)
  bytes.set(new Uint8Array(payload), 12 + nonce.length + enc.length)
  return bytes.buffer
}
