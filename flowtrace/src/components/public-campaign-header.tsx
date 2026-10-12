import { Link } from '@tanstack/react-router'
import { SealCheck } from '@phosphor-icons/react'
import type { CampaignReport, CampaignReportEarmark } from '@/lib/data'
import type { TransactionWithNodes } from '@/lib/supabase'
import { Wordmark } from '@/components/layout/wordmark'
import { CoverageBar, StatusMark } from '@/components/report'
import { formatDate, formatMoney, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'

/**
 * Masthead strip for public pages that sit outside the app shell: the
 * wordmark, then the organisation the page reports on and its Trust Score.
 */
export function PublicMasthead({
  organization,
}: {
  organization?: CampaignReport['organization']
}) {
  return (
    <header className="bg-masthead text-masthead-foreground">
      <div className="mx-auto flex min-h-14 max-w-6xl flex-wrap items-center justify-between gap-x-6 gap-y-2 px-4 py-2 sm:px-8">
        <Link to="/" aria-label="FlowTrace home" className="inline-flex items-center">
          <Wordmark />
        </Link>
        {organization && (
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
            <span className="inline-flex items-center gap-1.5 font-semibold">
              {organization.name}
              {organization.verified && (
                <SealCheck aria-label="Verified organisation" className="size-4" />
              )}
            </span>
            <span className="inline-flex items-baseline gap-1.5 border-l border-masthead-foreground/30 pl-3">
              <span className="text-masthead-foreground/75">Trust Score</span>
              <span className="figure text-lg">{organization.trust_score}</span>
              <span className="text-masthead-foreground/75">/100</span>
            </span>
          </p>
        )}
      </div>
    </header>
  )
}

/**
 * Where the money can go: each project or department a gift can be earmarked
 * to, with what it has received and how much of that is backed by proof.
 * Hovering or focusing one isolates it and dims the rest.
 */
export function EarmarkList({
  earmarks,
  currency = 'KES',
  className,
}: {
  earmarks: Array<CampaignReportEarmark>
  currency?: string
  className?: string
}) {
  // Projects that have received nothing say nothing about where money goes.
  // Each bar is drawn against that node's own receipts: the filled share is
  // the part backed by verified proof, so a fully evidenced node is full.
  const shown = earmarks.filter((e) => e.received > 0 || e.is_target)
  return (
    <ul
      className={cn(
        'divide-y divide-border [&:hover>li]:opacity-45 [&>li]:transition-opacity [&>li]:duration-150 [&>li:hover]:opacity-100 [&:focus-within>li]:opacity-45 [&>li:focus-within]:opacity-100',
        className,
      )}
    >
      {shown.map((e) => (
        <li key={e.id} tabIndex={0} className="py-3 outline-none focus-visible:bg-card">
          <div className="flex items-baseline justify-between gap-3 text-sm">
            <span className="min-w-0">
              <span className={cn('font-semibold', e.is_target && 'text-primary')}>
                {e.name}
              </span>
              {e.is_target && (
                <span className="ml-2 text-xs text-muted-foreground">this campaign</span>
              )}
            </span>
            <span className="shrink-0 font-semibold tabular-nums">
              {formatMoney(e.received, currency)}
            </span>
          </div>
          <CoverageBar
            total={Math.max(1, e.received)}
            filled={e.received}
            evidenced={e.evidenced}
            className="mt-1.5 h-2 bg-transparent"
            label={`${e.name}: ${formatMoney(e.received, currency)} received, ${formatMoney(e.evidenced, currency)} with proof`}
          />
          <p className="mt-1 text-xs text-muted-foreground tabular-nums">
            {e.received > 0
              ? `${formatMoney(e.evidenced, currency)} of it with verified proof (${Math.round((e.evidenced / e.received) * 100)}%)`
              : 'Nothing received yet'}
          </p>
        </li>
      ))}
    </ul>
  )
}

/** Recent programme spending, each line carrying its proof state. */
export function ProofSpendingList({
  spending,
  className,
}: {
  spending: Array<TransactionWithNodes>
  className?: string
}) {
  return (
    <ul className={cn('divide-y divide-border', className)}>
      {spending.map((t) => (
        <li
          key={t.id}
          className="grid grid-cols-[1fr_auto] gap-x-4 gap-y-1 py-3 text-sm"
        >
          <span className="min-w-0 font-semibold">{t.description}</span>
          <span className="text-right font-semibold tabular-nums">
            {formatMoney(t.amount, t.currency)}
          </span>
          <span className="min-w-0 text-xs text-muted-foreground">
            <time dateTime={t.transaction_date}>{formatShortDate(t.transaction_date)}</time>
            {t.from_node && t.to_node
              ? ` · ${t.from_node.name} to ${t.to_node.name}`
              : t.to_node
                ? ` · to ${t.to_node.name}`
                : ''}
          </span>
          <span className="flex flex-col items-end gap-0.5">
            <StatusMark status={t.verification_status} />
            {t.verification_count > 0 && (
              <span className="text-xs text-muted-foreground tabular-nums">
                {t.verification_count} {t.verification_count === 1 ? 'proof' : 'proofs'}
              </span>
            )}
          </span>
        </li>
      ))}
    </ul>
  )
}

/** "Started 1 Dec 2025" style line. */
export function campaignDates(startsAt: string, endsAt: string | null) {
  return `Started ${formatDate(startsAt)}${endsAt ? `, ends ${formatDate(endsAt)}` : ''}`
}
