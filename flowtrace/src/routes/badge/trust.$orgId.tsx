import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import type { BadgeStyle } from '@/lib/badge-utils'
import { getPublicOrganization } from '@/lib/data'
import { TrustBadgePreview } from '@/components/trust-badge-preview'

// Search params are optional and undefined when absent, so a bare URL renders
// with defaults instead of being redirected to a canonical URL.
interface BadgeSearch {
  style?: BadgeStyle
  label?: string
  format?: 'svg' | 'preview'
}

const STYLES: Array<BadgeStyle> = [
  'flat',
  'flat-square',
  'for-the-badge',
  'plastic',
]

export const Route = createFileRoute('/badge/trust/$orgId')({
  component: TrustBadgePage,
  validateSearch: (search: Record<string, unknown>): BadgeSearch => ({
    style: STYLES.includes(search.style as BadgeStyle)
      ? (search.style as BadgeStyle)
      : undefined,
    label:
      typeof search.label === 'string' && search.label.trim()
        ? search.label.slice(0, 40)
        : undefined,
    format:
      search.format === 'svg' || search.format === 'preview'
        ? search.format
        : undefined,
  }),
})

function TrustBadgePage() {
  const { orgId } = Route.useParams()
  const search = Route.useSearch()

  const org = useQuery({
    queryKey: ['organization', orgId],
    queryFn: () => getPublicOrganization(orgId),
  })
  const orgName = org.data?.name

  // The badge is SVG text built from escaped input; served here as a preview,
  // drawn larger in a framed plate so it reads as a specimen, not a stray chip.
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
      <figure className="flex w-full max-w-xl flex-col items-center gap-6">
        <div className="flex min-h-48 w-full items-center justify-center overflow-hidden border border-border bg-card px-6 py-12 sm:min-h-56">
          <TrustBadgePreview
            organizationId={orgId}
            published
            style={search.style ?? 'flat'}
            label={search.label ?? 'Trust Score'}
            scale={2.5}
            className="max-w-full [&_svg]:h-auto [&_svg]:max-w-full"
          />
        </div>
        <figcaption className="text-center text-sm text-muted-foreground">
          Trust Score badge for{' '}
          <span className="font-semibold text-foreground">
            {orgName ?? (org.isLoading ? '…' : 'this organisation')}
          </span>
        </figcaption>
      </figure>
    </main>
  )
}
