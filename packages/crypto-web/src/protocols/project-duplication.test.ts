import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

import type { StrongBoxBridge } from '../runtime/strong-box'
import { decryptTaskPayload, encryptTaskPayload } from './task'
import { encode as cborEncode } from 'cbor-x'
import { decodeAndValidatePayloadBytes } from './payload-validation'
import { computeTemplateImportSemanticCommitment } from './template-import-commitment'
import { decryptWorkListPayload, deriveWorkListKey, encryptWorkListPayload } from './work-list'
import { transformProjectDuplication, type ProjectDuplicationIds, type ProjectDuplicationPlan, type ProjectDuplicationSource } from './project-duplication'
import { canonicalizeProjectDuplicationPlan, computeProjectDuplicationCommitment } from './project-duplication-commitment'

type Fixture = {
  source: ProjectDuplicationSource
  ids: ProjectDuplicationIds
  title: string
  fallbackSectionName: string
  expected: ProjectDuplicationPlan
  listKeyHex: string
  canonical: string
  commitment: string
}
const fixtures = ['custom-project-v1', 'empty-project-v1'].map((name) => JSON.parse(
  readFileSync(new URL(`../../test/fixtures/project-duplication/${name}.json`, import.meta.url), 'utf8'),
) as Fixture)
const uuid = (n: number) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`
const clone = () => structuredClone(fixtures[0])

describe('project duplication v1', () => {
  for (const fixture of fixtures) {
    it(`matches the shared transform and commitment: ${fixture.title}`, async () => {
      const original = structuredClone(fixture)
      const result = transformProjectDuplication(fixture)
      expect(result).toEqual(fixture.expected)
      expect(fixture).toEqual(original)
      expect(canonicalizeProjectDuplicationPlan(result)).toBe(fixture.canonical)
      const listKey = new Uint8Array(Buffer.from(fixture.listKeyHex, 'hex'))
      const commitment = await computeProjectDuplicationCommitment({ listKey, plan: result })
      expect(commitment).toBe(fixture.commitment)
      expect(commitment).not.toBe(await computeTemplateImportSemanticCommitment({ listKey, plan: result }))
    })
  }

  it('excludes identities, archived tasks, schedules, progress, and unknown client metadata', () => {
    const plan = transformProjectDuplication(clone())
    expect(plan.tasks).toHaveLength(2)
    expect(JSON.stringify(plan)).not.toContain(uuid(20))
    expect(JSON.stringify(plan)).not.toContain(uuid(21))
    expect(JSON.stringify(plan)).not.toContain('excluded-source-membership-envelope')
    expect(JSON.stringify(plan)).not.toContain(uuid(8))
    expect(JSON.stringify(plan)).not.toContain('private.unknown')
    expect(plan.tasks[0].envelope.body.checklist?.[0].is_done).toBe(false)
    expect(plan.tasks[1].sectionId).toBeNull()
    expect(plan.tasks[0].envelope.body.rich_text?.blocks[0].text).toContain('@Zoë')
    expect(JSON.stringify(plan)).toContain('https://example.org/original?q=🦭')
    expect(plan.workList.sectionSnapshots.every((section) => !section.autoArchiveEnabled)).toBe(true)
    expect(plan.workList.referenceScheme?.revision).toBe(1)
  })

  it('preserves server task order and allocates fresh IDs for every checklist', () => {
    const fixture = clone()
    fixture.source.tasks.reverse()
    expect(transformProjectDuplication(fixture).tasks.map((task) => task.taskId)).toEqual([uuid(105), uuid(104)])
  })

  it('preserves an explicit unsectioned default', () => {
    const fixture = clone()
    const meta = fixture.source.envelope.body.client_meta as Record<string, Record<string, unknown>>
    meta['web.view'].default_section = null
    const plan = transformProjectDuplication(fixture)
    expect(plan.workList.envelope.body.client_meta).toMatchObject({ 'web.view': { default_section: null } })
  })

  it('supports disabled stage settings without turning them into active sections', () => {
    const fixture = clone()
    const stages = fixture.source.envelope.body.stage_metadata as Record<string, unknown>[]
    stages.push({ id: uuid(30), label: 'Later', enabled: false, order: 2 })
    fixture.ids.sections[uuid(30)] = uuid(130)
    const plan = transformProjectDuplication(fixture)
    expect(plan.workList.sectionSnapshots).toHaveLength(2)
    expect(plan.workList.envelope.body.stage_metadata).toContainEqual({ id: uuid(130), label: 'Later', enabled: false, order: 2 })
  })

  const invalidCases: [string, (fixture: Fixture) => void][] = [
    ['invalid archive state', (f) => { f.source.tasks[0].archivedAt = '' }],
    ['missing task decryption', (f) => { f.source.tasks[0].envelope = null }],
    ['unsupported project version', (f) => { f.source.envelope.version = 2 }],
    ['unsupported task version', (f) => { f.source.tasks[0].envelope!.version = 2 }],
    ['non-v7 operation', (f) => { f.ids.operationId = uuid(800) }],
    ['unknown project content', (f) => { f.source.envelope.body.secret_structure = {} }],
    ['unknown task content', (f) => { Object.assign(f.source.tasks[0].envelope!.body, { unknown_structure: {} }) }],
    ['missing source section', (f) => { f.source.sectionSnapshots.pop() }],
    ['duplicate snapshot', (f) => { f.source.sectionSnapshots[0] = f.source.sectionSnapshots[1] }],
    ['dangling task section', (f) => { f.source.tasks[0].sectionId = uuid(999) }],
    ['duplicate task', (f) => { f.source.tasks.push(f.source.tasks[0]) }],
    ['missing destination section map', (f) => { delete f.ids.sections[uuid(2)] }],
    ['missing destination checklist map', (f) => { delete f.ids.checklists[uuid(4)][uuid(7)] }],
    ['reused source identifier', (f) => { f.ids.tasks[uuid(4)] = uuid(7) }],
    ['duplicate destination identifier', (f) => { f.ids.workListId = f.ids.operationId }],
    ['unknown reference version', (f) => { Object.assign(f.source.referenceScheme!, { version: 2 }) }],
    ['foreign reference project', (f) => { f.source.referenceScheme!.workListId = uuid(999) }],
    ['unsupported layout', (f) => { (f.source.envelope.body.client_meta as Record<string, unknown>)['web.view'] = { layout: 'timeline' } }],
    ['unknown rich text version', (f) => { f.source.tasks[0].envelope!.body.rich_text!.version = 2 }],
    ['blank copy title', (f) => { f.title = '   ' }],
  ]
  it.each(invalidCases)('rejects %s before producing a graph', (_label, mutate) => {
    const fixture = clone()
    mutate(fixture)
    expect(() => transformProjectDuplication(fixture)).toThrow()
  })

  it('accepts 200 included tasks and rejects 201 without truncation', () => {
    const fixture = clone()
    fixture.source.tasks = Array.from({ length: 200 }, (_, index) => ({
      id: uuid(1000 + index), sectionId: null, priority: null, archivedAt: null,
      envelope: { kind: 'task', version: 1, body: { title: `Task ${index}` } },
    }))
    for (let index = 0; index < 201; index++) fixture.ids.tasks[uuid(1000 + index)] = uuid(2000 + index)
    expect(transformProjectDuplication(fixture).tasks).toHaveLength(200)
    fixture.source.tasks.push({ ...fixture.source.tasks[0], id: uuid(1200) })
    expect(() => transformProjectDuplication(fixture)).toThrow('200')
  })

  it.each([1, 3, 5, 8, null])('preserves native priority %s', (priority) => {
    const fixture = clone()
    fixture.source.tasks[0].priority = priority
    expect(transformProjectDuplication(fixture).tasks[0].priority).toBe(priority)
  })

  it.each([0, 2, 4, 6])('rejects unsupported priority %s', (priority) => {
    const fixture = clone()
    fixture.source.tasks[0].priority = priority
    expect(() => transformProjectDuplication(fixture)).toThrow('priority')
  })

  it('requires portable integer and Unicode canonical values', () => {
    for (const value of [NaN, Infinity, 1.5, Number.MAX_SAFE_INTEGER + 1, '\ud800', { '\udfff': true }]) {
      expect(() => canonicalizeProjectDuplicationPlan(value)).toThrow()
    }
    expect(canonicalizeProjectDuplicationPlan({ text: '🦭/é/e\u0301/東京', value: -0 })).toBe('{"text":"🦭/é/é/東京","value":0}')
  })

  it('binds actual encrypted destination envelopes to a fresh project key', async () => {
    // A real AEAD bridge tests key/context binding independently of the WASM runtime.
    const bridge: StrongBoxBridge = {
      async encrypt({ key, context, plaintext }) {
        const nonce = crypto.getRandomValues(new Uint8Array(12))
        const imported = await crypto.subtle.importKey('raw', new Uint8Array(key), 'AES-GCM', false, ['encrypt'])
        const sealed = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv: nonce, additionalData: new Uint8Array(context) }, imported, new Uint8Array(plaintext)))
        return new Uint8Array([...nonce, ...sealed])
      },
      async decrypt({ key, context, ciphertext }) {
        const imported = await crypto.subtle.importKey('raw', new Uint8Array(key), 'AES-GCM', false, ['decrypt'])
        return new Uint8Array(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: new Uint8Array(ciphertext.slice(0, 12)), additionalData: new Uint8Array(context) }, imported, new Uint8Array(ciphertext.slice(12))))
      },
    }
    const fixture = clone()
    const plan = transformProjectDuplication(fixture)
    const dataKey = new Uint8Array(32).fill(42)
    const sourceKey = await deriveWorkListKey({ dataKey, workListId: fixture.source.workListId })
    const destinationKey = await deriveWorkListKey({ dataKey, workListId: plan.workList.workListId })
    expect(destinationKey).not.toEqual(sourceKey)
    const project = await encryptWorkListPayload({ envelope: plan.workList.envelope, listKey: destinationKey, strongBox: bridge })
    await expect(decryptWorkListPayload({ ciphertext: project.base64, listKey: sourceKey, strongBox: bridge })).rejects.toThrow()
    await expect(decryptWorkListPayload({ ciphertext: project.base64, listKey: destinationKey, strongBox: bridge })).resolves.toMatchObject(plan.workList.envelope)
    const task = await encryptTaskPayload({ envelope: plan.tasks[0].envelope, listKey: destinationKey, strongBox: bridge })
    await expect(decryptTaskPayload({ ciphertext: task.base64, listKey: sourceKey, strongBox: bridge })).rejects.toThrow()
    await expect(decryptTaskPayload({ ciphertext: task.base64, listKey: destinationKey, strongBox: bridge })).resolves.toEqual(decodeAndValidatePayloadBytes(cborEncode(plan.tasks[0].envelope), 'task'))
  })
})
