// The ledger engine: every write goes through here, and every invariant in
// Docs/flowtrace-ledger-plan.md §5 is checked here rather than in the UI.
//
// - A posted entry balances (Σ debit = Σ credit in base currency), and it
//   balances within each fund, so fund balances always net to zero.
// - Posted entries are immutable. A correction is a reversal plus a new entry.
// - Nothing posts into a closed or locked period.
// - Expense on a restricted fund that has grants must name the grant. A grant
//   line is blocked outside the grant's dates or over its award, unless an
//   override reason is given.
// - Amounts are integer minor units; base amounts are frozen at posting.
import { LedgerError, fail } from './errors'
import { assertMinor, convert, sum } from './money'
import type { Issue } from './errors'
import type {
  Account,
  Actor,
  BankAccount,
  Bill,
  Budget,
  BudgetLine,
  CodedLine,
  DimensionValue,
  EntryInput,
  ExpenseClaim,
  FiscalPeriod,
  Fund,
  Grant,
  ISODate,
  JournalEntry,
  JournalLine,
  LedgerState,
  LineInput,
  Minor,
  OrgLedgerSettings,
  Payment,
  StatementLine,
  Vendor,
} from './types'

export interface LedgerEnv {
  now: () => string
  newId: () => string
}

export const defaultEnv: LedgerEnv = {
  now: () => new Date().toISOString(),
  newId: () => crypto.randomUUID(),
}

export function emptyLedger(settings: OrgLedgerSettings): LedgerState {
  return {
    settings,
    accounts: [],
    funds: [],
    grants: [],
    dimensions: [],
    periods: [],
    entries: [],
    lines: [],
    budgets: [],
    budgetLines: [],
    vendors: [],
    bills: [],
    payments: [],
    claims: [],
    bankAccounts: [],
    statementLines: [],
    audit: [],
    counters: {},
  }
}

const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
]

const pad = (n: number) => String(n).padStart(2, '0')

function lastDayOfMonth(year: number, month: number) {
  return new Date(Date.UTC(year, month, 0)).getUTCDate()
}

export function normalBalanceOf(type: Account['type']) {
  return type === 'asset' || type === 'expense' ? 'debit' : 'credit'
}

/**
 * A read-and-write view over one organisation's ledger. Reads are plain
 * property access on `state`; writes are the methods below.
 */
export class Ledger {
  constructor(
    public state: LedgerState,
    private env: LedgerEnv = defaultEnv,
  ) {}

  // -------------------------------------------------------------------------
  // Lookups
  // -------------------------------------------------------------------------

  account(id: string) {
    return this.state.accounts.find((a) => a.id === id)
  }
  accountByCode(code: string) {
    return this.state.accounts.find((a) => a.code === code)
  }
  fund(id: string) {
    return this.state.funds.find((f) => f.id === id)
  }
  grant(id: string) {
    return this.state.grants.find((g) => g.id === id)
  }
  entry(id: string) {
    return this.state.entries.find((e) => e.id === id)
  }
  linesOf(entryId: string) {
    return this.state.lines.filter((l) => l.entry_id === entryId)
  }
  periodFor(date: ISODate) {
    return this.state.periods.find(
      (p) => p.start_date <= date && date <= p.end_date,
    )
  }

  private nextNumber(prefix: string) {
    const n = (this.state.counters[prefix] ?? 0) + 1
    this.state.counters[prefix] = n
    return `${prefix}-${String(n).padStart(4, '0')}`
  }

  private audit(by: Actor, action: string, subject: string, detail = '') {
    this.state.audit.push({
      id: this.env.newId(),
      at: this.env.now(),
      by: by.id,
      action,
      subject,
      detail,
    })
  }

  private require(actor: Actor, perm: 'can_post' | 'can_approve' | 'can_close') {
    if (!actor[perm]) {
      const what = {
        can_post: 'post to the ledger',
        can_approve: 'approve spend',
        can_close: 'close periods',
      }[perm]
      fail('PERMISSION', `${actor.name} does not have permission to ${what}.`)
    }
  }

  // -------------------------------------------------------------------------
  // Setup: accounts, funds, grants, dimensions, periods
  // -------------------------------------------------------------------------

  addAccount(
    actor: Actor,
    input: Omit<Account, 'id' | 'normal_balance' | 'is_active'> & {
      id?: string
      is_active?: boolean
    },
  ): Account {
    this.require(actor, 'can_post')
    if (!input.code.trim() || !input.name.trim())
      fail('INVALID_INPUT', 'An account needs a code and a name.')
    if (this.accountByCode(input.code))
      fail('DUPLICATE', `Account code ${input.code} is already in use.`)
    if (input.parent_id) {
      const parent = this.account(input.parent_id)
      if (!parent) fail('UNKNOWN_ACCOUNT', 'The parent account does not exist.')
      if (!parent!.is_group)
        fail('INVALID_INPUT', 'The parent must be a header account.')
      if (parent!.type !== input.type)
        fail('INVALID_INPUT', 'A sub-account must have its parent’s type.')
    }
    const account: Account = {
      ...input,
      id: input.id ?? this.env.newId(),
      code: input.code.trim(),
      name: input.name.trim(),
      normal_balance: normalBalanceOf(input.type),
      is_active: input.is_active ?? true,
    }
    this.state.accounts.push(account)
    this.audit(actor, 'account.create', account.id, `${account.code} ${account.name}`)
    return account
  }

  updateAccount(
    actor: Actor,
    id: string,
    patch: Partial<Pick<Account, 'name' | 'is_active' | 'expense_function'>>,
  ) {
    this.require(actor, 'can_post')
    const a = this.account(id) ?? fail('NOT_FOUND', 'Account not found.')
    if (patch.name !== undefined && !patch.name.trim())
      fail('INVALID_INPUT', 'An account needs a name.')
    Object.assign(a, patch)
    this.audit(actor, 'account.update', id, JSON.stringify(patch))
    return a
  }

