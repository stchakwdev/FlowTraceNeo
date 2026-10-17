import { fromHex, toHex } from './http'

/** How long a signed file URL lasts. */
export const SIGNED_URL_SECONDS = 10 * 60

const enc = new TextEncoder()

async function hmacKey(secret: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    'raw',
    enc.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign', 'verify'],
  )
}

/** The signed message: `<key>\n<exp>`. */
function message(key: string, exp: number): Uint8Array {
  return enc.encode(`${key}\n${exp}`)
}

/** HMAC-SHA256 of `<key>\n<exp>` with `secret`, lowercase hex. */
export async function sign(secret: string, key: string, exp: number): Promise<string> {
  const mac = await crypto.subtle.sign('HMAC', await hmacKey(secret), message(key, exp))
  return toHex(mac)
}

export type SignatureCheck = 'ok' | 'expired' | 'bad'

/**
 * Check a signature. `crypto.subtle.verify` compares in constant time
 * (https://developers.cloudflare.com/workers/examples/signing-requests/).
 */
export async function verify(
  secret: string,
  key: string,
  exp: string | null,
  sig: string | null,
  nowSeconds: number,
): Promise<SignatureCheck> {
  if (!exp || !sig || !/^\d{1,12}$/.test(exp)) return 'bad'
  const expiry = Number(exp)
  const mac = fromHex(sig)
  if (!mac || mac.length !== 32) return 'bad'
  const ok = await crypto.subtle.verify('HMAC', await hmacKey(secret), mac, message(key, expiry))
  if (!ok) return 'bad'
  return expiry < nowSeconds ? 'expired' : 'ok'
}

/** A signed URL for `key` on `origin`, valid for `SIGNED_URL_SECONDS`. */
export async function signedUrl(
  secret: string,
  origin: string,
  key: string,
  nowSeconds: number,
): Promise<{ url: string; exp: number }> {
  const exp = nowSeconds + SIGNED_URL_SECONDS
  const sig = await sign(secret, key, exp)
  return { url: `${origin}/signed/${key}?exp=${exp}&sig=${sig}`, exp }
}
