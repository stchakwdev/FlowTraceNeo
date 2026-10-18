// IT1's WorkOS mode (src/it1-workos.ts, src/it1-org.ts): the .env.it1 values, the refusal
// sentence, token claims, the desktop's xdg-open wrapper, the dev server command, and the AuthKit
// driver against a stand-in of the hosted page (email, "Continue with email", password, Continue).
// No network beyond 127.0.0.1; the browser test runs where Playwright's Chromium is cached.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { afterAll, describe, expect, test } from 'bun:test'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { identityOfKey, runPeople } from './it1-org'
import { chromiumPath, launchBrowser } from './it1-web'
import {
  ENV_IT1_KEYS,
  devServerArgs,
  installXdgOpen,
  jwtClaims,
  onAuthKit,
  openedUrls,
  parseEnvText,
  redactAuthorizeUrl,
  refusalValues,
  secondsLeft,
  signInOnWeb,
  workosIssuer,
  workosValues,
} from './it1-workos'

const FULL = [
  '# IT1 WorkOS path',
  'WORKOS_CLIENT_ID=client_01TEST',
  'IT1_WORKOS_ALICE_ID=user_01ALICE',
  'IT1_WORKOS_ALICE_EMAIL="alice@flowtrace.test"',
  'IT1_WORKOS_ALICE_PASSWORD=a=b#c',
  '',
  'IT1_WORKOS_BOB_ID=user_01BOB',
  "IT1_WORKOS_BOB_EMAIL='bob@flowtrace.test'",
  'IT1_WORKOS_BOB_PASSWORD=bob-secret',
].join('\n')

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'it1-workos-'))
afterAll(() => fs.rmSync(tmp, { recursive: true, force: true }))

/** An unsigned JWT with these claims (the run reads claims; the host verifies signatures). */
const jwt = (claims: Record<string, unknown>) => `${Buffer.from('{"alg":"none"}').toString('base64url')}.${Buffer.from(JSON.stringify(claims)).toString('base64url')}.`

describe('the .env.it1 values', () => {
  test('parses KEY=value lines, skipping comments and blanks, keeping = and # inside values, taking quotes off', () => {
    const env = parseEnvText(FULL)
    expect(env.get('IT1_WORKOS_ALICE_PASSWORD')).toBe('a=b#c')
    expect(env.get('IT1_WORKOS_ALICE_EMAIL')).toBe('alice@flowtrace.test')
    expect(env.get('IT1_WORKOS_BOB_EMAIL')).toBe('bob@flowtrace.test')
    expect(env.has('# IT1 WorkOS path')).toBe(false)
  })

  test('gives both people when every key is set', () => {
    const got = workosValues(FULL)
    expect(got).toEqual({
      ok: true,
      values: {
        clientId: 'client_01TEST',
        alice: { id: 'user_01ALICE', email: 'alice@flowtrace.test', password: 'a=b#c' },
        bob: { id: 'user_01BOB', email: 'bob@flowtrace.test', password: 'bob-secret' },
      },
    })
  })

  test('names the missing or empty keys, never a value', () => {
    const got = workosValues(FULL.replace('IT1_WORKOS_BOB_PASSWORD=bob-secret', 'IT1_WORKOS_BOB_PASSWORD=').replace(/^WORKOS_CLIENT_ID=.*$/m, ''))
    expect(got).toEqual({ ok: false, missing: ['WORKOS_CLIENT_ID', 'IT1_WORKOS_BOB_PASSWORD'] })
    expect(workosValues('')).toEqual({ ok: false, missing: [...ENV_IT1_KEYS] })
  })
})

