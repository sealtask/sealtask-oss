import { afterEach, describe, expect, it, vi } from 'vitest'

const mockHash = vi.hoisted(() =>
  vi.fn(async (options: { hashLen: number }) => ({
    hash: new Uint8Array(options.hashLen).fill(9),
    hashHex: '',
    encoded: '',
  })),
)

vi.mock('argon2-browser', () => ({
  default: {
    ArgonType: { Argon2id: 2 },
    hash: mockHash,
  },
}))

vi.mock('argon2-browser/dist/argon2.wasm?url', () => ({
  default: '/assets/argon2.wasm',
}))

describe('Argon2 runtime configuration', () => {
  afterEach(() => {
    delete (globalThis as typeof globalThis & {
      argon2WasmPath?: string
      loadArgon2WasmBinary?: () => Promise<Uint8Array>
    }).argon2WasmPath
    delete (globalThis as typeof globalThis & {
      argon2WasmPath?: string
      loadArgon2WasmBinary?: () => Promise<Uint8Array>
    }).loadArgon2WasmBinary
  })

  it('uses configured defaults and loader when call-specific params are absent', async () => {
    const { configureArgon2Runtime, deriveKeyFromPassword } = await import('./argon2')
    const loadWasmBinary = vi.fn(async () => new Uint8Array([0, 97, 115, 109]))

    configureArgon2Runtime({
      defaultParams: {
        memoryKiB: 1_024,
        iterations: 1,
        parallelism: 2,
      },
      loadWasmBinary,
    })

    await expect(
      deriveKeyFromPassword('password', new Uint8Array(16).fill(1)),
    ).resolves.toEqual(new Uint8Array(32).fill(9))

    expect(mockHash).toHaveBeenCalledWith(
      expect.objectContaining({
        pass: 'password',
        hashLen: 32,
        type: 2,
        time: 1,
        mem: 1_024,
        parallelism: 2,
        raw: true,
      }),
    )
    expect(
      (globalThis as typeof globalThis & {
        loadArgon2WasmBinary?: () => Promise<Uint8Array>
      }).loadArgon2WasmBinary,
    ).toBe(loadWasmBinary)
    expect(() =>
      configureArgon2Runtime({
        defaultParams: {
          memoryKiB: 2_048,
          iterations: 2,
          parallelism: 1,
        },
      }),
    ).toThrow(/before the first derivation/i)
  })
})
