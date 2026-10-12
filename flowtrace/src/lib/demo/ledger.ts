// The demo organisation's books, built by running the bundled transactions
// through the bridge (the "backfill") and adding what a real NGO's books
// would hold around them: grants, budgets, bills, staff claims and a bank
// statement. Everything goes through the engine, so the demo obeys the same
// rules a live organisation does.
import {
  Ledger,
  SYSTEM_ACTOR,
  UNRESTRICTED_FUND_ID,
  accountId,
  applyCommand,
  autoMatch,
  bridgeTransaction,
  createOrgLedger,
  fromMajor,
} from '../ledger'
import { DEMO_ORG_ID, DEMO_USER_ID, flowNodes, transactions } from './dataset'
import type {
  Actor,
  BridgeContext,
  BridgeNode,
  BridgeTransaction,
  CommandResult,
  LedgerBackend,
  LedgerCommand,
  LedgerEnv,
  LedgerState,
} from '../ledger'

const A = accountId
const N = (suffix: string) => `a0eebc99-9c0b-4ef8-bb6d-6bb9bd380${suffix}`
const T = (suffix: string) => `b0eebc99-9c0b-4ef8-bb6d-6bb9bd380${suffix}`

/** The demo is a snapshot "as of" its latest movement. */
export const DEMO_AS_OF = '2026-02-04'

export const DEMO_ACTOR: Actor = {
  id: DEMO_USER_ID,
  name: 'Demo User',
  can_post: true,
  can_approve: true,
  can_close: true,
}

/** Field staff who submit claims in the demo (fictional). */
const FIELD_OFFICER: Actor = { id: 'staff-achieng', name: 'Achieng Odhiambo', can_post: false, can_approve: false, can_close: false }
const ME_OFFICER: Actor = { id: 'staff-kiprop', name: 'Kiprop Mutai', can_post: false, can_approve: false, can_close: false }

// Funds and grants. Harambee's grant is core (unrestricted) support; the
// others are restricted to a purpose, as their transaction notes say.
const FUND = {
  unrestricted: UNRESTRICTED_FUND_ID,
  maternal: 'fund-maternal-health',
  wash: 'fund-wash',
  education: 'fund-education',
}
const GRANT = {
  harambee: 'grant-harambee-2026',
  northlight: 'grant-northlight-2026',
  kisima: 'grant-kisima-wash',
  paef: 'grant-paef-education',
}

function seedEnv(): LedgerEnv {
  let n = 0
  return {
    now: () => `${DEMO_AS_OF}T09:00:00.000Z`,
    newId: () => `demo-${(++n).toString(36).padStart(4, '0')}`,
  }
}

