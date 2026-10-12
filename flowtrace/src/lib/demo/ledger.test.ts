import { beforeEach, describe, expect, it } from 'vitest'
import {
  activities,
  budgetVsActual,
  cashFlow,
  financialPosition,
  functionalExpenses,
  grantSummary,
  ledgerSignals,
  trialBalance,
} from '../ledger'
import { DEMO_ORG_ID, transactions } from './dataset'
import {
  DEMO_ACTOR,
  DEMO_AS_OF,
  buildDemoLedger,
  demoLedgerBackend,
  resetDemoLedger,
} from './ledger'

describe('demo ledger', () => {
  const ledger = buildDemoLedger()
  const s = ledger.state

  it('backfills every non-rejected transaction as an entry or an allocation', () => {
    const bridged = new Set(s.entries.filter((e) => e.source_type === 'transaction').map((e) => e.source_id))
    const allocations = transactions.filter((t) => t.category === 'Budget' && !bridged.has(t.id))
    expect(bridged.size + allocations.length).toBe(transactions.filter((t) => t.verification_status !== 'rejected').length)
  })

  it('foots: trial balance, financial position and cash flow', () => {
    expect(trialBalance(s, DEMO_AS_OF).difference).toBe(0)
    const fp = financialPosition(s, DEMO_AS_OF)
    expect(fp.difference).toBe(0)
    expect(fp.totalAssets).toBeGreaterThan(0)
    const cf = cashFlow(s, '2026-01-01', DEMO_AS_OF)
    expect(cf.difference).toBe(0)
  })

  it('cross-foots: the change in net assets matches the balance sheet', () => {
    const open = financialPosition(s, '2025-12-31')
    const close = financialPosition(s, DEMO_AS_OF)
    const act = activities(s, '2026-01-01', DEMO_AS_OF)
    expect(close.totalNetAssets - open.totalNetAssets).toBe(act.changeInNetAssets.total)
    expect(close.netAssets.restricted - open.netAssets.restricted).toBe(act.changeInNetAssets.restricted)
    const fx = functionalExpenses(s, '2026-01-01', DEMO_AS_OF)
    expect(fx.totals.total).toBe(act.totalExpenses.total)
  })

  it('cash basis drops accruals but still foots', () => {
    const accrual = activities(s, '2026-01-01', DEMO_AS_OF)
    const cash = activities(s, '2026-01-01', DEMO_AS_OF, 'cash')
    // The unpaid electricity bill is an expense on accrual only.
    expect(accrual.totalExpenses.total - cash.totalExpenses.total).toBe(3_980_000)
    expect(trialBalance(s, DEMO_AS_OF, 'cash').difference).toBe(0)
  })

  it('reconciles January on the main account and reports ledger signals', () => {
    const main = s.statementLines.filter((l) => l.bank_account_id === 'bank-main')
    const unmatched = main.filter((l) => !l.matched_line_id)
    expect(unmatched.map((l) => l.description)).toEqual(['INTEREST CREDIT'])
    const sig = ledgerSignals(s, DEMO_AS_OF)
    expect(sig.reconciledShare).toBeGreaterThan(0.3)
    expect(sig.grantCodedShare).toBeGreaterThan(0)
    expect(sig.inBudgetShare).toBeGreaterThan(0)
  })

  it('tracks grants and budgets', () => {
    const wash = grantSummary(s, 'grant-kisima-wash', DEMO_AS_OF)
    expect(wash.spent).toBe(250_000_000)
    expect(wash.committed).toBe(1_850_000) // the submitted field claim
    const bva = budgetVsActual(s, { grantId: 'grant-kisima-wash' })
    expect(bva.find((r) => r.account?.code === '5100')?.actual).toBe(250_000_000)
    const field = budgetVsActual(s).find((r) => r.label.includes('Field Operations'))
    expect(field?.status).toBe('over')
  })

  it('leaves work to do in the demo: a claim to approve, a bill to post, lines to match', () => {
    expect(s.claims.filter((c) => c.status === 'submitted')).toHaveLength(1)
    expect(s.bills.filter((b) => b.status === 'approved')).toHaveLength(1)
    expect(s.statementLines.filter((l) => l.bank_account_id === 'mpesa-till' && !l.matched_line_id)).toHaveLength(3)
  })
})

describe('demo backend', () => {
  beforeEach(() => resetDemoLedger())

  it('applies commands and rolls back on a rule failure', async () => {
    const before = await demoLedgerBackend.load(DEMO_ORG_ID)
    await expect(
      demoLedgerBackend.run(DEMO_ORG_ID, { type: 'period.close', periodId: 'period-2026-02' }, DEMO_ACTOR),
    ).rejects.toThrow(/Close Jan 2026 first/)
    const same = await demoLedgerBackend.load(DEMO_ORG_ID)
    expect(same.periods).toEqual(before.periods)

    const claim = same.claims.find((c) => c.status === 'submitted')!
    await demoLedgerBackend.run(DEMO_ORG_ID, { type: 'claim.approve', claimId: claim.id }, DEMO_ACTOR)
    const after = await demoLedgerBackend.load(DEMO_ORG_ID)
    expect(after.claims.find((c) => c.id === claim.id)?.status).toBe('posted')
    expect(after.entries.length).toBe(before.entries.length + 1)
  })
})

describe('live bridge', () => {
  beforeEach(() => resetDemoLedger())

  it('posts a newly recorded transaction, charged to its project’s grant', async () => {
    const { postDemoTransaction } = await import('./ledger')
    const before = await demoLedgerBackend.load(DEMO_ORG_ID)
    const reason = await postDemoTransaction({
      id: 'new-txn',
      from_node_id: 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a14',
      to_node_id: 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a35',
      amount: 120_000,
      currency: 'KES',
      exchange_rate: null,
      description: 'Pump spares',
      category: 'Programs',
      reference: 'PS-1',
      verification_status: 'pending',
      transaction_date: '2026-02-04',
    })
    expect(reason).toBeNull()
    const after = await demoLedgerBackend.load(DEMO_ORG_ID)
    const entry = after.entries.find((e) => e.source_id === 'new-txn')!
    expect(entry.status).toBe('posted')
    const spend = after.lines.find((l) => l.entry_id === entry.id && l.debit > 0)!
    expect(spend.grant_id).toBe('grant-kisima-wash')
    expect(after.entries.length).toBe(before.entries.length + 1)
  })
})
