import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { encode as cborEncode } from 'cbor-x'
import { beforeAll, describe, expect, it, vi } from 'vitest'

import hpkeVectors from '../test/fixtures/hpke-vectors.json'
import passwordV1Fixture from '../test/fixtures/password-v1.json'
import {
  ATTACHMENT_BLOB_CONTEXT,
  decodeAttachmentBlobKey,
  decryptAttachmentBytes,
} from './protocols/attachment'
import {
  decryptDataKeyCiphertext,
  decryptDataKeyCiphertextWithOpaqueExportKey,
  decryptRecoveryDataKeyCiphertext,
} from './protocols/data-key'
import { decryptCommentPayload } from './protocols/comment'
import {
  deriveMemberEnvelopeKey,
  encodeInvitePackageBindingContext,
  encodeRecipientBindingContext,
  encodeRecipientPlaintext,
} from './protocols/invite-issuance'
import { decryptNotePayload } from './protocols/note'
import { decryptTaskPayload } from './protocols/task'
import { derivePayloadBindingKey, computePayloadProof } from './protocols/work-list'
import {
  encodeHpkeEnvelope,
  hpkeOpen,
  hpkeSeal,
} from './runtime/hpke'
import type { StrongBoxBridge } from './runtime/strong-box-types'
import { derivePublicKey } from './runtime/x25519'
import {
  INVITE_PREVIEW_AUTH_KIND,
  INVITE_PREVIEW_AUTH_SCHEME,
  buildInvitePreviewAuthMessageFromPackageBody,
  createInvitePreviewAuthenticator,
  encodeInvitePreviewMacMessage,
  verifyInvitePreviewAuthenticator,
  type InvitePreviewAuthenticator,
  type InvitePreviewMacParams,
} from './trust/invite-auth'
import {
  computeStatementDigest,
  hashTransparencyLeaf,
  reconstructInclusionRoot,
  verifyConsistencyProof,
} from './trust/transparency-proofs'

type PasswordDataKeyVector = {
  password: string
  saltB64: string
  wrappingKeyB64: string
  contextUtf8: string
  nonceB64: string
  ciphertextB64: string
}

type ExportDataKeyVector = {
  exportKeyB64: string
  wrappingKeyB64: string
  wrappingInfoUtf8: string
  contextUtf8: string
  nonceB64: string
  ciphertextB64: string
}

type PayloadVector = {
  contextUtf8: string
  envelope: {
    kind: 'task' | 'comment' | 'note'
    version: number
    body: Record<string, unknown>
  }
  plaintextCborB64: string
  sealedPayloadB64: string
}

type CompatibilityCorpus = {
  schemaVersion: number
  dataKeys: {
    dataKeyB64: string
    passwordV1: PasswordDataKeyVector
    opaqueV2: ExportDataKeyVector
    recoveryV1: ExportDataKeyVector
  }
  strongBox: {
    keyB64: string
    contextUtf8: string
    plaintextB64: string
    nonceB64: string
    keyIdB64: string
    ciphertextB64: string
  }
  payloadProof: {
    listKeyB64: string
    bindingKeyB64: string
    ciphertextB64: string
    proofB64: string
  }
  payloads: {
    task: PayloadVector
    comment: PayloadVector
    note: PayloadVector
  }
  attachment: {
    plaintextB64: string
    fileKeyB64: string
    blobContextUtf8: string
    blobNonceB64: string
    blobCiphertextB64: string
    listKeyB64: string
    referenceContextUtf8: string
    referenceNonceB64: string
    blobRef: {
      version: 1
      ciphertext_bytes: number
      file_key_b64: string
      enc_context: string
    }
    blobRefCborB64: string
    blobKeyB64: string
  }
  inviteBindings: {
    workListId: string
    membershipId: string
    userId: string
    role: 'member' | 'admin'
    keyFingerprintB64: string
    expiresAt: string | null
    inviteProtocolVersion: 1 | 2
    reservationRevision: number
    recipientContextCborB64: string
    packageContextCborB64: string
    listKeyB64: string
    saltB64: string
    memberKeyB64: string
    issuedAt: string
    invitePackageDigestB64: string
    recipientPlaintextCborB64: string
  }
  invitePreviewAuth: {
    packageVersion: 1 | 2
    role: 'member' | 'admin'
    packageBody: Record<string, unknown> & {
      work_list_id: string
      membership_id: string
      inviter: {
        id: string
      }
    }
    inviterPrivateKeyB64: string
    inviterPublicKeyB64: string
    recipientPrivateKeyB64: string
    recipientPublicKeyB64: string
    inviterKeyGeneration: number
    inviterKeyFingerprintB64: string
    recipientKeyFingerprintB64: string
    v1: AuthVersionVector
    v2: AuthVersionVector
  }
  transparency: {
    statements: Array<{
      userId: string
      generation: number
      inviteKeyB64: string
      statementDigestB64: string
      leafHashB64: string
    }>
    targetIndex: number
    logSize: number
    inclusionProofB64: string[]
    rootHashB64: string
    consistency: {
      fromSize: number
      prefixRootB64: string
      proofB64: string[]
    }
  }
}

