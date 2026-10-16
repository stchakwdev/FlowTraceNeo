// Bridge from the original transaction model to the ledger (plan §5).
// A transaction is a business document; posting it creates a journal entry.
// from_node/to_node stay on the lines as flow_node_id, so every edge of the
// flow map traces to its journal lines.
import { UNRESTRICTED_FUND_ID, accountId } from './coa-template'
import { fromMajor } from './money'
import type { EntryInput, LineInput } from './types'

export interface BridgeNode {
  id: string
  name: string
  type: 'donor' | 'department' | 'vendor' | 'beneficiary' | 'account' | 'project'
}

export interface BridgeTransaction {
  id: string
  from_node_id: string | null
  to_node_id: string | null
  amount: number // major units, as stored on transactions
  currency: string
  exchange_rate: number | null
  description: string
  category: string | null
  reference: string | null
  verification_status: 'pending' | 'verified' | 'flagged' | 'rejected'
  transaction_date: string
}

export interface BridgeContext {
  baseCurrency: string
  nodes: Map<string, BridgeNode>
  /** GL cash account for each `account`-type node. */
  cashAccountForNode: (nodeId: string) => string | undefined
  /** Cash account that pays for spend made by departments and projects. */
  defaultCashAccount: string
  /** The grant (and so the fund) a transaction belongs to, if any. */
  grantFor: (t: BridgeTransaction) => { grant_id: string; fund_id: string } | null
  /** Dimension values standing for flow nodes (departments, projects, donors). */
  dimensionForNode: (nodeId: string) => string | undefined
  /** Optional per-transaction expense account override. */
  expenseAccountFor?: (t: BridgeTransaction) => string | undefined
}

export type BridgeResult =
  | { kind: 'entry'; input: EntryInput }
  /** Budget allocation to a department or project: a budget, not a posting. */
  | { kind: 'allocation'; dimensionNodeId: string; amount: number }
  | { kind: 'skip'; reason: string }

const CATEGORY_ACCOUNT: Record<string, string> = {
  salaries: '6100',
  operations: '7100',
  supplies: '7100',
  transport: '7200',
  utilities: '7300',
  training: '5600',
  equipment: '5500',
  programs: '5500',
  funding: '4100',
}

export function expenseAccountForCategory(category: string | null, to: BridgeNode | undefined) {
  const key = (category ?? '').toLowerCase()
  if (key === 'programs' && to?.type === 'beneficiary') return accountId('5400')
  const code = CATEGORY_ACCOUNT[key]
  return code && code !== '4100' ? accountId(code) : null
}

export function bridgeTransaction(t: BridgeTransaction, ctx: BridgeContext): BridgeResult {
  if (t.verification_status === 'rejected') return { kind: 'skip', reason: 'Rejected transactions are not posted.' }
  const from = t.from_node_id ? ctx.nodes.get(t.from_node_id) : undefined
  const to = t.to_node_id ? ctx.nodes.get(t.to_node_id) : undefined
  const amount = fromMajor(t.amount)
  if (amount <= 0) return { kind: 'skip', reason: 'No amount.' }
  const currency = t.currency
  const fx_rate = currency === ctx.baseCurrency ? 1 : (t.exchange_rate ?? undefined)
  if (!fx_rate) return { kind: 'skip', reason: `No exchange rate for ${currency}.` }
  const category = (t.category ?? '').toLowerCase()
  const memo = t.reference ? `${t.description} · ${t.reference}` : t.description
  const dims = (...nodes: Array<BridgeNode | undefined>) =>
    nodes.map((n) => (n ? ctx.dimensionForNode(n.id) : undefined)).filter((d): d is string => !!d)
  const cashOf = (n: BridgeNode | undefined) =>
    n?.type === 'account' ? ctx.cashAccountForNode(n.id) : undefined

  const base = { currency, fx_rate }
  const entry = (lines: Array<LineInput>): BridgeResult => ({
    kind: 'entry',
    input: { date: t.transaction_date, memo, source_type: 'transaction', source_id: t.id, lines },
  })

  // Budget allocations to departments and projects are budgets, not money
  // leaving the bank.
  if (category === 'budget' && (to?.type === 'department' || to?.type === 'project'))
    return { kind: 'allocation', dimensionNodeId: to.id, amount: t.amount }

  // Income: a donor pays into a cash account.
  if (from?.type === 'donor') {
    const cash = cashOf(to) ?? ctx.defaultCashAccount
    const g = ctx.grantFor(t)
    const fund_id = g?.fund_id ?? UNRESTRICTED_FUND_ID
    const income = category === 'funding' || g ? accountId('4100') : accountId('4200')
    return entry([
      { ...base, account_id: cash, debit: amount, fund_id, grant_id: g?.grant_id ?? null, flow_node_id: to?.id ?? null },
      { ...base, account_id: income, credit: amount, fund_id, grant_id: g?.grant_id ?? null, dimension_ids: dims(from), flow_node_id: from.id },
    ])
  }

  const g = ctx.grantFor(t)
  const fund_id = g?.fund_id ?? UNRESTRICTED_FUND_ID
  const grant_id = g?.grant_id ?? null

  // A transfer between two cash accounts (not payroll, which is spend).
  const fromCash = cashOf(from)
  const toCash = cashOf(to)
  if (fromCash && toCash && category !== 'salaries') {
    return entry([
      { ...base, account_id: toCash, debit: amount, fund_id, flow_node_id: to!.id },
      { ...base, account_id: fromCash, credit: amount, fund_id, flow_node_id: from!.id },
    ])
  }

  // Spend. Money with no recipient goes to suspense until it is explained.
  const credit = fromCash ?? ctx.defaultCashAccount
  const debit = !to
    ? accountId('1400')
    : (ctx.expenseAccountFor?.(t) ?? expenseAccountForCategory(t.category, to) ?? accountId('1400'))
  return entry([
    { ...base, account_id: debit, debit: amount, fund_id, grant_id, dimension_ids: dims(from, to), flow_node_id: to?.id ?? null },
    { ...base, account_id: credit, credit: amount, fund_id, flow_node_id: from?.id ?? null },
  ])
}
