import { toArrayBuffer } from './bytes'

export async function hmacSha256(
  key: Uint8Array,
  message: Uint8Array,
  unavailableMessage = 'WebCrypto HMAC-SHA256 is unavailable',
): Promise<Uint8Array> {
  const subtle = globalThis.crypto?.subtle
  if (!subtle) {
    throw new Error(unavailableMessage)
  }
  const cryptoKey = await subtle.importKey(
    'raw',
    toArrayBuffer(key),
    {
      name: 'HMAC',
      hash: 'SHA-256',
    },
    false,
    ['sign'],
  )
  return new Uint8Array(await subtle.sign('HMAC', cryptoKey, toArrayBuffer(message)))
}
