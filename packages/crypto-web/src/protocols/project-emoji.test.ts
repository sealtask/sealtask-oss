import { encode as cborEncode } from 'cbor-x'
import { describe, expect, it } from 'vitest'

import {
  decodeAndValidatePayloadBytes,
  validatePayloadBytes,
} from './payload-validation'
import {
  getProjectEmojiValidationError,
  isValidProjectEmoji,
  normalizeOptionalProjectEmoji,
} from './project-emoji'

function workListPayload(emoji: string) {
  return cborEncode({
    kind: 'work_list',
    version: 1,
    body: {
      title: 'Project',
      theme: { color: '#123ABC', emoji },
      sections: [],
    },
  })
}

describe('project emoji protocol', () => {
  it('uses the generated Unicode RGI sequence allowlist', () => {
    expect(isValidProjectEmoji('🧑‍💻')).toBe(true)
    expect(isValidProjectEmoji('🏳️‍🌈')).toBe(true)
    expect(getProjectEmojiValidationError('🧑‍🐱')).toBe(
      'not-one-emoji',
    )
    expect(getProjectEmojiValidationError('')).toBe('empty')
  })

  it('normalizes optional input without changing valid sequences', () => {
    expect(normalizeOptionalProjectEmoji('  🪴  ')).toBe('🪴')
    expect(normalizeOptionalProjectEmoji('   ')).toBeNull()
    expect(() => normalizeOptionalProjectEmoji('🧑‍🐱')).toThrow(
      'exactly one emoji',
    )
  })

  it('rejects non-RGI writes while retaining the legacy-read exception', () => {
    const bytes = workListPayload('🧑‍🐱')
    expect(() => validatePayloadBytes(bytes, 'work_list')).toThrow(
      'exactly one emoji',
    )
    expect(
      decodeAndValidatePayloadBytes(bytes, 'work_list').kind,
    ).toBe('work_list')
  })

  it.each(['', '123456789'])(
    'rejects legacy reads outside the exact 1–8 code-point range: %s',
    (emoji) => {
      expect(() =>
        decodeAndValidatePayloadBytes(
          workListPayload(emoji),
          'work_list',
        ),
      ).toThrow('between 1 and 8 characters')
    },
  )

  it('allows an explicitly injected policy without changing the default', () => {
    expect(() =>
      validatePayloadBytes(workListPayload('custom'), 'work_list', {
        projectEmoji: {
          maxUtf8Bytes: 64,
          getValidationError: (value) =>
            value === 'custom' ? null : 'not-one-emoji',
          isLegacyReadable: (
            value: unknown,
          ): value is string => value === 'custom',
        },
      }),
    ).not.toThrow()
    expect(isValidProjectEmoji('custom')).toBe(false)
  })
})
