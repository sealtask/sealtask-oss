#!/usr/bin/env node

import {
  lstat,
  readFile,
  readdir,
} from 'node:fs/promises'
import {
  dirname,
  extname,
  relative,
  resolve,
  sep,
} from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptPath = fileURLToPath(import.meta.url)
const defaultPackageRoot = resolve(dirname(scriptPath), '..')
const importableSourceExtensions = new Set([
  '.cjs',
  '.cts',
  '.js',
  '.jsx',
  '.mjs',
  '.mts',
  '.ts',
  '.tsx',
])
const legacyCborPattern =
  /(?:legacy[-_\s]*cbor|cbor[-_\s]*legacy|decodeLegacyCbor|encodeLegacyCbor)/i
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

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  const failures = await checkPackageBoundary(defaultPackageRoot)

  if (failures.length > 0) {
    console.error('Public crypto package boundary check failed:')
    for (const failure of failures) {
      console.error(`- ${failure}`)
    }
    process.exit(1)
  }

  console.log('Public crypto package boundary check passed.')
}

export async function checkPackageBoundary(packageRoot) {
  const resolvedPackageRoot = resolve(packageRoot)
  const allowedArtifact = resolve(
    resolvedPackageRoot,
    '../../artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
  )
  const failures = []
  const packageJson = await readPackageJson(
    resolvedPackageRoot,
    failures,
  )
  const publishedFiles = await collectPublishedFiles(
    resolvedPackageRoot,
    packageJson?.files,
    failures,
  )
  const publicDevelopmentFiles = []
  for (const directory of ['browser-tests', 'test']) {
    publicDevelopmentFiles.push(
      ...(await collectFilesIfPresent(
        resolve(resolvedPackageRoot, directory),
        failures,
      )),
    )
  }
  const filesToScan = new Map()

  for (const absolutePath of publishedFiles) {
    filesToScan.set(absolutePath, true)
  }
  for (const absolutePath of publicDevelopmentFiles) {
    if (!filesToScan.has(absolutePath)) {
      filesToScan.set(absolutePath, false)
    }
  }

  for (const [absolutePath, isPublished] of filesToScan) {
    const sourcePath = normalizePath(
      relative(resolvedPackageRoot, absolutePath),
    )
    const extension = extname(absolutePath).toLowerCase()

    if (legacyCborPattern.test(sourcePath)) {
      failures.push(
        `${sourcePath}: exposes a private legacy-CBOR artifact in the public package tree`,
      )
    }

    const contents = await readFile(absolutePath, 'utf8')
    const legacyCborContent =
      isPublished && sourcePath === 'README.md'
        ? extractMarkdownCode(contents)
        : contents
    if (legacyCborPattern.test(legacyCborContent)) {
      failures.push(
        `${sourcePath}: contains private legacy-CBOR compatibility code or fixture data`,
      )
    }

    if (!importableSourceExtensions.has(extension)) continue

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
        isWithin(resolvedPackageRoot, resolvedImport) ||
        resolvedImport === allowedArtifact
      ) {
        continue
      }
      failures.push(
        `${sourcePath}: relative import escapes the public package (${specifier})`,
      )
    }
  }

  return failures
}

async function readPackageJson(packageRoot, failures) {
  const packageJsonPath = resolve(packageRoot, 'package.json')

  try {
    return JSON.parse(await readFile(packageJsonPath, 'utf8'))
  } catch (error) {
    failures.push(
      `package.json: cannot read published file list (${formatError(error)})`,
    )
    return null
  }
}

async function collectPublishedFiles(
  packageRoot,
  publishedEntries,
  failures,
) {
  if (
    !Array.isArray(publishedEntries) ||
    publishedEntries.length === 0
  ) {
    failures.push(
      'package.json: files must list the public package contents',
    )
    return []
  }

  const files = []
  for (const entry of publishedEntries) {
    if (
      typeof entry !== 'string' ||
      entry.length === 0 ||
      hasGlobSyntax(entry)
    ) {
      failures.push(
        `package.json: unsupported files entry ${JSON.stringify(entry)}; use explicit files or directories`,
      )
      continue
    }

    const absolutePath = resolve(packageRoot, entry)
    if (!isWithin(packageRoot, absolutePath)) {
      failures.push(
        `package.json: files entry escapes the public package (${entry})`,
      )
      continue
    }

    files.push(
      ...(await collectFiles(
        absolutePath,
        packageRoot,
        failures,
      )),
    )
  }
  return files
}

async function collectFilesIfPresent(directory, failures) {
  try {
    return await collectFiles(directory, directory, failures)
  } catch (error) {
    if (error?.code === 'ENOENT') return []
    throw error
  }
}

async function collectFiles(path, boundaryRoot, failures) {
  const metadata = await lstat(path)

  if (metadata.isSymbolicLink()) {
    failures.push(
      `${normalizePath(relative(boundaryRoot, path))}: symbolic links are not allowed in the scanned package boundary`,
    )
    return []
  }
  if (metadata.isFile()) return [path]
  if (!metadata.isDirectory()) {
    failures.push(
      `${normalizePath(relative(boundaryRoot, path))}: unsupported published filesystem entry`,
    )
    return []
  }

  const files = []
  for (const entry of await readdir(path, {
    withFileTypes: true,
  })) {
    const absolutePath = resolve(path, entry.name)
    if (entry.isDirectory()) {
      files.push(
        ...(await collectFiles(
          absolutePath,
          boundaryRoot,
          failures,
        )),
      )
    } else if (entry.isFile()) {
      files.push(absolutePath)
    } else {
      failures.push(
        `${normalizePath(relative(boundaryRoot, absolutePath))}: symbolic links and special files are not allowed in the scanned package boundary`,
      )
    }
  }
  return files
}

function hasGlobSyntax(path) {
  return /[*?[\]{}()!]/.test(path)
}

function formatError(error) {
  return error instanceof Error ? error.message : String(error)
}

function extractMarkdownCode(markdown) {
  const snippets = []
  const fencedCodePattern =
    /^(`{3,}|~{3,})[^\r\n]*\r?\n([\s\S]*?)^\1\s*$/gm
  const indentedCodePattern = /^(?: {4}|\t)(.*)$/gm

  for (const match of markdown.matchAll(fencedCodePattern)) {
    snippets.push(match[2] ?? '')
  }
  for (const match of markdown.matchAll(indentedCodePattern)) {
    snippets.push(match[1] ?? '')
  }
  return snippets.join('\n')
}

function isWithin(parent, candidate) {
  const path = relative(parent, candidate)
  return (
    path === '' ||
    (!path.startsWith('..') && !path.startsWith(sep))
  )
}

function normalizePath(path) {
  return path.split(sep).join('/')
}
