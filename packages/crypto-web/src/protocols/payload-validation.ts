import { decode as cborDecode } from 'cbor-x'

import { toArrayBuffer, toUint8Array, type ByteSource } from '../runtime/bytes'
import {
  PROJECT_EMOJI_POLICY,
  type ProjectEmojiPolicy,
} from './project-emoji'
import { normalizeRichTextHref } from './rich-text-url'

export type PayloadKind =
  | 'work_list'
  | 'task'
  | 'comment'
  | 'recurrence-title'
  | 'recurrence-body'
  | 'note'

export type ValidatedPayloadEnvelope<
  Kind extends PayloadKind = PayloadKind,
> = {
  kind: Kind
  version: number
  body: unknown
}

export class PayloadValidationError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'PayloadValidationError'
    Object.setPrototypeOf(this, PayloadValidationError.prototype)
  }
}

export type { ProjectEmojiPolicy } from './project-emoji'

export type PayloadValidationDependencies = {
  projectEmoji?: ProjectEmojiPolicy
  normalizeRichTextHref?: (value: unknown) => string | null
}

const WORK_LIST_SCHEMA_VERSION = 1
const TASK_SCHEMA_VERSION = 1
const COMMENT_SCHEMA_VERSION = 1
const RECURRENCE_TITLE_SCHEMA_VERSION = 1
const RECURRENCE_BODY_SCHEMA_VERSION = 1
const NOTE_SCHEMA_VERSION = 1

const MAX_TITLE_LEN = 256
const MAX_DESCRIPTION_LEN = 2048
const MAX_SECTION_COUNT = 32
const MAX_CHECKLIST_ITEMS = 200
const MAX_ATTACHMENTS = 50
const MAX_REFERENCES = 50
const MAX_MENTIONS = 50
const MAX_RICH_TEXT_BLOCKS = 500
const MAX_RICH_TEXT_SPANS = 8192
const MAX_RICH_TEXT_DOCUMENT_MARKS = 8192
const MAX_RICH_TEXT_TEXT_LEN = 8192
const MAX_ATTACHMENT_NAME_LEN = 255
const MAX_URI_LEN = 2048
const MAX_SECTION_NAME_LEN = 80
export const MAX_CHECKLIST_TITLE_LEN = 1024
const MAX_REFERENCE_LABEL_LEN = 128
const MAX_BLOB_KEY_LEN = 1024
const MAX_RICH_TEXT_MARKS = 16
const MAX_ASSIGNEES_PER_CHECKLIST = 16
export const MAX_ATTACHMENT_WIRE_READ_BYTES = 100 * 1024 * 1024

const RICH_TEXT_FORMATS = ['plaintext', 'markdown', 'prosemirror'] as const
const RICH_TEXT_BLOCK_TYPES = [
  'paragraph',
  'heading',
  'blockquote',
  'code_block',
  'bullet_item',
  'ordered_item',
  'list_item',
] as const
const LEGACY_BLOCK_MARK_TYPES = ['bold', 'italic', 'code', 'link'] as const
const TEXT_SPAN_MARK_TYPES = [
  'bold',
  'italic',
  'strike',
  'code',
  'link',
  'mention',
] as const

type WorkListValidationPurpose = 'legacy-read' | 'strict-write'
type RichTextValidationOptions = {
  field: string
  allowEmpty: boolean
}
type ValidationContext = {
  dependencies: Required<PayloadValidationDependencies>
  purpose: WorkListValidationPurpose
}

export function validatePayloadBytes(
  input: ByteSource,
  expected: PayloadKind,
  dependencies: PayloadValidationDependencies = {},
): void {
  decodeAndValidatePayloadBytesForPurpose(
    input,
    expected,
    'strict-write',
    dependencies,
  )
}

export function decodeAndValidatePayloadBytes<Kind extends PayloadKind>(
  input: ByteSource,
  expected: Kind,
  dependencies: PayloadValidationDependencies = {},
): ValidatedPayloadEnvelope<Kind> {
  return decodeAndValidatePayloadBytesForPurpose(
    input,
    expected,
    'legacy-read',
    dependencies,
  )
}

