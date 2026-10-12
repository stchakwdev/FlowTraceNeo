import type { ReactNode } from 'react'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'

/**
 * A report section: small-caps title sitting on a heavy ink rule, with
 * optional actions on the right. This is the app's basic container.
 */
export function ReportSection({
  title,
  actions,
  meta,
  children,
  className,
  as: Tag = 'section',
  id,
}: {
  id?: string
  title: ReactNode
  actions?: ReactNode
  meta?: ReactNode
  children: ReactNode
  className?: string
  as?: 'section' | 'div'
}) {
  return (
    <Tag id={id} className={cn('rule-heavy scroll-mt-28', className)}>
      <div className="flex min-h-11 flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-border py-2">
        <h2 className="section-label font-sans [font-stretch:100%]">{title}</h2>
        {(meta || actions) && (
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            {meta}
            {actions}
          </div>
        )}
      </div>
      {children}
    </Tag>
  )
}

/** Marks figures drawn from the bundled demo dataset. */
export function DemoTag({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center border border-border px-1.5 text-[0.6875rem] font-semibold tracking-[0.12em] text-muted-foreground uppercase',
        className,
      )}
    >
      Demo data
    </span>
  )
}

/** "as of" date stamp, the sitrep's way of dating every figure. */
export function AsOf({
  date,
  className,
}: {
  date: Date | string
  className?: string
}) {
  const d = typeof date === 'string' ? new Date(date) : date
  return (
    <time
      dateTime={d.toISOString()}
      className={cn('text-xs text-muted-foreground', className)}
    >
      as of {formatDate(d)}
    </time>
  )
}

/**
 * A key figure. Size carries magnitude: `scale` lets the most important
 * figure in a row be set larger than its neighbours.
 */
export function KeyFigure({
  value,
  label,
  detail,
  scale = 'md',
  className,
}: {
  value: ReactNode
  label: ReactNode
  detail?: ReactNode
  scale?: 'sm' | 'md' | 'lg'
  className?: string
}) {
  return (
    <div className={cn('min-w-0', className)}>
      <p
        className={cn(
          'figure text-foreground',
          scale === 'sm' && 'text-3xl',
          scale === 'md' && 'text-4xl sm:text-[2.75rem]',
          scale === 'lg' && 'text-5xl sm:text-6xl',
        )}
      >
        {value}
      </p>
      <p className="mt-2 text-sm text-muted-foreground">{label}</p>
      {detail && <div className="mt-1 text-xs">{detail}</div>}
    </div>
  )
}

/** A row of key figures divided by hairlines, as in a sitrep header. */
export function KeyFigures({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        'grid grid-cols-2 border-b border-border sm:grid-cols-4 [&>*]:border-border [&>*]:py-4 [&>*:nth-child(odd)]:pr-4 [&>*:nth-child(even)]:border-l [&>*:nth-child(even)]:pl-4 sm:[&>*]:pr-4 sm:[&>*+*]:border-l sm:[&>*+*]:pl-4 sm:[&>*:first-child]:pl-0',
        className,
      )}
    >
      {children}
    </div>
  )
}

/**
 * Funding coverage bar: how much of `total` has `filled`, optionally with an
 * `evidenced` share drawn darker. A pending amount renders as a dashed line.
 */
export function CoverageBar({
  total,
  filled,
  evidenced,
  pending = false,
  className,
  label,
}: {
  total: number
  filled: number
  evidenced?: number
  pending?: boolean
  className?: string
  label?: string
}) {
  const pct = (n: number) =>
    total > 0 ? Math.max(0, Math.min(100, (n / total) * 100)) : 0
  if (pending) {
    return (
      <div
        role="img"
        aria-label={label ?? 'Pending, not yet received'}
        className={cn('flex h-5 items-center', className)}
      >
        <svg width="100%" height="4" aria-hidden="true" className="block">
          <line
            x1="0"
            y1="2"
            x2="100%"
            y2="2"
            strokeWidth="3"
            strokeDasharray="8 6"
            className="stroke-muted-foreground"
          />
        </svg>
      </div>
    )
  }
  return (
    <div
      role="img"
      aria-label={label ?? `${Math.round(pct(filled))}% of total`}
      className={cn('relative h-5 bg-gap', className)}
    >
      <div
        className="absolute inset-y-0 left-0 bg-report"
        style={{ width: `${pct(filled)}%` }}
      />
      {evidenced !== undefined && (
        <div
          className="absolute inset-y-0 left-0 bg-primary"
          style={{ width: `${pct(evidenced)}%` }}
        />
      )}
    </div>
  )
}

/** Empty or failed state set in the report's own voice. */
export function ReportNotice({
  title,
  children,
  action,
  tone = 'neutral',
  className,
}: {
  title: ReactNode
  children?: ReactNode
  action?: ReactNode
  tone?: 'neutral' | 'error'
  className?: string
}) {
  return (
    <div
      role={tone === 'error' ? 'alert' : undefined}
      className={cn('py-10 text-center', className)}
    >
      <p
        className={cn(
          'font-semibold',
          tone === 'error' ? 'text-destructive' : 'text-foreground',
        )}
      >
        {title}
      </p>
      {children && (
        <p className="mx-auto mt-2 max-w-md text-sm text-muted-foreground">
          {children}
        </p>
      )}
      {action && <div className="mt-4 flex justify-center">{action}</div>}
    </div>
  )
}