  addFund(actor: Actor, input: Omit<Fund, 'id'> & { id?: string }): Fund {
    this.require(actor, 'can_post')
    const fund = { ...input, id: input.id ?? this.env.newId() }
    this.state.funds.push(fund)
    this.audit(actor, 'fund.create', fund.id, fund.name)
    return fund
  }

  addGrant(actor: Actor, input: Omit<Grant, 'id'> & { id?: string }): Grant {
    this.require(actor, 'can_post')
    if (!this.fund(input.fund_id)) fail('UNKNOWN_FUND', 'Fund not found.')
    assertMinor(input.award, 'Award')
    if (input.award <= 0) fail('INVALID_INPUT', 'The award must be positive.')
    if (input.end_date < input.start_date)
      fail('INVALID_INPUT', 'A grant cannot end before it starts.')
    const grant = { ...input, id: input.id ?? this.env.newId() }
    this.state.grants.push(grant)
    this.audit(actor, 'grant.create', grant.id, grant.name)
    return grant
  }

  addDimension(
    actor: Actor,
    input: Omit<DimensionValue, 'id'> & { id?: string },
  ): DimensionValue {
    this.require(actor, 'can_post')
    const d = { ...input, id: input.id ?? this.env.newId() }
    this.state.dimensions.push(d)
    return d
  }

  /** Creates the twelve monthly periods of a fiscal year if missing. */
  ensureFiscalYear(year: number) {
    const startMonth = this.state.settings.fiscal_year_start_month
    for (let i = 0; i < 12; i++) {
      const m0 = (startMonth - 1 + i) % 12
      const y = year + Math.floor((startMonth - 1 + i) / 12) - (startMonth > 1 ? 1 : 0)
      const start = `${y}-${pad(m0 + 1)}-01`
      if (this.state.periods.some((p) => p.start_date === start)) continue
      this.state.periods.push({
        id: `period-${y}-${pad(m0 + 1)}`,
        name: `${MONTHS[m0]} ${y}`,
        fiscal_year: year,
        start_date: start,
        end_date: `${y}-${pad(m0 + 1)}-${pad(lastDayOfMonth(y, m0 + 1))}`,
        status: 'open',
      })
    }
    this.state.periods.sort((a, b) => (a.start_date < b.start_date ? -1 : 1))
  }

  // -------------------------------------------------------------------------
  // Validation
  // -------------------------------------------------------------------------

  /**
   * Every reason `input` could not post today, without changing anything.
   * Overridable issues are reported even when an override reason is given,
   * flagged with `overridable` so callers can decide.
   */
  validateEntry(input: EntryInput, ignoreEntryId?: string): Array<Issue> {
    const issues: Array<Issue> = []
    const base = this.state.settings.base_currency

    if (!/^\d{4}-\d{2}-\d{2}$/.test(input.date))
      issues.push({ code: 'INVALID_INPUT', message: 'Enter the entry date.' })
    if (!input.memo.trim())
      issues.push({ code: 'INVALID_INPUT', message: 'Describe the entry.' })

    const period = this.periodFor(input.date)
    if (!period)
      issues.push({
        code: 'NO_PERIOD',
        message: `No fiscal period covers ${input.date}.`,
      })
    else if (period.status === 'closed')
      issues.push({
        code: 'PERIOD_CLOSED',
        message: `${period.name} is closed. Reopen it or date the entry in an open period.`,
      })
    else if (period.status === 'locked')
      issues.push({
        code: 'PERIOD_LOCKED',
        message: `${period.name} is locked and can no longer take entries.`,
      })

    if (input.lines.length < 2)
      issues.push({
        code: 'TOO_FEW_LINES',
        message: 'An entry needs at least two lines.',
      })

    const byFund = new Map<string, Minor>()
    let total = 0
    // An object, not a let: TypeScript does not track writes made inside
    // the forEach callback below.
    const seen = { lineIssues: false }

    input.lines.forEach((l, i) => {
      const debit = l.debit ?? 0
      const credit = l.credit ?? 0
      const at = (code: Issue['code'], message: string, overridable = false) => {
        seen.lineIssues = true
        issues.push({ code, message: `Line ${i + 1}: ${message}`, line: i, overridable })
      }
      if (!Number.isSafeInteger(debit) || !Number.isSafeInteger(credit))
        return at('NOT_MINOR_UNITS', 'amounts must be whole cents.')
      if (debit < 0 || credit < 0)
        return at('NEGATIVE_AMOUNT', 'amounts cannot be negative.')
      if (debit > 0 && credit > 0)
        return at('LINE_BOTH_SIDES', 'use either a debit or a credit, not both.')
      if (debit === 0 && credit === 0)
        return at('LINE_ZERO', 'enter a debit or a credit.')

      const account = this.account(l.account_id)
      if (!account) return at('UNKNOWN_ACCOUNT', 'choose an account.')
      if (account.is_group)
        at('GROUP_ACCOUNT', `${account.code} ${account.name} is a header account; post to one of its sub-accounts.`)
      if (!account.is_active)
        at('INACTIVE_ACCOUNT', `${account.code} ${account.name} is inactive.`)

      const fund = this.fund(l.fund_id)
      if (!fund) return at('UNKNOWN_FUND', 'choose a fund.')

      for (const d of l.dimension_ids ?? [])
        if (!this.state.dimensions.some((x) => x.id === d))
          at('UNKNOWN_DIMENSION', 'a tag on this line no longer exists.')

      const currency = l.currency ?? base
      const rate = currency === base ? 1 : l.fx_rate
      if (!rate || !(rate > 0))
        return at('FX_RATE_REQUIRED', `enter the ${currency} to ${base} rate.`)
      const baseAmount = convert(debit - credit, rate)
      total += baseAmount
      byFund.set(fund.id, (byFund.get(fund.id) ?? 0) + baseAmount)

      // Restricted-fund and grant rules apply to spend: expense debits.
      const isSpend = account.type === 'expense' && debit > 0
      if (l.grant_id) {
        const grant = this.grant(l.grant_id)
        if (!grant) return at('UNKNOWN_GRANT', 'the grant no longer exists.')
        if (grant.fund_id !== fund.id)
          at('GRANT_FUND_MISMATCH', `${grant.name} belongs to ${this.fund(grant.fund_id)?.name ?? 'another fund'}.`)
        if (isSpend) {
          if (input.date < grant.start_date)
            at('GRANT_NOT_STARTED', `${grant.name} starts on ${grant.start_date}.`, true)
          if (input.date > grant.end_date)
            at('GRANT_ENDED', `${grant.name} ended on ${grant.end_date}.`, true)
        }
      } else if (
        isSpend &&
        fund.restriction !== 'unrestricted' &&
        this.state.grants.some((g) => g.fund_id === fund.id)
      ) {
        at('RESTRICTED_NEEDS_GRANT', `spend from ${fund.name} must name the grant it is charged to.`)
      }
    })

    if (!seen.lineIssues && input.lines.length >= 2) {
      if (total !== 0)
        issues.push({
          code: 'UNBALANCED',
          message: `Debits and credits differ by ${base} ${(Math.abs(total) / 100).toFixed(2)}.`,
        })
      else
        for (const [fundId, t] of byFund)
          if (t !== 0)
            issues.push({
              code: 'FUND_UNBALANCED',
              message: `${this.fund(fundId)?.name ?? 'A fund'} does not balance within this entry. Move money between funds with a transfer that debits and credits each fund.`,
            })
    }

    // Over-award: posted spend on the grant plus this entry's spend.
    if (!seen.lineIssues) {
      const spendByGrant = new Map<string, Minor>()
      for (const l of input.lines) {
        const a = this.account(l.account_id)
        if (!l.grant_id || a?.type !== 'expense') continue
        const rate = (l.currency ?? base) === base ? 1 : (l.fx_rate ?? 1)
        const amt = convert((l.debit ?? 0) - (l.credit ?? 0), rate)
        spendByGrant.set(l.grant_id, (spendByGrant.get(l.grant_id) ?? 0) + amt)
      }
      for (const [grantId, amount] of spendByGrant) {
        if (amount <= 0) continue
        const grant = this.grant(grantId)
        if (!grant) continue
        const spent = this.grantSpend(grantId, ignoreEntryId)
        const award = grant.currency === base ? grant.award : null
        if (award !== null && spent + amount > award)
          issues.push({
            code: 'GRANT_OVER_BUDGET',
            message: `This takes ${grant.name} over its award by ${base} ${((spent + amount - award) / 100).toFixed(2)}.`,
            overridable: true,
          })
      }
    }

    return issues
  }

