import { Link, useLocation } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { LedgerError } from '@/lib/ledger'
import { BooksNotConnectedError } from '@/lib/ledger-client'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { ReportNotice } from '@/components/report'

/** Page frame for the Books screens: title, lede and actions, then content. */
export function BooksPage({
  title,
  lede,
  actions,
  nav,
  children,
  width = 'wide',
}: {
  title: ReactNode
  lede?: ReactNode
  actions?: ReactNode
  /** Sub-navigation shown under the header. */
  nav?: ReactNode
  children: ReactNode
  width?: 'wide' | 'narrow'
}) {
  return (
    <div
      className={cn(
        'mx-auto px-4 py-6 sm:px-8 sm:py-8',
        width === 'wide' ? 'max-w-7xl' : 'max-w-3xl',
      )}
    >
      <header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">{title}</h1>
          {lede && (
            <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
              {lede}
            </p>
          )}
        </div>
        {actions && (
          <div className="flex flex-wrap items-center gap-2 self-start sm:self-auto">
            {actions}
          </div>
        )}
      </header>
      {nav && <div className="mt-6">{nav}</div>}
      <div className="mt-8 space-y-10">{children}</div>
    </div>
  )
}

/**
 * Segmented sub-navigation between sibling screens, in the same style as
 * the transaction status filter.
 */
export function SubNav({
  label,
  items,
}: {
  label: string
  items: Array<{ to: string; label: string; count?: number; exact?: boolean }>
}) {
  const { pathname } = useLocation()
  return (
    <nav aria-label={label} className="overflow-x-auto">
      <ul className="inline-flex gap-px border border-border bg-border">
        {items.map((item) => {
          const active = item.exact
            ? pathname === item.to || pathname === `${item.to}/`
            : pathname === item.to || pathname.startsWith(`${item.to}/`)
          return (
            <li key={item.to} className="bg-background">
              <Link
                to={item.to}
                aria-current={active ? 'page' : undefined}
                className={cn(
                  'flex h-10 items-center gap-2 px-3 text-sm whitespace-nowrap transition-colors',
                  active
                    ? 'bg-primary font-semibold text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                )}
              >
                {item.label}
                {item.count !== undefined && (
                  <span className="tabular-nums opacity-80">{item.count}</span>
                )}
              </Link>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}

/** Loading rows for tables. */
export function LoadingRows({ rows = 6, label }: { rows?: number; label: string }) {
  return (
    <div aria-busy="true" aria-label={label}>
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="flex items-center gap-6 border-b border-border py-4">
          <div className="skeleton-shimmer h-4 w-20" />
          <div className="flex-1 space-y-2">
            <div className="skeleton-shimmer h-4 w-2/3 max-w-sm" />
            <div className="skeleton-shimmer h-3 w-1/3 max-w-40" />
          </div>
          <div className="skeleton-shimmer h-4 w-24" />
        </div>
      ))}
    </div>
  )
}

/** Error, not-connected and loading states for any screen that reads the books. */
export function BooksState({
  isLoading,
  error,
  refetch,
  label,
}: {
  isLoading: boolean
  error: unknown
  refetch: () => unknown
  label: string
}) {
  if (isLoading) return <LoadingRows label={`Loading ${label}`} />
  if (error instanceof BooksNotConnectedError)
    return (
      <ReportNotice title="The books are not connected yet">
        This organisation’s ledger lives in SpacetimeDB. Ask an administrator to
        connect it; until then the transparency screens still work.
        {error.identity && (
          <>
            {' '}
            To add this device as a member, give them its identity:{' '}
            <code className="font-mono text-xs break-all">0x{error.identity}</code>
          </>
        )}
      </ReportNotice>
    )
  return (
    <ReportNotice
      tone="error"
      title={`The ${label} could not be loaded`}
      action={
        <Button variant="outline" onClick={() => refetch()}>
          Try again
        </Button>
      }
    >
      {error instanceof Error ? `${error.message}. ` : ''}Check your connection,
      then try again.
    </ReportNotice>
  )
}

/**
 * What stopped a command, in the ledger's own words. Overridable issues
 * say so, so the form can offer an override with a reason.
 */
export function IssueSummary({
  error,
  title = 'Not saved',
  className,
}: {
  error: unknown
  title?: string
  className?: string
}) {
  if (!error) return null
  const issues =
    error instanceof LedgerError
      ? error.issues
      : [{ code: 'INVALID_INPUT', message: error instanceof Error ? error.message : String(error) }]
  return (
    <div
      role="alert"
      className={cn('border border-destructive p-4 text-sm', className)}
    >
      <p className="font-semibold text-destructive">{title}</p>
      <ul className="mt-2 space-y-1">
        {issues.map((i, n) => (
          <li key={n}>
            {i.message}
            {'overridable' in i && i.overridable && (
              <span className="ml-1 text-muted-foreground">
                (can be overridden with a reason)
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  )
}

export const hasOverridable = (error: unknown) =>
  error instanceof LedgerError &&
  error.issues.length > 0 &&
  error.issues.every((i) => i.overridable)
