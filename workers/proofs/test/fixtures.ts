/** Shared test fixtures: file bytes, multipart bodies, fake tokens and a SpacetimeDB stub. */

export const ORG = 'org-a'

/** Bytes starting with `head` and padded with a pattern to `size`. */
export function bytesOf(head: number[] | string, size = 256): Uint8Array {
  const h = typeof head === 'string' ? new TextEncoder().encode(head) : Uint8Array.from(head)
  const out = new Uint8Array(Math.max(size, h.length))
  out.set(h)
  for (let i = h.length; i < out.length; i++) out[i] = (i * 31) & 0xff
  return out
}

export const JPEG = bytesOf([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46])
export const PNG = bytesOf([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
export const PDF = bytesOf('%PDF-1.7\n')
export const HTML = new TextEncoder().encode('<!doctype html><script>alert(1)</script>')

/** A multipart/form-data body built by hand, so it works with any fetch implementation. */
export function multipart(
  fields: Record<string, string>,
  file?: { name: string; type: string; bytes: Uint8Array },
): { body: Uint8Array; contentType: string } {
  const boundary = '----flowtrace' + Math.random().toString(16).slice(2)
  const enc = new TextEncoder()
  const parts: Uint8Array[] = []
  for (const [k, v] of Object.entries(fields)) {
    parts.push(enc.encode(`--${boundary}\r\nContent-Disposition: form-data; name="${k}"\r\n\r\n${v}\r\n`))
  }
  if (file) {
    parts.push(
      enc.encode(
        `--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="${file.name}"\r\nContent-Type: ${file.type}\r\n\r\n`,
      ),
    )
    parts.push(file.bytes)
    parts.push(enc.encode('\r\n'))
  }
  parts.push(enc.encode(`--${boundary}--\r\n`))
  const total = parts.reduce((n, p) => n + p.length, 0)
  const body = new Uint8Array(total)
  let at = 0
  for (const p of parts) {
    body.set(p, at)
    at += p.length
  }
  return { body, contentType: `multipart/form-data; boundary=${boundary}` }
}

function b64url(s: string): string {
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

/** A SpacetimeDB-shaped token (`{"hex_identity":..., "iss":"localhost", ...}`); unsigned. */
export function stdbToken(identity: string, tag = 'x'): string {
  const header = b64url(JSON.stringify({ typ: 'JWT', alg: 'ES256' }))
  const payload = b64url(
    JSON.stringify({
      hex_identity: identity,
      sub: `sub-${tag}`,
      iss: 'localhost',
      aud: ['spacetimedb'],
      iat: 1_700_000_000,
    }),
  )
  return `${header}.${payload}.sig-${tag}`
}

export const ID = {
  alice: 'a1'.repeat(32),
  tom: 'b2'.repeat(32),
  wendy: 'c3'.repeat(32),
  nobody: 'd4'.repeat(32),
}

/** A member row value list, in `member` column order, identity as `{ __identity__ }`. */
export function memberRow(identity: string, userId: string): unknown[] {
  return [`${ORG}:${identity}`, { __identity__: `0x${identity}` }, ORG, userId, userId, true, false, false]
}

const str = { String: [] }
export const MEMBER_SCHEMA = {
  elements: [
    { name: { some: 'key' }, algebraic_type: str },
    {
      name: { some: 'identity' },
      algebraic_type: { Product: { elements: [{ name: { some: '__identity__' }, algebraic_type: { U256: [] } }] } },
    },
    { name: { some: 'org_id' }, algebraic_type: str },
    { name: { some: 'user_id' }, algebraic_type: str },
    { name: { some: 'name' }, algebraic_type: str },
    { name: { some: 'can_post' }, algebraic_type: { Bool: [] } },
    { name: { some: 'can_approve' }, algebraic_type: { Bool: [] } },
    { name: { some: 'can_close' }, algebraic_type: { Bool: [] } },
  ],
}

export const DOC_SCHEMA = {
  elements: ['key', 'org_id', 'id', 'seq', 'data'].map((n) => ({
    name: { some: n },
    algebraic_type: n === 'seq' ? { U64: [] } : str,
  })),
}

export function docRow(org: string, record: { id: string }, seq = 0): unknown[] {
  return [`${org}:${record.id}`, org, record.id, seq, JSON.stringify(record)]
}

/** Mutable ledger state the stub serves. */
export interface LedgerState {
  members: Array<{ org: string; row: unknown[] }>
  docs: Record<'access_group' | 'access_group_member' | 'verification', Array<{ org: string; record: { id: string } }>>
  /** token -> identity it belongs to */
  tokens: Map<string, string>
  sqlCalls: string[]
}

export function newState(): LedgerState {
  return {
    members: [],
    docs: { access_group: [], access_group_member: [], verification: [] },
    tokens: new Map(),
    sqlCalls: [],
  }
}

export function group(id: string, perms: Record<string, boolean>) {
  return { id, organization_id: ORG, name: id, permissions: perms }
}

export function groupMember(id: string, groupId: string, userId: string | null) {
  return { id, group_id: groupId, user_id: userId, user_email: null }
}

/**
 * A stub of the SpacetimeDB HTTP API (`/v1/identity/:id/verify`, `/v1/database/:db/sql`) and
 * of a JWKS endpoint, as one fetch handler.
 */
export function ledgerStub(
  state: LedgerState,
  opts: { moduleToken: string; jwks?: unknown; jwksPath?: string },
) {
  return async (req: Request): Promise<Response> => {
    const url = new URL(req.url)
    if (opts.jwks && url.pathname === (opts.jwksPath ?? '/jwks')) {
      return Response.json(opts.jwks)
    }
    const verify = url.pathname.match(/^\/v1\/identity\/([0-9a-f]{64})\/verify$/)
    if (verify) {
      const token = (req.headers.get('Authorization') ?? '').replace(/^Bearer /, '')
      const owner = state.tokens.get(token)
      if (!owner) return new Response(null, { status: 401 })
      return new Response(null, { status: owner === verify[1] ? 204 : 400 })
    }
    if (req.method === 'POST' && /^\/v1\/database\/[^/]+\/sql$/.test(url.pathname)) {
      if (req.headers.get('Authorization') !== `Bearer ${opts.moduleToken}`) {
        return new Response('unauthorized', { status: 401 })
      }
      const q = await req.text()
      state.sqlCalls.push(q)
      const m = q.match(/^SELECT \* FROM (\w+)(?: WHERE org_id = '((?:[^']|'')*)')?$/)
      if (!m) return new Response('bad sql', { status: 400 })
      const [, table, orgLit] = m
      const org = orgLit?.replaceAll("''", "'")
      if (table === 'member') {
        const rows = state.members.filter((r) => org === undefined || r.org === org).map((r) => r.row)
        return Response.json([{ schema: MEMBER_SCHEMA, rows, total_duration_micros: 1, stats: {} }])
      }
      if (table in state.docs) {
        const list = state.docs[table as keyof LedgerState['docs']]
        const rows = list
          .filter((r) => org === undefined || r.org === org)
          .map((r, i) => docRow(r.org, r.record, i))
        return Response.json([{ schema: DOC_SCHEMA, rows, total_duration_micros: 1, stats: {} }])
      }
      return new Response('no such table', { status: 400 })
    }
    return new Response('not stubbed: ' + req.url, { status: 599 })
  }
}
