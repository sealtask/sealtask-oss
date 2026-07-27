export type ByteSource = Uint8Array | ArrayBuffer | ArrayBufferLike | ArrayLike<number>

export function toUint8Array(value: ByteSource): Uint8Array {
  if (value instanceof Uint8Array) {
    // `Buffer` subclasses Uint8Array but overrides `slice()` to return a
    // shared-memory view. Use the intrinsic typed-array copy semantics so
    // later zeroization can never mutate caller-owned Buffer storage.
    return Uint8Array.from(value)
  }
  if (isArrayBufferLike(value)) {
    const view = new Uint8Array(value)
    return view.slice()
  }
  return Uint8Array.from(value)
}

export function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength)
  const copy = new Uint8Array(buffer)
  copy.set(bytes)
  return buffer
}

export function zeroBytes(bytes?: Uint8Array | null) {
  bytes?.fill(0)
}

export function constantTimeEquals(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) {
    return false
  }
  let result = 0
  for (let index = 0; index < left.length; index += 1) {
    result |= left[index] ^ right[index]
  }
  return result === 0
}

function isArrayBufferLike(value: ByteSource): value is ArrayBufferLike {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { byteLength?: unknown }).byteLength === 'number' &&
    typeof (value as { length?: unknown }).length !== 'number'
  )
}
