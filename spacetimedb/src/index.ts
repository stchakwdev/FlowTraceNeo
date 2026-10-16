// FlowTrace ledger: SpacetimeDB server module.
//
// Every ledger write is one `run_command` reducer call. The reducer loads the
// organisation's rows into a LedgerState, runs the same engine the demo uses
// (flowtrace/src/lib/ledger), and writes back only what changed. A rule
// violation throws, and SpacetimeDB rolls the whole transaction back.
//
// - The actor comes from the `member` table, looked up by ctx.sender. A
//   client never supplies its own permissions.
// - All tables are private. Clients read through per-user `my_*` views that
//   only return rows of organisations the caller is a member of.
// - On top of the engine's own rules, the write-back refuses to change or
//   delete audit events, posted journal lines or posted entries (other than
//   marking them reversed), so a bug in the engine cannot rewrite history.
import { SenderError, schema, t, table } from 'spacetimedb/server'
import type { InferSchema, ReducerCtx } from 'spacetimedb/server'
import type { Identity } from 'spacetimedb'
import { Ledger } from '../../flowtrace/src/lib/ledger/engine'
import { LedgerError } from '../../flowtrace/src/lib/ledger/errors'
import { applyCommand, sanitizeCommand } from '../../flowtrace/src/lib/ledger/commands'
import { createOrgLedger as seedOrgLedger } from '../../flowtrace/src/lib/ledger/setup'
import {
  DOC_COLLECTIONS,
  DOC_COLLECTION_KEYS,
  LEDGER_ERROR_PREFIX,
  assembleState,
  docToRow,
  emptyDocs,
  entryToRow,
  lineToRow,
  rowKey,
} from '../../flowtrace/src/lib/ledger/spacetime-codec'
import type { LedgerEnv } from '../../flowtrace/src/lib/ledger/engine'
import type { LedgerCommand } from '../../flowtrace/src/lib/ledger/commands'
import type { Actor, LedgerState } from '../../flowtrace/src/lib/ledger/types'
import type { DocRow, DocTable, OrgRow, OrgRows } from '../../flowtrace/src/lib/ledger/spacetime-codec'

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

const ledgerOrg = table(
  { name: 'ledger_org' },
  {
    orgId: t.string().primaryKey(),
    settingsJson: t.string(),
    countersJson: t.string(),
    nextSeq: t.u64(),
    createdBy: t.identity(),
    createdAt: t.timestamp(),
  },
)

const member = table(
  { name: 'member' },
  {
    key: t.string().primaryKey(), // `${orgId}:${identity hex}`
    identity: t.identity().index('btree'),
    orgId: t.string().index('btree'),
    userId: t.string(),
    name: t.string(),
    canPost: t.bool(),
    canApprove: t.bool(),
    canClose: t.bool(),
  },
)

const journalEntry = table(
  { name: 'journal_entry' },
  {
    key: t.string().primaryKey(),
    orgId: t.string().index('btree'),
    id: t.string(),
    seq: t.u64(),
    number: t.string(),
    date: t.string(),
    periodId: t.string(),
    memo: t.string(),
    sourceType: t.string(),
    sourceId: t.option(t.string()),
    status: t.string(),
    reversalOf: t.option(t.string()),
    reversedBy: t.option(t.string()),
    overrideReason: t.option(t.string()),
    createdBy: t.string(),
    createdAt: t.string(),
    postedBy: t.option(t.string()),
    postedAt: t.option(t.string()),
  },
)

const journalLine = table(
  { name: 'journal_line' },
  {
    key: t.string().primaryKey(),
    orgId: t.string().index('btree'),
    id: t.string(),
    seq: t.u64(),
    entryId: t.string(),
    accountId: t.string(),
    debit: t.i64(),
    credit: t.i64(),
    currency: t.string(),
    fxRate: t.f64(),
    baseAmount: t.i64(),
    fundId: t.string(),
    grantId: t.option(t.string()),
    dimensionIds: t.array(t.string()),
    flowNodeId: t.option(t.string()),
    memo: t.option(t.string()),
  },
)

/** A collection stored as JSON documents, one row per domain object. */
const docTable = (name: string) =>
  table(
    { name },
    {
      key: t.string().primaryKey(),
      orgId: t.string().index('btree'),
      id: t.string(),
      seq: t.u64(),
      data: t.string(),
    },
  )

