// Rejected-command cases from the TS tests: each test that asserts a
// rejection becomes a scenario that repeats the test's calls in order on
// fresh books, the accepted calls recorded as setup and every rejected call
// with the exact LedgerError the engine throws. The test's own helpers and
// env are mirrored here line for line; coverage.ts checks that every such
// test is present with as many rejections as it asserts.
//
// Property tests (fast-check, random inputs) are left to the Rust property
// tests (task C7). reconcile.test.ts asserts no rejection.
import { frozenEnv, writeFixture } from './fixture'
import { UNRESTRICTED_FUND_ID, accountId, createOrgLedger } from './flowtrace'
import type { Actor, EntryInput, ExpenseClaim, Bill, JournalEntry, Payment } from './flowtrace'
import { src } from './sources'
import { Scenario } from './trace'

const A = accountId
const U = UNRESTRICTED_FUND_ID

// engine.test.ts:12-14 and review-regressions.test.ts:12-13
const admin: Actor = { id: 'u1', name: 'Admin', can_post: true, can_approve: true, can_close: true }
const clerk: Actor = { id: 'u2', name: 'Clerk', can_post: true, can_approve: false, can_close: false }
const viewer: Actor = { id: 'u3', name: 'Viewer', can_post: false, can_approve: false, can_close: false }

/** testEnv() in engine.test.ts:16-19 and env() in review-regressions.test.ts:16-19. */
const testEnv = () =>
  frozenEnv(
    '2026-03-01T00:00:00.000Z',
    (n) => `id-${n}`,
    'id- then n in decimal, n = 1, 2, 3, ... (id-1)',
    'flowtrace/src/lib/ledger/engine.test.ts:16-19 (testEnv) and review-regressions.test.ts:16-19 (env), injected through LedgerEnv (engine.ts:41-44)',
  )

interface ScenarioRecord {
  file: string
  test: string | null
  base: string
  scenario: Scenario
}

const scenarios: Array<ScenarioRecord> = []

/** createOrgLedger('org', 'KES', 2026, env), as both test files start. */
function org() {
  return createOrgLedger('org', 'KES', 2026, testEnv().env)
}

/** fresh() in engine.test.ts:24-40. */
function fresh(file: string, test: string) {
  const s = new Scenario(org())
  const restricted = s.ok<{ id: string }>('addFund', admin, { id: 'fund-wash', name: 'WASH', restriction: 'temporarily_restricted' })
  s.ok('addGrant', admin, { id: 'grant-wash', name: 'Water grant', donor_node_id: null, donor_name: 'Water Trust', fund_id: restricted.id, award: 1_000_000, currency: 'KES', start_date: '2026-01-01', end_date: '2026-06-30', terms: '' })
  scenarios.push({ file, test, base: 'engine.test.ts fresh()', scenario: s })
  return s
}

/** books() in review-regressions.test.ts:21-28. */
function books(file: string, test: string | null) {
  const s = new Scenario(org())
  s.ok('addFund', admin, { id: 'f-r', name: 'Restricted', restriction: 'temporarily_restricted' })
  s.ok('addGrant', admin, { id: 'g', name: 'Grant', donor_node_id: null, donor_name: 'D', fund_id: 'f-r', award: 10_000_000, currency: 'KES', start_date: '2026-01-01', end_date: '2026-12-31', terms: '' })
  s.ok('addVendor', admin, { id: 'v', name: 'Vendor', node_id: null, payment_details: null })
  s.ok('addBankAccount', admin, { id: 'b', name: 'Main', kind: 'bank', provider: 'X', account_id: A('1110'), currency: 'KES', number_masked: '1' })
  scenarios.push({ file, test, base: 'review-regressions.test.ts books()', scenario: s })
  return s
}

/** simple() in engine.test.ts:42-49. */
const simple = (amount: number, date = '2026-01-15'): EntryInput => ({
  date,
  memo: 'Office supplies',
  lines: [
    { account_id: A('7100'), debit: amount, fund_id: U },
    { account_id: A('1110'), credit: amount, fund_id: U },
  ],
})

