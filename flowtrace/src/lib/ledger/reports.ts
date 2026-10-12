// Reports read posted journal lines only. Every report here foots: the
// trial balance nets to zero, the Statement of Financial Position balances,
// and the Statement of Activities' change in net assets equals the change
// shown on the balance sheet.
import { sum } from './money'
import type {
  Account,
  AccountType,
  ExpenseFunction,
  Fund,
  ISODate,
  JournalEntry,
  JournalLine,
  LedgerState,
  Minor,
} from './types'

export type Basis = 'accrual' | 'cash'

export interface LineFilter {
  from?: ISODate
  to?: ISODate
  accountIds?: Array<string>
  fundId?: string
  grantId?: string
  dimensionId?: string
  basis?: Basis
  /** Leave out the year-end closing entries (for activity reports). */
  excludeClosing?: boolean
}

export interface ReportLine extends JournalLine {
  entry: JournalEntry
}

const inRange = (d: ISODate, f: LineFilter) =>
  (!f.from || d >= f.from) && (!f.to || d <= f.to)

/** Account plus every account below it in the tree. */
export function descendants(state: LedgerState, accountId: string) {
  const out = new Set([accountId])
  let grew = true
  while (grew) {
    grew = false
    for (const a of state.accounts)
      if (a.parent_id && out.has(a.parent_id) && !out.has(a.id)) {
        out.add(a.id)
        grew = true
      }
  }
  return out
}

/**
 * Posted lines (reversed entries count, offset by their reversals). With
 * `basis: 'cash'`, accruals are dropped and each bill payment or claim
 * reimbursement is re-coded to the expense lines it settled, pro rata.
 */
export function postedLines(state: LedgerState, f: LineFilter = {}): Array<ReportLine> {
  const entries = new Map(state.entries.map((e) => [e.id, e]))
  let lines: Array<ReportLine> = []
  for (const l of state.lines) {
    const entry = entries.get(l.entry_id)
    if (!entry || entry.status === 'draft') continue
    if (f.excludeClosing && entry.source_type === 'year_end_close') continue
    lines.push({ ...l, entry })
  }

  if (f.basis === 'cash') lines = toCashBasis(state, lines)

  const accountSet = f.accountIds
    ? new Set(f.accountIds.flatMap((id) => [...descendants(state, id)]))
    : null
  return lines.filter(
    (l) =>
      inRange(l.entry.date, f) &&
      (!accountSet || accountSet.has(l.account_id)) &&
      (!f.fundId || l.fund_id === f.fundId) &&
      (!f.grantId || l.grant_id === f.grantId) &&
      (!f.dimensionId || l.dimension_ids.includes(f.dimensionId)),
  )
}

function toCashBasis(state: LedgerState, lines: Array<ReportLine>): Array<ReportLine> {
  const sys = state.settings.system_accounts
  const payables = new Set([sys.accounts_payable, sys.claims_payable])
  const accrualSources = new Set(['bill', 'expense_claim'])
  const out: Array<ReportLine> = []
  for (const l of lines) {
    if (accrualSources.has(l.entry.source_type)) continue
    const isSettlement =
      (l.entry.source_type === 'payment' || l.entry.source_type === 'reimbursement') &&
      payables.has(l.account_id)
    if (!isSettlement) {
      out.push(l)
      continue
    }
    // Replace the payable debit with the document's expense coding.
    const doc =
      l.entry.source_type === 'payment'
        ? state.bills.find((b) => b.id === l.entry.source_id)
        : state.claims.find((c) => c.id === l.entry.source_id)
    if (!doc) {
      out.push(l)
      continue
    }
    const fundLines = doc.lines.filter((d) => d.fund_id === l.fund_id)
    const fundTotal = sum(fundLines.map((d) => d.amount))
    let allocated = 0
    fundLines.forEach((d, i) => {
      const part =
        i === fundLines.length - 1
          ? l.base_amount - allocated
          : Math.round((l.base_amount * d.amount) / fundTotal)
      allocated += part
      out.push({
        ...l,
        id: `${l.id}:${d.id}`,
        account_id: d.account_id,
        grant_id: d.grant_id,
        dimension_ids: d.dimension_ids,
        debit: part > 0 ? part : 0,
        credit: part < 0 ? -part : 0,
        base_amount: part,
        memo: d.description,
      })
    })
  }
  return out
}