const account = docTable('account')
const fund = docTable('fund')
const grant = docTable('grant')
const dimension = docTable('dimension')
const fiscalPeriod = docTable('fiscal_period')
const budget = docTable('budget')
const budgetLine = docTable('budget_line')
const vendor = docTable('vendor')
const bill = docTable('bill')
const payment = docTable('payment')
const expenseClaim = docTable('expense_claim')
const bankAccount = docTable('bank_account')
const statementLine = docTable('statement_line')
const auditEvent = docTable('audit_event')

/** The latest run_command result per caller, read through my_command_result. */
const commandResult = table(
  { name: 'command_result' },
  {
    identity: t.identity().primaryKey(),
    orgId: t.string(),
    requestId: t.string(),
    resultJson: t.string(),
    at: t.timestamp(),
  },
)

/**
 * Identities allowed to open books for a new organisation. The identity that
 * publishes the module becomes the first operator (see `init`), so nobody
 * else can claim an organisation id before its owner does.
 */
const operator = table(
  { name: 'operator' },
  {
    identity: t.identity().primaryKey(),
    addedAt: t.timestamp(),
  },
)

const spacetimedb = schema({
  operator,
  ledgerOrg,
  member,
  journalEntry,
  journalLine,
  account,
  fund,
  grant,
  dimension,
  fiscalPeriod,
  budget,
  budgetLine,
  vendor,
  bill,
  payment,
  expenseClaim,
  bankAccount,
  statementLine,
  auditEvent,
  commandResult,
})
export default spacetimedb

type Ctx = ReducerCtx<InferSchema<typeof spacetimedb>>

// ---------------------------------------------------------------------------
// Read access: per-user views over the caller's organisations
// ---------------------------------------------------------------------------

export const myLedgerOrg = spacetimedb.view(
  { name: 'my_ledger_org', public: true },
  t.array(ledgerOrg.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.ledgerOrg, (m, r) => m.orgId.eq(r.orgId)),
)
export const myMember = spacetimedb.view(
  { name: 'my_member', public: true },
  t.array(member.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.member, (m, r) => m.orgId.eq(r.orgId)),
)
export const myJournalEntry = spacetimedb.view(
  { name: 'my_journal_entry', public: true },
  t.array(journalEntry.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.journalEntry, (m, r) => m.orgId.eq(r.orgId)),
)
export const myJournalLine = spacetimedb.view(
  { name: 'my_journal_line', public: true },
  t.array(journalLine.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.journalLine, (m, r) => m.orgId.eq(r.orgId)),
)
export const myAccount = spacetimedb.view(
  { name: 'my_account', public: true },
  t.array(account.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.account, (m, r) => m.orgId.eq(r.orgId)),
)
export const myFund = spacetimedb.view(
  { name: 'my_fund', public: true },
  t.array(fund.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.fund, (m, r) => m.orgId.eq(r.orgId)),
)
export const myGrant = spacetimedb.view(
  { name: 'my_grant', public: true },
  t.array(grant.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.grant, (m, r) => m.orgId.eq(r.orgId)),
)
export const myDimension = spacetimedb.view(
  { name: 'my_dimension', public: true },
  t.array(dimension.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.dimension, (m, r) => m.orgId.eq(r.orgId)),
)
export const myFiscalPeriod = spacetimedb.view(
  { name: 'my_fiscal_period', public: true },
  t.array(fiscalPeriod.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.fiscalPeriod, (m, r) => m.orgId.eq(r.orgId)),
)
export const myBudget = spacetimedb.view(
  { name: 'my_budget', public: true },
  t.array(budget.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.budget, (m, r) => m.orgId.eq(r.orgId)),
)
export const myBudgetLine = spacetimedb.view(
  { name: 'my_budget_line', public: true },
  t.array(budgetLine.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.budgetLine, (m, r) => m.orgId.eq(r.orgId)),
)
export const myVendor = spacetimedb.view(
  { name: 'my_vendor', public: true },
  t.array(vendor.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.vendor, (m, r) => m.orgId.eq(r.orgId)),
)
export const myBill = spacetimedb.view(
  { name: 'my_bill', public: true },
  t.array(bill.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.bill, (m, r) => m.orgId.eq(r.orgId)),
)
export const myPayment = spacetimedb.view(
  { name: 'my_payment', public: true },
  t.array(payment.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.payment, (m, r) => m.orgId.eq(r.orgId)),
)
export const myExpenseClaim = spacetimedb.view(
  { name: 'my_expense_claim', public: true },
  t.array(expenseClaim.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.expenseClaim, (m, r) => m.orgId.eq(r.orgId)),
)
export const myBankAccount = spacetimedb.view(
  { name: 'my_bank_account', public: true },
  t.array(bankAccount.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.bankAccount, (m, r) => m.orgId.eq(r.orgId)),
)
export const myStatementLine = spacetimedb.view(
  { name: 'my_statement_line', public: true },
  t.array(statementLine.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.statementLine, (m, r) => m.orgId.eq(r.orgId)),
)
export const myAuditEvent = spacetimedb.view(
  { name: 'my_audit_event', public: true },
  t.array(auditEvent.rowType),
  (ctx) =>
    ctx.from.member
      .where((m) => m.identity.eq(ctx.sender))
      .rightSemijoin(ctx.from.auditEvent, (m, r) => m.orgId.eq(r.orgId)),
)
export const myCommandResult = spacetimedb.view(
  { name: 'my_command_result', public: true },
  t.array(commandResult.rowType),
  (ctx) => ctx.from.commandResult.where((r) => r.identity.eq(ctx.sender)),
)

