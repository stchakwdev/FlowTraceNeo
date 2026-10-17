// IT1's report comparison: one statement read three ways, reduced to one normal form and
// required byte-identical.
//
// - web: the figures as the Reports screen shows them (`ReportBody`, the tables and footing
//   checks of flowtrace/src/components/ledger/reports/statement.tsx), read from the DOM;
// - desktop: `desktop::statements::statement` on the books ledger-client rebuilds from the
//   module rows (`it1_probe reports`);
// - oracle: the same function on the books ledger-core gets by replaying the run's command log
//   from rust/fixtures/demo-books.json (`it1_probe oracle`).
//
// Normal form, per report id: the tables (first column head and figure heads; rows with kind,
// account code, label, detail and cells) and the footing differences. Money is an integer in
// minor units, so formatting never decides (pre-decision P4); gap rows and tables with no rows
// are dropped (the web shows an empty notice in place of such a table). Text is
// `canonicalText`: object keys sorted, one row per line, so a difference prints as a short diff.

export type NormCell = number | string

export interface NormRow {
  kind: string
  code: string | null
  label: string
  detail: string | null
  cells: Array<NormCell>
}

export interface NormTable {
  columns: Array<string>
  rows: Array<NormRow>
}

export interface NormReport {
  tables: Array<NormTable>
  footings: Array<NormCell>
}

export type NormReports = Record<string, NormReport>

/** `it1_probe reports` / `oracle` output (rust/crates/desktop/examples/it1_probe.rs, statement_json). */
export interface ProbeRow {
  kind: string
  code: string | null
  label: string
  detail: string | null
  cells: Array<number | string>
}

export interface ProbeTable {
  title: string | null
  first_column: string
  columns: Array<string>
  rows: Array<ProbeRow>
  empty: string | null
}

export interface ProbeReport {
  title: string
  tables: Array<ProbeTable>
  footings: Array<{ statement: string; difference: number }>
}

export type ProbeReports = Record<string, ProbeReport>

/** What it1-web.ts reads off the Reports screen: cells and footing differences as shown. */
export interface DomRow {
  kind: string
  code: string | null
  label: string
  detail: string | null
  cells: Array<string>
}

export interface DomReport {
  tables: Array<{ columns: Array<string>; rows: Array<DomRow> }>
  footings: Array<string>
}

const MONEY = /^(\()?([−-])?(\d{1,3}(?:,\d{3})*)\.(\d{2})(\))?$/

/**
 * A figure as `formatMinor` shows it (flowtrace/src/components/ledger/money.tsx:8-21; desktop
 * `format_minor`) as integer minor units: en-KE grouping, two decimals, negatives in parentheses
 * or after a minus sign, zero as an en dash. Anything else is trimmed text.
 */
export function parseCell(v: number | string): NormCell {
  if (typeof v === 'number') return v
  const t = v.trim().replace(/\s+/g, ' ')
  if (t === '–') return 0
  const m = MONEY.exec(t)
  if (!m || Boolean(m[1]) !== Boolean(m[5]) || (m[1] && m[2])) return t
  const minor = Number(m[3].replaceAll(',', '')) * 100 + Number(m[4])
  return m[1] || m[2] ? -minor : minor
}

const clean = (s: string | null) => {
  const t = s?.trim().replace(/\s+/g, ' ') ?? ''
  return t === '' ? null : t
}

function normRow(r: { kind: string; code: string | null; label: string; detail: string | null; cells: Array<number | string> }): NormRow {
  return { kind: r.kind, code: clean(r.code), label: clean(r.label) ?? '', detail: clean(r.detail), cells: r.cells.map(parseCell) }
}

const keep = (t: NormTable) => t.rows.length > 0

export function fromProbe(reports: ProbeReports): NormReports {
  const out: NormReports = {}
  for (const [id, r] of Object.entries(reports)) {
    out[id] = {
      tables: r.tables
        .map((t) => ({ columns: [t.first_column, ...t.columns].map((c) => clean(c) ?? ''), rows: t.rows.filter((x) => x.kind !== 'gap').map(normRow) }))
        .filter(keep),
      footings: r.footings.map((f) => f.difference),
    }
  }
  return out
}

export function fromDom(reports: Record<string, DomReport>): NormReports {
  const out: NormReports = {}
  for (const [id, r] of Object.entries(reports)) {
    out[id] = {
      tables: r.tables.map((t) => ({ columns: t.columns.map((c) => clean(c) ?? ''), rows: t.rows.filter((x) => x.kind !== 'gap').map(normRow) })).filter(keep),
      footings: r.footings.map(parseCell),
    }
  }
  return out
}

/** Sorted keys, one table row per line. */
export function canonicalText(reports: NormReports): string {
  const sorted = (v: unknown): unknown => {
    if (Array.isArray(v)) return v.map(sorted)
    if (v && typeof v === 'object') return Object.fromEntries(Object.keys(v).sort().map((k) => [k, sorted((v as Record<string, unknown>)[k])]))
    return v
  }
  const lines: Array<string> = []
  for (const id of Object.keys(reports).sort()) {
    const r = reports[id]
    lines.push(`${JSON.stringify(id)}: footings ${JSON.stringify(r.footings)}`)
    r.tables.forEach((t, i) => {
      lines.push(`${JSON.stringify(id)} table ${i}: ${JSON.stringify(t.columns)}`)
      for (const row of t.rows) lines.push(JSON.stringify(sorted(row)))
    })
  }
  return `${lines.join('\n')}\n`
}

/** A line diff (longest common subsequence): `-` lines only in `a`, `+` only in `b`. */
export function lineDiff(a: string, b: string, context = 2): string {
  const x = a.split('\n')
  const y = b.split('\n')
  const n = x.length
  const m = y.length
  const lcs = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) lcs[i][j] = x[i] === y[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1])
  const ops: Array<[' ' | '-' | '+', string]> = []
  let i = 0
  let j = 0
  while (i < n || j < m) {
    if (i < n && j < m && x[i] === y[j]) {
      ops.push([' ', x[i]])
      i++
      j++
    } else if (j < m && (i === n || lcs[i][j + 1] >= lcs[i + 1][j])) ops.push(['+', y[j++]])
    else ops.push(['-', x[i++]])
  }
  const shown = ops.map((o, k) => o[0] !== ' ' || ops.slice(Math.max(0, k - context), k + context + 1).some((p) => p[0] !== ' '))
  const out: Array<string> = []
  ops.forEach((o, k) => {
    if (shown[k]) out.push(`${o[0]} ${o[1]}`)
    else if (shown[k - 1]) out.push('  ...')
  })
  return out.join('\n')
}

export interface Comparison {
  equal: boolean
  texts: { web: string; desktop: string; oracle: string }
  /** One per unequal pair: a header line (`web vs desktop`) and the diff. */
  diffs: Array<string>
}

export function compareThree(r: { web: NormReports; desktop: NormReports; oracle: NormReports }): Comparison {
  const texts = { web: canonicalText(r.web), desktop: canonicalText(r.desktop), oracle: canonicalText(r.oracle) }
  const pairs: Array<[keyof typeof texts, keyof typeof texts]> = [
    ['web', 'desktop'],
    ['web', 'oracle'],
    ['desktop', 'oracle'],
  ]
  const diffs = pairs.filter(([a, b]) => texts[a] !== texts[b]).map(([a, b]) => `${a} vs ${b}\n${lineDiff(texts[a], texts[b])}`)
  return { equal: diffs.length === 0, texts, diffs }
}