function decodeAndValidatePayloadBytesForPurpose<Kind extends PayloadKind>(
  input: ByteSource,
  expected: Kind,
  purpose: WorkListValidationPurpose,
  dependencies: PayloadValidationDependencies,
): ValidatedPayloadEnvelope<Kind> {
  const envelope = decodeEnvelope(toUint8Array(input))
  if (envelope.kind !== expected) {
    throw new PayloadValidationError(
      `payload kind mismatch: expected ${expected}, got ${String(envelope.kind)}`,
    )
  }
  const expectedVersion = schemaVersionFor(expected)
  if (
    typeof envelope.version !== 'number' ||
    !Number.isInteger(envelope.version) ||
    envelope.version !== expectedVersion
  ) {
    throw new PayloadValidationError(
      `${expected} payload version ${String(envelope.version)} is not supported (expected ${expectedVersion})`,
    )
  }

  const context: ValidationContext = {
    purpose,
    dependencies: {
      projectEmoji:
        dependencies.projectEmoji ?? PROJECT_EMOJI_POLICY,
      normalizeRichTextHref:
        dependencies.normalizeRichTextHref ?? normalizeRichTextHref,
    },
  }
  switch (expected) {
    case 'work_list':
      validateWorkListPayload(envelope.body, context)
      break
    case 'task':
      validateTaskPayload(envelope.body, context)
      break
    case 'comment':
      validateCommentPayload(envelope.body, context)
      break
    case 'note':
      validateNotePayload(envelope.body, context)
      break
    case 'recurrence-title':
      ensureString(
        toRecord(envelope.body, 'recurrence-title.body').text,
        'text',
        1,
        MAX_TITLE_LEN,
      )
      break
    case 'recurrence-body':
      ensureString(
        toRecord(envelope.body, 'recurrence-body.body').text,
        'text',
        0,
        MAX_DESCRIPTION_LEN,
      )
      break
  }
  return {
    kind: expected,
    version: expectedVersion,
    body: normalizePayloadBody(envelope.body, expected, context),
  }
}

function decodeEnvelope(bytes: Uint8Array): {
  kind: unknown
  version: unknown
  body: unknown
} {
  let decoded: unknown
  try {
    decoded = cborDecode(bytes)
  } catch {
    throw new PayloadValidationError('payload is not valid CBOR')
  }
  const envelope = toRecord(decoded, 'payload envelope')
  if (envelope.body === undefined) {
    throw new PayloadValidationError('payload envelope is missing body')
  }
  return {
    kind: envelope.kind,
    version: envelope.version,
    body: envelope.body,
  }
}

function schemaVersionFor(kind: PayloadKind): number {
  switch (kind) {
    case 'work_list':
      return WORK_LIST_SCHEMA_VERSION
    case 'task':
      return TASK_SCHEMA_VERSION
    case 'comment':
      return COMMENT_SCHEMA_VERSION
    case 'recurrence-title':
      return RECURRENCE_TITLE_SCHEMA_VERSION
    case 'recurrence-body':
      return RECURRENCE_BODY_SCHEMA_VERSION
    case 'note':
      return NOTE_SCHEMA_VERSION
  }
}

function validateWorkListPayload(
  value: unknown,
  context: ValidationContext,
): void {
  const body = toRecord(value, 'work_list.body')
  ensureString(body.title, 'title', 1, MAX_TITLE_LEN)
  ensureOptionalString(body.description, 'description', 0, MAX_DESCRIPTION_LEN)
  if (body.theme !== undefined && body.theme !== null) {
    const theme = toRecord(body.theme, 'theme')
    if (
      typeof theme.color !== 'string' ||
      !/^#[0-9a-fA-F]{6}$/.test(theme.color)
    ) {
      throw new PayloadValidationError(
        'theme.color must be a #RRGGBB hex value',
      )
    }
    validateProjectEmoji(theme.emoji, context)
  }
  const sections = toArray(body.sections, 'sections')
  ensureCollectionLimit('sections', sections, MAX_SECTION_COUNT)
  sections.forEach((section, index) => {
    const record = toRecord(section, `sections[${index}]`)
    ensureUuid(record.id, `sections[${index}].id`)
    ensureString(
      record.name,
      `sections[${index}].name`,
      1,
      MAX_SECTION_NAME_LEN,
    )
    if (record.wip_limit !== undefined && record.wip_limit !== null) {
      if (
        typeof record.wip_limit !== 'number' ||
        !Number.isInteger(record.wip_limit) ||
        record.wip_limit <= 0
      ) {
        throw new PayloadValidationError(
          'sections.wip_limit must be an integer value > 0',
        )
      }
    }
  })
  if (body.client_meta !== undefined && body.client_meta !== null) {
    toRecord(body.client_meta, 'client_meta')
  }
}

