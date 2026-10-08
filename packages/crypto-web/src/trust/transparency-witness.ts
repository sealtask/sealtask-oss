import { verifyAsync } from '@noble/ed25519'
import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { constantTimeEquals } from '../runtime/bytes'
import type { TransparencyCheckpoint } from './transparency'
import { verifyConsistencyProof } from './transparency-proofs'
import { concatBytes, sha256 } from './transparency-wire'

const encoder = new TextEncoder()
export const WITNESS_MAX_AGE_SECONDS = 600
export const WITNESS_MAX_RESPONSE_BYTES = 16_384

export type WitnessIdentity = {
  name: string
  publicKey: Uint8Array
  logOrigin: string
}

export type TransparencyMonitorResponse = {
  logRoot: { size: number; hash?: string | null }
  consistencyProof: { fromSize: number; hashes: string[] }
}

export type TransparencyWitness = WitnessIdentity & {
  fetchCheckpoint: () => Promise<string>
  fetchMonitor: (fromSize: number) => Promise<TransparencyMonitorResponse>
}

export class WitnessPendingError extends Error {
  constructor() {
    super('Secure invitation verification is still being prepared. Please try again shortly.')
    this.name = 'WitnessPendingError'
  }
}

export function validateWitnessIdentity(identity: WitnessIdentity): void {
  if (!/^[^\s+\x00-\x1f\x7f]+$/u.test(identity.name)
    || !/^[^\s+\x00-\x1f\x7f]+$/u.test(identity.logOrigin)
    || identity.publicKey.length !== 32) {
    throw new Error('Invalid transparency witness identity configuration.')
  }
}

export function checkpointBody(origin: string, checkpoint: TransparencyCheckpoint): string {
  requireCheckpoint(checkpoint)
  return `${origin}\n${checkpoint.size}\n${encodeBase64(checkpoint.hash)}=\n`
}

export function cosignatureMessage(body: string, timestamp: number): Uint8Array {
  return encoder.encode(`cosignature/v1\ntime ${timestamp}\n${body}`)
}

export async function witnessKeyId(identity: WitnessIdentity): Promise<Uint8Array> {
  validateWitnessIdentity(identity)
  return (await sha256(concatBytes(
    encoder.encode(`${identity.name}\n`), new Uint8Array([4]), identity.publicKey,
  ), 'WebCrypto digest is required for witness verification')).slice(0, 4)
}

/** C2SP tlog-cosignature v1.0.1; a deliberately strict three-line checkpoint profile. */
export async function verifyWitnessNote(
  note: string,
  identity: WitnessIdentity,
  options: { nowSeconds?: number; maxAgeSeconds?: number } = {},
): Promise<TransparencyCheckpoint & { timestamp: number }> {
  validateWitnessIdentity(identity)
  if (typeof note !== 'string' || note.length > WITNESS_MAX_RESPONSE_BYTES
    || /[\x00-\x09\x0b-\x1f\x7f]/u.test(note)) {
    throw new Error('Malformed witness checkpoint.')
  }
  const lines = note.split('\n')
  const [origin, sizeText, hashText, separator] = lines
  const size = Number(sizeText)
  if (origin !== identity.logOrigin || !/^[1-9]\d*$/.test(sizeText)
    || !Number.isSafeInteger(size) || separator !== '' || lines.at(-1) !== '') {
    throw new Error('Witness checkpoint origin or framing is invalid.')
  }
  const hash = canonicalBase64(hashText, 32)
  const body = `${origin}\n${sizeText}\n${hashText}\n`
  const expectedId = await witnessKeyId(identity)
  const signatures = lines.slice(4, -1)
  if (signatures.length < 1 || signatures.length > 16) {
    throw new Error('Invalid witness signature count.')
  }
  let acceptedTimestamp: number | undefined
  const now = options.nowSeconds ?? Math.floor(Date.now() / 1000)
  const maxAge = options.maxAgeSeconds ?? WITNESS_MAX_AGE_SECONDS
  for (const line of signatures) {
    const match = /^— ([^\s+]+) ([A-Za-z0-9+/]+={0,2})$/u.exec(line)
    if (!match) throw new Error('Malformed witness signature line.')
    const bytes = decodeBase64(match[2])
    if (bytes.length < 4 || encodeBase64(bytes) !== match[2].replace(/=+$/, '')) {
      throw new Error('Malformed witness signature encoding.')
    }
    if (match[1] !== identity.name || !constantTimeEquals(bytes.slice(0, 4), expectedId)) continue
    if (bytes.length !== 76) throw new Error('Invalid witness cosignature length.')
    const timestamp = Number(new DataView(bytes.buffer, bytes.byteOffset + 4, 8).getBigUint64(0))
    if (!Number.isSafeInteger(timestamp) || timestamp <= 0 || timestamp > now + 60) {
      throw new Error('Witness checkpoint has an invalid timestamp.')
    }
    if (!await verifyAsync(bytes.slice(12), cosignatureMessage(body, timestamp), identity.publicKey, { zip215: false })) {
      throw new Error('Witness checkpoint signature is invalid.')
    }
    if (now - timestamp > maxAge) throw new WitnessPendingError()
    acceptedTimestamp = timestamp
  }
  if (acceptedTimestamp === undefined) throw new Error('No trusted witness signature.')
  return { size, hash, timestamp: acceptedTimestamp }
}

