import { cn } from '@/lib/utils'

export type ProofStatus = 'verified' | 'pending' | 'flagged' | 'rejected'

export const PROOF_STATUS_LABEL: Record<ProofStatus, string> = {
  verified: 'Evidenced',
  pending: 'Awaiting proof',
  flagged: 'Flagged',
  rejected: 'Rejected',
}

// SVG stroke props for each verification state. State is carried by the
// line's pattern, never by colour alone: solid (evidenced), dashed (awaiting
// proof), long dash with a cross in flag orange (flagged), dotted (rejected).
export function proofStroke(status: ProofStatus) {
  switch (status) {
    case 'verified':
      return { strokeDasharray: undefined, className: 'stroke-foreground' }
    case 'pending':
      return { strokeDasharray: '6 5', className: 'stroke-muted-foreground' }
    case 'flagged':
      return { strokeDasharray: '14 8', className: 'stroke-flag' }
    case 'rejected':
      return { strokeDasharray: '2 6', className: 'stroke-destructive' }
  }
}

/** A short sample of the line pattern for a verification state. */
export function StatusLine({
  status,
  width = 28,
  className,
}: {
  status: ProofStatus
  width?: number
  className?: string
}) {
  const stroke = proofStroke(status)
  const mid = width / 2
  const arm = width < 26 ? 2 : 3
  return (
    <svg
      width={width}
      height="10"
      viewBox={`0 0 ${width} 10`}
      aria-hidden="true"
      className={cn('shrink-0 overflow-visible', className)}
    >
      <line
        x1="0"
        y1="5"
        x2={width}
        y2="5"
        strokeWidth="3"
        strokeDasharray={stroke.strokeDasharray}
        className={stroke.className}
      />
      {status === 'flagged' && (
        <path
          d={`M${mid - arm} ${5 - arm - 1} L${mid + arm} ${5 + arm + 1} M${mid + arm} ${5 - arm - 1} L${mid - arm} ${5 + arm + 1}`}
          strokeWidth="2"
          className="stroke-flag"
        />
      )}
    </svg>
  )
}

/** Line sample plus its label; use wherever a verification status is shown. */
export function StatusMark({
  status,
  className,
  label,
}: {
  status: ProofStatus
  className?: string
  label?: string
}) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        status === 'flagged' && 'text-flag',
        status === 'rejected' && 'text-destructive',
        status === 'pending' && 'text-muted-foreground',
        className,
      )}
    >
      <StatusLine status={status} width={22} />
      {label ?? PROOF_STATUS_LABEL[status]}
    </span>
  )
}

/** Key explaining the three line patterns. */
export function ProofLegend({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-muted-foreground',
        className,
      )}
    >
      {(['verified', 'pending', 'flagged'] as const).map((status) => (
        <span key={status} className="inline-flex items-center gap-2">
          <StatusLine status={status} />
          {PROOF_STATUS_LABEL[status]}
        </span>
      ))}
    </div>
  )
}
