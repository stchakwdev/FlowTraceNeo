// Replays rust/fixtures/commands-accepted.json through the ledger-module
// `run_command` reducer on the LOCAL SpacetimeDB server, and compares every
// command_result and the final books with the fixture.
//
//   cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//   cd tools && ~/.bun/bin/bun run module-replay [--db ft-s4-test] [--keep]
//
// 1. Publishes target/wasm32-unknown-unknown/release/ledger_module.wasm as a
//    fresh local database (any earlier one of that name is deleted first).
// 2. Makes five identities over HTTP (POST /v1/identity): an operator, the
//    fixture's three actors and a stranger. The CLI identity that published
//    the module adds the operator; the operator opens the books
//    (create_org_ledger, checked against setup.json#/createOrgLedger), adds the three actors as members with the fixture's
//    user ids and permissions, and loads demo-books.json#/state (import_books,
//    which also replaces the member.add audit events).
// 3. Sends each step's command as its actor (run_command, request id
//    `req-<step>`), reads the actor's my_command_result row and the books
//    (every my_* view, mapped back to a LedgerState by the TS client codec,
//    flowtrace/src/lib/ledger/spacetime-codec.ts#assembleState).
// 4. The module draws UUIDs and stamps the reducer time where the fixture has
//    fx-0001, ... and a frozen clock (spacetimedb/src/index.ts:361-365). So
//    the fixture ids are mapped to the module's ids as the records appear
//    (each step's new records against the fixture delta's `added`), commands
//    are sent with mapped ids, and the comparison treats ids through that map
//    and a reducer timestamp in place of the frozen one as equal. Everything
//    else must be equal exactly.
// 5. Checks refusals and a repeated request id, deletes the database (unless
//    --keep) and prints `spacetime list -s local`.
//
// Only ever the local server (http://127.0.0.1:3100); the script refuses any
// other URL and any database name that does not start with `ft-`.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { DOC_COLLECTIONS, DOC_COLLECTION_KEYS, assembleState } from '../../flowtrace/src/lib/ledger/spacetime-codec.ts'
import type { DocCollection, DocRow, EntryRow, LineRow, OrgRows } from '../../flowtrace/src/lib/ledger/spacetime-codec.ts'
import { FIXTURES_DIR, RUST_DIR } from './src/paths'

type Json = null | boolean | number | string | Array<Json> | { [k: string]: Json }
type Obj = { [k: string]: Json }

// ---------------------------------------------------------------------------
// Arguments and guards
// ---------------------------------------------------------------------------

const argv = process.argv.slice(2)
const flag = (name: string) => argv.includes(`--${name}`)
const option = (name: string, fallback: string) => {
  const i = argv.indexOf(`--${name}`)
  return i >= 0 && argv[i + 1] ? argv[i + 1] : fallback
}
const DB = option('db', 'ft-s4-test')
const URL_BASE = option('url', 'http://127.0.0.1:3100')
const KEEP = flag('keep')
const SPACETIME = process.env.SPACETIME ?? path.join(os.homedir(), '.local/bin/spacetime')
const WASM = path.join(RUST_DIR, 'target/wasm32-unknown-unknown/release/ledger_module.wasm')

if (!DB.startsWith('ft-')) throw new Error(`Refusing database name ${DB}: use ft-<task>.`)
if (!/^http:\/\/(127\.0\.0\.1|localhost):\d+$/.test(URL_BASE)) throw new Error(`Refusing ${URL_BASE}: local server only.`)
if (!fs.existsSync(WASM)) throw new Error(`No module at ${WASM}; build it first (see the header).`)

const log = (...xs: Array<unknown>) => console.log(...xs)

// ---------------------------------------------------------------------------
// The local server: CLI for publish/delete/add_operator, HTTP for the rest
// ---------------------------------------------------------------------------

