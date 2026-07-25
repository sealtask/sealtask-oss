import { randomBytes } from '../runtime/random'

const RECOVERY_KEY_PREFIX = 'wl-rk-v1'
const RECOVERY_SECRET_BYTES = 32
const RECOVERY_SECRET_BASE64URL_LENGTH = 43
const RECOVERY_KEY_ID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export type ParsedRecoveryKey = {
  recoveryKeyId: string
  secret: string
}

export class RecoveryKeyFormatError extends Error {
  readonly code = 'invalid_recovery_key'

  constructor() {
    super('Enter a valid SealTask recovery key.')
    this.name = 'RecoveryKeyFormatError'
  }
}

export type RecoveryKeyParseOptions = {
  invalidRecoveryKey?: () => Error
}

export function createRecoverySecret(): string {
  return encodeBase64Url(randomBytes(RECOVERY_SECRET_BYTES))
}

export function formatRecoveryKey(recoveryKeyId: string, secret: string): string {
  return `${RECOVERY_KEY_PREFIX}:${recoveryKeyId}:${secret}`
}

export function parseRecoveryKey(
  value: string,
  options: RecoveryKeyParseOptions = {},
): ParsedRecoveryKey {
  const normalized = value.trim().replace(/\s+/g, '')
  const [prefix, recoveryKeyId, secret, extra] = normalized.split(':')
  if (
    prefix !== RECOVERY_KEY_PREFIX ||
    !recoveryKeyId ||
    !secret ||
    extra ||
    !RECOVERY_KEY_ID_PATTERN.test(recoveryKeyId) ||
    secret.length !== RECOVERY_SECRET_BASE64URL_LENGTH ||
    !/^[A-Za-z0-9_-]+$/.test(secret)
  ) {
    throw options.invalidRecoveryKey?.() ?? new RecoveryKeyFormatError()
  }
  return {
    recoveryKeyId: recoveryKeyId.toLowerCase(),
    secret,
  }
}

function encodeBase64Url(bytes: Uint8Array): string {
  let binary = ''
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte)
  })
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}
