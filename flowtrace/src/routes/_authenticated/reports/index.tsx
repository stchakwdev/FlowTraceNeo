import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useEffect, useMemo, useState } from 'react'
import { DownloadSimple, Printer } from '@phosphor-icons/react'
import type { ReportId, ReportParams } from '@/components/ledger/reports/statements'
import type { Basis, ISODate, LedgerState } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { useOrg } from '@/lib/org-context'
import { DEV_MODE } from '@/lib/auth'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { AsOf, DemoTag, ReportSection } from '@/components/report'
import { BooksPage, BooksState, downloadCsv } from '@/components/ledger'
import {
  REPORTS,
  ReportBody,
  reportCsv,
  reportMeta,
} from '@/components/ledger/reports/statements'

type ReportsSearch = {
  report?: string
  from?: string
  to?: string
  basis?: string
}

const str = (v: unknown) => (typeof v === 'string' && v ? v : undefined)
const isIso = (v?: string): v is ISODate => !!v && /^\d{4}-\d{2}-\d{2}$/.test(v)

export const Route = createFileRoute('/_authenticated/reports/')({
  validateSearch: (search: Record<string, unknown>): ReportsSearch => ({
    report: str(search.report),
    from: str(search.from),
    to: str(search.to),
    basis: str(search.basis),
  }),
  component: ReportsPage,
})

const toReportId = (v?: string): ReportId =>
  REPORTS.some((r) => r.id === v) ? (v as ReportId) : 'position'

/** Period presets drawn from the ledger's own fiscal periods. */
function presets(state: LedgerState, asOf: ISODate) {
  const periods = [...state.periods].sort((a, b) => a.start_date.localeCompare(b.start_date))
  const current =
    periods.find((p) => p.start_date <= asOf && asOf <= p.end_date) ?? periods.at(-1)
  if (!current) return []
  const year = periods.filter((p) => p.fiscal_year === current.fiscal_year)
  const last = periods.filter((p) => p.fiscal_year === current.fiscal_year - 1)
  const out: Array<{ key: string; label: string; from: ISODate; to: ISODate }> = [
    { key: 'month', label: `This month (${current.name})`, from: current.start_date, to: asOf < current.end_date ? asOf : current.end_date },
    { key: 'ytd', label: 'Year to date', from: year[0].start_date, to: asOf },
  ]
  if (last.length)
    out.push({ key: 'last', label: `Fiscal year ${current.fiscal_year - 1}`, from: last[0].start_date, to: last[last.length - 1].end_date })
  return out
}

