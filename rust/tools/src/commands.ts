// Every command type, accepted at least once: a scenario of raw commands
// over the demo books, each sent through sanitizeCommand and applyCommand
// (commands.ts), the way the demo backend and the SpacetimeDB module apply
// them. Records the raw and sanitised command, the CommandResult and the
// state delta of each step, and the final state.
import { frozenEnv, writeFixture } from './fixture'
import { Ledger, UNRESTRICTED_FUND_ID, accountId } from './flowtrace'
import type { Actor, LedgerState } from './flowtrace'
import { DEMO_ACTOR, DEMO_ENV } from './demo'
import { src } from './sources'
import { Scenario, recordLedgerCalls } from './trace'

const A = accountId
const U = UNRESTRICTED_FUND_ID

/** Fixture actors (fictional). The approver is the demo's own user. */
export const CLERK: Actor = { id: 'fixture-clerk', name: 'Fixture Clerk', can_post: true, can_approve: false, can_close: false }
export const OFFICER: Actor = { id: 'fixture-officer', name: 'Fixture Officer', can_post: false, can_approve: false, can_close: false }

export function commandsEnv() {
  return frozenEnv(
    DEMO_ENV.now,
    (n) => `fx-${String(n).padStart(4, '0')}`,
    'fx- then n left-padded with 0 to 4 digits, n = 1, 2, 3, ... (fx-0001)',
    'rust/tools/src/commands.ts#commandsEnv, injected through LedgerEnv (flowtrace/src/lib/ledger/engine.ts:41-44)',
  )
}

const idOf = (r: { type: string; id?: string }) => {
  if (r.type !== 'id' || !r.id) throw new Error(`expected an id result, got ${JSON.stringify(r)}`)
  return r.id
}

