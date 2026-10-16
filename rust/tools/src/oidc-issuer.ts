// A tiny OpenID Connect issuer for the local auth proof (auth-proof.ts): one HTTP server on
// 127.0.0.1 with a free port, and one issuer per path, each with its own RSA key.
//
// The SpacetimeDB 2.12 host trusts a token's signature when the keys at the `jwks_uri` of
// `{iss}/.well-known/openid-configuration` verify it, for any http(s) issuer
// (crates/core/src/auth/token_validation.rs at v2.12.0, `JsonWebKeySet::from_oidc_url`), so
// discovery plus JWKS is all an issuer needs. Tokens are RS256
// (https://www.rfc-editor.org/rfc/rfc7518#section-3.3) with a `kid` naming the key.

export interface MintClaims {
  sub: string
  aud?: string | Array<string>
  client_id?: string
  /** Lifetime in seconds; 300 by default. */
  ttlSeconds?: number
}

export interface Issuer {
  /** The `iss` claim and the discovery base, `http://127.0.0.1:<port>/<name>`. */
  iss: string
  mint: (claims: MintClaims) => Promise<string>
}

export interface Issuers {
  url: string
  get: (name: string) => Issuer
  /** Every path requested, in order: shows the host fetched discovery and keys. */
  requests: Array<string>
  stop: () => void
}

const b64url = (bytes: Uint8Array | string) => Buffer.from(bytes).toString('base64url')

/** The payload of a JWT, unverified. */
export function decodeJwt(token: string): Record<string, unknown> {
  return JSON.parse(Buffer.from(token.split('.')[1], 'base64url').toString()) as Record<string, unknown>
}

interface Keyed {
  kid: string
  privateKey: CryptoKey
  jwk: JsonWebKey & { kid: string; alg: string; use: string }
}

async function newKey(kid: string): Promise<Keyed> {
  const pair = await crypto.subtle.generateKey(
    { name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
    true,
    ['sign', 'verify'],
  )
  const { kty, n, e } = await crypto.subtle.exportKey('jwk', pair.publicKey)
  return { kid, privateKey: pair.privateKey, jwk: { kty, n, e, kid, alg: 'RS256', use: 'sig' } }
}

/** Starts one server with an issuer at `/<name>` for each name. */
export async function startIssuers(names: Array<string>): Promise<Issuers> {
  const keys = new Map<string, Keyed>()
  for (const name of names) keys.set(name, await newKey(`${name}-key-1`))
  const requests: Array<string> = []
  let base = ''
  const server = Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch(req) {
      const path = new URL(req.url).pathname
      requests.push(path)
      const [, name, ...rest] = path.split('/')
      const key = keys.get(name)
      const tail = rest.join('/')
      if (!key) return new Response('no such issuer', { status: 404 })
      const iss = `${base}/${name}`
      if (tail === '.well-known/openid-configuration') {
        return Response.json({
          issuer: iss,
          jwks_uri: `${iss}/jwks`,
          response_types_supported: ['id_token'],
          subject_types_supported: ['public'],
          id_token_signing_alg_values_supported: ['RS256'],
        })
      }
      if (tail === 'jwks') return Response.json({ keys: [key.jwk] })
      return new Response('not found', { status: 404 })
    },
  })
  base = `http://127.0.0.1:${server.port}`

  const get = (name: string): Issuer => {
    const key = keys.get(name)
    if (!key) throw new Error(`No issuer ${name}.`)
    const iss = `${base}/${name}`
    return {
      iss,
      mint: async ({ sub, aud, client_id, ttlSeconds = 300 }) => {
        const iat = Math.floor(Date.now() / 1000)
        const payload: Record<string, unknown> = { iss, sub, iat, exp: iat + ttlSeconds, jti: crypto.randomUUID() }
        if (aud !== undefined) payload.aud = aud
        if (client_id !== undefined) payload.client_id = client_id
        const head = `${b64url(JSON.stringify({ alg: 'RS256', typ: 'JWT', kid: key.kid }))}.${b64url(JSON.stringify(payload))}`
        const sig = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key.privateKey, new TextEncoder().encode(head))
        return `${head}.${b64url(new Uint8Array(sig))}`
      },
    }
  }

  return { url: base, get, requests, stop: () => void server.stop(true) }
}