  /** Posted net expense charged to a grant, in base currency. */
  grantSpend(grantId: string, ignoreEntryId?: string): Minor {
    const posted = new Set(
      this.state.entries
        .filter((e) => e.status !== 'draft' && e.id !== ignoreEntryId)
        .map((e) => e.id),
    )
    return sum(
      this.state.lines
        .filter(
          (l) =>
            l.grant_id === grantId &&
            posted.has(l.entry_id) &&
            this.account(l.account_id)?.type === 'expense',
        )
        .map((l) => l.base_amount),
    )
  }

  private blocking(issues: Array<Issue>, override: string | null | undefined) {
    return issues.filter((i) => !(i.overridable && override?.trim()))
  }

  // -------------------------------------------------------------------------
  // Journal entries
  // -------------------------------------------------------------------------

  private buildLines(entryId: string, input: EntryInput): Array<JournalLine> {
    const base = this.state.settings.base_currency
    return input.lines.map((l) => {
      const currency = l.currency ?? base
      const fx_rate = currency === base ? 1 : (l.fx_rate as number)
      const debit = l.debit ?? 0
      const credit = l.credit ?? 0
      return {
        id: this.env.newId(),
        entry_id: entryId,
        account_id: l.account_id,
        debit,
        credit,
        currency,
        fx_rate,
        base_amount: convert(debit - credit, fx_rate),
        fund_id: l.fund_id,
        grant_id: l.grant_id ?? null,
        dimension_ids: [...(l.dimension_ids ?? [])],
        flow_node_id: l.flow_node_id ?? null,
        memo: l.memo ?? null,
      }
    })
  }

  /** Saves an entry as a draft. Drafts are not checked for balance. */
  saveDraft(actor: Actor, input: EntryInput): JournalEntry {
    this.require(actor, 'can_post')
    const id = this.env.newId()
    const entry: JournalEntry = {
      id,
      number: this.nextNumber('JE'),
      date: input.date,
      period_id: this.periodFor(input.date)?.id ?? '',
      memo: input.memo.trim(),
      source_type: input.source_type ?? 'manual',
      source_id: input.source_id ?? null,
      status: 'draft',
      reversal_of: null,
      reversed_by: null,
      override_reason: input.override_reason?.trim() || null,
      created_by: actor.id,
      created_at: this.env.now(),
      posted_by: null,
      posted_at: null,
    }
    this.state.entries.push(entry)
    this.state.lines.push(...this.buildLines(id, input))
    this.audit(actor, 'entry.draft', id, entry.memo)
    return entry
  }

  /** Validates and posts a new entry in one step. */
  post(actor: Actor, input: EntryInput): JournalEntry {
    this.require(actor, 'can_post')
    const issues = this.blocking(this.validateEntry(input), input.override_reason)
    if (issues.length) throw new LedgerError(issues)
    const entry = this.saveDraft(actor, input)
    entry.status = 'posted'
    entry.posted_by = actor.id
    entry.posted_at = this.env.now()
    this.audit(actor, 'entry.post', entry.id, entry.number)
    return entry
  }

