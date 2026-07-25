import { decode as cborDecode, encode as cborEncode } from 'cbor-x'

import { toUint8Array } from '../runtime/bytes'
import { SEALED_PAYLOAD_VERSION } from '../runtime/constants'
import {
  getStrongBoxBridge,
  type StrongBoxBridge,
} from '../runtime/strong-box'
import { computePayloadProof } from './work-list'
import { parseSealedPayload } from './sealed-payload'
import { toSealedBlob } from './sealed-blob'

const encoder = new TextEncoder()
const AUDIT_PAYLOAD_CONTEXT = encoder.encode('audit-patch')

export type ChecklistToggle = {
  title: string
  changeType: 'toggled'
  nowDone: boolean
}

export type ChecklistAddition = {
  title: string
  changeType: 'added'
}

export type ChecklistRemoval = {
  title: string
  changeType: 'removed'
}

export type ChecklistChange =
  | ChecklistToggle
  | ChecklistAddition
  | ChecklistRemoval

export type PriorityChangeContext = {
  taskTitle: string
  oldPriority: number | null
  newPriority: number | null
}

export type SectionMoveContext = {
  taskTitle: string
  fromSectionName: string | null
  toSectionName: string | null
}

export type DueDateChangeContext = {
  taskTitle: string
  oldDueAt: string | null
  newDueAt: string | null
}

export type ArchiveContext = { taskTitle: string }
export type ChecklistContext = { changes: ChecklistChange[] }
export type DetailsChangeContext = { taskTitle: string }
export type CommentCreateContext = { taskTitle: string }
export type CommentUpdateContext = { taskTitle: string }
export type CommentDeleteContext = { taskTitle: string }
export type RecurrenceCreateContext = { titleTemplate: string }
export type RecurrenceUpdateContext = { titleTemplate: string }
export type RecurrenceDeleteContext = { titleTemplate: string }
export type MembershipCreateContext = {
  memberName: string
  role: string
}
export type MembershipUpdateContext = {
  memberName: string
  oldRole: string
  newRole: string
}
export type MembershipDeleteContext = { memberName: string }
export type WorkListCreateContext = { workListTitle: string }
export type WorkListUpdateContext = { workListTitle: string }
export type WorkListLifecycleContext = { workListTitle: string }
export type WorkListDeleteContext = { workListTitle: string }
export type WorkListSectionsSyncedContext = { workListTitle: string }
export type WorkListTaskMovedContext = {
  taskTitle: string
  fromSectionName: string | null
  toSectionName: string | null
}
export type WorkListSectionSortedContext = {
  sectionName: string | null
}
export type TaskCreateContext = { taskTitle: string }
export type TaskDeleteContext = { taskTitle: string }
export type NoteCreateContext = {
  noteTitle: string
  isPrivate: boolean
}
export type NoteUpdateContext = { noteTitle: string }
export type NoteDeleteContext = { noteTitle: string }

export type AuditOperation =
  | { type: 'task.created'; context: TaskCreateContext }
  | { type: 'task.deleted'; context: TaskDeleteContext }
  | { type: 'task.priority'; context: PriorityChangeContext }
  | { type: 'task.moved'; context: SectionMoveContext }
  | { type: 'task.dueDate'; context: DueDateChangeContext }
  | { type: 'task.archived'; context: ArchiveContext }
  | { type: 'task.unarchived'; context: ArchiveContext }
  | { type: 'task.details'; context: DetailsChangeContext }
  | { type: 'checklist.toggled'; context: ChecklistContext }
  | { type: 'comment.created'; context: CommentCreateContext }
  | { type: 'comment.updated'; context: CommentUpdateContext }
  | { type: 'comment.deleted'; context: CommentDeleteContext }
  | { type: 'recurrence.created'; context: RecurrenceCreateContext }
  | { type: 'recurrence.updated'; context: RecurrenceUpdateContext }
  | { type: 'recurrence.deleted'; context: RecurrenceDeleteContext }
  | { type: 'membership.created'; context: MembershipCreateContext }
  | { type: 'membership.updated'; context: MembershipUpdateContext }
  | { type: 'membership.deleted'; context: MembershipDeleteContext }
  | { type: 'work_list.created'; context: WorkListCreateContext }
  | { type: 'work_list.updated'; context: WorkListUpdateContext }
  | { type: 'work_list.archived'; context: WorkListLifecycleContext }
  | { type: 'work_list.unarchived'; context: WorkListLifecycleContext }
  | { type: 'work_list.deleted'; context: WorkListDeleteContext }
  | {
      type: 'work_list.sections_synced'
      context: WorkListSectionsSyncedContext
    }
  | { type: 'work_list.task_moved'; context: WorkListTaskMovedContext }
  | {
      type: 'work_list.section_sorted'
      context: WorkListSectionSortedContext
    }
  | { type: 'note.created'; context: NoteCreateContext }
  | { type: 'note.updated'; context: NoteUpdateContext }
  | { type: 'note.deleted'; context: NoteDeleteContext }

