import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { ArrowRight, FlowArrow, Plus } from '@phosphor-icons/react'
import type { TransactionWithNodes } from '@/lib/supabase'
import type { DonorLine } from '@/lib/dashboard-summary'
import { DEMO_BOOKS } from '@/lib/ledger-client'
import { useOrg } from '@/lib/org-context'
import { useTransactions } from '@/lib/queries'
import { getTrustGrade } from '@/lib/supabase'
import { formatAmount, formatCompact, formatDate } from '@/lib/format'
import { summarise } from '@/lib/dashboard-summary'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ProofLegend,
  ReportNotice,
  ReportSection,
  StatusMark,
} from '@/components/report'

export const Route = createFileRoute('/_authenticated/dashboard')({
  component: Dashboard,
})

const RECENT_COUNT = 8

function Dashboard() {
  const { organization, isLoading: orgLoading, error: orgError } = useOrg()
  const txQuery = useTransactions()
  const [focused, setFocused] = useState<string | null>(null)

  const summary = useMemo(
    () => summarise(txQuery.data ?? []),
    [txQuery.data],
  )
  const recent = (txQuery.data ?? []).slice(0, RECENT_COUNT)
  const trustScore = organization?.trust_score ?? 0

  const loading = orgLoading || txQuery.isLoading
  const error = orgError ?? txQuery.error
  const isEmpty = !loading && !error && summary.total === 0

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <header className="mb-8 flex flex-wrap items-end justify-between gap-x-6 gap-y-4">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">Dashboard</h1>
          <p className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
            {organization?.name && <span>{organization.name}</span>}
            {summary.latest && <AsOf date={summary.latest} className="text-sm" />}
            {DEMO_BOOKS && <DemoTag />}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Link
            to="/transactions/new"
            className={cn(buttonVariants(), 'h-10')}
          >
            <Plus weight="bold" />
            Record a transaction
          </Link>
          <Link
            to="/flows"
            className={cn(buttonVariants({ variant: 'outline' }), 'h-10')}
          >
            <FlowArrow />
            Open the flow map
          </Link>
        </div>
      </header>

      {error ? (
        <ReportSection title="Key figures">
          <ReportNotice
            tone="error"
            title="We couldn't load your transactions"
            action={
              <Button
                variant="outline"
                className="h-10"
                onClick={() => txQuery.refetch()}
              >
                Try again
              </Button>
            }
          >
            The request failed{error.message ? ` (${error.message})` : ''}.
            Check your connection and try again; nothing you recorded is lost.
          </ReportNotice>
        </ReportSection>
      ) : loading ? (
        <DashboardLoading />
      ) : isEmpty ? (
        <ReportSection title="Key figures">
          <ReportNotice
            title="No money movements recorded yet"
            action={
              <Link
                to="/transactions/new"
                className={cn(buttonVariants(), 'h-10')}
              >
                <Plus weight="bold" />
                Record a transaction
              </Link>
            }
          >
            Your key figures, funding by donor and recent movements will
            appear here once you record your first transaction.
          </ReportNotice>
        </ReportSection>
      ) : (
        <div className="space-y-10">
          <ReportSection title="Key figures">
            <KeyFigures>
              <KeyFigure
                scale="lg"
                value={formatCompact(summary.evidencedTotal)}
                label="KES received with proof"
              />
              <KeyFigure
                value={`${summary.verified}/${summary.total}`}
                label="movements evidenced"
              />
              <KeyFigure
                value={summary.pending}
                label="awaiting proof"
                detail={
                  summary.flagged > 0 ? (
                    <StatusMark
                      status="flagged"
                      label={`${summary.flagged} flagged`}
                    />
                  ) : undefined
                }
              />
              <KeyFigure
                value={
                  <Link
                    to="/trust-score"
                    className="underline decoration-foreground/20 underline-offset-4 hover:decoration-foreground"
                  >
                    {trustScore}
                  </Link>
                }
                label={`Trust Score, grade ${getTrustGrade(trustScore)}`}
              />
            </KeyFigures>
          </ReportSection>

          <div className="grid gap-10 lg:grid-cols-12 [&>*]:min-w-0">
            <ReportSection
              title="Funding received, by donor"
              className="lg:col-span-5"
            >
              {summary.donorLines.length === 0 ? (
                <ReportNotice title="No donor funding recorded yet">
                  Record a transaction from a donor node and it will appear
                  here with its proof status.
                </ReportNotice>
              ) : (
                <>
                  <ul
                    className="divide-y divide-border"
                    onMouseLeave={() => setFocused(null)}
                    aria-label="Funding received, by donor"
                  >
                    {summary.donorLines.map((donor) => (
                      <DonorRow
                        key={donor.id}
                        donor={donor}
                        largest={summary.largest}
                        dimmed={focused !== null && focused !== donor.id}
                        onFocus={() => setFocused(donor.id)}
                        onBlur={() => setFocused(null)}
                      />
                    ))}
                  </ul>
                  <div className="mt-2 space-y-3 border-t border-border pt-3">
                    <ProofLegend />
                    <p className="text-xs text-muted-foreground tabular-nums">
                      KES {formatAmount(summary.evidencedTotal)} evidenced of{' '}
                      {formatAmount(summary.committedTotal)} committed
                    </p>
                  </div>
                </>
              )}
            </ReportSection>

            <ReportSection
              title="Recent movements"
              className="lg:col-span-7"
              actions={
                <Link
                  to="/transactions"
                  className="inline-flex min-h-10 items-center gap-1 text-sm font-medium text-primary hover:underline"
                >
                  All transactions
                  <ArrowRight className="size-3.5" />
                </Link>
              }
            >
              <RecentMovements
                transactions={recent}
                focusedDonor={focused}
              />
            </ReportSection>
          </div>
        </div>
      )}
    </div>
  )
}

