// Replays rust/fixtures/commands-accepted.json through the ledger-module
// `run_command` reducer on the LOCAL SpacetimeDB server, and compares every
// command_result and the final books with the fixture.
//
//   cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//   cd tools && ~/.bun/bin/bun run module-replay [--db ft-s4-test] [--keep]
//   cd tools && ~/.bun/bin/bun run module-replay --existing --url <server> --db <name>
//
// 1. Publishes target/wasm32-unknown-unknown/release/ledger_module.wasm as a
//    fresh local database (any earlier one of that name is deleted first).
// 2. Opens the demo organisation (src/demo-replay.ts#openDemoOrg), checks the
//    seeded books against setup.json#/createOrgLedger, and loads
//    demo-books.json#/state (import_books).
// 3. Sends each step's command as its actor (run_command, request id
//    `req-<step>`), reads the actor's my_command_result row and the books
//    (every my_* view, mapped back to a LedgerState by the TS client codec,
//    flowtrace/src/lib/ledger/spacetime-codec.ts#assembleState), comparing
//    ids through a fixture-to-module map (src/demo-replay.ts#IdMap).
// 4. Checks refusals and a repeated request id, deletes the database (unless
//    --keep) and prints `spacetime list -s local`.
//
// Without --existing only ever the local server (http://127.0.0.1:3100);
// src/module-server.ts refuses any other URL and any database name that does
// not start with `ft-`. With --existing (maincloud dev for gate G3) steps 1 and
// 4's delete are skipped, the run opens a new organisation with a random id,
// and withdraws the operator it added; it WRITES that organisation into the
// database and leaves it there.
import fs from 'node:fs'
import path from 'node:path'
import { IdMap, demoFixtures, importDemoBooks, openDemoOrg, readFixture, replayCommandLog, retireOperator } from './src/demo-replay'
import { WASM, openServer, parseTarget } from './src/module-server'
import type { Json, Obj } from './src/module-server'
import { RUST_DIR } from './src/paths'

const argv = process.argv.slice(2)
const target = parseTarget(argv, 'ft-s4-test')
const server = openServer(target)
const EXISTING = target.existing
const KEEP = argv.includes('--keep')
const log = (...xs: Array<unknown>) => console.log(...xs)

if (!EXISTING && !fs.existsSync(WASM)) throw new Error(`No module at ${WASM}; build it first (see the header).`)

const fx = demoFixtures(EXISTING ? crypto.randomUUID() : undefined)
const { org: ORG, steps } = fx
const map = new IdMap(fx.frozenNow)
const started = performance.now()

if (EXISTING) log(`module-replay: existing database ${server.db} on ${server.url} (no publish or delete)`)
else {
  log(`module-replay: database ${server.db} on ${server.url}, module ${path.relative(RUST_DIR, WASM)} (${fs.statSync(WASM).size} bytes)`)
  log(server.publish().split('\n').pop())
}

const people = await openDemoOrg(server, fx, 'Replay Operator')
const { operator, stranger, who, actorKey, admin } = people
if (EXISTING) log(`organisation for this run: ${ORG}`)
{
  // The seeded books are TS createOrgLedger's for the same arguments, plus the ledger.create
  // audit event the reducer appends (index.ts:593). Read before import_books replaces them; the
  // member.add audit events add_member appended are left out of the comparison.
  const want = readFixture('setup.json').createOrgLedger as Obj
  const [org, currency, year] = want.args as [string, string, number]
  const seeded = await server.readBooks(operator, ORG)
  const audit = (seeded.audit as Array<Obj>).filter((e) => e.action !== 'member.add')
  const seedMismatches = map.mismatches.length
  if (currency !== fx.settings.base_currency || year !== 2026) map.mismatches.push(`seed: fixture args ${JSON.stringify(want.args)}`)
  const expected = JSON.parse(JSON.stringify(want.state).replaceAll(JSON.stringify(org), JSON.stringify(ORG))) as Obj
  map.compare({ ...expected, audit: [] }, { ...seeded, audit: [] }, 'create_org_ledger')
  if (audit.length !== 1 || audit[0].action !== 'ledger.create') map.mismatches.push(`seed: audit ${JSON.stringify(audit)}`)
  const records = Object.values(seeded).filter(Array.isArray).reduce((n, c) => n + c.length, 0)
  log(
    `create_org_ledger: ${records} records, ${map.mismatches.length === seedMismatches ? 'equal to' : 'DIFFERENT from'} ` +
      `setup.json#/createOrgLedger (org id ${org} -> ${ORG}) plus ${audit.map((e) => e.action).join(', ')}`,
  )
}

const imported = await importDemoBooks(server, fx, people, map)
log(`import_books: demo books read back ${imported.equal ? 'equal' : 'DIFFERENT'} (${JSON.stringify(fx.demoState).length} bytes of JSON)`)
for (const m of map.mismatches.slice(0, 10)) log(`mismatch: ${m}`)

