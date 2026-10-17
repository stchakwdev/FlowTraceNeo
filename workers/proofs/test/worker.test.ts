/**
 * The Worker end to end in Miniflare (https://developers.cloudflare.com/workers/testing/miniflare/):
 * a real local R2 bucket, SpacetimeDB and the WorkOS JWKS stubbed through `outboundService`.
 */
import { afterAll, beforeAll, describe, expect, test } from 'bun:test'
import { exportJWK, generateKeyPair, SignJWT } from 'jose'
import { Miniflare } from 'miniflare'
import { join } from 'node:path'
import {
  HTML,
  ID,
  JPEG,
  ORG,
  PDF,
  PNG,
  group,
  groupMember,
  ledgerStub,
  memberRow,
  multipart,
  newState,
  stdbToken,
} from './fixtures'

const ROOT = join(import.meta.dir, '..')
const BASE = 'http://proofs.test'
const STDB = 'https://stdb.test'
const ISSUER = 'https://auth.workos.test'
const AUDIENCE = 'client_test'
const MODULE_TOKEN = 'module-token'
const SIGNING_KEY = 'test-signing-key'
const ORIGIN = 'http://localhost:3000'

const state = newState()
let mf: Miniflare
let privateKey: CryptoKey
let workosToken: string

const tok = {
  alice: stdbToken(ID.alice, 'alice'),
  tom: stdbToken(ID.tom, 'tom'),
  nobody: stdbToken(ID.nobody, 'nobody'),
  forged: stdbToken(ID.alice, 'forged'),
  stolen: stdbToken(ID.alice, 'stolen'),
}

beforeAll(async () => {
  const built = await Bun.build({
    entrypoints: [join(ROOT, 'src/index.ts')],
    outdir: join(ROOT, 'dist'),
    target: 'browser',
    format: 'esm',
    naming: 'worker.js',
  })
  if (!built.success) throw new Error(built.logs.map(String).join('\n'))

  const pair = await generateKeyPair('RS256', { extractable: true })
  privateKey = pair.privateKey as CryptoKey
  const jwk = { ...(await exportJWK(pair.publicKey)), kid: 'k1', alg: 'RS256', use: 'sig' }
  workosToken = await new SignJWT({})
    .setProtectedHeader({ alg: 'RS256', kid: 'k1' })
    .setIssuer(ISSUER)
    .setSubject('user_wendy')
    .setAudience(AUDIENCE)
    .setIssuedAt()
    .setExpirationTime('5m')
    .sign(privateKey)

  // Ledger: alice sees full proofs, tom thumbnails, wendy (WorkOS) full; nobody is no member.
  state.tokens.set(tok.alice, ID.alice)
  state.tokens.set(tok.tom, ID.tom)
  state.tokens.set(tok.nobody, ID.nobody)
  state.tokens.set(tok.stolen, ID.tom) // a real token, but for a different identity
  state.members.push(
    { org: ORG, row: memberRow(ID.alice, 'alice') },
    { org: ORG, row: memberRow(ID.tom, 'tom') },
    { org: ORG, row: memberRow(ID.wendy, 'user_wendy') },
    { org: 'org-b', row: memberRow(ID.nobody, 'nobody') },
  )
  state.docs.access_group.push(
    ...[
      group('g-staff', { can_view_nodes: true, can_view_proofs: true, can_view_full_proofs: true }),
      group('g-viewers', { can_view_nodes: true, can_view_proofs: true }),
    ].map((record) => ({ org: ORG, record })),
  )
  state.docs.access_group_member.push(
    ...[
      groupMember('gm1', 'g-staff', 'alice'),
      groupMember('gm2', 'g-viewers', 'tom'),
      groupMember('gm3', 'g-staff', 'user_wendy'),
      // The same user id in another organisation's group must not count here.
    ].map((record) => ({ org: ORG, record })),
    { org: 'org-b', record: groupMember('gm9', 'g-staff', 'nobody') },
  )

  const stub = ledgerStub(state, {
    moduleToken: MODULE_TOKEN,
    jwks: { keys: [jwk] },
    jwksPath: '/sso/jwks/client_test',
  })
  mf = new Miniflare({
    modules: true,
    scriptPath: join(ROOT, 'dist/worker.js'),
    compatibilityDate: '2026-07-01',
    r2Buckets: ['PROOFS'],
    bindings: {
      SPACETIMEDB_URL: STDB,
      SPACETIMEDB_DATABASE: 'flowtrace-ledger-dev',
      ALLOWED_ORIGINS: `${ORIGIN},https://app.example.org`,
      AUTH_MODES: 'spacetimedb,workos',
      WORKOS_ISSUER: ISSUER,
      WORKOS_JWKS_URL: `${ISSUER}/sso/jwks/client_test`,
      WORKOS_AUDIENCE: AUDIENCE,
      MODULE_TOKEN,
      URL_SIGNING_KEY: SIGNING_KEY,
    },
    outboundService: (req: unknown) => stub(req as Request) as never,
  })
}, 60_000)