// ---------------------------------------------------------------------------
// Loading and writing back one organisation's books
// ---------------------------------------------------------------------------

/** The common shape of the doc tables, so one loop can serve all fourteen. */
interface DocTableHandle {
  insert: (row: DocRow) => DocRow
  key: { update: (row: DocRow) => DocRow; delete: (key: string) => boolean }
  orgId: { filter: (orgId: string) => Iterable<DocRow> }
}
const docs = (ctx: Ctx, name: DocTable) => ctx.db[name] as unknown as DocTableHandle

const memberKey = (orgId: string, identity: Identity) => rowKey(orgId, identity.toHexString())

/** Deterministic engine environment: reducer time and reducer-seeded UUIDs. */
const envFor = (ctx: Ctx): LedgerEnv => ({
  now: () => ctx.timestamp.toISOString(),
  newId: () => ctx.newUuidV4().toString(),
})

interface Loaded {
  org: OrgRow & { createdBy: Identity; createdAt: Ctx['timestamp'] }
  rows: OrgRows
  /** JSON of every object as loaded, by collection then row key. */
  prior: Record<string, Map<string, { json: string; seq: bigint }>>
  /** Status of every entry as loaded, by entry id. */
  priorStatus: Map<string, string>
}

function loadOrg(ctx: Ctx, orgId: string): { loaded: Loaded; state: LedgerState } {
  const org = ctx.db.ledgerOrg.orgId.find(orgId)
  if (!org) throw new SenderError(`No ledger for organisation ${orgId}.`)
  const rows: OrgRows = {
    org,
    entries: [...ctx.db.journalEntry.orgId.filter(orgId)],
    lines: [...ctx.db.journalLine.orgId.filter(orgId)],
    docs: emptyDocs(),
  }
  for (const k of DOC_COLLECTION_KEYS) rows.docs[k] = [...docs(ctx, DOC_COLLECTIONS[k]).orgId.filter(orgId)]
  const state = assembleState(rows)

  // Snapshot before the engine mutates `state` in place.
  const prior: Loaded['prior'] = {}
  const seqOf = new Map<string, bigint>()
  for (const r of rows.entries) seqOf.set(r.key, r.seq)
  for (const r of rows.lines) seqOf.set(r.key, r.seq)
  for (const k of DOC_COLLECTION_KEYS) for (const r of rows.docs[k]) seqOf.set(`${k}|${r.key}`, r.seq)
  const snap = (name: string, items: Array<{ id: string }>, seqKey: (key: string) => string) => {
    const m = new Map<string, { json: string; seq: bigint }>()
    for (const item of items) {
      const key = rowKey(orgId, item.id)
      m.set(key, { json: JSON.stringify(item), seq: seqOf.get(seqKey(key)) ?? 0n })
    }
    prior[name] = m
  }
  snap('entries', state.entries, (k) => k)
  snap('lines', state.lines, (k) => k)
  for (const k of DOC_COLLECTION_KEYS) snap(k, state[k], (key) => `${k}|${key}`)
  const priorStatus = new Map(state.entries.map((e) => [e.id, e.status as string]))
  return { loaded: { org, rows, prior, priorStatus }, state }
}

