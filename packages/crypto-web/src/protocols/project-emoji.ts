import { PROJECT_EMOJI_SEQUENCES } from './project-emoji-sequences'

export const MAX_PROJECT_EMOJI_UTF8_BYTES = 64

export type ProjectEmojiValidationError =
  | 'empty'
  | 'not-one-emoji'
  | 'too-long'

export type ProjectEmojiPolicy = {
  getValidationError(value: string): ProjectEmojiValidationError | null
  isLegacyReadable(value: unknown): value is string
  maxUtf8Bytes: number
}

const utf8Encoder = new TextEncoder()

export function normalizeProjectEmoji(value: string): string {
  return value.trim()
}

export function projectEmojiUtf8Length(value: string): number {
  return utf8Encoder.encode(value).byteLength
}

export function getProjectEmojiValidationError(
  value: string,
): ProjectEmojiValidationError | null {
  if (!value) return 'empty'
  if (projectEmojiUtf8Length(value) > MAX_PROJECT_EMOJI_UTF8_BYTES) {
    return 'too-long'
  }
  if (!PROJECT_EMOJI_SEQUENCES.has(value)) return 'not-one-emoji'
  return null
}

export function isValidProjectEmoji(value: string): boolean {
  return getProjectEmojiValidationError(value) === null
}

export function isLegacyReadableProjectEmoji(value: unknown): value is string {
  if (typeof value !== 'string') return false
  const codePointLength = Array.from(value).length
  return codePointLength >= 1 && codePointLength <= 8
}

export function normalizeOptionalProjectEmoji(value: string): string | null {
  const normalized = normalizeProjectEmoji(value)
  if (!normalized) return null

  const validationError = getProjectEmojiValidationError(normalized)
  if (validationError === 'too-long') {
    throw new Error(
      `Project emoji cannot exceed ${MAX_PROJECT_EMOJI_UTF8_BYTES} UTF-8 bytes`,
    )
  }
  if (validationError) {
    throw new Error('Project emoji must contain exactly one emoji')
  }
  return normalized
}

export const PROJECT_EMOJI_POLICY: ProjectEmojiPolicy = Object.freeze({
  maxUtf8Bytes: MAX_PROJECT_EMOJI_UTF8_BYTES,
  getValidationError: getProjectEmojiValidationError,
  isLegacyReadable: isLegacyReadableProjectEmoji,
})