const replay = await replayCommandLog(server, fx, people, map, imported.books)
for (const s of replay.steps) {
  const verdict = !s.ok ? 'REFUSED' : s.equal ? 'result equal' : 'RESULT DIFFERENT'
  log(`step ${String(s.index).padStart(2)} ${s.kind.padEnd(20)} ${s.actor.padEnd(8)} ${s.detail} ${verdict}`)
}
let books = replay.books
const COLLECTIONS = fx.collections
const records = COLLECTIONS.reduce((n, c) => n + (books[c] as Array<Obj>).length, 0)
log(
  `final books: ${records} records in ${COLLECTIONS.length} collections, ` +
    `${replay.finalEqual ? 'EQUAL to final_state' : `${replay.finalDifferences} DIFFERENCES from final_state`} ` +
    `(${map.ids.size} fixture ids mapped, ${map.stats.idsMapped} id fields and ${map.stats.stringsWithIds - map.stats.idsMapped} texts compared through the map, ` +
    `${map.stats.timestampsNormalised} reducer timestamps in place of ${fx.frozenNow})`,
)
const live = replay.steps.filter((s) => s.ok)
log(`steps: ${live.length} run (${live.filter((s) => s.equal).length} results equal), ${replay.steps.length - live.length} refused`)

// ---------------------------------------------------------------------------
// Refusals and a repeated request id
// ---------------------------------------------------------------------------

const count = (b: Obj) => COLLECTIONS.map((c) => (b[c] as Array<Obj>).length).join(',')
const shape = count(books)
const checks: Array<[string, boolean, string]> = []
const officer = who[actorKey('fixture-officer')]
const [lastOfficer] = await server.sql(officer, 'SELECT * FROM my_command_result')
const post = steps.find((s) => (s.command as Obj).type === 'entry.post') as Obj
const refused = await server.call(officer, 'run_command', [ORG, 'req-refused', JSON.stringify(map.translate(post.command as Json))])
const [afterOfficer] = await server.sql(officer, 'SELECT * FROM my_command_result')
checks.push([
  'a member without can_post is refused with LEDGER_ERROR issues and nothing is written',
  !refused.ok &&
    refused.text.startsWith('LEDGER_ERROR [') &&
    (JSON.parse(refused.text.slice('LEDGER_ERROR '.length)) as Array<Obj>)[0].code === 'PERMISSION' &&
    afterOfficer?.request_id === lastOfficer?.request_id &&
    count(await server.readBooks(admin, ORG)) === shape,
  refused.text,
])
const notJson = await server.call(admin, 'run_command', [ORG, 'req-bad', '{nope'])
checks.push(['text that is not JSON is refused', !notJson.ok && notJson.text === 'The command is not valid JSON.', notJson.text])
const outsider = await server.call(stranger, 'run_command', [ORG, 'req-x', JSON.stringify(map.translate(post.command as Json))])
checks.push([
  'a non-member is refused',
  !outsider.ok && outsider.text === `You are not a member of organisation ${ORG}.`,
  outsider.text,
])
const notOperator = await server.call(admin, 'import_books', [ORG, JSON.stringify(fx.demoState)])
checks.push([
  'import_books refuses a non-operator',
  !notOperator.ok && notOperator.text === 'Only an operator can import books.',
  notOperator.text,
])
const vendorStep = steps.findIndex((s) => (s.command as Obj).type === 'vendor.add')
const vendorsBefore = (books.vendors as Array<Obj>).length
const again = await server.call(admin, 'run_command', [ORG, `req-${vendorStep}`, JSON.stringify(map.translate(steps[vendorStep].command as Json))])
const [repeat] = await server.sql(admin, 'SELECT * FROM my_command_result')
books = await server.readBooks(admin, ORG)
const vendorsAfter = (books.vendors as Array<Obj>).length
checks.push([
  `a repeated request id (req-${vendorStep}, vendor.add) runs again and overwrites the caller's result, as TS`,
  again.ok &&
    repeat?.request_id === `req-${vendorStep}` &&
    (JSON.parse(repeat.result_json as string) as Obj).id !== map.ids.get((steps[vendorStep].result as Obj).id as string) &&
    vendorsAfter === vendorsBefore + 1,
  `${String(repeat?.result_json)}, vendors ${vendorsBefore} -> ${vendorsAfter}`,
])
for (const [what, ok, detail] of checks) log(`${ok ? 'ok  ' : 'FAIL'} ${what}: ${detail}`)

// ---------------------------------------------------------------------------
// Clean up and verdict
// ---------------------------------------------------------------------------

if (EXISTING) {
  retireOperator(server, people)
  const ms = server.calls.filter((c) => c.reducer === 'run_command').map((c) => c.ms).sort((a, b) => a - b)
  log(`withdrew operator 0x${operator.identity.slice(0, 12)}...; run_command round trip over ${ms.length} calls: min ${ms[0].toFixed(0)} ms, median ${ms[Math.floor(ms.length / 2)].toFixed(0)} ms, max ${ms[ms.length - 1].toFixed(0)} ms`)
  log(`left organisation ${ORG} in ${server.db}`)
} else if (KEEP) log(`kept database ${server.db}`)
else {
  log(server.remove())
  log(`spacetime list -s local:\n${server.list()}`)
}
const { mismatches } = map
const failed = mismatches.length > 0 || checks.some(([, ok]) => !ok)
for (const m of mismatches.slice(0, 40)) log(`mismatch: ${m}`)
if (mismatches.length > 40) log(`... ${mismatches.length - 40} more`)
log(`module-replay: ${failed ? 'FAILED' : 'passed'} in ${((performance.now() - started) / 1000).toFixed(1)} s`)
process.exit(failed ? 1 : 0)
