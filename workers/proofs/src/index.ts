/**
 * FlowTrace proofs Worker: stores proof files (receipts, invoices, photos) in R2 and serves
 * them with the same access rules as the ledger module. See README.md for the routes.
 */
import { authenticate, callerId, type Caller } from './auth'
import type { Env } from './env'
import { HttpError, errorResponse, json, preflight, sha256Hex, withCors } from './http'
import {
  isLinkId,
  isLinkKind,
  isOrgId,
  isSha256,
  originalKey,
  parseKey,
  thumbnailKey,
  type ParsedKey,
} from './keys'
import {
  accessGroups,
  allVerifications,
  findMember,
  groupMembers,
  members,
  verifications,
} from './module'
import { signedUrl, verify } from './sign'
import { isImage, sniff } from './sniff'
import { makeThumbnail } from './thumbnail'
import { callerVisibility, type Member } from './visibility'

/** Largest accepted file. Not exported: every named export of the entry is a Worker entrypoint. */
const MAX_BYTES = 25 * 1024 * 1024
/** Room for the multipart boundaries and the other fields. */
const FORM_OVERHEAD = 64 * 1024

const nowSeconds = () => Math.floor(Date.now() / 1000)

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    let response: Response
    try {
      response = await route(request, env)
    } catch (e) {
      response =
        e instanceof HttpError
          ? errorResponse(e.status, e.message)
          : errorResponse(500, 'Something went wrong on our side. Try again shortly.')
      if (!(e instanceof HttpError)) console.error(e)
    }
    return withCors(env, request, response)
  },
}

async function route(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url)
  const path = url.pathname
  const method = request.method

  if (method === 'OPTIONS') return preflight(env, request)
  if (method === 'POST' && path === '/upload') return upload(request, env)
  if (method === 'GET' && path.startsWith('/file/')) {
    return fileUrl(request, env, pathKey(path, '/file/'), url)
  }
  if ((method === 'GET' || method === 'HEAD') && path.startsWith('/signed/')) {
    return signed(request, env, pathKey(path, '/signed/'), url)
  }
  if ((method === 'GET' || method === 'HEAD') && path.startsWith('/public/')) {
    return publicFile(request, env, path.slice('/public/'.length).toLowerCase())
  }
  return errorResponse(404, 'Not found.')
}

/** The object key after a route prefix, URL-decoded. */
function pathKey(path: string, prefix: string): string {
  try {
    return decodeURIComponent(path.slice(prefix.length))
  } catch {
    throw new HttpError(404, 'No such file.')
  }
}

/** The caller's member row in `org`, or a 403. */
async function requireMember(env: Env, org: string, caller: Caller): Promise<Member> {
  const member = findMember(await members(env, org), caller)
  if (!member) throw new HttpError(403, 'You are not a member of this organisation.')
  return member
}

function field(form: FormData, name: string): string {
  const v = form.get(name)
  return typeof v === 'string' ? v : ''
}

/** `POST /upload`: store a proof file. The client then records it with `create_verification`. */
async function upload(request: Request, env: Env): Promise<Response> {
  const caller = await authenticate(env, request)

  const declared = Number(request.headers.get('Content-Length') ?? '0')
  if (declared > MAX_BYTES + FORM_OVERHEAD) {
    throw new HttpError(413, 'That file is too large. The limit is 25 MB.')
  }
  if (!(request.headers.get('Content-Type') ?? '').startsWith('multipart/form-data')) {
    throw new HttpError(400, 'Send the file as multipart/form-data.')
  }
  let form: FormData
  try {
    form = await request.formData()
  } catch {
    throw new HttpError(400, 'The upload could not be read as a form.')
  }

  const org = field(form, 'org')
  const linkKind = field(form, 'link_kind')
  const linkId = field(form, 'link_id')
  const file = form.get('file')
  if (!isOrgId(org)) throw new HttpError(400, 'Say which organisation this proof is for (org).')
  if (!isLinkKind(linkKind)) {
    throw new HttpError(400, 'link_kind must be claim, bill or transaction.')
  }
  if (!isLinkId(linkId)) {
    throw new HttpError(400, 'link_id must be 1 to 128 characters with no control characters.')
  }
  if (!file || typeof file === 'string') throw new HttpError(400, 'Attach the file as "file".')

  // Any member may attach a proof (`create_verification`).
  await requireMember(env, org, caller)

  if (file.size > MAX_BYTES) throw new HttpError(413, 'That file is too large. The limit is 25 MB.')
  if (file.size === 0) throw new HttpError(400, 'That file is empty.')

  const bytes = new Uint8Array(await file.arrayBuffer())
  const contentType = sniff(bytes)
  if (!contentType) {
    throw new HttpError(415, 'Only photos (JPEG, PNG, WebP, GIF, HEIC) and PDF files are accepted.')
  }

  const sha256 = await sha256Hex(bytes)
  const key = originalKey(org, sha256)
  const customMetadata = {
    uploaded_by: callerId(caller),
    link_kind: linkKind,
    link_id: linkId,
    original_name: (file.name || '').slice(0, 256),
  }
  // Content-addressed: an existing object already holds these exact bytes.
  if (!(await env.PROOFS.head(key))) {
    await env.PROOFS.put(key, bytes, { httpMetadata: { contentType }, customMetadata })
  }

  let thumb: string | null = null
  if (isImage(contentType)) {
    const tk = thumbnailKey(org, sha256)
    if (await env.PROOFS.head(tk)) {
      thumb = tk
    } else {
      const webp = await makeThumbnail(env.IMAGES, bytes)
      if (webp) {
        await env.PROOFS.put(tk, webp, {
          httpMetadata: { contentType: 'image/webp' },
          customMetadata,
        })
        thumb = tk
      }
    }
  }

  return json(
    { key, sha256, size: bytes.byteLength, content_type: contentType, thumbnail_key: thumb },
    201,
  )
}

