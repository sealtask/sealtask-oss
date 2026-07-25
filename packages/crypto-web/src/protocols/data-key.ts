import { deriveKeyFromPassword } from '../runtime/argon2'
import { KEY_SIZE_BYTES } from '../runtime/constants'
import { hkdfExpand } from '../runtime/hkdf'
import type { OpaqueExportKey } from '../runtime/opaque'
import { randomBytes } from '../runtime/random'
import { getStrongBoxBridge, type StrongBoxBridge } from '../runtime/strong-box'
import {
  parseSealedPayload,
  serializeSealedPayloadBase64,
  type SealedPayload,
} from './sealed-payload'

const encoder = new TextEncoder()

const DATA_KEY_CONTEXT = encoder.encode('worklist.user.data_key')
const DATA_KEY_OPAQUE_CONTEXT = encoder.encode(
  'worklist.user.data_key.v2.opaque_export',
)
const RECOVERY_DATA_KEY_CONTEXT = encoder.encode(
  'worklist.user.data_key.recovery.v1',
)
const OPAQUE_EXPORT_KEY_INFO = 'worklist.user.data_key.wrap.v2'
const RECOVERY_EXPORT_KEY_INFO = 'worklist.user.recovery_data_key.wrap.v1'
const DATA_KEY_PAYLOAD_VERSIONS = [1, 2] as const
const DATA_KEY_OPAQUE_PAYLOAD_VERSIONS = [2] as const
const OPAQUE_EXPORT_KEY_BYTES = 64
export const DATA_KEY_SALT_BYTES = 32

type KeyDeriver = (
  password: string,
  salt: Uint8Array,
) => Promise<Uint8Array>
type ExportKeyDeriver = (
  exportKey: OpaqueExportKey,
  info: string,
) => Promise<Uint8Array>

export type CreateCiphertextParams = {
  password: string
  dataKey?: Uint8Array
  salt?: Uint8Array
  strongBox?: StrongBoxBridge
  deriveKey?: KeyDeriver
}

export type CreateCiphertextResult = {
  ciphertext: string
  dataKey: Uint8Array
  salt: Uint8Array
}

export type CreateOpaqueCiphertextResult = {
  ciphertext: string
  dataKey: Uint8Array
}

export type DecryptCiphertextParams = {
  password: string
  ciphertext: string
  strongBox?: StrongBoxBridge
  deriveKey?: KeyDeriver
}

export type DecryptCiphertextResult = {
  dataKey: Uint8Array
  salt: Uint8Array
}

export type DecryptOpaqueCiphertextResult = {
  dataKey: Uint8Array
}

export type DataKeyCiphertextVersion = 1 | 2

export async function createDataKeyCiphertext(
  params: CreateCiphertextParams,
): Promise<CreateCiphertextResult> {
  const {
    password,
    salt = randomBytes(DATA_KEY_SALT_BYTES),
    dataKey = randomBytes(KEY_SIZE_BYTES),
    strongBox,
    deriveKey = deriveKeyFromPassword,
  } = params

  assertSaltLength(salt)
  const saltCopy = copyBytes(salt)
  const dataKeyCopy = copyBytes(dataKey)
  const wrappingKey = await deriveKey(password, saltCopy)
  const bridge = strongBox ?? (await getStrongBoxBridge())
  const sealed = await bridge.encrypt({
    key: wrappingKey,
    context: DATA_KEY_CONTEXT,
    plaintext: dataKeyCopy,
  })
  const payload: SealedPayload = {
    version: 1,
    ciphertext: concatBytes(saltCopy, sealed),
  }
  return {
    ciphertext: serializeSealedPayloadBase64(payload),
    dataKey: dataKeyCopy,
    salt: saltCopy,
  }
}

export async function decryptDataKeyCiphertext(
  params: DecryptCiphertextParams,
): Promise<DecryptCiphertextResult> {
  const {
    password,
    ciphertext,
    strongBox,
    deriveKey = deriveKeyFromPassword,
  } = params
  const payload = parseSealedPayload(ciphertext, DATA_KEY_PAYLOAD_VERSIONS)
  if (payload.version !== 1) {
    throw new Error(
      `data key ciphertext version ${payload.version} requires OPAQUE export key`,
    )
  }
  if (payload.ciphertext.length <= DATA_KEY_SALT_BYTES) {
    throw new Error('data key payload is truncated')
  }

  const salt = payload.ciphertext.slice(0, DATA_KEY_SALT_BYTES)
  const sealed = payload.ciphertext.slice(DATA_KEY_SALT_BYTES)
  assertSaltLength(salt)
  const wrappingKey = await deriveKey(password, salt)
  const bridge = strongBox ?? (await getStrongBoxBridge())
  const dataKey = await bridge.decrypt({
    key: wrappingKey,
    context: DATA_KEY_CONTEXT,
    ciphertext: sealed,
  })
  assertDataKeyLength(dataKey)
  return {
    dataKey: copyBytes(dataKey),
    salt: copyBytes(salt),
  }
}