describe('issuer, refusal and tokens', () => {
  test('the default issuer is the per-client user management URL', () => {
    expect(workosIssuer('client_01TEST')).toBe('https://api.workos.com/user_management/client_01TEST')
  })

  test("reads the issuer and audience out of the web's refusal sentence (spacetime-auth.ts refusalMessage)", () => {
    const iss = workosIssuer('client_01TEST')
    const sentence = `SpacetimeDB refused this sign-in: an operator must trust issuer ${iss} for audience client_01TEST on this database`
    expect(refusalValues(`Books. ${sentence}. Try again`)).toEqual({ issuer: iss, audience: 'client_01TEST' })
    expect(refusalValues('SpacetimeDB refused the connection')).toBeUndefined()
  })

  test('reads claims and the seconds left before exp', () => {
    const token = jwt({ iss: 'https://api.workos.com/user_management/client_01TEST', sub: 'user_01ALICE', exp: 1_800_000_300 })
    expect(jwtClaims(token).sub).toBe('user_01ALICE')
    expect(secondsLeft(token, 1_800_000_000_000)).toBe(300)
  })
})

describe('the people in workos mode', () => {
  test('Alice and Bob take their WorkOS user ids as user ids; Carol keeps hers', () => {
    const people = runPeople({ issuer: 'https://issuer.test', subjects: { alice: 'user_01ALICE', bob: 'user_01BOB' } })
    expect(people.map((p) => p.userId)).toEqual(['user_01ALICE', 'user_01BOB', 'it1-carol'])
    expect(runPeople().map((p) => p.userId)).toEqual(['it1-alice', 'it1-bob', 'it1-carol'])
  })

  test("takes the identity from a member row's key", () => {
    const hex = `c200${'ab'.repeat(30)}`
    expect(identityOfKey(`it1-20261018:${hex}`)).toBe(hex)
    expect(identityOfKey(`it1-20261018:0x${hex}`)).toBe(hex)
    expect(() => identityOfKey('it1-20261018:nope')).toThrow('not a member key')
  })
})

describe("the desktop's system browser", () => {
  test('the xdg-open wrapper records the URL it is given and opens nothing', () => {
    const urls = path.join(tmp, 'opened.urls')
    const PATH = installXdgOpen(path.join(tmp, 'bin'), urls, '/usr/bin:/bin')
    expect(PATH.startsWith(path.join(tmp, 'bin'))).toBe(true)
    const url = 'https://api.workos.com/user_management/authorize?response_type=code&client_id=client_01TEST&state=s3cr3t&code_challenge=abc'
    const r = Bun.spawnSync(['xdg-open', url], { env: { PATH } })
    expect(r.exitCode).toBe(0)
    expect(openedUrls(urls)).toEqual([url])
  })

  test('the transcript form hides state, challenge and client id', () => {
    const shown = redactAuthorizeUrl('https://api.workos.com/user_management/authorize?client_id=client_01TEST&state=s3cr3t&code_challenge=abc&provider=authkit')
    expect(shown).toBe('https://api.workos.com/user_management/authorize?client_id=(hidden)&state=(hidden)&code_challenge=(hidden)&provider=authkit')
  })

  test('knows a hosted AuthKit page by its host', () => {
    expect(onAuthKit('https://sample-env-staging.authkit.app/?client_id=x')).toBe(true)
    expect(onAuthKit('http://localhost:3000/ledger/accounts')).toBe(false)
  })
})

describe('the web dev server', () => {
  test('workos mode runs vite under bun --bun (Node 26 drops the sign-in redirect header); dummy mode as before', () => {
    expect(devServerArgs('workos', [])).toEqual(['--bun', 'run', 'dev'])
    expect(devServerArgs('dummy', [])).toEqual(['run', 'dev'])
    expect(devServerArgs('dummy', ['--host'])).toEqual(['run', 'dev', '--', '--host'])
  })
})

