/**
 * Structural CBOR checks shared by protocol readers that must reject decoder
 * ambiguities before handing bytes to cbor-x for schema validation.
 *
 * This intentionally accepts both minimal and non-minimal definite-length
 * encodings. The protocol contracts specify CBOR values, not one canonical
 * byte encoding, so compatible clients may choose different valid lengths.
 */

const UTF8_DECODER = new TextDecoder('utf-8', { fatal: true })
const ROOT_ITEM_DEPTH = 0
const TOP_LEVEL_MAP_CHILD_DEPTH = ROOT_ITEM_DEPTH + 1

export type StrictCborHeader = {
  majorType: number
  argument: bigint
  nextOffset: number
}

export type StrictCborTraversalPolicy = {
  /**
   * The deepest item that may be visited, with a root item at depth 0.
   */
  maxDepth: number
}

export function readStrictCborHeader(
  bytes: Uint8Array,
  offset: number,
): StrictCborHeader {
  if (!Number.isSafeInteger(offset) || offset < 0) {
    throw new Error('CBOR offset must be a non-negative safe integer')
  }
  if (offset >= bytes.byteLength) {
    throw new Error('truncated CBOR header')
  }

  const initial = bytes[offset]
  const majorType = initial >>> 5
  const additional = initial & 0x1f
  if (additional < 24) {
    return { majorType, argument: BigInt(additional), nextOffset: offset + 1 }
  }

  const argumentBytes =
    additional === 24
      ? 1
      : additional === 25
        ? 2
        : additional === 26
          ? 4
          : additional === 27
            ? 8
            : 0
  if (argumentBytes === 0 || offset + 1 + argumentBytes > bytes.byteLength) {
    throw new Error('unsupported or truncated CBOR argument')
  }

  let argument = 0n
  for (let index = 0; index < argumentBytes; index += 1) {
    argument = (argument << 8n) | BigInt(bytes[offset + 1 + index])
  }
  return {
    majorType,
    argument,
    nextOffset: offset + 1 + argumentBytes,
  }
}

export function readStrictCborText(
  bytes: Uint8Array,
  offset: number,
): { value: string; nextOffset: number } {
  const header = readStrictCborHeader(bytes, offset)
  if (header.majorType !== 3) {
    throw new Error('CBOR map key is not text')
  }
  const length = requireSafeCborLength(header.argument)
  if (length > bytes.byteLength - header.nextOffset) {
    throw new Error('truncated CBOR text')
  }

  const nextOffset = header.nextOffset + length
  return {
    value: UTF8_DECODER.decode(bytes.subarray(header.nextOffset, nextOffset)),
    nextOffset,
  }
}

/**
 * Reads one definite-length map with unique text keys and no trailing bytes.
 * Callers keep ownership of allowed-key and field-shape validation.
 */
export function readStrictCborTextMapKeys(
  bytes: Uint8Array,
  policy: StrictCborTraversalPolicy,
): string[] {
  const maxDepth = requireMaxDepth(policy)
  const header = readStrictCborHeader(bytes, 0)
  if (header.majorType !== 5) {
    throw new Error('top-level value is not a map')
  }
  const pairCount = requireSafeCborLength(header.argument)

  const keys: string[] = []
  const seen = new Set<string>()
  let offset = header.nextOffset
  if (pairCount > 0) {
    requireTraversalDepth(TOP_LEVEL_MAP_CHILD_DEPTH, maxDepth)
  }
  for (let index = 0; index < pairCount; index += 1) {
    const key = readStrictCborText(bytes, offset)
    if (seen.has(key.value)) {
      throw new Error('duplicate top-level map key')
    }
    seen.add(key.value)
    keys.push(key.value)
    offset = skipStrictCborItemAtDepth(
      bytes,
      key.nextOffset,
      TOP_LEVEL_MAP_CHILD_DEPTH,
      maxDepth,
    )
  }
  if (offset !== bytes.byteLength) {
    throw new Error('trailing bytes')
  }

  return keys
}

export function skipStrictCborItem(
  bytes: Uint8Array,
  offset: number,
  policy: StrictCborTraversalPolicy,
): number {
  return skipStrictCborItemAtDepth(
    bytes,
    offset,
    ROOT_ITEM_DEPTH,
    requireMaxDepth(policy),
  )
}

function requireMaxDepth(policy: StrictCborTraversalPolicy): number {
  if (!Number.isSafeInteger(policy.maxDepth) || policy.maxDepth < 0) {
    throw new Error('CBOR max depth must be a non-negative safe integer')
  }
  return policy.maxDepth
}

function skipStrictCborItemAtDepth(
  bytes: Uint8Array,
  offset: number,
  depth: number,
  maxDepth: number,
): number {
  requireTraversalDepth(depth, maxDepth)

  const header = readStrictCborHeader(bytes, offset)
  switch (header.majorType) {
    case 0:
    case 1:
      return header.nextOffset
    case 2:
    case 3:
      return skipStrictCborString(bytes, header)
    case 4:
      return skipStrictCborArray(bytes, header, depth, maxDepth)
    case 5:
      return skipStrictCborMap(bytes, header, depth, maxDepth)
    case 6:
      return skipStrictCborItemAtDepth(
        bytes,
        header.nextOffset,
        depth + 1,
        maxDepth,
      )
    case 7:
      return header.nextOffset
    default:
      throw new Error('unsupported CBOR major type')
  }
}

function requireTraversalDepth(depth: number, maxDepth: number): void {
  if (depth > maxDepth) {
    throw new Error('CBOR nesting is too deep')
  }
}

function requireSafeCborLength(argument: bigint): number {
  if (argument > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error('CBOR length exceeds the safe integer range')
  }
  return Number(argument)
}

function skipStrictCborString(
  bytes: Uint8Array,
  header: StrictCborHeader,
): number {
  const length = requireSafeCborLength(header.argument)
  if (length > bytes.byteLength - header.nextOffset) {
    throw new Error('truncated CBOR string')
  }
  return header.nextOffset + length
}

function skipStrictCborArray(
  bytes: Uint8Array,
  header: StrictCborHeader,
  depth: number,
  maxDepth: number,
): number {
  const itemCount = requireSafeCborLength(header.argument)
  let nextOffset = header.nextOffset
  for (let index = 0; index < itemCount; index += 1) {
    nextOffset = skipStrictCborItemAtDepth(
      bytes,
      nextOffset,
      depth + 1,
      maxDepth,
    )
  }
  return nextOffset
}

function skipStrictCborMap(
  bytes: Uint8Array,
  header: StrictCborHeader,
  depth: number,
  maxDepth: number,
): number {
  const pairCount = requireSafeCborLength(header.argument)
  let nextOffset = header.nextOffset
  for (let index = 0; index < pairCount; index += 1) {
    nextOffset = skipStrictCborItemAtDepth(
      bytes,
      nextOffset,
      depth + 1,
      maxDepth,
    )
    nextOffset = skipStrictCborItemAtDepth(
      bytes,
      nextOffset,
      depth + 1,
      maxDepth,
    )
  }
  return nextOffset
}
