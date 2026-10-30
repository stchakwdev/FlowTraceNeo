import { useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  ArrowLeft,
  ArrowSquareOut,
  DotsThree,
  Pause,
  Play,
  Plus,
  Trash,
} from '@phosphor-icons/react'
import type { CampaignStatus } from '@/lib/types'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
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
  KeyFigure,
  KeyFigures,
  ProofLegend,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { CampaignCoverage } from '@/components/campaign-progress-bar'
import {
  CampaignStatusMark,
  PaymentStatusMark,
  campaignTiming,
} from '@/components/campaign-status-badge'
import { DonationModal } from '@/components/donation-modal'
import {
  EarmarkList,
  ProofSpendingList,
} from '@/components/public-campaign-header'
import {
  useCampaign,
  useCampaignStats,
  useDeleteCampaign,
  useDonations,
  useUpdateCampaign,
  useUpdateDonationStatus,
} from '@/lib/queries'
import { getCampaignReport } from '@/lib/data'
import { formatDate, formatMoney, formatShortDate } from '@/lib/format'

export const Route = createFileRoute('/_authenticated/campaigns/$id')({
  component: CampaignDetailPage,
})

const METHOD_LABEL: Record<string, string> = {
  mpesa: 'M-Pesa',
  card: 'Card',
  bank: 'Bank transfer',
  paypal: 'PayPal',
}

