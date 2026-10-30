import type { ChainStatus } from '@/lib/types'
import { cn } from '@/lib/utils'

export const CHAIN_STATUS_LABEL: Record<ChainStatus, string> = {
  active: 'Active',
  pending: 'Pending',
  revoked: 'Revoked',
  rejected: 'Rejected',
}

/**
 * The link between two organisations, drawn the way the report draws proof:
 * the line's pattern carries the state. Active is a solid ink line, pending
 * is dashed, revoked or rejected is broken with a gap and a cut mark.
 * Fills the width of its container.
 */
export function ChainLine({
  status,
  className,
}: {
  status: ChainStatus
  className?: string
}) {
  const broken = status === 'revoked' || status === 'rejected'
  return (
    <svg
      width="100%"
      height="12"
      aria-hidden="true"
      className={cn('block overflow-visible', className)}
    >
      {broken ? (
        <>
          <line
            x1="0"
            y1="6"
            x2="42%"
            y2="6"
            strokeWidth="3"
            className="stroke-muted-foreground"
          />
          <line
            x1="58%"
            y1="6"
            x2="100%"
            y2="6"
            strokeWidth="3"
            className="stroke-muted-foreground"
          />
          <line
            x1="47%"
            y1="0"
            x2="49%"
            y2="12"
            strokeWidth="2"
            className="stroke-muted-foreground"
          />
          <line
            x1="51%"
            y1="0"
            x2="53%"
            y2="12"
            strokeWidth="2"
            className="stroke-muted-foreground"
          />
        </>
      ) : (
        <line
          x1="0"
          y1="6"
          x2="100%"
          y2="6"
          strokeWidth="3"
          strokeDasharray={status === 'pending' ? '7 5' : undefined}
          className={
            status === 'pending'
              ? 'stroke-muted-foreground'
              : 'stroke-foreground'
          }
        />
      )}
    </svg>
  )
}

/** A short sample of the chain line plus its state in words. */
export function ChainStatusMark({
  status,
  className,
}: {
  status: ChainStatus
  className?: string
}) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        status === 'active' ? 'text-foreground' : 'text-muted-foreground',
        className,
      )}
    >
      <ChainLine status={status} className="w-6" />
      {CHAIN_STATUS_LABEL[status]}
    </span>
  )
}

/** Key for the three chain line patterns. */
export function ChainLegend({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-muted-foreground',
        className,
      )}
    >
      <span className="inline-flex items-center gap-2">
        <ChainLine status="active" className="w-7" />
        Active
      </span>
      <span className="inline-flex items-center gap-2">
        <ChainLine status="pending" className="w-7" />
        Awaiting approval
      </span>
      <span className="inline-flex items-center gap-2">
        <ChainLine status="revoked" className="w-7" />
        Revoked or rejected
      </span>
    </div>
  )
}