function cli(args: Array<string>, mayFail = false): string {
  const r = Bun.spawnSync([SPACETIME, ...args], { stdout: 'pipe', stderr: 'pipe' })
  const out = `${r.stdout.toString()}${r.stderr.toString()}`.replace(/WARNING: This command is UNSTABLE[^\n]*\n\n?/g, '')
  if (r.exitCode !== 0 && !mayFail) throw new Error(`spacetime ${args.join(' ')} failed:\n${out}`)
  return out.trim()
}

interface Who {
  label: string
  identity: string
  token: string
}

async function newIdentity(label: string): Promise<Who> {
  const res = await fetch(`${URL_BASE}/v1/identity`, { method: 'POST' })
  if (!res.ok) throw new Error(`POST /v1/identity: ${res.status}`)
  const body = (await res.json()) as { identity: string; token: string }
  return { label, identity: body.identity, token: body.token }
}

/** A reducer call; `ok` false carries the reducer's error text. */
async function call(who: Who, reducer: string, args: Array<Json>): Promise<{ ok: boolean; text: string }> {
  const res = await fetch(`${URL_BASE}/v1/database/${DB}/call/${reducer}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${who.token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(args),
  })
  return { ok: res.ok, text: await res.text() }
}

async function mustCall(who: Who, reducer: string, args: Array<Json>) {
  const r = await call(who, reducer, args)
  if (!r.ok) throw new Error(`${reducer} as ${who.label} failed: ${r.text}`)
}

interface SqlColumn {
  name: { some: string }
  algebraic_type: { Sum?: { variants: Array<{ name: { some: string } }> } }
}

interface SqlResult {
  schema: { elements: Array<SqlColumn> }
  rows: Array<Array<Json>>
}

/**
 * SQL rows as objects keyed by column. An `Option` column is a sum value, which the HTTP API
 * writes as `[tag, value]` (tag by variant position) or `{ "some": value }`; it becomes the value
 * or undefined, as the TS SDK gives it to the codec.
 */
async function sql(who: Who, query: string): Promise<Array<Record<string, unknown>>> {
  const res = await fetch(`${URL_BASE}/v1/database/${DB}/sql`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${who.token}` },
    body: query,
  })
  if (!res.ok) throw new Error(`${query}: ${res.status} ${await res.text()}`)
  const [result] = (await res.json()) as Array<SqlResult>
  const columns = result.schema.elements.map((e) => {
    const variants = e.algebraic_type.Sum?.variants.map((v) => v.name.some)
    return { name: e.name.some, option: variants?.join() === 'some,none' }
  })
  const unwrap = (v: Json, option: boolean): unknown => {
    if (!option) return v
    if (Array.isArray(v)) return v[0] === 0 ? v[1] : undefined
    if (v && typeof v === 'object') return 'some' in v ? v.some : undefined
    throw new Error(`${query}: option value ${JSON.stringify(v)}`)
  }
  return result.rows.map((row) => Object.fromEntries(columns.map((c, i) => [c.name, unwrap(row[i], c.option)])))
}

// ---------------------------------------------------------------------------
// Rows back to books, through the TS client codec
// ---------------------------------------------------------------------------

const snake = (s: string) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)
const big = (v: unknown) => BigInt(v as number)

