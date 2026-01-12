import { createFileRoute } from '@tanstack/react-router'
import { Code, Share } from '@phosphor-icons/react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { EmbedCodeGenerator } from '@/components/embed-code-generator'
import { BadgePreviewGrid } from '@/components/trust-badge-preview'
import { useCampaigns } from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { useState } from 'react'

export const Route = createFileRoute('/_authenticated/settings/embed')({
  component: EmbedSettingsPage,
})

function EmbedSettingsPage() {
  const { organizationId } = useOrg()
  const { data: campaigns } = useCampaigns()
  const [selectedCampaignId, setSelectedCampaignId] = useState<string>('')

  const activeCampaigns = campaigns?.filter((c) => c.status === 'active' && c.is_public) || []
  const selectedCampaign = activeCampaigns.find((c) => c.id === selectedCampaignId)

  return (
    <div className="mx-auto max-w-4xl p-4 md:p-6">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Share className="h-6 w-6" />
          Embed & Badges
        </h1>
        <p className="text-muted-foreground mt-1">
          Share your transparency data with embeddable widgets and trust badges
        </p>
      </div>

      {/* Campaign Selection */}
      {activeCampaigns.length > 0 && (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle className="text-lg">Campaign Widget</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label>Select a campaign to embed</Label>
              <Select
                value={selectedCampaignId}
                onValueChange={setSelectedCampaignId}
              >
                <SelectTrigger className="w-full sm:w-[300px]">
                  <SelectValue placeholder="Choose a campaign..." />
                </SelectTrigger>
                <SelectContent>
                  {activeCampaigns.map((campaign) => (
                    <SelectItem key={campaign.id} value={campaign.id}>
                      {campaign.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {selectedCampaign && (
              <div className="pt-2">
                <p className="text-sm text-muted-foreground mb-4">
                  Public URL:{' '}
                  <code className="px-2 py-0.5 bg-muted rounded text-xs">
                    /donate/{selectedCampaign.public_slug}
                  </code>
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Embed Code Generator */}
      <EmbedCodeGenerator
        campaignId={selectedCampaignId || undefined}
        campaignSlug={selectedCampaign?.public_slug || undefined}
        campaignName={selectedCampaign?.name}
        organizationId={organizationId || ''}
      />

      {/* Badge Preview Grid */}
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="text-lg">Badge Styles Preview</CardTitle>
        </CardHeader>
        <CardContent>
          {organizationId && (
            <BadgePreviewGrid organizationId={organizationId} />
          )}
        </CardContent>
      </Card>

      {/* Usage Tips */}
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-lg">
            <Code className="h-5 w-5" />
            Usage Tips
          </CardTitle>
        </CardHeader>
        <CardContent className="prose prose-sm max-w-none dark:prose-invert">
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li>
              <strong>Campaign Widgets:</strong> Embed on your website to show real-time
              donation progress and accept contributions.
            </li>
            <li>
              <strong>Trust Badges:</strong> Add to your README, email signature, or
              website footer to showcase your transparency score.
            </li>
            <li>
              <strong>Responsive Design:</strong> The campaign widget automatically
              adjusts its height based on content.
            </li>
            <li>
              <strong>Theme Support:</strong> Choose light or dark theme to match your
              website&apos;s design.
            </li>
          </ul>
        </CardContent>
      </Card>
    </div>
  )
}