function validateProjectEmoji(
  value: unknown,
  context: ValidationContext,
): void {
  if (value === undefined || value === null) return
  if (typeof value !== 'string') {
    throw new PayloadValidationError('theme.emoji must be a string')
  }
  const policy = context.dependencies.projectEmoji
  if (context.purpose === 'legacy-read') {
    if (
      policy.getValidationError(value) !== null &&
      !policy.isLegacyReadable(value)
    ) {
      throw new PayloadValidationError(
        'theme.emoji must contain between 1 and 8 characters',
      )
    }
    return
  }
  const error = policy.getValidationError(value)
  if (error === 'too-long') {
    throw new PayloadValidationError(
      `theme.emoji cannot exceed ${policy.maxUtf8Bytes} UTF-8 bytes`,
    )
  }
  if (error) {
    throw new PayloadValidationError(
      'theme.emoji must contain exactly one emoji',
    )
  }
}

function validateTaskPayload(
  value: unknown,
  context: ValidationContext,
): void {
  const body = toRecord(value, 'task.body')
  ensureString(body.title, 'title', 1, MAX_TITLE_LEN)
  if (body.rich_text !== undefined && body.rich_text !== null) {
    validateRichTextDocument(
      body.rich_text,
      { field: 'rich_text', allowEmpty: false },
      context,
    )
  }
  validateUuidCollection(body.mentions, 'mentions', MAX_MENTIONS)

  const checklist = toArray(body.checklist, 'checklist')
  ensureCollectionLimit('checklist', checklist, MAX_CHECKLIST_ITEMS)
  checklist.forEach((entry, index) => validateChecklistItem(entry, index))

  const attachments = toArray(body.attachments, 'attachments')
  ensureCollectionLimit('attachments', attachments, MAX_ATTACHMENTS)
  attachments.forEach((attachment, index) =>
    validateAttachment(attachment, index),
  )

  const references = toArray(body.references, 'references')
  ensureCollectionLimit('references', references, MAX_REFERENCES)
  references.forEach((entry, index) => {
    const reference = toRecord(entry, `references[${index}]`)
    ensureString(
      reference.label,
      `references[${index}].label`,
      1,
      MAX_REFERENCE_LABEL_LEN,
    )
    ensureString(
      reference.uri,
      `references[${index}].uri`,
      1,
      MAX_URI_LEN,
    )
    if (
      !['url', 'task', 'doc'].includes(
        String(reference.kind ?? '').toLowerCase(),
      )
    ) {
      throw new PayloadValidationError(
        'references.kind must be url, task, or doc',
      )
    }
  })
  if (body.recurrence_state !== undefined && body.recurrence_state !== null) {
    const state = toRecord(body.recurrence_state, 'recurrence_state')
    ensureUuid(state.template_id, 'recurrence_state.template_id')
    ensureString(
      state.occurrence,
      'recurrence_state.occurrence',
      1,
      MAX_TITLE_LEN,
    )
  }
  if (body.client_meta !== undefined && body.client_meta !== null) {
    toRecord(body.client_meta, 'client_meta')
  }
}

function validateChecklistItem(value: unknown, index: number): void {
  const field = `checklist[${index}]`
  const item = toRecord(value, field)
  ensureUuid(item.id, `${field}.id`)
  ensureString(item.title, `${field}.title`, 1, MAX_CHECKLIST_TITLE_LEN)
  if (typeof item.is_done !== 'boolean') {
    throw new PayloadValidationError('checklist.is_done must be a boolean')
  }
  if (item.completed_at !== undefined && item.completed_at !== null) {
    if (
      typeof item.completed_at !== 'number' ||
      !Number.isFinite(item.completed_at)
    ) {
      throw new PayloadValidationError(
        'checklist.completed_at must be a unix timestamp',
      )
    }
  }
  validateUuidCollection(
    item.assignee_user_ids,
    `${field}.assignee_user_ids`,
    MAX_ASSIGNEES_PER_CHECKLIST,
  )
}

function validateAttachment(value: unknown, index: number): void {
  const field = `attachments[${index}]`
  const attachment = toRecord(value, field)
  ensureUuid(attachment.id, `${field}.id`)
  ensureString(
    attachment.file_name,
    `${field}.file_name`,
    1,
    MAX_ATTACHMENT_NAME_LEN,
  )
  ensureString(
    attachment.content_type,
    `${field}.content_type`,
    1,
    MAX_TITLE_LEN,
  )
  if (
    typeof attachment.size_bytes !== 'number' ||
    !Number.isFinite(attachment.size_bytes) ||
    !Number.isInteger(attachment.size_bytes) ||
    attachment.size_bytes <= 0 ||
    attachment.size_bytes > MAX_ATTACHMENT_WIRE_READ_BYTES
  ) {
    throw new PayloadValidationError(
      `attachments.size_bytes must be an integer between 1 and ${MAX_ATTACHMENT_WIRE_READ_BYTES}`,
    )
  }
  const blobKey = toByteArray(attachment.blob_key, `${field}.blob_key`)
  if (blobKey.length === 0 || blobKey.length > MAX_BLOB_KEY_LEN) {
    throw new PayloadValidationError(
      'attachments.blob_key must be a sealed reference',
    )
  }
  if (attachment.created_by_membership_id !== undefined) {
    ensureUuid(
      attachment.created_by_membership_id,
      `${field}.created_by_membership_id`,
    )
  }
}

