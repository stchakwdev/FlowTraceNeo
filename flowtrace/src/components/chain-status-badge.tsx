import {
  Clock,
  CheckCircle,
  XCircle,
  Prohibit,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import type { ChainStatus } from '@/lib/supabase'

interface ChainStatusBadgeProps {
  status: ChainStatus
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

const STATUS_CONFIG: Record<
  ChainStatus,
  {
    icon: React.ElementType
    label: string
    className: string
  }
> = {
  pending: {
    icon: Clock,
    label: 'Pending',
    className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  active: {
    icon: CheckCircle,
    label: 'Active',
    className: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  },
  rejected: {
    icon: XCircle,
    label: 'Rejected',
    className: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  },
  revoked: {
    icon: Prohibit,
    label: 'Revoked',
    className: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400',
  },
}

const SIZE_CLASSES = {
  sm: {
    container: 'px-1.5 py-0.5 gap-1 text-xs',
    icon: 'h-3 w-3',
  },
  md: {
    container: 'px-2 py-1 gap-1.5 text-sm',
    icon: 'h-3.5 w-3.5',
  },
  lg: {
    container: 'px-3 py-1.5 gap-2 text-sm',
    icon: 'h-4 w-4',
  },
}

export function ChainStatusBadge({
  status,
  size = 'md',
  className,
}: ChainStatusBadgeProps) {
  const config = STATUS_CONFIG[status]
  const sizeClasses = SIZE_CLASSES[size]
  const Icon = config.icon

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full font-medium',
        sizeClasses.container,
        config.className,
        className,
      )}
    >
      <Icon className={sizeClasses.icon} weight="fill" />
      {config.label}
    </span>
  )
}

// Compact version showing just the icon with tooltip
export function ChainStatusIcon({
  status,
  size = 'md',
  className,
}: ChainStatusBadgeProps) {
  const config = STATUS_CONFIG[status]
  const sizeClasses = SIZE_CLASSES[size]
  const Icon = config.icon

  return (
    <span
      className={cn(
        'inline-flex items-center justify-center rounded-full',
        size === 'sm' ? 'h-5 w-5' : size === 'md' ? 'h-6 w-6' : 'h-7 w-7',
        config.className,
        className,
      )}
      title={config.label}
    >
      <Icon className={sizeClasses.icon} weight="fill" />
    </span>
  )
}