function ReportsPage() {
  const search = Route.useSearch()
  const navigate = useNavigate({ from: Route.fullPath })
  const { state, isLoading, error, refetch } = useLedger()
  const { organization } = useOrg()
  const asOf = useLedgerAsOf()

  const id = toReportId(search.report)
  const meta = reportMeta(id)
  const ranges = useMemo(() => (state ? presets(state, asOf) : []), [state, asOf])
  const ytd = ranges.find((r) => r.key === 'ytd')
  const params: ReportParams = {
    from: isIso(search.from) ? search.from : (ytd?.from ?? asOf),
    to: isIso(search.to) ? search.to : asOf,
    basis: search.basis === 'cash' ? 'cash' : 'accrual',
  }
  const badRange = params.from > params.to

  const set = (patch: Partial<ReportsSearch>) =>
    navigate({ search: (prev) => ({ ...prev, ...patch }), replace: true })

  const exportCsv = () => {
    if (!state) return
    const range = meta.span === 'point' ? params.to : `${params.from}_to_${params.to}`
    downloadCsv(`${id}_${range}${meta.hasBasis ? `_${params.basis}` : ''}`, reportCsv(id, state, params))
  }

  return (
    <BooksPage
      title="Financial reports"
      lede="The statements a board, auditor or donor expects, read from posted journal lines. Each one shows the check that proves it adds up."
      actions={
        state && (
          <div className="flex flex-wrap gap-2 print:hidden">
            <Button variant="outline" onClick={exportCsv} disabled={badRange}>
              <DownloadSimple aria-hidden="true" />
              Export CSV
            </Button>
            <Button variant="outline" onClick={() => window.print()} disabled={badRange}>
              <Printer aria-hidden="true" />
              Print or save as PDF
            </Button>
          </div>
        )
      }
      nav={
        <nav aria-label="Reports" className="overflow-x-auto print:hidden">
          <ul className="inline-flex gap-px border border-border bg-border">
            {REPORTS.map((r) => {
              const active = r.id === id
              return (
                <li key={r.id} className="bg-background">
                  <Link
                    to="/reports"
                    search={(prev) => ({ ...prev, report: r.id })}
                    replace
                    aria-current={active ? 'page' : undefined}
                    className={cn(
                      'flex h-10 items-center px-3 text-sm whitespace-nowrap transition-colors',
                      active
                        ? 'bg-primary font-semibold text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                    )}
                  >
                    {r.short}
                  </Link>
                </li>
              )
            })}
          </ul>
        </nav>
      }
    >
      {/* Leave only the statement on paper: the app's masthead and rail go. */}
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="reports" />
      ) : (
        <>
          <PeriodControls
            span={meta.span}
            hasBasis={meta.hasBasis}
            params={params}
            ranges={ranges}
            onChange={set}
          />
          <ReportSection
            title={meta.title}
            className="print:mt-0"
            meta={
              <>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            {badRange ? (
              <p role="alert" className="py-6 text-sm text-destructive">
                The period starts after it ends. Choose a start date on or before the end date.
              </p>
            ) : (
              <ReportBody id={id} state={state} params={params} org={organization?.name} />
            )}
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}

function PeriodControls({
  span,
  hasBasis,
  params,
  ranges,
  onChange,
}: {
  span: 'point' | 'range'
  hasBasis: boolean
  params: ReportParams
  ranges: Array<{ key: string; label: string; from: ISODate; to: ISODate }>
  onChange: (patch: Partial<ReportsSearch>) => void
}) {
  const [fromText, setFromText] = useState(isoToDayMonthYear(params.from))
  const [toText, setToText] = useState(isoToDayMonthYear(params.to))

  // Presets and the back button change the URL; follow them unless the
  // field already says the same date.
  useEffect(() => {
    setFromText((t) => (parseDayMonthYear(t)?.iso === params.from ? t : isoToDayMonthYear(params.from)))
    setToText((t) => (parseDayMonthYear(t)?.iso === params.to ? t : isoToDayMonthYear(params.to)))
  }, [params.from, params.to])

  const fromParse = parseDayMonthYear(fromText)
  const toParse = parseDayMonthYear(toText)

  const commit = (which: 'from' | 'to', text: string) => {
    const p = parseDayMonthYear(text)
    if (p?.iso && p.iso !== params[which]) onChange({ [which]: p.iso })
  }
  const activePreset = ranges.find((r) =>
    span === 'point' ? r.to === params.to : r.from === params.from && r.to === params.to,
  )?.key

  return (
    <section aria-label="Report period and basis" className="rule-heavy print:hidden">
      <div className="grid gap-4 py-4 md:grid-cols-[auto_1fr] md:items-end">
        <fieldset className="min-w-0">
          <legend className="mb-1.5 text-sm font-medium">Period</legend>
          <ul className="flex flex-wrap gap-px border border-border bg-border">
            {ranges.map((r) => (
              <li key={r.key} className="grow bg-background sm:grow-0">
                <button
                  type="button"
                  aria-pressed={activePreset === r.key}
                  onClick={() => onChange({ from: r.from, to: r.to })}
                  className={cn(
                    'flex h-10 w-full items-center justify-center px-3 text-sm whitespace-nowrap transition-colors',
                    activePreset === r.key
                      ? 'bg-primary font-semibold text-primary-foreground'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                  )}
                >
                  {r.label}
                </button>
              </li>
            ))}
          </ul>
        </fieldset>
        <div className="flex flex-wrap items-end gap-3 md:justify-end">
          {span === 'range' && (
            <div className="w-full min-[400px]:w-40">
              <label htmlFor="report-from" className="mb-1.5 block text-sm font-medium">
                From
              </label>
              <DateField
                id="report-from"
                value={fromText}
                onValueChange={(t) => {
                  setFromText(t)
                  commit('from', t)
                }}
                aria-invalid={fromParse?.error ? true : undefined}
                aria-describedby={fromParse?.error ? 'report-from-error' : undefined}
                pickerLabel="Choose the start date from a calendar"
                className="h-10"
              />
            </div>
          )}
          <div className="w-full min-[400px]:w-40">
            <label htmlFor="report-to" className="mb-1.5 block text-sm font-medium">
              {span === 'point' ? 'As at' : 'To'}
            </label>
            <DateField
              id="report-to"
              value={toText}
              onValueChange={(t) => {
                setToText(t)
                commit('to', t)
              }}
              aria-invalid={toParse?.error ? true : undefined}
              aria-describedby={toParse?.error ? 'report-to-error' : undefined}
              pickerLabel="Choose the end date from a calendar"
              className="h-10"
            />
          </div>
          {hasBasis && (
            <fieldset>
              <legend className="mb-1.5 text-sm font-medium">Basis</legend>
              <div className="flex gap-px border border-border bg-border">
                {(['accrual', 'cash'] as Array<Basis>).map((b) => (
                  <button
                    key={b}
                    type="button"
                    aria-pressed={params.basis === b}
                    onClick={() => onChange({ basis: b === 'accrual' ? undefined : b })}
                    className={cn(
                      'h-10 px-3 text-sm transition-colors',
                      params.basis === b
                        ? 'bg-primary font-semibold text-primary-foreground'
                        : 'bg-background text-muted-foreground hover:bg-muted hover:text-foreground',
                    )}
                  >
                    {b === 'accrual' ? 'Accrual' : 'Cash'}
                  </button>
                ))}
              </div>
            </fieldset>
          )}
        </div>
      </div>
      {(fromParse?.error || toParse?.error) && (
        <div className="pb-3 text-sm font-semibold text-destructive" role="alert">
          {fromParse?.error && span === 'range' && <p id="report-from-error">Start date: {fromParse.error}</p>}
          {toParse?.error && <p id="report-to-error">End date: {toParse.error}</p>}
        </div>
      )}
      {hasBasis && params.basis === 'cash' && (
        <p className="border-t border-border py-2 text-xs text-muted-foreground">
          Cash basis leaves out unpaid bills and claims, and counts each payment against the expenses it settled.
        </p>
      )}
    </section>
  )
}
