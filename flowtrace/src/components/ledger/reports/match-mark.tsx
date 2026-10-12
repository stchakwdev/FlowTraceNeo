// Reconciliation state of a statement line, drawn as a line pattern plus its
// word so it reads in greyscale: matched automatically is a solid ink line,
// matched by a person a doubled ink line, a line that needs a decision is
// dashed, and an unexplained line is a long dash crossed with an X (a flag).
import { cn } from '@/lib/utils'

export type MatchState = 'auto' | 'manual' | 'ambiguous' | 'unmatched'

export const MATCH_LABEL: Record<MatchState, string> = {
  auto: 'Matched automatically',
  manual: 'Matched by hand',
  ambiguous: 'Needs a decision',
  unmatched: 'Unmatched',
}

export const MATCH_SHORT: Record<MatchState, string> = {
  auto: 'Auto',
  manual: 'By hand',
  ambiguous: 'Needs a decision',
  unmatched: 'Unmatched',
}

export function MatchLine({ state, width = 22, className }: { state: MatchState; width?: number; className?: string }) {
  const mid = width / 2
  return (
    <svg
      width={width}
      height="10"
      viewBox={`0 0 ${width} 10`}
      aria-hidden="true"
      className={cn('shrink-0 overflow-visible', className)}
    >
      {state === 'auto' && <line x1="0" y1="5" x2={width} y2="5" strokeWidth="3" className="stroke-foreground" />}
      {state === 'manual' && (
        <>
          <line x1="0" y1="2.5" x2={width} y2="2.5" strokeWidth="1.5" className="stroke-foreground" />
          <line x1="0" y1="7.5" x2={width} y2="7.5" strokeWidth="1.5" className="stroke-foreground" />
        </>
      )}
      {state === 'ambiguous' && (
        <line x1="0" y1="5" x2={width} y2="5" strokeWidth="3" strokeDasharray="6 5" className="stroke-muted-foreground" />
      )}
      {state === 'unmatched' && (
        <>
          <line x1="0" y1="5" x2={width} y2="5" strokeWidth="3" strokeDasharray="14 8" className="stroke-flag" />
          <path
            d={`M${mid - 2} 2 L${mid + 2} 8 M${mid + 2} 2 L${mid - 2} 8`}
            strokeWidth="2"
            className="stroke-flag"
          />
        </>
      )}
    </svg>
  )
}

export function MatchMark({ state, className }: { state: MatchState; className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 text-xs font-semibold whitespace-nowrap',
        state === 'unmatched' && 'text-flag',
        state === 'ambiguous' && 'text-muted-foreground',
        className,
      )}
    >
      <MatchLine state={state} />
      {MATCH_LABEL[state]}
    </span>
  )
}

export function MatchLegend({ className }: { className?: string }) {
  return (
    <div className={cn('flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted-foreground', className)}>
      {(['auto', 'manual', 'ambiguous', 'unmatched'] as const).map((s) => (
        <span key={s} className="inline-flex items-center gap-2">
          <MatchLine state={s} />
          {MATCH_LABEL[s]}
        </span>
      ))}
    </div>
  )
}