function validateCommentPayload(
  value: unknown,
  context: ValidationContext,
): void {
  const body = toRecord(value, 'comment.body')
  validateRichTextDocument(
    body.content,
    { field: 'content', allowEmpty: false },
    context,
  )
  validateUuidCollection(body.mentions, 'mentions', MAX_MENTIONS)
  const attachments = toArray(body.attachments, 'attachments')
  ensureCollectionLimit('attachments', attachments, MAX_ATTACHMENTS)
  attachments.forEach((attachment, index) =>
    validateAttachment(attachment, index),
  )
  if (body.client_meta !== undefined && body.client_meta !== null) {
    toRecord(body.client_meta, 'client_meta')
  }
}

function validateNotePayload(
  value: unknown,
  context: ValidationContext,
): void {
  const body = toRecord(value, 'note.body')
  ensureString(body.title, 'title', 1, MAX_TITLE_LEN)
  validateRichTextDocument(
    body.content,
    { field: 'content', allowEmpty: true },
    context,
  )
  validateUuidCollection(body.mentions, 'mentions', MAX_MENTIONS)
  const attachments = toArray(body.attachments, 'attachments')
  ensureCollectionLimit('attachments', attachments, MAX_ATTACHMENTS)
  attachments.forEach((attachment, index) =>
    validateNoteAttachment(attachment, index),
  )
  if (body.client_meta !== undefined && body.client_meta !== null) {
    toRecord(body.client_meta, 'client_meta')
  }
}

function validateNoteAttachment(value: unknown, index: number): void {
  const field = `attachments[${index}]`
  const attachment = toRecord(value, field)
  const canonical = isCanonicalNoteAttachment(attachment)
  const historical = isHistoricalNoteAttachment(attachment)
  if (canonical) validateAttachment(attachment, index)
  if (historical || !canonical) {
    ensureUuid(attachment.id, `${field}.id`)
    ensureString(
      attachment.name,
      `${field}.name`,
      1,
      MAX_ATTACHMENT_NAME_LEN,
    )
    if (
      typeof attachment.size !== 'number' ||
      !Number.isInteger(attachment.size) ||
      attachment.size <= 0 ||
      attachment.size > MAX_ATTACHMENT_WIRE_READ_BYTES
    ) {
      throw new PayloadValidationError(
        `${field}.size must be a supported positive integer`,
      )
    }
    ensureOptionalString(
      attachment.mime_type,
      `${field}.mime_type`,
      1,
      MAX_TITLE_LEN,
    )
  }
  if (canonical && historical) {
    if (attachment.file_name !== attachment.name) {
      throw new PayloadValidationError(
        `${field}.file_name must match ${field}.name`,
      )
    }
    if (attachment.size_bytes !== attachment.size) {
      throw new PayloadValidationError(
        `${field}.size_bytes must match ${field}.size`,
      )
    }
    if (
      attachment.mime_type !== undefined &&
      attachment.mime_type !== null &&
      attachment.content_type !== attachment.mime_type
    ) {
      throw new PayloadValidationError(
        `${field}.content_type must match ${field}.mime_type`,
      )
    }
  }
}

function isCanonicalNoteAttachment(
  attachment: Record<string, unknown>,
): boolean {
  return ['file_name', 'content_type', 'size_bytes', 'blob_key'].some(
    (field) => Object.hasOwn(attachment, field),
  )
}

function isHistoricalNoteAttachment(
  attachment: Record<string, unknown>,
): boolean {
  return ['name', 'size', 'mime_type'].some((field) =>
    Object.hasOwn(attachment, field),
  )
}