  /** The input that would recreate a stored entry. */
  inputOf(entryId: string): EntryInput {
    const e = this.entry(entryId) ?? fail('NOT_FOUND', 'Entry not found.')
    return {
      date: e.date,
      memo: e.memo,
      source_type: e.source_type,
      source_id: e.source_id,
      override_reason: e.override_reason,
      lines: this.linesOf(entryId).map((l) => ({
        account_id: l.account_id,
        debit: l.debit,
        credit: l.credit,
        currency: l.currency,
        fx_rate: l.fx_rate,
        fund_id: l.fund_id,
        grant_id: l.grant_id,
        dimension_ids: l.dimension_ids,
        flow_node_id: l.flow_node_id,
        memo: l.memo,
      })),
    }
  }

  /** Replaces a draft's content. Posted entries cannot be edited. */
  updateDraft(actor: Actor, entryId: string, input: EntryInput) {
    this.require(actor, 'can_post')
    const e = this.entry(entryId) ?? fail('NOT_FOUND', 'Entry not found.')
    if (e.status !== 'draft')
      fail('INVALID_STATE', `${e.number} is posted and cannot be changed. Reverse it and post a correction.`)
    e.date = input.date
    e.period_id = this.periodFor(input.date)?.id ?? ''
    e.memo = input.memo.trim()
    e.override_reason = input.override_reason?.trim() || null
    this.state.lines = this.state.lines.filter((l) => l.entry_id !== entryId)
    this.state.lines.push(...this.buildLines(entryId, input))
    this.audit(actor, 'entry.update', entryId, e.memo)
    return e
  }

  deleteDraft(actor: Actor, entryId: string) {
    this.require(actor, 'can_post')
    const e = this.entry(entryId) ?? fail('NOT_FOUND', 'Entry not found.')
    if (e.status !== 'draft')
      fail('INVALID_STATE', `${e.number} is posted and cannot be deleted.`)
    this.state.entries = this.state.entries.filter((x) => x.id !== entryId)
    this.state.lines = this.state.lines.filter((l) => l.entry_id !== entryId)
    this.audit(actor, 'entry.delete', entryId, e.number)
  }

  postDraft(actor: Actor, entryId: string, overrideReason?: string) {
    this.require(actor, 'can_post')
    const e = this.entry(entryId) ?? fail('NOT_FOUND', 'Entry not found.')
    if (e.status !== 'draft') fail('INVALID_STATE', `${e.number} is already posted.`)
    const input = this.inputOf(entryId)
    if (overrideReason?.trim()) input.override_reason = overrideReason.trim()
    const issues = this.blocking(this.validateEntry(input, entryId), input.override_reason)
    if (issues.length) throw new LedgerError(issues)
    e.status = 'posted'
    e.override_reason = input.override_reason ?? null
    e.period_id = this.periodFor(e.date)!.id
    e.posted_by = actor.id
    e.posted_at = this.env.now()
    this.audit(actor, 'entry.post', e.id, e.number)
    return e
  }

  /**
   * Posts the mirror image of a posted entry and marks the original
   * reversed. The original's lines are never touched.
   */
  reverse(actor: Actor, entryId: string, date: ISODate, reason: string) {
    this.require(actor, 'can_post')
    const e = this.entry(entryId) ?? fail('NOT_FOUND', 'Entry not found.')
    if (e.status !== 'posted')
      fail('INVALID_STATE', `${e.number} is ${e.status} and cannot be reversed.`)
    if (!reason.trim()) fail('INVALID_INPUT', 'Give a reason for the reversal.')
    const original = this.inputOf(entryId)
    const input: EntryInput = {
      date,
      memo: `Reversal of ${e.number}: ${reason.trim()}`,
      source_type: 'reversal',
      source_id: e.id,
      // A reversal reduces spend, so grant limits cannot block it.
      override_reason: 'Reversal',
      lines: original.lines.map((l) => ({ ...l, debit: l.credit, credit: l.debit })),
    }
    const rev = this.post(actor, input)
    rev.reversal_of = e.id
    e.status = 'reversed'
    e.reversed_by = rev.id
    this.audit(actor, 'entry.reverse', e.id, reason)
    return rev
  }

  // -------------------------------------------------------------------------
  // Periods
  // -------------------------------------------------------------------------

  private period(id: string): FiscalPeriod {
    return this.state.periods.find((p) => p.id === id) ?? fail('NOT_FOUND', 'Period not found.')
  }

  closePeriod(actor: Actor, periodId: string) {
    this.require(actor, 'can_close')
    const p = this.period(periodId)
    if (p.status !== 'open') fail('INVALID_STATE', `${p.name} is already ${p.status}.`)
    const earlierOpen = this.state.periods.find(
      (x) => x.start_date < p.start_date && x.status === 'open' && x.fiscal_year === p.fiscal_year,
    )
    if (earlierOpen)
      fail('INVALID_STATE', `Close ${earlierOpen.name} first; periods close in order.`)
    const drafts = this.state.entries.filter((e) => e.period_id === p.id && e.status === 'draft')
    if (drafts.length)
      fail('INVALID_STATE', `${p.name} has ${drafts.length} draft ${drafts.length === 1 ? 'entry' : 'entries'}. Post or delete ${drafts.length === 1 ? 'it' : 'them'} before closing.`)
    p.status = 'closed'
    this.audit(actor, 'period.close', p.id, p.name)
    return p
  }

