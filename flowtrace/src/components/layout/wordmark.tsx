import { cn } from '@/lib/utils'

/**
 * FlowTrace mark: three stations on one line, the first link solid
 * (evidenced) and the second dashed (awaiting proof).
 */
export function FlowTraceMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 28 12"
      aria-hidden="true"
      className={cn('h-3 w-7 shrink-0', className)}
    >
      <line x1="3" y1="6" x2="14" y2="6" stroke="currentColor" strokeWidth="2.5" />
      <line
        x1="14"
        y1="6"
        x2="25"
        y2="6"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeDasharray="3 2.5"
      />
      <rect x="0.5" y="3.5" width="5" height="5" fill="currentColor" />
      <rect
        x="11.5"
        y="3.5"
        width="5"
        height="5"
        className="fill-background"
        stroke="currentColor"
        strokeWidth="2"
      />
      <rect
        x="22.5"
        y="3.5"
        width="5"
        height="5"
        className="fill-background"
        stroke="currentColor"
        strokeWidth="2"
      />
    </svg>
  )
}

export function Wordmark({
  className,
  collapsed = false,
}: {
  className?: string
  collapsed?: boolean
}) {
  return (
    <span className={cn('inline-flex items-center gap-2', className)}>
      <FlowTraceMark />
      {!collapsed && (
        <span className="text-lg font-extrabold tracking-[-0.01em] [font-stretch:87%]">
          FlowTrace
        </span>
      )}
    </span>
  )
}