/** grantSpend() in engine.test.ts:163-170. */
const grantSpend = (amount: number, date = '2026-02-01', grant: string | null = 'grant-wash'): EntryInput => ({
  date,
  memo: 'Borehole',
  lines: [
    { account_id: A('5100'), debit: amount, fund_id: 'fund-wash', grant_id: grant },
    { account_id: A('1130'), credit: amount, fund_id: 'fund-wash' },
  ],
})

function engineTests() {
  const E = 'engine.test.ts'
  {
    const s = fresh(E, 'rejects unbalanced entries')
    const input = simple(50_000)
    input.lines[1].credit = 49_999
    s.reject('post', admin, input)
  }
  {
    const s = fresh(E, 'rejects lines with both sides, zero or negative amounts, or fractional cents')
    s.reject('post', admin, { ...simple(1), lines: [{ account_id: A('7100'), debit: 5, credit: 5, fund_id: U }, { account_id: A('1110'), credit: 0, fund_id: U }] })
    s.reject('post', admin, simple(-5))
    s.reject('post', admin, simple(10.5))
  }
  {
    const s = fresh(E, 'rejects posting to header accounts and single-line entries')
    const input = simple(100)
    input.lines[0].account_id = A('7000')
    s.reject('post', admin, input)
    s.reject('post', admin, { ...simple(100), lines: [simple(100).lines[0]] })
  }
  {
    const s = fresh(E, 'requires every fund to balance within an entry')
    s.reject('post', admin, {
      date: '2026-01-15',
      memo: 'Cross-fund',
      lines: [
        { account_id: A('5100'), debit: 100, fund_id: 'fund-wash', grant_id: 'grant-wash' },
        { account_id: A('1110'), credit: 100, fund_id: U },
      ],
    })
  }
  {
    const s = fresh(E, 'needs posting permission')
    s.reject('post', viewer, simple(100))
  }
  {
    const s = fresh(E, 'makes posted entries immutable; corrections are reversals')
    const e = s.ok<JournalEntry>('post', admin, simple(10_000))
    s.reject('updateDraft', admin, e.id, simple(1))
    s.reject('deleteDraft', admin, e.id)
    s.ok('reverse', admin, e.id, '2026-01-20', 'Wrong account')
    s.reject('reverse', admin, e.id, '2026-01-21', 'again')
  }
  {
    const s = fresh(E, 'blocks posting into closed and locked periods, and closes in order')
    s.reject('closePeriod', admin, 'period-2026-02')
    s.reject('closePeriod', clerk, 'period-2026-01')
    s.ok('closePeriod', admin, 'period-2026-01')
    s.reject('post', admin, simple(100))
    s.ok('lockPeriod', admin, 'period-2026-01')
    s.reject('post', admin, simple(100))
    s.reject('reopenPeriod', admin, 'period-2026-01', 'fix')
    s.reject('post', admin, simple(100, '2027-01-02'))
  }
  {
    const s = fresh(E, 'will not close a period with drafts in it')
    s.ok('saveDraft', admin, simple(100))
    s.reject('closePeriod', admin, 'period-2026-01')
  }
  {
    const s = fresh(E, 'requires restricted spend to name its grant')
    s.reject('post', admin, grantSpend(100, '2026-02-01', null))
  }
  {
    const s = fresh(E, 'blocks spend outside the grant’s dates unless overridden with a reason')
    s.reject('post', admin, grantSpend(100, '2026-07-02'))
    s.ok('post', admin, { ...grantSpend(100, '2026-07-02'), override_reason: 'Donor approved no-cost extension' })
  }
  {
    const s = fresh(E, 'blocks spend over the award unless overridden')
    s.ok('post', admin, grantSpend(900_000))
    s.reject('post', admin, grantSpend(100_001))
    s.ok('post', admin, grantSpend(100_000))
  }
  {
    const s = fresh(E, 'rejects a grant on the wrong fund')
    const input = grantSpend(100)
    input.lines.forEach((x) => (x.fund_id = U))
    s.reject('post', admin, input)
  }
  {
    const s = fresh(E, 'freezes base amounts at posting and balances in base currency')
    s.ok('post', admin, {
      date: '2026-01-10',
      memo: 'USD grant received',
      lines: [
        { account_id: A('1110'), debit: 100_000, currency: 'USD', fx_rate: 129.5, fund_id: U },
        { account_id: A('4100'), credit: 100_000, currency: 'USD', fx_rate: 129.5, fund_id: U },
      ],
    })
    s.reject('post', admin, { ...simple(1), lines: [{ account_id: A('1110'), debit: 1, currency: 'USD', fund_id: U }, { account_id: A('4100'), credit: 1, currency: 'USD', fund_id: U }] })
  }
  {
    const s = fresh(E, 'claims post on approval, cannot be self-approved, and reimburse to zero')
    const officer: Actor = { id: 'u9', name: 'Field Officer', can_post: false, can_approve: false, can_close: false }
    const c = s.ok<ExpenseClaim>('createClaim', officer, {
      claim_date: '2026-02-03',
      currency: 'KES',
      fx_rate: 1,
      purpose: 'Site visit',
      receipt_names: [],
      captured_offline: false,
      submit: true,
      lines: [{ description: 'Matatu fare', account_id: A('5700'), amount: 250_000, fund_id: 'fund-wash', grant_id: 'grant-wash', dimension_ids: [] }],
    })
    s.reject('approveClaim', officer, c.id)
    const { id: _id, number: _n, status: _s, approvals: _a, entry_id: _e, claimant: _c, claimant_name: _cn, ...rest } = c
    s.reject('createClaim', admin, { ...rest, id: c.id, lines: [] })
    s.reject('createClaim', admin, { ...rest, id: c.id, lines: c.lines })
    const selfClaim = s.ok<ExpenseClaim>('createClaim', admin, { ...rest, submit: true, lines: c.lines.map(({ id: _lid, ...x }) => x) })
    s.reject('approveClaim', admin, selfClaim.id)
    s.ok('approveClaim', admin, c.id, 'OK')
    s.ok('reimburseClaim', admin, c.id, { date: '2026-02-04', cash_account_id: A('1150'), reference: 'QAB12XYZ' })
  }
  {
    const s = fresh(E, 'moves income and expense to net assets by fund and closes the year')
    s.ok('post', admin, {
      date: '2026-01-05',
      memo: 'Grant',
      lines: [
        { account_id: A('1130'), debit: 500_000, fund_id: 'fund-wash', grant_id: 'grant-wash' },
        { account_id: A('4100'), credit: 500_000, fund_id: 'fund-wash', grant_id: 'grant-wash' },
      ],
    })
    s.ok('post', admin, simple(20_000))
    s.ok('closeYear', admin, 2026)
    s.reject('closeYear', admin, 2026)
  }
}

