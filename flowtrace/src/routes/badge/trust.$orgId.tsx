import { createFileRoute } from '@tanstack/react-router'
import { TrustBadgePreview } from '@/components/trust-badge-preview'

export const Route = createFileRoute('/badge/trust/$orgId')({
  component: TrustBadgePage,
  validateSearch: (search: Record<string, unknown>) => ({
    style: (search.style as 'flat' | 'flat-square' | 'for-the-badge' | 'plastic') || 'flat',
    label: (search.label as string) || 'Trust Score',
    format: (search.format as 'svg' | 'preview') || 'preview',
  }),
})

function TrustBadgePage() {
  const { orgId } = Route.useParams()
  const { style, label } = Route.useSearch()

  // For SVG format, we'd return raw SVG - but for now show preview
  // In production, this could be a server-rendered SVG endpoint

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <TrustBadgePreview
        organizationId={orgId}
        style={style}
        label={label}
      />
    </div>
  )
}
