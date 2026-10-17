// A stand-in for the two WorkOS AuthKit endpoints the desktop sign-in calls (AUTH2,
// rust/crates/desktop/src/signin), for the local proof in signin-proof.ts. It hands out access
// tokens from one of the test issuers of oidc-issuer.ts, so the local SpacetimeDB host verifies
// them through that issuer's discovery document, exactly as it verifies WorkOS tokens.
//
// The endpoints follow the WorkOS reference:
// - GET  /user_management/authorize: response_type=code, client_id, redirect_uri, state,
//   code_challenge, code_challenge_method=S256
//   (https://workos.com/docs/reference/authkit/authentication/get-authorization-url/pkce).
//   It shows a page with one "Continue" button (autofocused, so Enter works) instead of AuthKit's
//   hosted sign-in; the button redirects to redirect_uri with code and state.
// - POST /user_management/authenticate, JSON: grant_type authorization_code (client_id, code,
//   code_verifier) or refresh_token (client_id, refresh_token)
//   (https://workos.com/docs/reference/authkit/authentication/code and /refresh-token). The
//   answer has user, organization_id, access_token and refresh_token; access tokens carry
//   client_id and no aud, as AuthKit's do. Refresh tokens rotate and are single use here.
//
// Only 127.0.0.1 and only loopback redirect URIs.
import type { Issuer } from './oidc-issuer'

export interface MockUser {
  id: string
  email: string
  first_name: string
  last_name: string
}

export interface AuthKitOptions {
  clientId: string
  user: MockUser
  organizationId?: string
  /** Access token lifetime in seconds. */
  ttlSeconds?: number
}

export interface AuthKit {
  /** The API base the desktop's `workos_api` names, `http://127.0.0.1:<port>`. */
  api: string
  /** Every request, in order, without codes or tokens: what the transcript shows. */
  log: Array<string>
  stop: () => void
}

const b64url = (bytes: ArrayBuffer | Uint8Array) => Buffer.from(new Uint8Array(bytes)).toString('base64url')
const random = () => b64url(crypto.getRandomValues(new Uint8Array(24)))
const loopback = (uri: string) => /^http:\/\/127\.0\.0\.1:\d+\/callback$/.test(uri)
const escape = (s: string) => s.replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`)

export async function s256(verifier: string): Promise<string> {
  return b64url(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)))
}

export function startAuthKit(issuer: Issuer, opts: AuthKitOptions): AuthKit {
  const codes = new Map<string, { challenge: string; clientId: string }>()
  const refreshTokens = new Set<string>()
  const log: Array<string> = []
  const ttlSeconds = opts.ttlSeconds ?? 300
  const oauthError = (error: string, description: string, status = 400) =>
    Response.json({ error, error_description: description }, { status })

  async function answer() {
    const refresh_token = random()
    refreshTokens.add(refresh_token)
    return {
      user: { object: 'user', ...opts.user, email_verified: true },
      organization_id: opts.organizationId ?? null,
      access_token: await issuer.mint({ sub: opts.user.id, client_id: opts.clientId, ttlSeconds }),
      refresh_token,
      authentication_method: 'Password',
    }
  }

  const server = Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    async fetch(req) {
      const url = new URL(req.url)
      const q = url.searchParams
      if (req.method === 'GET' && url.pathname === '/user_management/authorize') {
        const redirect = q.get('redirect_uri') ?? ''
        log.push(
          `GET /user_management/authorize client_id=${q.get('client_id')} redirect_uri=${redirect} provider=${q.get('provider')} code_challenge_method=${q.get('code_challenge_method')} state=(${(q.get('state') ?? '').length} chars)`,
        )
        if (q.get('client_id') !== opts.clientId) return new Response('unknown client_id', { status: 400 })
        if (q.get('response_type') !== 'code') return new Response('response_type must be code', { status: 400 })
        if (q.get('code_challenge_method') !== 'S256' || !q.get('code_challenge'))
          return new Response('PKCE S256 is required for a public client', { status: 400 })
        if (!loopback(redirect)) return new Response('redirect_uri must be http://127.0.0.1:<port>/callback', { status: 400 })
        const code = random()
        codes.set(code, { challenge: q.get('code_challenge') ?? '', clientId: opts.clientId })
        const back = new URL(redirect)
        back.searchParams.set('code', code)
        back.searchParams.set('state', q.get('state') ?? '')
        const who = `${opts.user.first_name} ${opts.user.last_name} (${opts.user.email})`
        return new Response(
          `<!doctype html><meta charset=utf-8><title>Test AuthKit</title><body style="font-family:sans-serif;margin:3rem">` +
            `<h1>Test AuthKit for ${escape(issuer.iss)}</h1><p>Sign in to FlowTrace Books as ${escape(who)}.</p>` +
            `<form method=get action="/approve"><input type=hidden name=to value="${escape(back.toString())}">` +
            `<button autofocus style="font-size:1.4rem">Continue</button></form>`,
          { headers: { 'Content-Type': 'text/html; charset=utf-8' } },
        )
      }
      if (req.method === 'GET' && url.pathname === '/approve') {
        const to = q.get('to') ?? ''
        log.push('GET /approve (the person pressed Continue)')
        if (!loopback(to.split('?')[0])) return new Response('bad redirect', { status: 400 })
        return Response.redirect(to, 302)
      }
      if (req.method === 'POST' && url.pathname === '/user_management/authenticate') {
        const body = (await req.json()) as Record<string, string | undefined>
        const fields = Object.keys(body).sort().join(',')
        log.push(`POST /user_management/authenticate grant_type=${body.grant_type} fields=${fields}`)
        if (body.client_secret !== undefined) return oauthError('invalid_request', 'A public client sends no client_secret.')
        if (body.client_id !== opts.clientId) return oauthError('invalid_client', 'Unknown client_id.')
        if (body.grant_type === 'authorization_code') {
          const grant = codes.get(body.code ?? '')
          codes.delete(body.code ?? '')
          if (!grant) return oauthError('invalid_grant', 'The code is invalid or was already used.')
          if (!body.code_verifier || (await s256(body.code_verifier)) !== grant.challenge)
            return oauthError('invalid_grant', 'The code_verifier does not match the code_challenge.')
          return Response.json(await answer())
        }
        if (body.grant_type === 'refresh_token') {
          if (!refreshTokens.delete(body.refresh_token ?? '')) return oauthError('invalid_grant', 'Session has already ended.')
          return Response.json(await answer())
        }
        return oauthError('unsupported_grant_type', `No grant ${body.grant_type}.`)
      }
      return new Response('not found', { status: 404 })
    },
  })
  return { api: `http://127.0.0.1:${server.port}`, log, stop: () => void server.stop(true) }
}