async function readBooks(who: Who, orgId: string) {
  const mine = (rows: Array<Record<string, unknown>>) => rows.filter((r) => r.org_id === orgId)
  const [org] = mine(await sql(who, 'SELECT * FROM my_ledger_org'))
  if (!org) throw new Error(`${who.label} sees no ledger_org row for ${orgId}`)
  const entries: Array<EntryRow> = mine(await sql(who, 'SELECT * FROM my_journal_entry')).map((r) => ({
    key: r.key as string,
    orgId: r.org_id as string,
    id: r.id as string,
    seq: big(r.seq),
    number: r.number as string,
    date: r.date as string,
    periodId: r.period_id as string,
    memo: r.memo as string,
    sourceType: r.source_type as string,
    sourceId: r.source_id as string | undefined,
    status: r.status as string,
    reversalOf: r.reversal_of as string | undefined,
    reversedBy: r.reversed_by as string | undefined,
    overrideReason: r.override_reason as string | undefined,
    createdBy: r.created_by as string,
    createdAt: r.created_at as string,
    postedBy: r.posted_by as string | undefined,
    postedAt: r.posted_at as string | undefined,
  }))
  const lines: Array<LineRow> = mine(await sql(who, 'SELECT * FROM my_journal_line')).map((r) => ({
    key: r.key as string,
    orgId: r.org_id as string,
    id: r.id as string,
    seq: big(r.seq),
    entryId: r.entry_id as string,
    accountId: r.account_id as string,
    debit: big(r.debit),
    credit: big(r.credit),
    currency: r.currency as string,
    fxRate: r.fx_rate as number,
    baseAmount: big(r.base_amount),
    fundId: r.fund_id as string,
    grantId: r.grant_id as string | undefined,
    dimensionIds: r.dimension_ids as Array<string>,
    flowNodeId: r.flow_node_id as string | undefined,
    memo: r.memo as string | undefined,
  }))
  const docs = {} as Record<DocCollection, Array<DocRow>>
  for (const k of DOC_COLLECTION_KEYS) {
    docs[k] = mine(await sql(who, `SELECT * FROM my_${snake(DOC_COLLECTIONS[k])}`)).map((r) => ({
      key: r.key as string,
      orgId: r.org_id as string,
      id: r.id as string,
      seq: big(r.seq),
      data: r.data as string,
    }))
  }
  const rows: OrgRows = {
    org: {
      orgId,
      settingsJson: org.settings_json as string,
      countersJson: org.counters_json as string,
      nextSeq: big(org.next_seq),
    },
    entries,
    lines,
    docs,
  }
  // Through JSON, so the books compare as the fixture's plain JSON does.
  return JSON.parse(JSON.stringify(assembleState(rows))) as Obj
}

// ---------------------------------------------------------------------------
// Ids: fixture id -> module id, learned as records appear
// ---------------------------------------------------------------------------

const FX_ID = /^fx-\d{4}$/
const FX_TOKEN = /fx-\d{4}/g
const ISO_MS = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/
const ids = new Map<string, string>()
const taken = new Map<string, string>()
const stats = { idsMapped: 0, timestampsNormalised: 0, stringsWithIds: 0 }
const mismatches: Array<string> = []

/** Walks a fixture record and the module's record in step, learning fixture -> module ids. */
function learn(expected: Json, actual: unknown, where: string) {
  if (typeof expected === 'string' && FX_ID.test(expected) && typeof actual === 'string') {
    const known = ids.get(expected)
    if (known !== undefined && known !== actual) mismatches.push(`${where}: ${expected} is ${known} and ${actual}`)
    const owner = taken.get(actual)
    if (owner !== undefined && owner !== expected) mismatches.push(`${where}: ${actual} is ${owner} and ${expected}`)
    ids.set(expected, actual)
    taken.set(actual, expected)
  } else if (Array.isArray(expected) && Array.isArray(actual)) {
    expected.forEach((e, i) => learn(e, actual[i], `${where}[${i}]`))
  } else if (expected && typeof expected === 'object' && actual && typeof actual === 'object') {
    for (const k of Object.keys(expected)) learn((expected as Obj)[k], (actual as Obj)[k], `${where}.${k}`)
  }
}

/** A fixture value with every fixture id replaced by the module's (unknown ones kept). */
function translate(v: Json): Json {
  if (typeof v === 'string') return v.replace(FX_TOKEN, (t) => ids.get(t) ?? t)
  if (Array.isArray(v)) return v.map(translate)
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, translate(x)]))
  return v
}

/**
 * Compares a fixture value with the module's: ids through the map, the
 * fixture's frozen `now` against any reducer timestamp, all else exactly.
 */
