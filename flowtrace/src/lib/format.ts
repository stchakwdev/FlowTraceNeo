// Shared display formatting. Dates from the database are calendar dates
// ('2026-01-18') or UTC timestamps; formatting in UTC keeps a date-only value
// from showing a day early in timezones west of UTC.

type DateInput = string | number | Date

function toDate(value: DateInput) {
  return value instanceof Date ? value : new Date(value)
}

/** 18 Jan 2026 */
export function formatDate(value: DateInput) {
  return toDate(value).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  })
}

/** 18 Jan */
export function formatShortDate(value: DateInput) {
  return toDate(value).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
  })
}

/** 18 Jan 2026, 14:05 (for timestamps, shown in the viewer's local time) */
export function formatDateTime(value: DateInput) {
  return toDate(value).toLocaleString('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

/** 2,500,000 */
export function formatAmount(amount: number, fractionDigits = 0) {
  return amount.toLocaleString('en-KE', {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  })
}

/** KES 2,500,000 */
export function formatMoney(amount: number, currency = 'KES') {
  return `${currency} ${formatAmount(amount)}`
}

/** 15.7M, 850K, 920 */
export function formatCompact(amount: number) {
  if (Math.abs(amount) >= 1_000_000) {
    return `${(amount / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
  }
  if (Math.abs(amount) >= 10_000) return `${Math.round(amount / 1_000)}K`
  return formatAmount(amount)
}

/**
 * 18 Jan 2026, 07:00 EAT. Timestamps shown in East Africa Time (UTC+3, no
 * daylight saving) with the zone named, so a record reads the same in Nairobi
 * and on a server in UTC.
 */
export function formatDateTimeEAT(value: DateInput) {
  const text = toDate(value).toLocaleString('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    timeZone: 'Africa/Nairobi',
  })
  return `${text} EAT`
}
