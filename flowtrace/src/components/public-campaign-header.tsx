import { Link } from '@tanstack/react-router'
import { FlowArrow, Buildings, Calendar, MapPin } from '@phosphor-icons/react'
import type { CampaignWithDetails } from '@/lib/supabase'

interface PublicCampaignHeaderProps {
  campaign: CampaignWithDetails
}

export function PublicCampaignHeader({ campaign }: PublicCampaignHeaderProps) {
  const formatDate = (dateStr: string | null) => {
    if (!dateStr) return null
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'long',
      day: 'numeric',
      year: 'numeric',
    })
  }

  return (
    <header className="relative">
      {/* Hero Background */}
      {campaign.image_url ? (
        <div className="absolute inset-0 h-64 lg:h-80">
          <img
            src={campaign.image_url}
            alt={campaign.name}
            className="h-full w-full object-cover"
          />
          <div className="absolute inset-0 bg-gradient-to-t from-background via-background/80 to-background/40" />
        </div>
      ) : (
        <div className="absolute inset-0 h-64 lg:h-80 bg-gradient-to-br from-primary/20 to-primary/5" />
      )}

      {/* Content */}
      <div className="relative mx-auto max-w-6xl px-4 pt-8 pb-8">
        {/* Navigation Bar */}
        <nav className="flex items-center justify-between mb-8">
          <Link
            to="/"
            className="flex items-center gap-2 text-foreground/80 hover:text-foreground transition-colors"
          >
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
              <FlowArrow className="h-5 w-5 text-primary-foreground" weight="bold" />
            </div>
            <span className="text-lg font-bold">FlowTrace</span>
          </Link>
        </nav>

        {/* Campaign Title Area */}
        <div className="mt-16 lg:mt-24">
          {/* Category Badge */}
          {campaign.category && (
            <span className="inline-flex items-center px-3 py-1 rounded-full text-xs font-medium bg-primary/10 text-primary mb-4 capitalize">
              {campaign.category}
            </span>
          )}

          {/* Title */}
          <h1 className="text-3xl lg:text-4xl font-bold tracking-tight mb-4">
            {campaign.name}
          </h1>

          {/* Short Description */}
          {campaign.short_description && (
            <p className="text-lg text-muted-foreground max-w-2xl mb-6">
              {campaign.short_description}
            </p>
          )}

          {/* Meta Info */}
          <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Buildings className="h-4 w-4" />
              <span>Organization Campaign</span>
            </div>

            {campaign.starts_at && (
              <div className="flex items-center gap-1.5">
                <Calendar className="h-4 w-4" />
                <span>Started {formatDate(campaign.starts_at)}</span>
              </div>
            )}

            {campaign.ends_at && (
              <div className="flex items-center gap-1.5">
                <Calendar className="h-4 w-4" />
                <span>Ends {formatDate(campaign.ends_at)}</span>
              </div>
            )}

            {campaign.target_node && (
              <div className="flex items-center gap-1.5">
                <MapPin className="h-4 w-4" />
                <span>{campaign.target_node.name}</span>
              </div>
            )}
          </div>

          {/* Tags */}
          {campaign.tags && campaign.tags.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-4">
              {campaign.tags.map((tag) => (
                <span
                  key={tag}
                  className="px-2 py-0.5 text-xs rounded-md bg-muted text-muted-foreground"
                >
                  #{tag}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>
    </header>
  )
}
