import { client as opaqueClient, ready as opaqueReady } from '@serenity-kit/opaque'

import { decodeBase64 } from './base64'

const SERVER_IDENTIFIER = 'worklist.api'
// Must match backend RECOVERY_OPAQUE_SERVER_ID exactly; changing this
// invalidates recovery-key OPAQUE password files.
export const RECOVERY_SERVER_IDENTIFIER = 'worklist.api.recovery'
// @serenity-kit/opaque currently uses the OPAQUE 3DH Ristretto255/SHA-512
// suite, whose export key is the 64-byte SHA-512 output.
const OPAQUE_EXPORT_KEY_BYTES = 64

function ensureEmailIdentifier(email: string) {
  return email.trim().toLowerCase()
}

async function waitForOpaqueReady() {
  await opaqueReady
}

export type OpaqueRegistrationStart = {
  clientRegistrationState: string
  registrationRequest: string
}

export type OpaqueRegistrationFinishInput = {
  password: string
  clientRegistrationState: string
  serverRegistrationState: string
  email: string
  clientIdentifier?: string
  serverIdentifier?: string
}

type OpaqueRawExportKey = string | Uint8Array | ArrayBuffer

export type OpaqueExportKey = Uint8Array

export type OpaqueRegistrationFinishResult = {
  registrationRecord: string
  exportKey: OpaqueExportKey
}

export type OpaqueLoginStart = {
  clientLoginState: string
  startLoginRequest: string
}

export type OpaqueLoginFinishInput = {
  password: string
  clientLoginState: string
  serverLoginResponse: string
  email: string
  clientIdentifier?: string
  serverIdentifier?: string
}

export type OpaqueLoginFinishResult = {
  finishLoginRequest: string
  exportKey: OpaqueExportKey
}

export async function startOpaqueRegistration(password: string): Promise<OpaqueRegistrationStart> {
  await waitForOpaqueReady()
  return opaqueClient.startRegistration({ password })
}

export async function finishOpaqueRegistration(
  input: OpaqueRegistrationFinishInput,
): Promise<OpaqueRegistrationFinishResult> {
  await waitForOpaqueReady()
  const result = await opaqueClient.finishRegistration({
    password: input.password,
    clientRegistrationState: input.clientRegistrationState,
    registrationResponse: input.serverRegistrationState,
    identifiers: {
      client: input.clientIdentifier ?? ensureEmailIdentifier(input.email),
      server: input.serverIdentifier ?? SERVER_IDENTIFIER,
    },
  })

  return {
    registrationRecord: result.registrationRecord,
    exportKey: normalizeOpaqueExportKey(result.exportKey),
  }
}

export async function startOpaqueLogin(password: string): Promise<OpaqueLoginStart> {
  await waitForOpaqueReady()
  return opaqueClient.startLogin({ password })
}

export async function finishOpaqueLogin(
  input: OpaqueLoginFinishInput,
): Promise<OpaqueLoginFinishResult | undefined> {
  await waitForOpaqueReady()
  const result = await opaqueClient.finishLogin({
    password: input.password,
    clientLoginState: input.clientLoginState,
    loginResponse: input.serverLoginResponse,
    identifiers: {
      client: input.clientIdentifier ?? ensureEmailIdentifier(input.email),
      server: input.serverIdentifier ?? SERVER_IDENTIFIER,
    },
  })

  if (!result) {
    return undefined
  }

  return {
    finishLoginRequest: result.finishLoginRequest,
    exportKey: normalizeOpaqueExportKey(result.exportKey),
  }
}

export type OpaquePasswordChangeStart = {
  oldPasswordLoginRequest: string
  oldPasswordClientLoginState: string
  newPasswordRegistrationRequest: string
  newPasswordClientRegistrationState: string
}

export type OpaquePasswordChangeFinishInput = {
  oldPassword: string
  oldPasswordClientLoginState: string
  oldPasswordServerChallenge: string
  newPassword: string
  newPasswordClientRegistrationState: string
  newPasswordServerResponse: string
  email: string
}