export function exportCommands(outDir: string, demoState: LedgerState): LedgerState {
  const { env, doc } = commandsEnv()
  const ledger = new Ledger(structuredClone(demoState), env)
  const s = new Scenario(ledger)
  const admin = DEMO_ACTOR as Actor
  const rec = recordLedgerCalls(Ledger)
  try {
    // Journal: draft, edit, post; draft and delete; post and reverse; import.
    const d1 = idOf(s.cmd(admin, { type: 'entry.saveDraft', input: { date: '2026-02-04', memo: 'Fixture draft: printer paper', lines: [{ account_id: A('7100'), debit: 250_000, fund_id: U }, { account_id: A('1110'), credit: 250_000, fund_id: U }] } }))
    s.cmd(admin, { type: 'entry.updateDraft', entryId: d1, input: { date: '2026-02-04', memo: 'Fixture draft: printer paper and toner', lines: [{ account_id: A('7100'), debit: 275_000, fund_id: U, dimension_ids: ['dim-loc-nairobi'], memo: 'Paper and toner' }, { account_id: A('1110'), credit: 275_000, fund_id: U }] } })
    s.cmd(admin, { type: 'entry.postDraft', entryId: d1 })
    const d2 = idOf(s.cmd(admin, { type: 'entry.saveDraft', input: { date: '2026-02-04', memo: 'Fixture draft to delete', lines: [{ account_id: A('7300'), debit: 1_000, fund_id: U }, { account_id: A('1110'), credit: 1_000, fund_id: U }] } }))
    s.cmd(admin, { type: 'entry.deleteDraft', entryId: d2 })
    // A raw entry claiming a system source: the sanitiser makes it manual.
    const e1 = idOf(s.cmd(admin, { type: 'entry.post', input: { date: '2026-02-03', memo: 'Fixture: fuel top-up, Lodwar', source_type: 'bill', source_id: 'x', lines: [{ account_id: A('5700'), debit: 120_000, fund_id: 'fund-wash', grant_id: 'grant-kisima-wash', dimension_ids: ['dim-loc-turkana'] }, { account_id: A('1130'), credit: 120_000, fund_id: 'fund-wash' }] } }))
    s.cmd(admin, { type: 'entry.reverse', entryId: e1, date: '2026-02-04', reason: 'Charged to the wrong grant' })
    s.cmd(admin, { type: 'entry.post', input: { date: '2026-02-02', memo: 'Fixture: cheque 1001, audit retainer', lines: [{ account_id: A('7600'), debit: 50_000, fund_id: U }, { account_id: A('1110'), credit: 50_000, fund_id: U }] } })
    s.cmd(admin, { type: 'entry.import', inputs: [
      { date: '2026-03-02', memo: 'Imported: March rent', source_type: 'opening', lines: [{ account_id: A('7500'), debit: 4_500_000, fund_id: U }, { account_id: A('1110'), credit: 4_500_000, fund_id: U }] },
      { date: '2026-03-05', memo: 'Imported: unbalanced, kept as a draft', lines: [{ account_id: A('7600'), debit: 2_500_000, fund_id: U }, { account_id: A('2200'), credit: 2_400_000, fund_id: U }] },
    ] })

    // Chart, grants, budget.
    const acct = idOf(s.cmd(admin, { type: 'account.add', input: { id: 'caller-chosen-account', code: '7450', name: 'Internet and data bundles', type: 'expense', parent_id: A('7000'), is_group: false, is_cash: false, expense_function: 'management', cash_flow_class: 'operating' } }))
    s.cmd(admin, { type: 'account.update', accountId: acct, patch: { name: 'Internet, data and airtime', expense_function: 'program', type: 'asset', code: '9999' } })
    s.cmd(admin, { type: 'grant.add', input: { id: 'caller-chosen-grant', name: 'Fixture Trust emergency WASH grant', donor_node_id: null, donor_name: 'Fixture Trust', fund_id: 'fund-wash', award: 150_000_000, currency: 'KES', start_date: '2026-02-01', end_date: '2026-07-31', terms: 'Fixture terms.' } })
    // A zero budget with actual spend gives used = Infinity in budgetVsActual.
    s.cmd(admin, { type: 'budgetLine.set', input: { budget_id: 'budget-2026', account_id: acct, fund_id: U, grant_id: null, dimension_id: null, period_id: 'period-2026-02', amount: 0 } })
    const firstBudgetLine = ledger.state.budgetLines[0]
    s.cmd(admin, { type: 'budgetLine.set', input: { ...firstBudgetLine, amount: firstBudgetLine.amount + 10_000_000 } })

    // Payables: a two-fund foreign-currency bill, approved by someone else, part paid.
    const vendor = idOf(s.cmd(admin, { type: 'vendor.add', input: { id: 'caller-chosen-vendor', name: 'Fixture Data Ltd', node_id: null, payment_details: 'Paybill 000000' } }))
    const bill = idOf(s.cmd(CLERK, { type: 'bill.create', input: { id: 'caller-chosen-bill', vendor_id: vendor, number: 'FX-INV-001', bill_date: '2026-02-04', due_date: '2026-03-06', currency: 'USD', fx_rate: 129.37, proof_ids: ['proof-fixture-1'], lines: [
      { description: 'Data bundles, February', account_id: acct, amount: 12_345, fund_id: U, grant_id: null, dimension_ids: ['dim-loc-nairobi'] },
      { description: 'Water quality test kits', account_id: A('5500'), amount: 6_789, fund_id: 'fund-wash', grant_id: 'grant-kisima-wash', dimension_ids: ['dim-loc-turkana'] },
    ] } }))
    s.cmd(admin, { type: 'bill.approve', billId: bill })
    s.cmd(admin, { type: 'bill.post', billId: bill })
    s.cmd(admin, { type: 'bill.pay', billId: bill, date: '2026-02-04', cashAccountId: A('1110'), amount: 10_000, reference: 'EFT FX-1' })

    // Claims: one submitted then rejected; one created submitted with a client id, approved and reimbursed.
    const c1 = idOf(s.cmd(OFFICER, { type: 'claim.create', input: { claim_date: '2026-02-04', currency: 'KES', fx_rate: 1, purpose: 'Fixture: community meeting refreshments', receipt_names: ['receipt-1.jpg'], captured_offline: false, lines: [{ description: 'Refreshments', account_id: A('5600'), amount: 450_000, fund_id: U, grant_id: null, dimension_ids: [] }] } }))
    s.cmd(OFFICER, { type: 'claim.submit', claimId: c1 })
    s.cmd(admin, { type: 'claim.reject', claimId: c1, note: 'No attendance list attached.' })
    const c2 = idOf(s.cmd(OFFICER, { type: 'claim.create', input: { id: 'claim-offline-fixture-1', claim_date: '2026-02-04', currency: 'KES', fx_rate: 1, purpose: 'Fixture: antenatal outreach, Kilifi', receipt_names: ['fuel.jpg', 'meals.jpg'], captured_offline: true, submit: true, lines: [
      { description: 'Fuel', account_id: A('5700'), amount: 830_000, fund_id: 'fund-maternal-health', grant_id: 'grant-northlight-2026', dimension_ids: ['dim-loc-kilifi'] },
      { description: 'Meals', account_id: A('5700'), amount: 215_050, fund_id: 'fund-maternal-health', grant_id: 'grant-northlight-2026', dimension_ids: ['dim-loc-kilifi'] },
    ] } }))
    s.cmd(admin, { type: 'claim.approve', claimId: c2, note: 'Approved against the outreach plan.' })
    s.cmd(admin, { type: 'claim.reimburse', claimId: c2, date: '2026-02-04', cashAccountId: A('1150'), reference: 'RB4FIXTURE1' })

    // Banking: a new account, a statement import (one row already imported), matching.
    s.cmd(admin, { type: 'bank.add', input: { id: 'caller-chosen-bank', name: 'Payroll account', kind: 'bank', provider: 'Fixture Bank', account_id: A('1140'), currency: 'KES', number_masked: '0000' } })
    const fee = ledger.state.statementLines.find((x) => x.bank_account_id === 'bank-main' && x.reference === null && x.amount < 0)
    if (!fee) throw new Error('no reference-less bank-main statement line in the demo books')
    const imported = s.cmd(admin, { type: 'statement.import', bankAccountId: 'bank-main', rows: [
      { date: '2026-02-03', description: 'FIXTURE CHEQUE 1001', reference: 'CHQ1001', amount: -50_000 },
      { date: fee.date, description: fee.description, reference: fee.reference, amount: fee.amount },
    ] })
    s.cmd(admin, { type: 'statement.autoMatch', bankAccountId: 'mpesa-till' })
    const cheque = imported.type === 'ids' ? imported.ids[0] : ''
    const chequeLine = ledger.state.lines.find((l) => l.account_id === A('1110') && l.credit === 50_000 && ledger.entry(l.entry_id)?.memo.startsWith('Fixture: cheque'))
    if (!chequeLine) throw new Error('cheque journal line not found')
    s.cmd(admin, { type: 'statement.match', statementLineId: cheque, journalLineId: chequeLine.id })
    s.cmd(admin, { type: 'statement.unmatch', statementLineId: cheque })
    const interest = ledger.state.statementLines.find((x) => x.bank_account_id === 'bank-main' && !x.matched_line_id && x.amount > 0)
    if (!interest) throw new Error('no unmatched money-in line on bank-main')
    s.cmd(admin, { type: 'statement.post', statementLineId: interest.id, counter: { account_id: A('4300'), fund_id: U, memo: 'Interest, January' } })

    // Periods and years.
    s.cmd(admin, { type: 'period.close', periodId: 'period-2026-01' })
    s.cmd(admin, { type: 'period.lock', periodId: 'period-2026-01' })
    s.cmd(admin, { type: 'period.close', periodId: 'period-2026-02' })
    s.cmd(admin, { type: 'period.reopen', periodId: 'period-2026-02', reason: 'Late supplier invoice' })
    s.cmd(admin, { type: 'year.open', year: 2027 })
    s.cmd(admin, { type: 'year.close', year: 2026 })
  } finally {
    rec.stop()
  }

  const finalState = structuredClone(ledger.state)
  writeFixture(outDir, 'commands-accepted', {
    description:
      'Every LedgerCommand type, accepted at least once: raw commands over the demo books, each through ' +
      'sanitizeCommand then applyCommand, with the sanitised command, the CommandResult and the state delta. ' +
      'Starts from demo-books.json#/state; `final_state` is the books after the last step.',
    sources: [src('commands.ts', 'sanitizeCommand'), src('commands.ts', 'applyCommand')],
    covers: ['applyCommand', 'sanitizeCommand', ...rec.steps.map((x) => `Ledger.${x.method}`)],
    env: doc,
    actors: { admin, clerk: CLERK, officer: OFFICER },
    initial_state: 'demo-books.json#/state',
    delta_format: 'as in demo-backfill.json',
    steps: s.steps,
    final_state: finalState,
  })
  return finalState
}
