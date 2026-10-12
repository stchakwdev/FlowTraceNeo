// Statement import and matching. Bank CSVs and mobile-money statements
// (M-Pesa, MTN MoMo, Airtel Money) become signed statement lines; auto-match
// pairs each with a posted journal line on the same cash account by amount,
// a date window and the reference.
import { parseCsvRecords, parseStatementDate } from './csv'
import { parseMoney } from './money'
import type { Ledger } from './engine'
import type { Actor, ISODate, JournalEntry, JournalLine, Minor, StatementLine } from './types'

export type StatementFormat = 'mpesa' | 'mtn' | 'airtel' | 'bank'

export interface ParsedStatementRow {
  date: ISODate
  description: string
  reference: string | null
  amount: Minor
}

export interface ParseResult {
  format: StatementFormat
  rows: Array<ParsedStatementRow>
  /** Rows skipped, with the reason, by 1-based CSV line number. */
  skipped: Array<{ line: number; reason: string }>
}

const pick = (r: Record<string, string>, names: Array<string>) => {
  for (const n of names) if (r[n] !== undefined && r[n] !== '') return r[n]
  return ''
}

export function detectFormat(headers: Array<string>): StatementFormat {
  const h = new Set(headers)
  if (h.has('receipt no.') || h.has('receipt no') || (h.has('completion time') && h.has('paid in'))) return 'mpesa'
  if (h.has('external transaction id') || h.has('financial transaction id')) return 'mtn'
  if (h.has('transaction id') && (h.has('transaction type') || h.has('service type'))) return 'airtel'
  return 'bank'
}

/**
 * Reads a statement CSV. Amounts come from one signed column (amount) or a
 * pair (paid in / withdrawn, credit / debit, money in / money out). Failed
 * or reversed mobile-money rows are skipped.
 */
export function parseStatement(text: string): ParseResult {
  const { headers, records } = parseCsvRecords(text)
  const format = detectFormat(headers)
  const rows: Array<ParsedStatementRow> = []
  const skipped: ParseResult['skipped'] = []
  records.forEach((r, i) => {
    const line = i + 2
    const status = pick(r, ['transaction status', 'status']).toLowerCase()
    if (status && !['completed', 'success', 'successful', ''].includes(status)) {
      skipped.push({ line, reason: `status is ${status}` })
      return
    }
    const date = parseStatementDate(
      pick(r, ['completion time', 'date', 'transaction date', 'value date', 'posting date', 'date time', 'initiation time']),
    )
    if (!date) {
      skipped.push({ line, reason: 'no readable date' })
      return
    }
    let amount: Minor | null = null
    const signed = pick(r, ['amount', 'transaction amount'])
    const inRaw = pick(r, ['paid in', 'credit', 'money in', 'deposits', 'credit amount'])
    const outRaw = pick(r, ['withdrawn', 'withdrawen', 'debit', 'money out', 'withdrawals', 'debit amount'])
    if (inRaw || outRaw) {
      const inn = inRaw ? parseMoney(inRaw) : 0
      const out = outRaw ? parseMoney(outRaw) : 0
      if (inn !== null && out !== null) amount = Math.abs(inn) - Math.abs(out)
    } else if (signed) {
      amount = parseMoney(signed)
      const dir = pick(r, ['direction', 'dr/cr', 'type']).toLowerCase()
      if (amount !== null && (dir === 'dr' || dir === 'debit' || dir === 'out')) amount = -Math.abs(amount)
    }
    if (amount === null || amount === 0) {
      skipped.push({ line, reason: 'no amount' })
      return
    }
    rows.push({
      date,
      amount,
      description: pick(r, ['details', 'description', 'narration', 'narrative', 'particulars', 'transaction type', 'memo']) || 'Statement line',
      reference:
        pick(r, ['receipt no.', 'receipt no', 'reference', 'ref', 'transaction id', 'external transaction id', 'financial transaction id', 'cheque no', 'reference number']) || null,
    })
  })
  return { format, rows, skipped }
}

export interface MatchCandidate {
  line: JournalLine
  entry: JournalEntry
  dayGap: number
  referenceMatch: boolean
}

export interface AutoMatchResult {
  matched: Array<{ statementLineId: string; journalLineId: string }>
  ambiguous: Array<{ statementLineId: string; candidates: Array<MatchCandidate> }>
  unmatched: Array<string>
}

const daysApart = (a: ISODate, b: ISODate) =>
  Math.abs(new Date(`${a}T00:00:00Z`).getTime() - new Date(`${b}T00:00:00Z`).getTime()) / 86_400_000

const normRef = (s: string) => s.toUpperCase().replace(/[^A-Z0-9]/g, '')

/** Posted, unmatched journal lines on the statement's account that could be it. */
export function candidatesFor(ledger: Ledger, s: StatementLine, windowDays = 3, taken = new Set<string>()) {
  const state = ledger.state
  const bank = state.bankAccounts.find((b) => b.id === s.bank_account_id)
  if (!bank) return []
  const used = new Set([
    ...state.statementLines.map((x) => x.matched_line_id).filter((x): x is string => !!x),
    ...taken,
  ])
  const ref = s.reference ? normRef(s.reference) : ''
  const out: Array<MatchCandidate> = []
  for (const l of state.lines) {
    if (l.account_id !== bank.account_id || used.has(l.id)) continue
    if (l.debit - l.credit !== s.amount) continue
    const entry = ledger.entry(l.entry_id)
    if (!entry || entry.status === 'draft') continue
    const gap = daysApart(entry.date, s.date)
    if (gap > windowDays) continue
    const text = normRef(`${entry.memo} ${l.memo ?? ''} ${entry.number}`)
    out.push({ line: l, entry, dayGap: gap, referenceMatch: ref.length >= 4 && text.includes(ref) })
  }
  return out.sort((a, b) => Number(b.referenceMatch) - Number(a.referenceMatch) || a.dayGap - b.dayGap)
}

/**
 * Matches every unmatched statement line of a bank account it can. A line
 * matches when it has exactly one candidate, or exactly one candidate with
 * its reference; otherwise it is left for a person to confirm.
 */
export function autoMatch(ledger: Ledger, actor: Actor, bankAccountId: string, windowDays = 3): AutoMatchResult {
  const result: AutoMatchResult = { matched: [], ambiguous: [], unmatched: [] }
  const open = ledger.state.statementLines.filter((s) => s.bank_account_id === bankAccountId && !s.matched_line_id)
  const taken = new Set<string>()
  // Two passes: reference matches first, so they cannot be stolen by a
  // same-amount line that only matches on date.
  const pending: Array<StatementLine> = []
  for (const s of open) {
    const c = candidatesFor(ledger, s, windowDays, taken)
    const byRef = c.filter((x) => x.referenceMatch)
    if (byRef.length === 1) {
      ledger.matchStatementLine(actor, s.id, byRef[0].line.id, 'auto')
      taken.add(byRef[0].line.id)
      result.matched.push({ statementLineId: s.id, journalLineId: byRef[0].line.id })
    } else pending.push(s)
  }
  for (const s of pending) {
    const c = candidatesFor(ledger, s, windowDays, taken)
    if (c.length === 1) {
      ledger.matchStatementLine(actor, s.id, c[0].line.id, 'auto')
      taken.add(c[0].line.id)
      result.matched.push({ statementLineId: s.id, journalLineId: c[0].line.id })
    } else if (c.length > 1) result.ambiguous.push({ statementLineId: s.id, candidates: c })
    else result.unmatched.push(s.id)
  }
  return result
}
