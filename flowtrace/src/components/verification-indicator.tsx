import { cn } from '@/lib/utils'
import { StatusLine } from '@/components/report'

// Summary statistics for verifications
interface VerificationStats {
  total: number
  verified: number
  pending: number
  rejected: number
  trustPoints: number
}

type BadgeSize = 'sm' | 'md' | 'lg'

interface VerificationIndicatorProps {
  stats: VerificationStats
  size?: BadgeSize
  showDetails?: boolean
  className?: string
  onClick?: () => void
}

const SIZE_CLASSES: Record<BadgeSize, string> = {
  sm: 'min-h-5 gap-1 px-1.5 text-[0.6875rem]',
  md: 'min-h-6 gap-1.5 px-2 text-xs',
  lg: 'min-h-8 gap-2 px-3 text-sm',
}

/** Overall proof state: every proof verified reads solid, otherwise dashed. */
function overallStatus(stats: VerificationStats) {
  if (stats.total === 0) return 'pending' as const
  if (stats.rejected > 0) return 'rejected' as const
  return stats.verified === stats.total
    ? ('verified' as const)
    : ('pending' as const)
}

/** Proof count for a record, with its state drawn as a line pattern. */
export function VerificationIndicator({
  stats,
  size = 'md',
  showDetails = false,
  className,
  onClick,
}: VerificationIndicatorProps) {
  const Tag = onClick ? 'button' : 'span'
  const status = overallStatus(stats)
  return (
    <Tag
      type={onClick ? 'button' : undefined}
      onClick={onClick}
      className={cn(
        'inline-flex items-center border border-border font-medium tabular-nums',
        stats.total === 0 && 'text-muted-foreground',
        onClick && 'hover:bg-muted',
        SIZE_CLASSES[size],
        className,
      )}
    >
      <StatusLine status={status} width={14} />
      {stats.total === 0
        ? 'No proof'
        : `${stats.total} ${stats.total === 1 ? 'proof' : 'proofs'}`}
      {showDetails && stats.trustPoints > 0 && (
        <span className="font-bold">+{stats.trustPoints}</span>
      )}
    </Tag>
  )
}

// Compact proof count for flow nodes.
interface NodeVerificationBadgeProps {
  count: number
  trustPoints: number
  verifiedCount?: number
  className?: string
}

export function NodeVerificationBadge({
  count,
  trustPoints,
  verifiedCount = 0,
  className,
}: NodeVerificationBadgeProps) {
  if (count === 0) return null
  const status = verifiedCount >= count ? 'verified' : 'pending'
  return (
    <span
      title={`${count} ${count === 1 ? 'proof' : 'proofs'}, ${verifiedCount} verified, +${trustPoints} trust points`}
      className={cn(
        'inline-flex shrink-0 items-center gap-1 border border-border px-1 text-[0.6875rem] font-semibold tabular-nums',
        status === 'pending' && 'text-muted-foreground',
        className,
      )}
    >
      <StatusLine status={status} width={10} />
      {count}
      {trustPoints > 0 && <span>+{trustPoints}</span>}
    </span>
  )
}

// Breakdown of proofs by state, for a node's detail panel.
interface VerificationBreakdownProps {
  stats: VerificationStats
  className?: string
}

export function VerificationBreakdown({
  stats,
  className,
}: VerificationBreakdownProps) {
  if (stats.total === 0) {
    return (
      <div className={cn('border-t border-border pt-3', className)}>
        <p className="text-sm font-semibold">Proofs</p>
        <p className="mt-1 text-sm text-muted-foreground">
          No proof is attached to this node's transactions yet.
        </p>
      </div>
    )
  }

  const pct = (n: number) => (n / stats.total) * 100
  const rate = Math.round(pct(stats.verified))

  return (
    <div className={cn('border-t border-border pt-3', className)}>
      <div className="flex items-baseline justify-between gap-3">
        <p className="text-sm font-semibold">Proofs</p>
        <p className="text-sm tabular-nums">
          <span className="font-bold">+{stats.trustPoints}</span>{' '}
          <span className="text-muted-foreground">trust points</span>
        </p>
      </div>

      <div
        role="img"
        aria-label={`${rate}% of ${stats.total} proofs verified`}
        className="mt-3 flex h-3 bg-gap"
      >
        <div
          className="bg-primary"
          style={{ width: `${pct(stats.verified)}%` }}
        />
        <div
          className="bg-report"
          style={{ width: `${pct(stats.pending)}%` }}
        />
        <div
          className="bg-destructive"
          style={{ width: `${pct(stats.rejected)}%` }}
        />
      </div>

      <dl className="mt-3 grid grid-cols-3 gap-2 text-xs">
        {(
          [
            ['verified', 'Verified', stats.verified],
            ['pending', 'Awaiting review', stats.pending],
            ['rejected', 'Rejected', stats.rejected],
          ] as const
        ).map(([status, label, n]) => (
          <div key={status} className="min-w-0">
            <dt className="flex items-center gap-1.5 text-muted-foreground">
              <StatusLine status={status} width={14} />
              {label}
            </dt>
            <dd className="mt-0.5 font-semibold tabular-nums">{n}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}

export type { VerificationStats }
