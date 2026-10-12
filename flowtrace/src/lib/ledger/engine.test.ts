import { describe, expect, it } from 'vitest'
import fc from 'fast-check'
import { LedgerError } from './errors'
import { Ledger } from './engine'
import { UNRESTRICTED_FUND_ID, accountId } from './coa-template'
import { createOrgLedger } from './setup'
import { convert, parseMoney } from './money'
import { financialPosition, trialBalance } from './reports'
import type { LedgerEnv } from './engine'
import type { Actor, EntryInput } from './types'

const admin: Actor = { id: 'u1', name: 'Admin', can_post: true, can_approve: true, can_close: true }
const clerk: Actor = { id: 'u2', name: 'Clerk', can_post: true, can_approve: false, can_close: false }
const viewer: Actor = { id: 'u3', name: 'Viewer', can_post: false, can_approve: false, can_close: false }

function testEnv(): LedgerEnv {
  let n = 0
  return { now: () => '2026-03-01T00:00:00.000Z', newId: () => `id-${++n}` }
}

const A = accountId
const U = UNRESTRICTED_FUND_ID

function fresh() {
  const ledger = createOrgLedger('org', 'KES', 2026, testEnv())
  const restricted = ledger.addFund(admin, { id: 'fund-wash', name: 'WASH', restriction: 'temporarily_restricted' })
  ledger.addGrant(admin, {
    id: 'grant-wash',
    name: 'Water grant',
    donor_node_id: null,
    donor_name: 'Water Trust',
    fund_id: restricted.id,
    award: 1_000_000,
    currency: 'KES',
    start_date: '2026-01-01',
    end_date: '2026-06-30',
    terms: '',
  })
  return ledger
}

const simple = (amount: number, date = '2026-01-15'): EntryInput => ({
  date,
  memo: 'Office supplies',
  lines: [
    { account_id: A('7100'), debit: amount, fund_id: U },
    { account_id: A('1110'), credit: amount, fund_id: U },
  ],
})

const codes = (fn: () => unknown) => {
  try {
    fn()
  } catch (e) {
    if (e instanceof LedgerError) return e.issues.map((i) => i.code)
    throw e
  }
  return []
}

describe('money', () => {
  it('parses money to minor units without floats', () => {
    expect(parseMoney('1,250.5')).toBe(125050)
    expect(parseMoney('0.1')).toBe(10)
    expect(parseMoney('(300)')).toBe(-30000)
    expect(parseMoney('12.345')).toBeNull()
    expect(parseMoney('abc')).toBeNull()
  })

  it('converts with banker’s rounding', () => {
    expect(convert(1, 0.5)).toBe(0)
    expect(convert(3, 0.5)).toBe(2)
    expect(convert(100, 129.5)).toBe(12950)
  })
})