/** Signed debit-minus-credit balance per account, in base currency. */
export function balances(state: LedgerState, f: LineFilter = {}) {
  const m = new Map<string, Minor>()
  for (const l of postedLines(state, f)) m.set(l.account_id, (m.get(l.account_id) ?? 0) + l.base_amount)
  return m
}

/** Balance shown the natural way round (credit accounts positive). */
export const natural = (a: Pick<Account, 'normal_balance'>, signed: Minor) =>
  a.normal_balance === 'debit' ? signed : -signed

// ---------------------------------------------------------------------------
// Trial balance and general ledger
// ---------------------------------------------------------------------------

export interface TrialBalanceRow {
  account: Account
  debit: Minor
  credit: Minor
}

export function trialBalance(state: LedgerState, asOf?: ISODate, basis: Basis = 'accrual') {
  const bal = balances(state, { to: asOf, basis })
  const rows: Array<TrialBalanceRow> = []
  for (const a of [...state.accounts].sort(byCode)) {
    const b = bal.get(a.id) ?? 0
    if (b === 0) continue
    rows.push({ account: a, debit: b > 0 ? b : 0, credit: b < 0 ? -b : 0 })
  }
  const debit = sum(rows.map((r) => r.debit))
  const credit = sum(rows.map((r) => r.credit))
  return { rows, debit, credit, difference: debit - credit }
}

export interface GeneralLedgerRow {
  line: ReportLine
  account: Account
  running: Minor
}

export function generalLedger(state: LedgerState, f: LineFilter = {}) {
  const lines = postedLines(state, f).sort(
    (a, b) =>
      a.entry.date.localeCompare(b.entry.date) ||
      a.entry.number.localeCompare(b.entry.number),
  )
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  // Opening balance per account before the range.
  const opening = f.from
    ? balances(state, { ...f, from: undefined, to: dayBefore(f.from) })
    : new Map<string, Minor>()
  const running = new Map(opening)
  const rows: Array<GeneralLedgerRow> = lines.map((line) => {
    const r = (running.get(line.account_id) ?? 0) + line.base_amount
    running.set(line.account_id, r)
    return { line, account: accounts.get(line.account_id)!, running: r }
  })
  return { rows, opening, closing: running }
}

export function dayBefore(d: ISODate): ISODate {
  const t = new Date(`${d}T00:00:00Z`)
  t.setUTCDate(t.getUTCDate() - 1)
  return t.toISOString().slice(0, 10)
}

const byCode = (a: Account, b: Account) => a.code.localeCompare(b.code)

// ---------------------------------------------------------------------------
// Statement of Financial Position
// ---------------------------------------------------------------------------

export interface StatementRow {
  account: Account
  amount: Minor
}

export interface FinancialPosition {
  asOf: ISODate
  assets: Array<StatementRow>
  liabilities: Array<StatementRow>
  netAssets: {
    unrestricted: Minor
    restricted: Minor
    rows: Array<StatementRow>
  }
  totalAssets: Minor
  totalLiabilities: Minor
  totalNetAssets: Minor
  /** Assets − (liabilities + net assets). Always 0. */
  difference: Minor
}

function restrictedFunds(state: LedgerState) {
  return new Set(state.funds.filter((f) => f.restriction !== 'unrestricted').map((f) => f.id))
}