function compare(expected: Json, actual: unknown, where: string, frozenNow: string) {
  if (typeof expected === 'string') {
    if (expected === frozenNow && typeof actual === 'string' && ISO_MS.test(actual)) {
      if (actual !== expected) stats.timestampsNormalised += 1
      return
    }
    const mapped = translate(expected)
    if (mapped !== expected) {
      stats.stringsWithIds += 1
      if (FX_ID.test(expected)) stats.idsMapped += 1
    }
    if (mapped !== actual) mismatches.push(`${where}: want ${JSON.stringify(mapped)} got ${JSON.stringify(actual)}`)
    return
  }
  if (Array.isArray(expected)) {
    if (!Array.isArray(actual) || actual.length !== expected.length) {
      mismatches.push(`${where}: want ${expected.length} items got ${Array.isArray(actual) ? actual.length : typeof actual}`)
      return
    }
    expected.forEach((e, i) => compare(e, actual[i], `${where}[${i}]`, frozenNow))
    return
  }
  if (expected && typeof expected === 'object') {
    if (!actual || typeof actual !== 'object' || Array.isArray(actual)) {
      mismatches.push(`${where}: want an object got ${JSON.stringify(actual)}`)
      return
    }
    const want = Object.keys(expected).sort()
    const got = Object.keys(actual).sort()
    if (want.join() !== got.join()) mismatches.push(`${where}: want keys ${want.join(',')} got ${got.join(',')}`)
    for (const k of want) compare((expected as Obj)[k], (actual as Obj)[k], `${where}.${k}`, frozenNow)
    return
  }
  if (expected !== actual) mismatches.push(`${where}: want ${JSON.stringify(expected)} got ${JSON.stringify(actual)}`)
}

// ---------------------------------------------------------------------------
// The replay
// ---------------------------------------------------------------------------

const fixture = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'commands-accepted.json'), 'utf8')) as Obj
const demo = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'demo-books.json'), 'utf8')) as Obj
const demoState = demo.state as Obj
const settings = demoState.settings as Obj
const ORG = settings.organization_id as string
const FROZEN_NOW = (fixture.env as Obj).now as string
const steps = fixture.steps as Array<Obj>
const actors = fixture.actors as Record<string, Obj>
const started = performance.now()

log(`module-replay: database ${DB} on ${URL_BASE}, module ${path.relative(RUST_DIR, WASM)} (${fs.statSync(WASM).size} bytes)`)
cli(['delete', '-s', 'local', '-y', DB], true)
log(cli(['publish', '-s', 'local', '-b', WASM, '-y', DB]).split('\n').pop())

const operator = await newIdentity('operator')
const stranger = await newIdentity('stranger')
const who: Record<string, Who> = {}
for (const key of Object.keys(actors)) who[key] = await newIdentity(key)
const actorKey = (id: string) => Object.keys(actors).find((k) => actors[k].id === id) as string

cli(['call', '-s', 'local', DB, 'add_operator', JSON.stringify(`0x${operator.identity}`)])
await mustCall(operator, 'create_org_ledger', [ORG, settings.base_currency as string, 2026, 'Replay Operator'])
{
  // The seeded books are TS createOrgLedger's for the same arguments, plus the ledger.create
  // audit event the reducer appends (index.ts:593).
  const setup = JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, 'setup.json'), 'utf8')) as Obj
  const want = setup.createOrgLedger as Obj
  const [org, currency, year] = want.args as [string, string, number]
  const seeded = await readBooks(operator, ORG)
  const audit = seeded.audit as Array<Obj>
  const seedMismatches = mismatches.length
  if (currency !== settings.base_currency || year !== 2026) mismatches.push(`seed: fixture args ${JSON.stringify(want.args)}`)
  const expected = JSON.parse(JSON.stringify(want.state).replaceAll(JSON.stringify(org), JSON.stringify(ORG))) as Obj
  compare({ ...expected, audit: [] }, { ...seeded, audit: [] }, 'create_org_ledger', FROZEN_NOW)
  if (audit.length !== 1 || audit[0].action !== 'ledger.create') mismatches.push(`seed: audit ${JSON.stringify(audit)}`)
  const records = Object.values(seeded).filter(Array.isArray).reduce((n, c) => n + c.length, 0)
  log(
    `create_org_ledger: ${records} records, ${mismatches.length === seedMismatches ? 'equal to' : 'DIFFERENT from'} ` +
      `setup.json#/createOrgLedger (org id ${org} -> ${ORG}) plus ${audit.map((e) => e.action).join(', ')}`,
  )
}
for (const [key, a] of Object.entries(actors)) {
  await mustCall(operator, 'add_member', [
    ORG,
    // An Identity is a one-element product in SATS JSON over HTTP; the CLI takes the bare string.
    [`0x${who[key].identity}`],
    a.id as string,
    a.name as string,
    a.can_post as boolean,
    a.can_approve as boolean,
    a.can_close as boolean,
  ])
}
// After the members: add_member appends member.add audit events, which the import replaces.
await mustCall(operator, 'import_books', [ORG, JSON.stringify(demoState)])
const admin = who[actorKey('user_dummy')]

