import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowRight, ArrowsClockwise } from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import type { TrustScoreHistory } from '@/lib/supabase'
import { DEV_MODE } from '@/lib/auth'
import { useOrg } from '@/lib/org-context'
import {
  useOrgStats,
  useRecalculateTrustScore,
  useTransactions,
  useTrustHistory,
} from '@/lib/queries'
import { getTrustGrade } from '@/lib/supabase'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { ledgerSignals } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { Button } from '@/components/ui/button'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
  StatusLine,
} from '@/components/report'

export const Route = createFileRoute('/_authenticated/trust-score')({
  component: TrustScore,
})

const COMPONENTS = [
  {
    key: 'verification',
    label: 'Verification',
    weight: 40,
    explain: 'Share of movements whose proof has been checked and accepted.',
  },
  {
    key: 'documentation',
    label: 'Documentation',
    weight: 30,
    explain: 'Share of movements with at least one proof attached.',
  },
  {
    key: 'consistency',
    label: 'Consistency',
    weight: 30,
    explain: 'How well amounts, dates and parties agree across the records.',
  },
] as const

type ComponentKey = (typeof COMPONENTS)[number]['key']

function TrustScore() {
  const {
    organization,
    organizationId,
    isLoading: orgLoading,
    error: orgError,
    refetch: refetchOrg,
  } = useOrg()
  const statsQuery = useOrgStats()
  const historyQuery = useTrustHistory()
  const txQuery = useTransactions()
  const recalculate = useRecalculateTrustScore()

  const loading =
    orgLoading || statsQuery.isLoading || historyQuery.isLoading
  const error = orgError ?? statsQuery.error ?? historyQuery.error

  const history = historyQuery.data ?? []
  const latest = history.at(-1)
  const stats = statsQuery.data
  const score = organization?.trust_score ?? latest?.score ?? 0
  const grade = getTrustGrade(score)

  const componentScores: Record<ComponentKey, number | null> = {
    verification: latest?.verification_rate ?? stats?.verificationRate ?? null,
    documentation:
      latest?.documentation_rate ??
      stats?.proofStats.proofCoverageRate ??
      null,
    consistency: latest?.consistency_score ?? null,
  }

  const flaggedId = useMemo(() => {
    const flagged = (txQuery.data ?? []).filter(
      (t) => t.verification_status === 'flagged',
    )
    return flagged.length === 1 ? flagged[0].id : null
  }, [txQuery.data])

  const handleRecalculate = async () => {
    try {
      await recalculate.mutateAsync()
      await Promise.all([refetchOrg(), historyQuery.refetch()])
    } catch {
      // Error is surfaced through recalculate.error below.
    }
  }

  const retry = () => {
    void refetchOrg()
    void statsQuery.refetch()
    void historyQuery.refetch()
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <header className="mb-8 flex flex-wrap items-end justify-between gap-x-6 gap-y-4">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">Trust Score</h1>
          <p className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
            {organization?.name && <span>{organization.name}</span>}
            {latest && (
              <span>
                calculated {formatDate(latest.calculated_at)}
              </span>
            )}
            {DEV_MODE && <DemoTag />}
          </p>
        </div>
        {!loading && !error && organizationId && (
          <Button
            variant="outline"
            className="h-10"
            onClick={handleRecalculate}
            disabled={recalculate.isPending}
          >
            <ArrowsClockwise
              className={cn(recalculate.isPending && 'animate-spin')}
            />
            {recalculate.isPending ? 'Recalculating…' : 'Recalculate score'}
          </Button>
        )}
      </header>

      {recalculate.isError && (
        <p role="alert" className="mb-6 text-sm text-destructive">
          The score could not be recalculated
          {recalculate.error.message ? ` (${recalculate.error.message})` : ''}.
          The figures below are from the last successful calculation; try again
          in a moment.
        </p>
      )}

      {error ? (
        <ReportSection title="Score">
          <ReportNotice
            tone="error"
            title="We couldn't load your Trust Score"
            action={
              <Button variant="outline" className="h-10" onClick={retry}>
                Try again
              </Button>
            }
          >
            The request failed{error.message ? ` (${error.message})` : ''}.
            Check your connection and try again.
          </ReportNotice>
        </ReportSection>
      ) : loading ? (
        <TrustLoading />
      ) : !organizationId ? (
        <ReportSection title="Score">
          <ReportNotice title="No organization selected">
            Your Trust Score appears once your account belongs to an
            organization. Ask an admin to add you, then reload this page.
          </ReportNotice>
        </ReportSection>
      ) : (
        <div className="space-y-10">
          <div className="grid gap-10 lg:grid-cols-12">
            <ReportSection title="Score" className="lg:col-span-4">
              <div className="border-b border-border py-5">
                <p
                  className="flex items-baseline gap-2"
                  aria-label={`${score} out of 100, grade ${grade}`}
                >
                  <span className="figure text-8xl leading-[0.8] sm:text-9xl">
                    {score}
                  </span>
                  <span className="text-2xl text-muted-foreground tabular-nums">
                    / 100
                  </span>
                </p>
                <p className="mt-4 text-base">
                  Grade <span className="font-semibold">{grade}</span>
                </p>
              </div>
              <ScoreChange history={history} />
            </ReportSection>

            <ReportSection
              title="How it is made"
              className="lg:col-span-8"
              meta={<span>weighted</span>}
            >
              <ul className="divide-y divide-border">
                {COMPONENTS.map((c) => {
                  const value = componentScores[c.key]
                  return (
                    <li key={c.key} className="py-4">
                      <div className="flex items-baseline justify-between gap-4">
                        <p className="font-semibold">
                          {c.label}{' '}
                          <span className="font-normal whitespace-nowrap text-muted-foreground">
                            {c.weight}% of score
                          </span>
                        </p>
                        <p className="shrink-0 text-sm whitespace-nowrap tabular-nums">
                          {value === null ? (
                            <span className="text-muted-foreground">
                              Not yet calculated
                            </span>
                          ) : (
                            <>
                              <span className="figure text-2xl">
                                {Math.round(value)}
                              </span>
                              <span className="text-muted-foreground">
                                {' '}
                                / 100
                              </span>
                            </>
                          )}
                        </p>
                      </div>
                      <CoverageBar
                        className="mt-2"
                        total={100}
                        filled={value ?? 0}
                        evidenced={value ?? 0}
                        label={
                          value === null
                            ? `${c.label}: not yet calculated`
                            : `${c.label}: ${Math.round(value)} of 100`
                        }
                      />
                      <p className="mt-1.5 text-xs text-muted-foreground">
                        {c.explain}
                        {value !== null &&
                          ` Adds ${Math.round((value * c.weight) / 100)} points.`}
                      </p>
                    </li>
                  )
                })}
              </ul>
            </ReportSection>
          </div>

          <div className="grid gap-10 lg:grid-cols-12">
            <ReportSection
              title="History"
              className="lg:col-span-7"
              meta={
                history.length > 0 ? (
                  <span>last {history.length} calculations</span>
                ) : undefined
              }
            >
              {history.length === 0 ? (
                <ReportNotice title="No history yet">
                  Each time the score is calculated it is added here, so you
                  can see it move. Use Recalculate score to record the first
                  point.
                </ReportNotice>
              ) : (
                <HistoryChart history={history} />
              )}
            </ReportSection>

            <ReportSection
              title="How to raise it"
              className="lg:col-span-5"
            >
              <RaiseActions
                stats={stats}
                flaggedId={flaggedId}
                statsFailed={statsQuery.isError}
              />
            </ReportSection>
          </div>

          <LedgerSignals />
        </div>
      )}
    </div>
  )
}

