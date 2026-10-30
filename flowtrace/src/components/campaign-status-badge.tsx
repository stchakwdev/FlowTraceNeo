import type { Campaign, CampaignStatus, PaymentStatus } from '@/lib/types'
import { StatusMark } from '@/components/report'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'

// Campaign state is drawn as a short line sample plus a word, like proof
// state: a solid blue line for a live campaign, solid ink once it is complete,
// dashed while it is a draft or paused, dotted when it was cancelled.
const CAMPAIGN_STATUS: Record<
  CampaignStatus,
  { label: string; dash?: string; stroke: string; text: string }
> = {
  active: { label: 'Active', stroke: 'stroke-primary', text: 'text-foreground' },
  completed: {
    label: 'Completed',
    stroke: 'stroke-foreground',
    text: 'text-foreground',
  },
  draft: {
    label: 'Draft',
    dash: '6 5',
    stroke: 'stroke-muted-foreground',
    text: 'text-muted-foreground',
  },
  paused: {
    label: 'Paused',
    dash: '6 5',
    stroke: 'stroke-muted-foreground',
    text: 'text-muted-foreground',
  },
  cancelled: {
    label: 'Cancelled',
    dash: '2 5',
    stroke: 'stroke-muted-foreground',
    text: 'text-muted-foreground',
  },
}

export function CampaignStatusMark({
  status,
  className,
}: {
  status: CampaignStatus
  className?: string
}) {
  const s = CAMPAIGN_STATUS[status]
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        s.text,
        className,
      )}
    >
      <svg width="22" height="10" viewBox="0 0 22 10" aria-hidden="true">
        <line
          x1="0"
          y1="5"
          x2="22"
          y2="5"
          strokeWidth="3"
          strokeDasharray={s.dash}
          className={s.stroke}
        />
      </svg>
      {s.label}
    </span>
  )
}

// Payment state reuses the proof line patterns: a confirmed payment is solid,
// a pending one dashed, a failed one flagged.
const PAYMENT_STATUS = {
  confirmed: { status: 'verified', label: 'Confirmed' },
  pending: { status: 'pending', label: 'Awaiting payment' },
  failed: { status: 'flagged', label: 'Failed' },
  refunded: { status: 'rejected', label: 'Refunded' },
} as const

export function PaymentStatusMark({
  status,
  className,
}: {
  status: PaymentStatus
  className?: string
}) {
  const p = PAYMENT_STATUS[status]
  return <StatusMark status={p.status} label={p.label} className={className} />
}

const DAY = 24 * 60 * 60 * 1000

/**
 * Where a campaign stands in time, stated honestly: "Ended 30 Jun 2026" once
 * the end date has passed, never "0 days left".
 */
export function campaignTiming(
  campaign: Pick<Campaign, 'ends_at' | 'starts_at' | 'status'>,
  now = Date.now(),
): { ended: boolean; label: string; daysLeft: number | null } {
  if (!campaign.ends_at) {
    return {
      ended: campaign.status === 'completed' || campaign.status === 'cancelled',
      label: 'No end date',
      daysLeft: null,
    }
  }
  const end = new Date(campaign.ends_at).getTime()
  if (end <= now) {
    return {
      ended: true,
      label: `Ended ${formatDate(campaign.ends_at)}`,
      daysLeft: null,
    }
  }
  const daysLeft = Math.ceil((end - now) / DAY)
  return {
    ended: false,
    label:
      daysLeft <= 1
        ? `Last day, ends ${formatDate(campaign.ends_at)}`
        : `${daysLeft} days left, ends ${formatDate(campaign.ends_at)}`,
    daysLeft,
  }
}
