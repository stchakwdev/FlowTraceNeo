// One fixture per export of reports.ts, run over two sets of books: the demo
// books, and the books after the accepted-commands scenario (drafts, a
// reversal, a foreign-currency bill, a rejected claim, a locked period and a
// year-end close). Dates and filters come from the books themselves: every
// period with a posted entry, its fiscal year, and the demo as-of date.
import { canonical } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import {
  accountId,
  activities,
  balances,
  budgetVsActual,
  cashFlow,
  dayBefore,
  descendants,
  financialPosition,
  functionalExpenses,
  generalLedger,
  grantSummary,
  ledgerSignals,
  natural,
  postedLines,
  trialBalance,
  warningLevel,
} from './flowtrace'
import type { LedgerState } from './flowtrace'
import { DEMO_AS_OF } from './demo'
import { src } from './sources'

type Books = Record<string, LedgerState>

const STATE_REFS = {
  'demo-books': 'demo-books.json#/state',
  'commands-final': 'commands-accepted.json#/final_state',
}

const BASES = ['accrual', 'cash'] as const

const kebab = (s: string) => s.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()
const uniq = <T>(xs: Array<T>) => [...new Set(xs)]
const sorted = (xs: Array<string>) => uniq(xs).sort()

/** Periods with a posted (or reversed) entry, in period order. */
function activePeriods(state: LedgerState) {
  const ids = new Set(state.entries.filter((e) => e.status !== 'draft').map((e) => e.period_id))
  return state.periods.filter((p) => ids.has(p.id))
}

function yearsOf(state: LedgerState) {
  return uniq(activePeriods(state).map((p) => p.fiscal_year)).map((y) => {
    const ps = state.periods.filter((p) => p.fiscal_year === y)
    return { year: y, from: ps[0].start_date, to: ps[ps.length - 1].end_date }
  })
}

/** Month ends and year ends with activity, and the demo as-of date. */
function asOfs(state: LedgerState) {
  return sorted([...activePeriods(state).map((p) => p.end_date), ...yearsOf(state).map((y) => y.to), DEMO_AS_OF])
}

/** Each active period, each active fiscal year, and the year to the demo as-of date. */
function ranges(state: LedgerState): Array<[string, string]> {
  const out: Array<[string, string]> = activePeriods(state).map((p) => [p.start_date, p.end_date])
  for (const y of yearsOf(state)) out.push([y.from, y.to])
  const asOfYear = state.periods.find((p) => p.start_date <= DEMO_AS_OF && DEMO_AS_OF <= p.end_date)
  const first = state.periods.find((p) => p.fiscal_year === asOfYear?.fiscal_year)
  if (first) out.push([first.start_date, DEMO_AS_OF])
  return uniq(out.map((r) => r.join('|'))).map((r) => r.split('|') as [string, string])
}

const usedDimension = (state: LedgerState) => state.lines.find((l) => l.dimension_ids.length > 0)?.dimension_ids[0]
const budgetDimension = (state: LedgerState) => state.budgetLines.find((b) => b.dimension_id)?.dimension_id ?? undefined

interface Case {
  state?: string
  args: Json
  result: Json
}

function run(books: Books, fn: (state: LedgerState, ...args: Array<never>) => unknown, argsFor: (state: LedgerState) => Array<Array<unknown>>) {
  const cases: Array<Case> = []
  for (const [name, state] of Object.entries(books)) {
    for (const args of argsFor(state)) {
      const before = JSON.stringify(state)
      const result = fn(structuredClone(state), ...(args as Array<never>))
      if (JSON.stringify(state) !== before) throw new Error(`${fn.name} changed the books`)
      cases.push({ state: name, args: canonical(args), result: canonical(result) })
    }
  }
  return cases
}

function pure(fn: (...args: Array<never>) => unknown, argsList: Array<Array<unknown>>): Array<Case> {
  return argsList.map((args) => ({ args: canonical(args), result: canonical(fn(...(args as Array<never>))) }))
}

/**
 * byCode and the general ledger sort with String.prototype.localeCompare
 * (reports.ts:184-185, 207), which depends on the ICU locale. Fails unless,
 * for every set of books, it orders account codes, entry dates and entry
 * numbers exactly as a plain code-unit (byte) comparison does, so a Rust
 * port may sort by bytes.
 */
export function assertCollationIsByteOrder(books: Books) {
  const bytes = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)
  const same = (xs: Array<string>, what: string) => {
    const byLocale = [...xs].sort((a, b) => a.localeCompare(b)).join('\n')
    if (byLocale !== [...xs].sort(bytes).join('\n')) throw new Error(`localeCompare and byte order disagree on ${what}`)
  }
  for (const [name, s] of Object.entries(books)) {
    same(s.accounts.map((a) => a.code), `${name} account codes`)
    same(s.entries.map((e) => e.date), `${name} entry dates`)
    same(s.entries.map((e) => e.number), `${name} entry numbers`)
  }
}