type AuthVersionVector = {
  version: 1 | 2
  keyContextCborB64: string
  macMessageCborB64: string
  macB64: string
}

type StrongBoxExports = {
  memory: WebAssembly.Memory
  strong_box_result_size(): number
  strong_box_alloc(len: number): number
  strong_box_free(ptr: number, capacity: number): void
  strong_box_decrypt(
    keyPtr: number,
    keyLen: number,
    contextPtr: number,
    contextLen: number,
    ciphertextPtr: number,
    ciphertextLen: number,
    resultPtr: number,
  ): void
  strong_box_hpke_encap(
    recipientPublicKeyPtr: number,
    recipientPublicKeyLen: number,
    infoPtr: number,
    infoLen: number,
    aadPtr: number,
    aadLen: number,
    plaintextPtr: number,
    plaintextLen: number,
    resultPtr: number,
  ): void
  strong_box_hpke_decap(
    recipientPrivateKeyPtr: number,
    recipientPrivateKeyLen: number,
    infoPtr: number,
    infoLen: number,
    aadPtr: number,
    aadLen: number,
    encPtr: number,
    encLen: number,
    ciphertextPtr: number,
    ciphertextLen: number,
    resultPtr: number,
  ): void
}

type WasmRuntime = {
  wasm: StrongBoxExports
  memory: WebAssembly.Memory
  resultStructSize: number
}

const currentDirectory = path.dirname(fileURLToPath(import.meta.url))
const corpusPath = path.resolve(
  currentDirectory,
  '../../../testdata/crypto-compat-v1.json',
)
const strongBoxWasmPath = path.resolve(
  currentDirectory,
  '../../../artifacts/strong-box-wasm/strong_box_wasm_bg.wasm',
)
let corpus: CompatibilityCorpus
let strongBox: StrongBoxBridge

beforeAll(async () => {
  corpus = JSON.parse(await readFile(corpusPath, 'utf8')) as CompatibilityCorpus
  strongBox = createWasmStrongBoxBridge(await loadWasmRuntime())
})