describe('posting rules', () => {
  it('posts a balanced entry and the trial balance nets to zero', () => {
    const l = fresh()
    const e = l.post(admin, simple(50_000))
    expect(e.status).toBe('posted')
    expect(e.number).toBe('JE-0001')
    expect(trialBalance(l.state).difference).toBe(0)
  })

  it('rejects unbalanced entries', () => {
    const l = fresh()
    const input = simple(50_000)
    input.lines[1].credit = 49_999
    expect(codes(() => l.post(admin, input))).toContain('UNBALANCED')
    expect(l.state.entries).toHaveLength(0)
  })

  it('rejects lines with both sides, zero or negative amounts, or fractional cents', () => {
    const l = fresh()
    expect(codes(() => l.post(admin, { ...simple(1), lines: [{ account_id: A('7100'), debit: 5, credit: 5, fund_id: U }, { account_id: A('1110'), credit: 0, fund_id: U }] }))).toEqual(
      expect.arrayContaining(['LINE_BOTH_SIDES', 'LINE_ZERO']),
    )
    expect(codes(() => l.post(admin, simple(-5)))).toContain('NEGATIVE_AMOUNT')
    expect(codes(() => l.post(admin, simple(10.5)))).toContain('NOT_MINOR_UNITS')
  })

  it('rejects posting to header accounts and single-line entries', () => {
    const l = fresh()
    const input = simple(100)
    input.lines[0].account_id = A('7000')
    expect(codes(() => l.post(admin, input))).toContain('GROUP_ACCOUNT')
    expect(codes(() => l.post(admin, { ...simple(100), lines: [simple(100).lines[0]] }))).toContain('TOO_FEW_LINES')
  })

  it('requires every fund to balance within an entry', () => {
    const l = fresh()
    const input: EntryInput = {
      date: '2026-01-15',
      memo: 'Cross-fund',
      lines: [
        { account_id: A('5100'), debit: 100, fund_id: 'fund-wash', grant_id: 'grant-wash' },
        { account_id: A('1110'), credit: 100, fund_id: U },
      ],
    }
    expect(codes(() => l.post(admin, input))).toContain('FUND_UNBALANCED')
  })

  it('needs posting permission', () => {
    const l = fresh()
    expect(codes(() => l.post(viewer, simple(100)))).toEqual(['PERMISSION'])
  })

  it('makes posted entries immutable; corrections are reversals', () => {
    const l = fresh()
    const e = l.post(admin, simple(10_000))
    expect(codes(() => l.updateDraft(admin, e.id, simple(1)))).toEqual(['INVALID_STATE'])
    expect(codes(() => l.deleteDraft(admin, e.id))).toEqual(['INVALID_STATE'])
    const before = JSON.stringify(l.linesOf(e.id))
    const rev = l.reverse(admin, e.id, '2026-01-20', 'Wrong account')
    expect(JSON.stringify(l.linesOf(e.id))).toBe(before)
    expect(l.entry(e.id)!.status).toBe('reversed')
    expect(rev.reversal_of).toBe(e.id)
    expect(trialBalance(l.state).rows).toHaveLength(0)
    expect(codes(() => l.reverse(admin, e.id, '2026-01-21', 'again'))).toEqual(['INVALID_STATE'])
  })

  it('blocks posting into closed and locked periods, and closes in order', () => {
    const l = fresh()
    expect(codes(() => l.closePeriod(admin, 'period-2026-02'))).toEqual(['INVALID_STATE'])
    expect(codes(() => l.closePeriod(clerk, 'period-2026-01'))).toEqual(['PERMISSION'])
    l.closePeriod(admin, 'period-2026-01')
    expect(codes(() => l.post(admin, simple(100)))).toContain('PERIOD_CLOSED')
    l.lockPeriod(admin, 'period-2026-01')
    expect(codes(() => l.post(admin, simple(100)))).toContain('PERIOD_LOCKED')
    expect(codes(() => l.reopenPeriod(admin, 'period-2026-01', 'fix'))).toEqual(['PERIOD_LOCKED'])
    expect(codes(() => l.post(admin, simple(100, '2027-01-02')))).toContain('NO_PERIOD')
  })

  it('will not close a period with drafts in it', () => {
    const l = fresh()
    l.saveDraft(admin, simple(100))
    expect(codes(() => l.closePeriod(admin, 'period-2026-01'))).toEqual(['INVALID_STATE'])
  })
})

describe('restricted funds and grants', () => {
  const grantSpend = (amount: number, date = '2026-02-01', grant: string | null = 'grant-wash'): EntryInput => ({
    date,
    memo: 'Borehole',
    lines: [
      { account_id: A('5100'), debit: amount, fund_id: 'fund-wash', grant_id: grant },
      { account_id: A('1130'), credit: amount, fund_id: 'fund-wash' },
    ],
  })

  it('requires restricted spend to name its grant', () => {
    const l = fresh()
    expect(codes(() => l.post(admin, grantSpend(100, '2026-02-01', null)))).toContain('RESTRICTED_NEEDS_GRANT')
  })

  it('blocks spend outside the grant’s dates unless overridden with a reason', () => {
    const l = fresh()
    expect(codes(() => l.post(admin, grantSpend(100, '2026-07-02')))).toContain('GRANT_ENDED')
    const e = l.post(admin, { ...grantSpend(100, '2026-07-02'), override_reason: 'Donor approved no-cost extension' })
    expect(e.override_reason).toBe('Donor approved no-cost extension')
  })

  it('blocks spend over the award unless overridden', () => {
    const l = fresh()
    l.post(admin, grantSpend(900_000))
    expect(codes(() => l.post(admin, grantSpend(100_001)))).toContain('GRANT_OVER_BUDGET')
    l.post(admin, grantSpend(100_000))
    expect(l.grantSpend('grant-wash')).toBe(1_000_000)
  })

  it('rejects a grant on the wrong fund', () => {
    const l = fresh()
    const input = grantSpend(100)
    input.lines.forEach((x) => (x.fund_id = U))
    expect(codes(() => l.post(admin, input))).toContain('GRANT_FUND_MISMATCH')
  })
})

