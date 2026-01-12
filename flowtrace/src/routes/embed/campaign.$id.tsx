import { createFileRoute } from '@tanstack/react-router'
import { CampaignEmbedWidget } from '@/components/campaign-embed-widget'

export const Route = createFileRoute('/embed/campaign/$id')({
  component: CampaignEmbedPage,
  validateSearch: (search: Record<string, unknown>) => ({
    theme: (search.theme as 'light' | 'dark') || 'light',
    showAmount: search.showAmount !== 'false',
    showDonors: search.showDonors !== 'false',
    compact: search.compact === 'true',
  }),
})

function CampaignEmbedPage() {
  const { id } = Route.useParams()
  const { theme, showAmount, showDonors, compact } = Route.useSearch()

  return (
    <CampaignEmbedWidget
      campaignId={id}
      theme={theme}
      showAmount={showAmount}
      showDonors={showDonors}
      compact={compact}
    />
  )
}
