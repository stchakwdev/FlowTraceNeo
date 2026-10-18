// IT1: the integration test v1. Two users on the web and on the desktop against one fresh
// organisation, and the five reports read three ways (task IT1, phase 2 pre-decision P24):
//
//   cd rust/tools && ~/.bun/bin/bun run it1 --url https://maincloud.spacetimedb.com --db flowtrace-ledger-dev --offline-emulated
//   cd rust/tools && ~/.bun/bin/bun run it1 --local --offline-emulated
//
// In order, each step timed, one pass/FAIL/skip line each, all of it in rust/out/it1/<stamp>/:
//
//   a. open `it1-<stamp>`: the demo books, Alice (post, approve, close), Bob (post, approve) and
//      Carol (none), each a device identity this run mints (dummy auth);
//   b. web as Alice (Playwright): add an account, post a manual entry, submit a claim, enter a
//      bill, import the statement fixture;
//   c. desktop as Bob: the desktop binary opens the books on Bob's token file (`--timings on`),
//      then Bob approves the claim, approves and posts the bill and runs Auto-match through
//      ledger-client (it1_probe), where the window has no scriptable path;
//   d. web as Bob: the claim shows posted; web as Alice: pay the bill;
//   e. refusals: Carol posts an entry on the web and on the desktop (both refused with the
//      module's reason, books unchanged); one request id sent twice;
//   f. the offline claim: `--offline-emulated` (Playwright offline, then online) or `--phone`
//      (the user's Android phone, a manual step); Bob approves it on the desktop;
//   g. the five reports from the web DOM, from the desktop (ledger-client + ledger-core on the
//      module rows) and from the oracle (the run's command log replayed through ledger-core from
//      rust/fixtures/demo-books.json), normalised and required byte-identical;
//   h. the summary table, written to summary.md and printed.
//
// `--auth workos` (src/it1-workos.ts): Alice and Bob are WorkOS users (rust/tools/.env.it1) who
// join by subject; Alice and Bob sign in on AuthKit in the browser, and Bob signs in on the
// desktop through its "Sign in with WorkOS" flow, the system browser being an `xdg-open` wrapper
// that hands the URL to this run. Carol has no WorkOS user, so her web refusal runs on a second
// web app in dummy mode (port 3001) with her device token. The issuer must be trusted first
// (`add_trusted_issuer`, an operator's step: rust/docs/integration-test.md, "WorkOS mode").
//
// Guards (src/it1-flags.ts): only a database whose name ends in `-dev`, or with `--local` an
// ft- database of the run's own on the local server (published here, deleted at the end unless
// `--keep`). A database that is not local is reached through `remoteServer`
// (src/module-server.ts), which never publishes, deletes, renames or lists. Tokens never reach
// the transcript. Docs: rust/docs/integration-test.md.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import type { Browser } from 'playwright-core'
import { parseIt1Flags } from './src/it1-flags'
import type { It1Flags } from './src/it1-flags'
import { actorOf, openIt1Org, runPeople } from './src/it1-org'
import type { BySubject, It1Org } from './src/it1-org'
import { compareThree, fromDom, fromProbe } from './src/it1-report'
import type { DomReport, ProbeReports } from './src/it1-report'
import { addAccount, claimPage, enterBill, fillClaim, importStatement, launchBrowser, mainText, payBill, postEntry, readReport, settled, webPerson } from './src/it1-web'
import type { WebPerson } from './src/it1-web'
import { authkitSignIn, devServerArgs, installXdgOpen, jwtClaims, openedUrls, parseEnvText, passwordToken, redactAuthorizeUrl, secondsLeft, signInOnWeb, workosValues } from './src/it1-workos'
import type { WorkosIt1 } from './src/it1-workos'
import { WASM, localServer, remoteServer } from './src/module-server'
import type { Json, ModuleServer, Row } from './src/module-server'
import { OUT_DIR, REPO_DIR, RUST_DIR, TOOLS_DIR } from './src/paths'

const stamp = new Date().toISOString().replace(/\D/g, '').slice(0, 14)
let flags: It1Flags
try {
  flags = parseIt1Flags(process.argv.slice(2), stamp)
} catch (e) {
  console.error(`it1: ${e instanceof Error ? e.message : String(e)}`)
  process.exit(2)
}
const OUT = path.join(OUT_DIR, 'it1', stamp)
const SHOTS = path.join(OUT, 'shots')
fs.mkdirSync(SHOTS, { recursive: true })
const transcript = fs.openSync(path.join(OUT, 'transcript.log'), 'a')
function log(...xs: Array<unknown>) {
  const line = xs.map(String).join(' ')
  console.log(line)
  fs.writeSync(transcript, `${line}\n`)
}

/** The statement fixture the web imports: synthetic M-Pesa lines (rust/docs/screens). */
const STATEMENT = path.join(RUST_DIR, 'docs/screens/u8-mpesa-statement.csv')
const BANK = 'mpesa-till'
const TODAY = new Date().toISOString().slice(0, 10)
const YEAR = Number(TODAY.slice(0, 4))
/** Reports run year to date as at the run's date (the web screen's `ytd` preset). */
const FROM = `${YEAR}-01-01`
const ORG = `it1-${stamp}`
const PHONE_WAIT_MS = 15 * 60 * 1000

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

class Stop extends Error {}
interface Result {
  id: string
  name: string
  status: 'pass' | 'FAIL' | 'skip'
  ms: number
  detail: string
}
const results: Array<Result> = []

async function step(id: string, name: string, fn: () => Promise<string> | string, critical = true) {
  const t = performance.now()
  let status: Result['status'] = 'pass'
  let detail: string
  try {
    await freshTokens()
    detail = await fn()
  } catch (e) {
    status = 'FAIL'
    detail = e instanceof Error ? e.message : String(e)
  }
  const ms = Math.round(performance.now() - t)
  results.push({ id, name, status, ms, detail })
  log(`${status.padEnd(4)} ${String(ms).padStart(7)} ms  ${id} ${name}: ${detail}`)
  if (status === 'FAIL' && critical) throw new Stop(name)
}

