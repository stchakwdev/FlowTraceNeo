// A random UUID v4 for ids minted in the browser (request ids, claim ids).
// crypto.randomUUID exists only in a secure context (HTTPS or localhost), so
// a phone opening the dev server over plain HTTP on the LAN has none, while
// crypto.getRandomValues is there in every context.
// https://developer.mozilla.org/en-US/docs/Web/API/Crypto/randomUUID
export function randomId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function')
    return crypto.randomUUID()
  const bytes = new Uint8Array(16)
  if (
    typeof crypto !== 'undefined' &&
    typeof crypto.getRandomValues === 'function'
  ) {
    crypto.getRandomValues(bytes)
  } else {
    for (let i = 0; i < bytes.length; i++)
      bytes[i] = Math.floor(Math.random() * 256)
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x40
  bytes[8] = (bytes[8] & 0x3f) | 0x80
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}
