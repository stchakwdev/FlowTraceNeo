import type { Minor } from './types'

// Amounts are integer minor units. These helpers are the only place a
// decimal string or a rate turns into minor units.

export function assertMinor(n: number, what = 'Amount'): asserts n is Minor {
  if (!Number.isSafeInteger(n)) {
    throw new RangeError(`${what} must be a whole number of minor units.`)
  }
}

/** '1,250.5' -> 125050. Returns null for anything that is not money. */
export function parseMoney(raw: string): Minor | null {
  const s = raw.replace(/[,\s]/g, '').replace(/^\((.*)\)$/, '-$1')
  const m = /^(-)?(\d+)(?:\.(\d{1,2}))?$/.exec(s)
  if (!m) return null
  const minor = Number(m[2]) * 100 + Number((m[3] ?? '').padEnd(2, '0'))
  if (!Number.isSafeInteger(minor)) return null
  return m[1] ? -minor : minor
}

/** Major units as a number, for display only. */
export const toMajor = (minor: Minor) => minor / 100

/** Major units (e.g. seed data in whole shillings) to minor units. */
export function fromMajor(major: number): Minor {
  const minor = Math.round(major * 100)
  assertMinor(minor)
  return minor
}

/**
 * Converts an amount to base currency at `rate`, rounding half to even so
 * repeated conversions do not drift in one direction.
 */
export function convert(amount: Minor, rate: number): Minor {
  if (!(rate > 0) || !Number.isFinite(rate)) {
    throw new RangeError('An exchange rate must be a positive number.')
  }
  if (rate === 1) return amount
  const exact = amount * rate
  const floor = Math.floor(exact)
  const diff = exact - floor
  const eps = 1e-9
  let out: number
  if (diff > 0.5 + eps) out = floor + 1
  else if (diff < 0.5 - eps) out = floor
  else out = floor % 2 === 0 ? floor : floor + 1
  assertMinor(out)
  return out
}

export const sum = (xs: Iterable<Minor>) => {
  let t = 0
  for (const x of xs) t += x
  return t
}