afterAll(async () => {
  await mf?.dispose()
})

function fetchWorker(path: string, init: { method?: string; headers?: Record<string, string>; body?: Uint8Array } = {}) {
  return mf.dispatchFetch(`${BASE}${path}`, { redirect: 'manual', ...init } as never)
}

function uploadReq(
  token: string | null,
  file: { name: string; type: string; bytes: Uint8Array },
  fields: Record<string, string> = { org: ORG, link_kind: 'claim', link_id: 'claim-1' },
) {
  const { body, contentType } = multipart(fields, file)
  const headers: Record<string, string> = { 'Content-Type': contentType }
  if (token) headers.Authorization = `Bearer ${token}`
  return fetchWorker('/upload', { method: 'POST', headers, body })
}

/** Bytes as a plain array, for comparisons. */
const list = (b: ArrayBuffer | Uint8Array) => Array.from(new Uint8Array(b))

async function sha256Hex(b: Uint8Array) {
  return new Bun.CryptoHasher('sha256').update(b).digest('hex')
}

describe('upload', () => {
  test('stores the file content-addressed and reports key, hash and size', async () => {
    const res = await uploadReq(tok.alice, { name: 'receipt.jpg', type: 'image/jpeg', bytes: JPEG })
    expect(res.status).toBe(201)
    const body = (await res.json()) as Record<string, unknown>
    const sha = await sha256Hex(JPEG)
    expect(body).toEqual({
      key: `org/${ORG}/${sha}`,
      sha256: sha,
      size: JPEG.byteLength,
      content_type: 'image/jpeg',
      thumbnail_key: null,
    })
    const obj = await (await mf.getR2Bucket('PROOFS')).get(`org/${ORG}/${sha}`)
    expect(obj).not.toBeNull()
    expect(obj!.httpMetadata?.contentType).toBe('image/jpeg')
    expect(obj!.customMetadata).toEqual({
      uploaded_by: `spacetimedb:${ID.alice}`,
      link_kind: 'claim',
      link_id: 'claim-1',
      original_name: 'receipt.jpg',
    })
    expect(list(await obj!.arrayBuffer())).toEqual(list(JPEG))
  })

  test('accepts a PDF', async () => {
    const res = await uploadReq(tok.tom, { name: 'invoice.pdf', type: 'application/pdf', bytes: PDF })
    expect(res.status).toBe(201)
    const body = (await res.json()) as Record<string, unknown>
    expect(body.content_type).toBe('application/pdf')
    expect(body.thumbnail_key).toBeNull()
  })

  test('no token is 401', async () => {
    const res = await uploadReq(null, { name: 'a.jpg', type: 'image/jpeg', bytes: JPEG })
    expect(res.status).toBe(401)
    expect(((await res.json()) as { error: string }).error).toContain('Sign in')
  })

  test('a bad SpacetimeDB token is 401', async () => {
    expect((await uploadReq(tok.forged, { name: 'a.jpg', type: 'image/jpeg', bytes: JPEG })).status).toBe(401)
    expect((await uploadReq(tok.stolen, { name: 'a.jpg', type: 'image/jpeg', bytes: JPEG })).status).toBe(401)
    expect((await uploadReq('not-a-jwt', { name: 'a.jpg', type: 'image/jpeg', bytes: JPEG })).status).toBe(401)
  })

  test('a non-member is 403', async () => {
    const res = await uploadReq(tok.nobody, { name: 'a.jpg', type: 'image/jpeg', bytes: JPEG })
    expect(res.status).toBe(403)
  })

  test('over 25 MB is 413', async () => {
    const big = new Uint8Array(26 * 1024 * 1024)
    big.set(JPEG.subarray(0, 16))
    const res = await uploadReq(tok.alice, { name: 'big.jpg', type: 'image/jpeg', bytes: big })
    expect(res.status).toBe(413)
  }, 30_000)

  test('HTML disguised as a PNG is 415', async () => {
    const res = await uploadReq(tok.alice, { name: 'x.png', type: 'image/png', bytes: HTML })
    expect(res.status).toBe(415)
  })

  test('bad fields are 400', async () => {
    const file = { name: 'a.png', type: 'image/png', bytes: PNG }
    expect((await uploadReq(tok.alice, file, { org: ORG, link_kind: 'invoice', link_id: 'x' })).status).toBe(400)
    expect((await uploadReq(tok.alice, file, { org: "a'b", link_kind: 'bill', link_id: 'x' })).status).toBe(400)
    expect((await uploadReq(tok.alice, file, { org: ORG, link_kind: 'bill', link_id: '' })).status).toBe(400)
    expect((await uploadReq(tok.alice, { ...file, bytes: new Uint8Array() })).status).toBe(400)
  })

  test('a WorkOS token is accepted for a member matched by user_id', async () => {
    const res = await uploadReq(workosToken, { name: 'p.png', type: 'image/png', bytes: PNG })
    expect(res.status).toBe(201)
    const sha = await sha256Hex(PNG)
    const head = await (await mf.getR2Bucket('PROOFS')).head(`org/${ORG}/${sha}`)
    expect(head!.customMetadata!.uploaded_by).toBe('workos:user_wendy')
  })

  test('a WorkOS token signed by another key is 401', async () => {
    const other = await generateKeyPair('RS256')
    const forged = await new SignJWT({})
      .setProtectedHeader({ alg: 'RS256', kid: 'k1' })
      .setIssuer(ISSUER)
      .setSubject('user_wendy')
      .setAudience(AUDIENCE)
      .setExpirationTime('5m')
      .sign(other.privateKey)
    expect((await uploadReq(forged, { name: 'p.png', type: 'image/png', bytes: PNG })).status).toBe(401)
  })
})

