import { describe, expect, it } from 'vitest'
import { UNRESTRICTED_FUND_ID, accountId } from './coa-template'
import { createOrgLedger } from './setup'
import { autoMatch, parseStatement } from './reconcile'
import { parseJournalCsv } from './importers'
import { parseCsv, parseStatementDate, toCsv } from './csv'
import type { Actor } from './types'

const admin: Actor = { id: 'u1', name: 'Admin', can_post: true, can_approve: true, can_close: true }
const U = UNRESTRICTED_FUND_ID
const A = accountId

describe('csv', () => {
  it('round-trips quotes, commas and newlines', () => {
    const rows = [['a', 'b,c', 'say "hi"', 'line\nbreak']]
    expect(parseCsv(toCsv(rows))).toEqual(rows)
  })
  it('reads East African date styles', () => {
    expect(parseStatementDate('2026-01-15 14:03:22')).toBe('2026-01-15')
    expect(parseStatementDate('15/01/2026')).toBe('2026-01-15')
    expect(parseStatementDate('15 Jan 2026')).toBe('2026-01-15')
    expect(parseStatementDate('31/02/2026')).toBeNull()
  })
})

describe('statement parsing', () => {
  it('reads an M-Pesa statement and skips failed rows', () => {
    const csv = [
      'Receipt No.,Completion Time,Details,Transaction Status,Paid In,Withdrawn,Balance',
      'RB17Q0XT3M,2026-02-01 10:22:01,Customer Transfer Fuliza charge,Completed,,410.00,59590.00',
      'RB34NNA81C,2026-02-03 08:00:00,Funds received from Equity Bank,Completed,"15,000.00",,74590.00',
      'RB99FAILED,2026-02-03 09:00:00,Pay Bill to KPLC,Failed,,500.00,74590.00',
    ].join('\n')
    const r = parseStatement(csv)
    expect(r.format).toBe('mpesa')
    expect(r.rows).toEqual([
      { date: '2026-02-01', description: 'Customer Transfer Fuliza charge', reference: 'RB17Q0XT3M', amount: -41_000 },
      { date: '2026-02-03', description: 'Funds received from Equity Bank', reference: 'RB34NNA81C', amount: 1_500_000 },
    ])
    expect(r.skipped).toEqual([{ line: 4, reason: 'status is failed' }])
  })

  it('reads a bank CSV with debit and credit columns', () => {
    const r = parseStatement('Date,Narration,Reference,Debit,Credit\n15/01/2026,SUPPLIES,INV-9,"1,200.50",\n16/01/2026,GRANT,HHF-1,,5000')
    expect(r.format).toBe('bank')
    expect(r.rows.map((x) => x.amount)).toEqual([-120_050, 500_000])
  })
})

describe('auto-match', () => {
  it('matches by amount and date window, prefers reference, leaves ties for a person', () => {
    const l = createOrgLedger('org', 'KES', 2026)
    l.addBankAccount(admin, { id: 'b', name: 'Main', kind: 'bank', provider: 'X', account_id: A('1110'), currency: 'KES', number_masked: '1' })
    const pay = (amount: number, date: string, memo: string) =>
      l.post(admin, { date, memo, lines: [{ account_id: A('7100'), debit: amount, fund_id: U }, { account_id: A('1110'), credit: amount, fund_id: U }] })
    pay(10_000, '2026-01-10', 'Stationery · INV-100')
    pay(10_000, '2026-01-11', 'Stationery · INV-101')
    pay(20_000, '2026-01-12', 'Fuel')
    pay(30_000, '2026-01-12', 'Rent A')
    pay(30_000, '2026-01-12', 'Rent B')
    l.importStatement(admin, 'b', [
      { date: '2026-01-11', description: 'INV-101', reference: 'INV-101', amount: -10_000 },
      { date: '2026-01-12', description: 'X', reference: null, amount: -10_000 },
      { date: '2026-01-14', description: 'FUEL', reference: null, amount: -20_000 },
      { date: '2026-01-13', description: 'RENT', reference: null, amount: -30_000 },
      { date: '2026-01-30', description: 'LATE', reference: null, amount: -20_000 },
    ])
    const r = autoMatch(l, admin, 'b')
    expect(r.matched).toHaveLength(3)
    expect(r.ambiguous).toHaveLength(1)
    expect(r.unmatched).toHaveLength(1)
    const inv101 = l.state.statementLines.find((s) => s.reference === 'INV-101')!
    const jl = l.state.lines.find((x) => x.id === inv101.matched_line_id)!
    expect(l.entry(jl.entry_id)!.memo).toContain('INV-101')
    // Re-importing the same rows adds nothing.
    expect(l.importStatement(admin, 'b', [{ date: '2026-01-11', description: 'INV-101', reference: 'INV-101', amount: -10_000 }])).toHaveLength(0)
  })
})

describe('journal imports', () => {
  const l = createOrgLedger('org', 'KES', 2026)
  it('reads a QuickBooks journal export into draft inputs', () => {
    const csv = [
      'Date,Transaction Type,Num,Name,Memo/Description,Account,Debit,Credit',
      '01/15/2026,Journal Entry,12,,Office rent January,7500 Rent,"45,000.00",',
      ',,,,,Bank: Main operating account,,"45,000.00"',
      '20/01/2026,Journal Entry,13,,Unknown,9999 Mystery,100,',
    ].join('\n')
    const r = parseJournalCsv(csv, l.state, U)
    expect(r.source).toBe('quickbooks')
    expect(r.entries[0].lines).toEqual([
      { account_id: A('7500'), debit: 4_500_000, credit: 0, fund_id: U, memo: 'Office rent January' },
      { account_id: A('1110'), debit: 0, credit: 4_500_000, fund_id: U, memo: 'Office rent January' },
    ])
    expect(r.unmappedAccounts).toEqual(['9999 Mystery'])
  })
  it('reads a Xero manual journal import', () => {
    const csv = '*Narration,*Date,Description,*AccountCode,*TaxRate,*Amount\nAudit fee,31/01/2026,Audit,7600,Tax Exempt,25000\nAudit fee,31/01/2026,Audit,2200,Tax Exempt,-25000'
    const r = parseJournalCsv(csv, l.state, U)
    expect(r.source).toBe('xero')
    expect(r.entries).toHaveLength(1)
    expect(l.validateEntry(r.entries[0])).toEqual([])
  })
})