describe('Rust/browser crypto compatibility corpus', () => {
  it('keeps the browser Argon2 fixture mirror aligned with the Rust corpus', () => {
    expect(passwordV1Fixture).toEqual({
      password: corpus.dataKeys.passwordV1.password,
      saltB64: corpus.dataKeys.passwordV1.saltB64,
      wrappingKeyB64: corpus.dataKeys.passwordV1.wrappingKeyB64,
    })
  })

  it('decrypts the canonical StrongBox frame with the public WASM artifact', async () => {
    expect(corpus.schemaVersion).toBe(1)
    await expect(strongBox.decrypt({
      key: decode(corpus.strongBox.keyB64),
      context: encodeUtf8(corpus.strongBox.contextUtf8),
      ciphertext: decode(corpus.strongBox.ciphertextB64),
    })).resolves.toEqual(decode(corpus.strongBox.plaintextB64))
  })

  it('matches RFC 9180 and cross-opens between the canonical WASM and JS fallback', async () => {
    if (!strongBox.hpkeEncap || !strongBox.hpkeDecap) {
      throw new Error('canonical StrongBox WASM does not expose HPKE operations')
    }

    const vector = hpkeVectors.rfc9180_appendix_a2
    const senderPrivateKey = decodeHex(vector.sender_private_key)
    const recipientPrivateKey = decodeHex(vector.recipient_private_key)
    const recipientPublicKey = decodeHex(vector.recipient_public_key)
    const info = decodeHex(vector.info)
    const aad = decodeHex(vector.aad)
    const plaintext = decodeHex(vector.plaintext)
    const originalWorker = globalThis.Worker
    const randomSpy = vi.spyOn(globalThis.crypto, 'getRandomValues')
      .mockImplementation((target) => {
        const bytes = new Uint8Array(
          target.buffer,
          target.byteOffset,
          target.byteLength,
        )
        if (bytes.length !== senderPrivateKey.length) {
          throw new Error('unexpected random byte request in HPKE compatibility test')
        }
        bytes.set(senderPrivateKey)
        return target
      })
    globalThis.Worker = undefined as unknown as typeof Worker

    try {
      const wasmSealed = await strongBox.hpkeEncap({
        recipientPublicKey,
        info,
        aad,
        plaintext,
      })
      expect(wasmSealed.enc).toEqual(decodeHex(vector.enc))
      expect(wasmSealed.nonce).toEqual(decodeHex(vector.nonce))
      expect(wasmSealed.ciphertext).toEqual(decodeHex(vector.ciphertext))

      const wasmEnvelope = encodeHpkeEnvelope(wasmSealed)
      expect(new Uint8Array(wasmEnvelope)).toEqual(decodeHex(vector.envelope))
      await expect(hpkeOpen({
        recipientPrivateKey,
        info,
        aad,
        envelope: wasmEnvelope,
      })).resolves.toEqual(plaintext)

      const jsSealed = await hpkeSeal({
        recipientPublicKey,
        info,
        aad,
        plaintext,
      })
      expect(jsSealed.enc).toEqual(decodeHex(vector.enc))
      expect(jsSealed.nonce).toEqual(decodeHex(vector.nonce))
      expect(jsSealed.ciphertext).toEqual(decodeHex(vector.ciphertext))
      await expect(strongBox.hpkeDecap({
        recipientPrivateKey,
        info,
        aad,
        enc: jsSealed.enc,
        ciphertext: jsSealed.ciphertext,
      })).resolves.toEqual(plaintext)
    } finally {
      randomSpy.mockRestore()
      globalThis.Worker = originalWorker
    }
  })

  it('consumes password-v1, OPAQUE-v2, and recovery data-key wrappers', async () => {
    const expectedDataKey = decode(corpus.dataKeys.dataKeyB64)
    const passwordVector = corpus.dataKeys.passwordV1
    const passwordResult = await decryptDataKeyCiphertext({
      password: passwordVector.password,
      ciphertext: passwordVector.ciphertextB64,
      strongBox,
      deriveKey: async (password, salt) => {
        expect(password).toBe(passwordVector.password)
        expect(salt).toEqual(decode(passwordVector.saltB64))
        return decode(passwordVector.wrappingKeyB64)
      },
    })
    expect(passwordResult.dataKey).toEqual(expectedDataKey)
    expect(passwordResult.salt).toEqual(decode(passwordVector.saltB64))

    const opaqueResult = await decryptDataKeyCiphertextWithOpaqueExportKey({
      exportKey: decode(corpus.dataKeys.opaqueV2.exportKeyB64),
      ciphertext: corpus.dataKeys.opaqueV2.ciphertextB64,
      strongBox,
    })
    expect(opaqueResult.dataKey).toEqual(expectedDataKey)

    const recoveryResult = await decryptRecoveryDataKeyCiphertext({
      recoveryExportKey: decode(corpus.dataKeys.recoveryV1.exportKeyB64),
      ciphertext: corpus.dataKeys.recoveryV1.ciphertextB64,
      strongBox,
    })
    expect(recoveryResult.dataKey).toEqual(expectedDataKey)
  })

  it('matches payload CBOR, decrypts task/comment/note, and computes the proof', async () => {
    const listKey = decode(corpus.payloadProof.listKeyB64)
    for (const vector of Object.values(corpus.payloads)) {
      expect(encode(cborEncode(vector.envelope))).toBe(vector.plaintextCborB64)
    }

    await expect(decryptTaskPayload({
      ciphertext: corpus.payloads.task.sealedPayloadB64,
      listKey,
      strongBox,
    })).resolves.toEqual(corpus.payloads.task.envelope)
    await expect(decryptCommentPayload({
      ciphertext: corpus.payloads.comment.sealedPayloadB64,
      listKey,
      strongBox,
    })).resolves.toEqual(corpus.payloads.comment.envelope)
    await expect(decryptNotePayload({
      ciphertext: corpus.payloads.note.sealedPayloadB64,
      noteKey: listKey,
      strongBox,
    })).resolves.toEqual(corpus.payloads.note.envelope)

    const bindingKey = await derivePayloadBindingKey({ listKey })
    expect(bindingKey).toEqual(decode(corpus.payloadProof.bindingKeyB64))
    await expect(computePayloadProof({
      ciphertext: decode(corpus.payloadProof.ciphertextB64),
      bindingKey,
    })).resolves.toBe(corpus.payloadProof.proofB64)
  })

  it('matches attachment reference CBOR and decrypts blob/reference fixtures', async () => {
    const vector = corpus.attachment
    const blobRef = {
      version: vector.blobRef.version,
      ciphertext_bytes: vector.blobRef.ciphertext_bytes,
      file_key: decode(vector.blobRef.file_key_b64),
      enc_context: vector.blobRef.enc_context,
    }
    expect(ATTACHMENT_BLOB_CONTEXT).toEqual(encodeUtf8(vector.blobContextUtf8))
    expect(encode(cborEncode(blobRef))).toBe(vector.blobRefCborB64)

    await expect(decryptAttachmentBytes({
      ciphertext: decode(vector.blobCiphertextB64),
      fileKey: decode(vector.fileKeyB64),
      encContext: vector.blobContextUtf8,
      strongBox,
    })).resolves.toEqual(decode(vector.plaintextB64))

    const decodedBlobRef = await decodeAttachmentBlobKey({
      listKey: decode(vector.listKeyB64),
      blobKey: decode(vector.blobKeyB64),
      strongBox,
    })
    expect(decodedBlobRef).toEqual(blobRef)
  })

  it('matches invite binding contexts, recipient plaintext, and member-key derivation', async () => {
    const vector = corpus.inviteBindings
    expect(encode(encodeRecipientBindingContext({
      workListId: vector.workListId,
      membershipBindingId: vector.membershipId,
      role: vector.role,
      keyFingerprintB64: vector.keyFingerprintB64,
    }))).toBe(vector.recipientContextCborB64)
    expect(encode(encodeInvitePackageBindingContext({
      workListId: vector.workListId,
      membershipBindingId: vector.membershipId,
      role: vector.role,
      keyFingerprintB64: vector.keyFingerprintB64,
      expiresAt: vector.expiresAt,
      inviteProtocolVersion: vector.inviteProtocolVersion,
      reservationRevision: vector.reservationRevision,
    }))).toBe(vector.packageContextCborB64)
    expect(encode(encodeRecipientPlaintext({
      workListId: vector.workListId,
      membershipBindingId: vector.membershipId,
      role: vector.role,
      listKey: decode(vector.listKeyB64),
      issuedAt: vector.issuedAt,
      invitePackageDigestB64: vector.invitePackageDigestB64,
    }))).toBe(vector.recipientPlaintextCborB64)
    await expect(deriveMemberEnvelopeKey({
      listKey: decode(vector.listKeyB64),
      userId: vector.userId,
      salt: decode(vector.saltB64),
    })).resolves.toEqual(decode(vector.memberKeyB64))
  })

  it('authenticates invite preview v1 and v2 vectors with current package code', async () => {
    const vector = corpus.invitePreviewAuth
    const inviterPrivateKey = decode(vector.inviterPrivateKeyB64)
    const inviterPublicKey = decode(vector.inviterPublicKeyB64)
    const recipientPrivateKey = decode(vector.recipientPrivateKeyB64)
    const recipientPublicKey = decode(vector.recipientPublicKeyB64)
    await expect(derivePublicKey(inviterPrivateKey)).resolves.toEqual(inviterPublicKey)
    await expect(derivePublicKey(recipientPrivateKey)).resolves.toEqual(recipientPublicKey)

    const message = buildInvitePreviewAuthMessageFromPackageBody({
      signedBody: vector.packageBody,
      role: vector.role,
      packageVersion: vector.packageVersion,
    })
    expect(message).not.toBeNull()
    if (!message) {
      throw new Error('fixture invite preview body is invalid')
    }
    const macParams: InvitePreviewMacParams = {
      message,
      privateKey: inviterPrivateKey,
      peerPublicKey: recipientPublicKey,
      inviterUserId: vector.packageBody.inviter.id,
      inviterKeyGeneration: vector.inviterKeyGeneration,
      inviterKeyFingerprintB64: vector.inviterKeyFingerprintB64,
      recipientKeyFingerprintB64: vector.recipientKeyFingerprintB64,
    }
    expect(encode(encodeInvitePreviewMacMessage(macParams)))
      .toBe(vector.v2.macMessageCborB64)

    const currentAuthenticator = await createInvitePreviewAuthenticator({
      message,
      inviterAuth: {
        privateKey: inviterPrivateKey,
        publicKey: inviterPublicKey,
        generation: vector.inviterKeyGeneration,
      },
      recipientPublicKey,
    })
    expect(currentAuthenticator.body.mac).toBe(vector.v2.macB64)
    await expect(verifyInvitePreviewAuthenticator({
      message,
      authenticator: currentAuthenticator,
      recipientPrivateKey,
      recipientPublicKey,
      inviterPublicKey,
    })).resolves.toBe(true)

    const v1Authenticator: InvitePreviewAuthenticator = {
      kind: INVITE_PREVIEW_AUTH_KIND,
      version: 1,
      body: {
        scheme: INVITE_PREVIEW_AUTH_SCHEME,
        inviter_user_id: vector.packageBody.inviter.id,
        inviter_key_generation: vector.inviterKeyGeneration,
        inviter_key_fingerprint: vector.inviterKeyFingerprintB64,
        recipient_key_fingerprint: vector.recipientKeyFingerprintB64,
        mac: vector.v1.macB64,
      },
    }
    await expect(verifyInvitePreviewAuthenticator({
      message,
      authenticator: v1Authenticator,
      recipientPrivateKey,
      recipientPublicKey,
      inviterPublicKey,
    })).resolves.toBe(true)
  })

  it('matches transparency digests, inclusion proof, and consistency proof', async () => {
    const vector = corpus.transparency
    for (const statement of vector.statements) {
      const digest = await computeStatementDigest({
        userId: statement.userId,
        generation: statement.generation,
        inviteKey: decode(statement.inviteKeyB64),
      })
      expect(digest).toEqual(decode(statement.statementDigestB64))
      await expect(hashTransparencyLeaf(digest))
        .resolves.toEqual(decode(statement.leafHashB64))
    }

    const target = vector.statements[vector.targetIndex]
    if (!target) {
      throw new Error('fixture transparency target is missing')
    }
    await expect(reconstructInclusionRoot(
      decode(target.leafHashB64),
      vector.targetIndex,
      vector.logSize,
      vector.inclusionProofB64.map(decode),
    )).resolves.toEqual(decode(vector.rootHashB64))

    const consistency = await verifyConsistencyProof(
      vector.consistency.fromSize,
      vector.logSize,
      vector.consistency.proofB64.map(decode),
    )
    expect(consistency.prefixRoot)
      .toEqual(decode(vector.consistency.prefixRootB64))
    expect(consistency.fullRoot).toEqual(decode(vector.rootHashB64))
  })
})

