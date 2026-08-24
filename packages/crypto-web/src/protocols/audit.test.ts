import { describe, expect, it } from 'vitest'

import type { StrongBoxBridge } from '../runtime/strong-box'
import {
  buildAuditPatch,
  checklistChangedAudit,
  createAuditPatchSemantics,
  decryptAuditPayload,
  detailsChangeAudit,
  dueDateChangeAudit,
  extractAuditNarrative,
  priorityChangeAudit,
  sectionMoveAudit,
  taskContentChangeAudit,
  taskUpdateAudit,
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

  it('groups every semantic task edit into one audit envelope', () => {
    const operation = taskUpdateAudit([
      sectionMoveAudit({
        taskTitle: 'Ship',
        fromSectionName: 'Doing',
        toSectionName: 'Done',
      }),
      priorityChangeAudit({
        taskTitle: 'Ship',
        oldPriority: 3,
        newPriority: 5,
      }),
      checklistChangedAudit([
        { title: 'Verify release', changeType: 'toggled', nowDone: true },
      ]),
      detailsChangeAudit({ taskTitle: 'Ship' }),
      dueDateChangeAudit({
        taskTitle: 'Ship',
        oldDueAt: '2026-08-12',
        newDueAt: '2026-08-13',
      }),
    ])

    expect(createAuditPatchSemantics(operation)).toEqual({
      fields: [
        { field: 'sectionId', changeKind: 'update' },
        { field: 'priority', changeKind: 'update' },
        { field: 'checklist', changeKind: 'update' },
        { field: 'details', changeKind: 'update' },
        { field: 'dueAt', changeKind: 'update' },
      ],
      envelope: {
        kind: 'audit.task_updated',
        version: 2,
        body: {
          narrativeKey: 'features.audit.narratives.taskChanges',
          narrativeOptions: {
            changes: [
              {
                key: 'features.audit.narratives.taskMoved',
                options: { title: 'Ship', from: 'Doing', to: 'Done' },
              },
              {
                key: 'features.audit.narratives.priorityChanged',
                options: {
                  title: 'Ship',
                  oldPriority: { key: 'features.tasks.priority.medium' },
                  newPriority: { key: 'features.tasks.priority.high' },
                },
              },
              {
                key: 'features.audit.narratives.checklistItemToggled',
                options: {
                  title: 'Verify release',
                  state: { key: 'features.audit.narratives.checklistDone' },
                },
              },
              {
                key: 'features.audit.narratives.taskDetails',
                options: { title: 'Ship' },
              },
              {
                key: 'features.audit.narratives.dueDateChanged',
                options: {
                  title: 'Ship',
                  oldDate: { dateIso: '2026-08-12' },
                  newDate: { dateIso: '2026-08-13' },
                },
              },
            ],
          },
        },
      },
      payloadVersion: 1,
    })
  })

  it('keeps single task edits compatible and removes no-op children', () => {
    const operation = taskUpdateAudit([
      priorityChangeAudit({
        taskTitle: 'Ship',
        oldPriority: 3,
        newPriority: 3,
      }),
      dueDateChangeAudit({
        taskTitle: 'Ship',
        oldDueAt: null,
        newDueAt: '2026-08-13',
      }),
    ])

    expect(operation.type).toBe('task.dueDate')
    expect(createAuditPatchSemantics(operation)?.envelope.kind).toBe(
      'audit.due_date',
    )
  })

  it('describes every changed task content field with the compatible details narrative', () => {
    const operation = taskContentChangeAudit({
      taskTitle: 'Ship',
      fields: ['title', 'details', 'attachments'],
    })

    expect(createAuditPatchSemantics(operation)).toEqual({
      fields: [
        { field: 'title', changeKind: 'update' },
        { field: 'details', changeKind: 'update' },
        { field: 'attachments', changeKind: 'update' },
      ],
      envelope: {
        kind: 'audit.details',
        version: 2,
        body: {
          narrativeKey: 'features.audit.narratives.taskDetails',
          narrativeOptions: { title: 'Ship' },
        },
      },
      payloadVersion: 1,
    })
    expect(
      createAuditPatchSemantics(
        taskContentChangeAudit({ taskTitle: 'Ship', fields: [] }),
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

})