function skip(id: string, name: string, why: string) {
  results.push({ id, name, status: 'skip', ms: 0, detail: why })
  log(`skip ${'-'.padStart(7)}     ${id} ${name}: ${why}`)
}

function expect(ok: boolean, message: string): asserts ok {
  if (!ok) throw new Error(message)
}

// ---------------------------------------------------------------------------
// The command log the oracle replays: every command any client sent, in order
// ---------------------------------------------------------------------------

interface LogStep {
  label: string
  actor: string
  command: Json
  ok: boolean
  result?: Json
  issues?: Json
}
const commandLog: Array<LogStep> = []

// ---------------------------------------------------------------------------
// WorkOS mode: the values, and fresh access tokens for the run's own reads and the probe
// ---------------------------------------------------------------------------

const WORKOS_API = 'https://api.workos.com'
let workos: WorkosIt1 | undefined
let workosApiKey = ''
/** Set by preflight in workos mode: the trusted issuer Alice and Bob join at. */
let bySubject: BySubject | undefined

/**
 * Before every step once the organisation is open: an access token for Alice and Bob with at
 * least 90 s left (AuthKit's last five minutes), checked against the issuer and subject they
 * joined with, in their Who and token file (mode 0600, the probe's `--token-file`).
 */
async function freshTokens() {
  if (!workos || !bySubject || !opened) return
  for (const key of ['alice', 'bob'] as const) {
    const who = org.people[key]
    if (who.token !== '' && secondsLeft(who.token) > 90) continue
    const token = await passwordToken(WORKOS_API, workos.clientId, workosApiKey, workos[key])
    const claims = jwtClaims(token)
    expect(claims.iss === bySubject.issuer && claims.sub === workos[key].id, `${key}'s token has issuer ${String(claims.iss)} and subject ${String(claims.sub)}, not the ones ${key} joined with`)
    who.token = token
    fs.writeFileSync(org.tokenFiles[key], token, { mode: 0o600 })
  }
}

/** Appends what a web person sent since `from`, once each has its result. */
async function logWeb(p: WebPerson, from: number, label: string, refused = false) {
  const sent = await settled(p, from, refused ? 3000 : 15000)
  for (const c of sent) {
    expect(refused || c.result !== undefined, `${c.command.type} sent by ${p.key} had no result`)
    commandLog.push({ label, actor: p.key, command: c.command as Json, ok: c.result !== undefined, result: c.result as Json })
  }
  return sent
}

// ---------------------------------------------------------------------------
// The desktop: the binary, and it1_probe for ledger-client calls
// ---------------------------------------------------------------------------

interface ProbeOutcome {
  identity: string
  ms: number
  outcome: { ok: boolean; result?: Json; error?: string; issues?: Json }
}

function probe(args: Array<string>): unknown {
  const r = Bun.spawnSync([flags.probe, ...args], { stdout: 'pipe', stderr: 'pipe' })
  if (r.exitCode !== 0) throw new Error(`it1_probe ${args[0]}: ${r.stderr.toString().trim()}`)
  return JSON.parse(r.stdout.toString()) as unknown
}

const live = (who: string) => ['--server', flags.url, '--database', flags.db, '--token-file', org.tokenFiles[who as 'bob'], '--org', ORG]

/** One command as `who` through ledger-client; logged for the oracle. */
function desktopSend(who: string, label: string, command: Json): ProbeOutcome {
  const out = probe(['send', ...live(who), '--command', JSON.stringify(command)]) as ProbeOutcome
  commandLog.push({ label, actor: who, command, ok: out.outcome.ok, result: out.outcome.result, issues: out.outcome.issues })
  return out
}

function mustSend(who: string, label: string, command: Json): ProbeOutcome {
  const out = desktopSend(who, label, command)
  expect(out.outcome.ok, `${label}: ${out.outcome.error ?? 'refused'}`)
  return out
}

/**
 * The desktop binary on `who`'s token file, until it shows the books (`--timings on` prints
 * "books visible ... after launch") or 60 s pass; its stderr is kept as `desktop-<who>.log`.
 */
