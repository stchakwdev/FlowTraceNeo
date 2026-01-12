import { useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import {
  ArrowLeft,
  Calendar,
  CircleNotch,
  Clock,
  DotsThree,
  Heart,
  PencilSimple,
  Target,
  Trash,
  TrendUp,
  Users,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { CampaignProgressBar } from '@/components/campaign-progress-bar'
import { CampaignStatusBadge, PaymentStatusBadge } from '@/components/campaign-status-badge'
import { DonationModal } from '@/components/donation-modal'
import {
  useCampaign,
  useCampaignStats,
  useDonations,
  useUpdateCampaign,
  useDeleteCampaign,
} from '@/lib/queries'
import type { CampaignStatus } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/campaigns/$id')({
  component: CampaignDetailPage,
})

function CampaignDetailPage() {
  const { id } = Route.useParams()
  const navigate = Route.useNavigate()
  const [isDonationModalOpen, setIsDonationModalOpen] = useState(false)

  const { data: campaign, isLoading } = useCampaign(id)
  const { data: stats } = useCampaignStats(id)
  const { data: donations, isLoading: loadingDonations } = useDonations(id)
  const updateCampaign = useUpdateCampaign()
  const deleteCampaign = useDeleteCampaign()

  const formatAmount = (amount: number, currency = 'KES') => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  const formatDate = (dateStr: string | null) => {
    if (!dateStr) return 'N/A'
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  const handleStatusChange = async (newStatus: CampaignStatus) => {
    try {
      await updateCampaign.mutateAsync({
        id,
        updates: { status: newStatus },
      })
    } catch (error) {
      console.error('Failed to update campaign status:', error)
    }
  }

  const handleDelete = async () => {
    if (window.confirm('Are you sure you want to delete this campaign? This cannot be undone.')) {
      try {
        await deleteCampaign.mutateAsync(id)
        navigate({ to: '/campaigns' })
      } catch (error) {
        console.error('Failed to delete campaign:', error)
      }
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <CircleNotch className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!campaign) {
    return (
      <div className="mx-auto max-w-7xl p-4 md:p-6">
        <Card className="p-12 text-center">
          <h3 className="text-lg font-medium mb-2">Campaign not found</h3>
          <p className="text-muted-foreground mb-4">
            The campaign you're looking for doesn't exist or has been deleted.
          </p>
          <Button asChild>
            <Link to="/campaigns">Back to Campaigns</Link>
          </Button>
        </Card>
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-7xl p-4 md:p-6">
      {/* Back link */}
      <Link
        to="/campaigns"
        className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground mb-4"
      >
        <ArrowLeft className="h-4 w-4" />
        Back to Campaigns
      </Link>

      {/* Header */}
      <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <h1 className="text-2xl font-bold tracking-tight">{campaign.name}</h1>
            <CampaignStatusBadge status={campaign.status} />
          </div>
          {campaign.description && (
            <p className="text-muted-foreground max-w-2xl">{campaign.description}</p>
          )}
          {campaign.category && (
            <span className="inline-flex items-center mt-2 px-2 py-0.5 rounded text-xs font-medium bg-muted text-muted-foreground capitalize">
              {campaign.category}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {campaign.status === 'active' && (
            <Button onClick={() => setIsDonationModalOpen(true)}>
              <Heart className="h-4 w-4 mr-2" weight="fill" />
              Donate
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="icon">
                <DotsThree className="h-5 w-5" weight="bold" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem>
                <PencilSimple className="h-4 w-4 mr-2" />
                Edit Campaign
              </DropdownMenuItem>
              {campaign.status === 'draft' && (
                <DropdownMenuItem onClick={() => handleStatusChange('active')}>
                  <TrendUp className="h-4 w-4 mr-2" />
                  Activate Campaign
                </DropdownMenuItem>
              )}
              {campaign.status === 'active' && (
                <DropdownMenuItem onClick={() => handleStatusChange('paused')}>
                  <Clock className="h-4 w-4 mr-2" />
                  Pause Campaign
                </DropdownMenuItem>
              )}
              {campaign.status === 'paused' && (
                <DropdownMenuItem onClick={() => handleStatusChange('active')}>
                  <TrendUp className="h-4 w-4 mr-2" />
                  Resume Campaign
                </DropdownMenuItem>
              )}
              {campaign.status === 'draft' && (
                <DropdownMenuItem onClick={handleDelete} className="text-destructive">
                  <Trash className="h-4 w-4 mr-2" />
                  Delete Campaign
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Progress Card */}
      <Card className="mt-6">
        <CardContent className="p-6">
          <CampaignProgressBar
            current={campaign.current_amount}
            target={campaign.target_amount}
            currency={campaign.currency}
            size="lg"
          />
        </CardContent>
      </Card>

      {/* Stats Grid */}
      <div className="mt-6 grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Total Raised</CardTitle>
            <TrendUp className="h-4 w-4 text-emerald-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {formatAmount(stats?.total_raised || campaign.current_amount, campaign.currency)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Donations</CardTitle>
            <Heart className="h-4 w-4 text-pink-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats?.donation_count || 0}</div>
            <p className="text-xs text-muted-foreground">
              {stats?.unique_donors || 0} unique donors
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Avg Donation</CardTitle>
            <Users className="h-4 w-4 text-blue-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {formatAmount(stats?.avg_donation || 0, campaign.currency)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Days Active</CardTitle>
            <Calendar className="h-4 w-4 text-amber-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats?.days_active || 0}</div>
            {stats?.days_remaining !== null && stats?.days_remaining !== undefined && (
              <p className="text-xs text-muted-foreground">
                {stats.days_remaining} days remaining
              </p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Campaign Details */}
      <div className="mt-6 grid gap-6 lg:grid-cols-3">
        {/* Donations List */}
        <div className="lg:col-span-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Recent Donations</CardTitle>
            </CardHeader>
            <CardContent>
              {loadingDonations ? (
                <div className="flex items-center justify-center py-8">
                  <CircleNotch className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : donations && donations.length > 0 ? (
                <div className="space-y-3">
                  {donations.map((donation) => (
                    <div
                      key={donation.id}
                      className="flex items-center justify-between p-3 rounded-lg border"
                    >
                      <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
                          <Heart className="h-5 w-5 text-primary" weight="fill" />
                        </div>
                        <div>
                          <p className="font-medium">
                            {donation.is_anonymous
                              ? 'Anonymous'
                              : donation.donor_name || 'Anonymous'}
                          </p>
                          <p className="text-xs text-muted-foreground">
                            {formatDate(donation.created_at)}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <PaymentStatusBadge status={donation.payment_status} size="sm" />
                        <span className="font-semibold">
                          {formatAmount(donation.amount, donation.currency)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  <Heart className="h-8 w-8 mx-auto mb-2 opacity-50" />
                  <p>No donations yet</p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Campaign Info Sidebar */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Campaign Details</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Target</span>
                <span className="font-medium">
                  {formatAmount(campaign.target_amount, campaign.currency)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Currency</span>
                <span className="font-medium">{campaign.currency}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Started</span>
                <span className="font-medium">{formatDate(campaign.starts_at)}</span>
              </div>
              {campaign.ends_at && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Ends</span>
                  <span className="font-medium">{formatDate(campaign.ends_at)}</span>
                </div>
              )}
              {campaign.target_node && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Target Project</span>
                  <span className="font-medium">{campaign.target_node.name}</span>
                </div>
              )}
              <div className="flex justify-between">
                <span className="text-muted-foreground">Visibility</span>
                <span className="font-medium">
                  {campaign.is_public ? 'Public' : 'Private'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Allow Overflow</span>
                <span className="font-medium">
                  {campaign.allow_overflow ? 'Yes' : 'No'}
                </span>
              </div>
            </CardContent>
          </Card>

          {campaign.public_slug && campaign.is_public && (
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Public Link</CardTitle>
              </CardHeader>
              <CardContent>
                <code className="block p-2 rounded bg-muted text-xs break-all">
                  /donate/{campaign.public_slug}
                </code>
              </CardContent>
            </Card>
          )}
        </div>
      </div>

      {/* Donation Modal */}
      <DonationModal
        campaign={campaign}
        isOpen={isDonationModalOpen}
        onClose={() => setIsDonationModalOpen(false)}
      />
    </div>
  )
}