describe('multi-currency', () => {
  it('freezes base amounts at posting and balances in base currency', () => {
    const l = fresh()
    const e = l.post(admin, {
      date: '2026-01-10',
      memo: 'USD grant received',
      lines: [
        { account_id: A('1110'), debit: 100_000, currency: 'USD', fx_rate: 129.5, fund_id: U },
        { account_id: A('4100'), credit: 100_000, currency: 'USD', fx_rate: 129.5, fund_id: U },
      ],
    })
    expect(l.linesOf(e.id).map((x) => x.base_amount)).toEqual([12_950_000, -12_950_000])
    expect(codes(() => l.post(admin, { ...simple(1), lines: [{ account_id: A('1110'), debit: 1, currency: 'USD', fund_id: U }, { account_id: A('4100'), credit: 1, currency: 'USD', fund_id: U }] }))).toContain(
      'FX_RATE_REQUIRED',
    )
  })

  it('books a realised FX difference when a foreign bill is paid at a new rate', () => {
    const l = fresh()
    const v = l.addVendor(admin, { name: 'Pump supplier', node_id: null, payment_details: null })
    const b = l.createBill(clerk, {
      vendor_id: v.id,
      number: 'INV-1',
      bill_date: '2026-01-05',
      due_date: '2026-02-05',
      currency: 'USD',
      fx_rate: 130,
      proof_ids: [],
      lines: [{ description: 'Pump', account_id: A('5500'), amount: 10_000, fund_id: U, grant_id: null, dimension_ids: [] }],
    })
    l.approveBill(admin, b.id)
    l.postBill(admin, b.id)
    l.payBill(admin, b.id, { date: '2026-02-01', cash_account_id: A('1110'), amount: 10_000, fx_rate: 128 })
    const tb = trialBalance(l.state)
    expect(tb.difference).toBe(0)
    // AP cleared in base currency; 2 KES/USD gain on 100 USD = KES 200.
    expect(tb.rows.find((r) => r.account.code === '2100')).toBeUndefined()
    expect(tb.rows.find((r) => r.account.code === '4400')?.credit).toBe(20_000)
  })
})

describe('approval chain', () => {
  it('claims post on approval, cannot be self-approved, and reimburse to zero', () => {
    const l = fresh()
    const officer: Actor = { id: 'u9', name: 'Field Officer', can_post: false, can_approve: false, can_close: false }
    const c = l.createClaim(officer, {
      claim_date: '2026-02-03',
      currency: 'KES',
      fx_rate: 1,
      purpose: 'Site visit',
      receipt_names: [],
      captured_offline: false,
      submit: true,
      lines: [{ description: 'Matatu fare', account_id: A('5700'), amount: 250_000, fund_id: 'fund-wash', grant_id: 'grant-wash', dimension_ids: [] }],
    })
    expect(c.status).toBe('submitted')
    expect(codes(() => l.approveClaim(officer, c.id))).toEqual(['PERMISSION'])
    const { id: _id, number: _n, status: _s, approvals: _a, entry_id: _e, claimant: _c, claimant_name: _cn, ...rest } = c
    expect(codes(() => l.createClaim(admin, { ...rest, id: c.id, lines: [] }))).toEqual(['TOO_FEW_LINES'])
    expect(codes(() => l.createClaim(admin, { ...rest, id: c.id, lines: c.lines }))).toEqual(['DUPLICATE'])
    const selfClaim = l.createClaim(admin, { ...rest, submit: true, lines: c.lines.map(({ id: _lid, ...x }) => x) })
    expect(codes(() => l.approveClaim(admin, selfClaim.id))).toEqual(['SELF_APPROVAL'])
    l.approveClaim(admin, c.id, 'OK')
    expect(l.claim(c.id).status).toBe('posted')
    expect(l.grantSpend('grant-wash')).toBe(250_000)
    l.reimburseClaim(admin, c.id, { date: '2026-02-04', cash_account_id: A('1150'), reference: 'QAB12XYZ' })
    expect(l.claim(c.id).status).toBe('reimbursed')
    expect(l.outstanding('expense_claim', c.id)).toBe(0)
    expect(trialBalance(l.state).rows.find((r) => r.account.code === '2110')).toBeUndefined()
  })
})

describe('year-end close', () => {
  it('moves income and expense to net assets by fund and closes the year', () => {
    const l = fresh()
    l.post(admin, {
      date: '2026-01-05',
      memo: 'Grant',
      lines: [
        { account_id: A('1130'), debit: 500_000, fund_id: 'fund-wash', grant_id: 'grant-wash' },
        { account_id: A('4100'), credit: 500_000, fund_id: 'fund-wash', grant_id: 'grant-wash' },
      ],
    })
    l.post(admin, simple(20_000))
    const before = financialPosition(l.state, '2026-12-31')
    const close = l.closeYear(admin, 2026)!
    expect(close.source_type).toBe('year_end_close')
    const tb = trialBalance(l.state)
    expect(tb.rows.filter((r) => r.account.type === 'income' || r.account.type === 'expense')).toHaveLength(0)
    expect(tb.rows.find((r) => r.account.code === '3200')?.credit).toBe(500_000)
    expect(tb.rows.find((r) => r.account.code === '3100')?.debit).toBe(20_000)
    const after = financialPosition(l.state, '2026-12-31')
    expect(after.totalNetAssets).toBe(before.totalNetAssets)
    expect(after.netAssets.restricted).toBe(500_000)
    expect(l.state.periods.every((p) => p.status === 'closed')).toBe(true)
    expect(codes(() => l.closeYear(admin, 2026))).toEqual(['INVALID_STATE'])
  })
})

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

