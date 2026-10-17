import { afterAll, beforeAll, describe, expect, it } from 'bun:test'
import { decodeJwt, startIssuers } from './oidc-issuer'
import type { Issuers } from './oidc-issuer'

let issuers: Issuers

beforeAll(async () => {
  issuers = await startIssuers(['trusted', 'untrusted'])
})

afterAll(() => issuers.stop())

const b64url = (s: string) => Uint8Array.from(Buffer.from(s, 'base64url'))

describe('startIssuers', () => {
  it('serves OIDC discovery at {iss}/.well-known/openid-configuration', async () => {
    const iss = issuers.get('trusted').iss
    const res = await fetch(`${iss}/.well-known/openid-configuration`)
    const body = (await res.json()) as { issuer: string; jwks_uri: string }
    expect(body).toMatchObject({ issuer: iss, jwks_uri: `${iss}/jwks` })
  })

  it('gives every issuer its own path on 127.0.0.1', () => {
    expect(issuers.get('trusted').iss).toBe(`${issuers.url}/trusted`)
    expect(issuers.get('untrusted').iss).toBe(`${issuers.url}/untrusted`)
    expect(issuers.url).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/)
  })

  it('mints RS256 tokens its JWKS verifies', async () => {
    const issuer = issuers.get('trusted')
    const token = await issuer.mint({ sub: 'user_1', aud: 'proof' })
    const [h, p, sig] = token.split('.')
    const res = await fetch(`${issuer.iss}/jwks`)
    const { keys } = (await res.json()) as { keys: Array<JsonWebKey & { kid: string }> }
    const header = JSON.parse(Buffer.from(h, 'base64url').toString()) as { kid: string; alg: string }
    const jwk = keys.find((k) => k.kid === header.kid)
    expect(header.alg).toBe('RS256')
    const key = await crypto.subtle.importKey('jwk', jwk!, { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['verify'])
    const ok = await crypto.subtle.verify('RSASSA-PKCS1-v1_5', key, b64url(sig), new TextEncoder().encode(`${h}.${p}`))
    expect(ok).toBe(true)
  })

  it('does not verify a token of one issuer with the keys of another', async () => {
    const token = await issuers.get('untrusted').mint({ sub: 'user_1', aud: 'proof' })
    const [h, p, sig] = token.split('.')
    const res = await fetch(`${issuers.get('trusted').iss}/jwks`)
    const { keys } = (await res.json()) as { keys: Array<JsonWebKey> }
    const key = await crypto.subtle.importKey('jwk', keys[0], { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['verify'])
    expect(await crypto.subtle.verify('RSASSA-PKCS1-v1_5', key, b64url(sig), new TextEncoder().encode(`${h}.${p}`))).toBe(false)
  })

  it('writes iss, sub, iat, exp and the given claims, and leaves aud out when not given', async () => {
    const issuer = issuers.get('trusted')
    const claims = decodeJwt(await issuer.mint({ sub: 'user_2', client_id: 'client_1', ttlSeconds: 60 }))
    expect(claims).toMatchObject({ iss: issuer.iss, sub: 'user_2', client_id: 'client_1' })
    expect(claims.aud).toBeUndefined()
    expect((claims.exp as number) - (claims.iat as number)).toBe(60)
  })

  it('counts the requests for its discovery document and keys', async () => {
    const before = issuers.requests.length
    await fetch(`${issuers.get('untrusted').iss}/.well-known/openid-configuration`)
    expect(issuers.requests.slice(before)).toEqual(['/untrusted/.well-known/openid-configuration'])
  })
})