/** May the caller fetch `parsed`? Unknown and not-allowed are both false (the caller gets 404). */
async function mayFetch(env: Env, parsed: ParsedKey, caller: Caller, member: Member) {
  const [rows, groups, gms] = await Promise.all([
    verifications(env, parsed.org),
    accessGroups(env, parsed.org),
    groupMembers(env, parsed.org),
  ])
  const level = callerVisibility(groups, gms, member)
  // The key's own shape decides which column may name it, so a row cannot pass an original off
  // as a thumbnail (or the other way) to a viewer with less access.
  const asOriginal = !parsed.thumbnail && rows.some((v) => v.object_key === parsed.key)
  const asThumb = parsed.thumbnail && rows.some((v) => v.thumbnail_key === parsed.key)
  if (asOriginal && level === 'full') return true
  if (asThumb && (level === 'thumbnail' || level === 'full')) return true
  if (asOriginal || asThumb) return false
  // Not recorded yet: only the uploader may preview it.
  const head = await env.PROOFS.head(parsed.key)
  return head?.customMetadata?.uploaded_by === callerId(caller)
}

/** `GET /file/<key>`: a short-lived signed URL, or a 302 to it with `?redirect=1`. */
async function fileUrl(request: Request, env: Env, key: string, url: URL): Promise<Response> {
  const caller = await authenticate(env, request)
  const parsed = parseKey(key)
  if (!parsed) throw new HttpError(404, 'No such file.')
  const member = await requireMember(env, parsed.org, caller)
  if (!(await mayFetch(env, parsed, caller, member)) || !(await env.PROOFS.head(key))) {
    throw new HttpError(404, 'No such file.')
  }
  const { url: signedLink, exp } = await signedUrl(env.URL_SIGNING_KEY, url.origin, key, nowSeconds())
  if (url.searchParams.get('redirect') === '1') {
    return new Response(null, { status: 302, headers: { Location: signedLink, 'Cache-Control': 'no-store' } })
  }
  return json({ url: signedLink, expires_at: exp })
}

/** Headers for streaming a stored file. */
function fileHeaders(obj: R2Object, cacheControl: string): Headers {
  const h = new Headers()
  obj.writeHttpMetadata(h)
  if (!h.get('Content-Type')) h.set('Content-Type', 'application/octet-stream')
  h.set('ETag', obj.httpEtag)
  h.set('Content-Length', String(obj.size))
  h.set('Cache-Control', cacheControl)
  h.set('X-Content-Type-Options', 'nosniff')
  h.set(
    'Content-Security-Policy',
    "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; sandbox",
  )
  h.set('Content-Disposition', 'inline')
  return h
}

/** `GET|HEAD /signed/<key>?exp=&sig=`: stream the file if the signature holds. */
async function signed(request: Request, env: Env, key: string, url: URL): Promise<Response> {
  const now = nowSeconds()
  const check = await verify(
    env.URL_SIGNING_KEY,
    key,
    url.searchParams.get('exp'),
    url.searchParams.get('sig'),
    now,
  )
  if (check === 'expired') throw new HttpError(403, 'This link has expired. Ask for a new one.')
  if (check !== 'ok' || !parseKey(key)) throw new HttpError(403, 'This link is not valid.')
  const left = Math.max(0, Number(url.searchParams.get('exp')) - now)
  const cache = `private, max-age=${left}`
  if (request.method === 'HEAD') {
    const head = await env.PROOFS.head(key)
    if (!head) throw new HttpError(404, 'No such file.')
    return new Response(null, { headers: fileHeaders(head, cache) })
  }
  const obj = await env.PROOFS.get(key)
  if (!obj) throw new HttpError(404, 'No such file.')
  return new Response(obj.body, { headers: fileHeaders(obj, cache) })
}

/** True when an `If-None-Match` header matches `etag` (weak comparison, RFC 9110 13.1.2). */
function etagMatches(header: string | null, etag: string): boolean {
  if (!header) return false
  const bare = (t: string) => t.trim().replace(/^W\//, '')
  return header.split(',').some((t) => t.trim() === '*' || bare(t) === bare(etag))
}

/** `GET|HEAD /public/<sha256>`: a published proof, no sign-in. */
async function publicFile(request: Request, env: Env, sha256: string): Promise<Response> {
  if (!isSha256(sha256)) throw new HttpError(404, 'No such file.')
  const rows = await allVerifications(env)
  // The key must be the row's own organisation's original of this very hash: otherwise one
  // organisation could publish a row naming another's file.
  const parsed =
    rows
      .filter((v) => v.sha256 === sha256 && v.published === true && typeof v.object_key === 'string')
      .map((v) => ({ v, key: parseKey(v.object_key!) }))
      .find(({ v, key }) => key && !key.thumbnail && key.org === v.row_org_id && key.sha256 === sha256)
      ?.key ?? null
  if (!parsed) throw new HttpError(404, 'No such file.')

  const cache = 'public, max-age=3600'
  const head = await env.PROOFS.head(parsed.key)
  if (!head) throw new HttpError(404, 'No such file.')
  if (etagMatches(request.headers.get('If-None-Match'), head.httpEtag)) {
    return new Response(null, { status: 304, headers: { ETag: head.httpEtag, 'Cache-Control': cache } })
  }
  if (request.method === 'HEAD') return new Response(null, { headers: fileHeaders(head, cache) })
  const obj = await env.PROOFS.get(parsed.key)
  if (!obj) throw new HttpError(404, 'No such file.')
  return new Response(obj.body, { headers: fileHeaders(obj, cache) })
}