describe('the AuthKit driver, against a stand-in of the hosted page', () => {
  const PASSWORD = 'correct horse'
  /**
   * The app at /books redirects to /authkit (email, then password, two forms as AuthKit's), which
   * sends a correct sign-in back to /books with a session cookie. /books shows the books, or the
   * module's refusal sentence for `refused`.
   */
  function standIn(refused: boolean) {
    return Bun.serve({
      port: 0,
      hostname: '127.0.0.1',
      async fetch(req) {
        const url = new URL(req.url)
        const html = (body: string) => new Response(`<!doctype html><html><body>${body}</body></html>`, { headers: { 'content-type': 'text/html' } })
        if (url.pathname === '/books') {
          if (!(req.headers.get('cookie') ?? '').includes('session=1')) return Response.redirect(`${url.origin}/authkit`, 307)
          return html(refused ? '<p>SpacetimeDB refused this sign-in: an operator must trust issuer https://issuer.test/x for audience client_01TEST on this database</p>' : '<button>Add an account</button>')
        }
        if (url.pathname === '/authkit' && req.method === 'GET') {
          return html('<h1>Sign in</h1><form method="post"><input type="email" name="email"><button type="submit">Continue with email</button></form>')
        }
        if (url.pathname === '/authkit') {
          const form = await req.formData()
          if (form.has('password')) {
            if (form.get('password') !== PASSWORD) return html('<p>Wrong password</p>')
            return new Response(null, { status: 302, headers: { location: `${url.origin}/books`, 'set-cookie': 'session=1; Path=/' } })
          }
          return html(`<form method="post"><input type="hidden" name="email" value="${String(form.get('email'))}"><input type="password" name="password"><button type="submit">Continue</button></form>`)
        }
        return new Response('not found', { status: 404 })
      },
    })
  }
  const person = { id: 'user_01ALICE', email: 'alice@flowtrace.test', password: PASSWORD }

  test.skipIf(chromiumPath() === undefined)('signs in with email and password and comes back to the books', async () => {
    const server = standIn(false)
    const browser = await launchBrowser(false)
    try {
      const page = await (await browser.newContext()).newPage()
      const ms = await signInOnWeb(page, `http://127.0.0.1:${server.port}/books`, person, page.getByRole('button', { name: 'Add an account' }), 15000)
      expect(ms).toBeGreaterThan(0)
      expect(new URL(page.url()).pathname).toBe('/books')
    } finally {
      await browser.close()
      void server.stop(true)
    }
  }, 30000)

  test.skipIf(chromiumPath() === undefined)('says so when AuthKit sends an SSO-claimed email to its connection instead of a password', async () => {
    const server = standIn(false)
    const browser = await launchBrowser(false)
    try {
      const page = await (await browser.newContext()).newPage()
      // The stand-in's SSO case: the email form sends the browser to another host, neither the app
      // nor AuthKit (it does not resolve, so Chromium shows its own error page there).
      await page.route('**/authkit', (route) =>
        route.request().method() === 'POST' ? route.fulfill({ status: 302, headers: { location: 'http://idp.invalid/sso' } }) : route.continue(),
      )
      const run = signInOnWeb(page, `http://127.0.0.1:${server.port}/books`, person, page.getByRole('button', { name: 'Add an account' }), 15000)
      await expect(run).rejects.toThrow(/^AuthKit sent <user>@flowtrace\.test to \S+ instead of asking for a password: .*\(sso_required\)/)
    } finally {
      await browser.close()
      void server.stop(true)
    }
  }, 30000)

  test.skipIf(chromiumPath() === undefined)('fails with the issuer and audience when the module refuses the sign-in', async () => {
    const server = standIn(true)
    const browser = await launchBrowser(false)
    try {
      const page = await (await browser.newContext()).newPage()
      const run = signInOnWeb(page, `http://127.0.0.1:${server.port}/books`, person, page.getByRole('button', { name: 'Add an account' }), 15000)
      await expect(run).rejects.toThrow('add_trusted_issuer "https://issuer.test/x" "client_01TEST"')
    } finally {
      await browser.close()
      void server.stop(true)
    }
  }, 30000)
})
