/**
 * Who is calling. A bearer token is either a WorkOS AuthKit access token (verified against the
 * WorkOS JWKS) or a SpacetimeDB token (verified by the SpacetimeDB host). The payload is decoded
 * unverified only to pick the path; each path then verifies the token properly.
 */
import { createRemoteJWKSet, jwtVerify } from 'jose'
import { authModeOn, type Env } from './env'
import { HttpError, sha256Hex } from './http'

export type Caller =
  | { kind: 'spacetimedb'; identity: string }
  | { kind: 'workos'; userId: string }

/** A stable string for the caller, stored as `uploaded_by`. */
export function callerId(c: Caller): string {
  return c.kind === 'spacetimedb' ? `spacetimedb:${c.identity}` : `workos:${c.userId}`
}

/** The bearer token of a request, or null. */
export function bearer(request: Request): string | null {
  const h = request.headers.get('Authorization')
  const m = h?.match(/^Bearer\s+(\S+)\s*$/i)
  return m ? m[1] : null
}

/** A JWT's payload, decoded without checking the signature. Null when it is not a JWT. */
export function decodePayload(token: string): Record<string, unknown> | null {
  const parts = token.split('.')
  if (parts.length !== 3) return null
  try {
    const b64 = parts[1].replace(/-/g, '+').replace(/_/g, '/')
    const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4)
    const text = new TextDecoder().decode(Uint8Array.from(atob(padded), (c) => c.charCodeAt(0)))
    const v: unknown = JSON.parse(text)
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : null
  } catch {
    return null
  }
}

export type Route = { path: 'workos' } | { path: 'spacetimedb'; identity: string } | { path: 'none' }

/**
 * Pick the verification path from the unverified payload: WorkOS when the WorkOS path is on and
 * `iss` equals `WORKOS_ISSUER`, else SpacetimeDB when the payload names a 64-hex `hex_identity`.
 */
export function routeToken(env: Env, payload: Record<string, unknown>): Route {
  const workosOn = authModeOn(env, 'workos') && !!env.WORKOS_ISSUER && !!env.WORKOS_JWKS_URL
  if (workosOn && payload.iss === env.WORKOS_ISSUER) return { path: 'workos' }
  const hex = payload.hex_identity
  if (authModeOn(env, 'spacetimedb') && typeof hex === 'string' && /^[0-9a-fA-F]{64}$/.test(hex)) {
    return { path: 'spacetimedb', identity: hex.toLowerCase() }
  }
  return { path: 'none' }
}

const CACHE_MS = 60_000
const CACHE_MAX = 1000
const verified = new Map<string, { caller: Caller; until: number }>()

const jwksByUrl = new Map<string, ReturnType<typeof createRemoteJWKSet>>()
function jwks(url: string) {
  let set = jwksByUrl.get(url)
  if (!set) {
    set = createRemoteJWKSet(new URL(url))
    jwksByUrl.set(url, set)
  }
  return set
}

/** Forget cached verifications (tests). */
export function clearAuthCache(): void {
  verified.clear()
  jwksByUrl.clear()
}

/**
 * `GET /v1/identity/:identity/verify` (https://spacetimedb.com/docs/http/identity): 204 when the
 * token belongs to the identity, 400 when it belongs to another, 401 when it is invalid.
 */
async function verifySpacetime(env: Env, token: string, identity: string): Promise<Caller> {
  const url = `${env.SPACETIMEDB_URL.replace(/\/+$/, '')}/v1/identity/${identity}/verify`
  let res: Response
  try {
    res = await fetch(url, { headers: { Authorization: `Bearer ${token}` } })
  } catch {
    throw new HttpError(502, 'Sign-in could not be checked right now. Try again shortly.')
  }
  if (res.status === 204) return { kind: 'spacetimedb', identity }
  if (res.status === 400) throw new HttpError(401, 'This sign-in token does not match its identity.')
  if (res.status === 401) throw new HttpError(401, 'This sign-in token is not valid. Sign in again.')
  throw new HttpError(502, 'Sign-in could not be checked right now. Try again shortly.')
}

/** Verify a WorkOS access token with jose (https://github.com/panva/jose). */
async function verifyWorkos(env: Env, token: string): Promise<Caller> {
  try {
    const { payload } = await jwtVerify(token, jwks(env.WORKOS_JWKS_URL!), {
      issuer: env.WORKOS_ISSUER,
      audience: env.WORKOS_AUDIENCE ? env.WORKOS_AUDIENCE : undefined,
    })
    if (typeof payload.sub !== 'string' || payload.sub === '') throw new Error('no sub')
    return { kind: 'workos', userId: payload.sub }
  } catch {
    throw new HttpError(401, 'This sign-in token is not valid or has expired. Sign in again.')
  }
}

/** The verified caller of a request; throws a 401 `HttpError` otherwise. */
export async function authenticate(env: Env, request: Request, now = Date.now()): Promise<Caller> {
  const token = bearer(request)
  if (!token) throw new HttpError(401, 'Sign in first: this request has no sign-in token.')
  const payload = decodePayload(token)
  if (!payload) throw new HttpError(401, 'The sign-in token could not be read.')
  const route = routeToken(env, payload)
  if (route.path === 'none') throw new HttpError(401, 'This kind of sign-in token is not accepted.')

  const cacheKey = await sha256Hex(token)
  const hit = verified.get(cacheKey)
  if (hit && hit.until > now) return hit.caller

  const caller =
    route.path === 'workos'
      ? await verifyWorkos(env, token)
      : await verifySpacetime(env, token, route.identity)

  if (verified.size >= CACHE_MAX) {
    for (const [k, v] of verified) if (v.until <= now) verified.delete(k)
    if (verified.size >= CACHE_MAX) verified.clear()
  }
  // Never cache past the token's own expiry.
  const exp = typeof payload.exp === 'number' ? payload.exp * 1000 : Infinity
  verified.set(cacheKey, { caller, until: Math.min(now + CACHE_MS, exp) })
  return caller
}