/** History is append-only: these checks run whatever the engine did. */
function guardEntry(prevJson: string, next: { status: string } | undefined) {
  const prev = JSON.parse(prevJson) as Record<string, unknown> & { status: string }
  if (prev.status === 'draft') return
  if (!next) throw new Error(`Posted entry ${String(prev.number)} cannot be deleted.`)
  const allowed = new Set(['status', 'reversed_by'])
  const n = next as unknown as Record<string, unknown>
  for (const k of Object.keys(prev)) {
    if (!allowed.has(k) && JSON.stringify(prev[k]) !== JSON.stringify(n[k])) {
      throw new Error(`Posted entry ${String(prev.number)} cannot change (${k}).`)
    }
  }
  if (prev.status !== next.status && !(prev.status === 'posted' && next.status === 'reversed')) {
    throw new Error(`Entry ${String(prev.number)} cannot go from ${prev.status} to ${next.status}.`)
  }
}

function writeBack(ctx: Ctx, orgId: string, loaded: Loaded, state: LedgerState) {
  if (state.settings.organization_id !== orgId) throw new Error('Settings belong to another organisation.')
  let nextSeq = loaded.org.nextSeq

  /** Diffs one collection: insert new ids, update changed ones, delete gone ones. */
  function sync<T extends { id: string }>(
    name: string,
    items: Array<T>,
    ops: {
      insert: (item: T, seq: bigint) => void
      update: (item: T, seq: bigint) => void
      remove: (key: string) => void
      guard?: (key: string, prevJson: string, next: T | undefined) => void
    },
  ) {
    const prior = loaded.prior[name] ?? new Map<string, { json: string; seq: bigint }>()
    const seen = new Set<string>()
    for (const item of items) {
      const key = rowKey(orgId, item.id)
      if (seen.has(key)) throw new Error(`Duplicate id ${item.id} in ${name}.`)
      seen.add(key)
      const prev = prior.get(key)
      if (!prev) {
        ops.insert(item, nextSeq)
        nextSeq += 1n
        continue
      }
      const json = JSON.stringify(item)
      if (json === prev.json) continue
      ops.guard?.(key, prev.json, item)
      ops.update(item, prev.seq)
    }
    for (const [key, prev] of prior) {
      if (seen.has(key)) continue
      ops.guard?.(key, prev.json, undefined)
      ops.remove(key)
    }
  }

  sync('entries', state.entries, {
    insert: (e, seq) => ctx.db.journalEntry.insert(entryToRow(orgId, e, seq)),
    update: (e, seq) => ctx.db.journalEntry.key.update(entryToRow(orgId, e, seq)),
    remove: (key) => ctx.db.journalEntry.key.delete(key),
    guard: (_key, prevJson, next) => guardEntry(prevJson, next),
  })

  sync('lines', state.lines, {
    insert: (l, seq) => ctx.db.journalLine.insert(lineToRow(orgId, l, seq)),
    update: (l, seq) => ctx.db.journalLine.key.update(lineToRow(orgId, l, seq)),
    remove: (key) => ctx.db.journalLine.key.delete(key),
    guard: (_key, prevJson) => {
      const prev = JSON.parse(prevJson) as { entry_id: string; id: string }
      const status = loaded.priorStatus.get(prev.entry_id)
      if (status && status !== 'draft') throw new Error(`Line ${prev.id} of a posted entry cannot change.`)
    },
  })

  for (const k of DOC_COLLECTION_KEYS) {
    const table = docs(ctx, DOC_COLLECTIONS[k])
    const items = state[k] as Array<{ id: string }>
    sync(k, items, {
      insert: (d, seq) => table.insert(docToRow(orgId, d, seq)),
      update: (d, seq) => table.key.update(docToRow(orgId, d, seq)),
      remove: (key) => table.key.delete(key),
      guard:
        k === 'audit'
          ? () => {
              throw new Error('The audit log is append-only.')
            }
          : undefined,
    })
  }

  ctx.db.ledgerOrg.orgId.update({
    ...loaded.org,
    settingsJson: JSON.stringify(state.settings),
    countersJson: JSON.stringify(state.counters),
    nextSeq,
  })
}

