const CANONICAL_UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/

export function requireCanonicalTransparencyUserId(
  value: string,
  field = 'user_id',
): string {
  if (typeof value !== 'string' || !CANONICAL_UUID_PATTERN.test(value)) {
    throw new Error(
      `${field} must be a canonical lowercase hyphenated UUID string`,
    )
  }
  return value
}

export function transparencyUserIdToBytes(
  value: string,
  field = 'user_id',
): Uint8Array {
  const canonical = requireCanonicalTransparencyUserId(value, field)
  const compact = canonical.replaceAll('-', '')
  const bytes = new Uint8Array(16)
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(
      compact.slice(index * 2, index * 2 + 2),
      16,
    )
  }
  return bytes
}

export function compactTransparencyUserId(
  value: string,
  field = 'user_id',
): string {
  return requireCanonicalTransparencyUserId(value, field).replaceAll('-', '')
}