export const AUDIT_KINDS = {
  'task.created': 'audit.task_created',
  'task.deleted': 'audit.task_deleted',
  'task.priority': 'audit.priority',
  'task.moved': 'audit.section_move',
  'task.dueDate': 'audit.due_date',
  'task.archived': 'audit.archived',
  'task.unarchived': 'audit.unarchived',
  'task.details': 'audit.details',
  'checklist.toggled': 'audit.checklist',
  'comment.created': 'audit.comment_created',
  'comment.updated': 'audit.comment_updated',
  'comment.deleted': 'audit.comment_deleted',
  'recurrence.created': 'audit.recurrence_created',
  'recurrence.updated': 'audit.recurrence_updated',
  'recurrence.deleted': 'audit.recurrence_deleted',
  'membership.created': 'audit.membership_created',
  'membership.updated': 'audit.membership_updated',
  'membership.deleted': 'audit.membership_deleted',
  'work_list.created': 'audit.work_list_created',
  'work_list.updated': 'audit.work_list_updated',
  'work_list.archived': 'audit.work_list_updated',
  'work_list.unarchived': 'audit.work_list_updated',
  'work_list.deleted': 'audit.work_list_deleted',
  'work_list.sections_synced': 'audit.work_list_sections_synced',
  'work_list.task_moved': 'audit.work_list_task_moved',
  'work_list.section_sorted': 'audit.work_list_section_sorted',
  'note.created': 'audit.note_created',
  'note.updated': 'audit.note_updated',
  'note.deleted': 'audit.note_deleted',
} as const

export type AuditNarrativePrimitive =
  | string
  | number
  | boolean
  | null

export type AuditNarrativeDescriptor = {
  key: string
  options?: Record<string, AuditNarrativeValue>
}

export type AuditNarrativeDateValue = {
  dateIso: string
}

export type AuditNarrativeValue =
  | AuditNarrativePrimitive
  | AuditNarrativeDateValue
  | AuditNarrativeDescriptor
  | AuditNarrativeDescriptor[]

export type AuditEnvelope = {
  kind: string
  version: number
  body: {
    narrative?: string
    narrativeKey?: string
    narrativeOptions?: Record<string, AuditNarrativeValue>
  }
}

export type AuditPatchResult = {
  fields: Array<{ field: string; changeKind: string }>
  payloadCiphertext: string
  payloadCiphertextProof: string
  payloadVersion: number
}

export type AuditEncryptionParams = {
  listKey: Uint8Array
  bindingKey: Uint8Array
  strongBox?: StrongBoxBridge
}

export type AuditPatchSemantics = {
  fields: Array<{ field: string; changeKind: string }>
  envelope: AuditEnvelope
  payloadVersion: number
}

export type AuditPayloadBody = Record<string, unknown> & {
  narrativeKey?: string | null
  narrativeOptions?: Record<string, AuditNarrativeValue> | null
  narrative?: string | null
  summary?: string | null
  message?: string | null
  description?: string | null
  text?: string | null
}

export type AuditPayloadEnvelope = {
  kind: string
  version: number
  body: AuditPayloadBody
}