function validateRichTextDocument(
  value: unknown,
  options: RichTextValidationOptions,
  context: ValidationContext,
): void {
  const doc = toRecord(value, options.field)
  if (
    typeof doc.format !== 'string' ||
    !RICH_TEXT_FORMATS.includes(
      doc.format as (typeof RICH_TEXT_FORMATS)[number],
    )
  ) {
    throw new PayloadValidationError(
      `${options.field}.format must be plaintext, markdown, or prosemirror`,
    )
  }
  if (doc.version !== 1) {
    throw new PayloadValidationError(`${options.field}.version must be 1`)
  }
  const blocksField = `${options.field}.blocks`
  const blocks = toRequiredArray(doc.blocks, blocksField)
  if (!options.allowEmpty && blocks.length === 0) {
    throw new PayloadValidationError(`${blocksField} must not be empty`)
  }
  ensureCollectionLimit(blocksField, blocks, MAX_RICH_TEXT_BLOCKS)
  let spanCount = 0
  let inputMarkCount = 0
  let effectiveMarkCount = 0
  blocks.forEach((block, index) => {
    const counts = validateRichTextBlock(
      block,
      index,
      options.field,
      context,
    )
    spanCount += counts.spanCount
    inputMarkCount += counts.inputMarkCount
    effectiveMarkCount += counts.effectiveMarkCount
    if (spanCount > MAX_RICH_TEXT_SPANS) {
      throw new PayloadValidationError(
        `${options.field} cannot exceed ${MAX_RICH_TEXT_SPANS} input spans across all blocks`,
      )
    }
    if (inputMarkCount > MAX_RICH_TEXT_DOCUMENT_MARKS) {
      throw new PayloadValidationError(
        `${options.field} cannot exceed ${MAX_RICH_TEXT_DOCUMENT_MARKS} input marks across all blocks`,
      )
    }
    if (effectiveMarkCount > MAX_RICH_TEXT_DOCUMENT_MARKS) {
      throw new PayloadValidationError(
        `${options.field} cannot exceed ${MAX_RICH_TEXT_DOCUMENT_MARKS} effective marks across all blocks`,
      )
    }
  })
}

function validateRichTextBlock(
  value: unknown,
  index: number,
  rootField: string,
  context: ValidationContext,
): {
  spanCount: number
  inputMarkCount: number
  effectiveMarkCount: number
} {
  const field = `${rootField}.blocks[${index}]`
  const block = toRecord(value, field)
  if (
    typeof block.type !== 'string' ||
    !RICH_TEXT_BLOCK_TYPES.includes(
      block.type as (typeof RICH_TEXT_BLOCK_TYPES)[number],
    )
  ) {
    throw new PayloadValidationError(`${field}.type is not supported`)
  }
  ensureString(block.text, `${field}.text`, 0, MAX_RICH_TEXT_TEXT_LEN)
  if (block.attrs !== undefined) toRecord(block.attrs, `${field}.attrs`)

  const legacyMarks = block.marks === undefined
    ? []
    : toRequiredArray(block.marks, `${field}.marks`)
  ensureCollectionLimit(
    `${field}.marks`,
    legacyMarks,
    MAX_RICH_TEXT_MARKS,
  )
  legacyMarks.forEach((mark, markIndex) =>
    validateTextMark(
      mark,
      `${field}.marks[${markIndex}]`,
      LEGACY_BLOCK_MARK_TYPES,
      context,
    ),
  )

  if (block.content === undefined) {
    return {
      spanCount: 0,
      inputMarkCount: legacyMarks.length,
      effectiveMarkCount:
        typeof block.text === 'string' && block.text.length > 0
          ? legacyMarks.length
          : 0,
    }
  }
  const content = toRequiredArray(block.content, `${field}.content`)
  ensureCollectionLimit(
    `${field}.content`,
    content,
    MAX_RICH_TEXT_SPANS,
  )
  let inputMarkCount = legacyMarks.length
  let effectiveMarkCount = 0
  let aggregateTextLength = 0
  content.forEach((span, spanIndex) => {
    const spanField = `${field}.content[${spanIndex}]`
    const result = validateTextSpan(span, spanField, context)
    inputMarkCount += result.markCount
    effectiveMarkCount += legacyMarks.length + result.markCount
    aggregateTextLength += result.textLength
    if (aggregateTextLength > MAX_RICH_TEXT_TEXT_LEN) {
      throw new PayloadValidationError(
        `${field}.content text cannot exceed ${MAX_RICH_TEXT_TEXT_LEN} characters`,
      )
    }
    if (legacyMarks.length + result.markCount > MAX_RICH_TEXT_MARKS) {
      throw new PayloadValidationError(
        `${spanField}.marks combined with ${field}.marks cannot exceed ${MAX_RICH_TEXT_MARKS} entries`,
      )
    }
  })
  if (
    content.length === 0 &&
    typeof block.text === 'string' &&
    block.text.length > 0
  ) {
    effectiveMarkCount += legacyMarks.length
  }
  return {
    spanCount: content.length,
    inputMarkCount,
    effectiveMarkCount,
  }
}

