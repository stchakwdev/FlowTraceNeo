import {
  CheckCircle,
  Circle,
  Clock,
  Pause,
  XCircle,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import type { CampaignStatus } from '@/lib/supabase'

interface CampaignStatusBadgeProps {
  status: CampaignStatus
  size?: 'sm' | 'md'
  showIcon?: boolean
}

const statusConfig: Record<
  CampaignStatus,
  {
    label: string
    icon: typeof Circle
    className: string
  }
> = {
  draft: {
    label: 'Draft',
    icon: Circle,
    className: 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300',
  },
  active: {
    label: 'Active',
    icon: Clock,
    className: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  },
  paused: {
    label: 'Paused',
    icon: Pause,
    className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  completed: {
    label: 'Completed',
    icon: CheckCircle,
    className: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  },
  cancelled: {
    label: 'Cancelled',
    icon: XCircle,
    className: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  },
}

export function CampaignStatusBadge({
  status,
  size = 'md',
  showIcon = true,
}: CampaignStatusBadgeProps) {
  const config = statusConfig[status]
  const Icon = config.icon

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-full font-medium',
        config.className,
        size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-2.5 py-1 text-xs',
      )}
    >
      {showIcon && (
        <Icon
          className={cn('shrink-0', size === 'sm' ? 'h-3 w-3' : 'h-3.5 w-3.5')}
          weight="fill"
        />
      )}
      {config.label}
    </span>
  )
}

// Payment status badge for donations
interface PaymentStatusBadgeProps {
  status: 'pending' | 'confirmed' | 'failed' | 'refunded'
  size?: 'sm' | 'md'
}

const paymentStatusConfig: Record<
  PaymentStatusBadgeProps['status'],
  {
    label: string
    className: string
  }
> = {
  pending: {
    label: 'Pending',
    className: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  },
  confirmed: {
    label: 'Confirmed',
    className: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  },
  failed: {
    label: 'Failed',
    className: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  },
  refunded: {
    label: 'Refunded',
    className: 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300',
  },
}

export function PaymentStatusBadge({ status, size = 'md' }: PaymentStatusBadgeProps) {
  const config = paymentStatusConfig[status]

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full font-medium',
        config.className,
        size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-2.5 py-1 text-xs',
      )}
    >
      {config.label}
    </span>
  )
}
