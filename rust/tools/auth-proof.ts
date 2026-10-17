// The AUTH1 proof on the LOCAL SpacetimeDB server: the module trusts an OpenID Connect issuer an
// operator registered, derives one identity per subject, refuses an issuer nobody registered, and
// keeps device identities working until an operator turns them off; a registered service (MOD2)
// connects either way.
//
//   cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
//   cd ../flowtrace && ~/.bun/bin/bun install --frozen-lockfile   # the SDK the web uses
//   cd ../rust/tools && ~/.bun/bin/bun run auth-proof [--keep]
//
// It starts two issuers of its own on 127.0.0.1 (src/oidc-issuer.ts: discovery and JWKS, RS256),
// publishes a fresh `ft-auth1-<timestamp>`, trusts only the first issuer, and connects over the
// WebSocket with the web's generated bindings and SDK (flowtrace/src/lib/ledger/spacetime-bindings),
// as the browser does, and over HTTP for the refusal texts. Each step prints `pass` or `FAIL`; the
// database is deleted at the end, also on failure, unless `--keep`. Exits 1 on any failure.
//
// The local host fetches the issuer's keys from 127.0.0.1 itself, so this needs no internet. Only
// ever the local server: src/module-server.ts refuses any other URL and any name not `ft-...`.
import { DbConnection } from '../../flowtrace/src/lib/ledger/spacetime-bindings'
import { startIssuers } from './src/oidc-issuer'
import { args, localServer } from './src/module-server'
import type { Json } from './src/module-server'

const AUDIENCE = 'flowtrace-auth1-proof'
const ORG = 'ft-auth1-org'
const stamp = new Date().toISOString().replace(/\D/g, '').slice(0, 14)
const server = localServer(`ft-auth1-${stamp}`)
const KEEP = args().flag('keep')
const WS_URL = server.url.replace(/^http/, 'ws')

let failures = 0
async function step(name: string, fn: () => Promise<string> | string) {
  const t = performance.now()
  try {
    const detail = await fn()
    console.log(`pass  ${name} (${(performance.now() - t).toFixed(0)} ms)${detail ? `: ${detail}` : ''}`)
  } catch (e) {
    failures++
    console.log(`FAIL  ${name}: ${e instanceof Error ? e.message : String(e)}`)
  }
}

function expect(cond: boolean, message: string) {
  if (!cond) throw new Error(message)
}

type Connected = { ok: true; identity: string } | { ok: false; error: string }

/** One WebSocket connection with `token` (none: the host mints one), closed as soon as it opens. */
function connect(token: string | undefined): Promise<Connected> {
  return new Promise((resolve) => {
    let done = false
    const finish = (r: Connected) => {
      if (done) return
      done = true
      clearTimeout(timer)
      resolve(r)
    }
    const timer = setTimeout(() => finish({ ok: false, error: 'no answer in 10 s' }), 10_000)
    DbConnection.builder()
      .withUri(WS_URL)
      .withDatabaseName(server.db)
      .withToken(token)
      .onConnect((conn, identity) => {
        finish({ ok: true, identity: identity.toHexString() })
        conn.disconnect()
      })
      .onConnectError((_ctx, error) => finish({ ok: false, error: error.message || String(error) }))
      .onDisconnect((_ctx, error) => finish({ ok: false, error: error?.message ?? 'closed before connecting' }))
      .build()
  })
}