let books = await readBooks(admin, ORG)
compare(demoState, books, 'import', FROZEN_NOW)
log(`import_books: demo books read back ${mismatches.length === 0 ? 'equal' : 'DIFFERENT'} (${JSON.stringify(demoState).length} bytes of JSON)`)
for (const m of mismatches.slice(0, 10)) log(`mismatch: ${m}`)

const COLLECTIONS = Object.keys(demoState).filter((k) => Array.isArray(demoState[k]))
const outcome = { live: 0, failed: 0, resultsEqual: 0 }

for (const [i, step] of steps.entries()) {
  const cmd = step.command as Obj
  const kind = cmd.type as string
  const key = actorKey((step.actor as Obj).id as string)
  const caller = who[key]
  const requestId = `req-${i}`
  const before = new Map(COLLECTIONS.map((c) => [c, new Set((books[c] as Array<Obj>).map((x) => x.id as string))]))
  const sent = translate(cmd)
  const r = await call(caller, 'run_command', [ORG, requestId, JSON.stringify(sent)])
  const delta = step.delta as Obj

  if (!r.ok) {
    outcome.failed += 1
    mismatches.push(`step ${i} ${kind}: refused: ${r.text}`)
    log(`step ${String(i).padStart(2)} ${kind.padEnd(20)} ${key.padEnd(8)} REFUSED ${r.text}`)
    continue
  }

  books = await readBooks(admin, ORG)
  for (const c of COLLECTIONS) {
    const added = (((delta[c] as Obj | undefined)?.added ?? []) as Array<Json>)
    const fresh = (books[c] as Array<Obj>).filter((x) => !before.get(c)?.has(x.id as string))
    if (fresh.length !== added.length) mismatches.push(`step ${i} ${kind}: ${c} added ${fresh.length}, fixture ${added.length}`)
    added.forEach((e, j) => learn(e, fresh[j], `step ${i} ${c}`))
  }
  const [row] = await sql(caller, 'SELECT * FROM my_command_result')
  const before0 = mismatches.length
  if (row?.request_id !== requestId) mismatches.push(`step ${i}: command_result is ${row?.request_id}, not ${requestId}`)
  else compare(step.result as Json, JSON.parse(row.result_json as string), `step ${i} result`, FROZEN_NOW)
  const equal = mismatches.length === before0
  if (equal) outcome.resultsEqual += 1
  outcome.live += 1
  log(`step ${String(i).padStart(2)} ${kind.padEnd(20)} ${key.padEnd(8)} ${row?.result_json} ${equal ? 'result equal' : 'RESULT DIFFERENT'}`)
}

const before = mismatches.length
compare(fixture.final_state as Json, books, 'final_state', FROZEN_NOW)
const records = COLLECTIONS.reduce((n, c) => n + (books[c] as Array<Obj>).length, 0)
log(
  `final books: ${records} records in ${COLLECTIONS.length} collections, ` +
    `${mismatches.length === before ? 'EQUAL to final_state' : `${mismatches.length - before} DIFFERENCES from final_state`} ` +
    `(${ids.size} fixture ids mapped, ${stats.idsMapped} id fields and ${stats.stringsWithIds - stats.idsMapped} texts compared through the map, ` +
    `${stats.timestampsNormalised} reducer timestamps in place of ${FROZEN_NOW})`,
)
log(`steps: ${outcome.live} run (${outcome.resultsEqual} results equal), ${outcome.failed} refused`)

