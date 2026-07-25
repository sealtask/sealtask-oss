import { webcrypto } from 'node:crypto'

if (typeof globalThis.crypto === 'undefined') {
  Object.defineProperty(globalThis, 'crypto', {
    configurable: true,
    value: webcrypto,
  })
}

if (typeof globalThis.atob === 'undefined') {
  Object.defineProperty(globalThis, 'atob', {
    configurable: true,
    value: (value: string) => Buffer.from(value, 'base64').toString('binary'),
  })
}

if (typeof globalThis.btoa === 'undefined') {
  Object.defineProperty(globalThis, 'btoa', {
    configurable: true,
    value: (value: string) => Buffer.from(value, 'binary').toString('base64'),
  })
}
