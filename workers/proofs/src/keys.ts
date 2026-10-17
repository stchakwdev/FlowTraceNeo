/**
 * Object keys and the ids that go into them. Files are content-addressed per organisation:
 * `org/<org>/<sha256>` for the original and `org/<org>/thumb/<sha256>.webp` for its thumbnail.
 */

/** Organisation ids accepted in keys and SQL. Strict so nothing odd reaches a query. */
export const ORG_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/

const SHA256 = /^[0-9a-f]{64}$/

export function isOrgId(s: string): boolean {
  return ORG_ID.test(s)
}

export function isSha256(s: string): boolean {
  return SHA256.test(s)
}

/** A SQL string literal: single quotes doubled. Only ever called on a validated org id. */
export function sqlString(s: string): string {
  return `'${s.replaceAll("'", "''")}'`
}

export function originalKey(org: string, sha256: string): string {
  return `org/${org}/${sha256}`
}

export function thumbnailKey(org: string, sha256: string): string {
  return `org/${org}/thumb/${sha256}.webp`
}

export interface ParsedKey {
  key: string
  org: string
  sha256: string
  thumbnail: boolean
}

/** Parse an object key, or null when it is not one of the two shapes. */
export function parseKey(key: string): ParsedKey | null {
  const parts = key.split('/')
  if (parts[0] !== 'org' || parts.length < 3) return null
  const org = parts[1]
  if (!isOrgId(org)) return null
  if (parts.length === 3 && isSha256(parts[2])) {
    return { key, org, sha256: parts[2], thumbnail: false }
  }
  if (parts.length === 4 && parts[2] === 'thumb' && parts[3].endsWith('.webp')) {
    const sha256 = parts[3].slice(0, -'.webp'.length)
    if (isSha256(sha256)) return { key, org, sha256, thumbnail: true }
  }
  return null
}

/** What a proof links to (`create_verification` takes one of the three). */
export const LINK_KINDS = ['claim', 'bill', 'transaction'] as const
export type LinkKind = (typeof LINK_KINDS)[number]

export function isLinkKind(s: string): s is LinkKind {
  return (LINK_KINDS as readonly string[]).includes(s)
}

/** A link id: non-empty, at most 128 characters, no control characters. */
export function isLinkId(s: string): boolean {
  // eslint-disable-next-line no-control-regex
  return s.length > 0 && s.length <= 128 && !/[\u0000-\u001f\u007f-\u009f]/.test(s)
}
