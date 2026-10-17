// The AUTH2 local proof: the desktop signs in through the system browser against a stand-in for
// WorkOS AuthKit (src/authkit-mock.ts) whose tokens come from a test OpenID Connect issuer
// (src/oidc-issuer.ts), on the LOCAL SpacetimeDB server.
//
//   cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//   cd rust/tools && ~/.bun/bin/bun run signin-proof [--minutes 30] [--keep]
//
// It publishes a fresh `ft-auth2-<timestamp>`, opens books for `ft-auth2-org`, trusts the
// `trusted` issuer for the proof's client id, and adds Alice by issuer and subject
// (add_member_by_subject) before she ever connects. It writes two desktop configs under
// rust/out/auth2/ (gitignored): `trusted/config.toml` and `untrusted/config.toml`, whose issuer
// the database never trusts. Then it serves the issuers and the stand-ins until the minutes are
// up or Ctrl-C, and prints the commands to run by hand:
//
//   cargo run -p desktop -- --config out/auth2/trusted/config.toml --screen connect --sign-in on
//
// opens the browser on the stand-in's page; Continue sends it back to the app, which connects as
// Alice and lists ft-auth2-org. Running the same command without `--sign-in on` reconnects from
// the saved refresh token with no browser. The untrusted config shows the refusal message.
// Every request to the stand-ins (no codes or tokens) goes to rust/out/auth2/authkit-log.txt.
// The database is deleted at the end unless `--keep`. Only ever the local server.
import fs from 'node:fs'
import path from 'node:path'
import { startAuthKit } from './src/authkit-mock'
import { startIssuers } from './src/oidc-issuer'
import { args, localServer } from './src/module-server'
import type { Json } from './src/module-server'
import { RUST_DIR } from './src/paths'

const CLIENT = 'client_auth2_local_proof'
const ORG = 'ft-auth2-org'
const ALICE = { id: 'user_alice', email: 'alice@example.org', first_name: 'Alice', last_name: 'Test' }
const stamp = new Date().toISOString().replace(/\D/g, '').slice(0, 14)
const server = localServer(`ft-auth2-${stamp}`)
const KEEP = args().flag('keep')
const MINUTES = Number(args().option('minutes', '30'))
const OUT = path.join(RUST_DIR, 'out', 'auth2')

const operatorCall = (reducer: string, ...argv: Array<Json>) =>
  server.cli(['call', '-s', server.cliServer, server.db, reducer, ...argv.map((a) => JSON.stringify(a))])

const issuers = await startIssuers(['trusted', 'untrusted'])
const kits = {
  // Two-minute access tokens, so a refresh is due 96 s after signing in (signin.rs, refresh_delay).
  trusted: startAuthKit(issuers.get('trusted'), { clientId: CLIENT, user: ALICE, organizationId: 'org_auth2', ttlSeconds: 120 }),
  untrusted: startAuthKit(issuers.get('untrusted'), { clientId: CLIENT, user: ALICE, ttlSeconds: 120 }),
}
fs.mkdirSync(OUT, { recursive: true })
const logPath = path.join(OUT, 'authkit-log.txt')
const writeLog = () => {
  const lines = (['trusted', 'untrusted'] as const).flatMap((name) => kits[name].log.map((l) => `${name}: ${l}`))
  fs.writeFileSync(logPath, `${lines.join('\n')}\n`)
}

let finished = false
const finish = (code: number) => {
  if (finished) return
  finished = true
  writeLog()
  if (!KEEP) {
    try {
      server.remove()
      console.log(`deleted ${server.db}`)
    } catch (e) {
      console.log(`FAIL  delete ${server.db}: ${e instanceof Error ? e.message : String(e)}`)
    }
  } else console.log(`kept ${server.db} (--keep)`)
  kits.trusted.stop()
  kits.untrusted.stop()
  issuers.stop()
  process.exit(code)
}
process.on('SIGINT', () => finish(0))
process.on('SIGTERM', () => finish(0))

try {
  console.log(server.publish().split('\n').at(-1))
  operatorCall('create_org_ledger', ORG, 'KES', 2026, 'AUTH2 proof operator')
  operatorCall('add_trusted_issuer', issuers.get('trusted').iss, CLIENT, 'AUTH2 local proof')
  operatorCall('add_member_by_subject', ORG, issuers.get('trusted').iss, ALICE.id, 'Alice Test', true, true, false)
  for (const name of ['trusted', 'untrusted'] as const) {
    const dir = path.join(OUT, name)
    fs.mkdirSync(dir, { recursive: true })
    for (const f of fs.readdirSync(dir)) if (f.startsWith('token-')) fs.rmSync(path.join(dir, f))
    fs.writeFileSync(
      path.join(dir, 'config.toml'),
      [
        `server = "${server.url}"`,
        `database = "${server.db}"`,
        `workos_client_id = "${CLIENT}"`,
        `workos_issuer = "${issuers.get(name).iss}"`,
        `workos_api = "${kits[name].api}"`,
        '',
      ].join('\n'),
    )
  }
} catch (e) {
  console.log(`FAIL  setup: ${e instanceof Error ? e.message : String(e)}`)
  finish(1)
}

console.log(`database ${server.db} on ${server.url}; books ${ORG}; Alice added by subject ${ALICE.id}`)
console.log(`trusted issuer   ${issuers.get('trusted').iss} (audience ${CLIENT}), AuthKit stand-in ${kits.trusted.api}`)
console.log(`untrusted issuer ${issuers.get('untrusted').iss}, AuthKit stand-in ${kits.untrusted.api}`)
console.log(`configs in ${OUT}/{trusted,untrusted}/config.toml; request log ${logPath}`)
console.log('run from rust/:')
console.log('  cargo run -p desktop -- --config out/auth2/trusted/config.toml --screen connect --sign-in on')
console.log('  cargo run -p desktop -- --config out/auth2/trusted/config.toml --screen connect')
console.log('  cargo run -p desktop -- --config out/auth2/untrusted/config.toml --screen connect --sign-in on')
console.log(`serving for ${MINUTES} minutes; Ctrl-C ends it`)
const timer = setInterval(writeLog, 2000)
setTimeout(() => {
  clearInterval(timer)
  finish(0)
}, MINUTES * 60_000)