describe('file access', () => {
  const jpegKey = async () => `org/${ORG}/${await sha256Hex(JPEG)}`
  const thumbKey = async () => `org/${ORG}/thumb/${await sha256Hex(JPEG)}.webp`
  const fileReq = (key: string, token: string, q = '') =>
    fetchWorker(`/file/${key}${q}`, { headers: { Authorization: `Bearer ${token}` } })

  beforeAll(async () => {
    // Record the uploaded JPEG and a thumbnail for it, as `create_verification` would.
    const bucket = await mf.getR2Bucket('PROOFS')
    await bucket.put(await thumbKey(), new TextEncoder().encode('RIFF....WEBPthumb'), {
      httpMetadata: { contentType: 'image/webp' },
    })
    state.docs.verification.push({
      org: ORG,
      record: {
        id: 'v-jpeg',
        claim_id: 'claim-1',
        type: 'receipt',
        object_key: await jpegKey(),
        sha256: await sha256Hex(JPEG),
        thumbnail_key: await thumbKey(),
        status: 'pending',
        visibility_level: 'full',
      } as { id: string },
    })
  })

  test('a full viewer gets a signed URL that streams the bytes', async () => {
    const res = await fileReq(await jpegKey(), tok.alice)
    expect(res.status).toBe(200)
    const { url, expires_at } = (await res.json()) as { url: string; expires_at: number }
    // Miniflare hands the Worker its loopback address as the origin; check the path.
    expect(new URL(url).pathname).toBe(`/signed/${await jpegKey()}`)
    expect(new URL(url).searchParams.get('sig')).toMatch(/^[0-9a-f]{64}$/)
    const left = expires_at * 1000 - Date.now()
    expect(left).toBeGreaterThan(9 * 60_000)
    expect(left).toBeLessThanOrEqual(10 * 60_000)

    const file = await mf.dispatchFetch(url)
    expect(file.status).toBe(200)
    expect(file.headers.get('Content-Type')).toBe('image/jpeg')
    expect(file.headers.get('X-Content-Type-Options')).toBe('nosniff')
    expect(file.headers.get('Content-Disposition')).toBe('inline')
    expect(file.headers.get('Content-Security-Policy')).toContain('sandbox')
    expect(file.headers.get('Cache-Control')).toMatch(/^private, max-age=(59\d|600)$/)
    expect(file.headers.get('ETag')).toBeTruthy()
    expect(list(await file.arrayBuffer())).toEqual(list(JPEG))

    const head = await mf.dispatchFetch(url, { method: 'HEAD' })
    expect(head.status).toBe(200)
    expect(head.headers.get('Content-Length')).toBe(String(JPEG.byteLength))
  })

  test('redirect=1 answers 302 to the signed URL', async () => {
    const res = await fileReq(await jpegKey(), tok.alice, '?redirect=1')
    expect(res.status).toBe(302)
    expect(new URL(res.headers.get('Location')!).pathname).toBe(`/signed/${await jpegKey()}`)
  })

  test('a thumbnail-only viewer gets 404 for the original but the thumbnail works', async () => {
    expect((await fileReq(await jpegKey(), tok.tom)).status).toBe(404)
    const res = await fileReq(await thumbKey(), tok.tom)
    expect(res.status).toBe(200)
    const { url } = (await res.json()) as { url: string }
    const file = await mf.dispatchFetch(url)
    expect(file.status).toBe(200)
    expect(file.headers.get('Content-Type')).toBe('image/webp')
  })

  test('a row naming an original as its thumbnail does not open the original to thumbnail viewers', async () => {
    state.docs.verification.push({
      org: ORG,
      record: { id: 'v-sneak', claim_id: 'claim-1', thumbnail_key: await jpegKey() } as { id: string },
    })
    expect((await fileReq(await jpegKey(), tok.tom)).status).toBe(404)
    state.docs.verification.pop()
  })

  test('the uploader may preview an unrecorded file; others get 404', async () => {
    const key = `org/${ORG}/${await sha256Hex(PDF)}` // uploaded by tom, not recorded
    expect((await fileReq(key, tok.tom)).status).toBe(200)
    expect((await fileReq(key, tok.alice)).status).toBe(404)
  })

  test('unknown keys are 404, a non-member 403', async () => {
    expect((await fileReq(`org/${ORG}/${'0'.repeat(64)}`, tok.alice)).status).toBe(404)
    expect((await fileReq('org/bad key/x', tok.alice)).status).toBe(404)
    expect((await fileReq(await jpegKey(), tok.nobody)).status).toBe(403)
  })

  test('a tampered or expired signed URL is 403', async () => {
    const res = await fileReq(await jpegKey(), tok.alice)
    const { url } = (await res.json()) as { url: string }
    const u = new URL(url)
    const sig = u.searchParams.get('sig')!
    u.searchParams.set('sig', (sig[0] === 'a' ? 'b' : 'a') + sig.slice(1))
    expect((await mf.dispatchFetch(u.toString())).status).toBe(403)

    const other = new URL(url)
    other.pathname = `/signed/${await thumbKey()}`
    expect((await mf.dispatchFetch(other.toString())).status).toBe(403)

    // A correctly signed URL whose time is up.
    const key = await jpegKey()
    const exp = Math.floor(Date.now() / 1000) - 5
    const mac = new Bun.CryptoHasher('sha256', SIGNING_KEY).update(`${key}\n${exp}`).digest('hex')
    const expired = await mf.dispatchFetch(`${BASE}/signed/${key}?exp=${exp}&sig=${mac}`)
    expect(expired.status).toBe(403)
    expect(((await expired.json()) as { error: string }).error).toContain('expired')
  })
})

