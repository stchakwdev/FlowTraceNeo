import { cn } from '@/lib/utils'

interface CampaignProgressBarProps {
  current: number
  target: number
  currency?: string
  showLabels?: boolean
  showPercentage?: boolean
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

export function CampaignProgressBar({
  current,
  target,
  currency = 'KES',
  showLabels = true,
  showPercentage = true,
  size = 'md',
  className,
}: CampaignProgressBarProps) {
  const percentage = target > 0 ? Math.min((current / target) * 100, 100) : 0
  const isOverfunded = current > target
  const overfundPercentage = target > 0 ? ((current - target) / target) * 100 : 0

  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  const sizeClasses = {
    sm: 'h-1.5',
    md: 'h-2.5',
    lg: 'h-4',
  }

  return (
    <div className={cn('w-full', className)}>
      {/* Labels */}
      {showLabels && (
        <div className="mb-1.5 flex items-baseline justify-between text-sm">
          <span className="font-semibold text-foreground">
            {formatAmount(current)}
          </span>
          <span className="text-muted-foreground">
            of {formatAmount(target)}
          </span>
        </div>
      )}

      {/* Progress bar */}
      <div className={cn('relative w-full overflow-hidden rounded-full bg-muted', sizeClasses[size])}>
        {/* Main progress */}
        <div
          className={cn(
            'h-full rounded-full transition-all duration-500 ease-out',
            isOverfunded ? 'bg-emerald-500' : 'bg-primary',
          )}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />

        {/* Overfunded indicator */}
        {isOverfunded && (
          <div
            className="absolute right-0 top-0 h-full bg-emerald-400/50"
            style={{ width: `${Math.min(overfundPercentage, 50)}%` }}
          />
        )}
      </div>

      {/* Percentage */}
      {showPercentage && (
        <div className="mt-1 flex items-center justify-between text-xs">
          <span
            className={cn(
              'font-medium',
              percentage >= 100
                ? 'text-emerald-600 dark:text-emerald-400'
                : percentage >= 75
                  ? 'text-primary'
                  : percentage >= 50
                    ? 'text-amber-600 dark:text-amber-400'
                    : 'text-muted-foreground',
            )}
          >
            {percentage.toFixed(0)}% funded
          </span>
          {isOverfunded && (
            <span className="text-emerald-600 dark:text-emerald-400">
              +{overfundPercentage.toFixed(0)}% overflow
            </span>
          )}
        </div>
      )}
    </div>
  )
}

// Compact variant for use in cards/lists
export function CampaignProgressCompact({
  current,
  target,
  className,
}: {
  current: number
  target: number
  className?: string
}) {
  const percentage = target > 0 ? Math.min((current / target) * 100, 100) : 0

  return (
    <div className={cn('flex items-center gap-2', className)}>
      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
        <div
          className={cn(
            'h-full rounded-full transition-all duration-300',
            percentage >= 100 ? 'bg-emerald-500' : 'bg-primary',
          )}
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span
        className={cn(
          'text-xs font-medium tabular-nums',
          percentage >= 100
            ? 'text-emerald-600 dark:text-emerald-400'
            : 'text-muted-foreground',
        )}
      >
        {percentage.toFixed(0)}%
      </span>
    </div>
  )
}
