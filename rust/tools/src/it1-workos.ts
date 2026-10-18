// IT1's WorkOS mode (`--auth workos`): the AuthKit sign-in driver for Playwright, the desktop's
// system browser for its "Sign in with WorkOS" flow, and the values in rust/tools/.env.it1.
//
// The web signs in on the hosted AuthKit page with email and password (the sandbox's method):
// email, "Continue with email", password, Continue; AuthKit then returns to the web app's
// WORKOS_REDIRECT_URI and the app connects to SpacetimeDB with the AuthKit access token
// (flowtrace/README.md, "Signing in with WorkOS against dev"). The desktop opens the system
// browser through `xdg-open` (GPUI `open_url` on Linux, rust/crates/desktop/README.md); a wrapper
// first on PATH hands the URL to the run instead, which signs in on it with the same driver
// (AUTH2's approach, rust/docs/module-release.md section 5).
//
// The values are secrets: nothing here prints a password or a token.
import fs from 'node:fs'
import path from 'node:path'
import type { Locator, Page } from 'playwright-core'

export interface WorkosPerson {
  /** The WorkOS user id: the subject `add_member_by_subject` takes. */
  id: string
  email: string
  password: string
}

export interface WorkosIt1 {
  clientId: string
  alice: WorkosPerson
  bob: WorkosPerson
}

/** What rust/tools/.env.it1 must hold for `--auth workos`. */
export const ENV_IT1_KEYS = [
  'WORKOS_CLIENT_ID',
  'IT1_WORKOS_ALICE_ID',
  'IT1_WORKOS_ALICE_EMAIL',
  'IT1_WORKOS_ALICE_PASSWORD',
  'IT1_WORKOS_BOB_ID',
  'IT1_WORKOS_BOB_EMAIL',
  'IT1_WORKOS_BOB_PASSWORD',
] as const