async function loadWasmRuntime(): Promise<WasmRuntime> {
  const bytes = await readFile(strongBoxWasmPath)
  let memory: WebAssembly.Memory | null = null
  const imports: WebAssembly.Imports = {
    strong_box: {
      strong_box_random(pointer: number, length: number) {
        if (!memory) {
          throw new Error('StrongBox WASM memory is not initialized')
        }
        crypto.getRandomValues(new Uint8Array(memory.buffer, pointer, length))
        return 0
      },
    },
  }
  const { instance } = await WebAssembly.instantiate(bytes, imports)
  const wasm = instance.exports as unknown as StrongBoxExports
  memory = wasm.memory
  return {
    wasm,
    memory,
    resultStructSize: Number(wasm.strong_box_result_size()),
  }
}

function createWasmStrongBoxBridge(runtime: WasmRuntime): StrongBoxBridge {
  return {
    encrypt: async () => {
      throw new Error('compatibility fixtures only exercise deterministic decryptions')
    },
    decrypt: async ({ key, context, ciphertext }) =>
      decryptWithWasm(runtime, key, context, ciphertext),
    hpkeEncap: async ({ recipientPublicKey, info, aad, plaintext }) => {
      const result = executeHpkeWithWasm(runtime, 'encap', {
        recipientKey: recipientPublicKey,
        info,
        aad,
        payload: plaintext,
      })
      return {
        nonce: result.nonce,
        enc: result.enc,
        ciphertext: result.payload,
      }
    },
    hpkeDecap: async ({
      recipientPrivateKey,
      info,
      aad,
      enc,
      ciphertext,
    }) => executeHpkeWithWasm(runtime, 'decap', {
      recipientKey: recipientPrivateKey,
      info,
      aad,
      enc,
      payload: ciphertext,
    }).payload,
  }
}

