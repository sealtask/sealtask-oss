import { encode as cborEncode } from 'cbor-x'

import { decodeAndValidatePayloadBytes, validatePayloadBytes } from './payload-validation'
import type { TaskPayloadEnvelope, TaskPayloadRichText } from './task'
import { buildTaskReferenceScheme, type TaskReferenceScheme } from './task-reference-scheme'
import type { WorkListPayloadEnvelope } from './work-list'

export type ProjectDuplicationSource = {
  workListId: string
  timezone: string
  envelope: WorkListPayloadEnvelope
  sectionSnapshots: { id: string; position: number }[]
  /** Complete server task-list response in server order, including archived tasks. */
  tasks: {
    id: string
    sectionId: string | null
    priority: number | null
    archivedAt: string | null
    /** Required for every non-archived task; callers must never skip failed decryption. */
    envelope: TaskPayloadEnvelope | null
  }[]
  referenceScheme: TaskReferenceScheme | null
}

export type ProjectDuplicationIds = {
  operationId: string
  workListId: string
  /** Includes disabled stage metadata IDs as well as active section IDs. */
  sections: Record<string, string>
  tasks: Record<string, string>
  checklists: Record<string, Record<string, string>>
  fallbackSectionId: string
  schemeRevisionId: string
}

export type ProjectDuplicationPlan = {
  protocolVersion: 1
  resetPolicy: 'incomplete-without-dates-v1'
  operationId: string
  workList: {
    workListId: string
    timezone: string
    envelope: WorkListPayloadEnvelope
    sectionSnapshots: {
      id: string
      position: number
      autoArchiveEnabled: false
      autoArchiveAfterDays: null
    }[]
    referenceScheme: TaskReferenceScheme | null
  }
  tasks: {
    taskId: string
    sectionId: string | null
    priority: number | null
    dueAt: null
    startAt: null
    completedAt: null
    envelope: TaskPayloadEnvelope
  }[]
}

/**
 * Strict, immutable v1 copy contract. Inputs are complete decrypted envelopes,
 * never display summaries. The result contains only destination data; ID maps
 * and source identity are deliberately absent from the committed plaintext.
 */
