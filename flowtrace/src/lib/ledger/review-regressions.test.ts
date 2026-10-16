// Regressions for the bugs found in review of the first ledger version.
import { describe, expect, it } from 'vitest'
import { LedgerError } from './errors'
import { UNRESTRICTED_FUND_ID, accountId } from './coa-template'
import { applyCommand, sanitizeCommand } from './commands'
import { createOrgLedger } from './setup'
import { activities, grantSummary, trialBalance } from './reports'
import type { LedgerEnv } from './engine'
import type { LedgerCommand } from './commands'
import type { Actor } from './types'

const admin: Actor = { id: 'u1', name: 'Admin', can_post: true, can_approve: true, can_close: true }
const clerk: Actor = { id: 'u2', name: 'Clerk', can_post: true, can_approve: false, can_close: false }
const A = accountId
const U = UNRESTRICTED_FUND_ID

function env(): LedgerEnv {
  let n = 0
  return { now: () => '2026-03-01T00:00:00.000Z', newId: () => `id-${++n}` }
}

function books() {
  const l = createOrgLedger('org', 'KES', 2026, env())
  l.addFund(admin, { id: 'f-r', name: 'Restricted', restriction: 'temporarily_restricted' })
  l.addGrant(admin, { id: 'g', name: 'Grant', donor_node_id: null, donor_name: 'D', fund_id: 'f-r', award: 10_000_000, currency: 'KES', start_date: '2026-01-01', end_date: '2026-12-31', terms: '' })
  l.addVendor(admin, { id: 'v', name: 'Vendor', node_id: null, payment_details: null })
  l.addBankAccount(admin, { id: 'b', name: 'Main', kind: 'bank', provider: 'X', account_id: A('1110'), currency: 'KES', number_masked: '1' })
  return l
}
const run = (l: ReturnType<typeof books>, actor: Actor, cmd: unknown) => applyCommand(l, actor, sanitizeCommand(cmd))
const code = (fn: () => unknown) => {
  try {
    fn()
  } catch (e) {
    if (e instanceof LedgerError) return e.code
    throw e
  }
  return null
}
const spend = (amount: number) => ({
  date: '2026-02-01',
  memo: 'Spend',
  lines: [
    { account_id: A('7100'), debit: amount, fund_id: U },
    { account_id: A('1110'), credit: amount, fund_id: U },
  ],
})

describe('command boundary', () => {
  it('cannot change an account’s type, code or normal balance', () => {
    const l = books()
    l.post(admin, spend(100))
    run(l, admin, { type: 'account.update', accountId: A('7100'), patch: { type: 'asset', normal_balance: 'debit', code: '9', id: 'x', name: 'Stationery' } })
    const a = l.account(A('7100'))!
    expect([a.type, a.code, a.normal_balance, a.name]).toEqual(['expense', '7100', 'debit', 'Stationery'])
    expect(activities(l.state, '2026-01-01', '2026-12-31').totalExpenses.total).toBe(100)
  })

  it('cannot post a manual entry as a system source', () => {
    const l = books()
    run(l, admin, { type: 'entry.post', input: { ...spend(5_000), source_type: 'year_end_close', source_id: 'x' } })
    expect(l.state.entries[0].source_type).toBe('manual')
    expect(activities(l.state, '2026-01-01', '2026-12-31').totalExpenses.total).toBe(5_000)
  })

  it('rejects unknown commands, string rates and caller-chosen ids', () => {
    const l = books()
    expect(code(() => run(l, admin, { type: 'nope' }))).toBe('INVALID_INPUT')
    const fxString = { type: 'entry.post', input: { date: '2026-02-01', memo: 'x', lines: [{ account_id: A('1110'), debit: 1, currency: 'USD', fx_rate: '130', fund_id: U }, { account_id: A('4100'), credit: 1, currency: 'USD', fx_rate: '130', fund_id: U }] } }
    expect(code(() => run(l, admin, fxString))).toBe('FX_RATE_REQUIRED')
    run(l, admin, { type: 'vendor.add', input: { id: 'v', name: 'Twin', node_id: null, payment_details: null } })
    expect(l.state.vendors.filter((v) => v.id === 'v')).toHaveLength(1)
  })
})

describe('segregation of duties', () => {
  it('a bill cannot be approved by the person who entered it', () => {
    const l = books()
    const b = l.createBill(admin, { vendor_id: 'v', number: '1', bill_date: '2026-02-01', due_date: '2026-03-01', currency: 'KES', fx_rate: 1, proof_ids: [], lines: [{ description: 'x', account_id: A('7100'), amount: 100, fund_id: U, grant_id: null, dimension_ids: [] }] })
    expect(code(() => l.approveBill(admin, b.id))).toBe('SELF_APPROVAL')
  })
})

