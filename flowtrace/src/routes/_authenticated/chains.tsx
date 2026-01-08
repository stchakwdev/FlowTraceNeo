import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import {
  LinkSimple,
  Plus,
  Buildings,
  Shield,
  MapPin,
  Clock,
  ArrowRight,
  ArrowLeft,
  DotsThree,
  Eye,
  Trash,
  Prohibit,
  CircleNotch,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { ChainStatusBadge } from '@/components/chain-status-badge'
import { ChainRequestModal } from '@/components/chain-request-modal'
import { ChainApprovalModal } from '@/components/chain-approval-modal'
import {
  useOutgoingChains,
  useIncomingChains,
  useChainCounts,
  useCancelChainRequest,
  useRevokeChain,
} from '@/lib/queries'
import type { OrganizationChainWithDetails } from '@/lib/supabase'

export const Route = createFileRoute('/_authenticated/chains')({
  component: ChainsPage,
})

function ChainsPage() {
  const [activeTab, setActiveTab] = useState<'outgoing' | 'incoming'>('outgoing')
  const [isRequestModalOpen, setIsRequestModalOpen] = useState(false)
  const [selectedChain, setSelectedChain] = useState<OrganizationChainWithDetails | null>(null)
  const [isApprovalModalOpen, setIsApprovalModalOpen] = useState(false)

  const { data: outgoingChains, isLoading: loadingOutgoing } = useOutgoingChains()
  const { data: incomingChains, isLoading: loadingIncoming } = useIncomingChains()
  const { data: chainCounts } = useChainCounts()
  const cancelChain = useCancelChainRequest()
  const revokeChain = useRevokeChain()

  const chains = activeTab === 'outgoing' ? outgoingChains : incomingChains
  const isLoading = activeTab === 'outgoing' ? loadingOutgoing : loadingIncoming

  const handleCancelRequest = async (chainId: string) => {
    try {
      await cancelChain.mutateAsync(chainId)
    } catch (error) {
      console.error('Failed to cancel chain request:', error)
    }
  }

  const handleRevokeChain = async (chainId: string, revokedBy: string) => {
    try {
      await revokeChain.mutateAsync({ chainId, revokedBy })
    } catch (error) {
      console.error('Failed to revoke chain:', error)
    }
  }

  const getTrustGrade = (score: number) => {
    if (score >= 90) return 'A+'
    if (score >= 80) return 'A'
    if (score >= 70) return 'B'
    if (score >= 60) return 'C'
    if (score >= 50) return 'D'
    return 'F'
  }

  return (
    <div className="flex flex-col gap-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Chain Links</h1>
          <p className="text-muted-foreground">
            Connect with partner organizations for end-to-end transparency
          </p>
        </div>
        <Button onClick={() => setIsRequestModalOpen(true)}>
          <Plus className="h-4 w-4 mr-2" />
          New Chain Request
        </Button>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Active Chains</CardTitle>
            <LinkSimple className="h-4 w-4 text-emerald-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {(chainCounts?.outgoing_active || 0) + (chainCounts?.incoming_active || 0)}
            </div>
            <p className="text-xs text-muted-foreground">
              {chainCounts?.outgoing_active || 0} outgoing, {chainCounts?.incoming_active || 0} incoming
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Pending Requests</CardTitle>
            <Clock className="h-4 w-4 text-amber-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {(chainCounts?.outgoing_pending || 0) + (chainCounts?.incoming_pending || 0)}
            </div>
            <p className="text-xs text-muted-foreground">
              {chainCounts?.outgoing_pending || 0} sent, {chainCounts?.incoming_pending || 0} received
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Outgoing Links</CardTitle>
            <ArrowRight className="h-4 w-4 text-blue-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{chainCounts?.outgoing_active || 0}</div>
            <p className="text-xs text-muted-foreground">Organizations you link to</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Incoming Links</CardTitle>
            <ArrowLeft className="h-4 w-4 text-purple-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{chainCounts?.incoming_active || 0}</div>
            <p className="text-xs text-muted-foreground">Organizations linking to you</p>
          </CardContent>
        </Card>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 border-b">
        <button
          className={cn(
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === 'outgoing'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground hover:text-foreground',
          )}
          onClick={() => setActiveTab('outgoing')}
        >
          <ArrowRight className="h-4 w-4 inline mr-2" />
          Outgoing
          {(chainCounts?.outgoing_pending || 0) > 0 && (
            <span className="ml-2 px-1.5 py-0.5 text-xs rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
              {chainCounts?.outgoing_pending}
            </span>
          )}
        </button>
        <button
          className={cn(
            'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
            activeTab === 'incoming'
              ? 'border-primary text-primary'
              : 'border-transparent text-muted-foreground hover:text-foreground',
          )}
          onClick={() => setActiveTab('incoming')}
        >
          <ArrowLeft className="h-4 w-4 inline mr-2" />
          Incoming
          {(chainCounts?.incoming_pending || 0) > 0 && (
            <span className="ml-2 px-1.5 py-0.5 text-xs rounded-full bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
              {chainCounts?.incoming_pending}
            </span>
          )}
        </button>
      </div>

      {/* Chains List */}
      <div className="space-y-3">
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <CircleNotch className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : chains && chains.length > 0 ? (
          chains.map((chain) => (
            <ChainCard
              key={chain.id}
              chain={chain}
              direction={activeTab}
              onReview={() => {
                setSelectedChain(chain)
                setIsApprovalModalOpen(true)
              }}
              onCancel={() => handleCancelRequest(chain.id)}
              onRevoke={(revokedBy) => handleRevokeChain(chain.id, revokedBy)}
            />
          ))
        ) : (
          <Card className="p-12 text-center">
            <LinkSimple className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No chain links yet</h3>
            <p className="text-muted-foreground mb-4">
              {activeTab === 'outgoing'
                ? 'Start connecting with partner organizations to track money flow end-to-end.'
                : 'Other organizations can request to link with you for transparency.'}
            </p>
            {activeTab === 'outgoing' && (
              <Button onClick={() => setIsRequestModalOpen(true)}>
                <Plus className="h-4 w-4 mr-2" />
                Request Chain Link
              </Button>
            )}
          </Card>
        )}
      </div>

      {/* Modals */}
      <ChainRequestModal
        isOpen={isRequestModalOpen}
        onClose={() => setIsRequestModalOpen(false)}
        requestedBy="current-user-id" // TODO: Get from auth context
      />

      <ChainApprovalModal
        chain={selectedChain}
        isOpen={isApprovalModalOpen}
        onClose={() => {
          setIsApprovalModalOpen(false)
          setSelectedChain(null)
        }}
        approvedBy="current-user-id" // TODO: Get from auth context
      />
    </div>
  )
}