function actorOf(ctx: Ctx, orgId: string): Actor {
  const m = ctx.db.member.key.find(memberKey(orgId, ctx.sender))
  if (!m) throw new SenderError(`You are not a member of organisation ${orgId}.`)
  return { id: m.userId, name: m.name, can_post: m.canPost, can_approve: m.canApprove, can_close: m.canClose }
}

/** LedgerError becomes a SenderError the client can turn back into one. */
function rethrow(e: unknown): never {
  if (e instanceof LedgerError || (e instanceof Error && e.name === 'LedgerError')) {
    throw new SenderError(LEDGER_ERROR_PREFIX + JSON.stringify((e as LedgerError).issues))
  }
  throw e
}

function appendAudit(ctx: Ctx, orgId: string, by: string, action: string, subject: string, detail: string) {
  const org = ctx.db.ledgerOrg.orgId.find(orgId)
  if (!org) throw new SenderError(`No ledger for organisation ${orgId}.`)
  const event = { id: ctx.newUuidV4().toString(), at: ctx.timestamp.toISOString(), by, action, subject, detail }
  docs(ctx, 'auditEvent').insert(docToRow(orgId, event, org.nextSeq))
  ctx.db.ledgerOrg.orgId.update({ ...org, nextSeq: org.nextSeq + 1n })
}

// ---------------------------------------------------------------------------
// Reducers
// ---------------------------------------------------------------------------

const ORG_ID = /^[A-Za-z0-9_-]{1,128}$/

/**
 * Opens books for a new organisation (NGO chart of accounts, unrestricted
 * fund, twelve periods) and makes the caller a member with every permission.
 * The caller's user id is their JWT subject when they have one, else their
 * identity; `displayName` is only a label.
 */
export const init = spacetimedb.init((ctx) => {
  ctx.db.operator.insert({ identity: ctx.sender, addedAt: ctx.timestamp })
})

/** Lets an existing operator allow another identity to open books. */
export const addOperator = spacetimedb.reducer(
  { identity: t.identity() },
  (ctx, { identity }) => {
    if (!ctx.db.operator.identity.find(ctx.sender)) throw new SenderError('Only an operator can add operators.')
    if (!ctx.db.operator.identity.find(identity)) ctx.db.operator.insert({ identity, addedAt: ctx.timestamp })
  },
)

export const createOrgLedger = spacetimedb.reducer(
  { orgId: t.string(), baseCurrency: t.string(), fiscalYear: t.u32(), displayName: t.string() },
  (ctx, { orgId, baseCurrency, fiscalYear, displayName }) => {
    if (!ctx.db.operator.identity.find(ctx.sender))
      throw new SenderError('Only an operator can open books for a new organisation.')
    if (!ORG_ID.test(orgId)) throw new SenderError('Organisation id must be 1-128 letters, digits, _ or -.')
    if (!/^[A-Z]{3}$/.test(baseCurrency)) throw new SenderError('Base currency must be an ISO 4217 code, like KES.')
    if (fiscalYear < 2000 || fiscalYear > 2100) throw new SenderError('Fiscal year must be between 2000 and 2100.')
    if (ctx.db.ledgerOrg.orgId.find(orgId)) throw new SenderError(`Organisation ${orgId} already has a ledger.`)

    let ledger: Ledger
    try {
      ledger = seedOrgLedger(orgId, baseCurrency, fiscalYear, envFor(ctx))
    } catch (e) {
      rethrow(e)
    }
    const org = ctx.db.ledgerOrg.insert({
      orgId,
      settingsJson: JSON.stringify(ledger.state.settings),
      countersJson: JSON.stringify(ledger.state.counters),
      nextSeq: 0n,
      createdBy: ctx.sender,
      createdAt: ctx.timestamp,
    })
    const prior: Loaded['prior'] = {}
    writeBack(ctx, orgId, { org, rows: { org, entries: [], lines: [], docs: emptyDocs() }, prior, priorStatus: new Map() }, ledger.state)

    const userId = ctx.senderAuth.jwt?.subject ?? ctx.sender.toHexString()
    const name = displayName.trim() || 'Owner'
    ctx.db.member.insert({
      key: memberKey(orgId, ctx.sender),
      identity: ctx.sender,
      orgId,
      userId,
      name,
      canPost: true,
      canApprove: true,
      canClose: true,
    })
    appendAudit(ctx, orgId, userId, 'ledger.create', orgId, `Opened books in ${baseCurrency} for ${fiscalYear}.`)
  },
)

