import { useEffect, useMemo, useRef, useState } from 'react'
import { Check, Copy } from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import type { BadgeStyle } from '@/lib/badge-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { ReportSection } from '@/components/report'
import { ReportCheckbox } from '@/components/report-dialog'
import { TrustBadgePreview } from '@/components/trust-badge-preview'
import { cn } from '@/lib/utils'

const ORIGIN_FALLBACK = 'https://flowtrace.app'
const origin = () =>
  typeof window !== 'undefined' ? window.location.origin : ORIGIN_FALLBACK

/** A code sample with a copy button that says what happened. */
export function CopyBlock({
  id,
  label,
  code,
}: {
  id: string
  label: string
  code: string
}) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle')
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const preRef = useRef<HTMLPreElement>(null)

  useEffect(() => () => clearTimeout(timer.current), [])
  useEffect(() => setState('idle'), [code])

  const copy = async () => {
    clearTimeout(timer.current)
    try {
      await navigator.clipboard.writeText(code)
      setState('copied')
      timer.current = setTimeout(() => setState('idle'), 2500)
    } catch {
      setState('failed')
      // Select the text so a manual copy is one keystroke away.
      const sel = window.getSelection()
      if (preRef.current && sel) {
        const range = document.createRange()
        range.selectNodeContents(preRef.current)
        sel.removeAllRanges()
        sel.addRange(range)
      }
    }
  }

  return (
    <div className="flex min-w-0 flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <span id={`${id}-label`} className="text-sm font-semibold">
          {label}
        </span>
        <Button
          variant={state === 'copied' ? 'default' : 'outline'}
          size="sm"
          className="h-10 min-w-28"
          onClick={copy}
          aria-describedby={`${id}-status`}
        >
          {state === 'copied' ? (
            <Check weight="bold" aria-hidden="true" />
          ) : (
            <Copy aria-hidden="true" />
          )}
          {state === 'copied' ? 'Copied' : 'Copy code'}
        </Button>
      </div>
      <pre
        ref={preRef}
        tabIndex={0}
        aria-labelledby={`${id}-label`}
        className="max-h-72 overflow-auto border border-border bg-muted p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere] outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <code>{code}</code>
      </pre>
      <p id={`${id}-status`} aria-live="polite" className="min-h-4 text-xs">
        {state === 'copied' && (
          <span className="text-muted-foreground">
            Copied to your clipboard. Paste it into your site’s HTML.
          </span>
        )}
        {state === 'failed' && (
          <span className="text-destructive">
            Your browser blocked copying. The code is selected: press Ctrl+C (or
            Cmd+C) to copy it.
          </span>
        )}
      </p>
    </div>
  )
}

function Option({
  checked,
  onChange,
  label,
}: {
  checked: boolean
  onChange: (v: boolean) => void
  label: string
}) {
  return (
    <label className="flex min-h-10 cursor-pointer items-center gap-3 text-sm">
      <ReportCheckbox checked={checked} onCheckedChange={onChange} />
      {label}
    </label>
  )
}

/** Campaign widget: options, a live preview of the real widget, and code. */
export function CampaignWidgetEmbed({
  campaignId,
  campaignName,
  picker,
  className,
}: {
  campaignId?: string
  campaignName?: string
  /** The campaign chooser, rendered at the top of the options. */
  picker?: ReactNode
  className?: string
}) {
  const [theme, setTheme] = useState<'light' | 'dark'>('light')
  const [showAmount, setShowAmount] = useState(true)
  const [showDonors, setShowDonors] = useState(true)
  const [compact, setCompact] = useState(false)

  const query = useMemo(() => {
    const params = new URLSearchParams()
    params.set('theme', theme)
    if (!showAmount) params.set('showAmount', 'false')
    if (!showDonors) params.set('showDonors', 'false')
    if (compact) params.set('compact', 'true')
    return params.toString()
  }, [theme, showAmount, showDonors, compact])

  const height = compact ? 80 : 440
  const frameRef = useRef<HTMLIFrameElement>(null)
  const [previewHeight, setPreviewHeight] = useState<number | null>(null)

  // The widget reports its height; size the preview to fit, as the
  // snippet does on the host site.
  useEffect(() => {
    const onMessage = (e: MessageEvent) => {
      const data = e.data as { type?: string; height?: number } | null
      if (
        e.source === frameRef.current?.contentWindow &&
        data?.type === 'flowtrace-embed-height' &&
        typeof data.height === 'number'
      ) {
        setPreviewHeight(Math.ceil(data.height))
      }
    }
    window.addEventListener('message', onMessage)
    return () => window.removeEventListener('message', onMessage)
  }, [])
  const path = campaignId ? `/embed/campaign/${campaignId}?${query}` : null

  const code = path
    ? `<iframe
  src="${origin()}${path}"
  width="100%"
  height="${height}"
  style="border: 0;"
  loading="lazy"
  title="${(campaignName ?? 'Campaign').replace(/"/g, '&quot;')} on FlowTrace"
></iframe>
<script>
  window.addEventListener('message', function (e) {
    if (e.data && e.data.type === 'flowtrace-embed-height') {
      var frame = document.querySelector('iframe[src*="/embed/campaign/${campaignId}"]');
      if (frame) frame.style.height = e.data.height + 'px';
    }
  });
</script>`
    : ''

  return (
    <ReportSection title="Campaign widget" className={className}>
      <div className="grid gap-8 pt-5 lg:grid-cols-12">
        <div className="flex flex-col gap-5 lg:col-span-4">
          {picker}
          <div className="flex flex-col gap-2">
            <span id="embed-theme-label" className="text-sm font-semibold">
              Theme
            </span>
            <Select
              value={theme}
              items={{ light: 'Light', dark: 'Dark' }}
              onValueChange={(v) => v && setTheme(v)}
            >
              <SelectTrigger
                className="h-10 w-full"
                aria-labelledby="embed-theme-label"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="light">Light</SelectItem>
                <SelectItem value="dark">Dark</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <fieldset className="flex flex-col">
            <legend className="mb-1 text-sm font-semibold">Show</legend>
            <Option
              checked={showAmount}
              onChange={setShowAmount}
              label="Amount raised and goal"
            />
            <Option
              checked={showDonors}
              onChange={setShowDonors}
              label="Number of donors"
            />
            <Option
              checked={compact}
              onChange={setCompact}
              label="Compact, one line"
            />
          </fieldset>
        </div>

        <div className="flex min-w-0 flex-col gap-6 lg:col-span-8">
          {path ? (
            <>
              <div className="flex flex-col gap-2">
                <span className="text-sm font-semibold">Live preview</span>
                <div
                  className={cn(
                    'border border-dashed border-input p-3',
                    theme === 'dark' ? 'bg-masthead' : 'bg-card',
                  )}
                >
                  <iframe
                    ref={frameRef}
                    key={path}
                    src={path}
                    title={`Preview of the ${campaignName ?? 'campaign'} widget`}
                    width="100%"
                    height={previewHeight ?? height}
                    className="block border-0"
                  />
                </div>
                <p className="text-xs text-muted-foreground">
                  This is the real widget. It updates as donations arrive.
                </p>
              </div>
              <CopyBlock id="embed-widget" label="Embed code" code={code} />
            </>
          ) : (
            <p className="border border-dashed border-input p-6 text-sm text-muted-foreground">
              Choose a campaign to preview its widget and get the code.
            </p>
          )}
        </div>
      </div>
    </ReportSection>
  )
}

