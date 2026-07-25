import { describe, expect, it, vi } from 'vitest'

import { createStrongBoxBridge } from './strong-box'

type RequestMessage = {
  type: 'request'
  id: number
  payload: ArrayBuffer
}

class SynchronousWorker extends EventTarget {
  readonly terminate = vi.fn()
  readonly posted: RequestMessage[] = []

  constructor() {
    super()
    queueMicrotask(() => {
      this.dispatchEvent(new MessageEvent('message', { data: { type: 'ready' } }))
    })
  }

  postMessage(message: RequestMessage) {
    this.posted.push(message)
    const result = new Uint8Array(message.payload).slice().buffer
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
  it('uses an injected worker factory and handles a synchronous response', async () => {
    const worker = new SynchronousWorker()
    const factory = vi.fn(() => worker as unknown as Worker)
    const bridge = await createStrongBoxBridge({ workerFactory: factory })

    const plaintext = new Uint8Array([4, 5, 6])
    await expect(
      bridge.encrypt({
        key: new Uint8Array(32).fill(1),
        context: new Uint8Array([2]),
        plaintext,
      }),
    ).resolves.toEqual(plaintext)

    expect(factory).toHaveBeenCalledOnce()
    expect(worker.posted).toHaveLength(1)
  })
})