/** Adds a member or changes their permissions. Needs can_close. */
export const addMember = spacetimedb.reducer(
  {
    orgId: t.string(),
    identity: t.identity(),
    userId: t.string(),
    name: t.string(),
    canPost: t.bool(),
    canApprove: t.bool(),
    canClose: t.bool(),
  },
  (ctx, args) => {
    const actor = actorOf(ctx, args.orgId)
    if (!actor.can_close) throw new SenderError('Only members who can close periods can manage members.')
    if (!args.userId.trim()) throw new SenderError('A member needs a user id.')
    const key = memberKey(args.orgId, args.identity)
    const existing = ctx.db.member.key.find(key)
    // A user id is the person behind approvals and the audit trail, so it
    // is fixed once given and can never be borrowed from another member:
    // otherwise someone could approve their own claim under another name.
    const userId = existing ? existing.userId : args.userId.trim()
    for (const m of ctx.db.member.orgId.filter(args.orgId))
      if (m.userId === userId && !m.identity.isEqual(args.identity))
        throw new SenderError('That user id belongs to another member.')
    const self = args.identity.isEqual(ctx.sender)
    if (self && !args.canClose) throw new SenderError('You cannot remove your own close permission.')
    const row = {
      key,
      identity: args.identity,
      orgId: args.orgId,
      userId,
      name: args.name.trim() || userId,
      canPost: args.canPost,
      canApprove: args.canApprove,
      canClose: args.canClose,
    }
    if (existing) ctx.db.member.key.update(row)
    else ctx.db.member.insert(row)
    const perms = [row.canPost && 'post', row.canApprove && 'approve', row.canClose && 'close'].filter(Boolean).join(', ') || 'read only'
    appendAudit(ctx, args.orgId, actor.id, existing ? 'member.update' : 'member.add', row.userId, `${row.name}: ${perms}`)
  },
)

/** Removes a member. Needs can_close; you cannot remove yourself. */
export const removeMember = spacetimedb.reducer(
  { orgId: t.string(), identity: t.identity() },
  (ctx, { orgId, identity }) => {
    const actor = actorOf(ctx, orgId)
    if (!actor.can_close) throw new SenderError('Only members who can close periods can manage members.')
    if (identity.isEqual(ctx.sender)) throw new SenderError('You cannot remove yourself.')
    const key = memberKey(orgId, identity)
    const existing = ctx.db.member.key.find(key)
    if (!existing) throw new SenderError('No such member.')
    ctx.db.member.key.delete(key)
    appendAudit(ctx, orgId, actor.id, 'member.remove', existing.userId, existing.name)
  },
)

/**
 * The single ledger write path: applies one LedgerCommand (JSON) with the
 * caller's member permissions, then writes back the diff. The result is
 * stored in command_result under `requestId` for the caller to read.
 */
export const runCommand = spacetimedb.reducer(
  { orgId: t.string(), requestId: t.string(), commandJson: t.string() },
  (ctx, { orgId, requestId, commandJson }) => {
    const actor = actorOf(ctx, orgId)
    let cmd: LedgerCommand
    try {
      cmd = JSON.parse(commandJson) as LedgerCommand
    } catch {
      throw new SenderError('The command is not valid JSON.')
    }
    try {
      cmd = sanitizeCommand(cmd)
    } catch (e) {
      rethrow(e)
    }

    const { loaded, state } = loadOrg(ctx, orgId)
    const ledger = new Ledger(state, envFor(ctx))
    let result: ReturnType<typeof applyCommand> | undefined
    try {
      result = applyCommand(ledger, actor, cmd)
    } catch (e) {
      rethrow(e)
    }
    if (!result) throw new SenderError(`Unknown command ${cmd.type}.`)
    writeBack(ctx, orgId, loaded, ledger.state)

    const row = { identity: ctx.sender, orgId, requestId, resultJson: JSON.stringify(result), at: ctx.timestamp }
    if (ctx.db.commandResult.identity.find(ctx.sender)) ctx.db.commandResult.identity.update(row)
    else ctx.db.commandResult.insert(row)
  },
)