function validateTextSpan(
  value: unknown,
  field: string,
  context: ValidationContext,
): { textLength: number; markCount: number } {
  const span = toRecord(value, field)
  const text = ensureString(
    span.text,
    `${field}.text`,
    0,
    MAX_RICH_TEXT_TEXT_LEN,
  )
  const marks = span.marks === undefined
    ? []
    : toRequiredArray(span.marks, `${field}.marks`)
  ensureCollectionLimit(`${field}.marks`, marks, MAX_RICH_TEXT_MARKS)
  marks.forEach((mark, index) =>
    validateTextMark(
      mark,
      `${field}.marks[${index}]`,
      TEXT_SPAN_MARK_TYPES,
      context,
    ),
  )
  return { textLength: [...text].length, markCount: marks.length }
}

function validateTextMark(
  value: unknown,
  field: string,
  allowed: readonly string[],
  _context: ValidationContext,
): void {
  const mark = toRecord(value, field)
  if (typeof mark.type !== 'string' || !allowed.includes(mark.type)) {
    throw new PayloadValidationError(`${field}.type is not supported`)
  }
  const attrs = mark.attrs === undefined
    ? undefined
    : toRecord(mark.attrs, `${field}.attrs`)
  if (mark.type === 'link') {
    if (!attrs) {
      throw new PayloadValidationError(
        `${field}.attrs is required for link marks`,
      )
    }
    const href = ensureString(
      attrs.href,
      `${field}.attrs.href`,
      1,
      MAX_URI_LEN,
    )
    if (href.trim().length === 0) {
      throw new PayloadValidationError(
        `${field}.attrs.href must not be blank`,
      )
    }
  }
  if (mark.type === 'mention') {
    if (!attrs) {
      throw new PayloadValidationError(
        `${field}.attrs is required for mention marks`,
      )
    }
    const id = ensureString(
      attrs.id,
      `${field}.attrs.id`,
      1,
      MAX_TITLE_LEN,
    )
    if (id.trim().length === 0) {
      throw new PayloadValidationError(
        `${field}.attrs.id must not be blank`,
      )
    }
    ensureOptionalString(
      attrs.label,
      `${field}.attrs.label`,
      0,
      MAX_TITLE_LEN,
    )
  }
}

function normalizePayloadBody(
  value: unknown,
  kind: PayloadKind,
  context: ValidationContext,
): unknown {
  const body = toRecord(value, `${kind}.body`)
  switch (kind) {
    case 'work_list':
      return {
        ...body,
        ...(Array.isArray(body.sections)
          ? {
              sections: body.sections.map((section, index) => {
                const record = toRecord(section, `sections[${index}]`)
                return {
                  ...record,
                  id: normalizeUuid(record.id, `sections[${index}].id`),
                }
              }),
            }
          : {}),
      }
    case 'task':
      return normalizeTaskBody(body, context)
    case 'comment':
      return normalizeCommentBody(body, context)
    case 'note':
      return normalizeNoteBody(body, context)
    case 'recurrence-title':
    case 'recurrence-body':
      return { ...body }
  }
}

function normalizeTaskBody(
  body: Record<string, unknown>,
  context: ValidationContext,
): Record<string, unknown> {
  return {
    ...body,
    ...(body.rich_text !== undefined && body.rich_text !== null
      ? {
          rich_text: normalizeRichTextDocument(
            body.rich_text,
            'rich_text',
            context,
          ),
        }
      : {}),
    ...(Array.isArray(body.mentions)
      ? { mentions: normalizeUuidArray(body.mentions, 'mentions') }
      : {}),
    ...(Array.isArray(body.checklist)
      ? {
          checklist: body.checklist.map((entry, index) => {
            const field = `checklist[${index}]`
            const item = toRecord(entry, field)
            return {
              ...item,
              id: normalizeUuid(item.id, `${field}.id`),
              ...(Array.isArray(item.assignee_user_ids)
                ? {
                    assignee_user_ids: normalizeUuidArray(
                      item.assignee_user_ids,
                      `${field}.assignee_user_ids`,
                    ),
                  }
                : {}),
            }
          }),
        }
      : {}),
    ...(Array.isArray(body.attachments)
      ? { attachments: normalizeCanonicalAttachments(body.attachments) }
      : {}),
    ...(body.recurrence_state !== undefined &&
    body.recurrence_state !== null
      ? {
          recurrence_state: {
            ...toRecord(body.recurrence_state, 'recurrence_state'),
            template_id: normalizeUuid(
              toRecord(body.recurrence_state, 'recurrence_state')
                .template_id,
              'recurrence_state.template_id',
            ),
          },
        }
      : {}),
  }
}

