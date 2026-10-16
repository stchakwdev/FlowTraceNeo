// LedgerState <-> SpacetimeDB rows. Pure functions, shared by the server
// module (../../../../spacetimedb/src/index.ts imports this file) and the
// client backend (./spacetime-backend.ts), so both agree on one row shape.
//
// Journal entries and lines get typed columns (amounts as i64) so they can be
// queried with SQL. Every other collection is a "doc" row: the domain object
// as JSON, keyed by `${orgId}:${id}`. Ids are only unique per organisation
// (the chart of accounts template reuses ids), hence the composite key.
//
// Rows carry a `seq` so the state's array order survives the round trip.
import type {
  EntryStatus,
  JournalEntry,
  JournalLine,
  LedgerState,
  OrgLedgerSettings,
  SourceType,
} from './types'

/** LedgerState collections stored as JSON docs, and their table accessor. */
export const DOC_COLLECTIONS = {
  accounts: 'account',
  funds: 'fund',
  grants: 'grant',
  dimensions: 'dimension',
  periods: 'fiscalPeriod',
  budgets: 'budget',
  budgetLines: 'budgetLine',
  vendors: 'vendor',
  bills: 'bill',
  payments: 'payment',
  claims: 'expenseClaim',
  bankAccounts: 'bankAccount',
  statementLines: 'statementLine',
  audit: 'auditEvent',
} as const

export type DocCollection = keyof typeof DOC_COLLECTIONS
export type DocTable = (typeof DOC_COLLECTIONS)[DocCollection]
export const DOC_COLLECTION_KEYS = Object.keys(DOC_COLLECTIONS) as Array<DocCollection>

export interface OrgRow {
  orgId: string
  settingsJson: string
  countersJson: string
  nextSeq: bigint
}

export interface DocRow {
  key: string
  orgId: string
  id: string
  seq: bigint
  data: string
}

export interface EntryRow {
  key: string
  orgId: string
  id: string
  seq: bigint
  number: string
  date: string
  periodId: string
  memo: string
  sourceType: string
  sourceId: string | undefined
  status: string
  reversalOf: string | undefined
  reversedBy: string | undefined
  overrideReason: string | undefined
  createdBy: string
  createdAt: string
  postedBy: string | undefined
  postedAt: string | undefined
}

export interface LineRow {
  key: string
  orgId: string
  id: string
  seq: bigint
  entryId: string
  accountId: string
  debit: bigint
  credit: bigint
  currency: string
  fxRate: number
  baseAmount: bigint
  fundId: string
  grantId: string | undefined
  dimensionIds: Array<string>
  flowNodeId: string | undefined
  memo: string | undefined
}

export interface OrgRows {
  org: OrgRow
  entries: Array<EntryRow>
  lines: Array<LineRow>
  docs: Record<DocCollection, Array<DocRow>>
}

export const rowKey = (orgId: string, id: string) => `${orgId}:${id}`

const opt = (v: string | null): string | undefined => (v === null ? undefined : v)
const nul = (v: string | undefined | null): string | null => (v === undefined ? null : v)

/** i64 from a safe-integer minor amount. Throws on floats, never rounds. */
export function toI64(n: number): bigint {
  if (!Number.isSafeInteger(n)) throw new Error(`Amount ${n} is not a whole number of minor units.`)
  return BigInt(n)
}

export function fromI64(n: bigint): number {
  const v = Number(n)
  if (!Number.isSafeInteger(v)) throw new Error(`Amount ${n} is outside the safe integer range.`)
  return v
}

export function entryToRow(orgId: string, e: JournalEntry, seq: bigint): EntryRow {
  return {
    key: rowKey(orgId, e.id),
    orgId,
    id: e.id,
    seq,
    number: e.number,
    date: e.date,
    periodId: e.period_id,
    memo: e.memo,
    sourceType: e.source_type,
    sourceId: opt(e.source_id),
    status: e.status,
    reversalOf: opt(e.reversal_of),
    reversedBy: opt(e.reversed_by),
    overrideReason: opt(e.override_reason),
    createdBy: e.created_by,
    createdAt: e.created_at,
    postedBy: opt(e.posted_by),
    postedAt: opt(e.posted_at),
  }
}