function reviewRegressionTests() {
  const R = 'review-regressions.test.ts'
  {
    const s = books(R, 'rejects unknown commands, string rates and caller-chosen ids')
    s.rejectCmd(admin, { type: 'nope' })
    const fxString = { type: 'entry.post', input: { date: '2026-02-01', memo: 'x', lines: [{ account_id: A('1110'), debit: 1, currency: 'USD', fx_rate: '130', fund_id: U }, { account_id: A('4100'), credit: 1, currency: 'USD', fx_rate: '130', fund_id: U }] } }
    s.rejectCmd(admin, fxString)
    s.cmd(admin, { type: 'vendor.add', input: { id: 'v', name: 'Twin', node_id: null, payment_details: null } })
  }
  {
    const s = books(R, 'a bill cannot be approved by the person who entered it')
    const b = s.ok<Bill>('createBill', admin, { vendor_id: 'v', number: '1', bill_date: '2026-02-01', due_date: '2026-03-01', currency: 'KES', fx_rate: 1, proof_ids: [], lines: [{ description: 'x', account_id: A('7100'), amount: 100, fund_id: U, grant_id: null, dimension_ids: [] }] })
    s.reject('approveBill', admin, b.id)
  }
  {
    const s = books(R, 'reversing a payment reopens the bill; a paid bill cannot be reversed')
    // bill() in review-regressions.test.ts:85-93
    const b = s.ok<Bill>('createBill', clerk, {
      vendor_id: 'v', number: 'B', bill_date: '2026-02-01', due_date: '2026-03-01', currency: 'KES', fx_rate: 1, proof_ids: [],
      lines: [{ description: 'x', account_id: A('5500'), amount: 100, fund_id: U, grant_id: null, dimension_ids: [] }],
    })
    s.ok('approveBill', admin, b.id)
    s.ok('postBill', admin, b.id)
    const p = s.ok<Payment>('payBill', admin, b.id, { date: '2026-02-02', cash_account_id: A('1110'), amount: 100 })
    s.reject('reverse', admin, b.entry_id, '2026-02-03', 'wrong')
    s.ok('reverse', admin, p.entry_id, '2026-02-03', 'bounced')
    s.ok('reverse', admin, b.entry_id, '2026-02-03', 'duplicate bill')
  }
}

