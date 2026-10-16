// Recording engine calls: state deltas, a recorder that wraps Ledger.prototype
// to capture every top-level write (used for the demo backfill, whose code
// in flowtrace/src/lib/demo/ledger.ts is run unchanged), and a scenario
// runner for the steps this exporter drives itself.
import { canonical, fingerprint } from './canonical'
import type { Json } from './canonical'
import type { Actor, CommandResult, Ledger, LedgerState } from './flowtrace'
import { LedgerError, applyCommand, sanitizeCommand } from './flowtrace'

// ---------------------------------------------------------------------------
// State deltas
// ---------------------------------------------------------------------------

export interface CollectionDelta {
  added?: Array<Json>
  removed?: Array<string>
  changed?: Array<Json>
  /** Every id in order, present only when the order is not survivors then additions. */
  order?: Array<string>
}

type Delta = Record<string, Json>

const COLLECTIONS = [
  'accounts', 'funds', 'grants', 'dimensions', 'periods', 'entries', 'lines', 'budgets', 'budgetLines',
  'vendors', 'bills', 'payments', 'claims', 'bankAccounts', 'statementLines', 'audit',
] as const

/** What changed between two snapshots of a LedgerState, collection by collection, by id. */
export function diffState(before: LedgerState, after: LedgerState): Delta {
  const delta: Delta = {}
  const known = new Set<string>([...COLLECTIONS, 'settings', 'counters'])
  for (const k of Object.keys(after)) if (!known.has(k)) throw new Error(`diffState: unknown state key ${k}`)
  for (const name of COLLECTIONS) {
    const b = before[name] as Array<{ id: string }>
    const a = after[name] as Array<{ id: string }>
    const ids = (xs: Array<{ id: string }>) => xs.map((x) => x.id)
    if (new Set(ids(a)).size !== a.length || new Set(ids(b)).size !== b.length) {
      if (fingerprint(a) !== fingerprint(b)) delta[name] = { replaced: canonical(a) }
      continue
    }
    const bById = new Map(b.map((x) => [x.id, fingerprint(x)]))
    const aIds = new Set(ids(a))
    const d: CollectionDelta = {}
    const added = a.filter((x) => !bById.has(x.id))
    const changed = a.filter((x) => bById.has(x.id) && bById.get(x.id) !== fingerprint(x))
    const removed = ids(b).filter((id) => !aIds.has(id))
    if (added.length) d.added = added.map((x) => canonical(x))
    if (changed.length) d.changed = changed.map((x) => canonical(x))
    if (removed.length) d.removed = removed
    const expected = [...ids(b).filter((id) => aIds.has(id)), ...added.map((x) => x.id)]
    if (expected.join('\n') !== ids(a).join('\n')) d.order = ids(a)
    if (Object.keys(d).length) delta[name] = d as unknown as Json
  }
  if (fingerprint(before.settings) !== fingerprint(after.settings)) delta.settings = canonical(after.settings)
  const counters: Record<string, number> = {}
  for (const [k, v] of Object.entries(after.counters)) if (before.counters[k] !== v) counters[k] = v
  for (const k of Object.keys(before.counters))
    if (!(k in after.counters)) throw new Error(`diffState: counter ${k} was removed`)
  if (Object.keys(counters).length) delta.counters = counters
  return delta
}

export function errorOf(e: unknown): Json {
  if (e instanceof LedgerError)
    return canonical({ name: 'LedgerError', code: e.code, message: e.message, issues: e.issues })
  if (e instanceof Error) return { name: e.name, message: e.message }
  throw e
}

// ---------------------------------------------------------------------------
// Recorder: wraps Ledger.prototype, records top-level calls that change state
// ---------------------------------------------------------------------------

/** Module functions the demo reaches the engine through; named on each step as `via`. */
const VIA = ['applyCommand', 'autoMatch', 'createOrgLedger'] as const

export interface RecordedStep {
  seq: number
  via: (typeof VIA)[number] | 'direct'
  method: string
  args: Json
  result: Json
  delta: Delta
}

function viaOf(stack: string): RecordedStep['via'] {
  for (const line of stack.split('\n').slice(2)) {
    const m = /^\s*at (\S+) /.exec(line)
    const name = m?.[1].split('.').pop()
    if (name && (VIA as ReadonlyArray<string>).includes(name)) return name as RecordedStep['via']
  }
  return 'direct'
}

