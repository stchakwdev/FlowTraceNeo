// The IT1 report comparison (it1-report.ts): the web's figures as shown, the desktop's and the
// oracle's statements from it1_probe, all reduced to one normal form and required byte-identical.
// No network.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { canonicalText, compareThree, fromDom, fromProbe, parseCell } from './it1-report'
import type { DomReport, ProbeReports } from './it1-report'

describe('parseCell', () => {
  test('reads formatMinor figures as integer minor units', () => {
    // flowtrace/src/components/ledger/money.tsx:8-21: en-KE grouping, two decimals, negatives in
    // parentheses (or with a minus sign when signed), zero as an en dash.
    expect(parseCell('1,234.50')).toBe(123450)
    expect(parseCell('(1,234.50)')).toBe(-123450)
    expect(parseCell('−18,200.00')).toBe(-1820000)
    expect(parseCell('-0.05')).toBe(-5)
    expect(parseCell('–')).toBe(0)
    expect(parseCell('0.00')).toBe(0)
    expect(parseCell('27,615,750.00')).toBe(2761575000)
  })

  test('keeps anything else as trimmed text', () => {
    expect(parseCell(' 12.5% ')).toBe('12.5%')
    expect(parseCell('n/a')).toBe('n/a')
    expect(parseCell('1,234.5')).toBe('1,234.5')
  })

  test('takes numbers from the probe as they are', () => {
    expect(parseCell(123450)).toBe(123450)
  })
})

/** The probe's statement JSON (it1_probe reports). */
const probeReports = (balance: number): ProbeReports => ({
  'trial-balance': {
    title: 'Trial balance',
    tables: [
      {
        title: null,
        first_column: 'Account',
        columns: ['Debit', 'Credit'],
        rows: [
          { kind: 'heading', code: null, label: 'Assets', detail: null, cells: [] },
          { kind: 'line', code: '1110', label: 'Cash at bank', detail: null, cells: [balance, 0] },
          { kind: 'gap', code: null, label: '', detail: null, cells: [] },
          { kind: 'total', code: null, label: 'Total', detail: null, cells: [balance, 0] },
        ],
        empty: null,
      },
      { title: 'Nothing', first_column: 'Grant', columns: ['Amount'], rows: [], empty: 'No grants.' },
    ],
    footings: [{ statement: 'Debits equal credits.', difference: 0 }],
  },
})

/** The same statement as the web's DOM shows it (it1-web.ts reads it). */
const domReports = (shown: string): Record<string, DomReport> => ({
  'trial-balance': {
    tables: [
      {
        columns: ['Account', 'Debit', 'Credit'],
        rows: [
          { kind: 'heading', code: null, label: 'Assets', detail: null, cells: [] },
          { kind: 'line', code: '1110', label: 'Cash at bank', detail: null, cells: [shown, '–'] },
          { kind: 'total', code: null, label: 'Total', detail: null, cells: [shown, '–'] },
        ],
      },
    ],
    footings: ['0.00'],
  },
})

describe('normal form', () => {
  test('the web DOM and the probe JSON of the same statement normalise to the same bytes', () => {
    const a = canonicalText(fromProbe(probeReports(123450)))
    const b = canonicalText(fromDom(domReports('1,234.50')))
    expect(a).toBe(b)
  })

  test('sorted keys, integer minor units, gaps and empty tables dropped', () => {
    const text = canonicalText(fromProbe(probeReports(123450)))
    expect(text).toContain('"cells":[123450,0]')
    expect(text).not.toContain('gap')
    expect(text).not.toContain('Nothing')
    const keys = [...text.matchAll(/"([a-z_]+)":/g)].map((m) => m[1])
    const row = keys.slice(keys.indexOf('cells'), keys.indexOf('cells') + 5)
    expect(row).toEqual(['cells', 'code', 'detail', 'kind', 'label'])
  })
})

describe('compareThree', () => {
  test('passes when all three are identical', () => {
    const r = compareThree({
      web: fromDom(domReports('1,234.50')),
      desktop: fromProbe(probeReports(123450)),
      oracle: fromProbe(probeReports(123450)),
    })
    expect(r.equal).toBe(true)
    expect(r.diffs).toEqual([])
  })

  test('a single cell one minor unit off fails and is shown as a diff', () => {
    const r = compareThree({
      web: fromDom(domReports('1,234.50')),
      desktop: fromProbe(probeReports(123450)),
      oracle: fromProbe(probeReports(123451)),
    })
    expect(r.equal).toBe(false)
    expect(r.diffs.map((d) => d.split('\n')[0])).toEqual(['web vs oracle', 'desktop vs oracle'])
    const lines = r.diffs[1].split('\n')
    expect(lines.filter((l) => l.startsWith('- ')).length).toBe(2)
    expect(lines.filter((l) => l.startsWith('+ ')).length).toBe(2)
    expect(lines.find((l) => l.startsWith('- ') && l.includes('Cash at bank'))).toContain('"cells":[123450,0]')
    expect(lines.find((l) => l.startsWith('+ ') && l.includes('Cash at bank'))).toContain('"cells":[123451,0]')
  })

  test('a label difference on the web alone is caught against both others', () => {
    const web = domReports('1,234.50')
    web['trial-balance'].tables[0].rows[1].label = 'Cash in bank'
    const r = compareThree({ web: fromDom(web), desktop: fromProbe(probeReports(123450)), oracle: fromProbe(probeReports(123450)) })
    expect(r.equal).toBe(false)
    expect(r.diffs.map((d) => d.split('\n')[0])).toEqual(['web vs desktop', 'web vs oracle'])
  })

  test('a missing report fails', () => {
    const r = compareThree({ web: {}, desktop: fromProbe(probeReports(1)), oracle: fromProbe(probeReports(1)) })
    expect(r.equal).toBe(false)
  })
})