export function financialPosition(state: LedgerState, asOf: ISODate, basis: Basis = 'accrual'): FinancialPosition {
  const lines = postedLines(state, { to: asOf, basis })
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const restricted = restrictedFunds(state)
  const bal = new Map<string, Minor>()
  let unrestricted = 0
  let restrictedNA = 0
  for (const l of lines) {
    const a = accounts.get(l.account_id)!
    bal.set(a.id, (bal.get(a.id) ?? 0) + l.base_amount)
    // Equity, income and expense together are net assets, split by fund.
    if (a.type === 'equity' || a.type === 'income' || a.type === 'expense') {
      if (restricted.has(l.fund_id)) restrictedNA -= l.base_amount
      else unrestricted -= l.base_amount
    }
  }
  const rowsOf = (type: AccountType) =>
    [...state.accounts]
      .filter((a) => a.type === type && (bal.get(a.id) ?? 0) !== 0)
      .sort(byCode)
      .map((a) => ({ account: a, amount: natural(a, bal.get(a.id)!) }))
  const assets = rowsOf('asset')
  const liabilities = rowsOf('liability')
  const totalAssets = sum(assets.map((r) => r.amount))
  const totalLiabilities = sum(liabilities.map((r) => r.amount))
  const totalNetAssets = unrestricted + restrictedNA
  return {
    asOf,
    assets,
    liabilities,
    netAssets: { unrestricted, restricted: restrictedNA, rows: rowsOf('equity') },
    totalAssets,
    totalLiabilities,
    totalNetAssets,
    difference: totalAssets - totalLiabilities - totalNetAssets,
  }
}

// ---------------------------------------------------------------------------
// Statement of Activities, by fund restriction
// ---------------------------------------------------------------------------

export interface ActivityRow {
  account: Account
  unrestricted: Minor
  restricted: Minor
  total: Minor
}

export interface Activities {
  from: ISODate
  to: ISODate
  income: Array<ActivityRow>
  expenses: Array<ActivityRow>
  totalIncome: { unrestricted: Minor; restricted: Minor; total: Minor }
  totalExpenses: { unrestricted: Minor; restricted: Minor; total: Minor }
  changeInNetAssets: { unrestricted: Minor; restricted: Minor; total: Minor }
  /** Per fund: income, expense and change, for the fund schedule. */
  byFund: Array<{ fund: Fund; income: Minor; expenses: Minor; change: Minor }>
}

export function activities(state: LedgerState, from: ISODate, to: ISODate, basis: Basis = 'accrual'): Activities {
  const lines = postedLines(state, { from, to, basis, excludeClosing: true })
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const restricted = restrictedFunds(state)
  const cells = new Map<string, { u: Minor; r: Minor }>()
  const fundTotals = new Map<string, { income: Minor; expenses: Minor }>()
  for (const l of lines) {
    const a = accounts.get(l.account_id)!
    if (a.type !== 'income' && a.type !== 'expense') continue
    const v = natural(a, l.base_amount)
    const c = cells.get(a.id) ?? { u: 0, r: 0 }
    if (restricted.has(l.fund_id)) c.r += v
    else c.u += v
    cells.set(a.id, c)
    const ft = fundTotals.get(l.fund_id) ?? { income: 0, expenses: 0 }
    if (a.type === 'income') ft.income += v
    else ft.expenses += v
    fundTotals.set(l.fund_id, ft)
  }
  const rowsOf = (type: AccountType) =>
    [...state.accounts]
      .filter((a) => a.type === type && cells.has(a.id))
      .sort(byCode)
      .map((a) => {
        const c = cells.get(a.id)!
        return { account: a, unrestricted: c.u, restricted: c.r, total: c.u + c.r }
      })
      .filter((r) => r.total !== 0 || r.unrestricted !== 0)
  const income = rowsOf('income')
  const expenses = rowsOf('expense')
  const tot = (rows: Array<ActivityRow>) => ({
    unrestricted: sum(rows.map((r) => r.unrestricted)),
    restricted: sum(rows.map((r) => r.restricted)),
    total: sum(rows.map((r) => r.total)),
  })
  const ti = tot(income)
  const te = tot(expenses)
  return {
    from,
    to,
    income,
    expenses,
    totalIncome: ti,
    totalExpenses: te,
    changeInNetAssets: {
      unrestricted: ti.unrestricted - te.unrestricted,
      restricted: ti.restricted - te.restricted,
      total: ti.total - te.total,
    },
    byFund: state.funds
      .filter((f) => fundTotals.has(f.id))
      .map((fund) => {
        const t = fundTotals.get(fund.id)!
        return { fund, income: t.income, expenses: t.expenses, change: t.income - t.expenses }
      }),
  }
}