function normalizeCommentBody(
  body: Record<string, unknown>,
  context: ValidationContext,
): Record<string, unknown> {
  return {
    ...body,
    content: normalizeRichTextDocument(body.content, 'content', context),
    ...(Array.isArray(body.mentions)
      ? { mentions: normalizeUuidArray(body.mentions, 'mentions') }
      : {}),
    ...(Array.isArray(body.attachments)
      ? { attachments: normalizeCanonicalAttachments(body.attachments) }
      : {}),
  }
}

function normalizeNoteBody(
  body: Record<string, unknown>,
  context: ValidationContext,
): Record<string, unknown> {
  return {
    ...body,
    content: normalizeRichTextDocument(body.content, 'content', context),
    ...(Array.isArray(body.mentions)
      ? { mentions: normalizeUuidArray(body.mentions, 'mentions') }
      : {}),
    ...(Array.isArray(body.attachments)
      ? {
          attachments: body.attachments.map((attachment, index) => {
            const field = `attachments[${index}]`
            const record = toRecord(attachment, field)
            if (isCanonicalNoteAttachment(record)) {
              const normalized = normalizeCanonicalAttachment(record, index)
              if (isHistoricalNoteAttachment(record)) {
                delete normalized.name
                delete normalized.size
                delete normalized.mime_type
              }
              return normalized
            }
            return {
              ...record,
              id: normalizeUuid(record.id, `${field}.id`),
            }
          }),
        }
      : {}),
  }
}

function normalizeCanonicalAttachments(
  values: unknown[],
): Record<string, unknown>[] {
  return values.map(normalizeCanonicalAttachment)
}

function normalizeCanonicalAttachment(
  value: unknown,
  index: number,
): Record<string, unknown> {
  const field = `attachments[${index}]`
  const attachment = toRecord(value, field)
  return {
    ...attachment,
    id: normalizeUuid(attachment.id, `${field}.id`),
    blob_key: toByteArray(attachment.blob_key, `${field}.blob_key`),
  }
}

function normalizeRichTextDocument(
  value: unknown,
  field: string,
  context: ValidationContext,
): Record<string, unknown> {
  const document = toRecord(value, field)
  return {
    ...document,
    blocks: toRequiredArray(document.blocks, `${field}.blocks`).map(
      (block, index) =>
        normalizeRichTextBlock(block, index, field, context),
    ),
  }
}

function normalizeRichTextBlock(
  value: unknown,
  index: number,
  rootField: string,
  context: ValidationContext,
): Record<string, unknown> {
  const field = `${rootField}.blocks[${index}]`
  const block = toRecord(value, field)
  const legacyMarks = Array.isArray(block.marks)
    ? normalizeTextMarks(block.marks, `${field}.marks`, context)
    : null
  let content = Array.isArray(block.content)
    ? block.content
        .map((span, spanIndex) =>
          normalizeTextSpan(
            span,
            `${field}.content[${spanIndex}]`,
            context,
          ),
        )
        .filter((span) => typeof span.text === 'string' && span.text.length > 0)
    : null
  if (content?.length && legacyMarks?.length) {
    content = content.map((span) => ({
      ...span,
      marks: [
        ...legacyMarks,
        ...(Array.isArray(span.marks) ? span.marks : []),
      ],
    }))
  }
  const normalized: Record<string, unknown> = {
    ...block,
    type: block.type === 'list_item' ? 'bullet_item' : block.type,
    ...(block.attrs !== undefined
      ? { attrs: { ...toRecord(block.attrs, `${field}.attrs`) } }
      : {}),
    ...(content !== null ? { content } : {}),
  }
  if (legacyMarks !== null) delete normalized.marks
  if (
    legacyMarks?.length &&
    !content?.length &&
    typeof block.text === 'string' &&
    block.text.length > 0
  ) {
    normalized.content = [{ text: block.text, marks: legacyMarks }]
  }
  return normalized
}

function normalizeTextSpan(
  value: unknown,
  field: string,
  context: ValidationContext,
): Record<string, unknown> {
  const span = toRecord(value, field)
  const normalized = { ...span }
  if (Array.isArray(span.marks)) {
    delete normalized.marks
    const marks = normalizeTextMarks(span.marks, `${field}.marks`, context)
    if (marks.length > 0) normalized.marks = marks
  }
  return normalized
}

function normalizeTextMarks(
  values: unknown[],
  field: string,
  context: ValidationContext,
): Record<string, unknown>[] {
  return values
    .map((mark, index) =>
      normalizeTextMark(mark, `${field}[${index}]`, context),
    )
    .filter((mark): mark is Record<string, unknown> => mark !== null)
}