export function transformProjectDuplication(params: {
  source: ProjectDuplicationSource
  ids: ProjectDuplicationIds
  title: string
  fallbackSectionName: string
}): ProjectDuplicationPlan {
  const { source, ids } = params
  const sourceId = uuid(source.workListId)
  const body = validatedBody(source.envelope, 'work_list')
  onlyKeys(body, ['title', 'description', 'theme', 'sections', 'stage_metadata', 'client_meta'])
  const sections = array(body.sections).map(record)
  const stages = array(body.stage_metadata).map(record)
  const snapshots = [...source.sectionSnapshots].sort((a, b) => a.position - b.position)
  if (snapshots.length !== sections.length || snapshots.length > 32) fail('incomplete sections')
  const sectionIds = new Set(sections.map((section) => uuid(section.id)))
  if (sectionIds.size !== sections.length) fail('duplicate source section')
  const positions = new Set<number>()
  const snapshotIds = new Set<string>()
  for (const snapshot of snapshots) {
    const id = uuid(snapshot.id)
    if (!sectionIds.has(id) || snapshotIds.has(id)) fail('incomplete sections')
    if (!Number.isSafeInteger(snapshot.position) || snapshot.position < 0 || positions.has(snapshot.position)) fail('invalid section order')
    snapshotIds.add(id)
    positions.add(snapshot.position)
  }
  const allSourceIds = new Set<string>([sourceId, ...sectionIds])
  const stageIds = new Set<string>()
  for (const stage of stages) {
    const id = uuid(stage.id)
    if (stageIds.has(id)) fail('duplicate source stage')
    stageIds.add(id)
    allSourceIds.add(id)
  }
  const taskIds = new Set<string>()
  for (const task of source.tasks) {
    const id = uuid(task.id)
    if (taskIds.has(id) || allSourceIds.has(id)) fail('duplicate source task')
    taskIds.add(id)
    allSourceIds.add(id)
  }
  const included = source.tasks.filter((task) => task.archivedAt === null)
  if (included.length > 200) fail('more than 200 tasks')
  // Reject absent archive state instead of treating an incomplete response as archived.
  for (const task of source.tasks) {
    if (task.archivedAt !== null && (typeof task.archivedAt !== 'string' || !Number.isFinite(Date.parse(task.archivedAt)))) fail('missing task archive state')
  }
  const taskBodies = included.map((task) => {
    const taskBody = validatedBody(task.envelope, 'task')
    for (const item of array(taskBody.checklist)) allSourceIds.add(uuid(record(item).id))
    return taskBody
  })
  if (source.referenceScheme) allSourceIds.add(uuid(source.referenceScheme.schemeRevisionId))
  const destinationIds = new Set<string>()
  const fresh = (value: unknown) => {
    const id = uuid(value)
    if (allSourceIds.has(id) || destinationIds.has(id)) fail('destination identifiers must be fresh and unique')
    destinationIds.add(id)
    return id
  }
  const operationId = fresh(ids.operationId)
  if (operationId[14] !== '7') fail('operation identifier must be UUIDv7')
  const workListId = fresh(ids.workListId)
  const sectionMap = new Map<string, string>()
  for (const id of new Set([...sectionIds, ...stageIds])) {
    sectionMap.set(id, fresh(mapped(ids.sections, id)))
  }
  const mappedSection = (value: unknown) => {
    const id = sectionMap.get(uuid(value))
    if (!id) fail('unknown section reference')
    return id
  }
  const copiedSections = snapshots.map((snapshot) => {
    const section = sections.find((entry) => uuid(entry.id) === uuid(snapshot.id))!
    onlyKeys(section, ['id', 'name', 'wip_limit', 'description'])
    return {
      id: mappedSection(section.id),
      name: string(section.name),
      wip_limit: section.wip_limit ?? null,
      ...(section.description === undefined ? {} : { description: string(section.description) }),
    }
  })
  if (copiedSections.length === 0) {
    copiedSections.push({ id: fresh(ids.fallbackSectionId), name: string(params.fallbackSectionName), wip_limit: null })
  }
  const copiedStages = stages.map((stage) => {
    onlyKeys(stage, ['id', 'label', 'description', 'enabled', 'is_custom', 'order'])
    const result: Record<string, unknown> = { id: mappedSection(stage.id), label: string(stage.label) }
    if (stage.description !== undefined) result.description = string(stage.description)
    for (const key of ['enabled', 'is_custom']) {
      if (stage[key] !== undefined) {
        if (typeof stage[key] !== 'boolean') fail('invalid stage flag')
        result[key] = stage[key]
      }
    }
    if (stage.order !== undefined) {
      if (!Number.isSafeInteger(stage.order) || Number(stage.order) < 0) fail('invalid stage order')
      result.order = stage.order
    }
    return result
  })
  const clientMeta = body.client_meta == null ? {} : record(body.client_meta)
  const view = clientMeta['web.view'] == null ? {} : record(clientMeta['web.view'])
  const layout = view.layout ?? 'kanban'
  if (layout !== 'kanban' && layout !== 'list') fail('unsupported project layout')
  const activeSection = (value: unknown) => {
    if (!sectionIds.has(uuid(value))) fail('unknown active section reference')
    return mappedSection(value)
  }
  const defaultSection = view.default_section === null
    ? null
    : view.default_section === undefined || view.default_section === ''
      ? copiedSections[0].id : activeSection(view.default_section)
  const activeIds = view.active_stage_ids === undefined
    ? copiedSections.map((section) => section.id)
    : array(view.active_stage_ids).map(activeSection)
  if (new Set(activeIds).size !== activeIds.length) fail('duplicate active section')
  const copiedBody: Record<string, unknown> = {
    title: string(params.title).trim(),
    description: body.description ?? null,
    theme: body.theme == null ? null : copyTheme(record(body.theme)),
    sections: copiedSections,
    stage_metadata: copiedStages,
    client_meta: {
      'web.view': { layout, default_section: defaultSection, active_stage_ids: activeIds },
    },
  }
  const envelope: WorkListPayloadEnvelope = { kind: 'work_list', version: 1, body: copiedBody }
  validatePayloadBytes(cborEncode(envelope), 'work_list')
  const tasks = included.map((task, index) => {
    const taskBody = taskBodies[index]
    onlyKeys(taskBody, ['title', 'rich_text', 'checklist', 'attachments', 'references', 'mentions', 'client_meta', 'recurrence_state'])
    if (task.priority !== null && ![1, 3, 5, 8].includes(task.priority)) fail('invalid task priority')
    if (task.sectionId !== null && !sectionIds.has(uuid(task.sectionId))) fail('unknown task section')
    const checklistIds = new Set<string>()
    const checklist = array(taskBody.checklist).map((value) => {
      const item = record(value)
      onlyKeys(item, ['id', 'title', 'is_done', 'completed_at', 'assignee_user_ids'])
      const id = uuid(item.id)
      if (checklistIds.has(id)) fail('duplicate checklist item')
      checklistIds.add(id)
      return {
        id: fresh(mapped(record(mapped(ids.checklists, uuid(task.id))), id)),
        title: string(item.title), is_done: false, completed_at: null, assignee_user_ids: [],
      }
    })
    const taskEnvelope: TaskPayloadEnvelope = {
      kind: 'task', version: 1,
      body: {
        title: string(taskBody.title),
        rich_text: copyRichText(taskBody.rich_text),
        checklist, attachments: [], references: [], mentions: [], client_meta: {}, recurrence_state: null,
      },
    }
    validatePayloadBytes(cborEncode(taskEnvelope), 'task')
    return {
      taskId: fresh(mapped(ids.tasks, uuid(task.id))),
      sectionId: task.sectionId === null ? null : mappedSection(task.sectionId),
      priority: task.priority, dueAt: null, startAt: null, completedAt: null, envelope: taskEnvelope,
    }
  })
  let referenceScheme: TaskReferenceScheme | null = null
  if (source.referenceScheme) {
    const scheme = source.referenceScheme
    if (scheme.kind !== 'task_reference_scheme' || scheme.version !== 1 || uuid(scheme.workListId) !== sourceId || scheme.separator !== '-') fail('unsupported reference scheme')
    buildTaskReferenceScheme(scheme)
    referenceScheme = buildTaskReferenceScheme({
      workListId, schemeRevisionId: fresh(ids.schemeRevisionId), revision: 1,
      prefix: scheme.prefix, minimumDigits: scheme.minimumDigits,
    })
  }
  if (!source.timezone || typeof source.timezone !== 'string') fail('missing timezone')
  return {
    protocolVersion: 1, resetPolicy: 'incomplete-without-dates-v1', operationId,
    workList: {
      workListId, timezone: source.timezone, envelope,
      sectionSnapshots: copiedSections.map((section, position) => ({
        id: section.id, position, autoArchiveEnabled: false, autoArchiveAfterDays: null,
      })),
      referenceScheme,
    },
    tasks,
  }
}

