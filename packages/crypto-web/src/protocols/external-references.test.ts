import { Encoder } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import { encodeBase64 } from '../runtime/base64'
import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  buildTaskExternalReferences,
  buildWorkListExternalReferences,
  decodeWorkListExternalReferences,
  decryptTaskExternalReferences,
  decryptWorkListExternalReferences,
  encodeTaskExternalReferences,
  encodeWorkListExternalReferences,
  encryptTaskExternalReferences,
  encryptWorkListExternalReferences,
  EXTERNAL_REFERENCE_ITEMS_MAX,
} from './external-references'

const WORK_LIST_ID = '11111111-1111-7111-8111-111111111111'
const TASK_ID = '33333333-3333-7333-8333-333333333333'
const OTHER_ID = '44444444-4444-7444-8444-444444444444'
const LIST_KEY = new Uint8Array(32).fill(0x54)

function passthroughBridge(contexts: string[]): StrongBoxBridge {
  const decoder = new TextDecoder()
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

describe('external reference protocols', () => {
  it('round-trips work-list and task envelopes under separate contexts', async () => {
    const contexts: string[] = []
    const strongBox = passthroughBridge(contexts)
    const item = {
      label: 'Matter',
      value: 'L-204',
      system: 'Firm matter-management system',
    }
    const workList = buildWorkListExternalReferences({
      workListId: WORK_LIST_ID,
      items: [item],
    })
    const task = buildTaskExternalReferences({
      workListId: WORK_LIST_ID,
      taskId: TASK_ID,
      items: [item],
    })

    const workListCiphertext =
      await encryptWorkListExternalReferences({
        envelope: workList,
        listKey: LIST_KEY,
        strongBox,
      })
    const taskCiphertext = await encryptTaskExternalReferences({
      envelope: task,
      listKey: LIST_KEY,
      strongBox,
    })

    await expect(
      decryptWorkListExternalReferences({
        ciphertext: workListCiphertext.base64,
        listKey: LIST_KEY,
        expectedWorkListId: WORK_LIST_ID,
        strongBox,
      }),
    ).resolves.toEqual(workList)
    await expect(
      decryptTaskExternalReferences({
        ciphertext: taskCiphertext.base64,
        listKey: LIST_KEY,
        expectedWorkListId: WORK_LIST_ID,
        expectedTaskId: TASK_ID,
        strongBox,
      }),
    ).resolves.toEqual(task)
    expect(contexts).toEqual([
      'encrypt:worklist.work_list_external_references.v1',
      'encrypt:worklist.task_external_references.v1',
      'decrypt:worklist.work_list_external_references.v1',
      'decrypt:worklist.task_external_references.v1',
    ])
  })

  it('rejects wrong identities, noncanonical IDs, and unknown fields', () => {
    const envelope = buildWorkListExternalReferences({
      workListId: WORK_LIST_ID,
      items: [{ label: 'Matter', value: 'L-204' }],
    })
    const plaintext = encodeWorkListExternalReferences(envelope)

    expect(() =>
      decodeWorkListExternalReferences({
        plaintext,
        expectedWorkListId: OTHER_ID,
      }),
    ).toThrow('identity mismatch')
    expect(() =>
      buildWorkListExternalReferences({
        workListId:
          'AAAAAAAA-AAAA-7AAA-8AAA-AAAAAAAAAAAA',
        items: [],
      }),
    ).toThrow('canonical UUID')

    const encoder = new Encoder({
      useRecords: false,
      variableMapSize: true,
      tagUint8Array: false,
    })
    const invalid = encoder.encode({
      kind: 'work_list_external_references',
      version: 1,
      work_list_id: WORK_LIST_ID,
      items: [],
      unexpected: true,
    })
    expect(() =>
      decodeWorkListExternalReferences({
        plaintext: new Uint8Array(invalid),
        expectedWorkListId: WORK_LIST_ID,
      }),
    ).toThrow('envelope is invalid')
  })

  it('enforces item count, UTF-8 byte bounds, whitespace, and controls', () => {
    expect(() =>
      buildWorkListExternalReferences({
        workListId: WORK_LIST_ID,
        items: Array.from(
          { length: EXTERNAL_REFERENCE_ITEMS_MAX + 1 },
          () => ({ label: 'Matter', value: 'L-204' }),
        ),
      }),
    ).toThrow('cannot contain more than')
    expect(() =>
      buildWorkListExternalReferences({
        workListId: WORK_LIST_ID,
        items: [{ label: 'é'.repeat(33), value: 'L-204' }],
      }),
    ).toThrow('label is invalid')
    expect(() =>
      buildWorkListExternalReferences({
        workListId: WORK_LIST_ID,
        items: [{ label: ' Matter', value: 'L-204' }],
      }),
    ).toThrow('label is invalid')
    expect(() =>
      buildWorkListExternalReferences({
        workListId: WORK_LIST_ID,
        items: [{ label: 'Matter', value: 'L\u0000-204' }],
      }),
    ).toThrow('value is invalid')
  })

  it('matches the frozen Rust work-list and task plaintext vectors', () => {
    const workListEncoded = encodeWorkListExternalReferences(
      buildWorkListExternalReferences({
        workListId: WORK_LIST_ID,
        items: [
          { label: 'Matter', value: 'L-204', system: 'Clio' },
        ],
      }),
    )
    const taskEncoded = encodeTaskExternalReferences(
      buildTaskExternalReferences({
        workListId: WORK_LIST_ID,
        taskId: TASK_ID,
        items: [
          { label: 'Matter', value: 'L-204', system: 'Clio' },
        ],
      }),
    )
    expect(encodeBase64(workListEncoded)).toBe(
      'pGRraW5keB13b3JrX2xpc3RfZXh0ZXJuYWxfcmVmZXJlbmNlc2d2ZXJzaW9uAWx3b3JrX2xpc3RfaWR4JDExMTExMTExLTExMTEtNzExMS04MTExLTExMTExMTExMTExMWVpdGVtc4GjZWxhYmVsZk1hdHRlcmV2YWx1ZWVMLTIwNGZzeXN0ZW1kQ2xpbw',
    )
    expect(encodeBase64(taskEncoded)).toBe(
      'pWRraW5keBh0YXNrX2V4dGVybmFsX3JlZmVyZW5jZXNndmVyc2lvbgFsd29ya19saXN0X2lkeCQxMTExMTExMS0xMTExLTcxMTEtODExMS0xMTExMTExMTExMTFndGFza19pZHgkMzMzMzMzMzMtMzMzMy03MzMzLTgzMzMtMzMzMzMzMzMzMzMzZWl0ZW1zgaNlbGFiZWxmTWF0dGVyZXZhbHVlZUwtMjA0ZnN5c3RlbWRDbGlv',
    )
  })
})
