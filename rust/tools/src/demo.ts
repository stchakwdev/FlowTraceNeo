// The demo books (flowtrace/src/lib/demo/ledger.ts#buildDemoLedger), run
// unchanged with the engine's prototype wrapped by the recorder, so every
// top-level write of the backfill is captured with its arguments, its return
// value and the state delta. Also the bridge (bridge.ts) over the demo
// transactions, with the demo's bridge context written out as data.
import { canonical } from './canonical'
import type { Json } from './canonical'
import { writeFixture } from './fixture'
import type { FrozenEnv } from './fixture'
import { Ledger, autoMatch, bridgeTransaction, expenseAccountForCategory } from './flowtrace'
import type { BridgeNode, BridgeTransaction, LedgerEnv, LedgerState } from './flowtrace'
import { DEMO, src } from './sources'
import { recordLedgerCalls } from './trace'
import type { RecordedStep } from './trace'

// Loaded at run time: the demo module's import chain needs NODE_PATH.
const demo = await import('../../../flowtrace/src/lib/demo/ledger.ts')
const dataset = await import('../../../flowtrace/src/lib/demo/dataset.ts')

export const DEMO_AS_OF: string = demo.DEMO_AS_OF
export const DEMO_ACTOR = demo.DEMO_ACTOR
export const DEMO_ORG_ID: string = dataset.DEMO_ORG_ID

/** The demo's own env, seedEnv (flowtrace/src/lib/demo/ledger.ts:118-124). */
export const DEMO_ENV: FrozenEnv = {
  now: `${demo.DEMO_AS_OF}T09:00:00.000Z`,
  new_id: 'demo- then n in base 36 left-padded with 0 to 4 characters, n = 1, 2, 3, ... (demo-0001 ... demo-000a ...)',
  defined_at: `${DEMO}:118-124 (seedEnv), injected through LedgerEnv (flowtrace/src/lib/ledger/engine.ts:41-44)`,
}

/** The fields of BridgeTransaction (bridge.ts:15-27); the demo passes whole transaction rows. */
const BRIDGE_FIELDS = [
  'id', 'from_node_id', 'to_node_id', 'amount', 'currency', 'exchange_rate', 'description', 'category', 'reference',
  'verification_status', 'transaction_date',
] as const

export interface DemoRun {
  state: LedgerState
  /** The state right after the bank-main statement import, just before the demo's autoMatch. */
  preAutoMatch: LedgerState
  steps: Array<RecordedStep>
}

export function exportDemo(outDir: string): DemoRun {
  let preAutoMatch: LedgerState | null = null
  const rec = recordLedgerCalls(Ledger, (step, state) => {
    const args = step.args as Array<Json>
    if (step.method === 'importStatement' && args[1] === 'bank-main') preAutoMatch = structuredClone(state)
  })
  let ledger: Ledger
  try {
    ledger = demo.buildDemoLedger()
  } finally {
    rec.stop()
  }
  if (!(ledger instanceof Ledger)) throw new Error('the demo built its books with a different Ledger class')
  const initial = rec.initial()
  if (!initial || !preAutoMatch) throw new Error('the demo backfill was not recorded')
  const state = structuredClone(ledger.state)

  // The demo's autoMatch, re-run on the state it saw, must make exactly the
  // matches the trace recorded through it.
  const viaAuto = rec.steps.filter((s) => s.via === 'autoMatch')
  const replay = autoMatch(new Ledger(structuredClone(preAutoMatch), scratchEnv()), demo.DEMO_ACTOR, 'bank-main')
  const traced = viaAuto.map((s) => `${(s.args as Array<Json>)[1]}>${(s.args as Array<Json>)[2]}`)
  const replayed = replay.matched.map((m) => `${m.statementLineId}>${m.journalLineId}`)
  if (traced.join() !== replayed.join()) throw new Error('autoMatch replay differs from the demo trace')

  const byMethod: Record<string, number> = {}
  const byVia: Record<string, number> = {}
  for (const s of rec.steps) {
    byMethod[s.method] = (byMethod[s.method] ?? 0) + 1
    byVia[s.via] = (byVia[s.via] ?? 0) + 1
  }

  writeFixture(outDir, 'demo-books', {
    description:
      'The demo organisation’s books after buildDemoLedger(): chart of accounts, funds, grants, dimensions, periods, ' +
      'the backfilled entries, budget, bills, claims, bank accounts and statements, audit trail and counters.',
    sources: [`${DEMO}#buildDemoLedger`],
    covers: ['buildDemoLedger'],
    env: DEMO_ENV,
    as_of: DEMO_AS_OF,
    organization_id: DEMO_ORG_ID,
    state,
  })

  writeFixture(outDir, 'demo-backfill', {
    description:
      'Every top-level engine call of buildDemoLedger() that changed the books, in order: the Ledger method, its ' +
      'arguments as passed, its return value at return time, and the state delta. `via` names the module function ' +
      'the demo called the engine through (createOrgLedger, applyCommand, autoMatch) or `direct`. Replaying the ' +
      'steps from `initial_state` gives demo-books.json#/state.',
    sources: [`${DEMO}#buildDemoLedger`, src('setup.ts', 'createOrgLedger'), src('commands.ts', 'applyCommand'), src('reconcile.ts', 'autoMatch')],
    covers: ['buildDemoLedger', ...rec.steps.map((s) => `Ledger.${s.method}`)],
    env: DEMO_ENV,
    delta_format:
      'Per changed collection: added (records, in state order), removed (ids), changed (whole new records), and ' +
      'order (every id) only when the order is not survivors-then-additions; settings when changed; counters ' +
      'with the changed keys.',
    initial_state: initial,
    final_state: 'demo-books.json#/state',
    counts: { steps: rec.steps.length, by_method: byMethod, by_via: byVia },
    steps: rec.steps,
  })

  exportBridge(outDir, state, rec.steps)
  return { state, preAutoMatch, steps: rec.steps }
}

