import { afterAll, beforeAll, describe, expect, it } from 'bun:test'
import { s256, startAuthKit } from './authkit-mock'
import type { AuthKit } from './authkit-mock'
import { decodeJwt, startIssuers } from './oidc-issuer'
import type { Issuers } from './oidc-issuer'

let issuers: Issuers
let kit: AuthKit
const CLIENT = 'client_auth2_test'
const VERIFIER = 'dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk'

beforeAll(async () => {
  issuers = await startIssuers(['trusted'])
  kit = startAuthKit(issuers.get('trusted'), {
    clientId: CLIENT,
    user: { id: 'user_alice', email: 'alice@example.org', first_name: 'Alice', last_name: 'Test' },
  })
})

afterAll(() => {
  kit.stop()
  issuers.stop()
})

/** Walks the browser part: authorize page, Continue, redirect; returns the code. */
async function code(challenge: string, state = 'st-1') {
  const q = new URLSearchParams({
    response_type: 'code',
    provider: 'authkit',
    client_id: CLIENT,
    redirect_uri: 'http://127.0.0.1:45678/callback',
    state,
    code_challenge: challenge,
    code_challenge_method: 'S256',
  })
  const page = await (await fetch(`${kit.api}/user_management/authorize?${q}`)).text()
  const to = /name=to value="([^"]+)"/.exec(page)?.[1].replace(/&#38;/g, '&') ?? ''
  const res = await fetch(`${kit.api}/approve?${new URLSearchParams({ to })}`, { redirect: 'manual' })
  const back = new URL(res.headers.get('location') ?? '')
  expect(back.origin + back.pathname).toBe('http://127.0.0.1:45678/callback')
  expect(back.searchParams.get('state')).toBe(state)
  return back.searchParams.get('code') ?? ''
}

const authenticate = async (body: Record<string, string>) => {
  const res = await fetch(`${kit.api}/user_management/authenticate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return { status: res.status, body: (await res.json()) as Record<string, unknown> }
}

describe('startAuthKit', () => {
  it('computes the RFC 7636 appendix B challenge', async () => {
    expect(await s256(VERIFIER)).toBe('E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM')
  })

  it('trades a code for tokens only with the matching verifier, once', async () => {
    const challenge = await s256(VERIFIER)
    const wrong = await authenticate({ grant_type: 'authorization_code', client_id: CLIENT, code: await code(challenge), code_verifier: 'nope' })
    expect(wrong.status).toBe(400)
    const c = await code(challenge)
    const ok = await authenticate({ grant_type: 'authorization_code', client_id: CLIENT, code: c, code_verifier: VERIFIER })
    expect(ok.status).toBe(200)
    const claims = decodeJwt(ok.body.access_token as string)
    expect(claims).toMatchObject({ iss: issuers.get('trusted').iss, sub: 'user_alice', client_id: CLIENT })
    expect(claims.aud).toBeUndefined()
    expect(ok.body.user).toMatchObject({ id: 'user_alice', email: 'alice@example.org', first_name: 'Alice' })
    const again = await authenticate({ grant_type: 'authorization_code', client_id: CLIENT, code: c, code_verifier: VERIFIER })
    expect(again.status).toBe(400)
  })

  it('rotates refresh tokens and refuses a used one', async () => {
    const c = await code(await s256(VERIFIER))
    const first = await authenticate({ grant_type: 'authorization_code', client_id: CLIENT, code: c, code_verifier: VERIFIER })
    const r1 = first.body.refresh_token as string
    const second = await authenticate({ grant_type: 'refresh_token', client_id: CLIENT, refresh_token: r1 })
    expect(second.status).toBe(200)
    expect(second.body.refresh_token).not.toBe(r1)
    const reused = await authenticate({ grant_type: 'refresh_token', client_id: CLIENT, refresh_token: r1 })
    expect(reused).toEqual({ status: 400, body: { error: 'invalid_grant', error_description: 'Session has already ended.' } })
  })

  it('refuses a client secret, another client and a redirect off the loopback', async () => {
    expect((await authenticate({ grant_type: 'refresh_token', client_id: CLIENT, client_secret: 'sk', refresh_token: 'x' })).status).toBe(400)
    expect((await authenticate({ grant_type: 'refresh_token', client_id: 'client_other', refresh_token: 'x' })).status).toBe(400)
    const q = new URLSearchParams({ response_type: 'code', client_id: CLIENT, redirect_uri: 'https://evil.example/callback', code_challenge: 'c', code_challenge_method: 'S256' })
    expect((await fetch(`${kit.api}/user_management/authorize?${q}`)).status).toBe(400)
  })

  it('logs requests without codes or tokens', () => {
    expect(kit.log.some((l) => l.startsWith('POST /user_management/authenticate grant_type=refresh_token'))).toBe(true)
    expect(kit.log.join('\n')).not.toMatch(/code=|refresh_token=|eyJ/)
  })
})