  reopenPeriod(actor: Actor, periodId: string, reason: string) {
    this.require(actor, 'can_close')
    const p = this.period(periodId)
    if (p.status === 'locked') fail('PERIOD_LOCKED', `${p.name} is locked and cannot be reopened.`)
    if (p.status === 'open') fail('INVALID_STATE', `${p.name} is already open.`)
    if (!reason.trim()) fail('INVALID_INPUT', 'Give a reason for reopening.')
    const laterClosed = this.state.periods.find(
      (x) => x.start_date > p.start_date && x.status !== 'open',
    )
    if (laterClosed)
      fail('INVALID_STATE', `Reopen ${laterClosed.name} first; periods reopen in reverse order.`)
    p.status = 'open'
    this.audit(actor, 'period.reopen', p.id, reason)
    return p
  }

  lockPeriod(actor: Actor, periodId: string) {
    this.require(actor, 'can_close')
    const p = this.period(periodId)
    if (p.status !== 'closed') fail('INVALID_STATE', `Close ${p.name} before locking it.`)
    p.status = 'locked'
    this.audit(actor, 'period.lock', p.id, p.name)
    return p
  }

  /**
   * Year-end close: zeroes every income and expense account into net assets,
   * fund by fund (unrestricted to unrestricted net assets, restricted funds
   * to restricted net assets). Dated the last day of the year, posted while
   * that period is still open, then every period of the year is closed.
   */
  closeYear(actor: Actor, year: number) {
    this.require(actor, 'can_close')
    const periods = this.state.periods.filter((p) => p.fiscal_year === year)
    if (periods.length === 0) fail('NOT_FOUND', `There are no periods for ${year}.`)
    const last = periods[periods.length - 1]
    if (this.state.entries.some((e) => e.source_type === 'year_end_close' && e.date === last.end_date))
      fail('INVALID_STATE', `${year} has already been closed.`)
    const yearIds = new Set(periods.map((p) => p.id))
    const posted = new Set(
      this.state.entries.filter((e) => e.status !== 'draft' && yearIds.has(e.period_id)).map((e) => e.id),
    )
    const balances = new Map<string, Minor>() // account|fund
    for (const l of this.state.lines) {
      if (!posted.has(l.entry_id)) continue
      const a = this.account(l.account_id)
      if (a?.type !== 'income' && a?.type !== 'expense') continue
      const key = `${l.account_id}|${l.fund_id}`
      balances.set(key, (balances.get(key) ?? 0) + l.base_amount)
    }
    const sys = this.state.settings.system_accounts
    const lines: Array<LineInput> = []
    const netByFund = new Map<string, Minor>()
    for (const [key, bal] of balances) {
      if (bal === 0) continue
      const [account_id, fund_id] = key.split('|')
      lines.push({ account_id, fund_id, debit: bal < 0 ? -bal : 0, credit: bal > 0 ? bal : 0 })
      netByFund.set(fund_id, (netByFund.get(fund_id) ?? 0) + bal)
    }
    for (const [fund_id, net] of netByFund) {
      if (net === 0) continue
      const restricted = this.fund(fund_id)?.restriction !== 'unrestricted'
      lines.push({
        account_id: restricted ? sys.net_assets_restricted : sys.net_assets_unrestricted,
        fund_id,
        debit: net > 0 ? net : 0,
        credit: net < 0 ? -net : 0,
      })
    }
    // Earlier periods may already be closed; the closing entry lands in the
    // last period, which must still be open.
    if (last.status !== 'open') fail('PERIOD_CLOSED', `${last.name} must be open to post the year-end close.`)
    let entry: JournalEntry | null = null
    if (lines.length >= 2) {
      entry = this.post(actor, {
        date: last.end_date,
        memo: `Year-end close ${year}: income and expense to net assets`,
        source_type: 'year_end_close',
        lines,
        override_reason: 'Year-end close',
      })
    }
    for (const p of periods) if (p.status === 'open') p.status = 'closed'
    this.audit(actor, 'year.close', String(year), entry?.number ?? 'nothing to close')
    return entry
  }

  // -------------------------------------------------------------------------
  // Budgets
  // -------------------------------------------------------------------------

  addBudget(actor: Actor, input: Omit<Budget, 'id'> & { id?: string }): Budget {
    this.require(actor, 'can_post')
    const b = { ...input, id: input.id ?? this.env.newId() }
    this.state.budgets.push(b)
    this.audit(actor, 'budget.create', b.id, b.name)
    return b
  }

  setBudgetLine(actor: Actor, input: Omit<BudgetLine, 'id'> & { id?: string }): BudgetLine {
    this.require(actor, 'can_post')
    assertMinor(input.amount, 'Budget amount')
    if (input.amount < 0) fail('NEGATIVE_AMOUNT', 'A budget cannot be negative.')
    if (!this.state.budgets.some((b) => b.id === input.budget_id))
      fail('NOT_FOUND', 'Budget not found.')
    if (input.account_id && this.account(input.account_id)?.type !== 'expense')
      fail('INVALID_INPUT', 'Budget lines are set on expense accounts.')
    const existing = input.id ? this.state.budgetLines.find((l) => l.id === input.id) : undefined
    if (existing) {
      Object.assign(existing, input)
      return existing
    }
    const line = { ...input, id: input.id ?? this.env.newId() }
    this.state.budgetLines.push(line)
    return line
  }

  // -------------------------------------------------------------------------
  // Accounts payable: vendors, bills, payments
  // -------------------------------------------------------------------------

  addVendor(actor: Actor, input: Omit<Vendor, 'id'> & { id?: string }): Vendor {
    this.require(actor, 'can_post')
    if (!input.name.trim()) fail('INVALID_INPUT', 'A vendor needs a name.')
    const v = { ...input, id: input.id ?? this.env.newId() }
    this.state.vendors.push(v)
    return v
  }

  private bill(id: string): Bill {
    return this.state.bills.find((b) => b.id === id) ?? fail('NOT_FOUND', 'Bill not found.')
  }

