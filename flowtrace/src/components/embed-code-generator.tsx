import { useState, useMemo } from 'react'
import { Copy, Check, Code, Image } from '@phosphor-icons/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'
import type { BadgeStyle } from '@/lib/badge-utils'

interface EmbedCodeGeneratorProps {
  campaignId?: string
  campaignSlug?: string
  campaignName?: string
  organizationId: string
  className?: string
}

export function EmbedCodeGenerator({
  campaignId,
  campaignSlug,
  campaignName,
  organizationId,
  className,
}: EmbedCodeGeneratorProps) {
  const [copiedEmbed, setCopiedEmbed] = useState(false)
  const [copiedBadge, setCopiedBadge] = useState(false)

  // Embed settings
  const [embedTheme, setEmbedTheme] = useState<'light' | 'dark'>('light')
  const [showAmount, setShowAmount] = useState(true)
  const [showDonors, setShowDonors] = useState(true)
  const [compact, setCompact] = useState(false)

  // Badge settings
  const [badgeStyle, setBadgeStyle] = useState<BadgeStyle>('flat')
  const [badgeLabel, setBadgeLabel] = useState('Trust Score')

  const baseUrl = typeof window !== 'undefined' ? window.location.origin : 'https://flowtrace.app'

  // Generate embed URL
  const embedUrl = useMemo(() => {
    if (!campaignId) return null
    const params = new URLSearchParams()
    params.set('theme', embedTheme)
    if (!showAmount) params.set('showAmount', 'false')
    if (!showDonors) params.set('showDonors', 'false')
    if (compact) params.set('compact', 'true')
    return `${baseUrl}/embed/campaign/${campaignId}?${params.toString()}`
  }, [campaignId, embedTheme, showAmount, showDonors, compact, baseUrl])

  // Generate embed code
  const embedCode = useMemo(() => {
    if (!embedUrl) return null
    return `<iframe
  src="${embedUrl}"
  width="100%"
  height="${compact ? '80' : '320'}"
  frameborder="0"
  style="border: none; border-radius: 8px;"
  title="${campaignName || 'Campaign Widget'}"
></iframe>

<script>
  window.addEventListener('message', function(e) {
    if (e.data.type === 'flowtrace-embed-height') {
      const iframe = document.querySelector('iframe[src*="flowtrace"]');
      if (iframe) iframe.style.height = e.data.height + 'px';
    }
  });
</script>`
  }, [embedUrl, compact, campaignName])

  // Generate badge URL
  const badgeUrl = useMemo(() => {
    const params = new URLSearchParams()
    params.set('style', badgeStyle)
    params.set('label', badgeLabel)
    return `${baseUrl}/badge/trust/${organizationId}?${params.toString()}`
  }, [organizationId, badgeStyle, badgeLabel, baseUrl])

  // Generate badge markdown
  const badgeMarkdown = useMemo(() => {
    const profileUrl = `${baseUrl}/org/${organizationId}`
    return `[![Trust Score](${badgeUrl})](${profileUrl})`
  }, [badgeUrl, organizationId, baseUrl])

  // Generate badge HTML
  const badgeHtml = useMemo(() => {
    const profileUrl = `${baseUrl}/org/${organizationId}`
    return `<a href="${profileUrl}" target="_blank" rel="noopener noreferrer">
  <img src="${badgeUrl}" alt="Trust Score" />
</a>`
  }, [badgeUrl, organizationId, baseUrl])

  const handleCopyEmbed = async () => {
    if (!embedCode) return
    try {
      await navigator.clipboard.writeText(embedCode)
      setCopiedEmbed(true)
      setTimeout(() => setCopiedEmbed(false), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  const handleCopyBadge = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedBadge(true)
      setTimeout(() => setCopiedBadge(false), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return (
    <div className={cn('space-y-6', className)}>
      {/* Campaign Embed Widget */}
      {campaignId && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-lg">
              <Code className="h-5 w-5" />
              Campaign Embed Widget
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Settings */}
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label>Theme</Label>
                <Select
                  value={embedTheme}
                  onValueChange={(v) => setEmbedTheme(v as 'light' | 'dark')}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label>Display Options</Label>
                <div className="space-y-2">
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={showAmount}
                      onChange={(e) => setShowAmount(e.target.checked)}
                      className="rounded border-input"
                    />
                    Show amounts
                  </label>
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={showDonors}
                      onChange={(e) => setShowDonors(e.target.checked)}
                      className="rounded border-input"
                    />
                    Show donor count
                  </label>
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={compact}
                      onChange={(e) => setCompact(e.target.checked)}
                      className="rounded border-input"
                    />
                    Compact mode
                  </label>
                </div>
              </div>
            </div>

            {/* Preview */}
            <div className="border rounded-lg p-4 bg-muted/50">
              <p className="text-xs text-muted-foreground mb-2">Preview URL:</p>
              <code className="text-xs break-all">{embedUrl}</code>
            </div>

            {/* Code */}
            <div className="relative">
              <pre className="p-4 bg-muted rounded-lg overflow-x-auto text-xs">
                <code>{embedCode}</code>
              </pre>
              <Button
                size="sm"
                variant="ghost"
                onClick={handleCopyEmbed}
                className="absolute top-2 right-2"
              >
                {copiedEmbed ? (
                  <Check className="h-4 w-4 text-emerald-600" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Trust Badge */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-lg">
            <Image className="h-5 w-5" />
            Trust Score Badge
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Settings */}
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>Style</Label>
              <Select
                value={badgeStyle}
                onValueChange={(v) => setBadgeStyle(v as BadgeStyle)}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="flat">Flat</SelectItem>
                  <SelectItem value="flat-square">Flat Square</SelectItem>
                  <SelectItem value="for-the-badge">For the Badge</SelectItem>
                  <SelectItem value="plastic">Plastic</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Label</Label>
              <Input
                value={badgeLabel}
                onChange={(e) => setBadgeLabel(e.target.value)}
                placeholder="Trust Score"
              />
            </div>
          </div>

          {/* Preview */}
          <div className="border rounded-lg p-4 bg-muted/50">
            <p className="text-xs text-muted-foreground mb-2">Preview:</p>
            <img src={badgeUrl} alt="Badge preview" className="h-5" />
          </div>

          {/* Markdown */}
          <div className="space-y-2">
            <Label>Markdown (for README)</Label>
            <div className="relative">
              <pre className="p-3 bg-muted rounded-lg overflow-x-auto text-xs">
                <code>{badgeMarkdown}</code>
              </pre>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleCopyBadge(badgeMarkdown)}
                className="absolute top-1 right-1"
              >
                {copiedBadge ? (
                  <Check className="h-4 w-4 text-emerald-600" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>

          {/* HTML */}
          <div className="space-y-2">
            <Label>HTML</Label>
            <div className="relative">
              <pre className="p-3 bg-muted rounded-lg overflow-x-auto text-xs">
                <code>{badgeHtml}</code>
              </pre>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => handleCopyBadge(badgeHtml)}
                className="absolute top-1 right-1"
              >
                {copiedBadge ? (
                  <Check className="h-4 w-4 text-emerald-600" />
                ) : (
                  <Copy className="h-4 w-4" />
                )}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
