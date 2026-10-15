import { generateKeyPairSync, sign } from 'node:crypto'
import { describe, expect, it, vi } from 'vitest'
import { decodeBase64, encodeBase64 } from '../runtime/base64'
import { hashTransparencyNode } from './transparency-proofs'
import {
  fetchBoundedWitnessText, verifyMonitorExtension, verifyWitnessCoverage, verifyWitnessNote,
  witnessKeyId, WitnessPendingError, type WitnessIdentity,
} from './transparency-witness'

const keys = generateKeyPairSync('ed25519')
const identity: WitnessIdentity = {
  name: 'witness.example/key1', logOrigin: 'sealtask.com/transparency/v1',
  publicKey: decodeBase64(keys.publicKey.export({ format: 'jwk' }).x!),
}
const root = new Uint8Array(32).fill(17)
const timestamp = 1_790_358_000

// Construct bytes separately from the production encoder and use OpenSSL via
// node:crypto to cross-check the noble/browser verifier.
async function note(size = 2, hash: Uint8Array = root, time = timestamp): Promise<string> {
  const body = `${identity.logOrigin}\n${size}\n${Buffer.from(hash).toString('base64')}\n`
  const signature = sign(null, Buffer.from(`cosignature/v1\ntime ${time}\n${body}`), keys.privateKey)
  const payload = Buffer.alloc(76)
  payload.set(await witnessKeyId(identity))
  payload.writeBigUInt64BE(BigInt(time), 4)
  payload.set(signature, 12)
  return `${body}\n— ${identity.name} ${payload.toString('base64')}\n`
}

describe('timestamped witness checkpoints', () => {
  it('verifies independently encoded and signed C2SP bytes', async () => {
    await expect(verifyWitnessNote(await note(), identity, { nowSeconds: timestamp })).resolves.toEqual({ size: 2, hash: root, timestamp })
  })
  it.each(['root', 'origin', 'size', 'signature', 'unknown-key', 'expired', 'future', 'framing'])('rejects %s before trusting the checkpoint', async (attack) => {
    let value = await note()
    if (attack === 'root') value = value.replace(Buffer.from(root).toString('base64'), Buffer.alloc(32, 8).toString('base64'))
    if (attack === 'origin') value = value.replace(identity.logOrigin, 'another.example/log')
    if (attack === 'size') value = value.replace('\n2\n', '\n3\n')
    if (attack === 'signature') {
      value = value.replace(/([A-Za-z0-9+/]+={0,2})\n$/, (_, encoded: string) => {
        const payload = Buffer.from(encoded, 'base64')
        // Flip a signature bit; replacing a fixed suffix can leave it unchanged.
        payload[12] ^= 1
        return `${payload.toString('base64')}\n`
      })
    }
    if (attack === 'unknown-key') value = value.replace(identity.name, 'unknown.example/key')
    if (attack === 'expired') value = await note(2, root, timestamp - 601)
    if (attack === 'future') value = await note(2, root, timestamp + 61)
    if (attack === 'framing') value = value.replace('\n2\n', '\n02\n')
    await expect(verifyWitnessNote(value, identity, { nowSeconds: timestamp })).rejects.toThrow()
  })
  it('rejects oversized notes', async () => {
    await expect(verifyWitnessNote('x'.repeat(16_385), identity)).rejects.toThrow(/Malformed/)
  })
})

describe('witness continuity', () => {
  const left = new Uint8Array(32).fill(1)
  const right = new Uint8Array(32).fill(2)
  it('verifies an extension and rejects forks and rollbacks', async () => {
    const hash = await hashTransparencyNode(left, right)
    const response = { logRoot: { size: 2, hash: encodeBase64(hash) }, consistencyProof: { fromSize: 1, hashes: [encodeBase64(left), encodeBase64(right)] } }
    await expect(verifyMonitorExtension({ size: 1, hash: left }, response)).resolves.toEqual({ size: 2, hash })
    await expect(verifyMonitorExtension({ size: 1, hash: right }, response)).rejects.toThrow(/does not extend/)
    await expect(verifyMonitorExtension({ size: 3, hash }, response)).rejects.toThrow(/bounds/)
    await expect(verifyMonitorExtension({ size: 2, hash: left }, { logRoot: response.logRoot, consistencyProof: { fromSize: 2, hashes: [] } })).rejects.toThrow(/conflicts/)
  })
  it('requires identity coverage and connects a newer invitation head to an older witness', async () => {
    const now = Math.floor(Date.now() / 1000)
    const hash = await hashTransparencyNode(left, right)
    const fetchMonitor = vi.fn(async (fromSize: number) => ({
      logRoot: { size: 2, hash: encodeBase64(hash) },
      consistencyProof: { fromSize, hashes: fromSize === 1 ? [encodeBase64(left), encodeBase64(right)] : [] },
    }))
    const witness = { ...identity, fetchCheckpoint: async () => note(1, left, now), fetchMonitor }
    await verifyWitnessCoverage(witness, { size: 2, hash }, 0)
    expect(fetchMonitor).toHaveBeenCalledTimes(2)
    await expect(verifyWitnessCoverage(witness, { size: 2, hash }, 1)).rejects.toBeInstanceOf(WitnessPendingError)
  })
  it('connects a newer witness to an older invitation without accepting a split view', async () => {
    const now = Math.floor(Date.now() / 1000)
    const hash = await hashTransparencyNode(left, right)
    const witness = { ...identity, fetchCheckpoint: async () => note(2, hash, now), fetchMonitor: async (fromSize: number) => ({
      logRoot: { size: 2, hash: encodeBase64(hash) },
      consistencyProof: { fromSize, hashes: fromSize === 1 ? [encodeBase64(left), encodeBase64(right)] : [] },
    }) }
    await verifyWitnessCoverage(witness, { size: 1, hash: left }, 0)
    await expect(verifyWitnessCoverage(witness, { size: 1, hash: right }, 0)).rejects.toThrow(/does not extend/)
  })
  it('rejects a same-size conflicting witness without consulting the server', async () => {
    const witness = { ...identity, fetchCheckpoint: async () => note(2, root, Math.floor(Date.now() / 1000)), fetchMonitor: vi.fn() }
    await expect(verifyWitnessCoverage(witness, { size: 2, hash: right }, 1)).rejects.toThrow(/conflict/)
    expect(witness.fetchMonitor).not.toHaveBeenCalled()
  })
})

describe('bounded public witness transport', () => {
  it('uses anonymous no-cache requests and enforces a streamed size cap', async () => {
    const mock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('x'.repeat(16_385)))
    try {
      await expect(fetchBoundedWitnessText('https://witness.example/checkpoint')).rejects.toThrow(/size limit/)
      expect(mock).toHaveBeenCalledWith('https://witness.example/checkpoint', expect.objectContaining({ credentials: 'omit', redirect: 'error', cache: 'no-store', signal: expect.any(AbortSignal) }))
    } finally { mock.mockRestore() }
  })
  it.each([429, 503])('rejects HTTP %s', async (status) => {
    const mock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status }))
    try { await expect(fetchBoundedWitnessText('https://witness.example/checkpoint')).rejects.toThrow(/unavailable/) } finally { mock.mockRestore() }
  })
})
