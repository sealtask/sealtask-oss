/// <reference path="./argon2-browser.d.ts" />

import argon2 from 'argon2-browser'
import defaultWasmUrl from 'argon2-browser/dist/argon2.wasm?url'

import {
  DEFAULT_ARGON2_PARAMS,
  KEY_SIZE_BYTES,
  MIN_SALT_BYTES,
  type Argon2Params,
} from './constants'

export type Argon2WasmLoader = () => Promise<Uint8Array>

export type Argon2RuntimeOptions = {
  defaultParams?: Argon2Params
  loadWasmBinary?: Argon2WasmLoader
  wasmUrl?: string
}

type Argon2Global = typeof globalThis & {
  argon2WasmPath?: string
  loadArgon2WasmBinary?: Argon2WasmLoader
}

let configuredRuntime: Argon2RuntimeOptions | null = null
let derivationStarted = false

export function configureArgon2Runtime(options: Argon2RuntimeOptions) {
  if (derivationStarted) {
    throw new Error('Argon2 runtime must be configured before the first derivation')
  }
  configuredRuntime = { ...options }
}

function createUrlLoader(wasmUrl: string): Argon2WasmLoader {
  let wasmPromise: Promise<Uint8Array> | null = null
  return () => {
    if (!wasmPromise) {
      wasmPromise = fetch(wasmUrl).then(async (response) => {
        if (!response.ok) {
          throw new Error(`Failed to load Argon2 WASM from ${wasmUrl}`)
        }
        return new Uint8Array(await response.arrayBuffer())
      })
    }
    return wasmPromise
  }
}

function installArgon2Runtime() {
  const scope = globalThis as Argon2Global
  const wasmUrl = configuredRuntime?.wasmUrl ?? defaultWasmUrl
  const loader = configuredRuntime?.loadWasmBinary ?? createUrlLoader(wasmUrl)
  scope.argon2WasmPath = wasmUrl
  scope.loadArgon2WasmBinary = loader
}

const { hash, ArgonType } = argon2

export async function deriveKeyFromPassword(
  password: string,
  salt: Uint8Array,
  params?: Argon2Params,
): Promise<Uint8Array> {
  if (salt.byteLength < MIN_SALT_BYTES) {
    throw new Error(`Salt must be at least ${MIN_SALT_BYTES} bytes`)
  }

  if (!derivationStarted) {
    installArgon2Runtime()
    derivationStarted = true
  }

  const selectedParams =
    params ?? configuredRuntime?.defaultParams ?? DEFAULT_ARGON2_PARAMS
  const result = await hash({
    pass: password,
    salt,
    hashLen: KEY_SIZE_BYTES,
    type: ArgonType.Argon2id,
    time: selectedParams.iterations,
    mem: selectedParams.memoryKiB,
    parallelism: selectedParams.parallelism,
    raw: true,
  })

  return new Uint8Array(result.hash)
}
