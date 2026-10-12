import { CoverageBar } from '@/components/report'
import { formatMoney } from '@/lib/format'
import { cn } from '@/lib/utils'

interface CampaignCoverageProps {
  raised: number
  target: number
  /** Spending backed by verified proof, drawn as the darker share of the bar. */
  evidenced?: number | null
  currency?: string
  size?: 'sm' | 'md' | 'lg'
  /** Show the key under the bar. */
  showKey?: boolean
  className?: string
}

/**
 * Campaign funding coverage: raised against target, with the share already
 * spent with proof drawn darker. Figures are tabular and carry their currency.
 */
export function CampaignCoverage({
  raised,
  target,
  evidenced,
  currency = 'KES',
  size = 'md',
  showKey = true,
  className,
}: CampaignCoverageProps) {
  const pct = target > 0 ? Math.round((raised / target) * 100) : 0
  const over = raised - target
  const remaining = Math.max(0, target - raised)
  const hasEvidence = evidenced !== null && evidenced !== undefined

  return (
    <div className={cn('w-full', className)}>
      <div className="flex flex-wrap items-end justify-between gap-x-4 gap-y-1">
        <p className="min-w-0">
          <span
            className={cn(
              'figure block text-foreground',
              size === 'sm' && 'text-2xl',
              size === 'md' && 'text-3xl',
              size === 'lg' && 'text-4xl sm:text-5xl',
            )}
          >
            {formatMoney(raised, currency)}
          </span>
          <span className="mt-1.5 block text-sm text-muted-foreground">
            raised of{' '}
            <span className="font-semibold text-foreground tabular-nums">
              {formatMoney(target, currency)}
            </span>{' '}
            target
          </span>
        </p>
        <p
          className={cn(
            'figure text-foreground',
            size === 'lg' ? 'text-3xl' : 'text-xl',
          )}
        >
          {pct}%
        </p>
      </div>

      <CoverageBar
        total={Math.max(target, raised)}
        filled={raised}
        evidenced={hasEvidence ? evidenced : undefined}
        label={`${formatMoney(raised, currency)} raised of ${formatMoney(target, currency)}, ${pct}%${hasEvidence ? `; ${formatMoney(evidenced, currency)} spent with proof` : ''}`}
        className={cn('mt-3', size === 'sm' && 'h-3')}
      />

      {showKey && (
        <dl className="mt-3 flex flex-wrap gap-x-5 gap-y-1.5 text-xs text-muted-foreground">
          {hasEvidence && (
            <KeyItem swatch="bg-primary" label="Spent with proof">
              {formatMoney(evidenced, currency)}
            </KeyItem>
          )}
          <KeyItem
            swatch="bg-report"
            label={hasEvidence ? 'Raised, not yet spent with proof' : 'Raised'}
          >
            {formatMoney(
              hasEvidence ? Math.max(0, raised - evidenced) : raised,
              currency,
            )}
          </KeyItem>
          {over > 0 ? (
            <KeyItem swatch="bg-report" label="Over target">
              {formatMoney(over, currency)}
            </KeyItem>
          ) : (
            <KeyItem swatch="bg-gap" label="Still to raise">
              {formatMoney(remaining, currency)}
            </KeyItem>
          )}
        </dl>
      )}
    </div>
  )
}

function KeyItem({
  swatch,
  label,
  children,
}: {
  swatch: string
  label: string
  children: React.ReactNode
}) {
  return (
    <div className="inline-flex items-center gap-1.5">
      <span aria-hidden="true" className={cn('size-2.5 shrink-0', swatch)} />
      <dt>{label}</dt>
      <dd className="font-semibold text-foreground tabular-nums">{children}</dd>
    </div>
  )
}
