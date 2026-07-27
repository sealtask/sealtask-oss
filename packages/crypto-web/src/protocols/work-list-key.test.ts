import {
  decode as cborDecode,
  encode as cborEncode,
  Encoder,
} from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  parseSealedPayload,
  serializeSealedPayloadBase64,
} from './sealed-payload'
import {
  decryptWorkListKeyCiphertext,
  sealWorkListKeyForOwner,
} from './work-list'

const PROJECT_KEY_ENVELOPE_KIND = 'sealtask-project-key'
const PROJECT_KEY_ENVELOPE_VERSION = 2
const STANDARD_CBOR_ENCODER = new Encoder({
  tagUint8Array: false,
  useRecords: false,
})
const identityBridge: StrongBoxBridge = {
  async encrypt({ plaintext }) {
    return plaintext.slice()
  },
  async decrypt({ ciphertext }) {
    return ciphertext.slice()
  },
}

describe('project-key membership payloads', () => {
  it('defaults to a raw 32-byte plaintext readable by previously shipped iOS clients', async () => {
    const projectKey = Uint8Array.from(
      { length: 32 },
      (_, index) => index,
    )
    const sealed = await sealWorkListKeyForOwner({
      listKey: projectKey,
      dataKey: new Uint8Array(32).fill(0x44),
      strongBox: identityBridge,
    })
    const plaintext = parseSealedPayload(sealed.base64).ciphertext

    expect(plaintext).toEqual(projectKey)
    expect(plaintext).toHaveLength(32)
  })

  it('can explicitly seal a product-qualified v2 envelope after the rollout gate opens', async () => {
    const projectKey = Uint8Array.from(
      { length: 32 },
      (_, index) => index,
    )
    const sealed = await sealWorkListKeyForOwner({
      listKey: projectKey,
      dataKey: new Uint8Array(32).fill(0x44),
      strongBox: identityBridge,
      projectKeyWriteFormat: 'v2-envelope',
    })
    const plaintext = parseSealedPayload(sealed.base64).ciphertext

    expect(encodeBase64(plaintext)).toBe(
      'uQADZGtpbmR0c2VhbHRhc2stcHJvamVjdC1rZXlndmVyc2lvbgJja2V5WCAAAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHw',
    )
    expect(cborDecode(plaintext)).toEqual({
      kind: PROJECT_KEY_ENVELOPE_KIND,
      version: PROJECT_KEY_ENVELOPE_VERSION,
      key: projectKey,
    })
    await expect(
      decryptWorkListKeyCiphertext({
        ciphertext: sealed.base64,
        dataKey: new Uint8Array(32).fill(0x44),
        strongBox: identityBridge,
      }),
    ).resolves.toEqual(projectKey)
  })

  it('treats an ambiguous 32-byte plaintext as a legacy raw key before CBOR parsing', async () => {
    const projectKey = new Uint8Array(32)
    projectKey.set([0x58, 0x1e])
    projectKey.set(
      Uint8Array.from({ length: 30 }, (_, index) => index),
      2,
    )

    await expect(decryptPlaintext(projectKey)).resolves.toEqual(
      projectKey,
    )
  })

  it('never aliases caller-owned or bridge-returned Buffer storage during zeroization', async () => {
    const writerKey = Buffer.alloc(32, 0x47)
    await sealWorkListKeyForOwner({
      listKey: writerKey,
      dataKey: new Uint8Array(32).fill(0x44),
      strongBox: identityBridge,
    })
    expect(writerKey).toEqual(Buffer.alloc(32, 0x47))

    const bridgePlaintext = Buffer.alloc(32, 0x52)
    const bufferBridge: StrongBoxBridge = {
      ...identityBridge,
      async decrypt() {
        return bridgePlaintext
      },
    }
    const opened = await decryptWorkListKeyCiphertext({
      ciphertext: serializeSealedPayloadBase64({
        version: 1,
        ciphertext: new Uint8Array([0]),
      }),
      dataKey: new Uint8Array(32),
      strongBox: bufferBridge,
    })

    expect(opened).toEqual(new Uint8Array(32).fill(0x52))
    expect(bridgePlaintext).toEqual(Buffer.alloc(32))
  })

  it('zeroes the encoder-owned pooled plaintext after sealing', async () => {
    const projectKey = Uint8Array.from(
      { length: 32 },
      (_, index) => 0xa0 + index,
    )
    await sealWorkListKeyForOwner({
      listKey: projectKey,
      dataKey: new Uint8Array(32).fill(0x44),
      strongBox: {
        ...identityBridge,
        async encrypt() {
          return Uint8Array.of(1)
        },
      },
    })

    // cbor-x encoders share a retained backing pool. A tiny follow-up
    // encoding exposes that pool so this guards the writer's owned view.
    const poolProbe = STANDARD_CBOR_ENCODER.encode(0)
    expect(
      containsSubarray(new Uint8Array(poolProbe.buffer), projectKey),
    ).toBe(false)
  })

  it('retains the required historical byte-string and key-envelope compatibility', async () => {
    const projectKey = new Uint8Array(32).fill(0x35)
    const legacyBareArray = STANDARD_CBOR_ENCODER.encode(
      Array.from(projectKey),
    )
    const taggedByteString = concatBytes([
      Uint8Array.of(0xd8, 0x40),
      STANDARD_CBOR_ENCODER.encode(projectKey),
    ])
    const payloads = [
      STANDARD_CBOR_ENCODER.encode(projectKey),
      legacyBareArray,
      taggedByteString,
      cborEncode({ key: projectKey }),
      cborEncode({ key: Array.from(projectKey) }),
      cborEncode({ key: encodeBase64(projectKey) }),
      cborEncode({
        key: `${encodeBase64(projectKey).slice(0, 20)} \t\r\n${
          encodeBase64(projectKey).slice(20)
        }`,
      }),
      concatBytes([
        Uint8Array.of(0xa1),
        cborEncode('key'),
        taggedByteString,
      ]),
    ]

    for (const payload of payloads) {
      await expect(decryptPlaintext(payload)).resolves.toEqual(
        projectKey,
      )
    }
  })

  it('matches the shared legacy bare-byte-array vector', async () => {
    const legacyBareArray = STANDARD_CBOR_ENCODER.encode(
      Array.from({ length: 32 }, (_, index) => index),
    )
    expect(encodeBase64(legacyBareArray)).toBe(
      'mCAAAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGBgZGBoYGxgcGB0YHhgf',
    )
    await expect(decryptPlaintext(legacyBareArray)).resolves.toEqual(
      Uint8Array.from({ length: 32 }, (_, index) => index),
    )
  })

  it('rejects malformed lengths, envelope metadata, extra fields, and trailing data', async () => {
    const projectKey = new Uint8Array(32).fill(0x29)
    const validEnvelope = {
      kind: PROJECT_KEY_ENVELOPE_KIND,
      version: PROJECT_KEY_ENVELOPE_VERSION,
      key: projectKey,
    }
    const withTrailingData = appendByte(
      STANDARD_CBOR_ENCODER.encode(validEnvelope),
      0,
    )
    const duplicateKind = concatBytes([
      Uint8Array.of(0xa4),
      cborEncode('kind'),
      cborEncode('wrong-kind'),
      cborEncode('kind'),
      cborEncode(PROJECT_KEY_ENVELOPE_KIND),
      cborEncode('version'),
      cborEncode(PROJECT_KEY_ENVELOPE_VERSION),
      cborEncode('key'),
      STANDARD_CBOR_ENCODER.encode(projectKey),
    ])
    const taggedStrictKey = concatBytes([
      Uint8Array.of(0xa3),
      cborEncode('kind'),
      cborEncode(PROJECT_KEY_ENVELOPE_KIND),
      cborEncode('version'),
      cborEncode(PROJECT_KEY_ENVELOPE_VERSION),
      cborEncode('key'),
      Uint8Array.of(0xd8, 0x40),
      STANDARD_CBOR_ENCODER.encode(projectKey),
    ])
    const indefiniteMap = concatBytes([
      Uint8Array.of(0xbf),
      cborEncode('key'),
      STANDARD_CBOR_ENCODER.encode(projectKey),
      Uint8Array.of(0xff),
    ])
    const indefiniteByteString = concatBytes([
      Uint8Array.of(0x5f),
      STANDARD_CBOR_ENCODER.encode(projectKey),
      Uint8Array.of(0xff),
    ])
    const topLevelTaggedText = concatBytes([
      Uint8Array.of(0xd8, 0x40),
      cborEncode(encodeBase64(projectKey)),
    ])
    const doubleTaggedBytes = concatBytes([
      Uint8Array.of(0xd8, 0x40, 0xd8, 0x40),
      STANDARD_CBOR_ENCODER.encode(projectKey),
    ])
    const unicodeWhitespaceText = cborEncode({
      key: `${encodeBase64(projectKey).slice(0, 20)}\u00a0${
        encodeBase64(projectKey).slice(20)
      }`,
    })
    const nonCanonicalPadBits = cborEncode({
      key: `${'A'.repeat(42)}B=`,
    })
    const malformedPayloads = [
      new Uint8Array(31).fill(0x11),
      new Uint8Array(33).fill(0x11),
      new Uint8Array(513).fill(0x11),
      STANDARD_CBOR_ENCODER.encode(new Uint8Array(31).fill(0x11)),
      STANDARD_CBOR_ENCODER.encode(new Array(31).fill(0x11)),
      STANDARD_CBOR_ENCODER.encode(new Array(33).fill(0x11)),
      STANDARD_CBOR_ENCODER.encode([
        ...new Array(31).fill(0x11),
        256,
      ]),
      STANDARD_CBOR_ENCODER.encode([
        ...new Array(31).fill(0x11),
        -1,
      ]),
      STANDARD_CBOR_ENCODER.encode({
        ...validEnvelope,
        key: new Uint8Array(31).fill(0x29),
      }),
      STANDARD_CBOR_ENCODER.encode({
        ...validEnvelope,
        kind: 'seal-work-list-key',
      }),
      STANDARD_CBOR_ENCODER.encode({
        ...validEnvelope,
        version: 1,
      }),
      STANDARD_CBOR_ENCODER.encode({
        ...validEnvelope,
        extra: true,
      }),
      STANDARD_CBOR_ENCODER.encode({
        key: projectKey,
        extra: true,
      }),
      duplicateKind,
      taggedStrictKey,
      indefiniteMap,
      indefiniteByteString,
      topLevelTaggedText,
      doubleTaggedBytes,
      unicodeWhitespaceText,
      nonCanonicalPadBits,
      withTrailingData,
    ]

    for (const payload of malformedPayloads) {
      await expect(decryptPlaintext(payload)).rejects.toThrow()
    }

    for (const invalidKey of [
      new Uint8Array(31),
      new Uint8Array(33),
    ]) {
      await expect(
        sealWorkListKeyForOwner({
          listKey: invalidKey,
          dataKey: new Uint8Array(32),
          strongBox: identityBridge,
        }),
      ).rejects.toThrow('Project key must be exactly 32 bytes')
    }
  })
})

function decryptPlaintext(plaintext: Uint8Array): Promise<Uint8Array> {
  return decryptWorkListKeyCiphertext({
    ciphertext: serializeSealedPayloadBase64({
      version: 1,
      ciphertext: plaintext,
    }),
    dataKey: new Uint8Array(32),
    strongBox: identityBridge,
  })
}

function appendByte(bytes: Uint8Array, byte: number): Uint8Array {
  const result = new Uint8Array(bytes.length + 1)
  result.set(bytes)
  result[bytes.length] = byte
  return result
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const result = new Uint8Array(
    parts.reduce((length, part) => length + part.length, 0),
  )
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.length
  }
  return result
}

function containsSubarray(
  bytes: Uint8Array,
  candidate: Uint8Array,
): boolean {
  for (
    let offset = 0;
    offset <= bytes.length - candidate.length;
    offset += 1
  ) {
    let matches = true
    for (let index = 0; index < candidate.length; index += 1) {
      if (bytes[offset + index] !== candidate[index]) {
        matches = false
        break
      }
    }
    if (matches) return true
  }
  return false
}