export function recordLedgerCalls(
  LedgerClass: { prototype: object },
  onStep?: (step: RecordedStep, state: LedgerState) => void,
) {
  const proto = LedgerClass.prototype as Record<string, unknown>
  const originals = new Map<string, (...a: Array<unknown>) => unknown>()
  const steps: Array<RecordedStep> = []
  let initial: LedgerState | null = null
  let depth = 0
  const limit = Error.stackTraceLimit
  Error.stackTraceLimit = 100

  for (const name of Object.getOwnPropertyNames(proto)) {
    const fn = proto[name]
    if (name === 'constructor' || typeof fn !== 'function') continue
    const original = fn as (...a: Array<unknown>) => unknown
    originals.set(name, original)
    proto[name] = function (this: Ledger, ...args: Array<unknown>) {
      if (depth > 0) {
        depth++
        try {
          return original.apply(this, args)
        } finally {
          depth--
        }
      }
      const before = structuredClone(this.state)
      const fpBefore = fingerprint(this.state)
      const argsSnap = canonical(args)
      const via = viaOf(new Error().stack ?? '')
      depth++
      let result: unknown
      try {
        result = original.apply(this, args)
      } finally {
        depth--
      }
      const resultSnap = result === undefined ? null : canonical(result)
      if (fingerprint(this.state) !== fpBefore) {
        initial ??= before
        const step: RecordedStep = { seq: steps.length + 1, via, method: name, args: argsSnap, result: resultSnap, delta: diffState(before, this.state) }
        steps.push(step)
        onStep?.(step, this.state)
      }
      return result
    }
  }

  return {
    steps,
    initial: () => initial,
    stop() {
      for (const [name, fn] of originals) proto[name] = fn
      Error.stackTraceLimit = limit
    },
  }
}

// ---------------------------------------------------------------------------
// Scenario runner: steps this exporter drives, each recorded with its outcome
// ---------------------------------------------------------------------------

export type ScenarioStep = Record<string, Json>

export class Scenario {
  readonly steps: Array<ScenarioStep> = []
  constructor(readonly ledger: Ledger) {}

  private run(step: ScenarioStep, expect: 'ok' | 'rejected', fn: () => unknown): unknown {
    const before = structuredClone(this.ledger.state)
    const fp = fingerprint(before)
    try {
      const result = fn()
      if (expect === 'rejected') throw new Error(`expected a rejection: ${JSON.stringify(step)}`)
      this.steps.push({
        ...step,
        outcome: 'ok',
        result: result === undefined ? null : canonical(result),
        delta: diffState(before, this.ledger.state),
      })
      return result
    } catch (e) {
      if (expect === 'ok' || !(e instanceof Error) || e.message.startsWith('expected a rejection')) throw e
      const unchanged = fingerprint(this.ledger.state) === fp
      this.steps.push({
        ...step,
        outcome: 'rejected',
        error: errorOf(e),
        state_unchanged: unchanged,
        ...(unchanged ? {} : { delta: diffState(before, this.ledger.state) }),
      })
      return undefined
    }
  }

  private call(expect: 'ok' | 'rejected', method: string, args: Array<unknown>) {
    const fn = (this.ledger as unknown as Record<string, (...a: Array<unknown>) => unknown>)[method]
    if (typeof fn !== 'function') throw new Error(`no Ledger.${method}`)
    return this.run({ op: 'method', method, args: canonical(args) }, expect, () => fn.apply(this.ledger, args))
  }

  /** Calls a Ledger method that must succeed; returns its result. */
  ok<T = unknown>(method: string, ...args: Array<unknown>): T {
    return this.call('ok', method, args) as T
  }

  /** Calls a Ledger method that must be rejected. */
  reject(method: string, ...args: Array<unknown>) {
    this.call('rejected', method, args)
  }

  private command(expect: 'ok' | 'rejected', actor: Actor, raw: unknown) {
    let sanitized: unknown = null
    const step: ScenarioStep = { op: 'command', actor: canonical(actor), command: canonical(raw) }
    return this.run(step, expect, () => {
      sanitized = sanitizeCommand(raw)
      step.sanitized = canonical(sanitized)
      return applyCommand(this.ledger, actor, sanitized as never)
    }) as CommandResult | undefined
  }

  /** Sends a raw command through sanitizeCommand and applyCommand; it must succeed. */
  cmd(actor: Actor, raw: unknown): CommandResult {
    return this.command('ok', actor, raw) as CommandResult
  }

  /** Sends a raw command that must be rejected (by the sanitizer or the engine). */
  rejectCmd(actor: Actor, raw: unknown) {
    this.command('rejected', actor, raw)
  }

  get rejections() {
    return this.steps.filter((s) => s.outcome === 'rejected').length
  }

  /** Ledger methods this scenario called directly. */
  get methods() {
    return [...new Set(this.steps.filter((s) => s.op === 'method').map((s) => String(s.method)))]
  }
}
