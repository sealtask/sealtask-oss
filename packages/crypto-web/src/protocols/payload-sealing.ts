import { encode as cborEncode } from 'cbor-x'

import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  decodeAndValidatePayloadBytes,
  validatePayloadBytes,
  type PayloadKind,
  type PayloadValidationDependencies,
  type ValidatedPayloadEnvelope,
} from './payload-validation'
import { parseSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'
import type { SealedBlobPayload } from './types'

type PayloadEnvelope<Kind extends PayloadKind> = {
  kind: Kind
  version: number
  body: unknown
}

type PayloadCryptographicDomain<Kind extends PayloadKind> = {
  kind: Kind
  context: Uint8Array
}

/**
 * Internal common path for the structured payload protocols. Public protocol
 * modules keep their key names and domain constants visible at the call site.
 */
export async function sealPayloadEnvelope<Kind extends PayloadKind>(
  envelope: PayloadEnvelope<Kind>,
  key: Uint8Array,
  domain: PayloadCryptographicDomain<NoInfer<Kind>>,
  strongBox?: StrongBoxBridge,
  validation?: PayloadValidationDependencies,
): Promise<SealedBlobPayload> {
  const plaintext = toUint8Array(cborEncode(envelope))
  validatePayloadBytes(plaintext, domain.kind, validation)
  const bridge = strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key,
    context: domain.context,
    plaintext,
  })
  return toSealedBlob({ version: SEALED_PAYLOAD_VERSION, ciphertext })
}

export async function openPayloadEnvelope<Kind extends PayloadKind>(
  ciphertext: string,
  key: Uint8Array,
  domain: PayloadCryptographicDomain<Kind>,
  strongBox?: StrongBoxBridge,
  validation?: PayloadValidationDependencies,
): Promise<ValidatedPayloadEnvelope<Kind>> {
  const sealed = parseSealedPayload(ciphertext)
  const bridge = strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key,
    context: domain.context,
    ciphertext: sealed.ciphertext,
  })
  return decodeAndValidatePayloadBytes(plaintext, domain.kind, validation)
}
