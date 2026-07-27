import { describe, expect, it } from 'vitest'

import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  buildAuditPatch,
  createAuditPatchSemantics,
  decryptAuditPayload,
  extractAuditNarrative,
  priorityChangeAudit,
} from './audit'

const decoder = new TextDecoder()

describe('audit protocol', () => {
  it('materializes deterministic semantics and skips no-op changes', () => {
    expect(
      createAuditPatchSemantics(
        priorityChangeAudit({
          taskTitle: 'Ship',
          oldPriority: 3,
          newPriority: 1,
        }),
      ),
    ).toEqual({
      fields: [{ field: 'priority', changeKind: 'update' }],
      envelope: {
        kind: 'audit.priority',
        version: 2,
        body: {
          narrativeKey: 'features.audit.narratives.priorityChanged',
          narrativeOptions: {
            title: 'Ship',
            oldPriority: { key: 'features.tasks.priority.medium' },
            newPriority: { key: 'features.tasks.priority.low' },
          },
        },
      },
      payloadVersion: 1,
    })
    expect(
      createAuditPatchSemantics(
        priorityChangeAudit({
          taskTitle: 'No change',
          oldPriority: 3,
          newPriority: 3,
        }),
      ),
    ).toBeNull()
  })

  it('encrypts, proves, and decrypts audit envelopes with audit-patch context', async () => {
    const contexts: string[] = []
    const strongBox: StrongBoxBridge = {
      async encrypt({ context, plaintext }) {
        contexts.push(`encrypt:${decoder.decode(context)}`)
        return plaintext.slice()
      },
      async decrypt({ context, ciphertext }) {
        contexts.push(`decrypt:${decoder.decode(context)}`)
        return ciphertext.slice()
      },
    }
    const result = await buildAuditPatch(
      priorityChangeAudit({
        taskTitle: 'Ship',
        oldPriority: null,
        newPriority: 8,
      }),
      {
        listKey: new Uint8Array(32).fill(1),
        bindingKey: new Uint8Array(32).fill(2),
        strongBox,
      },
    )
    expect(result).not.toBeNull()
    expect(result?.payloadCiphertextProof).toMatch(
      /^[A-Za-z0-9+/]{43}$/,
    )
    await expect(
      decryptAuditPayload({
        ciphertext: result?.payloadCiphertext ?? '',
        listKey: new Uint8Array(32).fill(1),
        strongBox,
      }),
    ).resolves.toMatchObject({
      kind: 'audit.priority',
      version: 2,
      body: {
        narrativeKey: 'features.audit.narratives.prioritySet',
      },
    })
    expect(contexts).toEqual([
      'encrypt:audit-patch',
      'decrypt:audit-patch',
    ])
  })

  it('uses an injected renderer and falls back to legacy text fields', () => {
    expect(
      extractAuditNarrative(
        {
          narrativeKey: 'features.audit.narratives.taskCreated',
          narrativeOptions: { title: 'New feature' },
          narrative: 'Legacy feature created',
        },
        ({ options }) => `Created ${String(options?.title)}`,
      ),
    ).toBe('Created New feature')
    expect(
      extractAuditNarrative({
        narrativeKey: 'missing',
        summary: ' Summary text ',
      }),
    ).toBe('Summary text')
  })

  it('keeps task-reference audit narratives fixed and prefix-free', () => {
    expect(
      createAuditPatchSemantics({
        type: 'task_reference_scheme.enabled',
        context: {},
      }),
    ).toEqual({
      fields: [{ field: 'task_references', changeKind: 'set' }],
      envelope: {
        kind: 'audit.task_reference_scheme_enabled',
        version: 2,
        body: {
          narrativeKey:
            'features.audit.narratives.taskReferenceSchemeEnabled',
          narrativeOptions: undefined,
        },
      },
      payloadVersion: 1,
    })
    expect(
      createAuditPatchSemantics({
        type: 'task_reference_scheme.updated',
        context: {},
      }),
    ).toMatchObject({
      fields: [
        {
          field: 'task_reference_scheme_ciphertext',
          changeKind: 'update',
        },
      ],
      envelope: {
        kind: 'audit.task_reference_scheme_updated',
        body: {
          narrativeKey:
            'features.audit.narratives.taskReferenceSchemeUpdated',
        },
      },
    })
  })

  it('keeps external-reference audit narratives fixed and value-free', () => {
    expect(
      createAuditPatchSemantics({
        type: 'work_list.external_references_updated',
        context: {},
      }),
    ).toMatchObject({
      fields: [
        {
          field: 'work_list_external_references_ciphertext',
          changeKind: 'update',
        },
      ],
      envelope: {
        kind: 'audit.work_list_external_references_updated',
        body: {
          narrativeKey:
            'features.audit.narratives.workListExternalReferencesUpdated',
        },
      },
    })
    expect(
      createAuditPatchSemantics({
        type: 'task.external_references_updated',
        context: {},
      }),
    ).toMatchObject({
      fields: [
        {
          field: 'task_external_references_ciphertext',
          changeKind: 'update',
        },
      ],
      envelope: {
        kind: 'audit.task_external_references_updated',
        body: {
          narrativeKey:
            'features.audit.narratives.taskExternalReferencesUpdated',
        },
      },
    })
  })
})