export function buildNarrativeDescriptor(
  operation: AuditOperation,
): AuditNarrativeDescriptor {
  switch (operation.type) {
    case 'task.created':
      return descriptor('features.audit.narratives.taskCreated', {
        title: operation.context.taskTitle,
      })
    case 'task.deleted':
      return descriptor('features.audit.narratives.taskDeleted', {
        title: operation.context.taskTitle,
      })
    case 'task.priority':
      return buildPriorityNarrativeDescriptor(operation.context)
    case 'task.moved':
      return buildMoveNarrativeDescriptor(operation.context)
    case 'task.dueDate':
      return buildDueDateNarrativeDescriptor(operation.context)
    case 'task.archived':
      return descriptor('features.audit.narratives.taskArchived', {
        title: operation.context.taskTitle,
      })
    case 'task.unarchived':
      return descriptor('features.audit.narratives.taskUnarchived', {
        title: operation.context.taskTitle,
      })
    case 'task.details':
      return descriptor('features.audit.narratives.taskDetails', {
        title: operation.context.taskTitle,
      })
    case 'checklist.toggled':
      return buildChecklistNarrativeDescriptor(
        operation.context.changes,
      )
    case 'comment.created':
      return descriptor('features.audit.narratives.commentCreated', {
        title: operation.context.taskTitle,
      })
    case 'comment.updated':
      return descriptor('features.audit.narratives.commentUpdated', {
        title: operation.context.taskTitle,
      })
    case 'comment.deleted':
      return descriptor('features.audit.narratives.commentDeleted', {
        title: operation.context.taskTitle,
      })
    case 'recurrence.created':
      return descriptor('features.audit.narratives.recurrenceCreated', {
        title: operation.context.titleTemplate,
      })
    case 'recurrence.updated':
      return descriptor('features.audit.narratives.recurrenceUpdated', {
        title: operation.context.titleTemplate,
      })
    case 'recurrence.deleted':
      return descriptor('features.audit.narratives.recurrenceDeleted', {
        title: operation.context.titleTemplate,
      })
    case 'membership.created':
      return descriptor('features.audit.narratives.membershipCreated', {
        member: operation.context.memberName,
        role: roleDescriptor(operation.context.role),
      })
    case 'membership.updated':
      return descriptor('features.audit.narratives.membershipUpdated', {
        member: operation.context.memberName,
        oldRole: roleDescriptor(operation.context.oldRole),
        newRole: roleDescriptor(operation.context.newRole),
      })
    case 'membership.deleted':
      return descriptor('features.audit.narratives.membershipDeleted', {
        member: operation.context.memberName,
      })
    case 'work_list.created':
      return descriptor('features.audit.narratives.workListCreated', {
        title: operation.context.workListTitle,
      })
    case 'work_list.updated':
    case 'work_list.archived':
    case 'work_list.unarchived':
      return descriptor('features.audit.narratives.workListUpdated', {
        title: operation.context.workListTitle,
      })
    case 'work_list.deleted':
      return descriptor('features.audit.narratives.workListDeleted', {
        title: operation.context.workListTitle,
      })
    case 'work_list.sections_synced':
      return descriptor(
        'features.audit.narratives.workListSectionsSynced',
        { title: operation.context.workListTitle },
      )
    case 'work_list.task_moved':
      return buildMoveNarrativeDescriptor(operation.context)
    case 'work_list.section_sorted':
      return buildSectionSortedNarrativeDescriptor(
        operation.context.sectionName,
      )
    case 'note.created':
      return buildNoteCreatedNarrativeDescriptor(operation.context)
    case 'note.updated':
      return descriptor('features.audit.narratives.noteUpdated', {
        title: operation.context.noteTitle,
      })
    case 'note.deleted':
      return descriptor('features.audit.narratives.noteDeleted', {
        title: operation.context.noteTitle,
      })
  }
}

export function createAuditPatchSemantics(
  operation: AuditOperation,
): AuditPatchSemantics | null {
  if (!shouldGenerateAudit(operation)) return null

  const narrative = buildNarrativeDescriptor(operation)
  return {
    fields: buildFieldsArray(operation),
    envelope: {
      kind: AUDIT_KINDS[operation.type],
      version: 2,
      body: {
        narrativeKey: narrative.key,
        narrativeOptions: narrative.options,
      },
    },
    payloadVersion: 1,
  }
}

