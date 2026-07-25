export const KEY_SIZE_BYTES = 32
export const SEALED_PAYLOAD_VERSION = 1
export const MIN_SALT_BYTES = 8

export type Argon2Params = {
  memoryKiB: number
  iterations: number
  parallelism: number
}

export const PRODUCTION_ARGON2_PARAMS: Argon2Params = Object.freeze({
  memoryKiB: 64 * 1024,
  iterations: 3,
  parallelism: 1,
})

// The public engine always defaults to production-strength parameters. Test
// harnesses that deliberately use cheaper settings must pass them explicitly.
export const DEFAULT_ARGON2_PARAMS: Argon2Params = PRODUCTION_ARGON2_PARAMS
