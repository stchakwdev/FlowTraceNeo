import {
  Shield,
  CheckCircle,
  Clock,
  XCircle,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'

// Summary statistics for verifications
interface VerificationStats {
  total: number
  verified: number
  pending: number
  rejected: number
  trustPoints: number
}

// Badge sizes
type BadgeSize = 'sm' | 'md' | 'lg'

interface VerificationIndicatorProps {
  stats: VerificationStats
  size?: BadgeSize
  showDetails?: boolean
  className?: string
  onClick?: () => void
}

const SIZE_CLASSES: Record<BadgeSize, { container: string; icon: string; text: string }> = {
  sm: {
    container: 'px-1.5 py-0.5 gap-1',
    icon: 'h-3 w-3',
    text: 'text-[10px]',
  },
  md: {
    container: 'px-2 py-1 gap-1.5',
    icon: 'h-3.5 w-3.5',
    text: 'text-xs',
  },
  lg: {
    container: 'px-3 py-1.5 gap-2',
    icon: 'h-4 w-4',
    text: 'text-sm',
  },
}

export function VerificationIndicator({
  stats,
  size = 'md',
  showDetails = false,
  className,
  onClick,
}: VerificationIndicatorProps) {
  const sizeClasses = SIZE_CLASSES[size]

  if (stats.total === 0) {
    return (
      <div
        className={cn(
          'inline-flex items-center rounded-full',
          'bg-slate-100 dark:bg-slate-800/50 text-slate-500 dark:text-slate-400',
          sizeClasses.container,
          onClick && 'cursor-pointer hover:bg-slate-200 dark:hover:bg-slate-700/50 transition-colors',
          className,
        )}
        onClick={onClick}
      >
        <Shield className={sizeClasses.icon} />
        <span className={cn('font-medium', sizeClasses.text)}>No proofs</span>
      </div>
    )
  }

  // Determine overall status color
  const verificationRate = stats.verified / stats.total
  const statusColor =
    verificationRate >= 0.8
      ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-400'
      : verificationRate >= 0.5
        ? 'bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400'
        : 'bg-slate-100 dark:bg-slate-800/50 text-slate-600 dark:text-slate-400'

  return (
    <div
      className={cn(
        'inline-flex items-center rounded-full',
        statusColor,
        sizeClasses.container,
        onClick && 'cursor-pointer hover:opacity-80 transition-opacity',
        className,
      )}
      onClick={onClick}
    >
      <Shield className={sizeClasses.icon} weight="fill" />
      <span className={cn('font-medium', sizeClasses.text)}>
        {stats.total} proof{stats.total !== 1 ? 's' : ''}
      </span>
      {showDetails && stats.trustPoints > 0 && (
        <span className={cn('font-bold', sizeClasses.text)}>+{stats.trustPoints}</span>
      )}
    </div>
  )
}

// Compact inline badge for flow nodes
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

  const verificationRate = count > 0 ? verifiedCount / count : 0
  const bgColor =
    verificationRate >= 0.8
      ? 'bg-emerald-500/20 border-emerald-500/30'
      : verificationRate >= 0.5
        ? 'bg-amber-500/20 border-amber-500/30'
        : 'bg-slate-500/20 border-slate-500/30'
  const textColor =
    verificationRate >= 0.8
      ? 'text-emerald-400'
      : verificationRate >= 0.5
        ? 'text-amber-400'
        : 'text-slate-400'

  return (
    <div
      className={cn(
        'inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full',
        'border text-[10px] font-medium',
        bgColor,
        textColor,
        className,
      )}
    >
      <Shield className="h-3 w-3" weight="fill" />
      <span>{count}</span>
      {trustPoints > 0 && <span className="font-bold">+{trustPoints}</span>}
    </div>
  )
}

// Detailed breakdown component for node details panel
interface VerificationBreakdownProps {
  stats: VerificationStats
  className?: string
}

export function VerificationBreakdown({ stats, className }: VerificationBreakdownProps) {
  if (stats.total === 0) {
    return (
      <div className={cn('rounded-xl bg-slate-800/30 p-4 ring-1 ring-slate-700/30', className)}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-slate-500" />
            <span className="text-sm font-medium text-slate-400">Verifications</span>
          </div>
        </div>
        <p className="mt-2 text-sm text-slate-500">No verifications attached to transactions</p>
      </div>
    )
  }

  const verificationRate = Math.round((stats.verified / stats.total) * 100)

  return (
    <div className={cn('rounded-xl bg-slate-800/30 p-4 ring-1 ring-slate-700/30', className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Shield className="h-4 w-4 text-emerald-400" weight="fill" />
          <span className="text-sm font-medium text-slate-200">Verifications</span>
        </div>
        <div className="flex items-center gap-1 text-emerald-400">
          <span className="text-sm font-bold">+{stats.trustPoints}</span>
          <span className="text-xs text-slate-500">pts</span>
        </div>
      </div>

      {/* Progress bar */}
      <div className="mt-3">
        <div className="flex justify-between text-xs mb-1">
          <span className="text-slate-400">{verificationRate}% verified</span>
          <span className="text-slate-500">{stats.total} total</span>
        </div>
        <div className="h-2 bg-slate-700/50 rounded-full overflow-hidden">
          <div className="flex h-full">
            {stats.verified > 0 && (
              <div
                className="bg-emerald-500 transition-all"
                style={{ width: `${(stats.verified / stats.total) * 100}%` }}
              />
            )}
            {stats.pending > 0 && (
              <div
                className="bg-amber-500 transition-all"
                style={{ width: `${(stats.pending / stats.total) * 100}%` }}
              />
            )}
            {stats.rejected > 0 && (
              <div
                className="bg-red-500 transition-all"
                style={{ width: `${(stats.rejected / stats.total) * 100}%` }}
              />
            )}
          </div>
        </div>
      </div>

      {/* Status counts */}
      <div className="mt-3 grid grid-cols-3 gap-2">
        <div className="flex items-center gap-1.5 text-xs">
          <CheckCircle className="h-3.5 w-3.5 text-emerald-400" weight="fill" />
          <span className="text-slate-300">{stats.verified}</span>
          <span className="text-slate-500">verified</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <Clock className="h-3.5 w-3.5 text-amber-400" weight="fill" />
          <span className="text-slate-300">{stats.pending}</span>
          <span className="text-slate-500">pending</span>
        </div>
        <div className="flex items-center gap-1.5 text-xs">
          <XCircle className="h-3.5 w-3.5 text-red-400" weight="fill" />
          <span className="text-slate-300">{stats.rejected}</span>
          <span className="text-slate-500">rejected</span>
        </div>
      </div>
    </div>
  )
}

// Export types for external use
export type { VerificationStats }