export function buildDemoLedger(): Ledger {
  const ledger = createOrgLedger(DEMO_ORG_ID, 'KES', 2026, seedEnv())
  const sys = SYSTEM_ACTOR
  ledger.ensureFiscalYear(2025)

  // Funds, donors and grants ------------------------------------------------
  ledger.addFund(sys, { id: FUND.maternal, name: 'Maternal health (restricted)', restriction: 'temporarily_restricted' })
  ledger.addFund(sys, { id: FUND.wash, name: 'Water and sanitation (restricted)', restriction: 'temporarily_restricted' })
  ledger.addFund(sys, { id: FUND.education, name: 'Education (restricted)', restriction: 'temporarily_restricted' })

  const grant = (id: string, name: string, donor: string, fund: string, award: number, start: string, end: string, terms: string) =>
    ledger.addGrant(sys, {
      id,
      name,
      donor_node_id: N(donor),
      donor_name: flowNodes.find((n) => n.id === N(donor))!.name,
      fund_id: fund,
      award: fromMajor(award),
      currency: 'KES',
      start_date: start,
      end_date: end,
      terms,
    })
  grant(GRANT.harambee, 'Harambee core support 2026', 'a12', FUND.unrestricted, 34_000_000, '2026-01-01', '2026-12-31', 'Quarterly disbursements of KES 8.5M for community health programmes. Unrestricted within the programme.')
  grant(GRANT.northlight, 'Northlight maternal health 2026', 'a20', FUND.maternal, 3_200_000, '2026-01-01', '2026-12-31', 'Restricted to maternal health initiatives in Kilifi.')
  grant(GRANT.kisima, 'Kisima WASH: Turkana boreholes', 'a30', FUND.wash, 4_000_000, '2026-01-01', '2027-06-30', 'Restricted to the Clean Water Turkana project.')
  grant(GRANT.paef, 'PAEF education grant', 'a31', FUND.education, 6_000_000, '2026-02-01', '2027-01-31', 'Restricted to the Turkana Schools programme.')

  // Dimensions: departments, projects and donors from the flow nodes --------
  const dimByNode = new Map<string, string>()
  for (const n of flowNodes) {
    const kind = n.type === 'department' ? 'department' : n.type === 'project' ? 'project' : n.type === 'donor' ? 'donor' : null
    if (!kind) continue
    const d = ledger.addDimension(sys, { id: `dim-${n.id.slice(-3)}`, kind, name: n.name, node_id: n.id })
    dimByNode.set(n.id, d.id)
  }
  ledger.addDimension(sys, { id: 'dim-loc-turkana', kind: 'location', name: 'Turkana', node_id: null })
  ledger.addDimension(sys, { id: 'dim-loc-kilifi', kind: 'location', name: 'Kilifi', node_id: null })
  ledger.addDimension(sys, { id: 'dim-loc-nairobi', kind: 'location', name: 'Nairobi HQ', node_id: null })

  // Vendors from vendor nodes -----------------------------------------------
  const vendorByNode = new Map<string, string>()
  for (const n of flowNodes.filter((x) => x.type === 'vendor')) {
    const v = ledger.addVendor(sys, { id: `vendor-${n.id.slice(-3)}`, name: n.name, node_id: n.id, payment_details: null })
    vendorByNode.set(n.id, v.id)
  }

  // Bank and mobile-money accounts -------------------------------------------
  const cashByNode = new Map([
    [N('a11'), A('1110')], // General Fund
    [N('a13'), A('1120')], // Operations Budget
    [N('a14'), A('1130')], // Project Fund
    [N('a19'), A('1140')], // Payroll
  ])
  ledger.addBankAccount(sys, { id: 'bank-main', name: 'Main operating account', kind: 'bank', provider: 'Equity Bank', account_id: A('1110'), currency: 'KES', number_masked: '•••• 4821' })
  ledger.addBankAccount(sys, { id: 'bank-ops', name: 'Operations account', kind: 'bank', provider: 'Equity Bank', account_id: A('1120'), currency: 'KES', number_masked: '•••• 4839' })
  ledger.addBankAccount(sys, { id: 'bank-projects', name: 'Restricted projects account', kind: 'bank', provider: 'KCB', account_id: A('1130'), currency: 'KES', number_masked: '•••• 0917' })
  ledger.addBankAccount(sys, { id: 'mpesa-till', name: 'M-Pesa till', kind: 'mobile_money', provider: 'Safaricom M-Pesa', account_id: A('1150'), currency: 'KES', number_masked: 'Till 5•••12' })

  // Opening balances, then close and lock 2025 -------------------------------
  ledger.post(sys, {
    date: '2025-12-31',
    memo: 'Opening balances carried forward from the 2025 books',
    source_type: 'opening',
    lines: [
      { account_id: A('1110'), debit: fromMajor(3_000_000), fund_id: FUND.unrestricted },
      { account_id: A('1120'), debit: fromMajor(400_000), fund_id: FUND.unrestricted },
      { account_id: A('1150'), debit: fromMajor(60_000), fund_id: FUND.unrestricted },
      { account_id: A('1510'), debit: fromMajor(2_400_000), fund_id: FUND.unrestricted },
      { account_id: A('2200'), credit: fromMajor(85_000), fund_id: FUND.unrestricted },
      { account_id: A('3100'), credit: fromMajor(5_775_000), fund_id: FUND.unrestricted },
    ],
  })
  for (const p of ledger.state.periods.filter((x) => x.fiscal_year === 2025)) ledger.closePeriod(sys, p.id)
  for (const p of ledger.state.periods.filter((x) => x.fiscal_year === 2025 && x.end_date < '2025-12-01')) ledger.lockPeriod(sys, p.id)

  // Backfill: every bundled transaction through the bridge -------------------
  const nodes = new Map<string, BridgeNode>(flowNodes.map((n) => [n.id, { id: n.id, name: n.name, type: n.type }]))
  // Which grant each spend belongs to. Income follows the donor's grant.
  const grantByDonor: Record<string, string> = {
    [N('a12')]: GRANT.harambee,
    [N('a20')]: GRANT.northlight,
    [N('a30')]: GRANT.kisima,
    [N('a31')]: GRANT.paef,
  }
  const grantBySpend: Record<string, string> = {
    [T('b10')]: GRANT.kisima, // borehole phase 1
    [T('b11')]: GRANT.harambee, // community health worker stipends
    [T('b13')]: GRANT.northlight, // maternal health equipment
    [T('b16')]: GRANT.kisima, // unexplained withdrawal from the projects account
  }
  const fundOfGrant = (g: string) => ledger.grant(g)!.fund_id
  const ctx: BridgeContext = {
    baseCurrency: 'KES',
    nodes,
    cashAccountForNode: (id) => cashByNode.get(id),
    defaultCashAccount: A('1110'),
    dimensionForNode: (id) => dimByNode.get(id),
    grantFor: (t) => {
      const g = (t.from_node_id && grantByDonor[t.from_node_id]) || grantBySpend[t.id]
      return g ? { grant_id: g, fund_id: fundOfGrant(g) } : null
    },
    expenseAccountFor: (t) =>
      ({
        [T('b10')]: A('5100'), // borehole drilling: water and sanitation
        [T('b11')]: A('5400'), // stipends
        [T('b13')]: A('5500'), // equipment
        [T('b14')]: A('1150'), // M-Pesa float top-up: a transfer to the till
      })[t.id],
  }
  const allocations: Array<{ node: string; amount: number }> = []
  for (const t of [...transactions].sort((a, b) => a.transaction_date.localeCompare(b.transaction_date))) {
    const r = bridgeTransaction(t as BridgeTransaction, ctx)
    if (r.kind === 'entry') {
      // The flagged withdrawal is unexplained, so it sits in suspense and is
      // not spend; it needs no grant.
      ledger.post(sys, r.input)
    } else if (r.kind === 'allocation') allocations.push({ node: r.dimensionNodeId, amount: r.amount })
  }

  // Budget FY2026 --------------------------------------------------------------
  const budget = ledger.addBudget(sys, { id: 'budget-2026', name: 'FY2026 budget', fiscal_year: 2026, status: 'approved' })
  const line = (account: string | null, amount: number, o: { fund?: string; grant?: string; dim?: string } = {}) =>
    ledger.setBudgetLine(sys, {
      budget_id: budget.id,
      account_id: account ? A(account) : null,
      fund_id: o.fund ?? null,
      grant_id: o.grant ?? null,
      dimension_id: o.dim ?? null,
      period_id: null,
      amount: fromMajor(amount),
    })
  // Grant budgets, from the award letters.
  line('5100', 3_400_000, { grant: GRANT.kisima })
  line('5700', 300_000, { grant: GRANT.kisima })
  line('7400', 50_000, { grant: GRANT.kisima })
  line('5500', 2_400_000, { grant: GRANT.northlight })
  line('5600', 800_000, { grant: GRANT.northlight })
  line('5300', 5_200_000, { grant: GRANT.paef })
  line('5600', 800_000, { grant: GRANT.paef })
  line('5400', 9_000_000, { grant: GRANT.harambee })
  // Running costs, unrestricted.
  line('6100', 17_400_000, { fund: FUND.unrestricted })
  line('7100', 300_000, { fund: FUND.unrestricted })
  line('7200', 600_000, { fund: FUND.unrestricted })
  line('7300', 480_000, { fund: FUND.unrestricted })
  line('7400', 180_000, { fund: FUND.unrestricted })
  line('5600', 1_200_000, { fund: FUND.unrestricted })
  // Department allocations from the budget transfers.
  for (const a of allocations) line(null, a.amount, { dim: dimByNode.get(a.node) })

  // Accounts payable ----------------------------------------------------------
  const run = (actor: Actor, cmd: LedgerCommand) => applyCommand(ledger, actor, cmd)
  const idOf = (r: CommandResult) => (r.type === 'id' ? r.id : '')
  const power = idOf(run(sys, {
    type: 'bill.create',
    input: {
      vendor_id: vendorByNode.get(N('a34'))!,
      number: 'KPLC-0226-77140',
      bill_date: '2026-02-02',
      due_date: '2026-02-16',
      currency: 'KES',
      fx_rate: 1,
      proof_ids: [],
      lines: [{ description: 'Electricity, January usage, Nairobi HQ', account_id: A('7300'), amount: fromMajor(39_800), fund_id: FUND.unrestricted, grant_id: null, dimension_ids: ['dim-loc-nairobi'] }],
    },
  }))
  run(DEMO_ACTOR, { type: 'bill.approve', billId: power })
  run(DEMO_ACTOR, { type: 'bill.post', billId: power })

  const staples = idOf(run(sys, {
    type: 'bill.create',
    input: {
      vendor_id: vendorByNode.get(N('a17'))!,
      number: 'SK-INV-30418',
      bill_date: '2026-02-03',
      due_date: '2026-03-05',
      currency: 'KES',
      fx_rate: 1,
      proof_ids: [],
      lines: [
        { description: 'Exercise books and stationery, Turkana Schools', account_id: A('5300'), amount: fromMajor(186_000), fund_id: FUND.education, grant_id: GRANT.paef, dimension_ids: ['dim-loc-turkana'] },
        { description: 'Printer toner, HQ', account_id: A('7100'), amount: fromMajor(14_500), fund_id: FUND.unrestricted, grant_id: null, dimension_ids: ['dim-loc-nairobi'] },
      ],
    },
  }))
  run(DEMO_ACTOR, { type: 'bill.approve', billId: staples })

  const autox = idOf(run(sys, {
    type: 'bill.create',
    input: {
      vendor_id: vendorByNode.get(N('a18'))!,
      number: 'AX-2026-0112',
      bill_date: '2026-01-26',
      due_date: '2026-02-09',
      currency: 'KES',
      fx_rate: 1,
      proof_ids: [],
      lines: [{ description: 'Tyres, field vehicle KDA 412M', account_id: A('7200'), amount: fromMajor(48_000), fund_id: FUND.unrestricted, grant_id: null, dimension_ids: [dimByNode.get(N('a16'))!] }],
    },
  }))
  run(DEMO_ACTOR, { type: 'bill.approve', billId: autox })
  run(DEMO_ACTOR, { type: 'bill.post', billId: autox })
  run(DEMO_ACTOR, { type: 'bill.pay', billId: autox, date: '2026-02-02', cashAccountId: A('1120'), amount: fromMajor(48_000), reference: 'EFT 88213' })

  // Staff expense claims ------------------------------------------------------
  const reimbursed = idOf(run(ME_OFFICER, {
    type: 'claim.create',
    input: {
      claim_date: '2026-01-21',
      currency: 'KES',
      fx_rate: 1,
      purpose: 'Baseline survey, Kilifi health centres',
      receipt_names: ['matatu-receipts.jpg', 'lodging-kilifi.pdf'],
      captured_offline: false,
      submit: true,
      lines: [
        { description: 'Transport, Mombasa to Kilifi and back', account_id: A('5700'), amount: fromMajor(6_400), fund_id: FUND.maternal, grant_id: GRANT.northlight, dimension_ids: [dimByNode.get(N('a32'))!, 'dim-loc-kilifi'] },
        { description: 'Lodging, 2 nights', account_id: A('5700'), amount: fromMajor(5_900), fund_id: FUND.maternal, grant_id: GRANT.northlight, dimension_ids: [dimByNode.get(N('a32'))!, 'dim-loc-kilifi'] },
      ],
    },
  }))
  run(DEMO_ACTOR, { type: 'claim.approve', claimId: reimbursed, note: 'Receipts match the survey plan.' })
  run(DEMO_ACTOR, { type: 'claim.reimburse', claimId: reimbursed, date: '2026-01-23', cashAccountId: A('1150'), reference: 'RAN4K2PQ7Z' })

  run(FIELD_OFFICER, {
    type: 'claim.create',
    input: {
      claim_date: '2026-02-03',
      currency: 'KES',
      fx_rate: 1,
      purpose: 'Borehole site inspection, Lodwar',
      receipt_names: ['fuel-lodwar.jpg', 'meals.jpg'],
      captured_offline: true,
      submit: true,
      lines: [
        { description: 'Fuel, motorbike hire', account_id: A('5700'), amount: fromMajor(14_200), fund_id: FUND.wash, grant_id: GRANT.kisima, dimension_ids: [dimByNode.get(N('a35'))!, 'dim-loc-turkana'] },
        { description: 'Meals, 2 days', account_id: A('5700'), amount: fromMajor(4_300), fund_id: FUND.wash, grant_id: GRANT.kisima, dimension_ids: [dimByNode.get(N('a35'))!, 'dim-loc-turkana'] },
      ],
    },
  })

  // Bank statement for the main account: January, reconciled ---------------
  const mainLines = ledger.state.lines.filter((l) => {
    const e = ledger.entry(l.entry_id)!
    return l.account_id === A('1110') && e.source_type !== 'opening' && e.date <= '2026-01-31'
  })
  const statementRows = mainLines.map((l) => {
    const e = ledger.entry(l.entry_id)!
    const ref: string | null = e.memo.split(' · ').at(1) ?? null
    // Banks clear a day or two after the books record a movement.
    const shift = e.source_type === 'transaction' ? 1 : 0
    return { date: addDays(e.date, shift), description: e.memo.split(' · ')[0].toUpperCase(), reference: ref, amount: l.debit - l.credit }
  })
  statementRows.push(
    { date: '2026-01-31', description: 'LEDGER FEE JAN 2026', reference: null, amount: -fromMajor(1_250) },
    { date: '2026-01-31', description: 'INTEREST CREDIT', reference: null, amount: fromMajor(3_412) },
  )
  ledger.importStatement(sys, 'bank-main', statementRows)
  autoMatch(ledger, sys, 'bank-main')
  // The bank fee is explained and posted; the interest is left for the demo.
  const fee = ledger.state.statementLines.find((s) => s.description === 'LEDGER FEE JAN 2026')!
  ledger.postFromStatementLine(sys, fee.id, { account_id: A('7700'), fund_id: FUND.unrestricted, memo: 'Equity Bank ledger fee, January' })

  // M-Pesa till statement, early February: not yet matched.
  ledger.importStatement(sys, 'mpesa-till', [
    { date: '2026-01-23', description: 'Business Payment to KIPROP MUTAI', reference: 'RAN4K2PQ7Z', amount: -fromMajor(12_300) },
    { date: '2026-02-01', description: 'Customer Transfer Fuliza charge', reference: 'RB17Q0XT3M', amount: -fromMajor(410) },
    { date: '2026-02-03', description: 'Funds received from Equity Bank', reference: 'RB34NNA81C', amount: fromMajor(15_000) },
  ])

  return ledger
}

