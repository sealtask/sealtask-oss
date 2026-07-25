#!/usr/bin/env node

import { readFile, readdir } from 'node:fs/promises'
import {
  dirname,
  extname,
  relative,
  resolve,
  sep,
} from 'node:path'
import { fileURLToPath } from 'node:url'

const packageRoot = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
)
const sourceRoot = resolve(packageRoot, 'src')
const allowedArtifact = resolve(
  packageRoot,
  '../../artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
)
const forbiddenPackagePrefixes = [
  '@/',
  '@sentry/',
  '@tanstack/',
  'i18next',
  'react',
  'react-router',
]
const importPattern =
  /(?:from\s+|import\s*\()\s*['"]([^'"]+)['"]/g
const failures = []

for (const absolutePath of await collectSourceFiles(sourceRoot)) {
  const sourcePath = normalizePath(relative(packageRoot, absolutePath))
  const contents = await readFile(absolutePath, 'utf8')

  if (
    /(?:legacy[-_\s]*cbor|cbor[-_\s]*legacy|decodeLegacyCbor|encodeLegacyCbor)/i.test(
      contents,
    )
  ) {
    failures.push(
      `${sourcePath}: contains private legacy-CBOR compatibility code`,
    )
  }

  for (const match of contents.matchAll(importPattern)) {
    const specifier = match[1]
    if (!specifier) continue

    if (
      forbiddenPackagePrefixes.some(
        (prefix) =>
          specifier === prefix ||
          specifier.startsWith(prefix),
      ) ||
      specifier.includes('/generated/')
    ) {
      failures.push(
        `${sourcePath}: imports private application dependency ${specifier}`,
      )
      continue
    }

    if (!specifier.startsWith('.')) continue

    const resolvedImport = resolve(
      dirname(absolutePath),
      specifier.split('?')[0],
    )
    if (
      isWithin(packageRoot, resolvedImport) ||
      resolvedImport === allowedArtifact
    ) {
      continue
    }
    failures.push(
      `${sourcePath}: relative import escapes the public package (${specifier})`,
    )
  }
}

if (failures.length > 0) {
  console.error('Public crypto package boundary check failed:')
  for (const failure of failures) {
    console.error(`- ${failure}`)
  }
  process.exit(1)
}

console.log('Public crypto package boundary check passed.')

async function collectSourceFiles(directory) {
  const files = []
  for (const entry of await readdir(directory, {
    withFileTypes: true,
  })) {
    const absolutePath = resolve(directory, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await collectSourceFiles(absolutePath)))
    } else if (
      entry.isFile() &&
      ['.ts', '.tsx'].includes(extname(entry.name))
    ) {
      files.push(absolutePath)
    }
  }
  return files
}

function isWithin(parent, candidate) {
  const path = relative(parent, candidate)
  return path === '' || (!path.startsWith('..') && !path.startsWith(sep))
}

function normalizePath(path) {
  return path.split(sep).join('/')
}
