/**
 * Decoding the SpacetimeDB HTTP SQL response
 * (https://spacetimedb.com/docs/http/database#post-v1databasename_or_identitysql).
 * The body is an array of statement results; each has `schema.elements` (column names and
 * algebraic types) and `rows`, where each row is an array of values in column order.
 */

export interface SqlElement {
  name?: { some: string } | { none: unknown } | string | null
  algebraic_type?: unknown
}

export interface SqlStatementResult {
  schema: { elements: SqlElement[] }
  rows: unknown[][]
}

export type Row = Record<string, unknown>

function columnName(e: SqlElement, i: number): string {
  const n = e.name
  if (typeof n === 'string') return n
  if (n && typeof n === 'object' && 'some' in n && typeof n.some === 'string') return n.some
  return String(i)
}

function variantNames(t: unknown): string[] | null {
  if (!t || typeof t !== 'object') return null
  const sum = (t as { Sum?: { variants?: SqlElement[] } }).Sum
  if (!sum || !Array.isArray(sum.variants)) return null
  return sum.variants.map((v, i) => columnName(v, i))
}

/** An `Option<T>` column: a sum whose variants are `some` then `none`. */
export function isOptionType(t: unknown): boolean {
  const names = variantNames(t)
  return names !== null && names.length === 2 && names[0] === 'some' && names[1] === 'none'
}

/** An `Identity` column: a product with the one element `__identity__`. */
export function isIdentityType(t: unknown): boolean {
  if (!t || typeof t !== 'object') return false
  const p = (t as { Product?: { elements?: SqlElement[] } }).Product
  return (
    !!p &&
    Array.isArray(p.elements) &&
    p.elements.length === 1 &&
    columnName(p.elements[0], 0) === '__identity__'
  )
}

/**
 * An option value in either JSON encoding: `[0, v]` / `[1, []]` (tag and payload) or
 * `{ some: v }` / `{ none: [] }`. Returns null for none.
 */
export function decodeOption(v: unknown): unknown {
  if (Array.isArray(v) && v.length === 2 && (v[0] === 0 || v[0] === 1)) {
    return v[0] === 0 ? v[1] : null
  }
  if (v && typeof v === 'object' && !Array.isArray(v)) {
    if ('some' in v) return (v as { some: unknown }).some
    if ('none' in v) return null
  }
  return v
}

/**
 * An identity as lowercase hex without `0x`, from `"0x..."`, `{ "__identity__": "0x..." }` or a
 * one-element array. Returns null when it is none of those.
 */
export function normaliseIdentity(v: unknown): string | null {
  let x: unknown = v
  if (Array.isArray(x) && x.length === 1) x = x[0]
  if (x && typeof x === 'object' && '__identity__' in x) x = (x as { __identity__: unknown }).__identity__
  if (Array.isArray(x) && x.length === 1) x = x[0]
  if (typeof x !== 'string') return null
  const hex = x.toLowerCase().replace(/^0x/, '')
  return /^[0-9a-f]{64}$/.test(hex) ? hex : null
}

/** One statement's rows as objects keyed by column name, options and identities decoded. */
export function decodeStatement(result: SqlStatementResult): Row[] {
  const elements = result.schema?.elements ?? []
  const names = elements.map(columnName)
  return (result.rows ?? []).map((row) => {
    const out: Row = {}
    names.forEach((name, i) => {
      const t = elements[i]?.algebraic_type
      let v = row[i]
      if (isOptionType(t)) v = decodeOption(v)
      if (isIdentityType(t) || name === 'identity') v = normaliseIdentity(v) ?? v
      out[name] = v
    })
    return out
  })
}

/** All rows of the first statement in a response body. */
export function decodeResponse(body: unknown): Row[] {
  if (!Array.isArray(body) || body.length === 0) return []
  return decodeStatement(body[0] as SqlStatementResult)
}
