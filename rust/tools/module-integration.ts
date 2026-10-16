// The ledger-module end to end on the LOCAL SpacetimeDB server, in one command and under five
// minutes (gate G3's repeatable proof):
//
//   cd rust/tools && ~/.bun/bin/bun run integration [--keep] [--no-build]
//   cd rust/tools && ~/.bun/bin/bun run integration --existing --url <server> --db <name>
//
// Builds the release wasm, publishes it as a fresh database `ft-s2-<timestamp>`, opens the demo
// organisation, loads the demo books with `import_books` and the 37-step command log with
// `run_command`, then checks with SQL: row counts per table against the fixtures, balances and
// the trial balance computed from the `journal_line` rows, four refusals with the TS error text,
// the `my_*` views for a member and for `--anonymous`, and a repeated request id. Each step
// prints one `pass` or `FAIL` line with its time. The database is deleted at the end, also on
// failure, unless `--keep`. Exits 1 on any failure or when the run takes five minutes or more.
//
// `--existing` runs the same checks against a database that is already published (maincloud dev
// for gate G3): no build, publish or delete, a new organisation with a random id each run (every
// count and view check scopes to it), `init` checked as "the owner is an operator", and the
// operator the run added withdrawn at the end. It WRITES a demo organisation into that database
// and leaves it there; the last steps print the run-time measures and the rows left.
//
// Shared with module-replay.ts: src/module-server.ts (CLI, HTTP, SQL, books through the TS
// client codec) and src/demo-replay.ts (demo organisation, import, command log, id map).
// CLI reference: https://spacetimedb.com/docs/cli-reference; HTTP:
// https://spacetimedb.com/docs/http/database. Publish and delete only ever on the local server:
// src/module-server.ts refuses them on an existing database.
import fs from 'node:fs'
import { IdMap, demoFixtures, importDemoBooks, openDemoOrg, readFixture, replayCommandLog, retireOperator } from './src/demo-replay'
import type { DemoPeople } from './src/demo-replay'
import { COLLECTION_TABLES, WASM, args, openServer, parseTarget } from './src/module-server'
import type { Json, Obj, Row } from './src/module-server'
import { RUST_DIR } from './src/paths'

const LIMIT_SECONDS = 300
const { flag } = args()
const KEEP = flag('keep')
const stamp = new Date().toISOString().replace(/\D/g, '').slice(0, 14)
const target = parseTarget(process.argv.slice(2), `ft-s2-${stamp}`)
const server = openServer(target)
const EXISTING = target.existing
const started = performance.now()
const log = (...xs: Array<unknown>) => console.log(...xs)

// ---------------------------------------------------------------------------
// Steps: each timed, one pass or FAIL line
// ---------------------------------------------------------------------------

class Failed extends Error {}
const results: Array<{ name: string; ok: boolean; seconds: number }> = []
const skipped: Array<string> = []

/** A step that does not run in this mode, with why. */
function skip(name: string, why: string) {
  skipped.push(name)
  log(`skip ${'-'.padStart(6)}    ${name}: ${why}`)
}

/** Runs one check. `fn` returns the detail for the pass line or throws; a failed critical step stops the run. */
async function step(name: string, fn: () => Promise<string> | string, critical = false) {
  const t = performance.now()
  let ok = true
  let detail: string
  try {
    detail = await fn()
  } catch (e) {
    ok = false
    detail = e instanceof Error ? e.message : String(e)
  }
  const seconds = (performance.now() - t) / 1000
  results.push({ name, ok, seconds })
  log(`${ok ? 'pass' : 'FAIL'} ${seconds.toFixed(2).padStart(6)} s  ${name}: ${detail}`)
  if (!ok && critical) throw new Failed(name)
}

function expect(ok: boolean, message: string): asserts ok {
  if (!ok) throw new Error(message)
}

const sum = (xs: Array<number>) => xs.reduce((a, b) => a + b, 0)

