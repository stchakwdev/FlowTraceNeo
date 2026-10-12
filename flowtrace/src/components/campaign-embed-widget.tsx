import { useEffect } from 'react'
import { CircleNotch, Heart, ArrowSquareOut } from '@phosphor-icons/react'
import { useCampaign, useCampaignStats } from '@/lib/queries'
import { CampaignProgressBar } from './campaign-progress-bar'
import { cn } from '@/lib/utils'

interface CampaignEmbedWidgetProps {
  campaignId: string
  theme?: 'light' | 'dark'
  showAmount?: boolean
  showDonors?: boolean
  compact?: boolean
}

export function CampaignEmbedWidget({
  campaignId,
  theme = 'light',
  showAmount = true,
  showDonors = true,
  compact = false,
}: CampaignEmbedWidgetProps) {
  const { data: campaign, isLoading, error } = useCampaign(campaignId)
  const { data: stats } = useCampaignStats(campaignId)

  // Apply theme to document for embed
  useEffect(() => {
    document.documentElement.classList.remove('light', 'dark')
    document.documentElement.classList.add(theme)
  }, [theme])

  // Send height to parent for iframe resizing
  useEffect(() => {
    const sendHeight = () => {
      const height = document.body.scrollHeight
      window.parent.postMessage({ type: 'flowtrace-embed-height', height }, '*')
    }

    sendHeight()
    const observer = new ResizeObserver(sendHeight)
    observer.observe(document.body)

    return () => observer.disconnect()
  }, [campaign])

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <CircleNotch className="h-6 w-6 animate-spin text-primary" />
      </div>
    )
  }

  if (error || !campaign) {
    return (
      <div className="p-4 text-center text-muted-foreground text-sm">
        Campaign not available
      </div>
    )
  }

  const donateUrl = campaign.public_slug
    ? `${typeof window !== 'undefined' ? window.location.origin : ''}/donate/${campaign.public_slug}`
    : null

  if (compact) {
    return (
      <div
        className={cn(
          'p-4 rounded-lg border bg-card',
          theme === 'dark' ? 'dark' : '',
        )}
      >
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
            <Heart className="h-5 w-5 text-primary" weight="fill" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-medium text-sm truncate">{campaign.name}</h3>
            <p className="text-xs text-muted-foreground">
              {Math.round((campaign.current_amount / campaign.target_amount) * 100)}% funded
            </p>
          </div>
          {donateUrl && (
            <a
              href={donateUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary font-medium hover:underline"
            >
              Donate
            </a>
          )}
        </div>
      </div>
    )
  }

  return (
    <div
      className={cn(
        'p-6 rounded-lg border bg-card',
        theme === 'dark' ? 'dark' : '',
      )}
    >
      {/* Header */}
      <div className="flex items-start gap-3 mb-4">
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary/10 shrink-0">
          <Heart className="h-6 w-6 text-primary" weight="fill" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-lg leading-tight">{campaign.name}</h3>
          {campaign.short_description && (
            <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
              {campaign.short_description}
            </p>
          )}
        </div>
      </div>

      {/* Progress */}
      {showAmount && (
        <div className="mb-4">
          <CampaignProgressBar
            current={campaign.current_amount}
            target={campaign.target_amount}
            currency={campaign.currency}
            size="sm"
          />
        </div>
      )}

      {/* Stats */}
      {showDonors && (
        <div className="grid grid-cols-3 gap-2 text-center mb-4 py-3 bg-muted/50 rounded-lg">
          <div>
            <p className="text-lg font-bold text-primary">{stats?.donation_count || 0}</p>
            <p className="text-xs text-muted-foreground">Donations</p>
          </div>
          <div>
            <p className="text-lg font-bold text-primary">{stats?.unique_donors || 0}</p>
            <p className="text-xs text-muted-foreground">Donors</p>
          </div>
          <div>
            <p className="text-lg font-bold text-primary">
              {stats?.days_remaining ?? '∞'}
            </p>
            <p className="text-xs text-muted-foreground">Days Left</p>
          </div>
        </div>
      )}

      {/* Donate Button */}
      {donateUrl && (
        <a
          href={donateUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center justify-center gap-2 w-full py-2.5 px-4 bg-primary text-primary-foreground rounded-lg font-medium text-sm hover:bg-primary/90 transition-colors"
        >
          <Heart className="h-4 w-4" weight="fill" />
          Donate Now
          <ArrowSquareOut className="h-3.5 w-3.5 ml-1" />
        </a>
      )}

      {/* Footer */}
      <div className="mt-4 pt-3 border-t text-center">
        <a
          href="https://flowtrace.app"
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          Powered by FlowTrace
        </a>
      </div>
    </div>
  )
}