function decryptWithWasm(
  runtime: WasmRuntime,
  key: Uint8Array,
  context: Uint8Array,
  ciphertext: Uint8Array,
): Uint8Array {
  const { wasm, memory, resultStructSize } = runtime
  const resultPointer = wasm.strong_box_alloc(resultStructSize)
  const keyRef = copyIntoWasm(wasm, memory, key)
  const contextRef = copyIntoWasm(wasm, memory, context)
  const ciphertextRef = copyIntoWasm(wasm, memory, ciphertext)

  try {
    wasm.strong_box_decrypt(
      keyRef.pointer,
      keyRef.length,
      contextRef.pointer,
      contextRef.length,
      ciphertextRef.pointer,
      ciphertextRef.length,
      resultPointer,
    )
    const result = readWasmResult(wasm, memory, resultPointer)
    if (result.errorCode !== 0) {
      throw new Error(result.errorMessage ?? 'StrongBox decryption failed')
    }
    return result.bytes
  } finally {
    wasm.strong_box_free(resultPointer, resultStructSize)
    wasm.strong_box_free(keyRef.pointer, keyRef.capacity)
    wasm.strong_box_free(contextRef.pointer, contextRef.capacity)
    wasm.strong_box_free(ciphertextRef.pointer, ciphertextRef.capacity)
  }
}

