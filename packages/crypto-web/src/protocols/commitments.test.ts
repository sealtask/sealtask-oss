import { describe, expect, it } from 'vitest'

import { computeNoteCreateSemanticCommitment } from './note-create-commitment'
import {
  canonicalizeTemplateImportSemanticPlan,
  computeTemplateImportSemanticCommitment,
} from './template-import-commitment'

describe('semantic commitments', () => {
  it('keeps note commitments stable across retries and scoped to semantics', async () => {
    const listKey = new Uint8Array(32).fill(0x31)
    const plan = {
      title: 'Release context',
      content: {
        format: 'rich_text',
        blocks: [{ type: 'paragraph', text: 'Body' }],
      },
      mentions: [],
      attachments: [],
      clientMeta: {},
      isPrivate: false,
    }
    const first = await computeNoteCreateSemanticCommitment({
      listKey,
      plan,
    })
    const retry = await computeNoteCreateSemanticCommitment({
      listKey,
      plan: { ...plan },
    })
    const changed = await computeNoteCreateSemanticCommitment({
      listKey,
      plan: { ...plan, title: 'Different' },
    })

    expect(retry).toBe(first)
    expect(changed).not.toBe(first)
    expect(first).toMatch(/^[A-Za-z0-9+/]{43}$/)
  })

  it('has a frozen template canonicalization and commitment vector', async () => {
    const listKey = new Uint8Array(32).map((_, index) => index)
    const plan = {
      workList: { timezone: 'UTC', title: 'Launch' },
      tasks: [{ title: 'Invite', dueAt: null, priority: 5 }],
      protocolVersion: 1,
    }
    expect(canonicalizeTemplateImportSemanticPlan(plan)).toBe(
      '{"protocolVersion":1,"tasks":[{"dueAt":null,"priority":5,"title":"Invite"}],"workList":{"timezone":"UTC","title":"Launch"}}',
    )
    await expect(
      computeTemplateImportSemanticCommitment({ listKey, plan }),
    ).resolves.toBe(
      'xlbbZoUVK8AZhWZnW+Ljq5QKJMpo4ejks5zJ2O588bU',
    )
  })

  it('rejects ambiguous canonical values and invalid key sizes', async () => {
    const cyclic: Record<string, unknown> = {}
    cyclic.self = cyclic
    expect(() =>
      canonicalizeTemplateImportSemanticPlan(cyclic),
    ).toThrow('cycle')
    expect(() =>
      canonicalizeTemplateImportSemanticPlan({ value: undefined }),
    ).toThrow('unsupported value')
    await expect(
      computeTemplateImportSemanticCommitment({
        listKey: new Uint8Array(31),
        plan: {},
      }),
    ).rejects.toThrow('exactly 32 bytes')
  })
})
