import type {
  HpkeDecapInput,
  HpkeEncapInput,
  HpkeEncapResult,
  StrongBoxBridge,
  StrongBoxDecryptInput,
  StrongBoxEncryptInput,
} from './strong-box-types'

type EncodedRequest =
  | {
      type: 'request'
      id: number
      op: 'encrypt' | 'decrypt'
      key: ArrayBuffer
      context: ArrayBuffer
      payload: ArrayBuffer
    }
  | {
      type: 'request'
      id: number
      op: 'hpke_encap'
      recipientPublicKey: ArrayBuffer
      info: ArrayBuffer
      aad: ArrayBuffer
      payload: ArrayBuffer
    }
  | {
      type: 'request'
      id: number
      op: 'hpke_decap'
      recipientPrivateKey: ArrayBuffer
      info: ArrayBuffer
      aad: ArrayBuffer
      enc: ArrayBuffer
      payload: ArrayBuffer
    }

type WorkerResponse =
  | { type: 'ready' }
  | { type: 'init-error'; error: SerializedWorkerError }
  | {
      type: 'response'
      id: number
      status: 'ok'
      result: ArrayBuffer
      meta?: { cacheHit?: boolean }
    }
  | {
      type: 'response'
      id: number
      status: 'error'
      error: SerializedWorkerError
    }

type SerializedWorkerError = {
  message: string
  name?: string
}

export type StrongBoxWorkerFactory = () => Worker

export function createDefaultStrongBoxWorker(): Worker {
  return new Worker(new URL('./strong-box.worker.ts', import.meta.url), {
    type: 'module',
  })
}

export class StrongBoxWorkerClient implements StrongBoxBridge {
  private readonly worker: Worker
  private readonly pending = new Map<number, PendingRequest>()
  private nextId = 0
  private disposed = false

  private constructor(worker: Worker) {
    this.worker = worker
    this.worker.addEventListener('message', (event: MessageEvent<WorkerResponse>) => {
      const data = event.data
      if (!data || data.type !== 'response') {
        return
      }

      const pending = this.pending.get(data.id)
      if (!pending) {
        return
      }

      this.pending.delete(data.id)

      if (data.status === 'ok') {
        try {
          const value = pending.transform
            ? pending.transform(data.result)
            : new Uint8Array(data.result)
          pending.resolve(value)
        } catch (error) {
          pending.reject(error)
        }
      } else {
        pending.reject(deserializeWorkerError(data.error, 'StrongBox worker failed'))
      }
    })

    this.worker.addEventListener('error', (event) => {
      this.rejectPending(event.error ?? new Error('StrongBox worker error'))
    })
    this.worker.addEventListener('messageerror', () => {
      this.rejectPending(new Error('StrongBox worker message could not be decoded'))
    })
  }

  static async create(workerFactory: StrongBoxWorkerFactory = createDefaultStrongBoxWorker) {
    const worker = workerFactory()
    try {
      await waitForWorkerReady(worker)
      return new StrongBoxWorkerClient(worker)
    } catch (error) {
      worker.terminate()
      throw error
    }
  }

  async encrypt(input: StrongBoxEncryptInput) {
    return this.invoke('encrypt', input)
  }

  async decrypt(input: StrongBoxDecryptInput) {
    return this.invoke('decrypt', input)
  }

  async hpkeEncap(input: HpkeEncapInput): Promise<HpkeEncapResult> {
    return this.invokeHpke('hpke_encap', input)
  }

  async hpkeDecap(input: HpkeDecapInput): Promise<Uint8Array> {
    return this.invokeHpke('hpke_decap', input)
  }

  dispose() {
    if (this.disposed) {
      return
    }
    this.disposed = true
    this.worker.terminate()
    this.rejectPending(new Error('StrongBox worker client was disposed'))
  }

  private invoke(op: 'encrypt', input: StrongBoxEncryptInput): Promise<Uint8Array>
  private invoke(op: 'decrypt', input: StrongBoxDecryptInput): Promise<Uint8Array>
  private invoke(
    op: 'encrypt' | 'decrypt',
    input: StrongBoxEncryptInput | StrongBoxDecryptInput,
  ): Promise<Uint8Array> {
    const id = this.nextRequestId()
    const payloadView =
      op === 'encrypt'
        ? (input as StrongBoxEncryptInput).plaintext
        : (input as StrongBoxDecryptInput).ciphertext

    const message: EncodedRequest = {
      type: 'request',
      id,
      op,
      key: toTransferableBuffer(input.key),
      context: toTransferableBuffer(input.context),
      payload: toTransferableBuffer(payloadView),
    }

    return this.postRequest<Uint8Array>(
      id,
      message,
      [message.key, message.context, message.payload],
      (bytes) => new Uint8Array(bytes),
    )
  }