// ---------------------------------------------------------------------------
// SQL as the database owner (the CLI identity that published), on the private tables
// ---------------------------------------------------------------------------

/**
 * Parses the CLI's SQL table (`col | col` header, a dashed rule, one line per row, strings in
 * double quotes) into rows of trimmed cells.
 */
function cliRows(out: string): Array<Array<string>> {
  const lines = out.split('\n').filter((l) => l.trim() !== '')
  const rule = lines.findIndex((l) => /^-+(\+-+)*$/.test(l.trim()))
  if (rule < 1) throw new Error(`no SQL table in: ${out}`)
  return lines.slice(rule + 1).map((l) => l.split('|').map((c) => c.trim().replace(/^"(.*)"$/, '$1')))
}

const ownerSql = (query: string, anonymous = false) =>
  cliRows(server.cli(['sql', '-s', server.cliServer, ...(anonymous ? ['--anonymous'] : []), server.db, query]))

function ownerCount(table: string, orgId: string): number {
  const [[n]] = ownerSql(`SELECT COUNT(*) AS n FROM ${table} WHERE org_id = '${orgId}'`)
  return Number(n)
}

// ---------------------------------------------------------------------------
// Checks on one state of the books
// ---------------------------------------------------------------------------

/** Every module table against the books' collections; `extra` holds the non-book tables. */
function checkCounts(orgId: string, books: Obj, extra: Record<string, number>): string {
  const want: Array<[string, number]> = [
    ...COLLECTION_TABLES.map(([c, t]): [string, number] => [t, (books[c] as Array<Json>).length]),
    ...Object.entries(extra),
  ]
  const got = want.map(([t]) => ownerCount(t, orgId))
  const wrong = want.filter(([, n], i) => got[i] !== n).map(([t, n], i) => `${t} ${got[i]} want ${n}`)
  expect(wrong.length === 0, wrong.join(', '))
  return `${want.length} tables, ${sum(got)} rows: ${want.map(([t, n]) => `${t} ${n}`).join(', ')}`
}

interface PostedLine {
  entryId: string
  accountId: string
  fundId: string
  base: number
}

/** The posted lines (entry not a draft, reports.ts:59-67), from the member's `my_journal_*` rows. */
async function postedLines(who: DemoPeople['admin'], orgId: string): Promise<Array<PostedLine>> {
  const mine = (rows: Array<Row>) => rows.filter((r) => r.org_id === orgId)
  const status = new Map(mine(await server.sql(who, 'SELECT id, org_id, status FROM my_journal_entry')).map((r) => [r.id as string, r.status as string]))
  const lines = mine(await server.sql(who, 'SELECT org_id, entry_id, account_id, fund_id, base_amount FROM my_journal_line'))
  return lines
    .filter((l) => status.has(l.entry_id as string) && status.get(l.entry_id as string) !== 'draft')
    .map((l) => ({ entryId: l.entry_id as string, accountId: l.account_id as string, fundId: l.fund_id as string, base: Number(l.base_amount) }))
}

/**
 * Debits equal credits in base currency over all posted lines, per fund and per entry (the
 * engine refuses an entry that is UNBALANCED or FUND_UNBALANCED, engine.ts:404-416).
 */
function checkBalances(lines: Array<PostedLine>): string {
  const debit = sum(lines.filter((l) => l.base > 0).map((l) => l.base))
  const credit = -sum(lines.filter((l) => l.base < 0).map((l) => l.base))
  expect(debit === credit, `debits ${debit} credits ${credit}`)
  const by = (key: (l: PostedLine) => string) => {
    const m = new Map<string, number>()
    for (const l of lines) m.set(key(l), (m.get(key(l)) ?? 0) + l.base)
    return m
  }
  const funds = by((l) => l.fundId)
  const entries = by((l) => l.entryId)
  const off = [...funds, ...entries].filter(([, t]) => t !== 0)
  expect(off.length === 0, `not zero: ${off.map(([k, t]) => `${k} ${t}`).join(', ')}`)
  return `${lines.length} posted lines, debits = credits = ${debit} minor units; ${funds.size} funds and ${entries.size} entries each net 0`
}

