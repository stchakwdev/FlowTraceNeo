import { createFileRoute, Link } from '@tanstack/react-router'
import { CircleNotch, Heart, ArrowLeft } from '@phosphor-icons/react'
import { usePublicCampaign, useCampaignStats } from '@/lib/queries'
import { PublicCampaignHeader } from '@/components/public-campaign-header'
import { PublicDonationForm } from '@/components/public-donation-form'
import { SocialShareButtons } from '@/components/social-share-buttons'
import { CampaignProgressBar } from '@/components/campaign-progress-bar'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

export const Route = createFileRoute('/donate/$slug')({
  component: PublicCampaignPage,
  head: () => ({
    meta: [
      {
        title: `Donate | FlowTrace`,
      },
      {
        name: 'description',
        content: 'Support this campaign with a donation',
      },
    ],
  }),
})

function PublicCampaignPage() {
  const { slug } = Route.useParams()
  const { data: campaign, isLoading, error } = usePublicCampaign(slug)
  const { data: stats } = useCampaignStats(campaign?.id || '')

  if (isLoading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <CircleNotch className="h-8 w-8 animate-spin text-primary" />
      </div>
    )
  }

  if (error || !campaign) {
    return (
      <div className="min-h-screen bg-background">
        <div className="mx-auto max-w-2xl px-4 py-16">
          <Card className="text-center p-8">
            <Heart className="h-16 w-16 mx-auto text-muted-foreground mb-4" />
            <h1 className="text-2xl font-bold mb-2">Campaign Not Found</h1>
            <p className="text-muted-foreground mb-6">
              This campaign may have ended, been removed, or the link may be incorrect.
            </p>
            <Button nativeButton={false} render={<Link to="/" />}>
              <ArrowLeft className="h-4 w-4 mr-2" />
              Go to Homepage
            </Button>
          </Card>
        </div>
      </div>
    )
  }

  const shareUrl = typeof window !== 'undefined' ? window.location.href : ''
  const shareTitle = `Support ${campaign.name}`
  const shareText = campaign.short_description || campaign.description || `Donate to ${campaign.name}`

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <PublicCampaignHeader campaign={campaign} />

      {/* Main Content */}
      <div className="mx-auto max-w-6xl px-4 py-8">
        <div className="grid gap-8 lg:grid-cols-5">
          {/* Left Column - Campaign Info */}
          <div className="lg:col-span-3 space-y-6">
            {/* Progress Card */}
            <Card>
              <CardContent className="p-6">
                <CampaignProgressBar
                  current={campaign.current_amount}
                  target={campaign.target_amount}
                  currency={campaign.currency}
                  size="lg"
                />

                <div className="mt-6 grid grid-cols-3 gap-4 text-center">
                  <div>
                    <p className="text-2xl font-bold text-primary">
                      {stats?.donation_count || 0}
                    </p>
                    <p className="text-sm text-muted-foreground">Donations</p>
                  </div>
                  <div>
                    <p className="text-2xl font-bold text-primary">
                      {stats?.unique_donors || 0}
                    </p>
                    <p className="text-sm text-muted-foreground">Donors</p>
                  </div>
                  <div>
                    <p className="text-2xl font-bold text-primary">
                      {stats?.days_remaining !== null && stats?.days_remaining !== undefined
                        ? stats.days_remaining
                        : '∞'}
                    </p>
                    <p className="text-sm text-muted-foreground">Days Left</p>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Description */}
            {campaign.description && (
              <Card>
                <CardHeader>
                  <CardTitle>About This Campaign</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="prose prose-sm max-w-none dark:prose-invert">
                    <p className="whitespace-pre-wrap">{campaign.description}</p>
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Social Share */}
            <Card>
              <CardHeader>
                <CardTitle>Share This Campaign</CardTitle>
              </CardHeader>
              <CardContent>
                <SocialShareButtons
                  url={shareUrl}
                  title={shareTitle}
                  text={shareText}
                />
              </CardContent>
            </Card>
          </div>

          {/* Right Column - Donation Form */}
          <div className="lg:col-span-2">
            <div className="sticky top-4">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Heart className="h-5 w-5 text-primary" weight="fill" />
                    Make a Donation
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <PublicDonationForm campaign={campaign} />
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="border-t bg-card py-8 mt-12">
        <div className="mx-auto max-w-6xl px-4 text-center">
          <p className="text-sm text-muted-foreground">
            Powered by{' '}
            <Link to="/" className="font-medium text-primary hover:underline">
              FlowTrace
            </Link>
            {' - '}Financial Transparency Platform
          </p>
        </div>
      </footer>
    </div>
  )
}
