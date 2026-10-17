import { splitList } from './env'
import type { Env } from './env'

/** An error that becomes a JSON response `{ "error": message }` with `status`. */
export class HttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
  }
}

/** A JSON response. */
export function json(body: unknown, status = 200, headers: HeadersInit = {}): Response {
  const h = new Headers(headers)
  h.set('Content-Type', 'application/json; charset=utf-8')
  h.set('Cache-Control', h.get('Cache-Control') ?? 'no-store')
  return new Response(JSON.stringify(body), { status, headers: h })
}

/** A JSON error response. */
export function errorResponse(status: number, message: string): Response {
  return json({ error: message }, status)
}

/** The request's `Origin` when it is in `ALLOWED_ORIGINS` (exact match), else null. */
export function allowedOrigin(env: Env, request: Request): string | null {
  const origin = request.headers.get('Origin')
  if (!origin) return null
  return splitList(env.ALLOWED_ORIGINS).includes(origin) ? origin : null
}

/** Add CORS headers for an allowed origin; responses always vary on Origin. */
export function withCors(env: Env, request: Request, response: Response): Response {
  const origin = allowedOrigin(env, request)
  // Responses from R2 or `Response.redirect` can have immutable headers; copy to be safe.
  const out = new Response(response.body, response)
  out.headers.append('Vary', 'Origin')
  if (origin) {
    out.headers.set('Access-Control-Allow-Origin', origin)
    out.headers.set('Access-Control-Expose-Headers', 'ETag, Content-Type, Content-Length')
  }
  return out
}

/**
 * CORS preflight. https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS#preflighted_requests
 */
export function preflight(env: Env, request: Request): Response {
  const origin = allowedOrigin(env, request)
  if (!origin) return errorResponse(403, 'This website is not allowed to use the proofs service.')
  return new Response(null, {
    status: 204,
    headers: {
      'Access-Control-Allow-Origin': origin,
      'Access-Control-Allow-Methods': 'GET, POST',
      'Access-Control-Allow-Headers': 'Authorization, Content-Type',
      'Access-Control-Max-Age': '86400',
      Vary: 'Origin',
    },
  })
}

/** Lowercase hex of some bytes. */
export function toHex(bytes: ArrayBuffer | Uint8Array): string {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
  let s = ''
  for (const b of u8) s += b.toString(16).padStart(2, '0')
  return s
}

/** Bytes of an even-length hex string, or null if it is not hex. */
export function fromHex(hex: string): Uint8Array | null {
  if (hex.length % 2 !== 0 || !/^[0-9a-fA-F]*$/.test(hex)) return null
  const out = new Uint8Array(hex.length / 2)
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16)
  return out
}

/** SHA-256 as lowercase hex (Web Crypto, available in Workers and bun). */
export async function sha256Hex(data: ArrayBuffer | Uint8Array | string): Promise<string> {
  const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : data
  return toHex(await crypto.subtle.digest('SHA-256', bytes))
}