  private invokeHpke(op: 'hpke_encap', input: HpkeEncapInput): Promise<HpkeEncapResult>
  private invokeHpke(op: 'hpke_decap', input: HpkeDecapInput): Promise<Uint8Array>
  private invokeHpke(
    op: 'hpke_encap' | 'hpke_decap',
    input: HpkeEncapInput | HpkeDecapInput,
  ) {
    const id = this.nextRequestId()

    const message: EncodedRequest =
      op === 'hpke_encap'
        ? (() => {
            const encap = input as HpkeEncapInput
            return {
              type: 'request' as const,
              id,
              op,
              recipientPublicKey: toTransferableBuffer(encap.recipientPublicKey),
              info: toTransferableBuffer(encap.info),
              aad: toTransferableBuffer(encap.aad),
              payload: toTransferableBuffer(encap.plaintext),
            }
          })()
        : (() => {
            const decap = input as HpkeDecapInput
            return {
              type: 'request' as const,
              id,
              op,
              recipientPrivateKey: toTransferableBuffer(decap.recipientPrivateKey),
              info: toTransferableBuffer(decap.info),
              aad: toTransferableBuffer(decap.aad),
              enc: toTransferableBuffer(decap.enc),
              payload: toTransferableBuffer(decap.ciphertext),
            }
          })()

    if (op === 'hpke_encap') {
      const encap = message as Extract<EncodedRequest, { op: 'hpke_encap' }>
      return this.postRequest<HpkeEncapResult>(
        id,
        message,
        [encap.recipientPublicKey, encap.info, encap.aad, encap.payload],
        decodeHpkeResult,
      )
    }

    const decap = message as Extract<EncodedRequest, { op: 'hpke_decap' }>
    return this.postRequest<Uint8Array>(
      id,
      message,
      [decap.recipientPrivateKey, decap.info, decap.aad, decap.enc, decap.payload],
      (bytes) => decodeHpkeResult(bytes).payload,
    )
  }

  private postRequest<T extends Uint8Array | HpkeEncapResult>(
    id: number,
    message: EncodedRequest,
    transferList: Transferable[],
    transform: (value: ArrayBuffer) => T,
  ): Promise<T> {
    if (this.disposed) {
      return Promise.reject(new Error('StrongBox worker client was disposed'))
    }

    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: (value) => resolve(value as T),
        reject,
        transform,
      })

      try {
        this.worker.postMessage(message, transferList)
      } catch (error) {
        this.pending.delete(id)
        reject(error)
      }
    })
  }

  private nextRequestId() {
    if (this.disposed) {
      throw new Error('StrongBox worker client was disposed')
    }
    const id = this.nextId
    this.nextId += 1
    return id
  }

  private rejectPending(error: unknown) {
    for (const pending of this.pending.values()) {
      pending.reject(error)
    }
    this.pending.clear()
  }
}

type PendingRequest = {
  resolve: (value: Uint8Array | HpkeEncapResult) => void
  reject: (reason?: unknown) => void
  transform: (value: ArrayBuffer) => Uint8Array | HpkeEncapResult
}

function toTransferableBuffer(view: Uint8Array): ArrayBuffer {
  const copy = view.slice()
  return copy.buffer
}

function decodeHpkeResult(buffer: ArrayBuffer): HpkeEncapResult & { payload: Uint8Array } {
  if (buffer.byteLength < 12) {
    throw new Error('StrongBox HPKE response is truncated')
  }

  const view = new DataView(buffer)
  const bytes = new Uint8Array(buffer)
  const nonceLen = view.getUint32(0, true)
  const encLen = view.getUint32(4, true)
  const payloadLen = view.getUint32(8, true)
  const expectedLength = 12 + nonceLen + encLen + payloadLen
  if (expectedLength !== buffer.byteLength) {
    throw new Error('StrongBox HPKE response has invalid field lengths')
  }

  const nonceStart = 12
  const encStart = nonceStart + nonceLen
  const payloadStart = encStart + encLen

  const nonce = bytes.slice(nonceStart, encStart)
  const enc = bytes.slice(encStart, payloadStart)
  const payload = bytes.slice(payloadStart, expectedLength)

  return { nonce, enc, ciphertext: payload, payload }
}

function waitForWorkerReady(worker: Worker) {
  return new Promise<void>((resolve, reject) => {
    const handleMessage = (event: MessageEvent<WorkerResponse>) => {
      const data = event.data
      if (!data) {
        return
      }

      if (data.type === 'ready') {
        cleanup()
        resolve()
      } else if (data.type === 'init-error') {
        cleanup()
        reject(deserializeWorkerError(data.error, 'Failed to initialize StrongBox worker'))
      }
    }

    const handleError = (event: ErrorEvent) => {
      cleanup()
      reject(event.error ?? new Error(event.message))
    }

    const handleMessageError = () => {
      cleanup()
      reject(new Error('StrongBox worker initialization message could not be decoded'))
    }

    const cleanup = () => {
      worker.removeEventListener('message', handleMessage as EventListener)
      worker.removeEventListener('error', handleError)
      worker.removeEventListener('messageerror', handleMessageError)
    }

    worker.addEventListener('message', handleMessage as EventListener)
    worker.addEventListener('error', handleError)
    worker.addEventListener('messageerror', handleMessageError)
  })
}

function deserializeWorkerError(error: SerializedWorkerError | undefined, fallback: string) {
  const deserialized = new Error(error?.message ?? fallback)
  if (error?.name) {
    deserialized.name = error.name
  }
  return deserialized
}
