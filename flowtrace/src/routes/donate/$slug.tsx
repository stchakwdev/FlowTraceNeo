import { Link, createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  DemoTag,
  KeyFigure,
  KeyFigures,
  ProofLegend,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { buttonVariants } from '@/components/ui/button'
import { CampaignCoverage } from '@/components/campaign-progress-bar'
import { campaignTiming } from '@/components/campaign-status-badge'
import {
  EarmarkList,
  ProofSpendingList,
  PublicMasthead,
} from '@/components/public-campaign-header'
import { PublicDonationForm } from '@/components/public-donation-form'
import { SocialShareButtons } from '@/components/social-share-buttons'
import { useCampaignStats, usePublicCampaign } from '@/lib/queries'
import { getCampaignReport } from '@/lib/data'
import { DEV_MODE } from '@/lib/auth'
import { formatDate } from '@/lib/format'

export const Route = createFileRoute('/donate/$slug')({
  component: PublicCampaignPage,
  head: () => ({
    meta: [
      { title: 'Give to this campaign | FlowTrace' },
      {
        name: 'description',
        content:
          'See what this campaign has raised, where the money goes and the proof behind each payment, then give.',
      },
    ],
  }),
})

function PublicCampaignPage() {
  const { slug } = Route.useParams()
  const { data: campaign, isLoading, error } = usePublicCampaign(slug)
  const { data: stats } = useCampaignStats(campaign?.id ?? '')
  const report = useQuery({
    queryKey: ['campaign-report', campaign?.id, campaign?.current_amount],
    queryFn: () => getCampaignReport(campaign!.id),
    enabled: !!campaign,
    staleTime: 30 * 1000,
  })

  const shell = (children: React.ReactNode) => (
    <div className="min-h-screen bg-background text-foreground">
      <PublicMasthead organization={report.data?.organization} />
      {children}
      <footer className="mt-16 border-t border-border">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-2 px-4 py-6 text-xs text-muted-foreground sm:px-8">
          <p>
            Every figure on this page comes from the organisation's own books in{' '}
            <Link to="/" className="font-semibold text-foreground underline underline-offset-4">
              FlowTrace
            </Link>
            .
          </p>
          {DEV_MODE && <DemoTag />}
        </div>
      </footer>
    </div>
  )

  if (isLoading) {
    return shell(
      <main className="mx-auto max-w-6xl px-4 py-10 sm:px-8">
        <ReportNotice title="Loading campaign" />
      </main>,
    )
  }

  if (error || !campaign) {
    return shell(
      <main className="mx-auto max-w-2xl px-4 py-10 sm:px-8">
        <ReportSection title="Campaign">
          <ReportNotice
            title="This campaign is not taking donations"
            action={
              <Link to="/" className={buttonVariants({ variant: 'outline', className: 'h-10' })}>
                Go to FlowTrace
              </Link>
            }
          >
            It may have closed or been paused, or the link may be mistyped. Ask the
            organisation for its current donate link.
          </ReportNotice>
        </ReportSection>
      </main>,
    )
  }

  const timing = campaignTiming(campaign)
  const shareUrl = typeof window !== 'undefined' ? window.location.href : `/donate/${slug}`
  const orgName = report.data?.organization.name

  return shell(
    <main className="mx-auto max-w-6xl px-4 py-8 sm:px-8 sm:py-10">
      <div className="max-w-3xl">
        <h1 className="text-4xl text-balance sm:text-6xl">{campaign.name}</h1>
        {campaign.short_description && (
          <p className="mt-3 text-lg text-foreground/80">{campaign.short_description}</p>
        )}
        <p className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
          {orgName && <span>A campaign by {orgName}</span>}
          <span>Started {formatDate(campaign.starts_at)}</span>
          <span className={timing.ended ? 'font-semibold text-foreground' : undefined}>
            {timing.label}
          </span>
          {DEV_MODE && <DemoTag />}
        </p>
      </div>

      <div className="mt-8 grid gap-x-10 gap-y-8 lg:grid-cols-12">
        <ReportSection title="Raised so far" className="lg:col-span-7">
          <div className="py-5">
            <CampaignCoverage
              raised={campaign.current_amount}
              target={campaign.target_amount}
              evidenced={report.data?.evidenced_amount}
              currency={campaign.currency}
              size="lg"
            />
          </div>
          <KeyFigures className="grid-cols-3 border-t sm:grid-cols-3 [&>*:nth-child(3)]:border-l [&>*:nth-child(3)]:pl-4">
            <KeyFigure scale="sm" value={stats?.donation_count ?? '–'} label="Gifts" />
            <KeyFigure scale="sm" value={stats?.unique_donors ?? '–'} label="Donors" />
            <KeyFigure
              scale="sm"
              value={timing.daysLeft ?? (timing.ended ? 'Ended' : 'Open')}
              label={
                timing.daysLeft !== null
                  ? 'Days left'
                  : timing.ended && campaign.ends_at
                    ? formatDate(campaign.ends_at)
                    : 'No end date'
              }
            />
          </KeyFigures>
        </ReportSection>

        <aside className="lg:col-span-5 lg:col-start-8 lg:row-span-5 lg:row-start-1">
          <div className="lg:sticky lg:top-6">
            <ReportSection title="Give to this campaign">
              <div className="py-4">
                {timing.ended ? (
                  <p className="text-sm text-muted-foreground">
                    This campaign closed on {formatDate(campaign.ends_at!)}. Thank you
                    to everyone who gave. Its spending and proof stay on this page.
                  </p>
                ) : (
                  <PublicDonationForm
                    campaign={campaign}
                    earmarks={report.data?.earmarks}
                  />
                )}
              </div>
            </ReportSection>
          </div>
        </aside>

        {campaign.description && (
          <ReportSection title="The story" className="lg:col-span-7">
            <p className="max-w-[68ch] py-4 text-base leading-relaxed whitespace-pre-wrap">
              {campaign.description}
            </p>
          </ReportSection>
        )}

        <ReportSection
          title="Where the money goes"
          className="lg:col-span-7"
          meta={<span>Received and evidenced, {campaign.currency}</span>}
        >
          {report.isLoading ? (
            <ReportNotice title="Loading projects" />
          ) : report.error ? (
            <ReportNotice tone="error" title="Projects could not be loaded">
              {report.error.message}. Reload the page to try again.
            </ReportNotice>
          ) : report.data && report.data.earmarks.length > 0 ? (
            <>
              <EarmarkList earmarks={report.data.earmarks} currency={campaign.currency} />
              <div className="flex flex-wrap gap-x-5 gap-y-1 border-t border-border py-2 text-xs text-muted-foreground">
                <span className="inline-flex items-center gap-1.5">
                  <span aria-hidden="true" className="size-2.5 bg-primary" /> With verified proof
                </span>
                <span className="inline-flex items-center gap-1.5">
                  <span aria-hidden="true" className="size-2.5 bg-report" /> Received, proof to come
                </span>
              </div>
            </>
          ) : (
            <ReportNotice title="No projects listed yet">
              Projects this organisation funds appear here once it records them.
            </ReportNotice>
          )}
        </ReportSection>

        <ReportSection
          title="Recent spending, with proof"
          className="lg:col-span-7"
          meta={<ProofLegend className="hidden sm:flex" />}
        >
          {report.isLoading ? (
            <ReportNotice title="Loading spending" />
          ) : report.error ? (
            <ReportNotice tone="error" title="Spending could not be loaded">
              {report.error.message}. Reload the page to try again.
            </ReportNotice>
          ) : report.data && report.data.spending.length > 0 ? (
            <>
              <ProofSpendingList spending={report.data.spending} />
              <ProofLegend className="border-t border-border py-2 sm:hidden" />
            </>
          ) : (
            <ReportNotice title="No spending recorded yet">
              Payments appear here, with their receipts and photos, as the
              organisation spends the money.
            </ReportNotice>
          )}
        </ReportSection>

        <ReportSection title="Share" className="lg:col-span-7">
          <div className="py-4">
            <SocialShareButtons
              url={shareUrl}
              title={`Support ${campaign.name}`}
              text={campaign.short_description || `Give to ${campaign.name}`}
            />
          </div>
        </ReportSection>
      </div>
    </main>,
  )
}