function normalizeTextMark(
  value: unknown,
  field: string,
  context: ValidationContext,
): Record<string, unknown> | null {
  const mark = toRecord(value, field)
  const attrs = mark.attrs === undefined
    ? undefined
    : { ...toRecord(mark.attrs, `${field}.attrs`) }
  if (mark.type === 'link') {
    const href = context.dependencies.normalizeRichTextHref(attrs?.href)
    if (!href) return null
    return { ...mark, attrs: { ...attrs, href } }
  }
  return { ...mark, ...(attrs ? { attrs } : {}) }
}

function validateUuidCollection(
  value: unknown,
  field: string,
  maximum: number,
): void {
  const values = toArray(value, field)
  ensureCollectionLimit(field, values, maximum)
  values.forEach((entry, index) => ensureUuid(entry, `${field}[${index}]`))
}

function ensureCollectionLimit(
  field: string,
  values: unknown[],
  maximum: number,
): void {
  if (values.length > maximum) {
    throw new PayloadValidationError(
      `${field} cannot exceed ${maximum} entries`,
    )
  }
}

function ensureString(
  value: unknown,
  field: string,
  minimum: number,
  maximum: number,
): string {
  if (typeof value !== 'string') {
    throw new PayloadValidationError(`${field} must be a string`)
  }
  const length = [...value].length
  if (length < minimum) {
    throw new PayloadValidationError(
      `${field} must have at least ${minimum} characters`,
    )
  }
  if (length > maximum) {
    throw new PayloadValidationError(
      `${field} cannot exceed ${maximum} characters`,
    )
  }
  return value
}

function ensureOptionalString(
  value: unknown,
  field: string,
  minimum: number,
  maximum: number,
): string | null {
  return value === undefined || value === null
    ? null
    : ensureString(value, field, minimum, maximum)
}

function toArray(value: unknown, field: string): unknown[] {
  if (value === undefined || value === null) return []
  if (!Array.isArray(value)) {
    throw new PayloadValidationError(`${field} must be an array`)
  }
  return value
}

function toRequiredArray(value: unknown, field: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new PayloadValidationError(`${field} must be an array`)
  }
  return value
}

function toRecord(value: unknown, field: string): Record<string, unknown> {
  if (value instanceof Map) return Object.fromEntries(value.entries())
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>
  }
  throw new PayloadValidationError(`${field} must be an object`)
}

function ensureUuid(value: unknown, field: string): void {
  normalizeUuid(value, field)
}

function normalizeUuid(value: unknown, field: string): string {
  if (value instanceof Uint8Array) {
    if (value.length !== 16) {
      throw new PayloadValidationError(`${field} must be 16 bytes`)
    }
    return formatUuidBytes(value)
  }
  if (value instanceof ArrayBuffer) {
    if (value.byteLength !== 16) {
      throw new PayloadValidationError(`${field} must be 16 bytes`)
    }
    return formatUuidBytes(new Uint8Array(value))
  }
  if (Array.isArray(value)) {
    if (
      value.length !== 16 ||
      value.some(
        (entry) =>
          typeof entry !== 'number' ||
          !Number.isInteger(entry) ||
          entry < 0 ||
          entry > 255,
      )
    ) {
      throw new PayloadValidationError(
        `${field} must be an array of 16 bytes`,
      )
    }
    return formatUuidBytes(Uint8Array.from(value))
  }
  if (
    typeof value === 'string' &&
    /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/.test(
      value.trim(),
    )
  ) {
    return value.trim().toLowerCase()
  }
  throw new PayloadValidationError(
    `${field} must be a UUID or 16-byte array`,
  )
}

function normalizeUuidArray(values: unknown[], field: string): string[] {
  return values.map((value, index) =>
    normalizeUuid(value, `${field}[${index}]`),
  )
}

function formatUuidBytes(bytes: Uint8Array): string {
  const hex = Array.from(bytes, (byte) =>
    byte.toString(16).padStart(2, '0'),
  ).join('')
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join('-')
}

function toByteArray(value: unknown, field: string): Uint8Array {
  if (value instanceof Uint8Array) return value
  if (value instanceof ArrayBuffer) return new Uint8Array(value)
  if (
    Array.isArray(value) &&
    value.every(
      (entry) =>
        typeof entry === 'number' &&
        Number.isInteger(entry) &&
        entry >= 0 &&
        entry <= 255,
    )
  ) {
    return Uint8Array.from(value)
  }
  throw new PayloadValidationError(`${field} must be a byte array`)
}

export async function computeSchemaHash(
  bytes: Uint8Array,
): Promise<Uint8Array> {
  if (!globalThis.crypto?.subtle) {
    throw new Error('WebCrypto digest is unavailable')
  }
  return new Uint8Array(
    await globalThis.crypto.subtle.digest('SHA-256', toArrayBuffer(bytes)),
  )
}