/** `KEY=value` lines; blank lines and `#` comments skipped, one pair of quotes taken off. */
export function parseEnvText(text: string): Map<string, string> {
  const values = new Map<string, string>()
  for (const raw of text.split('\n')) {
    const line = raw.trim()
    if (line === '' || line.startsWith('#')) continue
    const eq = line.indexOf('=')
    if (eq < 1) continue
    const value = line.slice(eq + 1).trim()
    values.set(line.slice(0, eq).trim(), value.replace(/^(['"])(.*)\1$/, '$2'))
  }
  return values
}

/** The WorkOS values, or the keys .env.it1 lacks (by name only). */
export function workosValues(text: string): { ok: true; values: WorkosIt1 } | { ok: false; missing: Array<string> } {
  const env = parseEnvText(text)
  const missing = ENV_IT1_KEYS.filter((k) => (env.get(k) ?? '') === '')
  if (missing.length > 0) return { ok: false, missing }
  const get = (k: (typeof ENV_IT1_KEYS)[number]) => env.get(k) as string
  return {
    ok: true,
    values: {
      clientId: get('WORKOS_CLIENT_ID'),
      alice: { id: get('IT1_WORKOS_ALICE_ID'), email: get('IT1_WORKOS_ALICE_EMAIL'), password: get('IT1_WORKOS_ALICE_PASSWORD') },
      bob: { id: get('IT1_WORKOS_BOB_ID'), email: get('IT1_WORKOS_BOB_EMAIL'), password: get('IT1_WORKOS_BOB_PASSWORD') },
    },
  }
}

/**
 * The issuer of the environment's access tokens without a custom auth domain
 * (rust/docs/module-release.md section 5, "Registering WorkOS for a database").
 */
export const workosIssuer = (clientId: string) => `https://api.workos.com/user_management/${clientId}`

/**
 * The issuer and audience in the web's refusal sentence (flowtrace/src/lib/ledger/spacetime-auth.ts,
 * `refusalMessage`; the desktop says the same, rust/crates/desktop/src/signin.rs): what an
 * operator registers with `add_trusted_issuer`.
 */
export function refusalValues(text: string): { issuer: string; audience: string } | undefined {
  const m = /must trust issuer (\S+) for audience (\S+) on this database/.exec(text)
  return m ? { issuer: m[1], audience: m[2] } : undefined
}

/** The claims of a JWT, unverified (for `exp`, `iss` and `sub`; the host verifies). */
export function jwtClaims(token: string): Record<string, unknown> {
  const part = token.split('.')[1] ?? ''
  return JSON.parse(Buffer.from(part, 'base64url').toString('utf8')) as Record<string, unknown>
}

/** True when the hosted AuthKit page is showing (any `*.authkit.app` host). */
export const onAuthKit = (url: string) => /\.authkit\.app$/.test(new URL(url).host)

/**
 * Signs in on the hosted AuthKit page the browser is on, or is about to reach: email,
 * "Continue with email", password, Continue. Returns once the password was submitted; the caller
 * waits for wherever AuthKit sends the browser next.
 */
export async function authkitSignIn(page: Page, person: Pick<WorkosPerson, 'email' | 'password'>, timeout = 30000) {
  const email = page.locator('input[name="email"]')
  await email.waitFor({ timeout })
  await email.fill(person.email)
  await page.getByRole('button', { name: /^Continue/ }).click()
  const password = page.locator('input[type="password"]')
  // An email whose domain an organisation claims with SSO is sent to that connection instead
  // (WorkOS answers a password sign-in for it with `sso_required`): say so, not a bare timeout.
  const elsewhere = page.waitForURL((u) => !onAuthKit(u.href) && !/^http:\/\/(localhost|127\.0\.0\.1)[:/]/.test(u.href), { timeout })
  const shown = await Promise.race([password.waitFor({ timeout }).then(() => true), elsewhere.then(() => false)]).catch(() => undefined)
  if (shown === false) {
    throw new Error(`AuthKit sent ${person.email.replace(/^[^@]*/, '<user>')} to ${new URL(page.url()).host} instead of asking for a password: an organisation in this WorkOS environment claims that email domain with an SSO connection (sso_required). Use emails on an unclaimed domain, or remove the domain from that organisation.`)
  }
  if (shown === undefined) throw new Error(`AuthKit showed no password field within ${timeout} ms (at ${new URL(page.url()).host})`)
  await password.fill(person.password)
  await page.getByRole('button', { name: /^(Continue|Sign in)/ }).click()
}

/**
 * The desktop's system browser for one run: an `xdg-open` script for the front of PATH that
 * appends the URL it is given to `urlFile` and opens nothing.
 */
export function xdgOpenScript(urlFile: string): string {
  return `#!/bin/sh\n# IT1: the desktop's system browser; the run signs in on the URL (rust/tools/src/it1-workos.ts).\nprintf '%s\\n' "$1" >> ${JSON.stringify(urlFile)}\nexit 0\n`
}

/** Writes the wrapper into `binDir` and returns the PATH that puts it first. */
export function installXdgOpen(binDir: string, urlFile: string, pathVar = process.env.PATH ?? ''): string {
  fs.mkdirSync(binDir, { recursive: true })
  fs.writeFileSync(path.join(binDir, 'xdg-open'), xdgOpenScript(urlFile), { mode: 0o755 })
  return `${binDir}${path.delimiter}${pathVar}`
}

/** The AuthKit authorize URLs the wrapper received so far. */
export function openedUrls(urlFile: string): Array<string> {
  if (!fs.existsSync(urlFile)) return []
  return fs
    .readFileSync(urlFile, 'utf8')
    .split('\n')
    .filter((l) => l.startsWith('http'))
}

/** An authorize URL without its state and PKCE challenge, for the transcript. */
export const redactAuthorizeUrl = (url: string) => url.replace(/(state|code_challenge|client_id)=[^&]*/g, '$1=(hidden)')

/** Seconds until a JWT's `exp`. */
export const secondsLeft = (token: string, nowMs = Date.now()) => Number(jwtClaims(token).exp) - nowMs / 1000

/**
 * An AuthKit access token for `person` without a browser, for the run's own reads and the
 * probe's ledger-client calls as that person: `POST {api}/user_management/authenticate` with
 * `grant_type=password` and the environment's API key as client secret
 * (https://workos.com/docs/reference/user-management/authentication/password). Same environment,
 * so the same `iss` and `sub` as the person's browser sign-in, hence the same identity. The live
 * sign-ins themselves go through AuthKit (authkitSignIn); this only mints tokens between them.
 */
export async function passwordToken(api: string, clientId: string, apiKey: string, person: WorkosPerson): Promise<string> {
  const res = await fetch(`${api}/user_management/authenticate`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ client_id: clientId, client_secret: apiKey, grant_type: 'password', email: person.email, password: person.password }),
  })
  const body = (await res.json()) as { access_token?: string; user?: { id?: string }; code?: string; message?: string; error?: string; error_description?: string }
  if (!res.ok || typeof body.access_token !== 'string') {
    throw new Error(`WorkOS refused a password sign-in for ${person.id}: ${res.status} ${body.code ?? body.error ?? ''} ${body.message ?? body.error_description ?? ''}`.trim())
  }
  if (body.user?.id !== person.id) throw new Error(`WorkOS signed in ${String(body.user?.id)}, not ${person.id}: check the email in .env.it1`)
  return body.access_token
}

/**
 * The web's dev server command: under Bun's runtime in workos mode, where Node 26 with Nitro's
 * dev worker (nitro 3.0.1-alpha.1, srvx 0.10.1) delivers responses without their headers, so the
 * sign-in redirect arrives as a bare 307 with no Location and the browser shows a blank page
 * (seen on dev 2026-10-18; under `bun --bun` the same request carries its Location).
 */
export function devServerArgs(auth: 'dummy' | 'workos', extra: Array<string>): Array<string> {
  return [...(auth === 'workos' ? ['--bun'] : []), 'run', 'dev', ...(extra.length > 0 ? ['--', ...extra] : [])]
}

/**
 * Opens `url` on the web app and signs in on AuthKit when the app sends the browser there, then
 * waits for `ready` (the Books screen loaded as the person) or the module's refusal sentence,
 * which fails with the issuer and audience to register.
 */
export async function signInOnWeb(page: Page, url: string, person: WorkosPerson, ready: Locator, timeout = 60000): Promise<number> {
  const t = performance.now()
  await page.goto(url)
  if (onAuthKit(page.url()) || (await page.locator('input[name="email"]').count()) > 0) await authkitSignIn(page, person)
  await page.waitForURL((u) => u.origin === new URL(url).origin, { timeout })
  const refusal = page.getByText(/must trust issuer \S+ for audience \S+/)
  await Promise.race([ready.waitFor({ timeout }), refusal.waitFor({ timeout })])
  if (await refusal.isVisible()) {
    const v = refusalValues((await refusal.textContent()) ?? '')
    throw new Error(`the module refused the sign-in: an operator must run add_trusted_issuer "${v?.issuer}" "${v?.audience}" on this database`)
  }
  return Math.round(performance.now() - t)
}
