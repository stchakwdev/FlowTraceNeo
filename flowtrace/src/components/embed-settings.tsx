import { CircleNotch } from '@phosphor-icons/react'
import type { PublicEmbedConfig } from '@/lib/types'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { ReportNotice, ReportSection } from '@/components/report'
import {
  usePublicEmbedConfig,
  useRegenerateEmbedToken,
  useUpdatePublicEmbedConfig,
} from '@/lib/queries'
import { formatDate } from '@/lib/format'

type Flag =
  | 'allow_embed'
  | 'show_amounts'
  | 'show_trust_score'
  | 'show_node_names'
  | 'show_transaction_count'

const FLAGS: Array<[Flag, string, string]> = [
  ['allow_embed', 'Allow embedding', 'Other websites may show your flows.'],
  ['show_amounts', 'Show amounts', 'Embeds show how much moved, not only where.'],
  ['show_trust_score', 'Show Trust Score', 'Embeds carry your current score.'],
  ['show_node_names', 'Show node names', 'Embeds name departments and partners.'],
  [
    'show_transaction_count',
    'Show transaction count',
    'Embeds say how many movements are behind the figures.',
  ],
]

/**
 * What the organisation's public embeds show (`my_public_embed_config`) and the token in their
 * URLs. Only an admin or manager may change them; the module says so when someone else tries.
 */
export function EmbedSettings() {
  const config = usePublicEmbedConfig()
  const update = useUpdatePublicEmbedConfig()
  const regenerate = useRegenerateEmbedToken()

  const failure = update.error ?? regenerate.error

  return (
    <ReportSection
      title="Embed settings"
      meta={
        config.data ? (
          <span>Last changed {formatDate(config.data.updated_at)}</span>
        ) : undefined
      }
    >
      {config.isError ? (
        <ReportNotice
          tone="error"
          title="Your embed settings could not be loaded"
          action={
            <Button
              variant="outline"
              className="h-10"
              onClick={() => config.refetch()}
            >
              Try again
            </Button>
          }
        >
          {config.error.message || 'Check your connection and try again.'}
        </ReportNotice>
      ) : config.isLoading ? (
        <p className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
          <CircleNotch className="size-4 animate-spin" aria-hidden="true" />
          Loading your embed settings
        </p>
      ) : !config.data ? (
        <ReportNotice
          title="No embed settings yet"
          action={
            <Button
              className="h-10"
              disabled={update.isPending}
              onClick={() => update.mutate({})}
            >
              Set up embeds
            </Button>
          }
        >
          Embeds use the standard settings until you save your own.
        </ReportNotice>
      ) : (
        <ConfigForm
          config={config.data}
          busy={update.isPending || regenerate.isPending}
          onChange={(patch) => update.mutate(patch)}
          onRegenerate={() => regenerate.mutate()}
        />
      )}
      {failure && (
        <p role="alert" className="pt-3 text-sm text-destructive">
          Not saved. {failure.message}
        </p>
      )}
    </ReportSection>
  )
}

function ConfigForm({
  config,
  busy,
  onChange,
  onRegenerate,
}: {
  config: PublicEmbedConfig
  busy: boolean
  onChange: (patch: Partial<Pick<PublicEmbedConfig, Flag>>) => void
  onRegenerate: () => void
}) {
  return (
    <div className="flex flex-col">
      <ul className="divide-y divide-border">
        {FLAGS.map(([key, label, hint]) => (
          <li key={key} className="flex items-center justify-between gap-4 py-3">
            <span>
              <label
                id={`embed-${key}-label`}
                htmlFor={`embed-${key}`}
                className="block font-semibold"
              >
                {label}
              </label>
              <span className="block text-sm text-muted-foreground">{hint}</span>
            </span>
            <Switch
              id={`embed-${key}`}
              aria-labelledby={`embed-${key}-label`}
              checked={config[key]}
              disabled={busy}
              onCheckedChange={(checked) => onChange({ [key]: checked })}
            />
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-center justify-between gap-4 border-t border-border pt-4">
        <p className="min-w-0 text-sm">
          <span className="block font-semibold">Embed token</span>
          <span className="block font-mono text-xs [overflow-wrap:anywhere] text-muted-foreground">
            {config.embed_token}
          </span>
        </p>
        <Button
          variant="outline"
          className="h-10"
          disabled={busy}
          onClick={onRegenerate}
        >
          New token
        </Button>
      </div>
      <p className="pt-2 text-xs text-muted-foreground">
        A new token stops every embed that uses the old one.
      </p>
    </div>
  )
}