function DonorRow({
  donor,
  largest,
  dimmed,
  onFocus,
  onBlur,
}: {
  donor: DonorLine
  largest: number
  dimmed: boolean
  onFocus: () => void
  onBlur: () => void
}) {
  const onlyPending = donor.received === 0 && donor.awaiting > 0
  const amount = donor.received + donor.awaiting
  const state = onlyPending
    ? 'awaiting proof'
    : donor.evidenced >= donor.received
      ? 'evidenced'
      : `KES ${formatAmount(donor.evidenced)} evidenced`
  return (
    <li>
      <button
        type="button"
        onMouseEnter={onFocus}
        onFocus={onFocus}
        onBlur={onBlur}
        aria-label={`${donor.name}: KES ${formatAmount(amount)}, ${state}`}
        className={cn(
          'grid min-h-10 w-full grid-cols-[1fr_auto] items-center gap-x-3 gap-y-1 py-2 text-left text-sm transition-opacity duration-200 outline-none focus-visible:bg-report/15 sm:grid-cols-[minmax(0,12rem)_1fr_auto] sm:gap-4',
          dimmed && 'opacity-30',
        )}
      >
        <span className="col-span-2 truncate font-medium sm:col-span-1">
          {donor.name}
        </span>
        <CoverageBar
          total={largest}
          filled={onlyPending ? amount : donor.received}
          evidenced={onlyPending ? undefined : donor.evidenced}
          pending={onlyPending}
        />
        <span className="text-right font-semibold whitespace-nowrap tabular-nums">
          <span className="text-xs font-normal text-muted-foreground">KES</span>{' '}
          {formatCompact(amount)}
        </span>
      </button>
    </li>
  )
}

