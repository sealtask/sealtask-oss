import {
  StrongBoxWorkerClient,
  type StrongBoxWorkerFactory,
} from './strong-box-worker-client'

export type {
  HpkeDecapInput,
  HpkeEncapInput,
  HpkeEncapResult,
  StrongBoxBridge,
  StrongBoxDecryptInput,
  StrongBoxEncryptInput,
} from './strong-box-types'

import type { StrongBoxBridge } from './strong-box-types'

export type StrongBoxBridgeOptions = {
  workerFactory?: StrongBoxWorkerFactory
}

export type StrongBoxRuntimeOptions = {
  bridgeFactory?: () =>
    | StrongBoxBridge
    | Promise<StrongBoxBridge>
}

let bridgePromise: Promise<StrongBoxBridge> | null = null
let configuredBridgeFactory:
  | StrongBoxRuntimeOptions['bridgeFactory']
  | null = null

export function configureStrongBoxRuntime(
  options: StrongBoxRuntimeOptions,
): void {
  if (bridgePromise) {
    throw new Error(
      'StrongBox runtime must be configured before the first bridge request',
    )
  }
  configuredBridgeFactory = options.bridgeFactory ?? null
}

export async function getStrongBoxBridge(): Promise<StrongBoxBridge> {
  if (!bridgePromise) {
    const pendingBridge = Promise.resolve().then(() =>
      configuredBridgeFactory
        ? configuredBridgeFactory()
        : createStrongBoxBridge(),
    )
    bridgePromise = pendingBridge
    void pendingBridge.catch(() => {
      if (bridgePromise === pendingBridge) {
        bridgePromise = null
      }
    })
  }
  return bridgePromise
}

export async function createStrongBoxBridge(
  options: StrongBoxBridgeOptions = {},
): Promise<StrongBoxBridge> {
  assertWorkerBridgeSupport(Boolean(options.workerFactory))
  return StrongBoxWorkerClient.create(options.workerFactory)
}

function assertWorkerBridgeSupport(hasInjectedWorkerFactory: boolean) {
  const hasWorker = hasInjectedWorkerFactory || typeof Worker !== 'undefined'
  const hasWasm = typeof WebAssembly !== 'undefined'
  const hasCrypto =
    typeof globalThis.crypto !== 'undefined' &&
    typeof globalThis.crypto.getRandomValues === 'function'

  if (!hasWorker || !hasWasm || !hasCrypto) {
    throw new Error(
      'StrongBox WASM bridge is required but this environment lacks WebWorker, WebAssembly, or secure randomness support.',
    )
  }
}
