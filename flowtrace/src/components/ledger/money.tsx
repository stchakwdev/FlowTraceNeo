import type { Minor } from '@/lib/ledger'
import { cn } from '@/lib/utils'

/**
 * 1,250.50 from 125050 minor units. Accounting style: negatives in
 * parentheses, zero as an en dash so empty cells read as empty.
 */
export function formatMinor(
  minor: Minor,
  opts: { decimals?: 0 | 2; zero?: string; signed?: boolean } = {},
) {
  const { decimals = 2, zero = '–', signed = false } = opts
  if (minor === 0) return zero
  const abs = Math.abs(minor) / 100
  const text = abs.toLocaleString('en-KE', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  })
  if (minor < 0) return signed ? `−${text}` : `(${text})`
  return text
}

/** Compact figure for headline numbers: 15.7M, 850K. */
export function formatMinorCompact(minor: Minor) {
  const major = minor / 100
  const a = Math.abs(major)
  const sign = major < 0 ? '−' : ''
  if (a >= 1_000_000) return `${sign}${(a / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
  if (a >= 10_000) return `${sign}${Math.round(a / 1_000)}K`
  return `${sign}${Math.round(a).toLocaleString('en-KE')}`
}

/** A tabular amount, with an optional muted currency code before it. */
export function Money({
  value,
  currency,
  decimals,
  zero,
  signed,
  strong,
  className,
}: {
  value: Minor
  currency?: string
  decimals?: 0 | 2
  zero?: string
  signed?: boolean
  strong?: boolean
  className?: string
}) {
  return (
    <span className={cn('whitespace-nowrap tabular-nums', className)}>
      {currency && (
        <span className="mr-1 text-xs font-normal text-muted-foreground">
          {currency}
        </span>
      )}
      <span className={cn(strong && 'font-semibold', value < 0 && !signed && 'text-foreground')}>
        {formatMinor(value, { decimals, zero, signed })}
      </span>
    </span>
  )
}
