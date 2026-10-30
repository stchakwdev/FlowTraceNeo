import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowSquareOut } from '@phosphor-icons/react'
import { campaignTiming } from './campaign-status-badge'
import { FlowTraceMark } from '@/components/layout/wordmark'
import { CoverageBar } from '@/components/report'
import { buttonVariants } from '@/components/ui/button'
import { getPublicCampaignReport } from '@/lib/data'
import { formatMoney } from '@/lib/format'
import { cn } from '@/lib/utils'

interface CampaignEmbedWidgetProps {
  campaignId: string
  theme?: 'light' | 'dark'
  showAmount?: boolean
  showDonors?: boolean
  compact?: boolean
}

/**
 * A campaign's coverage for embedding on another site: raised against
 * target, the share spent with proof, and a link to give. Reads public data
 * only, so it works without a signed-in organisation.
 */
export function CampaignEmbedWidget({
  campaignId,
  theme = 'light',
  showAmount = true,
  showDonors = true,
  compact = false,
}: CampaignEmbedWidgetProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['campaign-report', campaignId, 'embed'],
    queryFn: () => getPublicCampaignReport(campaignId),
    enabled: !!campaignId,
    staleTime: 60 * 1000,
  })

  useEffect(() => {
    document.documentElement.classList.remove('light', 'dark')
    document.documentElement.classList.add(theme)
  }, [theme])

  // Tell the host page our height so it can size the iframe.
  useEffect(() => {
    const send = () =>
      window.parent.postMessage(
        { type: 'flowtrace-embed-height', height: document.body.scrollHeight },
        '*',
      )
    send()
    const observer = new ResizeObserver(send)
    observer.observe(document.body)
    return () => observer.disconnect()
  }, [data])

  const frame = 'rule-heavy border-x border-b border-border bg-card text-card-foreground'

  if (isLoading) {
    return (
      <div className={cn(frame, 'p-4 text-sm text-muted-foreground')} role="status">
        Loading campaign
      </div>
    )
  }

  if (error || !data) {
    return (
      <div className={cn(frame, 'p-4 text-sm')} role="alert">
        <p className="font-semibold">This campaign is not available</p>
        <p className="mt-1 text-xs text-muted-foreground">
          It may have closed, or the embed code points to a campaign that was
          removed.
        </p>
      </div>
    )
  }

  const { campaign, organization, evidenced_amount: evidenced } = data
  const pct =
    campaign.target_amount > 0
      ? Math.round((campaign.current_amount / campaign.target_amount) * 100)
      : 0
  const timing = campaignTiming(campaign)
  const open = campaign.status === 'active' && !timing.ended
  const donateUrl =
    campaign.public_slug && campaign.is_public
      ? `${typeof window !== 'undefined' ? window.location.origin : ''}/donate/${campaign.public_slug}`
      : null

  const donateLink = donateUrl && open && (
    <a
      href={donateUrl}
      target="_blank"
      rel="noopener noreferrer"
      className={buttonVariants({
        size: compact ? 'default' : 'lg',
        className: cn('h-10', !compact && 'w-full'),
      })}
    >
      Give
      <ArrowSquareOut />
    </a>
  )

  if (compact) {
    return (
      <div className={cn(frame, 'p-3')}>
        <div className="flex items-center gap-3">
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold">{campaign.name}</p>
            <p className="text-xs text-muted-foreground tabular-nums">
              {formatMoney(campaign.current_amount, campaign.currency)} · {pct}% of
              target
            </p>
          </div>
          {donateLink}
        </div>
        <CoverageBar
          total={Math.max(campaign.target_amount, campaign.current_amount)}
          filled={campaign.current_amount}
          evidenced={evidenced ?? undefined}
          className="mt-2 h-2"
          label={`${pct}% of target raised`}
        />
      </div>
    )
  }

  return (
    <div className={cn(frame, 'p-4')}>
      <p className="text-xs text-muted-foreground">{organization.name}</p>
      <h1 className="mt-0.5 text-2xl leading-tight">{campaign.name}</h1>
      {campaign.short_description && (
        <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
          {campaign.short_description}
        </p>
      )}

      {showAmount && (
        <div className="mt-4">
          <div className="flex items-baseline justify-between gap-3">
            <span className="figure text-3xl">
              {formatMoney(campaign.current_amount, campaign.currency)}
            </span>
            <span className="figure text-xl">{pct}%</span>
          </div>
          <p className="mt-1 text-xs text-muted-foreground tabular-nums">
            raised of {formatMoney(campaign.target_amount, campaign.currency)}
          </p>
          <CoverageBar
            total={Math.max(campaign.target_amount, campaign.current_amount)}
            filled={campaign.current_amount}
            evidenced={evidenced ?? undefined}
            className="mt-2 h-3"
            label={`${pct}% of target raised`}
          />
          {evidenced !== null && (
            <p className="mt-1.5 flex items-center gap-1.5 text-xs text-muted-foreground">
              <span aria-hidden="true" className="size-2.5 bg-primary" />
              <span className="font-semibold text-foreground tabular-nums">
                {formatMoney(evidenced, campaign.currency)}
              </span>
              spent with verified proof
            </p>
          )}
        </div>
      )}

      {showDonors && (
        <dl className="mt-4 grid grid-cols-2 border-y border-border text-sm">
          <div className="py-2 pr-3">
            <dt className="text-xs text-muted-foreground">Trust Score</dt>
            <dd className="figure text-xl">
              {organization.trust_score}
              <span className="text-sm text-muted-foreground">/100</span>
            </dd>
          </div>
          <div className="border-l border-border py-2 pl-3">
            <dt className="text-xs text-muted-foreground">Timing</dt>
            <dd className="text-sm font-semibold">{timing.label.split(',')[0]}</dd>
          </div>
        </dl>
      )}

      {donateLink && <div className="mt-4">{donateLink}</div>}

      <a
        href={typeof window !== 'undefined' ? window.location.origin : '/'}
        target="_blank"
        rel="noopener noreferrer"
        className="mt-3 inline-flex min-h-8 items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
      >
        <FlowTraceMark className="text-foreground" />
        Figures from FlowTrace
      </a>
    </div>
  )
}