// ---------------------------------------------------------------------------
// Statement of Functional Expenses
// ---------------------------------------------------------------------------

export interface FunctionalRow {
  account: Account
  program: Minor
  management: Minor
  fundraising: Minor
  total: Minor
}

export function functionalExpenses(state: LedgerState, from: ISODate, to: ISODate, basis: Basis = 'accrual') {
  const lines = postedLines(state, { from, to, basis, excludeClosing: true })
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const rows = new Map<string, FunctionalRow>()
  for (const l of lines) {
    const a = accounts.get(l.account_id)!
    if (a.type !== 'expense') continue
    const fn: ExpenseFunction = a.expense_function ?? 'program'
    const r = rows.get(a.id) ?? { account: a, program: 0, management: 0, fundraising: 0, total: 0 }
    r[fn] += l.base_amount
    r.total += l.base_amount
    rows.set(a.id, r)
  }
  const list = [...rows.values()].filter((r) => r.total !== 0).sort((a, b) => byCode(a.account, b.account))
  const totals = {
    program: sum(list.map((r) => r.program)),
    management: sum(list.map((r) => r.management)),
    fundraising: sum(list.map((r) => r.fundraising)),
    total: sum(list.map((r) => r.total)),
  }
  return {
    rows: list,
    totals,
    /** Share of spend that went to programmes, 0–1. */
    programRatio: totals.total > 0 ? totals.program / totals.total : 0,
  }
}

// ---------------------------------------------------------------------------
// Cash flow (direct method)
// ---------------------------------------------------------------------------

export interface CashFlow {
  opening: Minor
  closing: Minor
  operating: { receipts: Minor; payments: Minor; net: Minor }
  investing: Minor
  financing: Minor
  net: Minor
  /** opening + net − closing. Always 0. */
  difference: Minor
}

export function cashFlow(state: LedgerState, from: ISODate, to: ISODate): CashFlow {
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const isCash = (id: string) => !!accounts.get(id)?.is_cash
  const cashBal = (lines: Array<ReportLine>) => sum(lines.filter((l) => isCash(l.account_id)).map((l) => l.base_amount))
  const opening = cashBal(postedLines(state, { to: dayBefore(from) }))
  const lines = postedLines(state, { from, to })
  const byEntry = new Map<string, Array<ReportLine>>()
  for (const l of lines) byEntry.set(l.entry_id, [...(byEntry.get(l.entry_id) ?? []), l])

  let receipts = 0
  let payments = 0
  let investing = 0
  let financing = 0
  for (const group of byEntry.values()) {
    const cashMove = sum(group.filter((l) => isCash(l.account_id)).map((l) => l.base_amount))
    if (cashMove === 0) continue // transfers between cash accounts
    // Classify by the largest non-cash counterpart.
    const counter = group
      .filter((l) => !isCash(l.account_id))
      .sort((a, b) => Math.abs(b.base_amount) - Math.abs(a.base_amount))
      .at(0)
    const cls = counter ? (accounts.get(counter.account_id)?.cash_flow_class ?? 'operating') : 'operating'
    if (cls === 'investing') investing += cashMove
    else if (cls === 'financing') financing += cashMove
    else if (cashMove > 0) receipts += cashMove
    else payments += cashMove
  }
  const operatingNet = receipts + payments
  const net = operatingNet + investing + financing
  const closing = cashBal(postedLines(state, { to }))
  return {
    opening,
    closing,
    operating: { receipts, payments, net: operatingNet },
    investing,
    financing,
    net,
    difference: opening + net - closing,
  }
}

