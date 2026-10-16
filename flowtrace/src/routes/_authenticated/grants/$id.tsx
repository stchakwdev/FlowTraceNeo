import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowLeft, DownloadSimple, Warning } from '@phosphor-icons/react'
import type {
  BudgetVsActualRow,
  Grant,
  GrantSummary,
  LedgerState,
  ReportLine,
} from '@/lib/ledger'
import { budgetVsActual, grantSummary, postedLines } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { formatDate, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import {
  BooksPage,
  BooksState,
  Money,
  csvAmount,
  downloadCsv,
  formatMinor,
  formatMinorCompact,
} from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/grants/$id')({
  component: GrantPage,
})

const BVA_STATUS: Record<BudgetVsActualRow['status'], string> = {
  within: 'Within budget',
  warning: '80% or more used',
  over: 'Over budget',
}

function GrantPage() {
  const { id } = Route.useParams()
  const { state, isLoading, error, refetch } = useLedger()
  const grant = state?.grants.find((g) => g.id === id)
  const fund = grant ? state?.funds.find((f) => f.id === grant.fund_id) : undefined

  return (
    <BooksPage
      title={grant?.name ?? 'Grant'}
      lede={
        grant ? (
          <>
            {grant.donor_name} · {fund?.name ?? 'Fund'} · {formatDate(grant.start_date)} to{' '}
            {formatDate(grant.end_date)}
          </>
        ) : undefined
      }
      nav={
        <Link
          to="/grants"
          className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
          All grants
        </Link>
      }
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="grant" />
      ) : !grant ? (
        <ReportNotice
          title="This grant is not in the books"
          action={
            <Link to="/grants" className={buttonVariants({ variant: 'outline' })}>
              Back to grants
            </Link>
          }
        >
          The link may be wrong, or the grant belongs to another organisation.
        </ReportNotice>
      ) : (
        <GrantDetail state={state} grant={grant} />
      )}
    </BooksPage>
  )
}

const months = (n: number) => `${n < 10 ? n.toFixed(1).replace(/\.0$/, '') : Math.round(n)} ${n === 1 ? 'month' : 'months'}`

/** Will the money last to the end date, at the rate it is being spent? */
function runway(grant: Grant, s: GrantSummary) {
  if (s.monthsToEnd <= 0) return { flag: false, text: `The grant ended on ${formatDate(grant.end_date)}.` }
  if (s.monthsLeft === null)
    return { flag: false, text: 'No spend is posted yet, so there is no rate to project from.' }
  const gap = s.monthsLeft - s.monthsToEnd
  if (gap < -1)
    return {
      flag: true,
      text: `At this rate the award runs out in about ${months(s.monthsLeft)}, ${months(-gap)} before the grant ends on ${formatDate(grant.end_date)}.`,
    }
  if (s.monthsLeft > s.monthsToEnd * 1.5 && gap > 3) {
    const unspent = Math.max(0, s.award - s.spent - Math.round(s.burnPerMonth * s.monthsToEnd))
    return {
      flag: true,
      text: `At this rate about ${s.award > 0 ? Math.round((unspent / s.award) * 100) : 0}% of the award (${formatMinorCompact(unspent)}) is still unspent when the grant ends on ${formatDate(grant.end_date)}.`,
    }
  }
  return { flag: false, text: `At this rate the award lasts to about the end date, ${formatDate(grant.end_date)}.` }
}