describe('public', () => {
  beforeAll(async () => {
    state.docs.verification.push(
      {
        org: ORG,
        record: { id: 'v-pub', object_key: `org/${ORG}/${await sha256Hex(PNG)}`, sha256: await sha256Hex(PNG), published: true } as { id: string },
      },
      {
        org: ORG,
        record: { id: 'v-pdf', object_key: `org/${ORG}/${await sha256Hex(PDF)}`, sha256: await sha256Hex(PDF) } as { id: string },
      },
      {
        // Another organisation's admin records and publishes a proof naming this org's file.
        org: 'other-org',
        record: { id: 'v-steal', object_key: `org/${ORG}/${await sha256Hex(PDF)}`, sha256: await sha256Hex(PDF), published: true } as { id: string },
      },
    )
  })

  test('a published proof streams with a public cache header, and 304 on a matching ETag', async () => {
    const res = await fetchWorker(`/public/${await sha256Hex(PNG)}`)
    expect(res.status).toBe(200)
    expect(res.headers.get('Cache-Control')).toBe('public, max-age=3600')
    expect(res.headers.get('Content-Type')).toBe('image/png')
    expect(list(await res.arrayBuffer())).toEqual(list(PNG))
    const etag = res.headers.get('ETag')!
    const again = await fetchWorker(`/public/${await sha256Hex(PNG)}`, { headers: { 'If-None-Match': etag } })
    expect(again.status).toBe(304)
  })

  test("a published row cannot expose another organisation's file", async () => {
    const res = await fetchWorker(`/public/${await sha256Hex(PDF)}`)
    expect(res.status).toBe(404)
  })

  test('an unpublished proof is 404', async () => {
    expect((await fetchWorker(`/public/${await sha256Hex(PDF)}`)).status).toBe(404)
    expect((await fetchWorker(`/public/${'0'.repeat(64)}`)).status).toBe(404)
    expect((await fetchWorker('/public/nope')).status).toBe(404)
  })
})