function executeHpkeWithWasm(
  runtime: WasmRuntime,
  operation: 'encap' | 'decap',
  params: {
    recipientKey: Uint8Array
    info: Uint8Array
    aad: Uint8Array
    enc?: Uint8Array
    payload: Uint8Array
  },
) {
  const { wasm, memory, resultStructSize } = runtime
  const resultPointer = wasm.strong_box_alloc(resultStructSize)
  const recipientRef = copyIntoWasm(wasm, memory, params.recipientKey)
  const infoRef = copyIntoWasm(wasm, memory, params.info)
  const aadRef = copyIntoWasm(wasm, memory, params.aad)
  const payloadRef = copyIntoWasm(wasm, memory, params.payload)
  const encRef = params.enc
    ? copyIntoWasm(wasm, memory, params.enc)
    : undefined

  try {
    if (operation === 'encap') {
      wasm.strong_box_hpke_encap(
        recipientRef.pointer,
        recipientRef.length,
        infoRef.pointer,
        infoRef.length,
        aadRef.pointer,
        aadRef.length,
        payloadRef.pointer,
        payloadRef.length,
        resultPointer,
      )
    } else {
      if (!encRef) {
        throw new Error('HPKE decapsulation requires an encapsulated key')
      }
      wasm.strong_box_hpke_decap(
        recipientRef.pointer,
        recipientRef.length,
        infoRef.pointer,
        infoRef.length,
        aadRef.pointer,
        aadRef.length,
        encRef.pointer,
        encRef.length,
        payloadRef.pointer,
        payloadRef.length,
        resultPointer,
      )
    }

    const result = readWasmResult(wasm, memory, resultPointer)
    if (result.errorCode !== 0) {
      throw new Error(result.errorMessage ?? 'StrongBox HPKE operation failed')
    }
    return decodeHpkeResult(result.bytes)
  } finally {
    wasm.strong_box_free(resultPointer, resultStructSize)
    wasm.strong_box_free(recipientRef.pointer, recipientRef.capacity)
    wasm.strong_box_free(infoRef.pointer, infoRef.capacity)
    wasm.strong_box_free(aadRef.pointer, aadRef.capacity)
    wasm.strong_box_free(payloadRef.pointer, payloadRef.capacity)
    if (encRef) {
      wasm.strong_box_free(encRef.pointer, encRef.capacity)
    }
  }
}

