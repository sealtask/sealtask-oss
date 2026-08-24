export type SerializedStrongBoxWorkerError = {
  message: string
  name?: string
}

export type StrongBoxWorkerRequest =
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

export type StrongBoxWorkerResponse =
  | { type: 'ready' }
  | { type: 'init-error'; error: SerializedStrongBoxWorkerError }
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
      error: SerializedStrongBoxWorkerError
    }