describe('CORS and routing', () => {
  test('preflight echoes an allowed origin', async () => {
    const res = await fetchWorker('/upload', {
      method: 'OPTIONS',
      headers: { Origin: ORIGIN, 'Access-Control-Request-Method': 'POST' },
    })
    expect(res.status).toBe(204)
    expect(res.headers.get('Access-Control-Allow-Origin')).toBe(ORIGIN)
    expect(res.headers.get('Access-Control-Allow-Methods')).toBe('GET, POST')
    expect(res.headers.get('Access-Control-Allow-Headers')).toBe('Authorization, Content-Type')
  })

  test('other origins get no CORS headers', async () => {
    const res = await fetchWorker('/upload', { method: 'OPTIONS', headers: { Origin: 'https://evil.test' } })
    expect(res.status).toBe(403)
    expect(res.headers.get('Access-Control-Allow-Origin')).toBeNull()
  })

  test('JSON responses carry CORS headers for allowed origins', async () => {
    const res = await fetchWorker('/upload', { method: 'POST', headers: { Origin: ORIGIN } })
    expect(res.status).toBe(401)
    expect(res.headers.get('Access-Control-Allow-Origin')).toBe(ORIGIN)
  })

  test('anything else is 404 JSON', async () => {
    const res = await fetchWorker('/nope')
    expect(res.status).toBe(404)
    expect(await res.json()).toEqual({ error: 'Not found.' })
  })
})
