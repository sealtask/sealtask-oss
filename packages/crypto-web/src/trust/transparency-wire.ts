import { toArrayBuffer } from '../runtime/bytes'

export function encodeUint64(value: number, field: string): Uint8Array {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${field} must be a non-negative safe integer`)
  }
  const view = new DataView(new ArrayBuffer(8))
  view.setBigUint64(0, BigInt(value))
  return new Uint8Array(view.buffer)
}

export function encodeUint32(value: number, field: string): Uint8Array {
  if (!Number.isSafeInteger(value) || value < 0 || value > 0xffffffff) {
    throw new Error(`${field} must be a non-negative 32-bit integer`)
  }
  const view = new DataView(new ArrayBuffer(4))
  view.setUint32(0, value)
  return new Uint8Array(view.buffer)
}

export function concatBytes(...parts: readonly Uint8Array[]): Uint8Array {
  const output = new Uint8Array(parts.reduce((total, part) => total + part.length, 0))
  let offset = 0
  for (const part of parts) {
    output.set(part, offset)
    offset += part.length
  }
  return output
}

export async function sha256(
  input: Uint8Array,
  unavailableMessage: string,
): Promise<Uint8Array> {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error(unavailableMessage)
  }
  const digest = await subtle.digest('SHA-256', toArrayBuffer(input))
  return new Uint8Array(digest)
}