/**
 * trialBalance(state, undefined, 'accrual') (reports.ts:162-173) computed from the rows: base
 * amounts per account, accounts by code, zero balances dropped; compared with the fixture case
 * row for row (account id through the id map, debit, credit) and on the totals.
 */
async function checkTrialBalance(who: DemoPeople['admin'], orgId: string, lines: Array<PostedLine>, caseIndex: number, map: IdMap): Promise<string> {
  const tb = readFixture('report-trial-balance.json')
  const c = (tb.cases as Array<Obj>)[caseIndex]
  const [asOf, basis] = c.args as [string | null, string]
  expect(asOf === null && basis === 'accrual', `case ${caseIndex} is ${JSON.stringify(c.args)}`)
  const accounts = (await server.sql(who, 'SELECT org_id, data FROM my_account'))
    .filter((r) => r.org_id === orgId)
    .map((r) => JSON.parse(r.data as string) as { id: string; code: string })
    .sort((a, b) => a.code.localeCompare(b.code))
  const bal = new Map<string, number>()
  for (const l of lines) bal.set(l.accountId, (bal.get(l.accountId) ?? 0) + l.base)
  const rows = accounts
    .filter((a) => (bal.get(a.id) ?? 0) !== 0)
    .map((a) => {
      const b = bal.get(a.id) as number
      return { id: a.id, debit: b > 0 ? b : 0, credit: b < 0 ? -b : 0 }
    })
  const debit = sum(rows.map((r) => r.debit))
  const credit = sum(rows.map((r) => r.credit))
  const want = c.result as Obj
  const wantRows = (want.rows as Array<Obj>).map((r) => ({
    id: map.translate((r.account as Obj).id) as string,
    debit: r.debit as number,
    credit: r.credit as number,
  }))
  expect(JSON.stringify(rows) === JSON.stringify(wantRows), `rows differ: got ${JSON.stringify(rows).slice(0, 300)}`)
  expect(debit === want.debit && credit === want.credit && debit - credit === want.difference, `totals ${debit}/${credit}, want ${String(want.debit)}/${String(want.credit)}`)
  return `${rows.length} accounts, debit ${debit} credit ${credit} difference 0, equal to report-trial-balance.json#/cases/${caseIndex} (${String(c.state)}, no as-of date, accrual)`
}

// ---------------------------------------------------------------------------
// After a run on an existing database: tidy the operator table, measure, count what is left
// ---------------------------------------------------------------------------

