import { Link } from '@tanstack/react-router'
import {
  Calendar,
  DotsThree,
  Eye,
  PencilSimple,
  Target,
  Trash,
  Users,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { CampaignProgressCompact } from './campaign-progress-bar'
import { CampaignStatusBadge } from './campaign-status-badge'
import type { CampaignWithDetails } from '@/lib/supabase'

interface CampaignCardProps {
  campaign: CampaignWithDetails
  onEdit?: () => void
  onDelete?: () => void
}

export function CampaignCard({ campaign, onEdit, onDelete }: CampaignCardProps) {
  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency: campaign.currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  const getDaysRemaining = () => {
    if (!campaign.ends_at) return null
    const now = new Date()
    const end = new Date(campaign.ends_at)
    const diff = Math.ceil((end.getTime() - now.getTime()) / (1000 * 60 * 60 * 24))
    return diff > 0 ? diff : 0
  }

  const daysRemaining = getDaysRemaining()

  return (
    <Card className="group overflow-hidden transition-shadow hover:shadow-md">
      {/* Campaign Image */}
      {campaign.image_url && (
        <div className="relative h-40 w-full overflow-hidden bg-muted">
          <img
            src={campaign.image_url}
            alt={campaign.name}
            className="h-full w-full object-cover transition-transform group-hover:scale-105"
          />
          <div className="absolute inset-0 bg-gradient-to-t from-black/60 to-transparent" />
          <div className="absolute bottom-3 left-3">
            <CampaignStatusBadge status={campaign.status} size="sm" />
          </div>
        </div>
      )}

      <CardContent className={cn('p-4', !campaign.image_url && 'pt-4')}>
        {/* Header */}
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            {!campaign.image_url && (
              <div className="mb-2">
                <CampaignStatusBadge status={campaign.status} size="sm" />
              </div>
            )}
            <Link
              to="/campaigns/$id"
              params={{ id: campaign.id }}
              className="block"
            >
              <h3 className="font-semibold text-foreground line-clamp-2 hover:text-primary transition-colors">
                {campaign.name}
              </h3>
            </Link>
            {campaign.short_description && (
              <p className="mt-1 text-sm text-muted-foreground line-clamp-2">
                {campaign.short_description}
              </p>
            )}
          </div>

          <DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon" className="shrink-0 -mr-2" />}>
              <DotsThree className="h-5 w-5" weight="bold" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                render={<Link to="/campaigns/$id" params={{ id: campaign.id }} />}
              >
                <Eye className="h-4 w-4 mr-2" />
                View Details
              </DropdownMenuItem>
              {onEdit && (
                <DropdownMenuItem onClick={onEdit}>
                  <PencilSimple className="h-4 w-4 mr-2" />
                  Edit Campaign
                </DropdownMenuItem>
              )}
              {onDelete && campaign.status === 'draft' && (
                <DropdownMenuItem onClick={onDelete} className="text-destructive">
                  <Trash className="h-4 w-4 mr-2" />
                  Delete Campaign
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>

        {/* Progress */}
        <div className="mt-4">
          <div className="flex items-baseline justify-between text-sm mb-1.5">
            <span className="font-semibold">{formatAmount(campaign.current_amount)}</span>
            <span className="text-muted-foreground">of {formatAmount(campaign.target_amount)}</span>
          </div>
          <CampaignProgressCompact
            current={campaign.current_amount}
            target={campaign.target_amount}
          />
        </div>

        {/* Stats */}
        <div className="mt-4 flex items-center gap-4 text-xs text-muted-foreground">
          {campaign.target_node && (
            <span className="flex items-center gap-1">
              <Target className="h-3.5 w-3.5" />
              {campaign.target_node.name}
            </span>
          )}
          {campaign.stats?.unique_donors !== undefined && campaign.stats.unique_donors > 0 && (
            <span className="flex items-center gap-1">
              <Users className="h-3.5 w-3.5" />
              {campaign.stats.unique_donors} donors
            </span>
          )}
          {daysRemaining !== null && campaign.status === 'active' && (
            <span className="flex items-center gap-1">
              <Calendar className="h-3.5 w-3.5" />
              {daysRemaining === 0 ? 'Ends today' : `${daysRemaining} days left`}
            </span>
          )}
        </div>

        {/* Category Tag */}
        {campaign.category && (
          <div className="mt-3">
            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-muted text-muted-foreground capitalize">
              {campaign.category}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

// Compact version for dashboard widgets
export function CampaignCardCompact({
  campaign,
}: {
  campaign: CampaignWithDetails
}) {
  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency: campaign.currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  return (
    <Link
      to="/campaigns/$id"
      params={{ id: campaign.id }}
      className="block p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
    >
      <div className="flex items-center justify-between gap-2 mb-2">
        <h4 className="font-medium text-sm truncate">{campaign.name}</h4>
        <CampaignStatusBadge status={campaign.status} size="sm" showIcon={false} />
      </div>
      <div className="flex items-baseline justify-between text-xs mb-1">
        <span className="font-medium">{formatAmount(campaign.current_amount)}</span>
        <span className="text-muted-foreground">{formatAmount(campaign.target_amount)}</span>
      </div>
      <CampaignProgressCompact
        current={campaign.current_amount}
        target={campaign.target_amount}
      />
    </Link>
  )
}