export async function encryptAuditEnvelope(params: {
  envelope: AuditEnvelope
  listKey: Uint8Array
  bindingKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<{
  payloadCiphertext: string
  payloadCiphertextProof: string
}> {
  const plaintext = toUint8Array(cborEncode(params.envelope))
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const ciphertext = await bridge.encrypt({
    key: params.listKey,
    context: AUDIT_PAYLOAD_CONTEXT,
    plaintext,
  })
  const sealed = toSealedBlob({
    version: SEALED_PAYLOAD_VERSION,
    ciphertext,
  })
  return {
    payloadCiphertext: sealed.base64,
    payloadCiphertextProof: await computePayloadProof({
      ciphertext: sealed.bytes,
      bindingKey: params.bindingKey,
    }),
  }
}

export async function encryptAuditPatchSemantics(
  semantics: AuditPatchSemantics,
  encryption: AuditEncryptionParams,
): Promise<AuditPatchResult> {
  const encrypted = await encryptAuditEnvelope({
    envelope: semantics.envelope,
    listKey: encryption.listKey,
    bindingKey: encryption.bindingKey,
    strongBox: encryption.strongBox,
  })
  return {
    fields: semantics.fields,
    ...encrypted,
    payloadVersion: semantics.payloadVersion,
  }
}

export async function buildAuditPatch(
  operation: AuditOperation,
  encryption: AuditEncryptionParams,
): Promise<AuditPatchResult | null> {
  const semantics = createAuditPatchSemantics(operation)
  return semantics
    ? encryptAuditPatchSemantics(semantics, encryption)
    : null
}

export async function decryptAuditPayload(params: {
  ciphertext: string
  listKey: Uint8Array
  strongBox?: StrongBoxBridge
}): Promise<AuditPayloadEnvelope> {
  const sealed = parseSealedPayload(params.ciphertext)
  const bridge = params.strongBox ?? (await getStrongBoxBridge())
  const plaintext = await bridge.decrypt({
    key: params.listKey,
    context: AUDIT_PAYLOAD_CONTEXT,
    ciphertext: sealed.ciphertext,
  })
  const envelope = cborDecode(
    plaintext,
  ) as Partial<AuditPayloadEnvelope> | null
  if (!envelope || typeof envelope !== 'object') {
    throw new Error('Invalid audit payload envelope')
  }
  if (typeof envelope.kind !== 'string' || envelope.kind.length === 0) {
    throw new Error('Audit payload kind must be a string')
  }
  if (
    typeof envelope.version !== 'number' ||
    !Number.isFinite(envelope.version)
  ) {
    throw new Error('Audit payload version must be numeric')
  }
  if (!envelope.body || typeof envelope.body !== 'object') {
    throw new Error('Audit payload body must be an object')
  }
  return {
    kind: envelope.kind,
    version: envelope.version,
    body: envelope.body,
  }
}

export function extractAuditNarrative(
  body: AuditPayloadBody | null | undefined,
  renderDescriptor?: (
    descriptor: AuditNarrativeDescriptor,
  ) => string | null | undefined,
): string | null {
  if (!body) return null

  if (
    renderDescriptor &&
    typeof body.narrativeKey === 'string' &&
    body.narrativeKey.trim().length > 0
  ) {
    const key = body.narrativeKey.trim()
    const rendered = renderDescriptor({
      key,
      options: normalizeNarrativeOptions(body.narrativeOptions),
    })
    const trimmed = typeof rendered === 'string' ? rendered.trim() : ''
    if (trimmed.length > 0 && trimmed !== key) return trimmed
  }

  const candidateKeys = [
    'narrative',
    'summary',
    'message',
    'description',
    'text',
  ] as const
  for (const key of candidateKeys) {
    const value = body[key]
    if (typeof value === 'string' && value.trim().length > 0) {
      return value.trim()
    }
  }
  return null
}

export function taskCreateAudit(
  context: TaskCreateContext,
): AuditOperation {
  return { type: 'task.created', context }
}

export function taskDeleteAudit(
  context: TaskDeleteContext,
): AuditOperation {
  return { type: 'task.deleted', context }
}

export function priorityChangeAudit(
  context: PriorityChangeContext,
): AuditOperation {
  return { type: 'task.priority', context }
}

export function sectionMoveAudit(
  context: SectionMoveContext,
): AuditOperation {
  return { type: 'task.moved', context }
}

export function dueDateChangeAudit(
  context: DueDateChangeContext,
): AuditOperation {
  return { type: 'task.dueDate', context }
}

export function archiveAudit(context: ArchiveContext): AuditOperation {
  return { type: 'task.archived', context }
}

export function unarchiveAudit(context: ArchiveContext): AuditOperation {
  return { type: 'task.unarchived', context }
}

export function detailsChangeAudit(
  context: DetailsChangeContext,
): AuditOperation {
  return { type: 'task.details', context }
}

export function commentCreateAudit(
  context: CommentCreateContext,
): AuditOperation {
  return { type: 'comment.created', context }
}

export function commentUpdateAudit(
  context: CommentUpdateContext,
): AuditOperation {
  return { type: 'comment.updated', context }
}

export function commentDeleteAudit(
  context: CommentDeleteContext,
): AuditOperation {
  return { type: 'comment.deleted', context }
}

export function checklistChangedAudit(
  changes: ChecklistChange[],
): AuditOperation {
  return { type: 'checklist.toggled', context: { changes } }
}

/** @deprecated Use checklistChangedAudit. */
export const checklistToggledAudit = checklistChangedAudit

export function sectionMoveAuditFromIds(params: {
  taskTitle: string
  fromSectionId: string | null
  toSectionId: string | null
  sections: Array<{ id: string; name: string }>
}): AuditOperation {
  const findName = (id: string | null) =>
    id
      ? (params.sections.find((section) => section.id === id)?.name ??
        null)
      : null
  return sectionMoveAudit({
    taskTitle: params.taskTitle,
    fromSectionName: findName(params.fromSectionId),
    toSectionName: findName(params.toSectionId),
  })
}

export function noteCreateAudit(
  context: NoteCreateContext,
): AuditOperation {
  return { type: 'note.created', context }
}

export function noteUpdateAudit(
  context: NoteUpdateContext,
): AuditOperation {
  return { type: 'note.updated', context }
}

export function noteDeleteAudit(
  context: NoteDeleteContext,
): AuditOperation {
  return { type: 'note.deleted', context }
}

function descriptor(
  key: string,
  options?: Record<string, AuditNarrativeValue>,
): AuditNarrativeDescriptor {
  return options ? { key, options } : { key }
}

function buildNoteCreatedNarrativeDescriptor(context: {
  noteTitle: string
  isPrivate: boolean
}): AuditNarrativeDescriptor {
  return descriptor(
    context.isPrivate
      ? 'features.audit.narratives.noteCreatedPrivate'
      : 'features.audit.narratives.noteCreated',
    { title: context.noteTitle },
  )
}

function buildPriorityNarrativeDescriptor(
  context: PriorityChangeContext,
): AuditNarrativeDescriptor {
  const oldLabel = priorityDescriptor(context.oldPriority)
  const newLabel = priorityDescriptor(context.newPriority)
  if (!oldLabel && newLabel) {
    return descriptor('features.audit.narratives.prioritySet', {
      title: context.taskTitle,
      priority: newLabel,
    })
  }
  if (oldLabel && !newLabel) {
    return descriptor('features.audit.narratives.priorityRemoved', {
      title: context.taskTitle,
    })
  }
  return descriptor('features.audit.narratives.priorityChanged', {
    title: context.taskTitle,
    oldPriority: oldLabel,
    newPriority: newLabel,
  })
}

function priorityDescriptor(
  value: number | null,
): AuditNarrativeDescriptor | null {
  if (value === null || Number.isNaN(value) || value < 1) return null
  if (value >= 8) {
    return descriptor('features.tasks.priority.urgent')
  }
  if (value >= 5) return descriptor('features.tasks.priority.high')
  if (value >= 3) return descriptor('features.tasks.priority.medium')
  return descriptor('features.tasks.priority.low')
}

function buildMoveNarrativeDescriptor(
  context: SectionMoveContext,
): AuditNarrativeDescriptor {
  const from =
    context.fromSectionName ??
    descriptor('features.audit.narratives.backlog')
  const to =
    context.toSectionName ??
    descriptor('features.audit.narratives.backlog')
  if (context.fromSectionName === context.toSectionName) {
    return descriptor('features.audit.narratives.taskReordered', {
      title: context.taskTitle,
      section: to,
    })
  }
  return descriptor('features.audit.narratives.taskMoved', {
    title: context.taskTitle,
    from,
    to,
  })
}

function buildDueDateNarrativeDescriptor(
  context: DueDateChangeContext,
): AuditNarrativeDescriptor {
  const oldDate = context.oldDueAt
    ? { dateIso: context.oldDueAt }
    : null
  const newDate = context.newDueAt
    ? { dateIso: context.newDueAt }
    : null
  if (!oldDate && newDate) {
    return descriptor('features.audit.narratives.dueDateSet', {
      title: context.taskTitle,
      date: newDate,
    })
  }
  if (oldDate && !newDate) {
    return descriptor('features.audit.narratives.dueDateRemoved', {
      title: context.taskTitle,
    })
  }
  return descriptor('features.audit.narratives.dueDateChanged', {
    title: context.taskTitle,
    oldDate,
    newDate,
  })
}

function buildChecklistNarrativeDescriptor(
  changes: ChecklistChange[],
): AuditNarrativeDescriptor {
  if (changes.length === 1) {
    const change = changes[0]
    switch (change.changeType) {
      case 'added':
        return descriptor(
          'features.audit.narratives.checklistItemAdded',
          { title: change.title },
        )
      case 'removed':
        return descriptor(
          'features.audit.narratives.checklistItemRemoved',
          { title: change.title },
        )
      case 'toggled':
        return descriptor(
          'features.audit.narratives.checklistItemToggled',
          {
            title: change.title,
            state: descriptor(
              change.nowDone
                ? 'features.audit.narratives.checklistDone'
                : 'features.audit.narratives.checklistNotDone',
            ),
          },
        )
    }
  }

  const added = changes.filter(
    (change) => change.changeType === 'added',
  )
  const removed = changes.filter(
    (change) => change.changeType === 'removed',
  )
  const toggled = changes.filter(
    (change): change is ChecklistToggle =>
      change.changeType === 'toggled',
  )
  const summary: AuditNarrativeDescriptor[] = []
  if (added.length > 0) {
    summary.push(
      descriptor('features.audit.narratives.checklistAddedCount', {
        count: added.length,
      }),
    )
  }
  if (removed.length > 0) {
    summary.push(
      descriptor('features.audit.narratives.checklistRemovedCount', {
        count: removed.length,
      }),
    )
  }
  if (toggled.length > 0) {
    const doneCount = toggled.filter((change) => change.nowDone).length
    const undoneCount = toggled.length - doneCount
    if (doneCount > 0 && undoneCount > 0) {
      summary.push(
        descriptor('features.audit.narratives.checklistMixedCount', {
          doneCount,
          undoneCount,
        }),
      )
    } else if (doneCount > 0) {
      summary.push(
        descriptor('features.audit.narratives.checklistDoneCount', {
          count: doneCount,
        }),
      )
    } else {
      summary.push(
        descriptor(
          'features.audit.narratives.checklistUncheckedCount',
          { count: undoneCount },
        ),
      )
    }
  }
  return descriptor('features.audit.narratives.checklistUpdated', {
    summary,
  })
}

const ROLE_LABEL_KEYS: Record<string, string> = {
  owner: 'features.audit.narratives.roleOwner',
  admin: 'features.audit.narratives.roleAdmin',
  editor: 'features.audit.narratives.roleEditor',
  viewer: 'features.audit.narratives.roleViewer',
  member: 'features.audit.narratives.roleEditor',
}

function roleDescriptor(role: string): AuditNarrativeValue {
  const key = ROLE_LABEL_KEYS[role.toLowerCase()]
  return key ? descriptor(key) : role
}

function buildSectionSortedNarrativeDescriptor(
  sectionName: string | null,
): AuditNarrativeDescriptor {
  return descriptor('features.audit.narratives.sectionSorted', {
    section:
      sectionName ?? descriptor('features.audit.narratives.backlog'),
  })
}

function normalizeNarrativeOptions(
  options: AuditPayloadBody['narrativeOptions'],
): AuditNarrativeDescriptor['options'] {
  return options && typeof options === 'object' ? options : undefined
}

function shouldGenerateAudit(operation: AuditOperation): boolean {
  switch (operation.type) {
    case 'task.priority':
      return operation.context.oldPriority !== operation.context.newPriority
    case 'task.dueDate':
      return operation.context.oldDueAt !== operation.context.newDueAt
    case 'checklist.toggled':
      return operation.context.changes.length > 0
    default:
      return true
  }
}

function buildFieldsArray(
  operation: AuditOperation,
): Array<{ field: string; changeKind: string }> {
  switch (operation.type) {
    case 'task.created':
      return [{ field: 'task', changeKind: 'create' }]
    case 'task.deleted':
      return [{ field: 'task', changeKind: 'delete' }]
    case 'task.priority':
      return [
        {
          field: 'priority',
          changeKind:
            operation.context.oldPriority === null
              ? 'set'
              : operation.context.newPriority === null
                ? 'clear'
                : 'update',
        },
      ]
    case 'task.moved':
    case 'work_list.task_moved':
      return [{ field: 'sectionId', changeKind: 'update' }]
    case 'task.dueDate':
      return [
        {
          field: 'dueAt',
          changeKind:
            operation.context.oldDueAt === null
              ? 'set'
              : operation.context.newDueAt === null
                ? 'clear'
                : 'update',
        },
      ]
    case 'task.archived':
    case 'work_list.archived':
      return [{ field: 'archivedAt', changeKind: 'set' }]
    case 'task.unarchived':
    case 'work_list.unarchived':
      return [{ field: 'archivedAt', changeKind: 'clear' }]
    case 'task.details':
      return [{ field: 'details', changeKind: 'update' }]
    case 'checklist.toggled':
      return [{ field: 'checklist', changeKind: 'update' }]
    case 'comment.created':
      return [{ field: 'comment', changeKind: 'create' }]
    case 'comment.updated':
      return [{ field: 'comment', changeKind: 'update' }]
    case 'comment.deleted':
      return [{ field: 'comment', changeKind: 'delete' }]
    case 'recurrence.created':
      return [{ field: 'recurrence', changeKind: 'create' }]
    case 'recurrence.updated':
      return [{ field: 'recurrence', changeKind: 'update' }]
    case 'recurrence.deleted':
      return [{ field: 'recurrence', changeKind: 'delete' }]
    case 'membership.created':
      return [{ field: 'membership', changeKind: 'create' }]
    case 'membership.updated':
      return [{ field: 'membership', changeKind: 'update' }]
    case 'membership.deleted':
      return [{ field: 'membership', changeKind: 'delete' }]
    case 'work_list.created':
      return [{ field: 'work_list', changeKind: 'create' }]
    case 'work_list.updated':
      return [{ field: 'work_list', changeKind: 'update' }]
    case 'work_list.deleted':
      return [{ field: 'work_list', changeKind: 'delete' }]
    case 'work_list.sections_synced':
      return [{ field: 'sections', changeKind: 'update' }]
    case 'work_list.section_sorted':
      return [{ field: 'taskOrder', changeKind: 'update' }]
    case 'note.created':
      return [{ field: 'note', changeKind: 'create' }]
    case 'note.updated':
      return [{ field: 'note', changeKind: 'update' }]
    case 'note.deleted':
      return [{ field: 'note', changeKind: 'delete' }]
  }
}