// ---------------------------------------------------------------------------
// Refusals and a repeated request id
// ---------------------------------------------------------------------------

const count = (b: Obj) => COLLECTIONS.map((c) => (b[c] as Array<Obj>).length).join(',')
const shape = count(books)
const checks: Array<[string, boolean, string]> = []
const officer = who[actorKey('fixture-officer')]
const [lastOfficer] = await sql(officer, 'SELECT * FROM my_command_result')
const post = steps.find((s) => (s.command as Obj).type === 'entry.post') as Obj
const refused = await call(officer, 'run_command', [ORG, 'req-refused', JSON.stringify(translate(post.command as Json))])
const [afterOfficer] = await sql(officer, 'SELECT * FROM my_command_result')
checks.push([
  'a member without can_post is refused with LEDGER_ERROR issues and nothing is written',
  !refused.ok &&
    refused.text.startsWith('LEDGER_ERROR [') &&
    (JSON.parse(refused.text.slice('LEDGER_ERROR '.length)) as Array<Obj>)[0].code === 'PERMISSION' &&
    afterOfficer?.request_id === lastOfficer?.request_id &&
    count(await readBooks(admin, ORG)) === shape,
  refused.text,
])
const notJson = await call(admin, 'run_command', [ORG, 'req-bad', '{nope'])
checks.push(['text that is not JSON is refused', !notJson.ok && notJson.text === 'The command is not valid JSON.', notJson.text])
const outsider = await call(stranger, 'run_command', [ORG, 'req-x', JSON.stringify(translate(post.command as Json))])
checks.push([
  'a non-member is refused',
  !outsider.ok && outsider.text === `You are not a member of organisation ${ORG}.`,
  outsider.text,
])
const notOperator = await call(admin, 'import_books', [ORG, JSON.stringify(demoState)])
checks.push([
  'import_books refuses a non-operator',
  !notOperator.ok && notOperator.text === 'Only an operator can import books.',
  notOperator.text,
])
const vendorStep = steps.findIndex((s) => (s.command as Obj).type === 'vendor.add')
const vendorsBefore = (books.vendors as Array<Obj>).length
const again = await call(admin, 'run_command', [ORG, `req-${vendorStep}`, JSON.stringify(translate(steps[vendorStep].command as Json))])
const [repeat] = await sql(admin, 'SELECT * FROM my_command_result')
const vendorsAfter = ((await readBooks(admin, ORG)).vendors as Array<Obj>).length
checks.push([
  `a repeated request id (req-${vendorStep}, vendor.add) runs again and overwrites the caller's result, as TS`,
  again.ok &&
    repeat?.request_id === `req-${vendorStep}` &&
    (JSON.parse(repeat.result_json as string) as Obj).id !== ids.get((steps[vendorStep].result as Obj).id as string) &&
    vendorsAfter === vendorsBefore + 1,
  `${repeat?.result_json}, vendors ${vendorsBefore} -> ${vendorsAfter}`,
])
for (const [what, ok, detail] of checks) log(`${ok ? 'ok  ' : 'FAIL'} ${what}: ${detail}`)

// ---------------------------------------------------------------------------
// Clean up and verdict
// ---------------------------------------------------------------------------

if (KEEP) log(`kept database ${DB}`)
else {
  log(cli(['delete', '-s', 'local', '-y', DB]))
  log(`spacetime list -s local:\n${cli(['list', '-s', 'local'])}`)
}
const failed = mismatches.length > 0 || checks.some(([, ok]) => !ok)
for (const m of mismatches.slice(0, 40)) log(`mismatch: ${m}`)
if (mismatches.length > 40) log(`... ${mismatches.length - 40} more`)
log(`module-replay: ${failed ? 'FAILED' : 'passed'} in ${((performance.now() - started) / 1000).toFixed(1)} s`)
process.exit(failed ? 1 : 0)