export function getDataKeyCiphertextVersion(
  ciphertext: string,
): DataKeyCiphertextVersion {
  const payload = parseSealedPayload(ciphertext, DATA_KEY_PAYLOAD_VERSIONS)
  if (payload.version !== 1 && payload.version !== 2) {
    throw new Error(`Unsupported data key ciphertext version: ${payload.version}`)
  }
  return payload.version
}

export async function createDataKeyCiphertextWithOpaqueExportKey(params: {
  exportKey: OpaqueExportKey
  dataKey?: Uint8Array
  strongBox?: StrongBoxBridge
  deriveExportKey?: ExportKeyDeriver
}): Promise<CreateOpaqueCiphertextResult> {
  const {
    exportKey,
    dataKey,
    strongBox,
    deriveExportKey = deriveOpaqueWrappingKey,
  } = params
  const plaintext = dataKey ? copyBytes(dataKey) : randomBytes(KEY_SIZE_BYTES)
  let wrappingKey: Uint8Array | null = null
  let returnedPlaintext = false
  try {
    assertDataKeyLength(plaintext)
    wrappingKey = await deriveExportKey(exportKey, OPAQUE_EXPORT_KEY_INFO)
    const bridge = strongBox ?? (await getStrongBoxBridge())
    const sealed = await bridge.encrypt({
      key: wrappingKey,
      context: DATA_KEY_OPAQUE_CONTEXT,
      plaintext,
    })
    const ciphertext = serializeSealedPayloadBase64(
      { version: 2, ciphertext: sealed },
      DATA_KEY_OPAQUE_PAYLOAD_VERSIONS,
    )
    const returnedDataKey = dataKey ?? plaintext
    returnedPlaintext = !dataKey
    return { ciphertext, dataKey: returnedDataKey }
  } finally {
    if (dataKey || !returnedPlaintext) {
      plaintext.fill(0)
    }
    wrappingKey?.fill(0)
  }
}

export async function decryptDataKeyCiphertextWithOpaqueExportKey(params: {
  exportKey: OpaqueExportKey
  ciphertext: string
  strongBox?: StrongBoxBridge
  deriveExportKey?: ExportKeyDeriver
}): Promise<DecryptOpaqueCiphertextResult> {
  const {
    exportKey,
    ciphertext,
    strongBox,
    deriveExportKey = deriveOpaqueWrappingKey,
  } = params
  const payload = parseSealedPayload(ciphertext, DATA_KEY_PAYLOAD_VERSIONS)
  if (payload.version !== 2) {
    throw new Error(
      `data key ciphertext version ${payload.version} requires password migration`,
    )
  }
  if (payload.ciphertext.length === 0) {
    throw new Error('data key payload is empty')
  }

  const wrappingKey = await deriveExportKey(
    exportKey,
    OPAQUE_EXPORT_KEY_INFO,
  )
  const bridge = strongBox ?? (await getStrongBoxBridge())
  try {
    const dataKey = await bridge.decrypt({
      key: wrappingKey,
      context: DATA_KEY_OPAQUE_CONTEXT,
      ciphertext: payload.ciphertext,
    })
    assertDataKeyLength(dataKey)
    return { dataKey: copyBytes(dataKey) }
  } finally {
    wrappingKey.fill(0)
  }
}

export async function createRecoveryDataKeyCiphertext(params: {
  recoveryExportKey: OpaqueExportKey
  dataKey: Uint8Array
  strongBox?: StrongBoxBridge
  deriveExportKey?: ExportKeyDeriver
}): Promise<CreateOpaqueCiphertextResult> {
  const {
    recoveryExportKey,
    dataKey,
    strongBox,
    deriveExportKey = deriveOpaqueWrappingKey,
  } = params
  const plaintext = copyBytes(dataKey)
  let wrappingKey: Uint8Array | null = null
  try {
    assertDataKeyLength(plaintext)
    wrappingKey = await deriveExportKey(
      recoveryExportKey,
      RECOVERY_EXPORT_KEY_INFO,
    )
    const bridge = strongBox ?? (await getStrongBoxBridge())
    const sealed = await bridge.encrypt({
      key: wrappingKey,
      context: RECOVERY_DATA_KEY_CONTEXT,
      plaintext,
    })
    return {
      ciphertext: serializeSealedPayloadBase64(
        { version: 2, ciphertext: sealed },
        DATA_KEY_OPAQUE_PAYLOAD_VERSIONS,
      ),
      dataKey,
    }
  } finally {
    plaintext.fill(0)
    wrappingKey?.fill(0)
  }
}

