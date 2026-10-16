// Canonical JSON for the fixtures: object keys sorted, two-space indent,
// trailing newline. Values are written as the TS engine returned them; the
// only representation choices are the ones JSON forces, each listed in
// rust/fixtures/README.md under "Formatting notes":
// - a non-finite number becomes the string "Infinity", "-Infinity" or "NaN";
// - a Map becomes an object (keys sorted), a Set an array (insertion order);
// - undefined is dropped from objects and becomes null in arrays.

export type Json = null | boolean | number | string | Array<Json> | { [k: string]: Json }

/** JSON paths where a non-finite number was written as a string, per run. */
export const nonFinite: Array<string> = []

const isPlain = (v: object) => {
  const proto = Object.getPrototypeOf(v)
  return proto === Object.prototype || proto === null
}

export function canonical(v: unknown, at = '$'): Json {
  if (v === null) return null
  switch (typeof v) {
    case 'boolean':
    case 'string':
      return v
    case 'number':
      if (Number.isFinite(v)) return Object.is(v, -0) ? 0 : v
      nonFinite.push(at)
      return String(v)
    case 'undefined':
      throw new Error(`canonical: undefined at ${at}`)
    case 'object':
      break
    default:
      throw new Error(`canonical: cannot write a ${typeof v} at ${at}`)
  }
  const o = v as object
  if (Array.isArray(o)) return o.map((x, i) => (x === undefined ? null : canonical(x, `${at}[${i}]`)))
  if (o instanceof Set) return [...o].map((x, i) => canonical(x, `${at}[${i}]`))
  const entries: Array<[string, unknown]> =
    o instanceof Map
      ? [...o.entries()].map(([k, x]) => {
          if (typeof k !== 'string') throw new Error(`canonical: non-string Map key at ${at}`)
          return [k, x]
        })
      : isPlain(o)
        ? Object.entries(o)
        : (() => {
            throw new Error(`canonical: cannot write a ${o.constructor?.name ?? 'object'} at ${at}`)
          })()
  const out: { [k: string]: Json } = {}
  for (const [k, x] of entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
    if (!/^[\x20-\x7e]*$/.test(k)) throw new Error(`canonical: non-ASCII key ${JSON.stringify(k)} at ${at}`)
    if (x === undefined) continue
    out[k] = canonical(x, `${at}.${k}`)
  }
  return out
}

/** Pretty-prints canonical JSON. Keys are written in sorted order whatever their shape. */
export function stringify(v: Json, indent = ''): string {
  if (v === null || typeof v !== 'object') return JSON.stringify(v)
  const inner = `${indent}  `
  if (Array.isArray(v)) {
    if (v.length === 0) return '[]'
    return `[\n${v.map((x) => inner + stringify(x, inner)).join(',\n')}\n${indent}]`
  }
  const keys = Object.keys(v).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))
  if (keys.length === 0) return '{}'
  return `{\n${keys.map((k) => `${inner}${JSON.stringify(k)}: ${stringify(v[k], inner)}`).join(',\n')}\n${indent}}`
}

export const toJsonText = (v: unknown) => `${stringify(canonical(v))}\n`

/** A stable fingerprint of a value, for change detection. */
export const fingerprint = (v: unknown) => stringify(canonical(v))
