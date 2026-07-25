#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const ossRoot = path.resolve(scriptDir, '..')
const canonicalWasmPath = path.join(
  ossRoot,
  'artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
)
const canonicalWasmManifestPath = path.join(
  ossRoot,
  'artifacts/strong-box-wasm/build-manifest.json',
)
const cryptoPackagePath = path.join(ossRoot, 'packages/crypto-web/package.json')
const rustToolchainPath = path.join(ossRoot, 'rust-toolchain.toml')
const commitPattern = /^[0-9a-f]{40}$/
const sha256Pattern = /^[0-9a-f]{64}$/
const publicRepository = 'https://github.com/sealtask/sealtask-oss'

function fail(message) {
  throw new Error(message)
}

function readJson(filePath, label) {
  let value
  try {
    value = JSON.parse(readFileSync(filePath, 'utf8'))
  } catch (error) {
    fail(`Unable to read ${label} at ${filePath}: ${error}`)
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    fail(`${label} must contain a JSON object`)
  }
  return value
}

function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

function fileDigest(filePath) {
  return sha256Bytes(readFileSync(filePath))
}

function commandOutput(command, args, cwd = ossRoot) {
  return execFileSync(command, args, {
    cwd,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

function requireCommit(value, label) {
  const normalized = value?.trim().toLowerCase()
  if (!normalized || !commitPattern.test(normalized) || /^0+$/.test(normalized)) {
    fail(`${label} must be a non-zero 40-character lowercase Git commit`)
  }
  return normalized
}

function resolvePublicSubtreeCommit(explicitCommit) {
  const supplied =
    explicitCommit ??
    process.env.SEALTASK_OSS_SUBTREE_COMMIT ??
    process.env.VITE_OSS_SUBTREE_COMMIT
  if (supplied) {
    return requireCommit(supplied, 'public subtree commit')
  }

  let repositoryRoot
  try {
    repositoryRoot = path.resolve(
      commandOutput('git', ['rev-parse', '--show-toplevel']),
    )
  } catch {
    fail(
      'Public subtree commit is unavailable; set SEALTASK_OSS_SUBTREE_COMMIT for builds without Git metadata',
    )
  }

  if (repositoryRoot === ossRoot) {
    const status = commandOutput(
      'git',
      ['status', '--porcelain=v1', '--untracked-files=all', '--', '.'],
      repositoryRoot,
    )
    if (status) {
      fail('Public repository is dirty; no exact source commit can be recorded')
    }
    return requireCommit(
      commandOutput('git', ['rev-parse', 'HEAD'], repositoryRoot),
      'public repository commit',
    )
  }

  if (path.resolve(repositoryRoot, 'oss') !== ossRoot) {
    fail(`Git repository at ${repositoryRoot} does not contain the expected oss/ subtree`)
  }

  if (
    commandOutput(
      'git',
      ['rev-parse', '--is-shallow-repository'],
      repositoryRoot,
    ) === 'true'
  ) {
    fail(
      'Cannot derive public subtree provenance from a shallow clone; fetch full history or set an explicitly resolved SEALTASK_OSS_SUBTREE_COMMIT',
    )
  }

  const status = commandOutput(
    'git',
    ['status', '--porcelain=v1', '--untracked-files=all', '--', 'oss'],
    repositoryRoot,
  )
  if (status) {
    fail('Private OSS subtree is dirty; no exact public source commit can be recorded')
  }

  return requireCommit(
    commandOutput(
      'git',
      ['subtree', 'split', '--prefix=oss', 'HEAD'],
      repositoryRoot,
    ),
    'public subtree split commit',
  )
}

function resolveBunVersion() {
  const supplied = process.env.SEALTASK_BUN_VERSION?.trim()
  if (supplied) {
    return supplied
  }
  try {
    return commandOutput('bun', ['--version'])
  } catch {
    fail('Bun is required to record the browser build toolchain version')
  }
}

function pinnedRustToolchain() {
  const source = readFileSync(rustToolchainPath, 'utf8')
  const match = source.match(/^\s*channel\s*=\s*"([^"]+)"\s*$/m)
  if (!match?.[1]) {
    fail(`${rustToolchainPath} does not pin a Rust channel`)
  }
  return match[1]
}

function lockfileEntry(label, filePath, relativePath, source) {
  if (!statSync(filePath).isFile()) {
    fail(`${label} is not a file: ${filePath}`)
  }
  return {
    label,
    path: relativePath,
    source,
    sha256: fileDigest(filePath),
  }
}

function canonicalWasmIdentity() {
  const wasmBuild = readJson(
    canonicalWasmManifestPath,
    'canonical StrongBox WASM manifest',
  )
  const ossCargoLock = lockfileEntry(
    'oss-cargo',
    path.join(ossRoot, 'Cargo.lock'),
    'Cargo.lock',
    'public-oss',
  )
  const wasmBytes = readFileSync(canonicalWasmPath)
  const wasmSha256 = sha256Bytes(wasmBytes)
  const expectedWasmSha256 = wasmBuild.artifact?.sha256

  if (
    wasmBuild.schemaVersion !== 1 ||
    wasmBuild.artifact?.path !==
      'artifacts/strong-box-wasm/strong_box_wasm_bg.wasm'
  ) {
    fail('Canonical StrongBox WASM build manifest identity is invalid')
  }
  if (
    wasmBuild.source?.cargoLock?.path !== 'Cargo.lock' ||
    !sha256Pattern.test(wasmBuild.source?.cargoLock?.sha256 ?? '') ||
    wasmBuild.source.cargoLock.sha256 !== ossCargoLock.sha256
  ) {
    fail(
      'Canonical StrongBox WASM was not built from the current OSS Cargo.lock',
    )
  }
  if (
    !sha256Pattern.test(expectedWasmSha256 ?? '') ||
    expectedWasmSha256 !== wasmSha256
  ) {
    fail(
      'Canonical StrongBox WASM bytes do not match the checked build manifest',
    )
  }
  if (wasmBuild.artifact?.sizeBytes !== wasmBytes.byteLength) {
    fail('Canonical StrongBox WASM size does not match the checked build manifest')
  }

  const rustToolchain = pinnedRustToolchain()
  if (wasmBuild.build?.rustToolchain !== rustToolchain) {
    fail(
      `Rust toolchain mismatch: rust-toolchain.toml pins ${rustToolchain}, canonical WASM records ${String(wasmBuild.build?.rustToolchain)}`,
    )
  }

  return {
    ossCargoLock,
    rustToolchain,
    wasmBuild,
    wasmBytes,
    wasmSha256,
  }
}

function generateManifest(options) {
  const {
    ossCargoLock,
    rustToolchain,
    wasmBuild,
    wasmBytes,
    wasmSha256,
  } = canonicalWasmIdentity()
  const cryptoPackage = readJson(cryptoPackagePath, 'crypto-web package manifest')
  const lockfiles = [
    ossCargoLock,
    lockfileEntry(
      'oss-bun',
      path.join(ossRoot, 'bun.lock'),
      'bun.lock',
      'public-oss',
    ),
  ]
  if (options.frontendLockfile) {
    const frontendLockfile = path.resolve(options.frontendLockfile)
    lockfiles.push(
      lockfileEntry(
        'frontend-bun',
        frontendLockfile,
        options.frontendLockfileLabel ?? 'frontend/bun.lock',
        'application-build',
      ),
    )
  }

  return {
    schemaVersion: 1,
    source: {
      repository: publicRepository,
      publicSubtreeCommit: resolvePublicSubtreeCommit(
        options.publicSubtreeCommit,
      ),
    },
    package: {
      name: cryptoPackage.name,
      version: cryptoPackage.version,
    },
    lockfiles,
    toolchains: {
      bunVersion: resolveBunVersion(),
      rustToolchain,
      rustcVersion: wasmBuild.build?.rustcVersion,
      cargoVersion: wasmBuild.build?.cargoVersion,
      wasmTarget: wasmBuild.build?.target,
      wasmProfile: wasmBuild.build?.profile,
      canonicalPlatform: wasmBuild.build?.canonicalPlatform,
    },
    wasm: {
      canonicalArtifact: 'artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
      sha256: wasmSha256,
      sizeBytes: wasmBytes.byteLength,
    },
  }
}

function validateManifest(
  manifest,
  { requireFrontendLockfile = false } = {},
) {
  if (manifest.schemaVersion !== 1) {
    fail(`Unsupported crypto release manifest schema: ${String(manifest.schemaVersion)}`)
  }
  if (manifest.source?.repository !== publicRepository) {
    fail('Manifest public source repository is invalid')
  }
  requireCommit(
    manifest.source?.publicSubtreeCommit,
    'manifest public subtree commit',
  )
  if (
    manifest.package?.name !== '@sealtask/crypto-web' ||
    typeof manifest.package?.version !== 'string' ||
    !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(manifest.package.version)
  ) {
    fail('Manifest crypto-web package identity is invalid')
  }
  if (!Array.isArray(manifest.lockfiles) || manifest.lockfiles.length < 2) {
    fail('Manifest must contain at least the OSS Cargo and Bun lockfile hashes')
  }
  const lockfilesByLabel = new Map()
  for (const lockfile of manifest.lockfiles) {
    if (
      typeof lockfile?.label !== 'string' ||
      typeof lockfile?.path !== 'string' ||
      lockfile.path.length === 0 ||
      path.isAbsolute(lockfile.path) ||
      lockfile.path.includes('\\') ||
      lockfile.path
        .split('/')
        .some((part) => part === '' || part === '.' || part === '..') ||
      !['public-oss', 'application-build'].includes(lockfile?.source) ||
      !sha256Pattern.test(lockfile?.sha256 ?? '')
    ) {
      fail('Manifest contains an invalid lockfile entry')
    }
    if (lockfilesByLabel.has(lockfile.label)) {
      fail(`Manifest contains duplicate lockfile label ${lockfile.label}`)
    }
    lockfilesByLabel.set(lockfile.label, lockfile)
  }
  for (const [label, source, lockfilePath] of [
    ['oss-cargo', 'public-oss', 'Cargo.lock'],
    ['oss-bun', 'public-oss', 'bun.lock'],
  ]) {
    const lockfile = lockfilesByLabel.get(label)
    if (
      lockfile?.source !== source ||
      lockfile?.path !== lockfilePath
    ) {
      fail(`Manifest ${label} provenance is invalid`)
    }
  }
  const frontendLockfile = lockfilesByLabel.get('frontend-bun')
  if (
    frontendLockfile &&
    (frontendLockfile.source !== 'application-build' ||
      frontendLockfile.path !== 'frontend/bun.lock')
  ) {
    fail('Manifest frontend-bun provenance is invalid')
  }
  if (requireFrontendLockfile && !frontendLockfile) {
    fail('Deployed manifest must bind frontend/bun.lock')
  }
  for (const key of ['bunVersion', 'rustToolchain', 'rustcVersion', 'cargoVersion']) {
    if (
      typeof manifest.toolchains?.[key] !== 'string' ||
      manifest.toolchains[key].length === 0
    ) {
      fail(`Manifest toolchain field ${key} is missing`)
    }
  }
  if (
    manifest.toolchains?.wasmTarget !== 'wasm32-unknown-unknown' ||
    manifest.toolchains?.wasmProfile !== 'wasm-release' ||
    manifest.toolchains?.canonicalPlatform !== 'linux/amd64'
  ) {
    fail('Manifest canonical WASM toolchain target is invalid')
  }
  if (
    manifest.wasm?.canonicalArtifact !==
      'artifacts/strong-box-wasm/strong_box_wasm_bg.wasm' ||
    !sha256Pattern.test(manifest.wasm?.sha256 ?? '') ||
    !Number.isSafeInteger(manifest.wasm?.sizeBytes) ||
    manifest.wasm.sizeBytes <= 0
  ) {
    fail('Manifest WASM identity is invalid')
  }
}

function requireSafeAssetFileName(value) {
  if (
    typeof value !== 'string' ||
    value.length === 0 ||
    !/^[A-Za-z0-9._/-]+$/.test(value) ||
    !value.endsWith('.wasm') ||
    path.isAbsolute(value) ||
    value.includes('\\') ||
    value.split('/').some((part) => part === '' || part === '.' || part === '..')
  ) {
    fail('Manifest WASM asset filename is unsafe')
  }
  return value
}

function requireAssetUrl(manifest, baseUrl) {
  const assetFileName = requireSafeAssetFileName(manifest.wasm.assetFileName)
  const assetPath = manifest.wasm.assetPath
  if (
    typeof assetPath !== 'string' ||
    !assetPath.startsWith('/') ||
    assetPath.startsWith('//') ||
    assetPath.includes('\\') ||
    assetPath.includes('?') ||
    assetPath.includes('#') ||
    !assetPath.endsWith(`/${assetFileName}`)
  ) {
    fail('Served crypto manifest has an invalid WASM asset path')
  }

  const assetUrl = new URL(assetPath, baseUrl)
  if (
    assetUrl.origin !== baseUrl.origin ||
    assetUrl.pathname !== assetPath ||
    assetUrl.search ||
    assetUrl.hash
  ) {
    fail('Served crypto manifest WASM asset must be a normalized same-origin path')
  }
  return assetUrl
}

function requireImmutableWasmHeaders(response) {
  if (!response.headers.get('content-type')?.startsWith('application/wasm')) {
    fail('Served StrongBox asset must use application/wasm')
  }
  const cacheControl = response.headers.get('cache-control')?.toLowerCase() ?? ''
  const directives = new Set(
    cacheControl
      .split(',')
      .map((directive) => directive.trim())
      .filter(Boolean),
  )
  if (
    !directives.has('public') ||
    !directives.has('immutable') ||
    !directives.has('max-age=31536000')
  ) {
    fail(
      'Served StrongBox asset must use public, max-age=31536000, immutable caching',
    )
  }
}

function verifyExpectedCommit(manifest, expectedCommit) {
  if (!expectedCommit) {
    return
  }
  const normalized = requireCommit(expectedCommit, 'expected public subtree commit')
  if (manifest.source.publicSubtreeCommit !== normalized) {
    fail(
      `Served public subtree commit ${manifest.source.publicSubtreeCommit} does not match ${normalized}`,
    )
  }
}

function verifyWasmBytes(manifest, bytes) {
  const digest = sha256Bytes(bytes)
  if (digest !== manifest.wasm.sha256) {
    fail(`WASM asset digest ${digest} does not match manifest ${manifest.wasm.sha256}`)
  }
  if (bytes.byteLength !== manifest.wasm.sizeBytes) {
    fail(
      `WASM asset size ${bytes.byteLength} does not match manifest ${manifest.wasm.sizeBytes}`,
    )
  }
}

function findStrongBoxWasmFiles(distDir) {
  const matches = []
  const directories = [distDir]
  const strongBoxWasmPattern = /^strong_box_wasm_bg-[A-Za-z0-9_-]+\.wasm$/

  while (directories.length > 0) {
    const directory = directories.pop()
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const entryPath = path.join(directory, entry.name)
      if (strongBoxWasmPattern.test(entry.name)) {
        if (!entry.isFile()) {
          fail(`Emitted StrongBox WASM is not a regular file: ${entryPath}`)
        }
        matches.push(entryPath)
      } else if (entry.isDirectory()) {
        directories.push(entryPath)
      }
    }
  }

  return matches.sort()
}

function verifyWasmDist(options) {
  const distDir = path.resolve(options.dist)
  if (!statSync(distDir).isDirectory()) {
    fail(`Distribution path is not a directory: ${distDir}`)
  }

  const wasmFiles = findStrongBoxWasmFiles(distDir)
  if (wasmFiles.length !== 1) {
    fail(
      `Expected exactly one emitted StrongBox WASM in ${distDir}, found ${wasmFiles.length}`,
    )
  }

  const { wasmBytes, wasmSha256 } = canonicalWasmIdentity()
  const emittedWasmPath = wasmFiles[0]
  verifyWasmBytes(
    {
      wasm: {
        sha256: wasmSha256,
        sizeBytes: wasmBytes.byteLength,
      },
    },
    readFileSync(emittedWasmPath),
  )
  const emittedRelativePath = path.relative(distDir, emittedWasmPath)
  process.stdout.write(
    `Verified emitted StrongBox WASM ${emittedRelativePath} (${wasmSha256})\n`,
  )
}

function verifyDist(options) {
  const distDir = path.resolve(options.dist)
  const manifestPath = path.join(distDir, 'crypto-manifest.json')
  const manifest = readJson(manifestPath, 'emitted crypto release manifest')
  validateManifest(manifest, { requireFrontendLockfile: true })
  verifyExpectedCommit(manifest, options.expectedCommit)

  const assetFileName = requireSafeAssetFileName(manifest.wasm.assetFileName)
  if (
    typeof manifest.wasm.assetPath !== 'string' ||
    !manifest.wasm.assetPath.endsWith(`/${assetFileName}`)
  ) {
    fail('Manifest WASM asset path does not identify its emitted asset file')
  }
  const assetPath = path.resolve(distDir, assetFileName)
  if (!assetPath.startsWith(`${distDir}${path.sep}`)) {
    fail('Manifest WASM asset escapes the distribution directory')
  }
  verifyWasmBytes(manifest, readFileSync(assetPath))
  process.stdout.write(
    `Verified emitted crypto manifest and WASM (${manifest.wasm.sha256})\n`,
  )
}

async function verifyUrl(options) {
  const baseUrl = new URL(options.baseUrl)
  const manifestUrl = new URL('/crypto-manifest.json', baseUrl)
  const response = await fetch(manifestUrl, {
    headers: { accept: 'application/json' },
    redirect: 'error',
  })
  if (!response.ok) {
    fail(`Unable to fetch ${manifestUrl}: HTTP ${response.status}`)
  }
  if (!response.headers.get('content-type')?.startsWith('application/json')) {
    fail('Served crypto manifest must use application/json')
  }
  if (!response.headers.get('cache-control')?.includes('no-store')) {
    fail('Served crypto manifest must use Cache-Control: no-store')
  }

  const manifest = await response.json()
  validateManifest(manifest, { requireFrontendLockfile: true })
  verifyExpectedCommit(manifest, options.expectedCommit)
  const wasmUrl = requireAssetUrl(manifest, baseUrl)
  const wasmResponse = await fetch(wasmUrl, { redirect: 'error' })
  if (!wasmResponse.ok) {
    fail(`Unable to fetch ${wasmUrl}: HTTP ${wasmResponse.status}`)
  }
  requireImmutableWasmHeaders(wasmResponse)
  verifyWasmBytes(manifest, new Uint8Array(await wasmResponse.arrayBuffer()))
  process.stdout.write(
    `Verified served crypto manifest and WASM (${manifest.wasm.sha256})\n`,
  )
}

function optionValue(args, name, { required = false } = {}) {
  const index = args.indexOf(name)
  if (index === -1) {
    if (required) {
      fail(`${name} is required`)
    }
    return undefined
  }
  const value = args[index + 1]
  if (!value || value.startsWith('--')) {
    fail(`${name} requires a value`)
  }
  return value
}

async function main() {
  const [command = 'generate', ...args] = process.argv.slice(2)
  switch (command) {
    case 'generate': {
      const output = path.resolve(optionValue(args, '--output', { required: true }))
      const frontendLockfile = optionValue(args, '--frontend-lockfile')
      const manifest = generateManifest({
        publicSubtreeCommit: optionValue(args, '--public-subtree-commit'),
        frontendLockfile,
        frontendLockfileLabel: optionValue(args, '--frontend-lockfile-label'),
      })
      validateManifest(manifest)
      mkdirSync(path.dirname(output), { recursive: true })
      writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8')
      process.stdout.write(`Wrote ${output}\n`)
      return
    }
    case 'verify-dist':
      verifyDist({
        dist: optionValue(args, '--dist', { required: true }),
        expectedCommit: optionValue(args, '--expected-subtree-commit'),
      })
      return
    case 'verify-wasm-dist':
      verifyWasmDist({
        dist: optionValue(args, '--dist', { required: true }),
      })
      return
    case 'verify-url':
      await verifyUrl({
        baseUrl: optionValue(args, '--base-url', { required: true }),
        expectedCommit: optionValue(args, '--expected-subtree-commit'),
      })
      return
    default:
      fail(
        'usage: crypto-release-manifest.mjs {generate|verify-dist|verify-wasm-dist|verify-url} [options]',
      )
  }
}

main().catch((error) => {
  process.stderr.write(`error: ${error instanceof Error ? error.message : String(error)}\n`)
  process.exitCode = 1
})
