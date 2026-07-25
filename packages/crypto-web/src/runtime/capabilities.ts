import { constantTimeEquals } from './bytes'
import { KEY_SIZE_BYTES } from './constants'
import type { StrongBoxBridge } from './strong-box-types'

export type CryptoGateIssue =
  | { type: 'insecure-context' }
  | { type: 'missing-subtle' }
  | { type: 'missing-wasm-reference-types' }

export type BrowserCryptoScope = {
  isSecureContext?: boolean
  crypto?: Crypto
  WebAssembly?: Pick<typeof WebAssembly, 'validate'>
}

// Minimal module with a single externref parameter. OPAQUE login depends on
// wasm-bindgen reference types, and older Android WebViews reject this feature.
const WASM_REFERENCE_TYPES_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x05, 0x01, 0x60, 0x01, 0x6f, 0x00,
  0x03, 0x02, 0x01, 0x00,
  0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
])
const STRONG_BOX_HEALTH_CONTEXT = new TextEncoder().encode(
  'worklist.crypto.local_healthcheck',
)
const STRONG_BOX_HEALTH_PLAINTEXT = new TextEncoder().encode(
  'local_crypto_ready',
)
const STRONG_BOX_HEALTH_KEY = Uint8Array.from(
  { length: KEY_SIZE_BYTES },
  (_, index) => (index * 29 + 11) & 0xff,
)

export function detectCryptoGateIssue(
  scope: BrowserCryptoScope = globalThis,
): CryptoGateIssue | null {
  const isSecure =
    typeof scope.isSecureContext === 'boolean'
      ? scope.isSecureContext
      : Boolean(scope.isSecureContext)

  if (!isSecure) {
    return { type: 'insecure-context' }
  }
  if (!scope.crypto || !scope.crypto.subtle) {
    return { type: 'missing-subtle' }
  }
  if (!supportsWasmReferenceTypes(scope.WebAssembly)) {
    return { type: 'missing-wasm-reference-types' }
  }
  return null
}

export function supportsWasmReferenceTypes(
  wasm: BrowserCryptoScope['WebAssembly'] = globalThis.WebAssembly,
): boolean {
  if (!wasm || typeof wasm.validate !== 'function') {
    return false
  }
  try {
    return wasm.validate(WASM_REFERENCE_TYPES_PROBE)
  } catch {
    return false
  }
}

export async function verifyStrongBoxRoundTrip(
  bridge: StrongBoxBridge,
): Promise<void> {
  const ciphertext = await bridge.encrypt({
    key: STRONG_BOX_HEALTH_KEY.slice(),
    context: STRONG_BOX_HEALTH_CONTEXT.slice(),
    plaintext: STRONG_BOX_HEALTH_PLAINTEXT.slice(),
  })
  const decrypted = await bridge.decrypt({
    key: STRONG_BOX_HEALTH_KEY.slice(),
    context: STRONG_BOX_HEALTH_CONTEXT.slice(),
    ciphertext,
  })
  if (!constantTimeEquals(decrypted, STRONG_BOX_HEALTH_PLAINTEXT)) {
    throw new Error('StrongBox local round trip failed')
  }
}