export function rowToEntry(r: EntryRow): JournalEntry {
  return {
    id: r.id,
    number: r.number,
    date: r.date,
    period_id: r.periodId,
    memo: r.memo,
    source_type: r.sourceType as SourceType,
    source_id: nul(r.sourceId),
    status: r.status as EntryStatus,
    reversal_of: nul(r.reversalOf),
    reversed_by: nul(r.reversedBy),
    override_reason: nul(r.overrideReason),
    created_by: r.createdBy,
    created_at: r.createdAt,
    posted_by: nul(r.postedBy),
    posted_at: nul(r.postedAt),
  }
}

export function lineToRow(orgId: string, l: JournalLine, seq: bigint): LineRow {
  return {
    key: rowKey(orgId, l.id),
    orgId,
    id: l.id,
    seq,
    entryId: l.entry_id,
    accountId: l.account_id,
    debit: toI64(l.debit),
    credit: toI64(l.credit),
    currency: l.currency,
    fxRate: l.fx_rate,
    baseAmount: toI64(l.base_amount),
    fundId: l.fund_id,
    grantId: opt(l.grant_id),
    dimensionIds: [...l.dimension_ids],
    flowNodeId: opt(l.flow_node_id),
    memo: opt(l.memo),
  }
}

export function rowToLine(r: LineRow): JournalLine {
  return {
    id: r.id,
    entry_id: r.entryId,
    account_id: r.accountId,
    debit: fromI64(r.debit),
    credit: fromI64(r.credit),
    currency: r.currency,
    fx_rate: r.fxRate,
    base_amount: fromI64(r.baseAmount),
    fund_id: r.fundId,
    grant_id: nul(r.grantId),
    dimension_ids: [...r.dimensionIds],
    flow_node_id: nul(r.flowNodeId),
    memo: nul(r.memo),
  }
}

export function docToRow(orgId: string, doc: { id: string }, seq: bigint): DocRow {
  return { key: rowKey(orgId, doc.id), orgId, id: doc.id, seq, data: JSON.stringify(doc) }
}

const bySeq = <T extends { seq: bigint }>(rows: Iterable<T>) =>
  [...rows].sort((a, b) => (a.seq < b.seq ? -1 : a.seq > b.seq ? 1 : 0))

export function emptyDocs(): Record<DocCollection, Array<DocRow>> {
  const docs = {} as Record<DocCollection, Array<DocRow>>
  for (const k of DOC_COLLECTION_KEYS) docs[k] = []
  return docs
}

/** Rebuilds the engine's state from one organisation's rows. */
export function assembleState(rows: OrgRows): LedgerState {
  const settings = JSON.parse(rows.org.settingsJson) as OrgLedgerSettings
  const state: LedgerState = {
    settings,
    accounts: [],
    funds: [],
    grants: [],
    dimensions: [],
    periods: [],
    entries: bySeq(rows.entries).map(rowToEntry),
    lines: bySeq(rows.lines).map(rowToLine),
    budgets: [],
    budgetLines: [],
    vendors: [],
    bills: [],
    payments: [],
    claims: [],
    bankAccounts: [],
    statementLines: [],
    audit: [],
    counters: JSON.parse(rows.org.countersJson) as Record<string, number>,
  }
  for (const k of DOC_COLLECTION_KEYS) {
    ;(state[k] as Array<unknown>) = bySeq(rows.docs[k]).map((r) => JSON.parse(r.data) as unknown)
  }
  return state
}

/** The wire format of a rejected command: `LEDGER_ERROR ` + JSON issues. */
export const LEDGER_ERROR_PREFIX = 'LEDGER_ERROR '