export type OpaquePasswordChangeFinishResult = {
  oldPasswordFinishMessage: string
  newPasswordFinishMessage: string
  oldPasswordExportKey: OpaqueExportKey
  newPasswordExportKey: OpaqueExportKey
}

export async function startOpaquePasswordChange(
  oldPassword: string,
  newPassword: string,
): Promise<OpaquePasswordChangeStart> {
  await waitForOpaqueReady()

  const [oldPasswordLogin, newPasswordRegistration] = await Promise.all([
    startOpaqueLogin(oldPassword),
    startOpaqueRegistration(newPassword),
  ])

  return {
    oldPasswordLoginRequest: oldPasswordLogin.startLoginRequest,
    oldPasswordClientLoginState: oldPasswordLogin.clientLoginState,
    newPasswordRegistrationRequest: newPasswordRegistration.registrationRequest,
    newPasswordClientRegistrationState: newPasswordRegistration.clientRegistrationState,
  }
}

export async function finishOpaquePasswordChange(
  input: OpaquePasswordChangeFinishInput,
): Promise<OpaquePasswordChangeFinishResult> {
  await waitForOpaqueReady()
  let oldPasswordFinish: OpaqueLoginFinishResult | undefined
  let newPasswordFinish: OpaqueRegistrationFinishResult | undefined

  try {
    oldPasswordFinish = await finishOpaqueLogin({
      password: input.oldPassword,
      clientLoginState: input.oldPasswordClientLoginState,
      serverLoginResponse: input.oldPasswordServerChallenge,
      email: input.email,
    })

    if (!oldPasswordFinish?.finishLoginRequest) {
      throw new Error('Failed to verify old password')
    }

    newPasswordFinish = await finishOpaqueRegistration({
      password: input.newPassword,
      clientRegistrationState: input.newPasswordClientRegistrationState,
      serverRegistrationState: input.newPasswordServerResponse,
      email: input.email,
    })

    if (!newPasswordFinish?.registrationRecord) {
      throw new Error('Failed to register new password')
    }

    return {
      oldPasswordFinishMessage: oldPasswordFinish.finishLoginRequest,
      newPasswordFinishMessage: newPasswordFinish.registrationRecord,
      oldPasswordExportKey: oldPasswordFinish.exportKey,
      newPasswordExportKey: newPasswordFinish.exportKey,
    }
  } catch (error) {
    zeroizeOpaqueExportKey(oldPasswordFinish?.exportKey)
    zeroizeOpaqueExportKey(newPasswordFinish?.exportKey)
    throw error
  }
}

export function recoveryClientIdentifier(recoveryKeyId: string) {
  // Must match backend recovery_client_identifier(), which formats UUIDs with
  // Rust's lowercase hyphenated Display form. Changing this bricks existing
  // recovery-key OPAQUE password files.
  return `worklist.recovery.${recoveryKeyId.trim().toLowerCase()}`
}

export function normalizeOpaqueExportKey(exportKey: OpaqueRawExportKey): OpaqueExportKey {
  let decoded: Uint8Array
  if (typeof exportKey === 'string') {
    const normalized = exportKey.trim()
    if (normalized.length === 0) {
      throw new Error('OPAQUE export key is empty')
    }
    decoded = decodeBase64(normalized)
  } else if (exportKey instanceof ArrayBuffer) {
    decoded = new Uint8Array(exportKey.slice(0))
  } else {
    decoded = new Uint8Array(exportKey)
  }

  if (decoded.length !== OPAQUE_EXPORT_KEY_BYTES) {
    decoded.fill(0)
    throw new Error('OPAQUE export key must decode to 64 bytes')
  }

  return decoded
}

export function zeroizeOpaqueExportKey(exportKey: OpaqueExportKey | undefined | null) {
  exportKey?.fill(0)
}
