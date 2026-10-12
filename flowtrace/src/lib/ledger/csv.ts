// Small RFC 4180 CSV reader and writer. Statement exports from banks and
// mobile-money providers are CSV; reports export to CSV.

export function parseCsv(text: string): Array<Array<string>> {
  const rows: Array<Array<string>> = []
  let row: Array<string> = []
  let field = ''
  let quoted = false
  const src = text.replace(/^﻿/, '')
  for (let i = 0; i < src.length; i++) {
    const c = src[i]
    if (quoted) {
      if (c === '"') {
        if (src[i + 1] === '"') {
          field += '"'
          i++
        } else quoted = false
      } else field += c
      continue
    }
    if (c === '"') quoted = true
    else if (c === ',') {
      row.push(field)
      field = ''
    } else if (c === '\n' || c === '\r') {
      if (c === '\r' && src[i + 1] === '\n') i++
      row.push(field)
      field = ''
      if (row.some((f) => f.trim() !== '')) rows.push(row)
      row = []
    } else field += c
  }
  row.push(field)
  if (row.some((f) => f.trim() !== '')) rows.push(row)
  return rows
}

/** Rows as objects keyed by lower-cased, trimmed header names. */
export function parseCsvRecords(text: string) {
  const rows = parseCsv(text)
  if (rows.length === 0) return { headers: [] as Array<string>, records: [] as Array<Record<string, string>> }
  const headers = rows[0].map((h) => h.trim().toLowerCase().replace(/^\*/, ''))
  const records = rows.slice(1).map((r) => {
    const o: Record<string, string> = {}
    headers.forEach((h, i) => (o[h] = (r[i] ?? '').trim()))
    return o
  })
  return { headers, records }
}

const escape = (v: unknown) => {
  const s = v === null || v === undefined ? '' : String(v)
  return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
}

export function toCsv(rows: Array<Array<unknown>>): string {
  return rows.map((r) => r.map(escape).join(',')).join('\r\n') + '\r\n'
}

const MONTHS: Record<string, number> = {
  jan: 1, feb: 2, mar: 3, apr: 4, may: 5, jun: 6,
  jul: 7, aug: 8, sep: 9, oct: 10, nov: 11, dec: 12,
}

/**
 * Dates as banks write them: 2026-01-15, 2026-01-15 14:03:22, 15/01/2026,
 * 15-01-2026, 15 Jan 2026, 15-Jan-2026. Day-first, as in East Africa.
 */
export function parseStatementDate(raw: string): string | null {
  const s = raw.trim()
  let m = /^(\d{4})-(\d{1,2})-(\d{1,2})/.exec(s)
  if (m) return iso(+m[1], +m[2], +m[3])
  m = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{4})/.exec(s)
  // Day-first, unless that is impossible and month-first is not (US-style
  // exports such as QuickBooks write 01/15/2026).
  if (m) return iso(+m[3], +m[2], +m[1]) ?? (+m[2] > 12 ? iso(+m[3], +m[1], +m[2]) : null)
  m = /^(\d{1,2})[\s-]([A-Za-z]{3})[A-Za-z]*[\s-](\d{4})/.exec(s)
  if (m && MONTHS[m[2].toLowerCase()]) return iso(+m[3], MONTHS[m[2].toLowerCase()], +m[1])
  return null
}

function iso(y: number, mo: number, d: number) {
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return null
  const dt = new Date(Date.UTC(y, mo - 1, d))
  if (dt.getUTCMonth() !== mo - 1) return null
  return `${y}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}`
}