  createBill(
    actor: Actor,
    input: Omit<Bill, 'id' | 'status' | 'entry_id' | 'approved_by' | 'created_by' | 'lines'> & {
      id?: string
      lines: Array<Omit<CodedLine, 'id'>>
    },
  ): Bill {
    this.require(actor, 'can_post')
    if (!this.state.vendors.some((v) => v.id === input.vendor_id))
      fail('NOT_FOUND', 'Vendor not found.')
    this.checkCodedLines(input.lines)
    const bill: Bill = {
      ...input,
      id: input.id ?? this.env.newId(),
      lines: input.lines.map((l) => ({ ...l, id: this.env.newId() })),
      status: 'draft',
      entry_id: null,
      approved_by: null,
      created_by: actor.id,
    }
    this.state.bills.push(bill)
    this.audit(actor, 'bill.create', bill.id, bill.number)
    return bill
  }

  approveBill(actor: Actor, billId: string) {
    this.require(actor, 'can_approve')
    const b = this.bill(billId)
    if (b.status !== 'draft') fail('INVALID_STATE', `Bill ${b.number} is ${b.status}.`)
    b.status = 'approved'
    b.approved_by = actor.id
    this.audit(actor, 'bill.approve', b.id, b.number)
    return b
  }

  /** Accrual: Dr expense per line / Cr accounts payable per fund. */
  postBill(actor: Actor, billId: string, overrideReason?: string) {
    const b = this.bill(billId)
    if (b.status !== 'approved') fail('INVALID_STATE', `Approve bill ${b.number} before posting it.`)
    const vendor = this.state.vendors.find((v) => v.id === b.vendor_id)
    const entry = this.post(actor, {
      date: b.bill_date,
      memo: `Bill ${b.number} · ${vendor?.name ?? 'Vendor'}`,
      source_type: 'bill',
      source_id: b.id,
      override_reason: overrideReason,
      lines: this.accrualLines(b.lines, b.currency, b.fx_rate, this.state.settings.system_accounts.accounts_payable, vendor?.node_id ?? null),
    })
    b.status = 'posted'
    b.entry_id = entry.id
    return entry
  }

  /** Amount still owed on a posted bill or claim, in its currency. */
  outstanding(kind: 'bill' | 'expense_claim', id: string): Minor {
    const doc = kind === 'bill' ? this.bill(id) : this.claim(id)
    const total = sum(doc.lines.map((l) => l.amount))
    const paid = sum(
      this.state.payments
        .filter((p) => p.applies_to.kind === kind && p.applies_to.id === id)
        .map((p) => p.amount),
    )
    return total - paid
  }

  payBill(
    actor: Actor,
    billId: string,
    input: { date: ISODate; cash_account_id: string; amount: Minor; reference?: string | null; fx_rate?: number },
  ): Payment {
    const b = this.bill(billId)
    if (b.status !== 'posted') fail('INVALID_STATE', `Bill ${b.number} must be posted before it is paid.`)
    const payment = this.pay(actor, { kind: 'bill', id: b.id }, b.lines, b.currency, b.fx_rate, input,
      this.state.settings.system_accounts.accounts_payable, `Payment of bill ${b.number}`)
    if (this.outstanding('bill', b.id) === 0) b.status = 'paid'
    return payment
  }

  // -------------------------------------------------------------------------
  // Expense claims (staff spend)
  // -------------------------------------------------------------------------

  claim(id: string): ExpenseClaim {
    return this.state.claims.find((c) => c.id === id) ?? fail('NOT_FOUND', 'Claim not found.')
  }

  createClaim(
    actor: Actor,
    input: Omit<ExpenseClaim, 'id' | 'number' | 'status' | 'approvals' | 'entry_id' | 'lines' | 'claimant' | 'claimant_name'> & {
      id?: string
      lines: Array<Omit<CodedLine, 'id'>>
      submit?: boolean
    },
  ): ExpenseClaim {
    // Any member can claim; no posting permission needed.
    this.checkCodedLines(input.lines)
    if (!input.purpose.trim()) fail('INVALID_INPUT', 'Say what the money was spent on.')
    if (input.id && this.state.claims.some((c) => c.id === input.id))
      fail('DUPLICATE', 'A claim with this id already exists.')
    const { submit, ...rest } = input
    const claim: ExpenseClaim = {
      ...rest,
      id: input.id ?? this.env.newId(),
      number: this.nextNumber('EC'),
      claimant: actor.id,
      claimant_name: actor.name,
      lines: input.lines.map((l) => ({ ...l, id: this.env.newId() })),
      status: 'draft',
      approvals: [],
      entry_id: null,
    }
    this.state.claims.push(claim)
    if (submit) this.submitClaim(actor, claim.id)
    return claim
  }

  submitClaim(actor: Actor, claimId: string) {
    const c = this.claim(claimId)
    if (c.claimant !== actor.id) fail('PERMISSION', 'Only the claimant can submit a claim.')
    if (c.status !== 'draft') fail('INVALID_STATE', `Claim ${c.number} is ${c.status}.`)
    c.status = 'submitted'
    c.approvals.push({ at: this.env.now(), by: actor.id, action: 'submitted', note: null })
    this.audit(actor, 'claim.submit', c.id, c.number)
    return c
  }

  /**
   * Approval posts the claim (Dr expense / Cr staff claims payable), so
   * budget-vs-actual moves the moment a manager approves. Claimants cannot
   * approve their own claims.
   */
  approveClaim(actor: Actor, claimId: string, note?: string, overrideReason?: string) {
    this.require(actor, 'can_approve')
    const c = this.claim(claimId)
    if (c.status !== 'submitted') fail('INVALID_STATE', `Claim ${c.number} is ${c.status}.`)
    if (c.claimant === actor.id) fail('SELF_APPROVAL', 'You cannot approve your own claim.')
    const entry = this.post(actor, {
      date: c.claim_date,
      memo: `Expense claim ${c.number} · ${c.claimant_name} · ${c.purpose}`,
      source_type: 'expense_claim',
      source_id: c.id,
      override_reason: overrideReason,
      lines: this.accrualLines(c.lines, c.currency, c.fx_rate, this.state.settings.system_accounts.claims_payable, null),
    })
    c.entry_id = entry.id
    c.status = 'posted'
    c.approvals.push({ at: this.env.now(), by: actor.id, action: 'approved', note: note?.trim() || null })
    c.approvals.push({ at: this.env.now(), by: actor.id, action: 'posted', note: entry.number })
    this.audit(actor, 'claim.approve', c.id, c.number)
    return c
  }

