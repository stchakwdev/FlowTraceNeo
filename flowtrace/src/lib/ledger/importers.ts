// Journal imports from the books small NGOs already keep (plan Phase 6).
// QuickBooks Online "Journal" report exports and Xero manual-journal CSVs
// become draft entries; nothing posts until a person reviews it.
import { parseCsvRecords, parseStatementDate } from './csv'
import { parseMoney } from './money'
import type { EntryInput, LedgerState, LineInput } from './types'

export type JournalSource = 'quickbooks' | 'xero'

export interface JournalImport {
  source: JournalSource
  entries: Array<EntryInput>
  /** Account names or codes in the file with no account in the chart. */
  unmappedAccounts: Array<string>
  problems: Array<string>
}

export function detectJournalSource(headers: Array<string>): JournalSource | null {
  const h = new Set(headers)
  if (h.has('accountcode') && h.has('amount') && h.has('narration')) return 'xero'
  if (h.has('account') && (h.has('debit') || h.has('credit'))) return 'quickbooks'
  return null
}

/**
 * Reads a journal CSV into draft entry inputs. Accounts resolve by code
 * first, then by name (case-insensitive); every line goes to `fundId`.
 */
export function parseJournalCsv(text: string, state: LedgerState, fundId: string): JournalImport {
  const { headers, records } = parseCsvRecords(text)
  const source = detectJournalSource(headers)
  const result: JournalImport = { source: source ?? 'quickbooks', entries: [], unmappedAccounts: [], problems: [] }
  if (!source) {
    result.problems.push('This file is not a QuickBooks journal export or a Xero manual journal import.')
    return result
  }
  const byCode = new Map(state.accounts.map((a) => [a.code.toLowerCase(), a]))
  const byName = new Map(state.accounts.map((a) => [a.name.toLowerCase(), a]))
  const resolve = (raw: string) => {
    const k = raw.trim().toLowerCase()
    // QuickBooks writes "1110 Bank: Main operating account" or just the name.
    const code = k.split(/\s+/)[0]
    return byCode.get(k) ?? byCode.get(code) ?? byName.get(k) ?? byName.get(k.replace(/^\d+\s+/, ''))
  }
  const unmapped = new Set<string>()
  const groups = new Map<string, { date: string; memo: string; lines: Array<LineInput> }>()
  let lastKey = ''

  records.forEach((r, i) => {
    const rowNo = i + 2
    let key: string
    let date: string | null
    let memo: string
    let accountRaw: string
    let debit = 0
    let credit = 0
    if (source === 'xero') {
      date = parseStatementDate(r['date'] ?? '')
      memo = r['narration'] || 'Imported journal'
      key = `${r['date']}|${memo}`
      accountRaw = r['accountcode'] ?? ''
      const amt = parseMoney(r['amount'] ?? '')
      if (amt === null) return void result.problems.push(`Row ${rowNo}: no amount.`)
      if (amt > 0) debit = amt
      else credit = -amt
    } else {
      // QuickBooks repeats date and number only on the first row of a journal.
      const num = r['num'] || r['no.'] || r['journal no.'] || ''
      date = parseStatementDate(r['date'] ?? '')
      key = date || num ? `${r['date']}|${num}` : lastKey
      memo = r['memo/description'] || r['memo'] || r['description'] || r['name'] || ''
      accountRaw = r['account'] ?? ''
      const d = r['debit'] ? parseMoney(r['debit']) : 0
      const c = r['credit'] ? parseMoney(r['credit']) : 0
      if (d === null || c === null) return void result.problems.push(`Row ${rowNo}: unreadable amount.`)
      debit = d
      credit = c
    }
    if (!accountRaw && !debit && !credit) return
    lastKey = key
    const g = groups.get(key) ?? { date: date ?? '', memo: memo || 'Imported journal', lines: [] }
    if (!g.date && date) g.date = date
    memo = memo || g.memo
    groups.set(key, g)
    const account = resolve(accountRaw)
    if (!account) {
      unmapped.add(accountRaw)
      return
    }
    g.lines.push({ account_id: account.id, debit, credit, fund_id: fundId, memo })
  })

  for (const g of groups.values()) {
    if (!g.date) {
      result.problems.push(`“${g.memo}” has no date.`)
      continue
    }
    result.entries.push({ date: g.date, memo: g.memo, lines: g.lines, source_type: 'import' })
  }
  result.unmappedAccounts = [...unmapped].filter(Boolean)
  return result
}
