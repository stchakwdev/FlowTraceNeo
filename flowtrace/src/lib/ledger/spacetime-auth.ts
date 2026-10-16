// Which token the SpacetimeDB connection presents. Kept apart from spacetime-backend.ts so the
// WorkOS auth bridge (auth-context.tsx) can register itself without loading the SDK.
//
// - dummy auth mode: this browser's device identity. The token the host minted is saved in
//   localStorage and presented again, so a browser profile stays one identity.
// - workos auth mode: the signed-in person's WorkOS AuthKit access token. The module trusts it
//   once an operator registers the WorkOS issuer (rust/crates/ledger-module/src/auth.rs), and the
//   host derives the identity from its issuer and subject, so one WorkOS user is one identity on
//   every browser and on the desktop. The token is never written to storage, and a fresh one is
//   asked for on every connection: AuthKit's getAccessToken refreshes a token that is about to
//   expire (@workos/authkit-tanstack-react-start, client/useAccessToken).
export type AuthMode = 'dummy' | 'workos'
export type AccessTokenSource = () => Promise<string | undefined>

/** Workos mode with nobody signed in: there is no token to connect with. */
export class SignInRequiredError extends Error {
  constructor() {
    super('Sign in to open the books.')
    this.name = 'SignInRequiredError'
  }
}

export interface ConnectionToken {
  /** For DbConnection.builder().withToken; undefined lets the host mint a device identity. */
  token: string | undefined
  /** Whether to save the token the host returns on connect: device identities only. */
  saveReturned: boolean
}

let source: AccessTokenSource | null = null
let waiting: Array<(s: AccessTokenSource) => void> = []

/** Registered by the WorkOS auth bridge with AuthKit's getAccessToken; null when it unmounts. */
export function setAccessTokenSource(next: AccessTokenSource | null) {
  source = next
  if (!next) return
  const ready = waiting
  waiting = []
  for (const resolve of ready) resolve(next)
}

const SOURCE_WAIT_MS = 10_000

/**
 * A current WorkOS access token. A connection can start before the bridge's effect has run
 * (children's effects run first), so this waits up to `waitMs` for the bridge to register.
 */
export async function workosAccessToken(
  waitMs = SOURCE_WAIT_MS,
): Promise<string | undefined> {
  const current =
    source ??
    (await new Promise<AccessTokenSource | null>((resolve) => {
      const timer = setTimeout(() => {
        waiting = waiting.filter((w) => w !== onReady)
        resolve(null)
      }, waitMs)
      const onReady = (s: AccessTokenSource) => {
        clearTimeout(timer)
        resolve(s)
      }
      waiting.push(onReady)
    }))
  return current ? current() : undefined
}

/** The token for the next connection in the given auth mode. */
export async function connectionToken(
  mode: AuthMode,
  sources: { device: () => string | undefined; workos: AccessTokenSource },
): Promise<ConnectionToken> {
  if (mode === 'dummy') return { token: sources.device(), saveReturned: true }
  const token = await sources.workos()
  if (!token) throw new SignInRequiredError()
  return { token, saveReturned: false }
}

/** A JWT's payload, unverified; null when it is not one. */
function claimsOf(token: string): Record<string, unknown> | null {
  try {
    const part = token.split('.')[1]
    const json = atob(part.replace(/-/g, '+').replace(/_/g, '/'))
    const claims: unknown = JSON.parse(json)
    return claims && typeof claims === 'object'
      ? (claims as Record<string, unknown>)
      : null
  } catch {
    return null
  }
}

/**
 * What to tell the person when the module refused the connection. The host closes the socket
 * without the module's reason, so this names what the module checks
 * (rust/crates/ledger-module/src/auth.rs, check_connection): in workos mode the token's issuer
 * and audience (its aud claim, else its client_id), which an operator registers with
 * add_trusted_issuer; in dummy mode the device-identity switch.
 */
export function refusalMessage(
  mode: AuthMode,
  token: string | undefined,
): string {
  if (mode === 'dummy')
    return 'SpacetimeDB refused this device’s identity: device identities are turned off on this database'
  const claims = token ? claimsOf(token) : null
  const iss = claims?.iss
  const aud = ([] as Array<unknown>)
    .concat(claims?.aud ?? [])
    .find((a) => typeof a === 'string')
  const audience = aud ?? claims?.client_id
  if (typeof iss !== 'string' || typeof audience !== 'string')
    return 'SpacetimeDB refused the connection'
  return `SpacetimeDB refused this sign-in: an operator must trust issuer ${iss} for audience ${audience} on this database`
}