function GrantDetail({ state, grant }: { state: LedgerState; grant: Grant }) {
  const asOf = useLedgerAsOf()
  const s = useMemo(() => grantSummary(state, grant.id, asOf), [state, grant.id, asOf])
  const bva = useMemo(() => budgetVsActual(state, { grantId: grant.id, to: asOf }), [state, grant.id, asOf])
  const lines = useMemo(
    () =>
      postedLines(state, { grantId: grant.id, to: asOf, excludeClosing: true }).sort(
        (a, b) => b.entry.date.localeCompare(a.entry.date) || b.entry.number.localeCompare(a.entry.number),
      ),
    [state, grant.id, asOf],
  )
  const accounts = useMemo(() => new Map(state.accounts.map((a) => [a.id, a])), [state.accounts])
  const base = state.settings.base_currency
  const run = runway(grant, s)
  const pct = (n: number) => `${Math.round((n / Math.max(1, s.award)) * 100)}%`

  const exportCsv = () =>
    downloadCsv(`grant-${grant.name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-lines-${asOf}`, [
      ['Date', 'Entry', 'Account code', 'Account', 'Memo', `Debit (${base})`, `Credit (${base})`, 'Currency', 'Amount in currency'],
      ...lines.map((l) => {
        const a = accounts.get(l.account_id)
        return [
          l.entry.date,
          l.entry.number,
          a?.code ?? '',
          a?.name ?? '',
          l.memo ?? l.entry.memo,
          l.base_amount > 0 ? csvAmount(l.base_amount) : '',
          l.base_amount < 0 ? csvAmount(-l.base_amount) : '',
          l.currency,
          csvAmount(l.debit - l.credit),
        ]
      }),
    ])

  return (
    <>
      <ReportSection
        title="Award"
        meta={
          <>
            <AsOf date={asOf} />
            {DEMO_BOOKS && <DemoTag />}
          </>
        }
      >
        <KeyFigures>
          <KeyFigure scale="lg" value={formatMinorCompact(s.award)} label={`Awarded, ${grant.currency}`} />
          <KeyFigure value={formatMinorCompact(s.spent)} label="Spent" detail={<span className="text-muted-foreground">{pct(s.spent)} of the award</span>} />
          <KeyFigure value={formatMinorCompact(s.committed)} label="Committed" detail={<span className="text-muted-foreground">Approved bills and submitted claims</span>} />
          <KeyFigure value={formatMinorCompact(s.remaining)} label="Remaining" />
        </KeyFigures>
        <KeyFigures>
          <KeyFigure scale="sm" value={formatMinorCompact(s.received)} label="Received from the donor" />
          <KeyFigure scale="sm" value={formatMinorCompact(s.burnPerMonth)} label="Spent per month, on average" />
          <KeyFigure scale="sm" value={s.monthsLeft === null ? '–' : months(s.monthsLeft)} label="Money lasts, at that rate" />
          <KeyFigure scale="sm" value={months(Math.round(s.monthsToEnd * 10) / 10)} label="Until the grant ends" />
        </KeyFigures>
        <div className="space-y-3 py-4">
          <CoverageBar
            total={s.award}
            filled={s.spent + s.committed}
            evidenced={s.spent}
            label={`${pct(s.spent)} spent, ${pct(s.committed)} committed`}
          />
          <p className="flex flex-wrap gap-x-5 gap-y-1 text-xs text-muted-foreground">
            <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-primary" /> Spent</span>
            <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-report" /> Committed</span>
            <span className="inline-flex items-center gap-2"><span className="inline-block h-3 w-5 bg-gap" /> Remaining</span>
          </p>
          <p className={cn('flex items-start gap-2 text-sm', run.flag ? 'font-semibold text-flag' : 'text-muted-foreground')}>
            {run.flag && <Warning weight="bold" aria-hidden="true" className="mt-0.5 size-4 shrink-0" />}
            {run.flag && <span className="sr-only">Flag: </span>}
            {run.text}
          </p>
        </div>
      </ReportSection>

      <ReportSection title="Terms">
        <p className="max-w-prose py-4 text-sm">{grant.terms || 'No terms recorded.'}</p>
      </ReportSection>

      <ReportSection
        title="Budget vs actual"
        meta={
          <Link to="/grants/budgets" className="font-semibold text-foreground underline underline-offset-4">
            Whole budget
          </Link>
        }
      >
        {bva.length === 0 ? (
          <ReportNotice title="No budget lines for this grant">
            Add budget lines for this grant on the budget page to compare spend against them.
          </ReportNotice>
        ) : (
          <ul className="divide-y divide-border">
            {bva.map((r) => (
              <li key={r.lineId} className="grid gap-3 py-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
                <div className="min-w-0">
                  <p className="font-semibold">{r.account ? `${r.account.code} · ${r.account.name}` : r.label}</p>
                  <BvaStatus row={r} />
                </div>
                <div className="min-w-0">
                  <CoverageBar
                    total={r.budget}
                    filled={Math.min(r.actual, r.budget)}
                    evidenced={Math.min(r.actual, r.budget)}
                    label={`${Number.isFinite(r.used) ? Math.round(r.used * 100) : 100}% of budget spent`}
                  />
                  <dl className="mt-2 grid grid-cols-3 gap-2 text-xs">
                    <div>
                      <dt className="text-muted-foreground">Budget</dt>
                      <dd><Money value={r.budget} decimals={0} /></dd>
                    </div>
                    <div>
                      <dt className="text-muted-foreground">Actual</dt>
                      <dd><Money value={r.actual} decimals={0} strong /></dd>
                    </div>
                    <div className="text-right">
                      <dt className="text-muted-foreground">{r.variance < 0 ? 'Over by' : 'Left'}</dt>
                      <dd><Money value={Math.abs(r.variance)} decimals={0} strong zero="0" /></dd>
                    </div>
                  </dl>
                </div>
              </li>
            ))}
          </ul>
        )}
      </ReportSection>

      <ReportSection
        title="Charged to this grant"
        meta={<span>{lines.length} lines</span>}
        actions={
          lines.length > 0 && (
            <Button variant="outline" size="sm" onClick={exportCsv}>
              <DownloadSimple />
              CSV
            </Button>
          )
        }
      >
        {lines.length === 0 ? (
          <ReportNotice title="Nothing posted to this grant yet">
            Income received and costs charged to the grant appear here once posted.
          </ReportNotice>
        ) : (
          <>
            <LinesTable lines={lines} accounts={accounts} base={base} />
            <LinesStack lines={lines} accounts={accounts} />
          </>
        )}
      </ReportSection>
    </>
  )
}

