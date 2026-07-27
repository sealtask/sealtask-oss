import { Encoder, decode as cborDecode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { decodeBase64, encodeBase64 } from '../runtime/base64'
import type { StrongBoxBridge } from '../runtime/strong-box'
import { parseSealedPayload } from './sealed-payload'
import {
  TASK_REFERENCE_SCHEME_AEAD_CIPHERTEXT_BYTES,
  TASK_REFERENCE_SCHEME_CONTEXT,
  TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES,
  TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES,
  TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES,
  TASK_REFERENCE_ORDINARY_REVISION_MAX,
  TASK_REFERENCE_REPAIR_REVISION_MAX,
  TASK_REFERENCE_REVISION_MAX,
  buildTaskReferenceScheme,
  decodeTaskReferenceSchemePlaintext,
  decryptTaskReferenceScheme,
  encryptTaskReferenceScheme,
  formatTaskReference,
  formatTaskReferenceAliases,
  parseProjectReferenceNumber,
  parseTaskReference,
} from './task-reference-scheme'

const WORK_LIST_ID = '11111111-1111-7111-8111-111111111111'
const LETTER_WORK_LIST_ID = 'aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa'
const REVISION_ID = '22222222-2222-7222-8222-222222222222'
const listKey = new Uint8Array(32).fill(7)
const decoder = new TextDecoder()
const STRONG_BOX_FRAME_BYTES =
  TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES -
  TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES
const canonicalEncoder = new Encoder({
  useRecords: false,
  variableMapSize: true,
  tagUint8Array: false,
})

function framingBridge(contexts: string[] = []): StrongBoxBridge {
  return {
    async encrypt({ context, plaintext }) {
      contexts.push(`encrypt:${decoder.decode(context)}`)
      const framed = new Uint8Array(
        STRONG_BOX_FRAME_BYTES + plaintext.byteLength,
      )
      framed.set(plaintext, STRONG_BOX_FRAME_BYTES)
      return framed
    },
    async decrypt({ context, ciphertext }) {
      contexts.push(`decrypt:${decoder.decode(context)}`)
      return ciphertext.slice(STRONG_BOX_FRAME_BYTES)
    },
  }
}

function decryptsTo(plaintext: Uint8Array): StrongBoxBridge {
  return {
    ...framingBridge(),
    async decrypt() {
      return plaintext.slice()
    },
  }
}

function sealFakePlaintext(plaintext: Uint8Array): string {
  const ciphertext = new Uint8Array(TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES)
  ciphertext.set(plaintext, STRONG_BOX_FRAME_BYTES)
  return encodeBase64(
    canonicalEncoder.encode({ version: 1, ciphertext }),
  )
}

describe('task reference scheme protocol', () => {
  it('decodes the frozen Rust plaintext vector', () => {
    const plaintext = decodeBase64(
      'qWRraW5kdXRhc2tfcmVmZXJlbmNlX3NjaGVtZWd2ZXJzaW9uAWx3b3JrX2xpc3RfaWR4JDExMTExMTExLTExMTEtNzExMS04MTExLTExMTExMTExMTExMXJzY2hlbWVfcmV2aXNpb25faWR4JDIyMjIyMjIyLTIyMjItNzIyMi04MjIyLTIyMjIyMjIyMjIyMmhyZXZpc2lvbgFmcHJlZml4Y0xBV2lzZXBhcmF0b3JhLW5taW5pbXVtX2RpZ2l0cwRncGFkZGluZ1kBM6WlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaWlpaU=',
    )

    expect(plaintext).toHaveLength(
      TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES,
    )
    expect(
      decodeTaskReferenceSchemePlaintext({
        plaintext,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
      }),
    ).toEqual({
      kind: 'task_reference_scheme',
      version: 1,
      workListId: WORK_LIST_ID,
      schemeRevisionId: REVISION_ID,
      revision: 1,
      prefix: 'LAW',
      separator: '-',
      minimumDigits: 4,
    })
  })

  it('round-trips a strict fixed-size padded envelope with the frozen context', async () => {
    const contexts: string[] = []
    const strongBox = framingBridge(contexts)
    const scheme = buildTaskReferenceScheme({
      workListId: WORK_LIST_ID,
      schemeRevisionId: REVISION_ID,
      revision: 1,
      prefix: ' law ',
      minimumDigits: 4,
    })

    const sealed = await encryptTaskReferenceScheme({
      scheme,
      listKey,
      strongBox,
    })
    expect(parseSealedPayload(sealed.base64).ciphertext).toHaveLength(
      TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES,
    )
    expect(sealed.bytes).toHaveLength(
      TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES,
    )
    await expect(
      decryptTaskReferenceScheme({
        ciphertext: sealed.base64,
        listKey,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
        strongBox,
      }),
    ).resolves.toEqual({
      kind: 'task_reference_scheme',
      version: 1,
      workListId: WORK_LIST_ID,
      schemeRevisionId: REVISION_ID,
      revision: 1,
      prefix: 'LAW',
      separator: '-',
      minimumDigits: 4,
    })
    expect(contexts).toEqual([
      'encrypt:worklist.task_reference_scheme.v1',
      'decrypt:worklist.task_reference_scheme.v1',
    ])
    expect(TASK_REFERENCE_SCHEME_CONTEXT).toEqual(
      new TextEncoder().encode('worklist.task_reference_scheme.v1'),
    )
    expect(TASK_REFERENCE_SCHEME_AEAD_CIPHERTEXT_BYTES).toBe(528)
  })

  it('keeps every valid prefix and integer encoding at exactly 512 bytes', async () => {
    const strongBox = framingBridge()
    for (const [prefix, revision, minimumDigits] of [
      ['AB', 1, 1],
      ['A123456789', 36, 8],
    ] as const) {
      const sealed = await encryptTaskReferenceScheme({
        scheme: buildTaskReferenceScheme({
          workListId: WORK_LIST_ID,
          schemeRevisionId: REVISION_ID,
          revision,
          prefix,
          minimumDigits,
        }),
        listKey,
        strongBox,
      })
      expect(parseSealedPayload(sealed.base64).ciphertext).toHaveLength(
        TASK_REFERENCE_SCHEME_STRONG_BOX_BYTES,
      )
      expect(sealed.bytes).toHaveLength(
        TASK_REFERENCE_SCHEME_SEALED_PAYLOAD_BYTES,
      )
    }
  })

  it('validates public row identity and revision after decryption', async () => {
    const strongBox = framingBridge()
    const sealed = await encryptTaskReferenceScheme({
      scheme: buildTaskReferenceScheme({
        workListId: WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 3,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
      listKey,
      strongBox,
    })

    await expect(
      decryptTaskReferenceScheme({
        ciphertext: sealed.base64,
        listKey,
        expectedWorkListId: '33333333-3333-7333-8333-333333333333',
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 3,
        strongBox,
      }),
    ).rejects.toThrow('project identity mismatch')
    await expect(
      decryptTaskReferenceScheme({
        ciphertext: sealed.base64,
        listKey,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 4,
        strongBox,
      }),
    ).rejects.toThrow('revision mismatch')
  })

  it('rejects non-fixed or structurally extended plaintext envelopes', async () => {
    const short = sealFakePlaintext(new Uint8Array(511))
    await expect(
      decryptTaskReferenceScheme({
        ciphertext: short,
        listKey,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
        strongBox: decryptsTo(new Uint8Array(511)),
      }),
    ).rejects.toThrow('invalid size')

    const valid = await encryptTaskReferenceScheme({
      scheme: buildTaskReferenceScheme({
        workListId: WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 1,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
      listKey,
      strongBox: framingBridge(),
    })
    const decoded = cborDecode(
      parseSealedPayload(valid.base64).ciphertext.slice(
        STRONG_BOX_FRAME_BYTES,
      ),
    ) as Record<string, unknown>
    const extended = canonicalEncoder.encode({
      ...decoded,
      blind_index: 'forbidden',
    })
    const padded = new Uint8Array(512)
    padded.set(extended.subarray(0, 512))
    const malformed = sealFakePlaintext(padded)
    await expect(
      decryptTaskReferenceScheme({
        ciphertext: malformed,
        listKey,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
        strongBox: decryptsTo(padded),
      }),
    ).rejects.toThrow()
  })

  it('rejects duplicate top-level CBOR map keys before object decoding', async () => {
    const valid = await encryptTaskReferenceScheme({
      scheme: buildTaskReferenceScheme({
        workListId: WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 1,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
      listKey,
      strongBox: framingBridge(),
    })
    const plaintext = parseSealedPayload(valid.base64).ciphertext.slice(
      STRONG_BOX_FRAME_BYTES,
    )
    const decoded = cborDecode(plaintext) as Record<string, unknown>
    const duplicatePair = concatBytes(
      canonicalEncoder.encode('prefix'),
      canonicalEncoder.encode(decoded.prefix),
    )
    const base = canonicalEncoder.encode({
      ...decoded,
      padding: (decoded.padding as Uint8Array).subarray(
        duplicatePair.byteLength,
      ),
    })
    expect(base[0]).toBe(0xa9)
    const duplicateEnvelope = concatBytes(
      Uint8Array.of(0xaa),
      base.subarray(1),
      duplicatePair,
    )
    expect(duplicateEnvelope).toHaveLength(
      TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES,
    )
    expect(
      (cborDecode(duplicateEnvelope) as Record<string, unknown>).prefix,
    ).toBe('OPS')

    await expect(
      decryptTaskReferenceScheme({
        ciphertext: sealFakePlaintext(duplicateEnvelope),
        listKey,
        expectedWorkListId: WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
        strongBox: decryptsTo(duplicateEnvelope),
      }),
    ).rejects.toThrow('strict CBOR')
  })

  it('rejects uppercase UUID text in decrypted scheme envelopes', async () => {
    const valid = await encryptTaskReferenceScheme({
      scheme: buildTaskReferenceScheme({
        workListId: LETTER_WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 1,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
      listKey,
      strongBox: framingBridge(),
    })
    const plaintext = parseSealedPayload(valid.base64).ciphertext.slice(
      STRONG_BOX_FRAME_BYTES,
    )
    const decoded = cborDecode(plaintext) as Record<string, unknown>
    const uppercaseEnvelope = canonicalEncoder.encode({
      ...decoded,
      work_list_id: LETTER_WORK_LIST_ID.toUpperCase(),
    })
    expect(uppercaseEnvelope).toHaveLength(
      TASK_REFERENCE_SCHEME_PLAINTEXT_BYTES,
    )

    await expect(
      decryptTaskReferenceScheme({
        ciphertext: sealFakePlaintext(uppercaseEnvelope),
        listKey,
        expectedWorkListId: LETTER_WORK_LIST_ID,
        expectedSchemeRevisionId: REVISION_ID,
        expectedRevision: 1,
        strongBox: decryptsTo(uppercaseEnvelope),
      }),
    ).rejects.toThrow('canonical UUID')
    expect(() =>
      buildTaskReferenceScheme({
        workListId: LETTER_WORK_LIST_ID.toUpperCase(),
        schemeRevisionId: REVISION_ID,
        revision: 1,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
    ).toThrow('canonical UUID')
  })

  it('formats current and historical aliases and parses safe references', () => {
    const current = buildTaskReferenceScheme({
      workListId: WORK_LIST_ID,
      schemeRevisionId: REVISION_ID,
      revision: 2,
      prefix: 'LEGAL',
      minimumDigits: 4,
    })
    const historical = buildTaskReferenceScheme({
      workListId: WORK_LIST_ID,
      schemeRevisionId: '33333333-3333-7333-8333-333333333333',
      revision: 1,
      prefix: 'LAW',
      minimumDigits: 1,
    })

    expect(formatTaskReference(current, 31)).toBe('LEGAL-0031')
    expect(
      formatTaskReferenceAliases({
        current,
        historical: [historical, historical],
        referenceNumber: 31,
      }),
    ).toEqual(['LEGAL-0031', 'LAW-31'])
    expect(parseTaskReference(' law - 00031 ')).toEqual({
      prefix: 'LAW',
      referenceNumber: 31,
    })
    expect(parseTaskReference('LAW-0')).toBeNull()
    expect(parseTaskReference(`LAW-${Number.MAX_SAFE_INTEGER + 1}`)).toBeNull()
    expect(parseProjectReferenceNumber('#00031')).toBe(31)
  })

  it('rejects invalid grammar and unsafe scheme values', () => {
    expect(TASK_REFERENCE_ORDINARY_REVISION_MAX).toBe(32)
    expect(TASK_REFERENCE_REPAIR_REVISION_MAX).toBe(4)
    expect(TASK_REFERENCE_REVISION_MAX).toBe(36)
    expect(() =>
      buildTaskReferenceScheme({
        workListId: WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 1,
        prefix: 'A',
        minimumDigits: 1,
      }),
    ).toThrow('prefix')
    expect(() =>
      buildTaskReferenceScheme({
        workListId: WORK_LIST_ID,
        schemeRevisionId: REVISION_ID,
        revision: 37,
        prefix: 'OPS',
        minimumDigits: 1,
      }),
    ).toThrow('between 1 and 36')
  })
})

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const result = new Uint8Array(
    parts.reduce((total, part) => total + part.byteLength, 0),
  )
  let offset = 0
  for (const part of parts) {
    result.set(part, offset)
    offset += part.byteLength
  }
  return result
}