describe('documents', () => {
  const bill = (l: ReturnType<typeof books>, lines: Array<{ amount: number; fund: string; grant?: string }>, currency = 'KES', fx_rate = 1) => {
    const b = l.createBill(clerk, {
      vendor_id: 'v', number: 'B', bill_date: '2026-02-01', due_date: '2026-03-01', currency, fx_rate, proof_ids: [],
      lines: lines.map((x) => ({ description: 'x', account_id: A('5500'), amount: x.amount, fund_id: x.fund, grant_id: x.grant ?? null, dimension_ids: [] })),
    })
    l.approveBill(admin, b.id)
    l.postBill(admin, b.id)
    return b
  }

  it('posts a multi-line foreign-currency bill with rounding', () => {
    const l = books()
    bill(l, [{ amount: 1, fund: U }, { amount: 1, fund: U }], 'USD', 1.5)
    expect(trialBalance(l.state).difference).toBe(0)
  })

  it('partial payments clear every fund’s payable exactly', () => {
    const l = books()
    const b = bill(l, [{ amount: 100, fund: U }, { amount: 200, fund: 'f-r', grant: 'g' }])
    for (let i = 0; i < 300; i++) l.payBill(admin, b.id, { date: '2026-02-02', cash_account_id: A('1110'), amount: 1 })
    expect(l.state.bills[0].status).toBe('paid')
    const ap = l.state.lines.filter((x) => x.account_id === A('2100'))
    for (const f of [U, 'f-r']) expect(ap.filter((x) => x.fund_id === f).reduce((s, x) => s + x.base_amount, 0)).toBe(0)
  })

  it('reversing a payment reopens the bill; a paid bill cannot be reversed', () => {
    const l = books()
    const b = bill(l, [{ amount: 100, fund: U }])
    const p = l.payBill(admin, b.id, { date: '2026-02-02', cash_account_id: A('1110'), amount: 100 })
    expect(code(() => l.reverse(admin, b.entry_id!, '2026-02-03', 'wrong'))).toBe('INVALID_STATE')
    l.reverse(admin, p.entry_id, '2026-02-03', 'bounced')
    expect(l.state.bills[0].status).toBe('posted')
    expect(l.outstanding('bill', b.id)).toBe(100)
    l.reverse(admin, b.entry_id!, '2026-02-03', 'duplicate bill')
    expect(l.state.bills[0].status).toBe('void')
    expect(activities(l.state, '2026-01-01', '2026-12-31', 'cash').totalExpenses.total).toBe(0)
  })

  it('reports commitments in base currency', () => {
    const l = books()
    const b = l.createBill(clerk, { vendor_id: 'v', number: 'U', bill_date: '2026-02-01', due_date: '2026-03-01', currency: 'USD', fx_rate: 130, proof_ids: [], lines: [{ description: 'x', account_id: A('5500'), amount: 1_000, fund_id: 'f-r', grant_id: 'g', dimension_ids: [] }] })
    l.approveBill(admin, b.id)
    expect(grantSummary(l.state, 'g', '2026-02-01').committed).toBe(130_000)
  })
})

describe('year-end close', () => {
  it('closes with a deactivated account and a grant refund, and again after reopening', () => {
    const l = books()
    l.post(admin, spend(100))
    l.updateAccount(admin, A('7100'), { is_active: false })
    // A refund of grant spend: a net credit on a restricted expense line.
    l.post(admin, { date: '2026-03-01', memo: 'Refund', lines: [{ account_id: A('1130'), debit: 50, fund_id: 'f-r' }, { account_id: A('5500'), credit: 50, fund_id: 'f-r', grant_id: 'g' }] })
    const close = l.closeYear(admin, 2026)!
    l.reopenPeriod(admin, 'period-2026-12', 'late invoice')
    l.reverse(admin, close.id, '2026-12-31', 'late invoice')
    l.post(admin, { ...spend(25), date: '2026-12-30', lines: [{ account_id: A('7300'), debit: 25, fund_id: U }, { account_id: A('1110'), credit: 25, fund_id: U }] })
    l.closeYear(admin, 2026)
    const tb = trialBalance(l.state)
    expect(tb.rows.filter((r) => r.account.type === 'expense' || r.account.type === 'income')).toHaveLength(0)
  })
})

describe('statement import', () => {
  it('keeps identical rows within one file, skips them on re-import', () => {
    const l = books()
    const row = { date: '2026-02-01', description: 'ATM WITHDRAWAL', reference: null, amount: -100_000 }
    expect(l.importStatement(admin, 'b', [row, row])).toHaveLength(2)
    expect(l.importStatement(admin, 'b', [row, row])).toHaveLength(0)
    expect(l.importStatement(admin, 'b', [row, row, row])).toHaveLength(1)
  })
})

describe('fiscal years', () => {
  it('opens a new year by command', () => {
    const l = books()
    applyCommand(l, admin, { type: 'year.open', year: 2027 } as LedgerCommand)
    expect(l.periodFor('2027-06-15')?.status).toBe('open')
  })
})