// ---------------------------------------------------------------------------
// Budgets and grants
// ---------------------------------------------------------------------------

export interface BudgetVsActualRow {
  lineId: string
  label: string
  account: Account | null
  budget: Minor
  actual: Minor
  variance: Minor
  /** actual / budget, 0 when there is no budget. */
  used: number
  status: 'within' | 'warning' | 'over'
}

export function warningLevel(used: number): BudgetVsActualRow['status'] {
  if (used > 1) return 'over'
  if (used >= 0.8) return 'warning'
  return 'within'
}

export function budgetVsActual(
  state: LedgerState,
  opts: { budgetId?: string; grantId?: string; dimensionId?: string; to?: ISODate; basis?: Basis } = {},
): Array<BudgetVsActualRow> {
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const periods = new Map(state.periods.map((p) => [p.id, p]))
  const budgets = new Map(state.budgets.map((b) => [b.id, b]))
  const expenseIds = state.accounts.filter((a) => a.type === 'expense').map((a) => a.id)
  return state.budgetLines
    .filter(
      (bl) =>
        (!opts.budgetId || bl.budget_id === opts.budgetId) &&
        (!opts.grantId || bl.grant_id === opts.grantId) &&
        (!opts.dimensionId || bl.dimension_id === opts.dimensionId),
    )
    .map((bl) => {
      const budget = budgets.get(bl.budget_id)
      const period = bl.period_id ? periods.get(bl.period_id) : undefined
      const yearPeriods = state.periods.filter((p) => p.fiscal_year === budget?.fiscal_year)
      const from = period?.start_date ?? yearPeriods[0]?.start_date
      let to = period?.end_date ?? yearPeriods[yearPeriods.length - 1]?.end_date
      if (opts.to && (!to || opts.to < to)) to = opts.to
      const actual = sum(
        postedLines(state, {
          from,
          to,
          basis: opts.basis,
          excludeClosing: true,
          accountIds: bl.account_id ? [bl.account_id] : expenseIds,
          fundId: bl.fund_id ?? undefined,
          grantId: bl.grant_id ?? undefined,
          dimensionId: bl.dimension_id ?? undefined,
        })
          .filter((l) => accounts.get(l.account_id)?.type === 'expense')
          .map((l) => l.base_amount),
      )
      const account = bl.account_id ? (accounts.get(bl.account_id) ?? null) : null
      const used = bl.amount > 0 ? actual / bl.amount : actual > 0 ? Infinity : 0
      const dim = bl.dimension_id ? state.dimensions.find((d) => d.id === bl.dimension_id) : null
      const grant = bl.grant_id ? state.grants.find((g) => g.id === bl.grant_id) : null
      const label = [account ? `${account.code} ${account.name}` : 'All expenses', grant?.name, dim?.name, period?.name]
        .filter(Boolean)
        .join(' · ')
      return {
        lineId: bl.id,
        label,
        account,
        budget: bl.amount,
        actual,
        variance: bl.amount - actual,
        used,
        status: warningLevel(used),
      }
    })
}

export interface GrantSummary {
  grantId: string
  award: Minor
  received: Minor
  spent: Minor
  /** Approved bills not yet posted, and submitted claims, coded to the grant. */
  committed: Minor
  remaining: Minor
  used: number
  status: BudgetVsActualRow['status']
  /** Average spend per elapsed month. */
  burnPerMonth: Minor
  /** Months of award left at the current burn, or null when not spending. */
  monthsLeft: number | null
  /** Months until the grant ends. */
  monthsToEnd: number
}