function ScoreChange({ history }: { history: Array<TrustScoreHistory> }) {
  if (history.length < 2) {
    return (
      <p className="py-3 text-sm text-muted-foreground">
        A change will show after the next calculation.
      </p>
    )
  }
  const first = history[0]
  const last = history[history.length - 1]
  const prev = history[history.length - 2]
  const sinceFirst = last.score - first.score
  const sincePrev = last.score - prev.score
  const signed = (n: number) => (n > 0 ? `+${n}` : String(n))
  return (
    <dl className="divide-y divide-border text-sm">
      <div className="flex items-baseline justify-between gap-4 py-2.5">
        <dt className="text-muted-foreground">
          Since {formatDate(prev.calculated_at)}
        </dt>
        <dd className="font-semibold tabular-nums">{signed(sincePrev)} pts</dd>
      </div>
      <div className="flex items-baseline justify-between gap-4 py-2.5">
        <dt className="text-muted-foreground">
          Since {formatDate(first.calculated_at)}
        </dt>
        <dd className="font-semibold tabular-nums">
          {signed(sinceFirst)} pts
        </dd>
      </div>
      <div className="flex items-baseline justify-between gap-4 py-2.5">
        <dt className="text-muted-foreground">Last calculated</dt>
        <dd>
          <AsOf date={last.calculated_at} className="text-sm text-foreground" />
        </dd>
      </div>
    </dl>
  )
}