function RecentMovements({
  transactions,
  focusedDonor,
}: {
  transactions: Array<TransactionWithNodes>
  focusedDonor: string | null
}) {
  const involves = (t: TransactionWithNodes) =>
    focusedDonor === null ||
    t.from_node?.id === focusedDonor ||
    t.to_node?.id === focusedDonor

  return (
    <>
      {/* Stacked rows on phones */}
      <ul className="divide-y divide-border sm:hidden">
        {transactions.map((t) => (
          <li
            key={t.id}
            className={cn(
              'transition-opacity duration-200',
              !involves(t) && 'opacity-30',
            )}
          >
            <Link
              to="/transactions/$id"
              params={{ id: t.id }}
              className="block py-3"
            >
              <div className="flex items-baseline justify-between gap-3">
                <p className="min-w-0 truncate text-sm font-medium">
                  <NodeName name={t.from_node?.name} /> &rarr;{' '}
                  <NodeName name={t.to_node?.name} />
                </p>
                <p className="shrink-0 text-sm font-semibold tabular-nums">
                  {t.currency} {formatAmount(t.amount)}
                </p>
              </div>
              <div className="mt-1 flex items-center justify-between gap-3 text-xs text-muted-foreground">
                <span className="truncate">{t.description}</span>
                <span className="shrink-0">{formatDate(t.transaction_date)}</span>
              </div>
              <StatusMark
                status={t.verification_status}
                className="mt-1.5"
              />
            </Link>
          </li>
        ))}
      </ul>

      {/* Table from small tablets up */}
      <div className="hidden overflow-x-auto sm:block">
        <table className="w-full text-sm">
          <caption className="sr-only">
            The {transactions.length} most recent money movements
          </caption>
          <thead>
            <tr className="border-b border-border text-left text-xs text-muted-foreground">
              <th scope="col" className="py-2 pr-4 font-medium">
                Movement
              </th>
              <th scope="col" className="w-px py-2 pr-4 font-medium whitespace-nowrap">
                Date
              </th>
              <th scope="col" className="w-px py-2 pr-4 text-right font-medium">
                Amount
              </th>
              <th scope="col" className="w-px py-2 font-medium">
                Proof
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {transactions.map((t) => (
              <tr
                key={t.id}
                className={cn(
                  'transition-opacity duration-200 hover:bg-report/10',
                  !involves(t) && 'opacity-30',
                )}
              >
                <td className="max-w-0 py-2.5 pr-4 sm:w-full">
                  <Link
                    to="/transactions/$id"
                    params={{ id: t.id }}
                    className="block truncate font-medium hover:underline"
                  >
                    <NodeName name={t.from_node?.name} /> &rarr;{' '}
                    <NodeName name={t.to_node?.name} />
                  </Link>
                  <p className="truncate text-xs text-muted-foreground">
                    {t.description}
                  </p>
                </td>
                <td className="py-2.5 pr-4 whitespace-nowrap text-muted-foreground tabular-nums">
                  {formatDate(t.transaction_date)}
                </td>
                <td className="py-2.5 pr-4 text-right font-semibold whitespace-nowrap tabular-nums">
                  {t.currency} {formatAmount(t.amount)}
                </td>
                <td className="py-2.5">
                  <StatusMark status={t.verification_status} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  )
}

function NodeName({ name }: { name: string | undefined }) {
  return name ? <>{name}</> : <span className="text-muted-foreground">Unassigned</span>
}

function DashboardLoading() {
  return (
    <div className="space-y-10" aria-busy="true" aria-live="polite">
      <span className="sr-only">Loading the situation report</span>
      <div className="rule-heavy">
        <div className="grid grid-cols-2 gap-4 border-b border-border py-6 sm:grid-cols-4">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="space-y-3">
              <div className="h-10 w-24 skeleton-shimmer" />
              <div className="h-3 w-28 skeleton-shimmer" />
            </div>
          ))}
        </div>
      </div>
      <div className="grid gap-10 lg:grid-cols-12">
        <div className="rule-heavy space-y-4 pt-14 lg:col-span-5">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="h-5 skeleton-shimmer" />
          ))}
        </div>
        <div className="rule-heavy space-y-4 pt-14 lg:col-span-7">
          {[0, 1, 2, 3, 4].map((i) => (
            <div key={i} className="h-8 skeleton-shimmer" />
          ))}
        </div>
      </div>
    </div>
  )
}
