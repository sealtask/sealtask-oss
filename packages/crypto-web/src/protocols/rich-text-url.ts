export const MAX_RICH_TEXT_HREF_LENGTH = 2048

const EXPLICIT_SCHEME = /^([a-z][a-z0-9+.-]*):/i
const SAFE_SCHEMES = new Set(['http:', 'https:', 'mailto:', 'tel:'])

export function normalizeRichTextHref(value: unknown): string | null {
  if (typeof value !== 'string') return null

  const href = value.trim()
  if (
    href.length === 0 ||
    [...href].length > MAX_RICH_TEXT_HREF_LENGTH ||
    hasAsciiWhitespaceOrControl(href) ||
    href.startsWith('//') ||
    href.includes('\\')
  ) {
    return null
  }

  const schemeMatch = EXPLICIT_SCHEME.exec(href)
  if (schemeMatch) {
    const scheme = `${schemeMatch[1]?.toLowerCase()}:`
    if (SAFE_SCHEMES.has(scheme)) {
      if (
        ((scheme === 'http:' || scheme === 'https:') && !/^https?:\/\//i.test(href)) ||
        ((scheme === 'mailto:' || scheme === 'tel:') &&
          href.slice(scheme.length).startsWith('/'))
      ) {
        return null
      }
      try {
        const parsed = new URL(href)
        if (parsed.protocol.toLowerCase() !== scheme) return null
        if (
          (scheme === 'http:' || scheme === 'https:') &&
          parsed.hostname.length === 0
        ) {
          return null
        }
        if (
          (scheme === 'mailto:' || scheme === 'tel:') &&
          href.slice(scheme.length).length === 0
        ) {
          return null
        }
      } catch {
        return null
      }
      return href
    }
    if (!/^[a-z0-9.-]+:\d+(?:[/?#]|$)/i.test(href)) return null
  }

  if (
    href.startsWith('/') ||
    href.startsWith('./') ||
    href.startsWith('../') ||
    href.startsWith('?') ||
    href.startsWith('#')
  ) {
    return null
  }

  const normalized = `https://${href}`
  if ([...normalized].length > MAX_RICH_TEXT_HREF_LENGTH) return null
  try {
    const parsed = new URL(normalized)
    if (parsed.protocol !== 'https:' || !isValidBareHostname(parsed.hostname)) {
      return null
    }
  } catch {
    return null
  }
  return normalized
}

function hasAsciiWhitespaceOrControl(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index)
    if (codeUnit <= 0x20 || codeUnit === 0x7f) return true
  }
  return false
}

function isValidBareHostname(hostname: string): boolean {
  if (hostname === 'localhost') return true
  if (/^\[[0-9a-f:.]+\]$/i.test(hostname)) return true
  if (/^\d+(?:\.\d+){3}$/.test(hostname)) {
    return hostname.split('.').every((part) => {
      const octet = Number(part)
      return Number.isInteger(octet) && octet >= 0 && octet <= 255
    })
  }
  if (hostname.length > 253 || !hostname.includes('.')) return false
  return hostname.split('.').every((label) =>
    /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/i.test(label),
  )
}