function addDays(d: string, n: number) {
  const t = new Date(`${d}T00:00:00Z`)
  t.setUTCDate(t.getUTCDate() + n)
  return t.toISOString().slice(0, 10)
}

// ---------------------------------------------------------------------------
// Demo backend: in memory, reset on reload like the rest of the demo.
// ---------------------------------------------------------------------------

let demoLedger: Ledger | null = null
const listeners = new Set<(s: LedgerState) => void>()

function current() {
  demoLedger ??= buildDemoLedger()
  return demoLedger
}

/** Restore the seed state. Used by tests. */
export function resetDemoLedger() {
  demoLedger = null
}

let liveIds = 0
const liveEnv: LedgerEnv = {
  now: () => new Date().toISOString(),
  newId: () => `local-${Date.now().toString(36)}-${(++liveIds).toString(36)}`,
}

const settle = <T>(value: T): Promise<T> =>
  new Promise((resolve) => setTimeout(() => resolve(structuredClone(value)), 12))

export const demoLedgerBackend: LedgerBackend = {
  kind: 'demo',
  load: (organizationId) => {
    if (organizationId !== DEMO_ORG_ID) return Promise.reject(new Error('Only the demo organisation has books in demo mode.'))
    return settle(current().state)
  },
  run: (organizationId, cmd, actor) => {
    if (organizationId !== DEMO_ORG_ID) return Promise.reject(new Error('Only the demo organisation has books in demo mode.'))
    // Apply to a copy so a rejected command leaves nothing half-done, the
    // way a SpacetimeDB reducer rolls back.
    const draft = new Ledger(structuredClone(current().state), liveEnv)
    try {
      const result = applyCommand(draft, actor, cmd)
      demoLedger = draft
      for (const fn of listeners) fn(draft.state)
      return settle(result)
    } catch (e) {
      return new Promise((_, reject) => setTimeout(() => reject(e), 12))
    }
  },
  subscribe: (_org, onChange) => {
    listeners.add(onChange)
    return () => listeners.delete(onChange)
  },
}
