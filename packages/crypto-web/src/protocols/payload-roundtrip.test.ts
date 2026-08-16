import { encode as cborEncode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import type { StrongBoxBridge } from '../runtime/strong-box'
import * as protocolExports from './index'
import {
  buildCommentPayloadEnvelope,
  decryptCommentPayload,
  encryptCommentPayload,
} from './comment'
import {
  buildNotePayloadEnvelope,
  decryptNoteKey,
  decryptNotePayload,
  encryptNoteKey,
  encryptNotePayload,
} from './note'
import {
  parseSealedPayload,
  parseSealedPayloadBytes,
  parseStrictSealedPayload,
  serializeSealedPayload,
  serializeSealedPayloadBase64,
} from './sealed-payload'
import {
  buildTaskPayloadEnvelope,
  decryptTaskPayload,
  encryptTaskPayload,
} from './task'
import {
  decryptWorkListPayload,
  encryptWorkListPayload,
} from './work-list'

const decoder = new TextDecoder()

function identityBridge(contexts: string[]): StrongBoxBridge {
  return {
    async encrypt({ context, plaintext }) {
      contexts.push(`encrypt:${decoder.decode(context)}`)
      return plaintext.slice()
    },
    async decrypt({ context, ciphertext }) {
      contexts.push(`decrypt:${decoder.decode(context)}`)
      return ciphertext.slice()
    },
  }
}

describe('sealed protocol payloads', () => {
  it('serializes and parses sealed payloads in byte and Base64 form', () => {
    const payload = {
      version: 1,
      ciphertext: new Uint8Array([9, 8, 7]),
    }
    expect(
      parseSealedPayloadBytes(serializeSealedPayload(payload)),
    ).toEqual(payload)
    expect(
      parseSealedPayload(serializeSealedPayloadBase64(payload)),
    ).toEqual(payload)
    expect(() => parseSealedPayload('encrypted-title')).toThrow(
      'Invalid sealed payload structure',
    )
  })

  it('strictly rejects unknown, duplicate, and trailing outer fields', () => {
    const unknown = new Uint8Array(
      cborEncode({
        version: 1,
        ciphertext: new Uint8Array([1]),
        extra: true,
      }),
    )
    const valid = serializeSealedPayload({
      version: 1,
      ciphertext: new Uint8Array([1]),
    })
    const trailing = new Uint8Array(valid.byteLength + 1)
    trailing.set(valid)
    const duplicate = new Uint8Array([
      0xa3,
      0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x01,
      0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x01,
      0x6a, 0x63, 0x69, 0x70, 0x68, 0x65, 0x72, 0x74, 0x65, 0x78,
      0x74, 0x41, 0x01,
    ])

    for (const bytes of [unknown, trailing, duplicate]) {
      expect(() =>
        parseStrictSealedPayload(encodeBase64(bytes)),
      ).toThrow('Invalid strict sealed payload structure')
    }
  })

  it('keeps strict sealed-payload framing separate from semantic decoding', () => {
    const truncated = concatBytes(
      Uint8Array.of(0xa2),
      cborText('version'),
      Uint8Array.of(0x01),
      cborText('ciphertext'),
      Uint8Array.of(0x42, 0x01),
    )
    const tooDeep = strictSealedPayloadWithVersion(nestedArrays(9))
    const atDepthLimit = strictSealedPayloadWithVersion(nestedArrays(8))

    for (const bytes of [
      truncated,
      tooDeep,
    ]) {
      expect(() => parseStrictSealedPayload(encodeBase64(bytes))).toThrow(
        'Invalid strict sealed payload structure',
      )
    }

    // Eight nested containers is the sealed payload's historic limit. Its
    // field is still invalid, but it reaches semantic validation rather than
    // being rejected by the structural traversal.
    expect(() =>
      parseStrictSealedPayload(encodeBase64(atDepthLimit)),
    ).toThrow('Invalid sealed payload structure')

    expect(
      parseStrictSealedPayload(encodeBase64(
        strictSealedPayloadWithVersion(Uint8Array.of(0xf9, 0x3c, 0x00)),
      )),
    ).toEqual({ version: 1, ciphertext: Uint8Array.of(1) })
    expect(() =>
      parseStrictSealedPayload(encodeBase64(
        strictSealedPayloadWithVersion(Uint8Array.of(0xf4)),
      )),
    ).toThrow('Invalid sealed payload structure')
  })

  it('accepts the tagged byte-string representation used by compatible CBOR writers', () => {
    const taggedCiphertext = concatBytes(
      Uint8Array.of(0xa2),
      cborText('version'),
      Uint8Array.of(0x01),
      cborText('ciphertext'),
      // RFC 8746 tag 64 wraps an unsigned 8-bit typed array.
      Uint8Array.of(0xd8, 0x40, 0x43, 0x09, 0x08, 0x07),
    )

    expect(parseStrictSealedPayload(encodeBase64(taggedCiphertext))).toEqual({
      version: 1,
      ciphertext: Uint8Array.of(9, 8, 7),
    })
  })

  it('round-trips work-list, task, comment, and note envelopes with frozen contexts', async () => {
    const contexts: string[] = []
    const strongBox = identityBridge(contexts)
    const listKey = new Uint8Array(32).fill(7)
    const richText = {
      format: 'plaintext' as const,
      version: 1,
      blocks: [{ type: 'paragraph' as const, text: 'Body' }],
    }

    const workList = {
      kind: 'work_list' as const,
      version: 1,
      body: { title: 'Project', sections: [] },
    }
    const sealedWorkList = await encryptWorkListPayload({
      envelope: workList,
      listKey,
      strongBox,
    })
    await expect(
      decryptWorkListPayload({
        ciphertext: sealedWorkList.base64,
        listKey,
        strongBox,
      }),
    ).resolves.toEqual(workList)

    const task = buildTaskPayloadEnvelope({
      title: 'Task',
      rich_text: richText,
    })
    const sealedTask = await encryptTaskPayload({
      envelope: task,
      listKey,
      strongBox,
    })
    await expect(
      decryptTaskPayload({
        ciphertext: sealedTask.base64,
        listKey,
        strongBox,
      }),
    ).resolves.toEqual(task)

    const comment = buildCommentPayloadEnvelope({
      content: richText,
    })
    const sealedComment = await encryptCommentPayload({
      envelope: comment,
      listKey,
      strongBox,
    })
    await expect(
      decryptCommentPayload({
        ciphertext: sealedComment.base64,
        listKey,
        strongBox,
      }),
    ).resolves.toEqual(comment)

    const note = buildNotePayloadEnvelope({
      title: 'Note',
      content: richText,
    })
    const sealedNote = await encryptNotePayload({
      envelope: note,
      noteKey: listKey,
      strongBox,
    })
    await expect(
      decryptNotePayload({
        ciphertext: sealedNote.base64,
        noteKey: listKey,
        strongBox,
      }),
    ).resolves.toEqual(note)

    expect(contexts).toEqual([
      'encrypt:worklist.work_list.v1',
      'decrypt:worklist.work_list.v1',
      'encrypt:worklist.task.v1',
      'decrypt:worklist.task.v1',
      'encrypt:worklist.comment.v1',
      'decrypt:worklist.comment.v1',
      'encrypt:worklist.note.v1',
      'decrypt:worklist.note.v1',
    ])
  })

  it('returns ciphertext fields without a plaintext fingerprint', async () => {
    const strongBox = identityBridge([])
    const listKey = new Uint8Array(32).fill(7)
    const richText = {
      format: 'plaintext' as const,
      version: 1,
      blocks: [{ type: 'paragraph' as const, text: 'Body' }],
    }

    const sealedPayloads = await Promise.all([
      encryptWorkListPayload({
        envelope: {
          kind: 'work_list',
          version: 1,
          body: { title: 'Project', sections: [] },
        },
        listKey,
        strongBox,
      }),
      encryptTaskPayload({
        envelope: buildTaskPayloadEnvelope({
          title: 'Task',
          rich_text: richText,
        }),
        listKey,
        strongBox,
      }),
      encryptCommentPayload({
        envelope: buildCommentPayloadEnvelope({ content: richText }),
        listKey,
        strongBox,
      }),
      encryptNotePayload({
        envelope: buildNotePayloadEnvelope({
          title: 'Note',
          content: richText,
        }),
        noteKey: listKey,
        strongBox,
      }),
    ])

    for (const sealed of sealedPayloads) {
      expect(Object.keys(sealed).sort()).toEqual(['base64', 'bytes'])
      expect(sealed).not.toHaveProperty('schemaHash')
    }
    expect(protocolExports).not.toHaveProperty('computeSchemaHash')
    expect(protocolExports).not.toHaveProperty('sealPayloadEnvelope')
    expect(protocolExports).not.toHaveProperty('openPayloadEnvelope')
  })

  it('wraps and unwraps private note keys with the note-key context', async () => {
    const contexts: string[] = []
    const strongBox = identityBridge(contexts)
    const dataKey = new Uint8Array(32).fill(3)
    const noteKey = new Uint8Array(32).map((_, index) => index)

    const wrapped = await encryptNoteKey(noteKey, dataKey, strongBox)
    await expect(
      decryptNoteKey(wrapped, dataKey, strongBox),
    ).resolves.toEqual(noteKey)
    expect(contexts).toEqual([
      'encrypt:worklist.note.key.v1',
      'decrypt:worklist.note.key.v1',
    ])
  })
})

function strictSealedPayloadWithVersion(version: Uint8Array): Uint8Array {
  return concatBytes(
    Uint8Array.of(0xa2),
    cborText('version'),
    version,
    cborText('ciphertext'),
    Uint8Array.of(0x41, 0x01),
  )
}

function cborText(value: string): Uint8Array {
  return new Uint8Array(cborEncode(value))
}

function nestedArrays(depth: number): Uint8Array {
  return Uint8Array.of(...Array<number>(depth).fill(0x81), 0x00)
}

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
