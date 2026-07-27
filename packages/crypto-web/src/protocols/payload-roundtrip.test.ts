import { encode as cborEncode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import type { StrongBoxBridge } from '../runtime/strong-box'
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

  it('round-trips task, comment, and note envelopes with frozen contexts', async () => {
    const contexts: string[] = []
    const strongBox = identityBridge(contexts)
    const listKey = new Uint8Array(32).fill(7)
    const richText = {
      format: 'plaintext' as const,
      version: 1,
      blocks: [{ type: 'paragraph' as const, text: 'Body' }],
    }

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
      'encrypt:worklist.task.v1',
      'decrypt:worklist.task.v1',
      'encrypt:worklist.comment.v1',
      'decrypt:worklist.comment.v1',
      'encrypt:worklist.note.v1',
      'decrypt:worklist.note.v1',
    ])
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