/** The sanitiser's other rejections (commands.ts:107-137), not asserted by a test. */
function sanitizerCases() {
  const s = books('commands.ts', null)
  s.rejectCmd(admin, null)
  s.rejectCmd(admin, { type: 'entry.post' })
  s.rejectCmd(admin, { type: 'entry.saveDraft', input: { date: '2026-02-01', memo: 7, lines: [] } })
  s.rejectCmd(admin, { type: 'entry.import', inputs: 'not a list' })
  s.rejectCmd(admin, { type: 'entry.import', inputs: [{ date: '2026-02-01', memo: 'No lines' }] })
}

export function exportRejected(outDir: string) {
  scenarios.length = 0
  engineTests()
  reviewRegressionTests()
  sanitizerCases()
  const doc = testEnv().doc
  const total = scenarios.reduce((n, x) => n + x.scenario.rejections, 0)
  const fromTests = scenarios.filter((x) => x.test !== null).reduce((n, x) => n + x.scenario.rejections, 0)
  writeFixture(outDir, 'rejected-cases', {
    description:
      'Rejections asserted by the TS tests (engine.test.ts, review-regressions.test.ts; reconcile.test.ts asserts ' +
      'none), one scenario per test: the test’s calls in order on books built as the test builds them, accepted ' +
      'calls as setup (result and delta) and each rejected call with the error the engine throws (code, message, ' +
      'issues) and whether the books were left unchanged. Plus the sanitiser’s own rejections (test: null).',
    sources: [
      src('engine.test.ts'), src('review-regressions.test.ts'), src('reconcile.test.ts'),
      src('errors.ts', 'LedgerError'), src('commands.ts', 'sanitizeCommand'),
    ],
    covers: [
      'LedgerError', 'fail', 'sanitizeCommand', 'applyCommand',
      ...scenarios.flatMap((x) => x.scenario.methods.map((m) => `Ledger.${m}`)),
    ],
    env: doc,
    setup:
      'Each scenario starts from createOrgLedger(\'org\', \'KES\', 2026, env) with a fresh env (setup.json), then ' +
      'runs its steps. op "method" calls the Ledger method with args as written; op "command" sends `command` ' +
      'through sanitizeCommand then applyCommand as `actor`.',
    counts: { scenarios: scenarios.length, rejected: total, rejected_from_tests: fromTests },
    scenarios: scenarios.map((x) => ({ file: x.file, test: x.test, base: x.base, steps: x.scenario.steps })),
  })
  return { total, fromTests }
}