function copyTheme(theme: Record<string, unknown>): Record<string, unknown> {
  onlyKeys(theme, ['color', 'emoji'])
  return { color: theme.color, emoji: theme.emoji ?? null }
}

function copyRichText(value: unknown): TaskPayloadRichText | null {
  if (value == null) return null
  const document = record(value)
  onlyKeys(document, ['format', 'version', 'blocks'])
  if (document.version !== 1 || !['plaintext', 'markdown', 'prosemirror'].includes(String(document.format))) fail('unsupported rich text')
  const blocks = array(document.blocks).map((value) => {
    const block = record(value)
    onlyKeys(block, ['type', 'text', 'content', 'attrs'])
    const result: Record<string, unknown> = { type: block.type, text: string(block.text) }
    if (block.attrs !== undefined) {
      const attrs = record(block.attrs)
      // Only presentation attributes are copied; embedded identity attributes are removed.
      onlyKeys(attrs, ['level', 'language', 'order', 'start', 'indent', 'checked', 'id', 'user_id', 'membership_id'])
      const copied: Record<string, unknown> = {}
      for (const key of ['level', 'order', 'start', 'indent']) {
        if (attrs[key] !== undefined) {
          if (!Number.isSafeInteger(attrs[key]) || Number(attrs[key]) < 0) fail('unsupported rich text attribute')
          copied[key] = attrs[key]
        }
      }
      if (attrs.language !== undefined && attrs.language !== null) copied.language = string(attrs.language)
      if (attrs.checked !== undefined) {
        if (typeof attrs.checked !== 'boolean') fail('unsupported rich text attribute')
        copied.checked = false
      }
      result.attrs = copied
    }
    if (block.content !== undefined) {
      result.content = array(block.content).map((value) => {
        const span = record(value)
        onlyKeys(span, ['text', 'marks'])
        return { text: string(span.text), marks: array(span.marks).flatMap((value) => {
          const mark = record(value)
          onlyKeys(mark, ['type', 'attrs'])
          if (mark.type === 'mention') return []
          if (mark.type === 'link') return [{ type: 'link', attrs: { href: string(record(mark.attrs).href) } }]
          if (!['bold', 'italic', 'strike', 'code'].includes(String(mark.type))) fail('unsupported rich text mark')
          return [{ type: mark.type }]
        }) }
      })
    }
    return result
  })
  return { format: document.format, version: 1, blocks } as TaskPayloadRichText
}

function validatedBody(value: unknown, kind: 'work_list' | 'task'): Record<string, unknown> {
  const envelope = record(value)
  onlyKeys(envelope, ['kind', 'version', 'body'])
  if (envelope.kind !== kind || envelope.version !== 1) fail('unsupported payload version')
  const validated = decodeAndValidatePayloadBytes(cborEncode(envelope), kind)
  return record(validated.body)
}

function mapped(map: Record<string, unknown>, id: string): unknown {
  if (!Object.hasOwn(map, id)) fail('missing destination ID mapping')
  return map[id]
}

function record(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) fail('expected complete object')
  return value as Record<string, unknown>
}

function array(value: unknown): unknown[] {
  if (value === undefined || value === null) return []
  if (!Array.isArray(value)) fail('expected array')
  return value
}

function string(value: unknown): string {
  if (typeof value !== 'string') fail('expected string')
  return value
}

function uuid(value: unknown): string {
  const id = string(value).toLowerCase()
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id)) fail('invalid UUID')
  return id
}

function onlyKeys(value: Record<string, unknown>, keys: string[]): void {
  if (Object.keys(value).some((key) => !keys.includes(key))) fail('unsupported structural content')
}

function fail(reason: string): never {
  // Do not include source plaintext or identifiers in errors/logs.
  throw new Error(`Project duplication cannot continue: ${reason}`)
}