function copyIntoWasm(
  wasm: StrongBoxExports,
  memory: WebAssembly.Memory,
  data: Uint8Array,
) {
  const capacity = Math.max(data.byteLength, 1)
  const pointer = wasm.strong_box_alloc(capacity)
  if (data.byteLength > 0) {
    new Uint8Array(memory.buffer, pointer, data.byteLength).set(data)
  }
  return { pointer, length: data.byteLength, capacity }
}

function readWasmResult(
  wasm: StrongBoxExports,
  memory: WebAssembly.Memory,
  resultPointer: number,
) {
  const view = new DataView(memory.buffer)
  const valuePointer = view.getUint32(resultPointer, true)
  const valueLength = view.getUint32(resultPointer + 4, true)
  const valueCapacity = view.getUint32(resultPointer + 8, true)
  const errorCode = view.getUint32(resultPointer + 12, true)
  const bytes = valueLength === 0
    ? new Uint8Array(0)
    : new Uint8Array(memory.buffer.slice(
      valuePointer,
      valuePointer + valueLength,
    ))
  const errorMessage = errorCode !== 0 && bytes.length > 0
    ? new TextDecoder().decode(bytes)
    : undefined
  if (valueCapacity > 0) {
    wasm.strong_box_free(valuePointer, valueCapacity)
  }
  return { bytes, errorCode, errorMessage }
}

function decodeHpkeResult(bytes: Uint8Array) {
  if (bytes.byteLength < 12) {
    throw new Error('StrongBox HPKE response is truncated')
  }
  const view = new DataView(
    bytes.buffer,
    bytes.byteOffset,
    bytes.byteLength,
  )
  const nonceLength = view.getUint32(0, true)
  const encLength = view.getUint32(4, true)
  const payloadLength = view.getUint32(8, true)
  const expectedLength = 12 + nonceLength + encLength + payloadLength
  if (expectedLength !== bytes.byteLength) {
    throw new Error('StrongBox HPKE response has invalid field lengths')
  }

  const nonceStart = 12
  const encStart = nonceStart + nonceLength
  const payloadStart = encStart + encLength
  return {
    nonce: bytes.slice(nonceStart, encStart),
    enc: bytes.slice(encStart, payloadStart),
    payload: bytes.slice(payloadStart),
  }
}

function decode(value: string): Uint8Array {
  return new Uint8Array(Buffer.from(value, 'base64'))
}

function decodeHex(value: string): Uint8Array {
  if (value.length % 2 !== 0 || !/^[0-9a-f]*$/i.test(value)) {
    throw new Error('fixture contains invalid hexadecimal data')
  }
  return Uint8Array.from(
    value.match(/.{2}/g) ?? [],
    (byte) => Number.parseInt(byte, 16),
  )
}

function encode(value: Uint8Array): string {
  return Buffer.from(value).toString('base64').replace(/=+$/, '')
}

function encodeUtf8(value: string): Uint8Array {
  return new TextEncoder().encode(value)
}