/** Worklist's existing monitor proof encoding includes the prefix subtree hash. */
export async function verifyMonitorExtension(
  previous: TransparencyCheckpoint,
  response: TransparencyMonitorResponse,
): Promise<TransparencyCheckpoint> {
  requireCheckpoint(previous)
  const next = { size: response.logRoot.size, hash: decodeBase64(response.logRoot.hash ?? '') }
  requireCheckpoint(next)
  const proof = response.consistencyProof
  if (proof.fromSize !== previous.size || next.size < previous.size
    || !Array.isArray(proof.hashes) || proof.hashes.length > 64) {
    throw new Error('Witness continuity proof has invalid bounds.')
  }
  if (next.size === previous.size) {
    if (proof.hashes.length !== 0 || !constantTimeEquals(previous.hash, next.hash)) {
      throw new Error('Witness continuity proof conflicts with the previous checkpoint.')
    }
  } else {
    const reconstructed = await verifyConsistencyProof(previous.size, next.size, proof.hashes.map((hash) => decodeBase64(hash)))
    if (!constantTimeEquals(reconstructed.prefixRoot, previous.hash)
      || !constantTimeEquals(reconstructed.fullRoot, next.hash)) {
      throw new Error('Witness continuity proof does not extend the previous checkpoint.')
    }
  }
  return next
}

export async function verifyWitnessCoverage(
  witness: TransparencyWitness,
  checkpoint: TransparencyCheckpoint,
  firstIdentityLeafIndex: number,
): Promise<void> {
  const note = await witness.fetchCheckpoint().catch(() => { throw new WitnessPendingError() })
  const signed = await verifyWitnessNote(note, witness)
  if (signed.size <= firstIdentityLeafIndex) throw new WitnessPendingError()
  if (signed.size === checkpoint.size) {
    if (!constantTimeEquals(signed.hash, checkpoint.hash)) throw new Error('Witness and invitation roots conflict.')
    return
  }
  // Prove both roots are prefixes of ONE tree. The public monitor only serves
  // the latest head; retry a bounded number of times if an append races reads.
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const [left, right] = await Promise.all([
      witness.fetchMonitor(signed.size).then((response) => verifyMonitorExtension(signed, response)),
      witness.fetchMonitor(checkpoint.size).then((response) => verifyMonitorExtension(checkpoint, response)),
    ])
    if (left.size === right.size) {
      if (!constantTimeEquals(left.hash, right.hash)) throw new Error('Witness and invitation histories conflict.')
      return
    }
  }
  throw new WitnessPendingError()
}

export async function fetchBoundedWitnessText(url: string): Promise<string> {
  const response = await fetch(url, {
    signal: AbortSignal.timeout(8_000), redirect: 'error', credentials: 'omit', cache: 'no-store',
  })
  if (!response.ok || !response.body) throw new Error('Transparency verification service is unavailable.')
  const reader = response.body.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      length += value.byteLength
      if (length > WITNESS_MAX_RESPONSE_BYTES) throw new Error('Transparency response exceeds its size limit.')
      chunks.push(value)
    }
    return new TextDecoder('utf-8', { fatal: true }).decode(concatBytes(...chunks))
  } finally {
    await reader.cancel()
  }
}

function canonicalBase64(value: string, length: number): Uint8Array {
  if (typeof value !== 'string' || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) throw new Error('Invalid checkpoint hash encoding.')
  const bytes = decodeBase64(value)
  if (bytes.length !== length || `${encodeBase64(bytes)}=` !== value) throw new Error('Noncanonical checkpoint hash.')
  return bytes
}

function requireCheckpoint(checkpoint: TransparencyCheckpoint): void {
  if (!Number.isSafeInteger(checkpoint.size) || checkpoint.size <= 0 || checkpoint.hash.length !== 32) {
    throw new Error('Invalid witness checkpoint bounds.')
  }
}
