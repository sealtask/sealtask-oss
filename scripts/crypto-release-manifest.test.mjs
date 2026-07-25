import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const scriptPath = fileURLToPath(
  new URL('./crypto-release-manifest.mjs', import.meta.url),
)
const canonicalWasmPath = fileURLToPath(
  new URL(
    '../artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
    import.meta.url,
  ),
)

function createDist(t) {
  const distDir = mkdtempSync(path.join(os.tmpdir(), 'sealtask-wasm-dist-'))
  const assetDir = path.join(distDir, '_astro')
  mkdirSync(assetDir)
  t.after(() => rmSync(distDir, { force: true, recursive: true }))
  return { assetDir, distDir }
}

function copyCanonicalWasm(assetDir, fileName = 'strong_box_wasm_bg-testhash.wasm') {
  const assetPath = path.join(assetDir, fileName)
  copyFileSync(canonicalWasmPath, assetPath)
  return assetPath
}

function verifyDist(distDir) {
  return spawnSync(
    process.execPath,
    [scriptPath, 'verify-wasm-dist', '--dist', distDir],
    { encoding: 'utf8' },
  )
}

test('verify-wasm-dist accepts one hashed canonical StrongBox WASM', (t) => {
  const { assetDir, distDir } = createDist(t)
  copyCanonicalWasm(assetDir)

  const result = verifyDist(distDir)

  assert.equal(result.status, 0, result.stderr)
  assert.match(result.stdout, /Verified emitted StrongBox WASM/)
})

test('verify-wasm-dist rejects zero or multiple StrongBox WASM files', async (t) => {
  await t.test('zero files', (t) => {
    const { distDir } = createDist(t)

    const result = verifyDist(distDir)

    assert.equal(result.status, 1)
    assert.match(result.stderr, /found 0/)
  })

  await t.test('multiple files', (t) => {
    const { assetDir, distDir } = createDist(t)
    copyCanonicalWasm(assetDir, 'strong_box_wasm_bg-first.wasm')
    copyCanonicalWasm(assetDir, 'strong_box_wasm_bg-second.wasm')

    const result = verifyDist(distDir)

    assert.equal(result.status, 1)
    assert.match(result.stderr, /found 2/)
  })
})

test('verify-wasm-dist rejects modified canonical bytes', (t) => {
  const { assetDir, distDir } = createDist(t)
  const assetPath = copyCanonicalWasm(assetDir)
  writeFileSync(assetPath, new Uint8Array([0]))

  const result = verifyDist(distDir)

  assert.equal(result.status, 1)
  assert.match(result.stderr, /digest .* does not match/)
})

test('verify-wasm-dist requires Vite-hashed filenames', (t) => {
  const { assetDir, distDir } = createDist(t)
  copyCanonicalWasm(assetDir, 'strong_box_wasm_bg.wasm')

  const result = verifyDist(distDir)

  assert.equal(result.status, 1)
  assert.match(result.stderr, /found 0/)
})

test('verify-wasm-dist rejects matching non-regular entries', (t) => {
  const { assetDir, distDir } = createDist(t)
  symlinkSync(
    canonicalWasmPath,
    path.join(assetDir, 'strong_box_wasm_bg-symlink.wasm'),
  )

  const result = verifyDist(distDir)

  assert.equal(result.status, 1)
  assert.match(result.stderr, /not a regular file/)
})