function BvaStatus({ row }: { row: Pick<BudgetVsActualRow, 'status' | 'used'> }) {
  const used = Number.isFinite(row.used) ? `${Math.round(row.used * 100)}% used` : 'Spent with no budget'
  return (
    <p
      className={cn(
        'mt-1 inline-flex items-center gap-1.5 text-xs font-semibold',
        row.status === 'over' ? 'text-destructive' : row.status === 'warning' ? 'text-flag' : 'text-muted-foreground',
      )}
    >
      {row.status !== 'within' && <Warning weight="bold" aria-hidden="true" className="size-3.5" />}
      {BVA_STATUS[row.status]} · {used}
    </p>
  )
}

type AccountMap = Map<string, { code: string; name: string; type: string }>

function LinesTable({ lines, accounts, base }: { lines: Array<ReportLine>; accounts: AccountMap; base: string }) {
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[52rem] text-sm">
        <caption className="sr-only">Posted journal lines charged to this grant, in {base}</caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">Date</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Entry</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Account</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Memo</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Debit</th>
            <th scope="col" className="py-2.5 text-right font-semibold">Credit</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((l) => {
            const a = accounts.get(l.account_id)
            return (
              <tr key={l.id} className="border-b border-border align-top">
                <td className="py-2.5 pr-4 whitespace-nowrap text-muted-foreground">{formatDate(l.entry.date)}</td>
                <td className="py-2.5 pr-4">
                  <Link to="/ledger/entries/$id" params={{ id: l.entry.id }} className="font-mono text-xs underline underline-offset-4 hover:text-primary">
                    {l.entry.number}
                  </Link>
                </td>
                <td className="py-2.5 pr-4">
                  <span className="font-mono text-xs text-muted-foreground">{a?.code}</span> {a?.name ?? 'Account'}
                </td>
                <td className="max-w-[20rem] py-2.5 pr-4 text-muted-foreground">{l.memo ?? l.entry.memo}</td>
                <td className="py-2.5 pr-4 text-right"><Money value={Math.max(0, l.base_amount)} /></td>
                <td className="py-2.5 text-right"><Money value={Math.max(0, -l.base_amount)} /></td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

function LinesStack({ lines, accounts }: { lines: Array<ReportLine>; accounts: AccountMap }) {
  return (
    <ul className="divide-y divide-border md:hidden">
      {lines.map((l) => {
        const a = accounts.get(l.account_id)
        return (
          <li key={l.id} className="py-3">
            <div className="flex items-start justify-between gap-3">
              <p className="min-w-0 text-sm font-semibold">{l.memo ?? l.entry.memo}</p>
              <span className="shrink-0 text-sm font-semibold tabular-nums">
                {l.base_amount >= 0 ? 'Dr ' : 'Cr '}
                {formatMinor(Math.abs(l.base_amount))}
              </span>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              {formatShortDate(l.entry.date)} · {a ? `${a.code} ${a.name}` : 'Account'} ·{' '}
              <Link to="/ledger/entries/$id" params={{ id: l.entry.id }} className="font-mono underline underline-offset-4">
                {l.entry.number}
              </Link>
            </p>
          </li>
        )
      })}
    </ul>
  )
}