async function launchDesktop(who: string, extra: Array<string>, givenConfig?: string): Promise<string> {
  const dir = path.join(OUT, `desktop-${who}`)
  fs.mkdirSync(dir, { recursive: true })
  const config = givenConfig ?? path.join(dir, 'config.toml')
  if (givenConfig === undefined) fs.writeFileSync(config, `server = ${JSON.stringify(flags.url)}\ndatabase = ${JSON.stringify(flags.db)}\ntoken_file = ${JSON.stringify(org.tokenFiles[who as 'bob'])}\n`)
  const stderrPath = path.join(OUT, `desktop-${who}.log`)
  const env = { ...process.env, XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR ?? `/run/user/${os.userInfo().uid}`, WAYLAND_DISPLAY: process.env.WAYLAND_DISPLAY ?? 'wayland-1' }
  const child = Bun.spawn([flags.desktop, '--config', config, '--timings', 'on', ...extra], { env, stdout: 'ignore', stderr: Bun.file(stderrPath) })
  const end = Date.now() + 60000
  try {
    for (;;) {
      const text = fs.existsSync(stderrPath) ? fs.readFileSync(stderrPath, 'utf8') : ''
      const seen = /books visible (\d+) ms after launch/.exec(text)
      const identity = /status Connected \{ identity: "([0-9a-f]+)"/.exec(text)?.[1]
      if (seen) {
        expect(identity === org.people[who as 'bob'].identity, `the desktop connected as ${String(identity)}, not ${who}`)
        return `books visible ${seen[1]} ms after launch as ${who} (identity 0x${identity.slice(0, 12)}...), ${text.split('\n').filter(Boolean).length} timing lines in desktop-${who}.log`
      }
      if (child.exitCode !== null) throw new Error(`the desktop exited with ${child.exitCode}: ${text.trim().split('\n').slice(-3).join(' / ')}`)
      if (Date.now() > end) throw new Error(`no books after 60 s: ${text.trim().split('\n').slice(-3).join(' / ')}`)
      await Bun.sleep(200)
    }
  } finally {
    child.kill()
    await child.exited
  }
}

/**
 * Workos mode: Bob signs in on the desktop, then the desktop opens the books as him.
 *
 * 1. The desktop starts with `--sign-in on` (the Connection screen's "Sign in with WorkOS",
 *    rust/crates/desktop/src/main.rs) on a fresh device token file, with an `xdg-open` wrapper
 *    first on PATH; the wrapper hands the AuthKit URL to this run, which signs Bob in on it in its
 *    own browser context. AuthKit sends that browser to the desktop's loopback callback; the
 *    desktop trades the code (PKCE) and reconnects as Bob (`status Connected` with Bob's identity).
 * 2. The desktop is closed and started again without the flag: it trades the refresh token it
 *    saved (`<token file>.workos`) before connecting, so it opens the books as Bob from the start.
 */
async function signInDesktop(who: 'bob'): Promise<string> {
  expect(workos !== undefined && browser !== undefined, 'workos mode only')
  const dir = path.join(OUT, `desktop-${who}`)
  fs.mkdirSync(dir, { recursive: true })
  const config = path.join(dir, 'config.toml')
  const deviceFile = path.join(OUT, 'tokens', `${who}-desktop`)
  fs.writeFileSync(config, `server = ${JSON.stringify(flags.url)}\ndatabase = ${JSON.stringify(flags.db)}\ntoken_file = ${JSON.stringify(deviceFile)}\nworkos_client_id = ${JSON.stringify(workos.clientId)}\n`)
  const urls = path.join(dir, 'xdg-open.urls')
  const env = {
    ...process.env,
    PATH: installXdgOpen(path.join(dir, 'bin'), urls),
    XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR ?? `/run/user/${os.userInfo().uid}`,
    WAYLAND_DISPLAY: process.env.WAYLAND_DISPLAY ?? 'wayland-1',
  }
  const expected = org.people[who].identity
  const signInLog = path.join(OUT, `desktop-${who}-signin.log`)
  const t = performance.now()
  const child = Bun.spawn([flags.desktop, '--config', config, '--timings', 'on', '--sign-in', 'on'], { env, stdout: 'ignore', stderr: Bun.file(signInLog) })
  let signedIn = 0
  try {
    const end = Date.now() + 60000
    while (openedUrls(urls).length === 0) {
      if (child.exitCode !== null) throw new Error(`the desktop exited with ${child.exitCode}: ${fs.readFileSync(signInLog, 'utf8').trim().split('\n').slice(-3).join(' / ')}`)
      expect(Date.now() < end, 'the desktop opened no sign-in URL within 60 s')
      await Bun.sleep(200)
    }
    const authorize = openedUrls(urls)[0]
    log(`  the desktop opened ${redactAuthorizeUrl(authorize)}`)
    const context = await browser.newContext()
    try {
      const page = await context.newPage()
      await page.goto(authorize)
      await authkitSignIn(page, workos[who])
      await page.waitForURL(/^http:\/\/127\.0\.0\.1:\d+\/callback/, { timeout: 30000 })
      await page.getByText('Signed in').first().waitFor({ timeout: 15000 })
    } finally {
      await context.close()
    }
    for (;;) {
      const text = fs.existsSync(signInLog) ? fs.readFileSync(signInLog, 'utf8') : ''
      if (text.includes(`status Connected { identity: "${expected}" }`)) break
      const refused = /refused: (.*)/.exec(text)?.[1]
      if (refused) throw new Error(`the desktop's sign-in was refused: ${refused}`)
      if (child.exitCode !== null) throw new Error(`the desktop exited with ${child.exitCode}`)
      expect(Date.now() < end + 60000, `the desktop did not connect as ${who} within 60 s of signing in: ${text.trim().split('\n').slice(-3).join(' / ')}`)
      await Bun.sleep(200)
    }
    signedIn = Math.round(performance.now() - t)
  } finally {
    child.kill()
    await child.exited
  }
  expect(fs.existsSync(`${deviceFile}.workos`), 'the desktop saved no refresh token')
  const books = await launchDesktop(who, ['--report', 'trial-balance'], config)
  return `signed in on AuthKit through the desktop's sign-in (xdg-open wrapper), connected as ${who} (identity 0x${expected.slice(0, 12)}...) ${signedIn} ms after launch; relaunched on the saved refresh token: ${books}`
}

// ---------------------------------------------------------------------------
// Owner reads (the CLI login; private tables)
// ---------------------------------------------------------------------------

function ownerRows(query: string): Array<Array<string>> {
  const out = server.cli(['sql', '-s', server.cliServer, server.db, query])
  const lines = out.split('\n').filter((l) => l.trim() !== '')
  const rule = lines.findIndex((l) => /^-+(\+-+)*$/.test(l.trim()))
  if (rule < 1) throw new Error(`no SQL table in: ${out}`)
  return lines.slice(rule + 1).map((l) => l.split('|').map((c) => c.trim().replace(/^"(.*)"$/, '$1')))
}

const count = (table: string) => Number(ownerRows(`SELECT COUNT(*) AS n FROM ${table} WHERE org_id = '${ORG}'`)[0][0])
const bookCounts = () => ['journal_entry', 'journal_line', 'expense_claim', 'bill'].map((t) => `${t} ${count(t)}`).join(', ')

/** The claims as a member reads them (`my_expense_claim`), parsed. */
async function claims(who: string): Promise<Array<Record<string, Json>>> {
  const rows: Array<Row> = await server.sql(org.people[who as 'alice'], 'SELECT org_id, data FROM my_expense_claim')
  return rows.filter((r) => r.org_id === ORG).map((r) => JSON.parse(r.data as string) as Record<string, Json>)
}

// ---------------------------------------------------------------------------
// The web app: the one at --web-url, or a dev server this run starts on the run's database
// ---------------------------------------------------------------------------

let devServer: ReturnType<typeof Bun.spawn> | undefined
/** Workos mode: Carol's dummy-mode web app. */
let carolServer: ReturnType<typeof Bun.spawn> | undefined
const CAROL_PORT = '3001'
const carolUrl = `http://localhost:${CAROL_PORT}`

async function reachable(url: string) {
  try {
    return (await fetch(url, { signal: AbortSignal.timeout(3000) })).status < 500
  } catch {
    return false
  }
}

async function ensureWeb(): Promise<string> {
  if (await reachable(flags.webUrl)) return `using the web app already at ${flags.webUrl}: it must run with VITE_LEDGER_BACKEND=spacetimedb on ${flags.db} (else the members see no books and step b fails)`
  expect(/^http:\/\/localhost:3000\/?$/.test(flags.webUrl), `nothing answers at ${flags.webUrl}; start the web app there, or leave --web-url out and the run starts one`)
  const ws = flags.url.replace(/^http/, 'ws')
  // The auth mode is set here, over flowtrace/.env.local (Vite keeps variables already set).
  const env = { ...process.env, VITE_LEDGER_BACKEND: 'spacetimedb', VITE_SPACETIMEDB_URI: ws, VITE_SPACETIMEDB_MODULE: flags.db, VITE_SPACETIMEDB_ORG: ORG, VITE_AUTH_MODE: flags.auth }
  const args = devServerArgs(flags.auth, flags.offline === 'phone' ? ['--host'] : [])
  // stderr goes to a file: an unread pipe fills and stops vite (under bun --bun it writes more there).
  devServer = Bun.spawn([process.execPath, ...args], { cwd: path.join(REPO_DIR, 'flowtrace'), env, stdout: Bun.file(path.join(OUT, 'web-dev.log')), stderr: Bun.file(path.join(OUT, 'web-dev.err.log')) })
  await answers(flags.webUrl, 'web-dev.log')
  let note = `started the web app on ${flags.webUrl} (vite dev${flags.auth === 'workos' ? ' under bun --bun, workos auth' : ''}) against ${flags.db} on ${ws}, organisation ${ORG}`
  if (flags.auth === 'workos') {
    // Carol has no WorkOS user: her refusal on the web runs on a dummy-mode app with her device token.
    // The vite binary itself: `bun run vite` waits silently here, and `run dev` already fixes the
    // port. NODE_ENV=test keeps the TanStack devtools event bus off: the first app holds its port
    // 42069 (@tanstack/devtools-event-bus starts only under NODE_ENV=development).
    carolServer = Bun.spawn([path.join(REPO_DIR, 'flowtrace/node_modules/.bin/vite'), 'dev', '--port', CAROL_PORT], { cwd: path.join(REPO_DIR, 'flowtrace'), env: { ...env, VITE_AUTH_MODE: 'dummy', NODE_ENV: 'test' }, stdout: Bun.file(path.join(OUT, 'web-dev-carol.log')), stderr: Bun.file(path.join(OUT, 'web-dev-carol.err.log')) })
    await answers(carolUrl, 'web-dev-carol.log')
    note += `; Carol's dummy-mode app on ${carolUrl}`
  }
  return note
}

async function answers(url: string, logName: string) {
  const end = Date.now() + 90000
  while (!(await reachable(url))) {
    expect(Date.now() < end, `the web dev server did not answer within 90 s; see ${logName}`)
    await Bun.sleep(500)
  }
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

const server: ModuleServer = flags.local ? localServer(flags.db, flags.url) : remoteServer(flags.db, flags.url)
/** Set by step a; every later step runs only after it. */
let org = undefined as unknown as It1Org
let opened = false
let browser: Browser | undefined
let published = false
const web: Partial<Record<'alice' | 'bob' | 'carol', WebPerson>> = {}
const started = performance.now()
let comparison: ReturnType<typeof compareThree> | undefined

log(`it1: ${ORG} on ${flags.db} at ${flags.url}${flags.local ? ' (local: published and deleted by this run)' : ' (existing database: calls and reads only)'}; auth ${flags.auth}; offline claim ${flags.offline}; output ${path.relative(RUST_DIR, OUT)}`)

if (flags.auth === 'workos') {
  const read = (file: string) => (fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '')
  const got = workosValues(read(path.join(TOOLS_DIR, '.env.it1')))
  const web = parseEnvText(read(path.join(REPO_DIR, 'flowtrace/.env.local')))
  const missing = [
    ...(got.ok ? [] : got.missing.map((k) => `rust/tools/.env.it1 ${k}`)),
    ...['WORKOS_CLIENT_ID', 'WORKOS_API_KEY', 'WORKOS_COOKIE_PASSWORD', 'WORKOS_REDIRECT_URI'].filter((k) => (web.get(k) ?? '') === '').map((k) => `flowtrace/.env.local ${k}`),
  ]
  if (!got.ok || missing.length > 0) {
    skip('-', 'WorkOS mode', `skipped: missing ${missing.join(', ')}; nothing was written. See rust/docs/integration-test.md, "WorkOS mode".`)
    process.exit(0)
  }
  if (web.get('WORKOS_CLIENT_ID') !== got.values.clientId) {
    skip('-', 'WorkOS mode', 'skipped: WORKOS_CLIENT_ID differs between rust/tools/.env.it1 and flowtrace/.env.local; nothing was written.')
    process.exit(0)
  }
  expect(/^http:\/\/localhost:3000\/?$/.test(flags.webUrl), 'workos mode needs the web app on http://localhost:3000, the redirect URI registered at WorkOS')
  workos = got.values
  workosApiKey = web.get('WORKOS_API_KEY') ?? ''
}

try {
  await step('0', 'preflight', async () => {
    for (const [what, file] of [
      ['the desktop', flags.desktop],
      ['it1_probe', flags.probe],
    ]) {
      expect(fs.existsSync(file), `${what} is not built at ${file}: cd rust && cargo build --release --locked -p desktop --bin desktop --example it1_probe`)
    }
    if (flags.local) {
      expect(fs.existsSync(WASM), `no module at ${WASM}: cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module`)
      server.publish()
      published = true
    } else {
      // The database must carry the module with AUTH1 (trusted_issuer) and MOD1/MOD2.
      const probeTable = server.cliStatus(['sql', '-s', server.cliServer, server.db, 'SELECT COUNT(*) AS n FROM trusted_issuer'])
      expect(probeTable.ok, `${flags.db} does not carry main's module yet (no trusted_issuer table): publish main to it first. ${probeTable.text.split('\n')[0]}`)
    }
    const webNote = await ensureWeb()
    let trust = ''
    if (workos) {
      // Alice and Bob join at the issuer an operator trusts for this client id. With none, Alice
      // signs in on the web once so the module's refusal names the issuer and audience to register.
      const row = ownerRows('SELECT issuer, audience FROM trusted_issuer').find((r) => r[1] === workos?.clientId)
      if (!row) {
        browser ??= await launchBrowser(flags.headed)
        const page = await (await browser.newContext()).newPage()
        await signInOnWeb(page, `${flags.webUrl.replace(/\/+$/, '')}/ledger/accounts`, workos.alice, page.getByRole('button', { name: 'Add an account' }))
        throw new Error('no trusted_issuer row has this client id as audience, and the web was not refused')
      }
      bySubject = { issuer: row[0], subjects: { alice: workos.alice.id, bob: workos.bob.id } }
      trust = `; trusted issuer ${row[0]} for audience ${row[1]}`
    }
    return `${flags.local ? `published ${flags.db} on the local server; ` : ''}${webNote}${trust}`
  })

  await step('a', 'open the organisation', async () => {
    const memberKey = (userId: string) => {
      expect(/^[A-Za-z0-9_-]+$/.test(userId), `unexpected user id ${userId}`)
      return ownerRows(`SELECT key FROM member WHERE org_id = '${ORG}' AND user_id = '${userId}'`)[0]?.[0] ?? ''
    }
    org = await openIt1Org(server, ORG, OUT, bySubject, memberKey)
    opened = true
    await freshTokens()
    const members = count('member')
    expect(members === 4, `${members} members`)
    if (YEAR > 2026) {
      // The demo books open fiscal year 2026 only; a later run opens its own year first.
      const r = await server.call(org.people.alice, 'run_command', [ORG, `it1-year-${YEAR}`, JSON.stringify({ type: 'year.open', year: YEAR })])
      expect(r.ok, `year.open ${YEAR}: ${r.text}`)
      commandLog.push({ label: `year.open ${YEAR}`, actor: 'alice', command: { type: 'year.open', year: YEAR }, ok: true, result: { type: 'none' } })
    }
    const joined = bySubject ? `Alice and Bob by WorkOS subject at ${bySubject.issuer} (identities 0x${org.people.alice.identity.slice(0, 12)}..., 0x${org.people.bob.identity.slice(0, 12)}...), Carol by device identity` : 'each a device identity'
    return `${ORG}: demo books loaded (import_books), members Alice (post, approve, close), Bob (post, approve), Carol (none) and the run's operator, ${joined}; ${bookCounts()}`
  })

  browser ??= await launchBrowser(flags.headed)
  // Workos mode: Alice and Bob sign in on AuthKit, so their contexts start with no token.
  const tokens: Record<string, string | undefined> = Object.fromEntries(runPeople(bySubject).map((p) => [p.key, bySubject?.subjects[p.key] ? undefined : org.people[p.key].token]))
  web.alice = await webPerson(browser, 'alice', tokens.alice, SHOTS)
  const alice = web.alice
  const base = flags.webUrl.replace(/\/+$/, '')
  let claimId = ''
  let billId = ''

  if (workos) {
    const person = workos.alice
    await step('w1', 'web, Alice: sign in with WorkOS', async () => {
      const ms = await signInOnWeb(alice.page, `${base}/ledger/accounts`, person, alice.page.getByRole('button', { name: 'Add an account' }))
      await alice.shot('signed-in')
      return `AuthKit email and password, back on ${base}, books of ${ORG} loaded as Alice ${ms} ms after opening the page`
    })
  }

  await step('b1', 'web, Alice: add an account', async () => {
    const from = alice.commands.length
    await addAccount(alice, base, { code: '7460', name: 'IT1 internet and data' })
    const [c] = await logWeb(alice, from, 'web: account.add 7460')
    await alice.shot('account-added')
    return `account.add -> ${JSON.stringify(c.result)}`
  })
  await step('b2', 'web, Alice: post a manual entry', async () => {
    const from = alice.commands.length
    await postEntry(alice, base, { memo: `IT1 ${stamp} manual entry`, debit: '7460', credit: '1110', amount: '1500' })
    await alice.page.waitForURL(/\/ledger\/entries\/[^/]+$/, { timeout: 15000 })
    const [c] = await logWeb(alice, from, 'web: entry.post')
    await alice.shot('entry-posted')
    return `entry.post 7460 Dr / 1110 Cr 1,500.00 -> ${JSON.stringify(c.result)}`
  })
  await step('b3', 'web, Alice: submit a claim', async () => {
    const from = alice.commands.length
    await fillClaim(alice, base, { purpose: `IT1 ${stamp} site visit`, description: 'Matatu fare', amount: '3450', account: '5700' })
    await alice.page.waitForURL(/\/spend\/claims\/(?!new)[^/]+$/, { timeout: 15000 })
    const [c] = await logWeb(alice, from, 'web: claim.create')
    claimId = (c.result as { id: string }).id
    await alice.shot('claim-submitted')
    return `claim.create (submit) 5700 3,450.00 -> ${claimId}`
  })
  await step('b4', 'web, Alice: enter a bill', async () => {
    const from = alice.commands.length
    await enterBill(alice, base, { supplier: '', number: `IT1-${stamp}`, description: 'Printing', amount: '18200', account: '7100' })
    const [c] = await logWeb(alice, from, 'web: bill.create')
    billId = (c.result as { id: string }).id
    await alice.shot('bill-entered')
    return `bill.create IT1-${stamp} 7100 18,200.00 -> ${billId}`
  })
  await step('b5', 'web, Alice: import the statement fixture', async () => {
    const from = alice.commands.length
    const label = await importStatement(alice, base, BANK, STATEMENT)
    const [c] = await logWeb(alice, from, 'web: statement.import')
    await alice.shot('statement-imported')
    return `${path.relative(RUST_DIR, STATEMENT)} into ${BANK}: "${label}" -> ${((c.result as { ids: Array<string> }).ids ?? []).length} new lines (one repeats a demo line and is skipped by the engine's dedupe)`
  })

  if (workos) await step('c1', 'desktop, Bob: sign in with WorkOS, then open the books', () => signInDesktop('bob'))
  else await step('c1', 'desktop, Bob: open the books', () => launchDesktop('bob', ['--report', 'trial-balance']))
  await step('c2', "desktop, Bob: approve Alice's claim", () => {
    const r = mustSend('bob', 'desktop: claim.approve', { type: 'claim.approve', claimId })
    return `claim.approve ${claimId} -> ${JSON.stringify(r.outcome.result)} in ${r.ms} ms (ledger-client as 0x${r.identity.slice(0, 12)}...)`
  })
  await step('c3', 'desktop, Bob: approve and post the bill', () => {
    const a = mustSend('bob', 'desktop: bill.approve', { type: 'bill.approve', billId })
    const p = mustSend('bob', 'desktop: bill.post', { type: 'bill.post', billId })
    return `bill.approve in ${a.ms} ms, bill.post -> entry ${JSON.stringify(p.outcome.result)} in ${p.ms} ms`
  })
  await step('c4', 'desktop, Bob: Auto-match on the reconciliation screen', () => {
    const r = mustSend('bob', 'desktop: statement.autoMatch', { type: 'statement.autoMatch', bankAccountId: BANK })
    const m = (r.outcome.result as { result: { matched: Array<Json>; ambiguous: Array<Json>; unmatched: Array<Json> } }).result
    return `statement.autoMatch ${BANK}: ${m.matched.length} matched, ${m.ambiguous.length} ambiguous, ${m.unmatched.length} unmatched, in ${r.ms} ms`
  })

  web.bob = await webPerson(browser, 'bob', tokens.bob, SHOTS)
  const bob = web.bob
  if (workos) {
    const person = workos.bob
    await step('w2', 'web, Bob: sign in with WorkOS', async () => {
      const ms = await signInOnWeb(bob.page, `${base}/spend/claims/${claimId}`, person, bob.page.getByRole('heading', { name: 'What happens next' }))
      return `AuthKit email and password, back on ${base}, the claim page loaded as Bob ${ms} ms after opening the page`
    })
  }
  await step('d1', 'web, Bob: the claim shows posted', async () => {
    const text = await claimPage(bob, base, claimId)
    await bob.shot('claim-posted')
    expect(text.includes('Approved and posted'), `the claim page says: ${text.slice(0, 200)}`)
    return `"Approved and posted" on /spend/claims/${claimId}; Bob sent no command`
  })
  await step('d2', 'web, Alice: pay the bill', async () => {
    const from = alice.commands.length
    await payBill(alice, base, billId, { account: '1110', reference: `IT1-${stamp}` })
    const [c] = await logWeb(alice, from, 'web: bill.pay')
    const text = await mainText(alice.page)
    await alice.shot('bill-paid')
    expect(text.includes('Paid in full'), `the bill page says: ${text.slice(0, 200)}`)
    return `bill.pay ${(c.command as { amount: number }).amount} from 1110 -> ${JSON.stringify(c.result)}; "Paid in full"`
  })

  const reason = 'Carol does not have permission to post to the ledger.'
  web.carol = await webPerson(browser, 'carol', tokens.carol, SHOTS)
  const carol = web.carol
  // Workos mode: Carol has no WorkOS user, so she is refused on the dummy-mode app.
  const carolBase = workos ? carolUrl : base
  await step('e1', 'refusal, web: Carol posts an entry', async () => {
    const before = bookCounts()
    const from = carol.commands.length
    await postEntry(carol, carolBase, { memo: `IT1 ${stamp} Carol tries`, debit: '7100', credit: '1110', amount: '1' })
    await carol.page.getByText('Not saved or posted').waitFor({ timeout: 15000 })
    const sent = await logWeb(carol, from, 'web: entry.post by Carol', true)
    const text = await mainText(carol.page)
    await carol.shot('refused')
    expect(sent.length === 1 && sent[0].result === undefined, `sent ${sent.length} commands, result ${JSON.stringify(sent[0]?.result)}`)
    expect(text.includes(reason), `the page says: ${text.slice(0, 300)}`)
    expect(bookCounts() === before, `books changed: ${before} -> ${bookCounts()}`)
    return `"Not saved or posted. ${reason}"; books unchanged (${before})`
  })
  await step('e2', 'refusal, desktop: Carol posts an entry', () => {
    const before = bookCounts()
    const entry = { type: 'entry.post', input: { date: TODAY, memo: `IT1 ${stamp} Carol tries`, lines: [{ account_id: 'acc-7100', debit: 100, fund_id: 'fund-unrestricted' }, { account_id: 'acc-1110', credit: 100, fund_id: 'fund-unrestricted' }] } }
    const r = desktopSend('carol', 'desktop: entry.post by Carol', entry)
    expect(!r.outcome.ok && r.outcome.error === reason, `outcome ${JSON.stringify(r.outcome)}`)
    expect(bookCounts() === before, `books changed: ${before} -> ${bookCounts()}`)
    return `ledger-client: "${r.outcome.error}" (${JSON.stringify(r.outcome.issues)}), the same reason as the web; books unchanged`
  })
  await step('e3', 'one request id sent twice', async () => {
    // spacetimedb/src/index.ts:658-687 (the TS module, ground truth): run_command applies the
    // command and stores the result under the caller's request id; nothing dedupes a repeated
    // id. So the second call runs the command again and overwrites the one result row.
    const requestId = `it1-repeat-${stamp}`
    const cmd = { type: 'vendor.add', input: { name: `IT1 ${stamp} repeat supplier`, node_id: null, payment_details: 'Paybill 000000' } }
    const vendors = () => count('vendor')
    const before = vendors()
    const results: Array<Json> = []
    for (let i = 0; i < 2; i++) {
      await server.mustCall(org.people.alice, 'run_command', [ORG, requestId, JSON.stringify(cmd)])
      const rows = (await server.sql(org.people.alice, 'SELECT request_id, result_json FROM my_command_result')).filter((r) => r.request_id === requestId)
      expect(rows.length === 1, `${rows.length} result rows for ${requestId}`)
      results.push(JSON.parse(rows[0].result_json as string) as Json)
      commandLog.push({ label: `http: vendor.add, request id ${requestId} (${i + 1})`, actor: 'alice', command: cmd, ok: true, result: results[i] })
    }
    const after = vendors()
    expect(after === before + 2, `vendors ${before} -> ${after}`)
    return `sent twice: one my_command_result row, overwritten (${JSON.stringify(results[0])} -> ${JSON.stringify(results[1])}); vendors ${before} -> ${after}: the command ran twice, as the TS module does (index.ts:658-687), not a no-op`
  })

  let offlineClaim = ''
  if (flags.offline === 'none') skip('f', 'offline claim', 'neither --offline-emulated nor --phone given')
  else if (flags.offline === 'emulated') {
    await step('f1', 'web, Alice offline: submit a claim, then reconnect', async () => {
      const page = alice.page
      await page.goto(`${base}/spend/claims/new`)
      await page.getByRole('textbox', { name: /^Purpose/ }).waitFor({ timeout: 30000 })
      const from = alice.commands.length
      await alice.context.setOffline(true)
      await page.getByRole('textbox', { name: /^Purpose/ }).fill(`IT1 ${stamp} offline claim`)
      await page.getByRole('textbox', { name: /^What it was for/ }).fill('Boda boda')
      await page.getByRole('textbox', { name: /^Amount, KES/ }).fill('800')
      await page.getByRole('combobox', { name: /^Kind of cost/ }).click()
      await page.getByRole('option', { name: /^5700/ }).click()
      await page.getByRole('button', { name: 'Submit for approval' }).click()
      await page.getByText('Saved on this phone').waitFor({ timeout: 15000 })
      await alice.shot('offline-saved')
      expect(alice.commands.length === from, 'a command left while offline')
      const offlineAt = performance.now()
      await alice.context.setOffline(false)
      // The queue is flushed on the browser's `online` event (useOfflineClaimSync).
      const end = Date.now() + 60000
      while (alice.commands.length === from) {
        expect(Date.now() < end, 'the queued claim was not sent within 60 s of going online')
        await Bun.sleep(100)
      }
      const [c] = await logWeb(alice, from, 'web: claim.create offline')
      const back = performance.now() - offlineAt
      await alice.shot('offline-sent')
      offlineClaim = (c.result as { id: string }).id
      expect((c.command as { input: { captured_offline: boolean } }).input.captured_offline, 'not marked captured_offline')
      return `saved on the device while offline (no frame sent), sent ${Math.round(back)} ms after going online: claim.create captured_offline -> ${offlineClaim}`
    })
  } else {
    await step('f1', 'phone, Alice: an offline claim from the Android phone', async () => {
      const lan = Bun.spawnSync(['ip', 'route', 'get', '1.1.1.1']).stdout.toString().match(/src (\S+)/)?.[1] ?? '<this machine>'
      const port = new URL(flags.webUrl).port || '3000'
      log(`\n  PHONE STEP (manual). On the Android phone, on the same network as this machine:\n  1. Open http://${lan}:${port}/spend/claims/new in Chrome. The Books screens say "The books are not connected yet" and show this phone's identity (0x and 64 hex digits).\n  2. Type that identity below and press Enter; the run adds it as "Alice (phone)".\n  3. Once the claim form shows, turn on aeroplane mode.\n  4. Submit a claim: purpose "IT1 ${stamp} phone claim", one line, any amount, kind of cost 5700 Field travel. The page says "Saved on this phone".\n  5. Turn aeroplane mode off and keep the page open. The run waits up to 15 minutes for the claim.\n`)
      const typed = (prompt('  Phone identity:') ?? '').trim().replace(/^0x/, '')
      expect(/^[0-9a-f]{64}$/.test(typed), `not an identity: ${typed}`)
      await server.mustCall(org.operator, 'add_member', [ORG, [`0x${typed}`], 'it1-alice-phone', 'Alice (phone)', true, false, false])
      const end = Date.now() + PHONE_WAIT_MS
      for (;;) {
        const c = (await claims('alice')).find((x) => x.claimant === 'it1-alice-phone' && x.captured_offline === true)
        if (c) {
          offlineClaim = c.id as string
          // The phone is no Playwright page, so its command is rebuilt from the claim it wrote
          // (claim.create with submit, flowtrace/src/routes/_authenticated/spend/claims.new.tsx).
          const lines = (c.lines as Array<Record<string, Json>>).map((l) => ({ description: l.description, account_id: l.account_id, amount: l.amount, fund_id: l.fund_id, grant_id: l.grant_id, dimension_ids: l.dimension_ids }))
          const input = { id: c.id, claim_date: c.claim_date, currency: c.currency, fx_rate: c.fx_rate, purpose: c.purpose, receipt_names: c.receipt_names, captured_offline: true, lines, submit: true }
          commandLog.push({ label: 'phone: claim.create offline', actor: 'phone', command: { type: 'claim.create', input }, ok: true, result: { type: 'id', id: offlineClaim } })
          return `claim ${String(c.number)} from the phone, captured offline, ${offlineClaim}`
        }
        expect(Date.now() < end, 'no claim from the phone within 15 minutes')
        await Bun.sleep(3000)
      }
    })
  }
  if (offlineClaim !== '') {
    await step('f2', 'desktop, Bob: approve the offline claim', () => {
      const r = mustSend('bob', 'desktop: claim.approve offline', { type: 'claim.approve', claimId: offlineClaim })
      return `claim.approve ${offlineClaim} -> ${JSON.stringify(r.outcome.result)} in ${r.ms} ms`
    })
  }

  // g. The five reports, three ways.
  const domReports: Record<string, DomReport> = {}
  let desktopReports: ProbeReports = {}
  let oracleReports: ProbeReports = {}
  await step('g1', 'reports, web: read the five statements off the DOM', async () => {
    for (const id of ['position', 'activities', 'functional', 'cash-flow', 'trial-balance']) {
      domReports[id] = await readReport(alice, base, id, FROM, TODAY)
      await alice.shot(`report-${id}`)
    }
    fs.writeFileSync(path.join(OUT, 'reports-web-dom.json'), JSON.stringify(domReports, null, 1))
    return `${Object.keys(domReports).length} reports, ${FROM} to ${TODAY}, accrual, as Alice: ${Object.entries(domReports).map(([k, r]) => `${k} ${r.tables.reduce((n, t) => n + t.rows.length, 0)} rows`).join(', ')}`
  })
  await step('g2', 'reports, desktop: ledger-client + ledger-core on the module rows', () => {
    desktopReports = probe(['reports', ...live('bob'), '--from', FROM, '--to', TODAY]) as ProbeReports
    fs.writeFileSync(path.join(OUT, 'reports-desktop.json'), JSON.stringify(desktopReports, null, 1))
    return `desktop::statements::statement on the books ledger-client rebuilt as Bob: ${Object.keys(desktopReports).join(', ')}`
  })
  await step('g3', 'reports, oracle: the command log replayed through ledger-core', () => {
    const actors = Object.fromEntries([...runPeople(bySubject).map((p) => [p.key, actorOf(p)]), ['phone', { id: 'it1-alice-phone', name: 'Alice (phone)', can_post: true, can_approve: false, can_close: false }]])
    const logFile = path.join(OUT, 'command-log.json')
    fs.writeFileSync(logFile, JSON.stringify({ org: ORG, now: new Date().toISOString(), actors, steps: commandLog }, null, 1))
    const out = probe(['oracle', '--log', logFile, '--from', FROM, '--to', TODAY]) as { steps: Array<{ step: number; label: string; agrees: boolean; detail: string }>; reports: ProbeReports }
    oracleReports = out.reports
    fs.writeFileSync(path.join(OUT, 'reports-oracle.json'), JSON.stringify(oracleReports, null, 1))
    const off = out.steps.filter((s) => !s.agrees)
    expect(off.length === 0, `the oracle disagrees with the module on ${off.map((s) => `step ${s.step} (${s.label}): ${s.detail}`).join('; ')}`)
    return `${out.steps.length} commands from rust/fixtures/demo-books.json through ledger-core (ledger_module::command::apply): every outcome as the module's (${out.steps.filter((s) => s.detail.startsWith('refused')).length} refused)`
  })
  await step('g4', 'reports: web, desktop and oracle byte-identical', () => {
    comparison = compareThree({ web: fromDom(domReports), desktop: fromProbe(desktopReports), oracle: fromProbe(oracleReports) })
    for (const [k, t] of Object.entries(comparison.texts)) fs.writeFileSync(path.join(OUT, `normalised-${k}.txt`), t)
    if (!comparison.equal) {
      fs.writeFileSync(path.join(OUT, 'report-diff.txt'), comparison.diffs.join('\n\n'))
      throw new Error(`differences (report-diff.txt):\n${comparison.diffs.join('\n\n').slice(0, 4000)}`)
    }
    const lines = comparison.texts.web.split('\n').length - 1
    return `normalised-{web,desktop,oracle}.txt identical: ${lines} lines, ${Buffer.byteLength(comparison.texts.web)} bytes, sha256 ${new Bun.CryptoHasher('sha256').update(comparison.texts.web).digest('hex').slice(0, 16)}`
  })
} catch (e) {
  if (!(e instanceof Stop)) {
    results.push({ id: '!', name: 'unexpected error', status: 'FAIL', ms: 0, detail: e instanceof Error ? (e.stack ?? e.message) : String(e) })
    log(`FAIL unexpected error: ${e instanceof Error ? (e.stack ?? e.message) : String(e)}`)
  }
} finally {
  for (const p of Object.values(web)) {
    if (p?.errors.length) log(`browser console errors (${p.key}): ${p.errors.slice(0, 5).join(' | ')}`)
  }
  await browser?.close()
  for (const s of [devServer, carolServer]) {
    if (s) {
      s.kill()
      await s.exited
    }
  }
  // The run's operator is withdrawn; the organisation stays (rust/docs/integration-test.md).
  if (opened) {
    await step('h0', 'withdraw the run operator', () => {
      server.cli(['call', '-s', server.cliServer, server.db, 'remove_operator', JSON.stringify(`0x${org.operator.identity}`)])
      return `0x${org.operator.identity.slice(0, 12)}... removed; ${ORG} stays with its books and members`
    }, false).catch(() => undefined)
  }
  if (published && !flags.keep) {
    await step('h1', 'delete the local database', () => {
      server.remove()
      return `deleted ${flags.db}`
    }, false).catch(() => undefined)
  }
}

// h. The summary.
const total = Math.round(performance.now() - started)
const failed = results.filter((r) => r.status === 'FAIL')
const table = [
  `# IT1 run ${stamp}`,
  '',
  `Organisation \`${ORG}\` on \`${flags.db}\` (${flags.url}), auth ${flags.auth}, offline claim ${flags.offline}. Reports ${FROM} to ${TODAY}, accrual.`,
  '',
  '| Step | What | Result | ms |',
  '|---|---|---|---:|',
  ...results.map((r) => `| ${r.id} | ${r.name} | ${r.status} | ${r.ms} |`),
  `| | total | ${failed.length === 0 ? 'pass' : 'FAIL'} | ${total} |`,
  '',
  `Three-way report comparison: ${comparison ? (comparison.equal ? 'identical' : 'DIFFERENT, see report-diff.txt') : 'not reached'}.`,
  '',
  ...results.map((r) => `- **${r.id} ${r.name}** (${r.status}): ${r.detail.replace(/\n/g, ' ')}`),
  '',
].join('\n')
fs.writeFileSync(path.join(OUT, 'summary.md'), table)
log(`\n${table}`)
log(`it1: ${failed.length === 0 ? 'passed' : `FAILED (${failed.map((r) => r.id).join(', ')})`} in ${(total / 1000).toFixed(1)} s; ${path.relative(RUST_DIR, OUT)}`)
process.exit(failed.length === 0 ? 0 : 1)
