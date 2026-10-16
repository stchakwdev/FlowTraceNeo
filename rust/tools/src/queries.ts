// The engine's read methods over the demo books: lookups, validateEntry
// (what the entry form shows before posting), grantSpend, inputOf and
// outstanding. Each case is the method, its arguments and its return value
// or the error it throws.
import { canonical } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import { Ledger, UNRESTRICTED_FUND_ID, accountId } from './flowtrace'
import type { EntryInput, LedgerState } from './flowtrace'
import { src } from './sources'
import { errorOf } from './trace'

const A = accountId
const U = UNRESTRICTED_FUND_ID

export function exportQueries(outDir: string, demoState: LedgerState) {
  // Reads never call the env; a throwing env proves it.
  const ledger = new Ledger(structuredClone(demoState), {
    now: () => {
      throw new Error('a read called env.now')
    },
    newId: () => {
      throw new Error('a read called env.newId')
    },
  })
  const fp = JSON.stringify(ledger.state)
  const cases: Array<Record<string, Json>> = []
  const q = (method: string, ...args: Array<unknown>) => {
    const fn = (ledger as unknown as Record<string, (...a: Array<unknown>) => unknown>)[method]
    try {
      const result = fn.apply(ledger, args)
      cases.push({ method, args: canonical(args), result: result === undefined ? null : canonical(result) })
    } catch (e) {
      cases.push({ method, args: canonical(args), error: errorOf(e) })
    }
  }
  const st = ledger.state
  const posted = st.entries.filter((e) => e.status === 'posted')
  const docEntries = st.entries.filter((e) => e.source_type === 'bill' || e.source_type === 'expense_claim')

  for (const id of [A('1110'), A('7000'), 'acc-missing']) q('account', id)
  for (const code of ['1110', '5100', '9999']) q('accountByCode', code)
  for (const id of ['fund-wash', 'fund-missing']) q('fund', id)
  for (const id of ['grant-kisima-wash', 'grant-missing']) q('grant', id)
  for (const id of [st.entries[0].id, st.entries[st.entries.length - 1].id, 'entry-missing']) q('entry', id)
  for (const id of [st.entries[0].id, docEntries[0].id, 'entry-missing']) q('linesOf', id)
  for (const d of ['2024-12-31', '2025-06-15', '2025-12-31', '2026-02-04', '2026-12-31', '2027-01-01']) q('periodFor', d)
  for (const e of [...posted.slice(0, 2), ...docEntries]) q('inputOf', e.id)
  q('inputOf', 'entry-missing')
  for (const g of st.grants) q('grantSpend', g.id)
  const grantEntry = st.lines.find((l) => l.grant_id === 'grant-kisima-wash' && A('5100') === l.account_id)
  if (grantEntry) q('grantSpend', 'grant-kisima-wash', grantEntry.entry_id)
  for (const b of st.bills) q('outstanding', 'bill', b.id)
  for (const c of st.claims) q('outstanding', 'expense_claim', c.id)
  q('outstanding', 'bill', 'bill-missing')
  for (const c of st.claims) q('claim', c.id)
  q('claim', 'claim-missing')

  // validateEntry: inputs a person might type, each with every issue it has.
  const pay = (amount: number, extra: Partial<EntryInput> = {}): EntryInput => ({
    date: '2026-02-04',
    memo: 'Fixture validation',
    lines: [{ account_id: A('7100'), debit: amount, fund_id: U }, { account_id: A('1110'), credit: amount, fund_id: U }],
    ...extra,
  })
  const wash = (amount: number, date: string, grant: string | null = 'grant-kisima-wash'): EntryInput => ({
    date,
    memo: 'Fixture borehole spend',
    lines: [
      { account_id: A('5100'), debit: amount, fund_id: 'fund-wash', grant_id: grant },
      { account_id: A('1130'), credit: amount, fund_id: 'fund-wash' },
    ],
  })
  const inputs: Array<[EntryInput, string?, { system?: boolean }?]> = [
    [pay(10_000)],
    [pay(10_000, { date: '2026-13-01' })],
    [pay(10_000, { memo: '   ' })],
    [pay(10_000, { date: '2025-11-15' })],
    [pay(10_000, { date: '2025-12-15' })],
    [pay(10_000, { date: '2028-01-01' })],
    [pay(10_000, { lines: [{ account_id: A('7100'), debit: 10_000, fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('7100'), debit: 10_000, fund_id: U }, { account_id: A('1110'), credit: 9_999, fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('7000'), debit: 10_000, fund_id: U, dimension_ids: ['dim-gone'] }, { account_id: 'acc-missing', credit: 10_000, fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('7100'), debit: 10_000, fund_id: 'fund-gone' }, { account_id: A('1110'), credit: 10_000, fund_id: U, grant_id: 'grant-gone' }] })],
    [pay(10_000, { lines: [{ account_id: A('7100'), debit: 1, credit: 1, fund_id: U }, { account_id: A('1110'), debit: -1, fund_id: U }, { account_id: A('1110'), debit: 0.5, fund_id: U }, { account_id: A('1110'), fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('1110'), debit: 10_000, currency: 'USD', fund_id: U }, { account_id: A('4100'), credit: 10_000, currency: 'USD', fx_rate: 0, fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('1110'), debit: 333, currency: 'USD', fx_rate: 129.37, fund_id: U }, { account_id: A('4100'), credit: 43_080, fund_id: U }] })],
    [pay(10_000, { lines: [{ account_id: A('5100'), debit: 10_000, fund_id: 'fund-wash', grant_id: 'grant-kisima-wash' }, { account_id: A('1110'), credit: 10_000, fund_id: U }] })],
    [wash(10_000, '2026-02-04', null)],
    [wash(10_000, '2026-02-04', 'grant-northlight-2026')],
    [wash(10_000, '2027-07-01')],
    [wash(500_000_000, '2026-02-04')],
    [{ ...wash(500_000_000, '2026-02-04'), override_reason: 'Donor approved' }],
    [wash(500_000_000, '2026-02-04'), undefined, { system: true }],
    [wash(10_000, '2027-07-01'), undefined, { system: true }],
  ]
  if (grantEntry) inputs.push([wash(10_000, '2026-02-04'), grantEntry.entry_id])
  for (const [input, ignore, opts] of inputs) q('validateEntry', input, ignore, opts)

  if (JSON.stringify(ledger.state) !== fp) throw new Error('a read method changed the books')
  writeFixture(outDir, 'engine-queries', {
    description:
      'The Ledger read methods over the demo books: account, accountByCode, fund, grant, entry, linesOf, periodFor, ' +
      'inputOf, grantSpend, outstanding, claim and validateEntry (every issue an input has, with line and ' +
      'overridable). Each case: method, args (undefined written as null), result or error.',
    sources: [src('engine.ts', 'Ledger')],
    covers: [...new Set(cases.map((c) => `Ledger.${String(c.method)}`))],
    env: null,
    env_note: 'reads never call the env; this run injects one that throws if called',
    state: 'demo-books.json#/state',
    cases,
  })
}
