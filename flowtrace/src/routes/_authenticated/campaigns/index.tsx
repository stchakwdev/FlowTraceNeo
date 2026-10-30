import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { MagnifyingGlass, Plus } from '@phosphor-icons/react'
import type { CampaignStatus, CampaignWithDetails } from '@/lib/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import {
  AsOf,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { CampaignTable } from '@/components/campaign-card'
import { CampaignCreateModal } from '@/components/campaign-create-modal'
import {
  useCampaignCounts,
  useCampaigns,
  useDeleteCampaign,
} from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import { formatCompact } from '@/lib/format'

export const Route = createFileRoute('/_authenticated/campaigns/')({
  component: CampaignsPage,
})

const STATUS_FILTERS: Array<{ value: CampaignStatus | 'all'; label: string }> = [
  { value: 'all', label: 'All statuses' },
  { value: 'active', label: 'Active' },
  { value: 'draft', label: 'Drafts' },
  { value: 'paused', label: 'Paused' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
]

function CampaignsPage() {
  const { organization } = useOrg()
  const [isCreateOpen, setIsCreateOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<CampaignStatus | 'all'>('all')
  const [pendingDelete, setPendingDelete] = useState<CampaignWithDetails | null>(null)
  const [deleteError, setDeleteError] = useState<string | null>(null)

  const { data: campaigns, isLoading, error, refetch } = useCampaigns()
  const { data: counts } = useCampaignCounts()
  const deleteCampaign = useDeleteCampaign()

  const q = query.trim().toLowerCase()
  const filtered = (campaigns ?? []).filter((c) => {
    const matchesSearch =
      !q ||
      c.name.toLowerCase().includes(q) ||
      c.description?.toLowerCase().includes(q) ||
      c.category?.toLowerCase().includes(q)
    return matchesSearch && (statusFilter === 'all' || c.status === statusFilter)
  })

  const confirmDelete = async () => {
    if (!pendingDelete) return
    setDeleteError(null)
    try {
      await deleteCampaign.mutateAsync(pendingDelete.id)
      setPendingDelete(null)
    } catch (e) {
      setDeleteError(
        e instanceof Error
          ? `The draft was not deleted: ${e.message}`
          : 'The draft was not deleted. Try again.',
      )
    }
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-4xl sm:text-5xl">Campaigns</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {organization?.name ?? 'Your organisation'} · fundraising campaigns ·{' '}
            <AsOf date={new Date()} />
          </p>
        </div>
        <Button className="h-10 self-start sm:self-auto" onClick={() => setIsCreateOpen(true)}>
          <Plus />
          New campaign
        </Button>
      </div>

      <KeyFigures className="mt-6 rule-heavy">
        <KeyFigure
          scale="md"
          value={`KES ${formatCompact(counts?.total_raised ?? 0)}`}
          label="Raised across campaigns"
        />
        <KeyFigure
          value={counts?.active_campaigns ?? 0}
          label="Open to donors"
          detail={
            <span className="text-muted-foreground">
              of {counts?.total_campaigns ?? 0} campaigns
            </span>
          }
        />
        <KeyFigure value={counts?.total_donations ?? 0} label="Confirmed donations" />
        <KeyFigure value={counts?.completed_campaigns ?? 0} label="Completed" />
      </KeyFigures>

      <ReportSection
        title="All campaigns"
        className="mt-8"
        meta={
          campaigns ? (
            <span className="tabular-nums">
              {filtered.length} of {campaigns.length}
            </span>
          ) : null
        }
      >
        <div className="flex flex-col gap-2 border-b border-border py-3 sm:flex-row sm:items-center">
          <div className="relative sm:max-w-xs sm:flex-1">
            <MagnifyingGlass
              aria-hidden="true"
              className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              type="search"
              aria-label="Search campaigns"
              placeholder="Search by name or category"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="h-10 pl-9"
            />
          </div>
          <Select
            value={statusFilter}
            items={STATUS_FILTERS}
            onValueChange={(v) => setStatusFilter((v ?? 'all') as CampaignStatus | 'all')}
          >
            <SelectTrigger aria-label="Filter by status" className="h-10 w-full sm:w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STATUS_FILTERS.map((f) => (
                <SelectItem key={f.value} value={f.value}>
                  {f.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {isLoading ? (
          <ReportNotice title="Loading campaigns" />
        ) : error ? (
          <ReportNotice
            tone="error"
            title="Campaigns could not be loaded"
            action={
              <Button variant="outline" className="h-10" onClick={() => void refetch()}>
                Try again
              </Button>
            }
          >
            {error.message}
          </ReportNotice>
        ) : filtered.length > 0 ? (
          <CampaignTable campaigns={filtered} onDelete={setPendingDelete} />
        ) : campaigns && campaigns.length > 0 ? (
          <ReportNotice
            title="No campaigns match"
            action={
              <Button
                variant="outline"
                className="h-10"
                onClick={() => {
                  setQuery('')
                  setStatusFilter('all')
                }}
              >
                Clear filters
              </Button>
            }
          >
            Nothing matches “{query || STATUS_FILTERS.find((f) => f.value === statusFilter)?.label}”.
          </ReportNotice>
        ) : (
          <ReportNotice
            title="No campaigns yet"
            action={
              <Button className="h-10" onClick={() => setIsCreateOpen(true)}>
                <Plus />
                New campaign
              </Button>
            }
          >
            Campaigns you create appear here with what they have raised and how
            much of it is spent with proof.
          </ReportNotice>
        )}
      </ReportSection>

      <CampaignCreateModal isOpen={isCreateOpen} onClose={() => setIsCreateOpen(false)} />

      <AlertDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingDelete(null)
            setDeleteError(null)
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete this draft?</AlertDialogTitle>
            <AlertDialogDescription>
              {pendingDelete?.name} has not been opened to donors. Deleting it
              cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          {deleteError && (
            <p role="alert" className="text-xs font-semibold text-destructive">
              {deleteError}
            </p>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel>Keep draft</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={deleteCampaign.isPending}
              onClick={() => void confirmDelete()}
            >
              Delete draft
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
