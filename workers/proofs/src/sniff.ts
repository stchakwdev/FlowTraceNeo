/**
 * Decide a file's type from its first bytes, never from the declared type. Signatures:
 * https://en.wikipedia.org/wiki/List_of_file_signatures and, for HEIF, ISO/IEC 23008-12
 * (an ISO-BMFF `ftyp` box whose brand names the HEIF family).
 */
export type ProofType =
  | 'image/jpeg'
  | 'image/png'
  | 'image/webp'
  | 'image/gif'
  | 'image/heic'
  | 'image/heif'
  | 'application/pdf'

const HEIC_BRANDS = new Set(['heic', 'heix', 'hevc', 'hevx', 'heim', 'heis'])
const HEIF_BRANDS = new Set(['mif1', 'msf1'])

function startsWith(b: Uint8Array, sig: number[], at = 0): boolean {
  if (b.length < at + sig.length) return false
  return sig.every((x, i) => b[at + i] === x)
}

function ascii(b: Uint8Array, from: number, to: number): string {
  return String.fromCharCode(...b.subarray(from, to))
}

/** The proof type of `bytes`, or null when it is not an allowed image or PDF. */
export function sniff(bytes: Uint8Array): ProofType | null {
  if (startsWith(bytes, [0xff, 0xd8, 0xff])) return 'image/jpeg'
  if (startsWith(bytes, [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) return 'image/png'
  if (bytes.length >= 6 && /^GIF8[79]a$/.test(ascii(bytes, 0, 6))) return 'image/gif'
  if (bytes.length >= 12 && ascii(bytes, 0, 4) === 'RIFF' && ascii(bytes, 8, 12) === 'WEBP') {
    return 'image/webp'
  }
  if (bytes.length >= 12 && ascii(bytes, 4, 8) === 'ftyp') {
    const brand = ascii(bytes, 8, 12)
    if (HEIC_BRANDS.has(brand)) return 'image/heic'
    if (HEIF_BRANDS.has(brand)) return 'image/heif'
  }
  if (bytes.length >= 5 && ascii(bytes, 0, 5) === '%PDF-') return 'application/pdf'
  return null
}

/** True for the image types (everything allowed except PDF). */
export function isImage(t: ProofType): boolean {
  return t !== 'application/pdf'
}
