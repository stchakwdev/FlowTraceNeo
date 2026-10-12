import { createFileRoute } from '@tanstack/react-router'
import { CampaignEmbedWidget } from '@/components/campaign-embed-widget'

// Every search param is optional. Returning undefined for an absent param
// (instead of a default) keeps the router from redirecting a bare URL to a
// canonical one with the defaults filled in; defaults apply in the component.
interface EmbedSearch {
  theme?: 'light' | 'dark'
  showAmount?: boolean
  showDonors?: boolean
  compact?: boolean
}

const flag = (v: unknown) =>
  v === undefined ? undefined : v === true || v === 'true' ? true : v === false || v === 'false' ? false : undefined

export const Route = createFileRoute('/embed/campaign/$id')({
  component: CampaignEmbedPage,
  validateSearch: (search: Record<string, unknown>): EmbedSearch => ({
    theme: search.theme === 'dark' || search.theme === 'light' ? search.theme : undefined,
    showAmount: flag(search.showAmount),
    showDonors: flag(search.showDonors),
    compact: flag(search.compact),
  }),
})

function CampaignEmbedPage() {
  const { id } = Route.useParams()
  const search = Route.useSearch()

  return (
    <div className="mx-auto max-w-md bg-background p-2">
      <CampaignEmbedWidget
        campaignId={id}
        theme={search.theme ?? 'light'}
        showAmount={search.showAmount ?? true}
        showDonors={search.showDonors ?? true}
        compact={search.compact ?? false}
      />
    </div>
  )
}
