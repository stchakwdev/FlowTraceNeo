import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowRight } from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import type { TrustComponent, TrustScoreHistory } from '@/lib/types'
import { useOrg } from '@/lib/org-context'
import {
  useOrgStats,
  useTransactions,
  useTrustHistory,
  useTrustSummary,
} from '@/lib/queries'
import { getTrustGrade } from '@/lib/types'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { ledgerSignals } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf } from '@/lib/ledger-client'
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

/**
 * What each part of the score means. The weights come with the score: the module's four
 * (verification 32, documentation 24, consistency 24, model 20; P16), or the demo's three
 * (40/30/30, types.ts calculateTrustScore).
 */
const COMPONENT_TEXT: Record<
  TrustComponent['key'],
  { label: string; explain: string }
> = {
  verification: {
    label: 'Verification',
    explain: 'Share of movements whose proof has been checked and accepted.',
  },
  documentation: {
    label: 'Documentation',
    explain: 'Share of movements with at least one proof attached.',
  },
  consistency: {
    label: 'Consistency',
    explain: 'How well amounts, dates and parties agree across the records.',
  },
  model: {
    label: 'Independent review',
    explain:
      'A review of the records against a fixed rubric, less 5 points for each confirmed flag. Until the first review it matches the other three, so it does not move the score.',
  },
}

const weightOf = (
  components: Array<TrustComponent>,
  key: TrustComponent['key'],
) => components.find((c) => c.key === key)?.weight ?? 0

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
  const summaryQuery = useTrustSummary()
  const txQuery = useTransactions()

  const loading =
    orgLoading ||
    statsQuery.isLoading ||
    historyQuery.isLoading ||
    summaryQuery.isLoading
  const error =
    orgError ?? statsQuery.error ?? historyQuery.error ?? summaryQuery.error

  const history = historyQuery.data ?? []
  const latest = history.at(-1)
  const stats = statsQuery.data
  const summary = summaryQuery.data ?? null
  const score = summary?.score ?? organization?.trust_score ?? latest?.score ?? 0
  const grade = summary?.grade ?? getTrustGrade(score)
  const components = summary?.components ?? []
  const calculatedAt = summary?.computed_at ?? latest?.calculated_at ?? null

  const flaggedId = useMemo(() => {
    const flagged = (txQuery.data ?? []).filter(
      (t) => t.verification_status === 'flagged',
    )
    return flagged.length === 1 ? flagged[0].id : null
  }, [txQuery.data])

  const retry = () => {
    void refetchOrg()
    void statsQuery.refetch()
    void historyQuery.refetch()
    void summaryQuery.refetch()
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <header className="mb-8 flex flex-wrap items-end justify-between gap-x-6 gap-y-4">
        <div className="min-w-0">
          <h1 className="text-4xl sm:text-5xl">Trust Score</h1>
          <p className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
            {organization?.name && <span>{organization.name}</span>}
            {calculatedAt && (
              <span>calculated {formatDate(calculatedAt)}</span>
            )}
            {DEMO_BOOKS && <DemoTag />}
          </p>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            The score updates by itself each time a movement or a proof is
            recorded or changed, so there is nothing to recalculate.
          </p>
        </div>
      </header>

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
              {components.length === 0 ? (
                <ReportNotice title="Not yet calculated">
                  The score is worked out when the first movement is recorded.
                </ReportNotice>
              ) : (
              <ul className="divide-y divide-border">
                {components.map((part) => {
                  const c = { ...COMPONENT_TEXT[part.key], weight: part.weight }
                  const value = part.value
                  return (
                    <li key={part.key} className="py-4">
                      <div className="flex items-baseline justify-between gap-4">
                        <p className="font-semibold">
                          {c.label}{' '}
                          <span className="font-normal whitespace-nowrap text-muted-foreground">
                            {c.weight}% of score
                          </span>
                        </p>
                        <p className="shrink-0 text-sm whitespace-nowrap tabular-nums">
                          <span className="figure text-2xl">
                            {Math.round(value)}
                          </span>
                          <span className="text-muted-foreground">
                            {' '}
                            / 100
                          </span>
                        </p>
                      </div>
                      <CoverageBar
                        className="mt-2"
                        total={100}
                        filled={value}
                        evidenced={value}
                        label={`${c.label}: ${Math.round(value)} of 100`}
                      />
                      <p className="mt-1.5 text-xs text-muted-foreground">
                        {c.explain}
                        {` Adds ${Math.round((value * c.weight) / 100)} points.`}
                      </p>
                    </li>
                  )
                })}
              </ul>
              )}
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
                  Each scoring run adds a point here, so you can see the score
                  move. None has been recorded yet; the score above is already
                  current.
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
                verificationWeight={weightOf(components, 'verification')}
                documentationWeight={weightOf(components, 'documentation')}
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
  verificationWeight,
  documentationWeight,
  flaggedId,
  statsFailed,
}: {
  stats: ReturnType<typeof useOrgStats>['data']
  verificationWeight: number
  documentationWeight: number
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
  // points, which carries the verification weight in the score.
  const pointsFor = (count: number) =>
    total > 0 ? Math.round((count / total) * verificationWeight) : 0
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
      detail: `Documentation is ${proofStats.proofCoverageRate}% covered and counts for ${documentationWeight}% of the score.`,
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
          {DEMO_BOOKS && <DemoTag />}
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