export function exportReports(outDir: string, books: Books) {
  assertCollationIsByteOrder(books)
  const write = (fn: { name: string }, signature: string, cases: Array<Case>, note: string) =>
    writeFixture(outDir, `report-${kebab(fn.name)}`, {
      description: `${fn.name}${signature} from reports.ts. ${note} Each case: the books it ran on (see states), ` +
        'the arguments after the books (undefined written as null), and the return value.',
      sources: [src('reports.ts', fn.name)],
      covers: [fn.name],
      env: null,
      function: fn.name,
      signature: `${fn.name}${signature}`,
      states: cases.some((c) => c.state) ? STATE_REFS : null,
      cases,
    })

  write(trialBalance, '(state, asOf?, basis = "accrual")',
    run(books, trialBalance, (s) => [undefined, ...asOfs(s)].flatMap((d) => BASES.map((b) => [d, b]))),
    'Every as-of date (none, month ends and year ends with activity, the demo as-of date), both bases.')

  write(balances, '(state, filter = {})',
    run(books, balances, (s) => [
      [{}], [{ basis: 'cash' }], [{ excludeClosing: true }],
      ...asOfs(s).map((to) => [{ to }]),
      ...ranges(s).map(([from, to]) => [{ from, to }]),
      ...s.funds.map((f) => [{ fundId: f.id }]),
      ...s.grants.map((g) => [{ grantId: g.id }]),
      [{ accountIds: [accountId('1100')] }],
      [{ dimensionId: usedDimension(s) }],
    ]),
    'The Map result is written as an object keyed by account id.')

  write(postedLines, '(state, filter = {})',
    run(books, postedLines, (s) => [
      [{}], [{ basis: 'cash' }],
      [{ from: ranges(s)[1][0], to: ranges(s)[1][1], excludeClosing: true }],
      [{ accountIds: [accountId('1100')], basis: 'cash' }],
      [{ grantId: 'grant-kisima-wash' }],
      [{ dimensionId: usedDimension(s) }],
    ]),
    'Includes cash-basis re-coding of payments and reimbursements (ids of re-coded lines are "<line id>:<document line id>").')

  write(generalLedger, '(state, filter = {})',
    run(books, generalLedger, (s) => [
      [{}],
      [{ from: ranges(s)[1][0], to: ranges(s)[1][1] }],
      [{ from: ranges(s)[1][0], to: ranges(s)[1][1], accountIds: [accountId('1100')] }],
      [{ basis: 'cash', from: '2026-01-01', to: DEMO_AS_OF }],
    ]),
    'opening and closing are Maps, written as objects keyed by account id.')

  write(financialPosition, '(state, asOf, basis = "accrual")',
    run(books, financialPosition, (s) => asOfs(s).flatMap((d) => BASES.map((b) => [d, b]))),
    'Every as-of date with activity and the demo as-of date, both bases.')

  write(activities, '(state, from, to, basis = "accrual")',
    run(books, activities, (s) => ranges(s).flatMap(([f, t]) => BASES.map((b) => [f, t, b]))),
    'Every active period, every active fiscal year and the year to the demo as-of date, both bases.')

  write(functionalExpenses, '(state, from, to, basis = "accrual")',
    run(books, functionalExpenses, (s) => ranges(s).flatMap(([f, t]) => BASES.map((b) => [f, t, b]))),
    'Same ranges as activities. programRatio is a float.')

  write(cashFlow, '(state, from, to)',
    run(books, cashFlow, (s) => ranges(s).map(([f, t]) => [f, t])),
    'Same ranges as activities.')

  write(budgetVsActual, '(state, opts = {})',
    run(books, budgetVsActual, (s) => [
      [{}], [{ basis: 'cash' }], [{ budgetId: 'budget-2026' }],
      ...s.grants.map((g) => [{ grantId: g.id }]),
      [{ dimensionId: budgetDimension(s) }],
      ...asOfs(s).map((to) => [{ to }]),
    ]),
    'used is a float (actual / budget), "Infinity" when the budget is 0 and actual > 0.')

  write(grantSummary, '(state, grantId, asOf)',
    run(books, grantSummary, (s) => s.grants.flatMap((g) => asOfs(s).map((d) => [g.id, d]))),
    'Every grant at every as-of date. used, monthsLeft and monthsToEnd are floats.')

  write(ledgerSignals, '(state, asOf?)',
    run(books, ledgerSignals, (s) => [[undefined], ...asOfs(s).map((d) => [d])]),
    'All three shares are floats in 0..1.')

  write(descendants, '(state, accountId)',
    run(books, descendants, (s) => [...s.accounts.filter((a) => a.is_group).map((a) => [a.id]), [accountId('1110')], ['acc-missing']]),
    'Every header account, one posting account and an unknown id. The Set result is written as an array in insertion order.')

  write(natural, '(account, signed)',
    pure(natural, ['debit', 'credit'].flatMap((nb) => [12_345, -12_345, 0].map((n) => [{ normal_balance: nb }, n]))),
    'Pure: no books.')

  write(dayBefore, '(date)',
    pure(dayBefore, sorted([
      '2024-03-01', '2025-01-01', '2026-01-01', '2026-02-01', '2026-03-01', '2028-03-01', '2100-03-01', '2026-12-31',
      ...Object.values(books).flatMap((s) => s.periods.map((p) => p.start_date)),
    ]).map((d) => [d])),
    'Pure: no books. Every period start of both sets of books, plus leap-year edges.')

  write(warningLevel, '(used)',
    pure(warningLevel, [0, 0.5, 0.7999999, 0.8, 1, 1.0000001, 2, Infinity, -1].map((u) => [u])),
    'Pure: no books. Thresholds at 0.8 and 1.')
}