const POSTABLE = ['1110', '1120', '1130', '1150', '2100', '4100', '4200', '5100', '5500', '6100', '7100', '7300'].map(A)

const lineArb = fc.record({
  account: fc.constantFrom(...POSTABLE),
  amount: fc.integer({ min: 1, max: 50_000_000 }),
  side: fc.constantFrom('debit', 'credit'),
})

/** A random balanced entry in the unrestricted fund. */
const balancedArb = fc
  .tuple(fc.array(lineArb, { minLength: 1, maxLength: 6 }), fc.constantFrom(...POSTABLE), fc.integer({ min: 1, max: 28 }), fc.integer({ min: 1, max: 12 }))
  .map(([lines, plug, day, month]): EntryInput => {
    const net = lines.reduce((s, l) => s + (l.side === 'debit' ? l.amount : -l.amount), 0)
    const out = lines.map((l) => ({ account_id: l.account, fund_id: U, [l.side]: l.amount }))
    if (net !== 0) out.push({ account_id: plug, fund_id: U, ...(net > 0 ? { credit: net } : { debit: -net }) })
    else out.push({ account_id: plug, fund_id: U, debit: 1 }, { account_id: plug === POSTABLE[0] ? POSTABLE[1] : POSTABLE[0], fund_id: U, credit: 1 })
    return { date: `2026-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`, memo: 'Generated', lines: out }
  })

describe('properties', () => {
  it('any sequence of balanced postings and reversals keeps the books balanced', () => {
    fc.assert(
      fc.property(fc.array(fc.tuple(balancedArb, fc.boolean()), { minLength: 1, maxLength: 25 }), (ops) => {
        const l = fresh()
        for (const [input, reverse] of ops) {
          const e = l.post(admin, input)
          if (reverse) l.reverse(admin, e.id, input.date, 'test')
        }
        expect(trialBalance(l.state).difference).toBe(0)
        const fp = financialPosition(l.state, '2026-12-31')
        expect(fp.difference).toBe(0)
      }),
      { numRuns: 150 },
    )
  })

  it('any entry whose debits and credits differ is rejected and leaves no trace', () => {
    fc.assert(
      fc.property(balancedArb, fc.integer({ min: 1, max: 1_000_000 }), (input, skew) => {
        const l = fresh()
        const bad = structuredClone(input)
        const target = bad.lines[0]
        if (target.debit) target.debit += skew
        else target.credit = (target.credit ?? 0) + skew
        expect(codes(() => l.post(admin, bad))).toContain('UNBALANCED')
        expect(l.state.entries).toHaveLength(0)
        expect(l.state.lines).toHaveLength(0)
      }),
      { numRuns: 150 },
    )
  })

  it('reversing everything returns every account to zero', () => {
    fc.assert(
      fc.property(fc.array(balancedArb, { minLength: 1, maxLength: 15 }), (inputs) => {
        const l = fresh()
        const posted = inputs.map((i) => l.post(admin, i))
        for (const e of posted) l.reverse(admin, e.id, '2026-12-31', 'undo')
        expect(trialBalance(l.state).rows).toHaveLength(0)
      }),
      { numRuns: 100 },
    )
  })

  it('posted lines never change once posted', () => {
    fc.assert(
      fc.property(fc.array(balancedArb, { minLength: 2, maxLength: 10 }), (inputs) => {
        const l = fresh()
        const first = l.post(admin, inputs[0])
        const snapshot = JSON.stringify(l.linesOf(first.id))
        for (const i of inputs.slice(1)) l.post(admin, i)
        l.reverse(admin, first.id, '2026-12-31', 'x')
        expect(JSON.stringify(l.linesOf(first.id))).toBe(snapshot)
      }),
      { numRuns: 100 },
    )
  })
})

describe('Ledger class', () => {
  it('is constructible over plain state, as the SpacetimeDB module does', () => {
    const l = fresh()
    const copy = new Ledger(structuredClone(l.state), testEnv())
    copy.post(admin, simple(100))
    expect(copy.state.entries).toHaveLength(1)
    expect(l.state.entries).toHaveLength(0)
  })
})