function CampaignDetailPage() {
  const { id } = Route.useParams()
  const navigate = Route.useNavigate()
  const [donationOpen, setDonationOpen] = useState(false)
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [actionError, setActionError] = useState<string | null>(null)

  const { data: campaign, isLoading, error, refetch } = useCampaign(id)
  const { data: stats } = useCampaignStats(id)
  const donationsQuery = useDonations(id)
  const report = useQuery({
    queryKey: ['campaign-report', id, campaign?.current_amount],
    queryFn: () => getCampaignReport(id),
    enabled: !!campaign,
    staleTime: 30 * 1000,
  })
  const updateCampaign = useUpdateCampaign()
  const deleteCampaign = useDeleteCampaign()
  const updateDonation = useUpdateDonationStatus()

  // No payment processor confirms a gift yet, so a member marks it received
  // (pre-decision P19; the module keeps new gifts pending).
  const confirmDonation = async (donationId: string) => {
    setActionError(null)
    try {
      await updateDonation.mutateAsync({ id: donationId, status: 'confirmed' })
    } catch (e) {
      setActionError(
        `The donation was not confirmed${e instanceof Error ? `: ${e.message}` : '.'} Try again.`,
      )
    }
  }

  const setStatus = async (status: CampaignStatus) => {
    setActionError(null)
    try {
      await updateCampaign.mutateAsync({ id, updates: { status } })
    } catch (e) {
      setActionError(
        `The campaign status did not change${e instanceof Error ? `: ${e.message}` : '.'} Try again.`,
      )
    }
  }

  const handleDelete = async () => {
    try {
      await deleteCampaign.mutateAsync(id)
      setDeleteOpen(false)
      void navigate({ to: '/campaigns' })
    } catch (e) {
      setDeleteOpen(false)
      setActionError(
        `The draft was not deleted${e instanceof Error ? `: ${e.message}` : '.'}`,
      )
    }
  }

  const page = 'mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8'
  const backLink = (
    <Link
      to="/campaigns"
      className="inline-flex min-h-10 items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeft />
      All campaigns
    </Link>
  )

  if (isLoading) {
    return (
      <div className={page}>
        {backLink}
        <ReportNotice title="Loading campaign" />
      </div>
    )
  }

  if (error || !campaign) {
    return (
      <div className={page}>
        {backLink}
        <ReportSection title="Campaign" className="mt-4">
          <ReportNotice
            tone={error ? 'error' : 'neutral'}
            title={error ? 'This campaign could not be loaded' : 'Campaign not found'}
            action={
              error ? (
                <Button variant="outline" className="h-10" onClick={() => void refetch()}>
                  Try again
                </Button>
              ) : (
                <Link to="/campaigns" className={buttonVariants({ variant: 'outline' })}>
                  Back to campaigns
                </Link>
              )
            }
          >
            {error
              ? error.message
              : 'It may have been deleted, or the link is wrong. Open it again from the campaign list.'}
          </ReportNotice>
        </ReportSection>
      </div>
    )
  }

  const timing = campaignTiming(campaign)
  const donations = donationsQuery.data ?? []
  const publicUrl =
    campaign.is_public && campaign.public_slug ? `/donate/${campaign.public_slug}` : null
  const canReceive = campaign.status === 'active' && !timing.ended

  return (
    <div className={page}>
      {backLink}

      <div className="mt-2 flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">{campaign.name}</h1>
          <p className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
            <CampaignStatusMark status={campaign.status} />
            <span>{timing.label}</span>
            {campaign.target_node && <span>Funds {campaign.target_node.name}</span>}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {canReceive && (
            <Button className="h-10" onClick={() => setDonationOpen(true)}>
              <Plus />
              Record donation
            </Button>
          )}
          {publicUrl && campaign.status === 'active' && (
            <a
              href={publicUrl}
              target="_blank"
              rel="noopener noreferrer"
              className={buttonVariants({ variant: 'outline', className: 'h-10' })}
            >
              <ArrowSquareOut />
              Donate page
            </a>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="outline"
                  size="icon"
                  className="size-10"
                  aria-label="More campaign actions"
                />
              }
            >
              <DotsThree className="size-5" weight="bold" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {campaign.status === 'draft' && (
                <DropdownMenuItem onClick={() => void setStatus('active')}>
                  <Play />
                  Open to donors
                </DropdownMenuItem>
              )}
              {campaign.status === 'active' && (
                <DropdownMenuItem onClick={() => void setStatus('paused')}>
                  <Pause />
                  Pause campaign
                </DropdownMenuItem>
              )}
              {campaign.status === 'paused' && (
                <DropdownMenuItem onClick={() => void setStatus('active')}>
                  <Play />
                  Resume campaign
                </DropdownMenuItem>
              )}
              {campaign.status === 'draft' && (
                <DropdownMenuItem
                  onClick={() => setDeleteOpen(true)}
                  className="text-destructive"
                >
                  <Trash />
                  Delete draft
                </DropdownMenuItem>
              )}
              {campaign.status !== 'draft' &&
                campaign.status !== 'active' &&
                campaign.status !== 'paused' && (
                  <DropdownMenuItem disabled>No actions for a closed campaign</DropdownMenuItem>
                )}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {actionError && (
        <p role="alert" className="mt-3 text-sm font-semibold text-destructive">
          {actionError}
        </p>
      )}

      {campaign.description && (
        <p className="mt-4 max-w-[68ch] text-base text-foreground/85">
          {campaign.description}
        </p>
      )}

      <ReportSection
        title="Funding coverage"
        className="mt-8"
        meta={<span>{campaign.currency}</span>}
      >
        <div className="py-5">
          <CampaignCoverage
            raised={campaign.current_amount}
            target={campaign.target_amount}
            evidenced={report.data?.evidenced_amount}
            currency={campaign.currency}
            size="lg"
          />
          {report.error && (
            <p className="mt-2 text-xs text-destructive">
              Spending with proof could not be loaded: {report.error.message}
            </p>
          )}
        </div>
        <KeyFigures className="border-t">
          <KeyFigure scale="sm" value={stats?.donation_count ?? '–'} label="Confirmed donations" />
          <KeyFigure scale="sm" value={stats?.unique_donors ?? '–'} label="Donors" />
          <KeyFigure
            scale="sm"
            value={stats ? formatMoney(stats.avg_donation, campaign.currency) : '–'}
            label="Average gift"
          />
          <KeyFigure
            scale="sm"
            value={
              timing.daysLeft !== null
                ? timing.daysLeft
                : timing.ended
                  ? 'Ended'
                  : 'Open'
            }
            label={
              timing.daysLeft !== null
                ? `Days left, to ${formatShortDate(campaign.ends_at!)}`
                : timing.ended && campaign.ends_at
                  ? `on ${formatDate(campaign.ends_at)}`
                  : 'No end date'
            }
          />
        </KeyFigures>
      </ReportSection>

      <div className="mt-8 grid gap-8 lg:grid-cols-3">
        <div className="min-w-0 space-y-8 lg:col-span-2">
          <ReportSection
            title="Donations"
            meta={<span className="tabular-nums">{donations.length} recorded</span>}
          >
            {donationsQuery.isLoading ? (
              <ReportNotice title="Loading donations" />
            ) : donationsQuery.error ? (
              <ReportNotice tone="error" title="Donations could not be loaded">
                {donationsQuery.error.message}
              </ReportNotice>
            ) : donations.length === 0 ? (
              <ReportNotice title="No donations yet">
                Gifts appear here as they are made on the donate page or recorded
                with Record donation.
              </ReportNotice>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[34rem] text-sm">
                  <thead>
                    <tr className="border-b border-border text-left text-xs text-muted-foreground">
                      <th scope="col" className="py-2 pr-4 font-semibold">Donor</th>
                      <th scope="col" className="py-2 pr-4 font-semibold">Date</th>
                      <th scope="col" className="py-2 pr-4 font-semibold">Payment</th>
                      <th scope="col" className="py-2 text-right font-semibold">Amount</th>
                    </tr>
                  </thead>
                  <tbody>
                    {donations.map((d) => (
                      <tr key={d.id} className="border-b border-border">
                        <td className="py-2.5 pr-4 font-medium">
                          {d.is_anonymous ? 'Anonymous' : d.donor_name || 'Anonymous'}
                          {d.earmarked_node && (
                            <span className="block text-xs font-normal text-muted-foreground">
                              For {d.earmarked_node.name}
                            </span>
                          )}
                        </td>
                        <td className="py-2.5 pr-4 text-muted-foreground tabular-nums">
                          {formatShortDate(d.created_at)}
                        </td>
                        <td className="py-2.5 pr-4">
                          <PaymentStatusMark status={d.payment_status} />
                          <span className="block text-xs text-muted-foreground">
                            {METHOD_LABEL[d.payment_method ?? ''] ?? d.payment_method}
                          </span>
                          {d.payment_status === 'pending' && (
                            <Button
                              variant="outline"
                              size="sm"
                              className="mt-1"
                              disabled={updateDonation.isPending}
                              onClick={() => void confirmDonation(d.id)}
                            >
                              Confirm received
                            </Button>
                          )}
                        </td>
                        <td className="py-2.5 text-right font-semibold tabular-nums">
                          {formatMoney(d.amount, d.currency)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </ReportSection>

          <ReportSection title="Recent programme spending" meta={<ProofLegend />}>
            {report.isLoading ? (
              <ReportNotice title="Loading spending" />
            ) : report.error ? (
              <ReportNotice tone="error" title="Spending could not be loaded">
                {report.error.message}
              </ReportNotice>
            ) : report.data && report.data.spending.length > 0 ? (
              <ProofSpendingList spending={report.data.spending} />
            ) : (
              <ReportNotice title="No spending recorded yet">
                Payments out of this campaign's projects appear here with their
                proof once they are recorded as transactions.
              </ReportNotice>
            )}
          </ReportSection>
        </div>

        <div className="min-w-0 space-y-8">
          <ReportSection title="Details">
            <dl className="divide-y divide-border text-sm">
              {[
                ['Target', formatMoney(campaign.target_amount, campaign.currency)],
                ['Funds', campaign.target_node?.name ?? 'Not tied to one project'],
                ['Category', campaign.category ? campaign.category[0].toUpperCase() + campaign.category.slice(1) : 'None'],
                ['Started', formatDate(campaign.starts_at)],
                ['Ends', campaign.ends_at ? formatDate(campaign.ends_at) : 'No end date'],
                ['Visibility', campaign.is_public ? 'Public donate page' : 'Private'],
                ['After target', campaign.allow_overflow ? 'Keeps accepting gifts' : 'Closes to new gifts'],
              ].map(([k, v]) => (
                <div key={k} className="flex justify-between gap-4 py-2.5">
                  <dt className="text-muted-foreground">{k}</dt>
                  <dd className="text-right font-medium tabular-nums">{v}</dd>
                </div>
              ))}
            </dl>
            {publicUrl && (
              <div className="border-t border-border py-3">
                <p className="text-xs text-muted-foreground">Public link</p>
                <code className="mt-1 block font-mono text-xs break-all">{publicUrl}</code>
              </div>
            )}
          </ReportSection>

          {report.data && report.data.earmarks.length > 0 && (
            <ReportSection title="Where gifts can go">
              <EarmarkList earmarks={report.data.earmarks} currency={campaign.currency} />
            </ReportSection>
          )}
        </div>
      </div>

      <DonationModal
        campaign={campaign}
        earmarks={report.data?.earmarks}
        isOpen={donationOpen}
        onClose={() => setDonationOpen(false)}
      />

      <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete this draft?</AlertDialogTitle>
            <AlertDialogDescription>
              {campaign.name} has not been opened to donors. Deleting it cannot be
              undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep draft</AlertDialogCancel>
            <Button
              variant="destructive"
              disabled={deleteCampaign.isPending}
              onClick={() => void handleDelete()}
            >
              Delete draft
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