const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
]

function monthLabel(iso: string) {
  const d = new Date(iso)
  return `${MONTHS[d.getUTCMonth()]} ${String(d.getUTCFullYear()).slice(2)}`
}

/**
 * Score over time. The SVG draws only the strokes (stretched to the box with
 * non-scaling strokes); labels and the latest-point marker are HTML so they
 * stay legible at any width.
 */
function HistoryChart({ history }: { history: Array<TrustScoreHistory> }) {
  const scores = history.map((h) => h.score)
  const lo = Math.max(0, Math.floor((Math.min(...scores) - 5) / 10) * 10)
  const hi = 100
  const ticks: Array<number> = []
  for (let v = lo; v <= hi; v += lo <= 50 ? 25 : 10) ticks.push(v)
  if (ticks[ticks.length - 1] !== hi) ticks.push(hi)

  const n = history.length
  const x = (i: number) => (n === 1 ? 50 : (i / (n - 1)) * 100)
  const y = (v: number) => ((hi - v) / (hi - lo)) * 100
  const points = history.map((h, i) => `${x(i)},${y(h.score)}`).join(' ')
  const last = history[n - 1]
  const labelEvery = n > 6 ? 2 : 1

  const summary = `Trust Score from ${history[0].score} on ${formatDate(
    history[0].calculated_at,
  )} to ${last.score} on ${formatDate(last.calculated_at)}`

  return (
    <figure className="pt-4">
      <div className="flex gap-2">
        {/* y axis labels */}
        <div className="relative w-7 shrink-0 text-right text-[0.6875rem] text-muted-foreground tabular-nums">
          {ticks.map((t) => (
            <span
              key={t}
              className="absolute right-0 -translate-y-1/2"
              style={{ top: `${y(t)}%` }}
            >
              {t}
            </span>
          ))}
        </div>
        <div className="relative h-52 flex-1 sm:h-60">
          <svg
            role="img"
            aria-label={summary}
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            className="absolute inset-0 size-full overflow-visible"
          >
            {ticks.map((t) => (
              <line
                key={t}
                x1="0"
                x2="100"
                y1={y(t)}
                y2={y(t)}
                vectorEffect="non-scaling-stroke"
                strokeWidth="1"
                className="stroke-border"
              />
            ))}
            <polyline
              points={points}
              fill="none"
              vectorEffect="non-scaling-stroke"
              strokeWidth="2.5"
              strokeLinejoin="miter"
              className="stroke-primary"
            />
          </svg>
          {history.map((h, i) => {
            const isLast = i === n - 1
            return (
              <span
                key={h.id}
                aria-hidden="true"
                className={cn(
                  'absolute -translate-x-1/2 -translate-y-1/2',
                  isLast
                    ? 'size-3 bg-primary ring-2 ring-background'
                    : 'size-1.5 bg-primary',
                )}
                style={{ left: `${x(i)}%`, top: `${y(h.score)}%` }}
              />
            )
          })}
          <span
            aria-hidden="true"
            className="figure absolute -translate-x-full -translate-y-[130%] pr-1 text-xl"
            style={{ left: `${x(n - 1)}%`, top: `${y(last.score)}%` }}
          >
            {last.score}
          </span>
        </div>
      </div>
      {/* x axis labels */}
      <div
        aria-hidden="true"
        className="relative mt-2 ml-9 h-4 text-[0.6875rem] text-muted-foreground"
      >
        {history.map((h, i) =>
          i % labelEvery === (n - 1) % labelEvery ? (
            <span
              key={h.id}
              className={cn(
                'absolute whitespace-nowrap',
                i === 0
                  ? 'translate-x-0'
                  : i === n - 1
                    ? '-translate-x-full'
                    : '-translate-x-1/2',
                i === n - 1 && 'font-semibold text-foreground',
              )}
              style={{ left: `${x(i)}%` }}
            >
              {monthLabel(h.calculated_at)}
            </span>
          ) : null,
        )}
      </div>
      <figcaption className="sr-only">
        <table>
          <caption>Trust Score history</caption>
          <thead>
            <tr>
              <th scope="col">Calculated</th>
              <th scope="col">Score</th>
            </tr>
          </thead>
          <tbody>
            {history.map((h) => (
              <tr key={h.id}>
                <td>{formatDate(h.calculated_at)}</td>
                <td>{h.score}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </figcaption>
    </figure>
  )
}

interface Action {
  key: string
  mark: ReactNode
  title: string
  detail: string
  link?: ReactNode
}

function RaiseActions({
  stats,
  flaggedId,
  statsFailed,
}: {
  stats: ReturnType<typeof useOrgStats>['data']
  flaggedId: string | null
  statsFailed: boolean
}) {
  if (statsFailed || !stats) {
    return (
      <ReportNotice title="Suggestions are unavailable">
        We couldn't count your transactions just now, so we can't suggest next
        steps. Reload the page to try again.
      </ReportNotice>
    )
  }

  const { total, pending, flagged, proofStats } = stats
  const unproven = Math.max(0, total - proofStats.transactionsWithProofs)
  // Each newly evidenced movement lifts the verification rate by 100/total
  // points, which carries 40% weight in the score.
  const pointsFor = (count: number) =>
    total > 0 ? Math.round((count / total) * 100 * 0.4) : 0
  const plural = (n: number, one: string, many: string) =>
    `${n} ${n === 1 ? one : many}`

  const linkClass =
    'inline-flex min-h-10 items-center gap-1 text-sm font-medium text-primary hover:underline'

  const actions: Array<Action> = []
  if (pending > 0) {
    actions.push({
      key: 'pending',
      mark: <StatusLine status="pending" width={22} />,
      title: `${plural(pending, 'movement is', 'movements are')} awaiting proof`,
      detail: `Attach a receipt, photo or statement to each. Evidencing all of them adds up to ${pointsFor(pending)} points.`,
      link: (
        <Link
          to="/transactions"
          search={{ status: 'pending' }}
          className={linkClass}
        >
          Show movements awaiting proof
          <ArrowRight className="size-3.5" />
        </Link>
      ),
    })
  }
  if (flagged > 0) {
    actions.push({
      key: 'flagged',
      mark: <StatusLine status="flagged" width={22} />,
      title: `${plural(flagged, 'movement is', 'movements are')} flagged`,
      detail:
        'Resolve the flag with a matching receipt, or record why the money moved.',
      link: flaggedId ? (
        <Link
          to="/transactions/$id"
          params={{ id: flaggedId }}
          className={linkClass}
        >
          Review the flagged movement
          <ArrowRight className="size-3.5" />
        </Link>
      ) : (
        <Link
          to="/transactions"
          search={{ status: 'flagged' }}
          className={linkClass}
        >
          Review flagged movements
          <ArrowRight className="size-3.5" />
        </Link>
      ),
    })
  }
  if (unproven > 0) {
    actions.push({
      key: 'unproven',
      mark: <StatusLine status="pending" width={22} />,
      title: `${plural(unproven, 'movement has', 'movements have')} no proof attached`,
      detail: `Documentation is ${proofStats.proofCoverageRate}% covered and counts for 30% of the score.`,
    })
  }
  if (proofStats.pendingProofs > 0) {
    actions.push({
      key: 'proofs',
      mark: <StatusLine status="pending" width={22} />,
      title: `${plural(proofStats.pendingProofs, 'proof is', 'proofs are')} waiting for review`,
      detail: 'A reviewer can accept them from each transaction’s page.',
    })
  }

  if (actions.length === 0) {
    return (
      <ReportNotice title="Nothing outstanding">
        Every movement is evidenced and nothing is flagged. Keep attaching
        proof as you record new transactions to hold the score.
      </ReportNotice>
    )
  }

  return (
    <ol className="divide-y divide-border">
      {actions.map((a) => (
        <li key={a.key} className="flex gap-3 py-4">
          <span className="pt-1.5">{a.mark}</span>
          <div className="min-w-0">
            <p className="font-semibold">{a.title}</p>
            <p className="mt-1 text-sm text-muted-foreground">{a.detail}</p>
            {a.link && <div className="mt-1">{a.link}</div>}
          </div>
        </li>
      ))}
    </ol>
  )
}

/**
 * What the books say about the same organisation: how much cash movement
 * is matched to a statement, how much spend sits within budget, and how
 * much is coded to a grant. Shown beside the score, not part of it. Hidden
 * when the books are not connected or still loading.
 */
function LedgerSignals() {
  const { state } = useLedger()
  const asOf = useLedgerAsOf()
  const signals = useMemo(
    () => (state ? ledgerSignals(state, asOf) : null),
    [state, asOf],
  )
  if (!signals) return null
  const pct = (n: number) => `${Math.round(n * 100)}%`
  const linkClass =
    'inline-flex min-h-10 items-center gap-1 text-sm font-medium text-primary hover:underline'
  return (
    <ReportSection
      title="Ledger signals"
      meta={
        <>
          <AsOf date={asOf} />
          {DEV_MODE && <DemoTag />}
        </>
      }
    >
      <p className="border-b border-border py-3 text-sm text-muted-foreground">
        Read from the books, alongside the score. They do not change it.
      </p>
      <KeyFigures className="sm:grid-cols-3">
        <KeyFigure
          value={pct(signals.reconciledShare)}
          label="Cash movement reconciled"
          detail={
            <>
              <span className="block text-muted-foreground">
                Share of bank and mobile-money movement, by value, matched to a
                statement line.
              </span>
              <Link to="/reconcile" className={linkClass}>
                Reconcile accounts
                <ArrowRight className="size-3.5" />
              </Link>
            </>
          }
        />
        <KeyFigure
          value={pct(signals.inBudgetShare)}
          label="Spend within budget"
          detail={
            <>
              <span className="block text-muted-foreground">
                Share of budgeted spend on budget lines that are not over.
              </span>
              <Link to="/grants/budgets" className={linkClass}>
                Budget against actual
                <ArrowRight className="size-3.5" />
              </Link>
            </>
          }
        />
        <KeyFigure
          value={pct(signals.grantCodedShare)}
          label="Spend coded to a grant"
          detail={
            <>
              <span className="block text-muted-foreground">
                Share of spend charged to a named grant, so donors can trace it.
              </span>
              <Link to="/grants" className={linkClass}>
                See grants
                <ArrowRight className="size-3.5" />
              </Link>
            </>
          }
        />
      </KeyFigures>
    </ReportSection>
  )
}

function TrustLoading() {
  return (
    <div className="space-y-10" aria-busy="true" aria-live="polite">
      <span className="sr-only">Loading your Trust Score</span>
      <div className="grid gap-10 lg:grid-cols-12">
        <div className="rule-heavy space-y-4 pt-14 lg:col-span-4">
          <div className="h-24 w-40 skeleton-shimmer" />
          <div className="h-4 w-48 skeleton-shimmer" />
        </div>
        <div className="rule-heavy space-y-6 pt-14 lg:col-span-8">
          {[0, 1, 2].map((i) => (
            <div key={i} className="space-y-2">
              <div className="h-4 w-40 skeleton-shimmer" />
              <div className="h-5 skeleton-shimmer" />
            </div>
          ))}
        </div>
      </div>
      <div className="rule-heavy h-64 pt-14">
        <div className="size-full skeleton-shimmer" />
      </div>
    </div>
  )
}