/** An env for engine calls whose ids and times are not written to a fixture. */
function scratchEnv(): LedgerEnv {
  let n = 0
  return { now: () => DEMO_ENV.now, newId: () => `scratch-${++n}` }
}

function exportBridge(outDir: string, state: LedgerState, steps: Array<RecordedStep>) {
  const ctx = demo.demoBridgeContext(new Ledger(structuredClone(state), scratchEnv()))
  const nodes = [...(ctx.nodes as Map<string, BridgeNode>).values()]
  const cashFor: Record<string, string> = {}
  const dimFor: Record<string, string> = {}
  for (const n of nodes) {
    const c = ctx.cashAccountForNode(n.id)
    if (c) cashFor[n.id] = c
    const d = ctx.dimensionForNode(n.id)
    if (d) dimFor[n.id] = d
  }
  // Same order as the backfill (flowtrace/src/lib/demo/ledger.ts:199).
  const txs = [...dataset.transactions].sort((a, b) => a.transaction_date.localeCompare(b.transaction_date))
  const byBytes = [...dataset.transactions].sort((a, b) => (a.transaction_date < b.transaction_date ? -1 : a.transaction_date > b.transaction_date ? 1 : 0))
  if (txs.map((t) => t.id).join() !== byBytes.map((t) => t.id).join())
    throw new Error('localeCompare and byte order disagree on the demo transaction dates')
  const cases = txs.map((t) => {
    const bt = t as unknown as BridgeTransaction
    const transaction: Record<string, unknown> = {}
    for (const k of BRIDGE_FIELDS) transaction[k] = bt[k]
    return {
      transaction,
      grant_for: ctx.grantFor(bt),
      expense_account_for: ctx.expenseAccountFor?.(bt) ?? null,
      result: bridgeTransaction(bt, ctx),
    }
  })

  // Every bridged entry is a post in the backfill trace, input for input.
  const posted = new Set(
    steps.filter((s) => s.method === 'post' && s.via === 'direct').map((s) => JSON.stringify((s.args as Array<Json>)[1])),
  )
  for (const c of cases)
    if (c.result.kind === 'entry' && !posted.has(JSON.stringify(canonical(c.result.input))))
      throw new Error(`bridged transaction ${String(c.transaction.id)} is not a post in the backfill trace`)

  const categories = [null, '', 'salaries', 'Salaries', 'operations', 'supplies', 'transport', 'utilities', 'training',
    'equipment', 'programs', 'funding', 'budget', 'unknown']
  const toNodes: Array<BridgeNode | null> = [
    null,
    { id: 'n-beneficiary', name: 'Beneficiary', type: 'beneficiary' },
    { id: 'n-vendor', name: 'Vendor', type: 'vendor' },
  ]
  const categoryCases = categories.flatMap((category) =>
    toNodes.map((to) => ({ args: [category, to], result: expenseAccountForCategory(category, to ?? undefined) })),
  )

  writeFixture(outDir, 'bridge', {
    description:
      'bridgeTransaction over every demo transaction, in backfill order, with the demo bridge context written out ' +
      'as data (its function results per node and per transaction); and expenseAccountForCategory over every ' +
      'category the bridge maps.',
    sources: [src('bridge.ts', 'bridgeTransaction'), src('bridge.ts', 'expenseAccountForCategory'), `${DEMO}#demoBridgeContext`],
    covers: ['bridgeTransaction', 'expenseAccountForCategory'],
    env: null,
    context: {
      base_currency: ctx.baseCurrency,
      default_cash_account: ctx.defaultCashAccount,
      nodes,
      cash_account_for_node: cashFor,
      dimension_for_node: dimFor,
      note: 'grant_for and expense_account_for are the context functions evaluated per transaction (in each case).',
    },
    cases,
    expense_account_for_category: categoryCases,
  })
}