export async function decryptRecoveryDataKeyCiphertext(params: {
  recoveryExportKey: OpaqueExportKey
  ciphertext: string
  strongBox?: StrongBoxBridge
  deriveExportKey?: ExportKeyDeriver
}): Promise<DecryptOpaqueCiphertextResult> {
  const {
    recoveryExportKey,
    ciphertext,
    strongBox,
    deriveExportKey = deriveOpaqueWrappingKey,
  } = params
  const payload = parseSealedPayload(ciphertext, DATA_KEY_PAYLOAD_VERSIONS)
  if (payload.version !== 2) {
    throw new Error(`unsupported recovery data key version: ${payload.version}`)
  }
  const wrappingKey = await deriveExportKey(
    recoveryExportKey,
    RECOVERY_EXPORT_KEY_INFO,
  )
  const bridge = strongBox ?? (await getStrongBoxBridge())
  try {
    const dataKey = await bridge.decrypt({
      key: wrappingKey,
      context: RECOVERY_DATA_KEY_CONTEXT,
      ciphertext: payload.ciphertext,
    })
    assertDataKeyLength(dataKey)
    return { dataKey: copyBytes(dataKey) }
  } finally {
    wrappingKey.fill(0)
  }
}

export type RewrapDataKeyCiphertextParams = {
  oldPassword: string
  newPassword: string
  oldCiphertext: string
  strongBox?: StrongBoxBridge
  deriveKey?: KeyDeriver
}

export type RewrapDataKeyCiphertextResult = {
  newCiphertext: string
  dataKey: Uint8Array
  oldSalt: Uint8Array
  newSalt: Uint8Array
}

export async function rewrapDataKeyCiphertext(
  params: RewrapDataKeyCiphertextParams,
): Promise<RewrapDataKeyCiphertextResult> {
  const {
    oldPassword,
    newPassword,
    oldCiphertext,
    strongBox,
    deriveKey,
  } = params
  const { dataKey, salt: oldSalt } = await decryptDataKeyCiphertext({
    password: oldPassword,
    ciphertext: oldCiphertext,
    strongBox,
    deriveKey,
  })
  const {
    ciphertext: newCiphertext,
    salt: newSalt,
  } = await createDataKeyCiphertext({
    password: newPassword,
    dataKey,
    strongBox,
    deriveKey,
  })
  return {
    newCiphertext,
    dataKey: copyBytes(dataKey),
    oldSalt: copyBytes(oldSalt),
    newSalt,
  }
}

function concatBytes(...chunks: Uint8Array[]): Uint8Array {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0)
  const result = new Uint8Array(total)
  let offset = 0
  for (const chunk of chunks) {
    result.set(chunk, offset)
    offset += chunk.length
  }
  return result
}

function copyBytes(source: Uint8Array): Uint8Array {
  return source.slice()
}

function assertSaltLength(salt: Uint8Array): void {
  if (salt.length !== DATA_KEY_SALT_BYTES) {
    throw new Error(`data key salt must be ${DATA_KEY_SALT_BYTES} bytes`)
  }
}

function assertDataKeyLength(dataKey: Uint8Array): void {
  if (dataKey.length !== KEY_SIZE_BYTES) {
    throw new Error('data key must be 32 bytes')
  }
}

async function deriveOpaqueWrappingKey(
  exportKey: OpaqueExportKey,
  info: string,
): Promise<Uint8Array> {
  const exportKeyBytes = exportKeyToBytes(exportKey)
  try {
    return await hkdfExpand({
      parent: exportKeyBytes,
      info,
      length: KEY_SIZE_BYTES,
    })
  } finally {
    exportKeyBytes.fill(0)
  }
}

function exportKeyToBytes(exportKey: OpaqueExportKey): Uint8Array {
  const decoded = copyBytes(exportKey)
  if (decoded.length !== OPAQUE_EXPORT_KEY_BYTES) {
    decoded.fill(0)
    throw new Error('OPAQUE export key must be 64 bytes')
  }
  return decoded
}
