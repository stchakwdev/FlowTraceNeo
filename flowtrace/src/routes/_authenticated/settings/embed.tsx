import { useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import { ArrowLeft, CircleNotch } from '@phosphor-icons/react'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ReportNotice, ReportSection } from '@/components/report'
import {
  CampaignWidgetEmbed,
  TrustBadgeEmbed,
} from '@/components/embed-code-generator'
import { useCampaigns } from '@/lib/queries'
import { useOrg } from '@/lib/org-context'

export const Route = createFileRoute('/_authenticated/settings/embed')({
  component: EmbedSettingsPage,
})

function EmbedSettingsPage() {
  const { organizationId, organization } = useOrg()
  const campaigns = useCampaigns()
  const [chosenId, setChosenId] = useState<string | null>(null)

  const embeddable =
    campaigns.data?.filter((c) => c.status === 'active' && c.is_public) ?? []
  const selected =
    embeddable.find((c) => c.id === chosenId) ?? embeddable.at(0) ?? null

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-10 px-4 py-6 sm:px-8 sm:py-8">
      <header>
        <Link
          to="/settings"
          className="mb-3 inline-flex min-h-10 items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft aria-hidden="true" className="size-4" />
          Settings
        </Link>
        <h1 className="text-4xl sm:text-5xl">Embed &amp; badges</h1>
        <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
          Put {organization?.name ?? 'your organisation'}’s live figures on your
          own website: a campaign widget that takes donations, and a badge that
          shows your Trust Score.
        </p>
      </header>

      {campaigns.isError ? (
        <ReportSection title="Campaign widget">
          <ReportNotice
            tone="error"
            title="Your campaigns could not be loaded"
            action={
              <Button
                variant="outline"
                className="h-10"
                onClick={() => campaigns.refetch()}
              >
                Try again
              </Button>
            }
          >
            {campaigns.error instanceof Error
              ? campaigns.error.message
              : 'Check your connection and try again.'}
          </ReportNotice>
        </ReportSection>
      ) : campaigns.isLoading ? (
        <ReportSection title="Campaign widget">
          <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
            <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
            Loading your campaigns
          </p>
        </ReportSection>
      ) : embeddable.length === 0 ? (
        <ReportSection title="Campaign widget">
          <ReportNotice
            title="No campaign is ready to embed"
            action={
              <Link
                to="/campaigns"
                className={buttonVariants({ className: 'h-10' })}
              >
                Go to campaigns
              </Link>
            }
          >
            A widget needs an active, public campaign. Publish one and its
            widget code appears here.
          </ReportNotice>
        </ReportSection>
      ) : (
        <CampaignWidgetEmbed
          campaignId={selected?.id}
          campaignName={selected?.name}
          picker={
            <div className="flex flex-col gap-2">
              <span id="embed-campaign-label" className="text-sm font-semibold">
                Campaign
              </span>
              <Select
                value={selected?.id ?? ''}
                items={Object.fromEntries(
                  embeddable.map((c) => [c.id, c.name]),
                )}
                onValueChange={(v) => v && setChosenId(v)}
              >
                <SelectTrigger
                  className="h-10 w-full"
                  aria-labelledby="embed-campaign-label"
                  aria-describedby="embed-campaign-hint"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {embeddable.map((c) => (
                    <SelectItem key={c.id} value={c.id}>
                      {c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {selected?.public_slug && (
                <p
                  id="embed-campaign-hint"
                  className="text-xs text-muted-foreground [overflow-wrap:anywhere]"
                >
                  Donation page:{' '}
                  <a
                    href={`/donate/${selected.public_slug}`}
                    className="font-mono text-foreground underline underline-offset-4 hover:text-primary"
                  >
                    /donate/{selected.public_slug}
                  </a>
                </p>
              )}
            </div>
          }
        />
      )}

      <TrustBadgeEmbed organizationId={organizationId ?? ''} />
    </div>
  )
}