/** A reducer call over HTTP as the holder of `token`: status and body. */
async function httpCall(token: string, reducer: string, argv: Array<Json>) {
  const res = await fetch(`${server.url}/v1/database/${server.db}/call/${reducer}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(argv),
  })
  return { status: res.status, text: await res.text() }
}

/** The CLI, as the identity that published the module (an operator through `init`). */
const operatorCall = (reducer: string, ...argv: Array<Json>) =>
  server.cli(['call', '-s', server.cliServer, server.db, reducer, ...argv.map((a) => JSON.stringify(a))])

const issuers = await startIssuers(['trusted', 'untrusted'])
const trusted = issuers.get('trusted')
const untrusted = issuers.get('untrusted')
console.log(`issuers: ${trusted.iss} (trusted below), ${untrusted.iss} (never trusted)`)

try {
  await step(`publish ${server.db} on ${server.url}`, () => server.publish().split('\n').at(-1) ?? '')

  await step('the operator trusts the first issuer for one audience', () => {
    operatorCall('add_trusted_issuer', trusted.iss, AUDIENCE, 'AUTH1 local proof')
    return `${trusted.iss}, audience ${AUDIENCE}`
  })

  let alice = ''
  await step('a token from the trusted issuer connects', async () => {
    const r = await connect(await trusted.mint({ sub: 'user_alice', aud: AUDIENCE }))
    expect(r.ok, `refused: ${r.ok ? '' : r.error}`)
    alice = r.ok ? r.identity : ''
    return `identity ${alice}`
  })

  await step('a second token for the same subject is the same identity', async () => {
    await Bun.sleep(1100) // a later iat, so the second token differs from the first
    const r = await connect(await trusted.mint({ sub: 'user_alice', aud: [AUDIENCE, 'other'], ttlSeconds: 120 }))
    expect(r.ok && r.identity === alice, `got ${JSON.stringify(r)}, wanted ${alice}`)
    return `identity ${alice} again (new iat, jti and exp; aud as a list)`
  })

  await step('a token without aud connects by its client_id, as WorkOS AuthKit tokens do', async () => {
    const r = await connect(await trusted.mint({ sub: 'user_alice', client_id: AUDIENCE }))
    expect(r.ok && r.identity === alice, `got ${JSON.stringify(r)}, wanted ${alice}`)
    return `identity ${alice} again`
  })

  await step('another subject at the same issuer is another identity', async () => {
    const r = await connect(await trusted.mint({ sub: 'user_carol', aud: AUDIENCE }))
    expect(r.ok && r.identity !== alice, `got ${JSON.stringify(r)}`)
    return `identity ${r.ok ? r.identity : ''}`
  })

  await step('a token from the trusted issuer for another audience is refused', async () => {
    const token = await trusted.mint({ sub: 'user_alice', aud: 'someone-else' })
    const ws = await connect(token)
    expect(!ws.ok, 'the WebSocket connected')
    const http = await httpCall(token, 'remove_member', [ORG, { __identity__: `0x${'0'.repeat(64)}` }])
    const want = `This database does not trust tokens from ${trusted.iss} for audience someone-else.`
    expect(http.status === 403 && http.text === want, `HTTP ${http.status} ${http.text}`)
    return `WebSocket refused (${ws.ok ? '' : ws.error}); HTTP 403 "${http.text}"`
  })

  await step('a validly signed token from an issuer nobody trusted is refused', async () => {
    const token = await untrusted.mint({ sub: 'user_alice', aud: AUDIENCE })
    const ws = await connect(token)
    expect(!ws.ok, 'the WebSocket connected')
    const http = await httpCall(token, 'remove_member', [ORG, { __identity__: `0x${'0'.repeat(64)}` }])
    const want = `This database does not trust tokens from ${untrusted.iss}.`
    expect(http.status === 403 && http.text === want, `HTTP ${http.status} ${http.text}`)
    return `HTTP 403 "${http.text}"`
  })

  await step('the host fetched discovery and keys of both issuers itself', () => {
    const fetched = new Set(issuers.requests)
    for (const name of ['trusted', 'untrusted'])
      for (const path of [`/${name}/.well-known/openid-configuration`, `/${name}/jwks`])
        expect(fetched.has(path), `the host never requested ${path}`)
    return [...fetched].join(', ')
  })

  const device = await server.newIdentity('device')
  await step('a device identity (host-issued token, dummy mode) connects while allowed', async () => {
    const r = await connect(device.token)
    expect(r.ok && r.identity === device.identity, JSON.stringify(r))
    const fresh = await connect(undefined)
    expect(fresh.ok, `a client with no token was refused: ${JSON.stringify(fresh)}`)
    return `identity ${device.identity}; a client with no token connects too`
  })

  await step('the operator turns device identities off; devices are refused, the rest connect', async () => {
    operatorCall('set_device_identities', false)
    const ws = await connect(device.token)
    expect(!ws.ok, 'the device WebSocket connected')
    const http = await httpCall(device.token, 'remove_member', [ORG, { __identity__: `0x${'0'.repeat(64)}` }])
    const want = 'This database does not accept device identities; sign in with an account it trusts.'
    expect(http.status === 403 && http.text === want, `HTTP ${http.status} ${http.text}`)
    const oidc = await connect(await trusted.mint({ sub: 'user_alice', aud: AUDIENCE }))
    expect(oidc.ok && oidc.identity === alice, `the trusted token was refused: ${JSON.stringify(oidc)}`)
    operatorCall('add_trusted_issuer', trusted.iss, AUDIENCE, 'AUTH1 local proof (relabelled)')
    return `HTTP 403 "${http.text}"; the trusted token and the operator's CLI still work`
  })

  await step('a registered service connects while device identities are off, and not once removed', async () => {
    const svc = await server.newIdentity('service')
    operatorCall('register_service', `0x${svc.identity}`, 'auth1-proof-service', ['flags'])
    const admitted = await connect(svc.token)
    expect(admitted.ok && admitted.identity === svc.identity, `the registered service was refused: ${JSON.stringify(admitted)}`)
    operatorCall('remove_service', `0x${svc.identity}`)
    const removed = await connect(svc.token)
    expect(!removed.ok, 'the removed service still connected with devices off')
    return `service ${svc.identity} let in while registered, refused once removed`
  })

  await step('the operator turns device identities back on; the device connects again', async () => {
    operatorCall('set_device_identities', true)
    const r = await connect(device.token)
    expect(r.ok && r.identity === device.identity, JSON.stringify(r))
    return ''
  })

  let bob = ''
  await step('the operator adds a member by issuer and subject before they ever connect', () => {
    operatorCall('create_org_ledger', ORG, 'KES', 2026, 'Proof Operator')
    operatorCall('add_member_by_subject', ORG, trusted.iss, 'user_bob', 'Bob Member', true, false, false)
    const refused = server.cliStatus([
      'call', '-s', server.cliServer, server.db, 'add_member_by_subject',
      JSON.stringify(ORG), JSON.stringify(untrusted.iss), '"user_eve"', '"Eve"', 'true', 'false', 'false',
    ])
    expect(!refused.ok && refused.text.includes(`No trusted issuer ${untrusted.iss}`), refused.text)
    const line = refused.text.split('\n').find((l) => l.includes('No trusted issuer'))?.trim()
    return `user_bob added; an untrusted issuer refused ("${line}")`
  })

  await step('bob signs in later and is that member: the module derived the host\'s identity', async () => {
    const token = await trusted.mint({ sub: 'user_bob', aud: AUDIENCE })
    const ws = await connect(token)
    expect(ws.ok, JSON.stringify(ws))
    bob = ws.ok ? ws.identity : ''
    const who = { label: 'bob', identity: bob, token }
    // my_member holds every member of the caller's organisations; bob's row is the one the
    // module keyed by the identity it derived in add_member_by_subject.
    const hex = (v: unknown) => String((v as { __identity__?: string }).__identity__ ?? v).replace(/^0x/, '')
    const members = (await server.sql(who, 'SELECT * FROM my_member')).filter((m) => m.org_id === ORG)
    const mine = members.find((m) => m.user_id === 'user_bob')
    expect(mine !== undefined, `bob sees no member row of his: ${JSON.stringify(members)}`)
    expect(hex(mine!.identity) === bob, `member row identity ${hex(mine!.identity)}, connection identity ${bob}`)
    expect(mine!.can_post === true && mine!.can_close === false, JSON.stringify(mine))
    const [org] = (await server.sql(who, 'SELECT * FROM my_ledger_org')).filter((r) => r.org_id === ORG)
    expect(org !== undefined, 'bob cannot read the books')
    return `identity ${bob} = the member row's identity; user id user_bob; reads ${ORG}`
  })
} finally {
  if (!KEEP) await step(`delete ${server.db}`, () => (server.remove(), ''))
  else console.log(`kept ${server.db} (--keep)`)
  issuers.stop()
}

console.log(failures === 0 ? 'auth-proof: all steps passed' : `auth-proof: ${failures} step(s) FAILED`)
process.exit(failures === 0 ? 0 : 1)