async function existingDatabaseEpilogue() {
  if (people) {
    const p = people
    await step('withdraw the run operator (remove_operator)', () => {
      retireOperator(server, p)
      const ops = ownerSql('SELECT identity FROM operator').map(([id]) => id.replace(/^0x/, ''))
      expect(!ops.includes(p.operator.identity), 'the operator is still there')
      return `0x${p.operator.identity.slice(0, 12)}... removed by the CLI identity; ${ops.length} operator row(s) remain`
    })
  }
  await step('run_command round trip', () => {
    const ms = server.calls.filter((c) => c.reducer === 'run_command').map((c) => c.ms).sort((a, b) => a - b)
    expect(ms.length > 0, 'no run_command call was made')
    const at = (q: number) => ms[Math.min(ms.length - 1, Math.floor(q * ms.length))].toFixed(0)
    return `${ms.length} HTTP calls: min ${at(0)} ms, median ${at(0.5)} ms, p90 ${at(0.9)} ms, max ${at(1)} ms`
  })
  await step('rows this run left in the database', () => {
    const tables = [...COLLECTION_TABLES.map(([, t]) => t), 'ledger_org', 'member', 'command_result']
    const counts = tables.map((t): [string, number] => [t, ownerCount(t, ORG)])
    return `${sum(counts.map(([, n]) => n))} rows for ${ORG}: ${counts.map(([t, n]) => `${t} ${n}`).join(', ')}`
  })
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

// An existing database may hold earlier runs' books: each run opens its own organisation.
const fx = demoFixtures(EXISTING ? crypto.randomUUID() : undefined)
const ORG = fx.org
const map = new IdMap(fx.frozenNow)
let people: DemoPeople | undefined
let books: Obj = {}
let published = false

log(`module-integration: database ${server.db} on ${server.url}${EXISTING ? ' (existing: no build, publish or delete)' : ''}`)
try {
  if (EXISTING) skip('build the wasm', 'the database is already published')
  else if (!flag('no-build')) {
    await step(
      'build the wasm',
      async () => {
        // At most two cargo builds at once across threads: wait while two others run. A cargo
        // that started this script (the ledger-module guard test) is not another build.
        const ancestors = new Set<number>()
        for (let pid = process.pid; pid > 1; ) {
          ancestors.add(pid)
          const stat = fs.readFileSync(`/proc/${pid}/stat`, 'utf8')
          pid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1])
        }
        const others = () =>
          Bun.spawnSync(['pgrep', '-f', '(^|/)cargo(-clippy)? (build|test|clippy|check|run)'])
            .stdout.toString()
            .split('\n')
            .filter((p) => p !== '' && !ancestors.has(Number(p))).length
        const waitStart = performance.now()
        while (others() >= 2) await Bun.sleep(5000)
        const waited = (performance.now() - waitStart) / 1000
        const r = Bun.spawnSync(['cargo', 'build', '--release', '--locked', '--target', 'wasm32-unknown-unknown', '-p', 'ledger-module'], {
          cwd: RUST_DIR,
          stdout: 'pipe',
          stderr: 'pipe',
        })
        expect(r.exitCode === 0, r.stderr.toString().split('\n').slice(-15).join('\n'))
        return `cargo build --release --target wasm32-unknown-unknown -p ledger-module, ${Bun.file(WASM).size} bytes${waited >= 1 ? `, after ${waited.toFixed(0)} s waiting for other cargo builds` : ''}`
      },
      true,
    )
  }

  if (EXISTING) skip('publish', '--existing never publishes')
  else {
    await step(
      'publish',
      () => {
        const out = server.publish()
        published = true
        return out.split('\n').pop() ?? ''
      },
      true,
    )
  }

  if (EXISTING) {
    await step('init made the publisher an operator', async () => {
      // index.ts:541-543, on a database other runs may have used: the owner (the CLI login, so
      // the CLI's calls are an operator's) must be an operator; any others are listed.
      const { owner_identity } = await server.info()
      const ops = ownerSql('SELECT identity FROM operator').map(([id]) => id.replace(/^0x/, ''))
      expect(ops.includes(owner_identity), `operators ${JSON.stringify(ops)}, owner ${owner_identity}`)
      return `the owner 0x${owner_identity.slice(0, 12)}... is an operator; ${ops.length} operator row(s) before this run`
    })
  } else await step('init made the publisher the only operator', async () => {
    // index.ts:541-543: the publisher becomes the first operator. The publisher is the owner
    // GET /v1/database/:name reports; the private operator table is readable by the owner only.
    const { owner_identity } = await server.info()
    const ops = ownerSql('SELECT identity FROM operator')
    expect(ops.length === 1 && ops[0][0].replace(/^0x/, '') === owner_identity, `operators ${JSON.stringify(ops)}, owner ${owner_identity}`)
    return `operator table holds exactly the owner 0x${owner_identity.slice(0, 12)}...`
  })
  if (EXISTING) log(`organisation for this run: ${ORG}`)

  await step(
    'open the demo organisation',
    async () => {
      people = await openDemoOrg(server, fx, 'Integration Operator')
      const members = ownerCount('member', ORG)
      expect(members === 4, `${members} members`)
      return `create_org_ledger ${ORG} (${String(fx.settings.base_currency)}, 2026) by an HTTP operator the CLI added; add_member x3 (${Object.keys(fx.actors).join(', ')}); 4 members with the owner`
    },
    true,
  )
  const p = people as DemoPeople

  await step(
    'load the demo books (import_books)',
    async () => {
      const r = await importDemoBooks(server, fx, p, map)
      books = r.books
      expect(r.equal, map.mismatches.slice(0, 5).join('; '))
      const n = fx.collections.reduce((k, c) => k + (fx.demoState[c] as Array<Json>).length, 0)
      return `demo-books.json#/state, ${n} records, loaded by import_books (the demo backfill is engine-level, not commands); read back equal`
    },
    true,
  )

  await step('row counts after import', () => checkCounts(ORG, fx.demoState, { ledger_org: 1, member: 4, command_result: 0 }))
  let lines: Array<PostedLine> = []
  await step('balances after import', async () => checkBalances((lines = await postedLines(p.admin, ORG))))
  await step('trial balance after import', () => checkTrialBalance(p.admin, ORG, lines, 0, map))

  await step(
    'replay the command log (run_command)',
    async () => {
      const r = await replayCommandLog(server, fx, p, map, books)
      books = r.books
      const live = r.steps.filter((s) => s.ok && s.equal).length
      expect(live === fx.steps.length && r.finalEqual, `${live}/${fx.steps.length} results equal; ${map.mismatches.slice(0, 5).join('; ')}`)
      return `commands-accepted.json, ${fx.steps.length} commands (${new Set(fx.steps.map((s) => (s.command as Obj).type)).size} types) loaded by run_command; ${live} results equal, final books equal to final_state`
    },
    true,
  )

  const callers = new Set(fx.steps.map((s) => (s.actor as Obj).id as string)).size
  await step('row counts after the command log', () =>
    checkCounts(ORG, fx.commands.final_state as Obj, { ledger_org: 1, member: 4, command_result: callers }),
  )
  await step('balances after the command log', async () => checkBalances((lines = await postedLines(p.admin, ORG))))
  await step('trial balance after the command log', () => checkTrialBalance(p.admin, ORG, lines, 12, map))

  // Refusals: the TS module's SenderError texts.
  const anyCommand = JSON.stringify(map.translate(fx.steps[0].command as Json))
  await step('refusal: non-member run_command', async () => {
    const r = await server.call(p.stranger, 'run_command', [ORG, 'req-stranger', anyCommand])
    const want = `You are not a member of organisation ${ORG}.` // index.ts:509
    expect(!r.ok && r.text === want, r.text)
    return `${r.text} (index.ts:509)`
  })
  await step('refusal: non-operator create_org_ledger', async () => {
    // The demo admin owns books but is no operator.
    const r = await server.call(p.admin, 'create_org_ledger', ['org-s2-refused', 'KES', 2026, 'Not An Operator'])
    expect(!r.ok && r.text === 'Only an operator can open books for a new organisation.', r.text) // index.ts:558
    expect(ownerCount('ledger_org', 'org-s2-refused') === 0, 'books were opened')
    return `${r.text} (index.ts:558)`
  })
  await step('refusal: non-operator import_books', async () => {
    const before = ownerCount('journal_entry', ORG)
    const r = await server.call(p.admin, 'import_books', [ORG, JSON.stringify(fx.demoState)])
    // No TS counterpart (import_books is Rust-only); text from ledger-module src/rules.rs:99-104.
    expect(!r.ok && r.text === 'Only an operator can import books.', r.text)
    expect(ownerCount('journal_entry', ORG) === before, 'rows changed')
    return `${r.text} (rules.rs check_import_books; no TS reducer)`
  })
  await step('refusal: a member removes themselves', async () => {
    const r = await server.call(p.admin, 'remove_member', [ORG, [`0x${p.admin.identity}`]])
    expect(!r.ok && r.text === 'You cannot remove yourself.', r.text) // index.ts:646
    expect(ownerCount('member', ORG) === 4, 'member count changed')
    return `${r.text} (index.ts:646; the caller can close, so the self check is what refuses)`
  })

  await step('views: a member sees rows, --anonymous none', async () => {
    const views = [...COLLECTION_TABLES.map(([, t]) => t), 'ledger_org', 'member', 'command_result']
    const counts: Array<string> = []
    for (const t of views) {
      const member = (await server.sql(p.admin, `SELECT * FROM my_${t}`)).length
      const table = t === 'command_result' ? 1 : ownerCount(t, ORG) // my_command_result: the caller's own row only
      expect(member > 0 && member === table, `my_${t}: member sees ${member}, table holds ${table}`)
      const anon = ownerSql(`SELECT * FROM my_${t}`, true).length
      expect(anon === 0, `my_${t}: --anonymous sees ${anon}`)
      counts.push(`${t} ${member}`)
    }
    return `${views.length} my_* views: member rows equal the org's table rows (${counts.join(', ')}); --anonymous 0 rows each`
  })

  await step('a repeated request id overwrites the result', async () => {
    const i = fx.steps.findIndex((s) => (s.command as Obj).type === 'vendor.add')
    const [first] = await server.sql(p.admin, 'SELECT * FROM my_command_result')
    await server.mustCall(p.admin, 'run_command', [ORG, `req-${i}`, JSON.stringify(map.translate(fx.steps[i].command as Json))])
    const [firstAgain] = await server.sql(p.admin, 'SELECT * FROM my_command_result')
    await server.mustCall(p.admin, 'run_command', [ORG, `req-${i}`, JSON.stringify(map.translate(fx.steps[i].command as Json))])
    const rows = await server.sql(p.admin, 'SELECT * FROM my_command_result')
    const idOf = (r: Row | undefined) => (JSON.parse(r?.result_json as string) as Obj).id
    expect(rows.length === 1 && rows[0].request_id === `req-${i}`, `rows ${JSON.stringify(rows)}`)
    expect(firstAgain.request_id === `req-${i}` && idOf(rows[0]) !== idOf(firstAgain), `result not overwritten: ${String(rows[0].result_json)}`)
    return `req-${i} (vendor.add) sent twice: one my_command_result row, result ${String(firstAgain.result_json)} -> ${String(rows[0].result_json)} (was ${String(first.request_id)})`
  })
} catch (e) {
  if (!(e instanceof Failed)) {
    results.push({ name: 'unexpected error', ok: false, seconds: 0 })
    log(`FAIL unexpected error: ${e instanceof Error ? (e.stack ?? e.message) : String(e)}`)
  }
} finally {
  if (EXISTING) await existingDatabaseEpilogue()
  else if (KEEP) log(`kept database ${server.db}`)
  else if (published) {
    await step('delete the database', () => {
      server.remove()
      const left = server.list()
      expect(!left.includes(server.db), left)
      return `spacetime list -s local: ${left.split('\n').pop()}`
    })
  }
}

const total = (performance.now() - started) / 1000
if (skipped.length > 0) log(`skipped in this mode: ${skipped.join(', ')}`)
results.push({ name: 'under five minutes', ok: total < LIMIT_SECONDS, seconds: total })
log(`${total < LIMIT_SECONDS ? 'pass' : 'FAIL'} ${total.toFixed(2).padStart(6)} s  under five minutes: total wall time ${total.toFixed(1)} s (limit ${LIMIT_SECONDS} s)`)
const failed = results.filter((r) => !r.ok)
log(`module-integration: ${failed.length === 0 ? 'passed' : `FAILED (${failed.map((r) => r.name).join(', ')})`}, ${results.length - failed.length}/${results.length} checks in ${total.toFixed(1)} s`)
process.exit(failed.length === 0 ? 0 : 1)
