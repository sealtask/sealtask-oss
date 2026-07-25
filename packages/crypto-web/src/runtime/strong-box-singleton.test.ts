import { expect, it, vi } from 'vitest'

it('retries StrongBox creation after a transient initialization failure', async () => {
  const bridge = {
    encrypt: vi.fn(),
    decrypt: vi.fn(),
  }
  const create = vi
    .fn()
    .mockRejectedValueOnce(new Error('transient worker failure'))
    .mockResolvedValueOnce(bridge)

  vi.doMock('./strong-box-worker-client', () => ({
    StrongBoxWorkerClient: { create },
  }))
  vi.stubGlobal('Worker', class {})

  const { getStrongBoxBridge } = await import('./strong-box')

  await expect(getStrongBoxBridge()).rejects.toThrow('transient worker failure')
  await expect(getStrongBoxBridge()).resolves.toBe(bridge)
  expect(create).toHaveBeenCalledTimes(2)
})

it('supports an injected host bridge factory before first use', async () => {
  vi.resetModules()
  const bridge = {
    encrypt: vi.fn(),
    decrypt: vi.fn(),
  }
  const bridgeFactory = vi.fn(async () => bridge)
  const {
    configureStrongBoxRuntime,
    getStrongBoxBridge,
  } = await import('./strong-box')

  configureStrongBoxRuntime({ bridgeFactory })

  await expect(getStrongBoxBridge()).resolves.toBe(bridge)
  await expect(getStrongBoxBridge()).resolves.toBe(bridge)
  expect(bridgeFactory).toHaveBeenCalledOnce()
  expect(() =>
    configureStrongBoxRuntime({ bridgeFactory }),
  ).toThrow(
    'StrongBox runtime must be configured before the first bridge request',
  )
})
