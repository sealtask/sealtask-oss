import { createPrivateKey, createPublicKey, sign } from 'node:crypto'
import { open, readFile, rename, unlink } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { decodeBase64 } from '@sealtask/crypto-web/runtime/base64'
import { constantTimeEquals } from '@sealtask/crypto-web/runtime/bytes'
import type { TransparencyCheckpoint } from '@sealtask/crypto-web/trust/transparency'
import {
  checkpointBody, cosignatureMessage, verifyMonitorExtension, verifyWitnessNote,
  validateWitnessIdentity, witnessKeyId, WITNESS_MAX_RESPONSE_BYTES,
  type TransparencyMonitorResponse, type WitnessIdentity,
} from '@sealtask/crypto-web/trust/transparency-witness'

export type AuditorConfig = {
  name: string
  logOrigin: string
  monitorUrl: string
  seed: { size: number; hash: string }
  stateDirectory: string
  publicDirectory: string
  privateKeyFile: string
}

type AuditStage = 'state' | 'monitor' | 'consistency' | 'persistence'

export class WitnessAuditError extends Error {
  constructor(readonly stage: AuditStage, cause: unknown) {
    super(cause instanceof Error ? cause.message : 'Witness audit failed', { cause })
    this.name = 'WitnessAuditError'
  }
}

/** One invocation under an exclusive OS flock; no database or account credentials. */
export async function auditOnce(
  config: AuditorConfig,
  fetchMonitor: (url: string) => Promise<TransparencyMonitorResponse>,
  options: { initialize?: boolean; nowSeconds?: number } = {},
): Promise<{ size: number; timestamp: number }> {
  const monitor = new URL(config.monitorUrl)
  if (monitor.protocol !== 'https:' || monitor.username || monitor.password || monitor.search || monitor.hash) {
    throw new Error('Monitor must be a fixed HTTPS URL without credentials or query parameters.')
  }
  const privateKey = createPrivateKey(await readFile(config.privateKeyFile))
  if (privateKey.asymmetricKeyType !== 'ed25519') throw new Error('Witness signing key must be Ed25519.')
  const jwk = createPublicKey(privateKey).export({ format: 'jwk' })
  const identity: WitnessIdentity = { name: config.name, logOrigin: config.logOrigin, publicKey: decodeBase64(jwk.x!) }
  validateWitnessIdentity(identity)
  const seed = { size: config.seed.size, hash: decodeBase64(config.seed.hash) }
  checkpointBody(config.logOrigin, seed) // Validate before accepting any state.
  const statePath = join(config.stateDirectory, 'checkpoint')
  const now = options.nowSeconds ?? Math.floor(Date.now() / 1000)
  let previous: TransparencyCheckpoint
  if (options.initialize) {
    // Initialization is explicit and exclusive; it never overwrites existing state.
    const note = await signCheckpoint(identity, seed, now, (message) => sign(null, message, privateKey))
    await atStage('state', () => createDurable(statePath, note))
    previous = seed
  } else {
    // Missing/corrupt state must stop the signer, never silently bootstrap again.
    previous = await atStage('state', async () => {
      const bytes = await readFile(statePath)
      if (bytes.length > WITNESS_MAX_RESPONSE_BYTES) throw new Error('Witness state is oversized.')
      const stored = await verifyWitnessNote(bytes.toString('utf8'), identity, { nowSeconds: now, maxAgeSeconds: Infinity })
      if (stored.timestamp > now) throw new Error('Witness signing clock regressed.')
      if (stored.size < seed.size || (stored.size === seed.size && !constantTimeEquals(stored.hash, seed.hash))) {
        throw new Error('Witness state conflicts with its bootstrap checkpoint.')
      }
      return stored
    })
  }
  monitor.searchParams.set('fromSize', String(previous.size))
  const response = await atStage('monitor', () => fetchMonitor(monitor.href))
  const next = await atStage('consistency', () => verifyMonitorExtension(previous, response))
  const note = await signCheckpoint(identity, next, now, (message) => sign(null, message, privateKey))
  // Persist and fsync the private high-water mark BEFORE making any signature
  // publicly readable. A failure after this write is recoverable on the next run.
  await atStage('persistence', async () => {
    await atomicWrite(statePath, note, 0o600)
    await atomicWrite(join(config.publicDirectory, 'checkpoint'), note, 0o644)
  })
  return { size: next.size, timestamp: now }
}

async function atStage<T>(stage: AuditStage, operation: () => Promise<T>): Promise<T> {
  try { return await operation() } catch (error) { throw new WitnessAuditError(stage, error) }
}

export async function signCheckpoint(
  identity: WitnessIdentity,
  checkpoint: TransparencyCheckpoint,
  timestamp: number,
  signer: (message: Uint8Array) => Uint8Array,
): Promise<string> {
  if (!Number.isSafeInteger(timestamp) || timestamp <= 0) throw new Error('Invalid witness signing timestamp.')
  const body = checkpointBody(identity.logOrigin, checkpoint)
  const bytes = Buffer.alloc(76)
  bytes.set(await witnessKeyId(identity), 0)
  bytes.writeBigUInt64BE(BigInt(timestamp), 4)
  const signature = signer(cosignatureMessage(body, timestamp))
  if (signature.length !== 64) throw new Error('Invalid witness signature length.')
  bytes.set(signature, 12)
  return `${body}\n— ${identity.name} ${bytes.toString('base64')}\n`
}

async function createDurable(path: string, value: string): Promise<void> {
  const file = await open(path, 'wx', 0o600)
  try {
    await file.writeFile(value)
    await file.sync()
  } finally {
    await file.close()
  }
  await syncDirectory(dirname(path))
}

async function atomicWrite(path: string, value: string, mode: number): Promise<void> {
  const temporary = `${path}.${process.pid}.tmp`
  const file = await open(temporary, 'wx', mode)
  try {
    await file.writeFile(value)
    await file.chmod(mode)
    await file.sync()
    await file.close()
    await rename(temporary, path)
    await syncDirectory(dirname(path))
  } finally {
    await file.close().catch(() => undefined)
    await unlink(temporary).catch((error: NodeJS.ErrnoException) => { if (error.code !== 'ENOENT') throw error })
  }
}

async function syncDirectory(path: string): Promise<void> {
  const directory = await open(path, 'r')
  try { await directory.sync() } finally { await directory.close() }
}