  rejectClaim(actor: Actor, claimId: string, note: string) {
    this.require(actor, 'can_approve')
    const c = this.claim(claimId)
    if (c.status !== 'submitted') fail('INVALID_STATE', `Claim ${c.number} is ${c.status}.`)
    if (!note.trim()) fail('INVALID_INPUT', 'Tell the claimant why the claim is rejected.')
    c.status = 'rejected'
    c.approvals.push({ at: this.env.now(), by: actor.id, action: 'rejected', note: note.trim() })
    this.audit(actor, 'claim.reject', c.id, note)
    return c
  }

  reimburseClaim(
    actor: Actor,
    claimId: string,
    input: { date: ISODate; cash_account_id: string; reference?: string | null },
  ) {
    const c = this.claim(claimId)
    if (c.status !== 'posted') fail('INVALID_STATE', `Claim ${c.number} must be approved before it is reimbursed.`)
    const payment = this.pay(actor, { kind: 'expense_claim', id: c.id }, c.lines, c.currency, c.fx_rate,
      { ...input, amount: this.outstanding('expense_claim', c.id) },
      this.state.settings.system_accounts.claims_payable, `Reimbursement of ${c.number} · ${c.claimant_name}`)
    c.status = 'reimbursed'
    c.approvals.push({ at: this.env.now(), by: actor.id, action: 'reimbursed', note: input.reference ?? null })
    return payment
  }

  // -------------------------------------------------------------------------
  // Shared posting helpers
  // -------------------------------------------------------------------------

  private checkCodedLines(lines: Array<Omit<CodedLine, 'id'>>) {
    if (lines.length === 0) fail('TOO_FEW_LINES', 'Add at least one line.')
    lines.forEach((l, i) => {
      assertMinor(l.amount, `Line ${i + 1} amount`)
      if (l.amount <= 0) fail('NEGATIVE_AMOUNT', `Line ${i + 1}: the amount must be more than zero.`)
      const a = this.account(l.account_id)
      if (!a) fail('UNKNOWN_ACCOUNT', `Line ${i + 1}: choose an account.`)
      if (!this.fund(l.fund_id)) fail('UNKNOWN_FUND', `Line ${i + 1}: choose a fund.`)
    })
  }

  /** Dr each coded line; Cr the payable account once per fund. */
  private accrualLines(
    coded: Array<CodedLine>,
    currency: string,
    fx_rate: number,
    payable: string,
    nodeId: string | null,
  ): Array<LineInput> {
    const lines: Array<LineInput> = coded.map((l) => ({
      account_id: l.account_id,
      debit: l.amount,
      currency,
      fx_rate,
      fund_id: l.fund_id,
      grant_id: l.grant_id,
      dimension_ids: l.dimension_ids,
      flow_node_id: nodeId,
      memo: l.description,
    }))
    for (const [fund_id, amount] of this.byFund(coded)) {
      lines.push({ account_id: payable, credit: amount, currency, fx_rate, fund_id, flow_node_id: nodeId })
    }
    return lines
  }

  private byFund(coded: Array<Pick<CodedLine, 'fund_id' | 'amount'>>) {
    const m = new Map<string, Minor>()
    for (const l of coded) m.set(l.fund_id, (m.get(l.fund_id) ?? 0) + l.amount)
    return m
  }

  /**
   * Dr payable / Cr cash, split across funds in the same proportion as the
   * document, so each fund's payable clears with its own cash.
   */
  private pay(
    actor: Actor,
    appliesTo: Payment['applies_to'],
    coded: Array<CodedLine>,
    currency: string,
    docRate: number,
    input: { date: ISODate; cash_account_id: string; amount: Minor; reference?: string | null; fx_rate?: number },
    payable: string,
    memo: string,
  ): Payment {
    const base = this.state.settings.base_currency
    const cash = this.account(input.cash_account_id)
    if (!cash?.is_cash) fail('UNKNOWN_ACCOUNT', 'Choose the bank, mobile money or cash account the money left.')
    assertMinor(input.amount, 'Payment')
    const owed = this.outstanding(appliesTo.kind, appliesTo.id)
    if (input.amount <= 0) fail('NEGATIVE_AMOUNT', 'The payment must be more than zero.')
    if (input.amount > owed) fail('INVALID_INPUT', `Only ${(owed / 100).toFixed(2)} is outstanding.`)
    const payRate = currency === base ? 1 : (input.fx_rate ?? docRate)
    const total = sum(coded.map((l) => l.amount))
    const shares = [...this.byFund(coded)]
    let allocated = 0
    const lines: Array<LineInput> = []
    shares.forEach(([fund_id, amount], i) => {
      const part = i === shares.length - 1 ? input.amount - allocated : Math.round((input.amount * amount) / total)
      allocated += part
      if (part === 0) return
      // The payable clears at the rate it was booked at; cash leaves at
      // today's rate. The difference is a realised FX gain or loss.
      lines.push({ account_id: payable, debit: part, currency, fx_rate: docRate, fund_id })
      lines.push({ account_id: cash!.id, credit: part, currency, fx_rate: payRate, fund_id })
      const diff = convert(part, docRate) - convert(part, payRate)
      if (diff !== 0)
        lines.push({
          account_id: this.state.settings.system_accounts.fx_gain_loss,
          fund_id,
          debit: diff < 0 ? -diff : 0,
          credit: diff > 0 ? diff : 0,
        })
    })
    const entry = this.post(actor, {
      date: input.date,
      memo: input.reference ? `${memo} · ${input.reference}` : memo,
      source_type: appliesTo.kind === 'bill' ? 'payment' : 'reimbursement',
      source_id: appliesTo.id,
      lines,
    })
    const payment: Payment = {
      id: this.env.newId(),
      date: input.date,
      cash_account_id: cash!.id,
      amount: input.amount,
      currency,
      fx_rate: payRate,
      reference: input.reference ?? null,
      applies_to: appliesTo,
      entry_id: entry.id,
      created_by: actor.id,
    }
    this.state.payments.push(payment)
    return payment
  }