function monthsBetween(a: ISODate, b: ISODate) {
  const da = new Date(`${a}T00:00:00Z`)
  const db = new Date(`${b}T00:00:00Z`)
  return (db.getTime() - da.getTime()) / (1000 * 60 * 60 * 24 * 30.4375)
}

export function grantSummary(state: LedgerState, grantId: string, asOf: ISODate): GrantSummary {
  const grant = state.grants.find((g) => g.id === grantId)!
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const lines = postedLines(state, { grantId, to: asOf, excludeClosing: true })
  const spent = sum(lines.filter((l) => accounts.get(l.account_id)?.type === 'expense').map((l) => l.base_amount))
  const received = -sum(lines.filter((l) => accounts.get(l.account_id)?.type === 'income').map((l) => l.base_amount))
  const codedTo = (docLines: Array<{ grant_id: string | null; amount: Minor }>) =>
    sum(docLines.filter((l) => l.grant_id === grantId).map((l) => l.amount))
  const committed =
    sum(state.bills.filter((b) => b.status === 'approved').map((b) => codedTo(b.lines))) +
    sum(state.claims.filter((c) => c.status === 'submitted').map((c) => codedTo(c.lines)))
  const remaining = grant.award - spent - committed
  const used = grant.award > 0 ? (spent + committed) / grant.award : 0
  const elapsed = Math.max(1, monthsBetween(grant.start_date, asOf < grant.end_date ? asOf : grant.end_date))
  const burnPerMonth = Math.round(spent / elapsed)
  return {
    grantId,
    award: grant.award,
    received,
    spent,
    committed,
    remaining,
    used,
    status: warningLevel(used),
    burnPerMonth,
    monthsLeft: burnPerMonth > 0 ? Math.max(0, (grant.award - spent) / burnPerMonth) : null,
    monthsToEnd: Math.max(0, monthsBetween(asOf, grant.end_date)),
  }
}

// ---------------------------------------------------------------------------
// Ledger signals for the trust score
// ---------------------------------------------------------------------------

export interface LedgerSignals {
  /** Share of cash movement (by value) matched to a bank statement line. */
  reconciledShare: number
  /** Share of budgeted spend whose budget line is not over. */
  inBudgetShare: number
  /** Share of spend coded to a grant. */
  grantCodedShare: number
}

export function ledgerSignals(state: LedgerState, asOf?: ISODate): LedgerSignals {
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const lines = postedLines(state, { to: asOf, excludeClosing: true })
  const reconcilable = new Set(state.bankAccounts.map((b) => b.account_id))
  const cash = lines.filter((l) => reconcilable.has(l.account_id) && l.entry.source_type !== 'opening')
  const matched = new Set(state.statementLines.map((s) => s.matched_line_id).filter(Boolean))
  const cashValue = sum(cash.map((l) => Math.abs(l.base_amount)))
  const matchedValue = sum(cash.filter((l) => matched.has(l.id)).map((l) => Math.abs(l.base_amount)))

  const spend = lines.filter((l) => accounts.get(l.account_id)?.type === 'expense')
  const spendValue = sum(spend.map((l) => l.base_amount))
  const grantValue = sum(spend.filter((l) => l.grant_id).map((l) => l.base_amount))

  const bva = budgetVsActual(state, { to: asOf })
  const budgeted = sum(bva.map((r) => r.actual))
  const inBudget = sum(bva.filter((r) => r.status !== 'over').map((r) => r.actual))

  const share = (a: Minor, b: Minor) => (b > 0 ? Math.min(1, Math.max(0, a / b)) : 0)
  return {
    reconciledShare: share(matchedValue, cashValue),
    inBudgetShare: budgeted > 0 ? share(inBudget, budgeted) : 1,
    grantCodedShare: share(grantValue, spendValue),
  }
}
