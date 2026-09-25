import { generateKeyPairSync } from 'node:crypto'
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { decodeBase64, encodeBase64 } from '@sealtask/crypto-web/runtime/base64'
import { hashTransparencyNode } from '@sealtask/crypto-web/trust/transparency-proofs'
import { verifyWitnessNote } from '@sealtask/crypto-web/trust/transparency-witness'
import { auditOnce, type AuditorConfig } from './auditor'

const keys = generateKeyPairSync('ed25519')
const left = new Uint8Array(32).fill(1)
const right = new Uint8Array(32).fill(2)
const now = 1_790_358_000
let directory: string
let config: AuditorConfig

beforeEach(async () => {
  directory = await mkdtemp(join(tmpdir(), 'worklist-witness-test-'))
  config = { name: 'independent.example/w1', logOrigin: 'sealtask.com/transparency/v1',
    monitorUrl: 'https://sealtask.com/transparency/logs/monitor', seed: { size: 1, hash: encodeBase64(left) },
    stateDirectory: join(directory, 'private'), publicDirectory: join(directory, 'public'), privateKeyFile: join(directory, 'key.pem') }
  await mkdir(config.stateDirectory)
  await mkdir(config.publicDirectory)
  await writeFile(config.privateKeyFile, keys.privateKey.export({ format: 'pem', type: 'pkcs8' }), { mode: 0o600 })
})
afterEach(async () => { await rm(directory, { recursive: true, force: true }) })

const unchanged = async () => ({ logRoot: { size: 1, hash: encodeBase64(left) }, consistencyProof: { fromSize: 1, hashes: [] } })
async function initialize() { await auditOnce(config, unchanged, { initialize: true, nowSeconds: now }) }

describe('durable standalone witness', () => {
  it('publishes readable public notes while keeping private state private under the service umask', async () => {
    const original = process.umask(0o077)
    try {
      await initialize()
      expect((await stat(join(config.publicDirectory, 'checkpoint'))).mode & 0o777).toBe(0o644)
      expect((await stat(join(config.stateDirectory, 'checkpoint'))).mode & 0o777).toBe(0o600)
    } finally { process.umask(original) }
  })
  it('requires explicit initialization and never overwrites a prior checkpoint', async () => {
    const fetcher = vi.fn(unchanged)
    await expect(auditOnce(config, fetcher)).rejects.toThrow()
    expect(fetcher).not.toHaveBeenCalled()
    await initialize()
    await expect(auditOnce(config, fetcher, { initialize: true, nowSeconds: now })).rejects.toThrow(/EEXIST/)
    expect(fetcher).not.toHaveBeenCalled()
  })
  it('restarts from the durable checkpoint, advances, and emits a browser-verifiable signature', async () => {
    await initialize()
    const root = await hashTransparencyNode(left, right)
    const fetcher = vi.fn(async () => ({ logRoot: { size: 2, hash: encodeBase64(root) }, consistencyProof: { fromSize: 1, hashes: [encodeBase64(left), encodeBase64(right)] } }))
    await auditOnce(config, fetcher, { nowSeconds: now + 5 })
    expect(fetcher).toHaveBeenCalledWith(`${config.monitorUrl}?fromSize=1`)
    const publicNote = await readFile(join(config.publicDirectory, 'checkpoint'), 'utf8')
    expect(await readFile(join(config.stateDirectory, 'checkpoint'), 'utf8')).toBe(publicNote)
    await expect(verifyWitnessNote(publicNote, { ...config, publicKey: decodeBase64(keys.publicKey.export({ format: 'jwk' }).x!) }, { nowSeconds: now + 5 }))
      .resolves.toMatchObject({ size: 2, hash: root })
    const afterRestart = vi.fn(async () => ({ logRoot: { size: 2, hash: encodeBase64(root) }, consistencyProof: { fromSize: 2, hashes: [] } }))
    await auditOnce(config, afterRestart, { nowSeconds: now + 10 })
    expect(afterRestart).toHaveBeenCalledWith(`${config.monitorUrl}?fromSize=2`)
  })
  it.each(['fork', 'rollback', 'bad-proof', 'wrong-base', 'malformed', 'timeout', 'rate-limit'])('keeps durable/public state unchanged after %s', async (attack) => {
    await initialize()
    const before = await readFile(join(config.stateDirectory, 'checkpoint'), 'utf8')
    const fetcher = async () => {
      if (attack === 'timeout') throw new DOMException('timeout', 'TimeoutError')
      if (attack === 'rate-limit') throw new Error('HTTP 429')
      if (attack === 'malformed') return {} as never
      return { logRoot: { size: attack === 'rollback' ? 0 : attack === 'bad-proof' ? 2 : 1, hash: encodeBase64(right) },
        consistencyProof: { fromSize: attack === 'wrong-base' ? 0 : 1, hashes: attack === 'bad-proof' ? [encodeBase64(right)] : [] } }
    }
    await expect(auditOnce(config, fetcher, { nowSeconds: now + 5 })).rejects.toThrow()
    expect(await readFile(join(config.stateDirectory, 'checkpoint'), 'utf8')).toBe(before)
    expect(await readFile(join(config.publicDirectory, 'checkpoint'), 'utf8')).toBe(before)
    // A transient failure does not poison subsequent valid runs.
    await expect(auditOnce(config, unchanged, { nowSeconds: now + 10 })).resolves.toMatchObject({ size: 1 })
  })
  it('fails closed on corrupt state and clock rollback without fetching or publishing', async () => {
    await initialize()
    const fetcher = vi.fn(unchanged)
    await expect(auditOnce(config, fetcher, { nowSeconds: now - 61 })).rejects.toThrow(/timestamp/)
    await writeFile(join(config.stateDirectory, 'checkpoint'), 'corrupt')
    await expect(auditOnce(config, fetcher, { nowSeconds: now })).rejects.toThrow()
    expect(fetcher).not.toHaveBeenCalled()
  })
  it('retains advanced private state if public publication fails and recovers without resetting', async () => {
    await initialize()
    await rm(config.publicDirectory, { recursive: true })
    const root = await hashTransparencyNode(left, right)
    await expect(auditOnce(config, async () => ({ logRoot: { size: 2, hash: encodeBase64(root) }, consistencyProof: { fromSize: 1, hashes: [encodeBase64(left), encodeBase64(right)] } }), { nowSeconds: now + 5 })).rejects.toThrow()
    expect(await readFile(join(config.stateDirectory, 'checkpoint'), 'utf8')).toContain('\n2\n')
    await mkdir(config.publicDirectory)
    const fetcher = vi.fn(async () => ({ logRoot: { size: 2, hash: encodeBase64(root) }, consistencyProof: { fromSize: 2, hashes: [] } }))
    await auditOnce(config, fetcher, { nowSeconds: now + 10 })
    expect(fetcher).toHaveBeenCalledWith(`${config.monitorUrl}?fromSize=2`)
    expect(await readFile(join(config.publicDirectory, 'checkpoint'), 'utf8')).toContain('\n2\n')
  })
})
