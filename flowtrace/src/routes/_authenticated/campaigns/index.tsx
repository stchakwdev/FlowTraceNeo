import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import {
  CircleNotch,
  FunnelSimple,
  Megaphone,
  Plus,
} from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { CampaignCard } from '@/components/campaign-card'
import { CampaignCreateModal } from '@/components/campaign-create-modal'
import {
  useCampaigns,
  useCampaignCounts,
  useDeleteCampaign,
} from '@/lib/queries'
import type { CampaignStatus } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/campaigns/')({
  component: CampaignsPage,
})

const STATUS_FILTERS: { value: CampaignStatus | 'all'; label: string }[] = [
  { value: 'all', label: 'All Campaigns' },
  { value: 'active', label: 'Active' },
  { value: 'draft', label: 'Drafts' },
  { value: 'paused', label: 'Paused' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
]

function CampaignsPage() {
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<CampaignStatus | 'all'>('all')

  const { data: campaigns, isLoading } = useCampaigns()
  const { data: counts } = useCampaignCounts()
  const deleteCampaign = useDeleteCampaign()

  // Filter campaigns
  const filteredCampaigns = campaigns?.filter((c) => {
    const matchesSearch =
      c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.description?.toLowerCase().includes(searchQuery.toLowerCase()) ||
      c.category?.toLowerCase().includes(searchQuery.toLowerCase())
    const matchesStatus = statusFilter === 'all' || c.status === statusFilter
    return matchesSearch && matchesStatus
  })

  const formatAmount = (amount: number) => {
    return new Intl.NumberFormat('en-KE', {
      style: 'currency',
      currency: 'KES',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount)
  }

  const handleDeleteCampaign = async (id: string) => {
    if (window.confirm('Are you sure you want to delete this campaign?')) {
      try {
        await deleteCampaign.mutateAsync(id)
      } catch (error) {
        console.error('Failed to delete campaign:', error)
      }
    }
  }

  return (
    <div className="px-6 py-6">
      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="section-label">FUNDRAISING</p>
          <h1 className="mt-1 font-display text-2xl font-bold">Campaigns</h1>
          <p className="text-sm text-muted-foreground">
            Create fundraising campaigns and track donor contributions
          </p>
        </div>
        <Button onClick={() => setIsCreateModalOpen(true)}>
          <Plus className="h-4 w-4 mr-2" />
          New Campaign
        </Button>
      </div>

      {/* Stats Cards */}
      <div className="mt-6 grid gap-4 md:grid-cols-4">
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-primary" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Total Campaigns</p>
            <p className="financial-figure text-2xl font-bold">{counts?.total_campaigns || 0}</p>
            <p className="text-xs text-muted-foreground">{counts?.active_campaigns || 0} active</p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-success" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Total Raised</p>
            <p className="financial-figure text-2xl font-bold">{formatAmount(counts?.total_raised || 0)}</p>
            <p className="text-xs text-muted-foreground">Across all campaigns</p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-chart-4" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Total Donations</p>
            <p className="financial-figure text-2xl font-bold">{counts?.total_donations || 0}</p>
            <p className="text-xs text-muted-foreground">Confirmed donations</p>
          </div>
        </div>
        <div className="relative overflow-hidden rounded-lg border border-border bg-card p-4">
          <div className="absolute left-0 top-0 h-full w-1 bg-accent" />
          <div className="pl-2">
            <p className="text-sm text-muted-foreground">Completed</p>
            <p className="financial-figure text-2xl font-bold">{counts?.completed_campaigns || 0}</p>
            <p className="text-xs text-muted-foreground">Fully funded campaigns</p>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="mt-6 flex flex-col gap-4 sm:flex-row sm:items-center">
        <div className="relative flex-1 max-w-sm">
          <Input
            placeholder="Search campaigns..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-10"
          />
          <FunnelSimple className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        </div>

        <Select
          value={statusFilter}
          onValueChange={(value) =>
            setStatusFilter(value as CampaignStatus | 'all')
          }
        >
          <SelectTrigger className="w-[180px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {STATUS_FILTERS.map((filter) => (
              <SelectItem key={filter.value} value={filter.value}>
                {filter.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Campaigns Grid */}
      <div className="mt-6">
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <CircleNotch className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : filteredCampaigns && filteredCampaigns.length > 0 ? (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {filteredCampaigns.map((campaign) => (
              <CampaignCard
                key={campaign.id}
                campaign={campaign}
                onDelete={() => handleDeleteCampaign(campaign.id)}
              />
            ))}
          </div>
        ) : (
          <Card className="p-12 text-center">
            <Megaphone className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No campaigns found</h3>
            <p className="text-muted-foreground mb-4">
              {searchQuery || statusFilter !== 'all'
                ? 'Try adjusting your filters'
                : 'Create your first campaign to start accepting donations'}
            </p>
            {!searchQuery && statusFilter === 'all' && (
              <Button onClick={() => setIsCreateModalOpen(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Create Campaign
              </Button>
            )}
          </Card>
        )}
      </div>

      {/* Create Modal */}
      <CampaignCreateModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
      />
    </div>
  )
}