  // -------------------------------------------------------------------------
  // Bank accounts and statement lines
  // -------------------------------------------------------------------------

  addBankAccount(actor: Actor, input: Omit<BankAccount, 'id'> & { id?: string }): BankAccount {
    this.require(actor, 'can_post')
    const gl = this.account(input.account_id)
    if (!gl?.is_cash) fail('INVALID_INPUT', 'Link the bank account to a cash account in the chart.')
    const b = { ...input, id: input.id ?? this.env.newId() }
    this.state.bankAccounts.push(b)
    return b
  }

  /**
   * Adds statement lines, skipping any already imported (same account,
   * date, amount and reference, or same description when there is no
   * reference). Returns the lines actually added.
   */
  importStatement(
    actor: Actor,
    bankAccountId: string,
    rows: Array<Pick<StatementLine, 'date' | 'description' | 'reference' | 'amount'>>,
  ): Array<StatementLine> {
    this.require(actor, 'can_post')
    if (!this.state.bankAccounts.some((b) => b.id === bankAccountId))
      fail('NOT_FOUND', 'Bank account not found.')
    const key = (r: Pick<StatementLine, 'date' | 'amount' | 'reference' | 'description'>) =>
      `${r.date}|${r.amount}|${r.reference ?? r.description}`
    const seen = new Set(
      this.state.statementLines.filter((s) => s.bank_account_id === bankAccountId).map(key),
    )
    const batch = this.env.newId()
    const added: Array<StatementLine> = []
    for (const r of rows) {
      assertMinor(r.amount, 'Statement amount')
      if (seen.has(key(r))) continue
      seen.add(key(r))
      const line: StatementLine = {
        ...r,
        id: this.env.newId(),
        bank_account_id: bankAccountId,
        import_batch: batch,
        matched_line_id: null,
        match_kind: null,
      }
      this.state.statementLines.push(line)
      added.push(line)
    }
    this.audit(actor, 'statement.import', bankAccountId, `${added.length} lines`)
    return added
  }

  /** Links a statement line to a posted journal line on the same account. */
  matchStatementLine(actor: Actor, statementLineId: string, journalLineId: string, kind: 'auto' | 'manual' = 'manual') {
    this.require(actor, 'can_post')
    const s = this.state.statementLines.find((x) => x.id === statementLineId) ?? fail('NOT_FOUND', 'Statement line not found.')
    const j = this.state.lines.find((x) => x.id === journalLineId) ?? fail('NOT_FOUND', 'Journal line not found.')
    const bank = this.state.bankAccounts.find((b) => b.id === s.bank_account_id)!
    if (j.account_id !== bank.account_id) fail('INVALID_INPUT', 'That journal line is on a different account.')
    const e = this.entry(j.entry_id)
    if (e?.status === 'draft') fail('INVALID_STATE', 'Post the entry before matching it.')
    if (j.debit - j.credit !== s.amount) fail('INVALID_INPUT', 'The amounts differ.')
    if (this.state.statementLines.some((x) => x.matched_line_id === journalLineId && x.id !== s.id))
      fail('DUPLICATE', 'That journal line is already matched.')
    s.matched_line_id = journalLineId
    s.match_kind = kind
    this.audit(actor, 'statement.match', s.id, `${kind} ${journalLineId}`)
    return s
  }

  unmatchStatementLine(actor: Actor, statementLineId: string) {
    this.require(actor, 'can_post')
    const s = this.state.statementLines.find((x) => x.id === statementLineId) ?? fail('NOT_FOUND', 'Statement line not found.')
    s.matched_line_id = null
    s.match_kind = null
    this.audit(actor, 'statement.unmatch', s.id)
    return s
  }

  /**
   * Turns an unmatched statement line into a posted entry against
   * `counterAccountId` (or suspense), then matches it.
   */
  postFromStatementLine(
    actor: Actor,
    statementLineId: string,
    counter: { account_id?: string; fund_id: string; grant_id?: string | null; memo?: string },
  ) {
    const s = this.state.statementLines.find((x) => x.id === statementLineId) ?? fail('NOT_FOUND', 'Statement line not found.')
    if (s.matched_line_id) fail('INVALID_STATE', 'This line is already matched.')
    const bank = this.state.bankAccounts.find((b) => b.id === s.bank_account_id)!
    const counterId = counter.account_id ?? this.state.settings.system_accounts.suspense
    const amt = Math.abs(s.amount)
    const cashSide: LineInput = { account_id: bank.account_id, fund_id: counter.fund_id, debit: s.amount > 0 ? amt : 0, credit: s.amount < 0 ? amt : 0 }
    const other: LineInput = { account_id: counterId, fund_id: counter.fund_id, grant_id: counter.grant_id ?? null, debit: s.amount < 0 ? amt : 0, credit: s.amount > 0 ? amt : 0 }
    const entry = this.post(actor, {
      date: s.date,
      memo: counter.memo?.trim() || s.description,
      source_type: 'statement',
      source_id: s.id,
      lines: [cashSide, other],
    })
    const cashLine = this.linesOf(entry.id).find((l) => l.account_id === bank.account_id)!
    this.matchStatementLine(actor, s.id, cashLine.id, 'manual')
    return entry
  }
}