const BADGE_STYLES: Record<BadgeStyle, string> = {
  flat: 'Flat',
  'flat-square': 'Flat, square corners',
  'for-the-badge': 'Large',
  plastic: 'Plastic',
}

/** Trust Score badge: style, label, live preview, and Markdown/HTML code. */
export function TrustBadgeEmbed({
  organizationId,
  className,
}: {
  organizationId: string
  className?: string
}) {
  const [style, setStyle] = useState<BadgeStyle>('flat')
  const [label, setLabel] = useState('Trust Score')
  const shownLabel = label.trim() || 'Trust Score'

  const base = origin()
  const params = new URLSearchParams({ style, label: shownLabel }).toString()
  const badgeUrl = `${base}/badge/trust/${organizationId}?${params}`
  const profileUrl = `${base}/badge/trust/${organizationId}`
  const markdown = `[![${shownLabel}](${badgeUrl})](${profileUrl})`
  const html = `<a href="${profileUrl}" target="_blank" rel="noopener noreferrer">
  <img src="${badgeUrl}" alt="${shownLabel.replace(/"/g, '&quot;')} on FlowTrace" />
</a>`

  return (
    <ReportSection title="Trust Score badge" className={className}>
      <div className="grid gap-8 pt-5 lg:grid-cols-12">
        <div className="flex flex-col gap-5 lg:col-span-4">
          <div className="flex flex-col gap-2">
            <span id="badge-style-label" className="text-sm font-semibold">
              Style
            </span>
            <Select
              value={style}
              items={BADGE_STYLES}
              onValueChange={(v) => v && setStyle(v as BadgeStyle)}
            >
              <SelectTrigger
                className="h-10 w-full"
                aria-labelledby="badge-style-label"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(Object.keys(BADGE_STYLES) as Array<BadgeStyle>).map((s) => (
                  <SelectItem key={s} value={s}>
                    {BADGE_STYLES[s]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-2">
            <label htmlFor="badge-label" className="text-sm font-semibold">
              Label
            </label>
            <Input
              id="badge-label"
              value={label}
              maxLength={24}
              onChange={(e) => setLabel(e.target.value)}
              className="h-10"
              aria-describedby="badge-label-hint"
            />
            <p id="badge-label-hint" className="text-xs text-muted-foreground">
              The words on the left of the badge. Up to 24 characters.
            </p>
          </div>
        </div>

        <div className="flex min-w-0 flex-col gap-6 lg:col-span-8">
          <div className="flex flex-col gap-2">
            <span className="text-sm font-semibold">Live preview</span>
            <div className="flex min-h-16 items-center border border-dashed border-input bg-card p-4">
              {organizationId ? (
                <TrustBadgePreview
                  organizationId={organizationId}
                  style={style}
                  label={shownLabel}
                />
              ) : (
                <span className="text-sm text-muted-foreground">
                  The badge appears once your organisation has loaded.
                </span>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              Shows your current Trust Score, and updates when it changes.
            </p>
          </div>
          <CopyBlock id="badge-html" label="HTML, for websites" code={html} />
          <CopyBlock
            id="badge-md"
            label="Markdown, for READMEs and docs"
            code={markdown}
          />
        </div>
      </div>
    </ReportSection>
  )
}

interface EmbedCodeGeneratorProps {
  campaignId?: string
  campaignSlug?: string
  campaignName?: string
  organizationId: string
  className?: string
}

/** Both embeds together: the campaign widget (when a campaign is chosen) and the badge. */
export function EmbedCodeGenerator({
  campaignId,
  campaignName,
  organizationId,
  className,
}: EmbedCodeGeneratorProps) {
  return (
    <div className={cn('flex flex-col gap-10', className)}>
      {campaignId && (
        <CampaignWidgetEmbed
          campaignId={campaignId}
          campaignName={campaignName}
        />
      )}
      <TrustBadgeEmbed organizationId={organizationId} />
    </div>
  )
}