interface ChainCardProps {
  chain: OrganizationChainWithDetails
  direction: 'outgoing' | 'incoming'
  onReview: () => void
  onCancel: () => void
  onRevoke: (revokedBy: string) => void
}

function ChainCard({ chain, direction, onReview, onCancel, onRevoke }: ChainCardProps) {
  // For outgoing, show target org. For incoming, show source org.
  const org = direction === 'outgoing' ? chain.target_org : chain.source_org

  const getTrustGrade = (score: number) => {
    if (score >= 90) return 'A+'
    if (score >= 80) return 'A'
    if (score >= 70) return 'B'
    if (score >= 60) return 'C'
    if (score >= 50) return 'D'
    return 'F'
  }

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  return (
    <Card className="p-4">
      <div className="flex items-center gap-4">
        {/* Organization Logo */}
        <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 shrink-0">
          {org.logo_url ? (
            <img
              src={org.logo_url}
              alt={org.name}
              className="h-10 w-10 rounded-lg object-cover"
            />
          ) : (
            <Buildings className="h-6 w-6 text-primary" />
          )}
        </div>

        {/* Organization Info */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="font-semibold truncate">{org.name}</h3>
            <ChainStatusBadge status={chain.status} size="sm" />
          </div>
          <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
            {org.country && (
              <span className="flex items-center gap-1">
                <MapPin className="h-3.5 w-3.5" />
                {org.country}
              </span>
            )}
            <span className="flex items-center gap-1">
              <Shield className="h-3.5 w-3.5 text-emerald-600" />
              Trust: {getTrustGrade(org.trust_score)}
            </span>
            <span className="flex items-center gap-1">
              <Clock className="h-3.5 w-3.5" />
              {chain.status === 'pending'
                ? `Requested ${formatDate(chain.requested_at)}`
                : chain.approved_at
                  ? `Since ${formatDate(chain.approved_at)}`
                  : formatDate(chain.requested_at)}
            </span>
          </div>
          {chain.granted_group && chain.status === 'active' && (
            <p className="text-xs text-muted-foreground mt-1">
              Access level: <span className="font-medium">{chain.granted_group.name}</span>
            </p>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 shrink-0">
          {direction === 'incoming' && chain.status === 'pending' && (
            <Button size="sm" onClick={onReview}>
              Review
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon">
                <DotsThree className="h-5 w-5" weight="bold" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem>
                <Eye className="h-4 w-4 mr-2" />
                View Details
              </DropdownMenuItem>
              {chain.status === 'pending' && direction === 'outgoing' && (
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={onCancel}
                >
                  <Trash className="h-4 w-4 mr-2" />
                  Cancel Request
                </DropdownMenuItem>
              )}
              {chain.status === 'active' && (
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={() => onRevoke('current-user-id')}
                >
                  <Prohibit className="h-4 w-4 mr-2" />
                  Revoke Access
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Request Message (if pending and has message) */}
      {chain.status === 'pending' && chain.request_message && (
        <div className="mt-3 pt-3 border-t">
          <p className="text-sm text-muted-foreground">
            <span className="font-medium">Message:</span> {chain.request_message}
          </p>
        </div>
      )}

      {/* Rejection Reason (if rejected) */}
      {chain.status === 'rejected' && chain.rejection_reason && (
        <div className="mt-3 pt-3 border-t">
          <p className="text-sm text-destructive">
            <span className="font-medium">Reason:</span> {chain.rejection_reason}
          </p>
        </div>
      )}
    </Card>
  )
}
