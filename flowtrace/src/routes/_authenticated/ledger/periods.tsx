import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { PeriodStatusMark } from './-shared'
import type { FiscalPeriod, LedgerState } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { formatDate, formatShortDate } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Field } from '@/components/form'
import { ReportDialog } from '@/components/report-dialog'
import { AsOf, DemoTag, ReportNotice, ReportSection } from '@/components/report'
import { BooksPage, BooksState, IssueSummary, SubNav } from '@/components/ledger'
import { LEDGER_NAV } from '@/components/ledger/ledger-nav'

export const Route = createFileRoute('/_authenticated/ledger/periods')({
  component: PeriodsPage,
})

type Year = {
  year: number
  periods: Array<FiscalPeriod>
  closed: boolean
  canClose: boolean
  drafts: number
}

function buildYears(state: LedgerState): Array<Year> {
  const byYear = new Map<number, Array<FiscalPeriod>>()
  for (const p of state.periods) {
    const list = byYear.get(p.fiscal_year) ?? []
    list.push(p)
    byYear.set(p.fiscal_year, list)
  }
  return [...byYear.entries()]
    .sort((a, b) => b[0] - a[0])
    .map(([year, periods]) => {
      periods.sort((a, b) => a.start_date.localeCompare(b.start_date))
      const last = periods[periods.length - 1]
      const ids = new Set(periods.map((p) => p.id))
      const closed = state.entries.some((e) => e.source_type === 'year_end_close' && e.date === last.end_date)
      return {
        year,
        periods,
        closed,
        canClose: !closed && last.status === 'open',
        drafts: state.entries.filter((e) => e.status === 'draft' && ids.has(e.period_id)).length,
      }
    })
}

type Pending =
  | { kind: 'reopen'; period: FiscalPeriod }
  | { kind: 'lock'; period: FiscalPeriod }
  | { kind: 'year'; year: Year }
  | null

function PeriodsPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const [failure, setFailure] = useState<unknown>(null)
  const [dialog, setDialog] = useState<Pending>(null)
  const [reason, setReason] = useState('')
  const [reasonError, setReasonError] = useState<string>()
  const [dialogFailure, setDialogFailure] = useState<unknown>(null)
  const [done, setDone] = useState<string | null>(null)

  const years = useMemo(() => (state ? buildYears(state) : []), [state])
  const counts = useMemo(() => {
    const m = new Map<string, { posted: number; drafts: number }>()
    for (const e of state?.entries ?? []) {
      const c = m.get(e.period_id) ?? { posted: 0, drafts: 0 }
      if (e.status === 'draft') c.drafts++
      else c.posted++
      m.set(e.period_id, c)
    }
    return m
  }, [state])

  const closePeriod = async (p: FiscalPeriod) => {
    setFailure(null)
    setDone(null)
    try {
      await command.mutateAsync({ type: 'period.close', periodId: p.id })
      setDone(`${p.name} is closed. Entries can no longer be dated in it unless it is reopened.`)
    } catch (e) {
      setFailure(e)
    }
  }

  const nextYear = years.length ? Math.max(...years.map((y) => y.year)) + 1 : null
  const openYear = async () => {
    if (nextYear === null) return
    setFailure(null)
    setDone(null)
    try {
      await command.mutateAsync({ type: 'year.open', year: nextYear })
      setDone(`Fiscal year ${nextYear} is open: twelve monthly periods were added.`)
    } catch (e) {
      setFailure(e)
    }
  }

  const openDialog = (d: Pending) => {
    setReason('')
    setReasonError(undefined)
    setDialogFailure(null)
    setDialog(d)
  }

  const confirm = async () => {
    if (!dialog) return
    setDialogFailure(null)
    try {
      if (dialog.kind === 'reopen') {
        if (!reason.trim()) {
          setReasonError('Say why the period needs to take entries again.')
          return
        }
        await command.mutateAsync({ type: 'period.reopen', periodId: dialog.period.id, reason: reason.trim() })
        setDone(`${dialog.period.name} is open again. The reason is in the audit trail.`)
      } else if (dialog.kind === 'lock') {
        await command.mutateAsync({ type: 'period.lock', periodId: dialog.period.id })
        setDone(`${dialog.period.name} is locked.`)
      } else {
        await command.mutateAsync({ type: 'year.close', year: dialog.year.year })
        setDone(`${dialog.year.year} is closed. Income and expense have moved to net assets.`)
      }
      setDialog(null)
    } catch (e) {
      setDialogFailure(e)
    }
  }

  return (
    <BooksPage
      title="Periods"
      lede="Each month is a fiscal period. Close a month once its entries are complete, so nothing is backdated into figures already reported. Lock it once the accounts for it are signed off."
      nav={<SubNav label="Ledger" items={LEDGER_NAV} />}
      actions={
        nextYear !== null && (
          <Button variant="outline" size="lg" onClick={openYear} disabled={command.isPending}>
            Open fiscal year {nextYear}
          </Button>
        )
      }
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="periods" />
      ) : years.length === 0 ? (
        <ReportNotice title="No fiscal periods">Periods are created when the books are set up.</ReportNotice>
      ) : (
        <>
          {!!failure && <IssueSummary error={failure} title="Not closed" />}
          {done && (
            <p role="status" className="border border-border p-4 text-sm">
              {done}
            </p>
          )}
          <p className="flex flex-wrap gap-x-6 gap-y-2 text-xs text-muted-foreground">
            <PeriodStatusMark status="open" /> takes entries
            <PeriodStatusMark status="closed" /> can be reopened with a reason
            <PeriodStatusMark status="locked" /> sealed for good
          </p>
          {years.map((y) => (
            <ReportSection
              key={y.year}
              title={`Fiscal year ${y.year}`}
              meta={
                <>
                  <AsOf date={asOf} className="hidden sm:inline" />
                  {DEMO_BOOKS && <DemoTag />}
                </>
              }
              actions={
                y.closed ? (
                  <span className="font-semibold text-foreground">Year closed</span>
                ) : (
                  <Button
                    variant="outline"
                    onClick={() => openDialog({ kind: 'year', year: y })}
                    disabled={!y.canClose}
                    aria-describedby={!y.canClose ? `year-${y.year}-note` : undefined}
                  >
                    Close the year
                  </Button>
                )
              }
            >
              {!y.closed && !y.canClose && (
                <p id={`year-${y.year}-note`} className="border-b border-border py-2 text-xs text-muted-foreground">
                  The last period of {y.year} must be open to post the year-end close.
                </p>
              )}
              <ul className="divide-y divide-border">
                {y.periods.map((p) => {
                  const c = counts.get(p.id) ?? { posted: 0, drafts: 0 }
                  return (
                    <li
                      key={p.id}
                      className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-2 py-3 md:grid-cols-[10rem_8rem_minmax(0,1fr)_auto]"
                    >
                      <div className="min-w-0">
                        <p className="font-semibold">{p.name}</p>
                        <p className="text-xs text-muted-foreground tabular-nums">
                          {formatShortDate(p.start_date)} to {formatDate(p.end_date)}
                        </p>
                      </div>
                      <div className="justify-self-end md:justify-self-start">
                        <PeriodStatusMark status={p.status} />
                      </div>
                      <p className="text-sm text-muted-foreground">
                        <Link
                          to="/ledger"
                          search={{ period: p.id }}
                          className="inline-flex min-h-10 items-center underline-offset-4 hover:text-foreground hover:underline"
                        >
                          {c.posted} {c.posted === 1 ? 'entry' : 'entries'}
                        </Link>
                        {c.drafts > 0 && (
                          <>
                            {' · '}
                            <Link
                              to="/ledger"
                              search={{ period: p.id, status: 'draft' }}
                              className="font-semibold text-foreground underline underline-offset-4"
                            >
                              {c.drafts} {c.drafts === 1 ? 'draft' : 'drafts'}
                            </Link>
                          </>
                        )}
                      </p>
                      <div className="col-span-2 flex flex-wrap gap-2 md:col-span-1 md:justify-end">
                        {p.status === 'open' && (
                          <Button
                            variant="outline"
                            onClick={() => closePeriod(p)}
                            disabled={command.isPending}
                            aria-label={`Close ${p.name}`}
                          >
                            Close
                          </Button>
                        )}
                        {p.status === 'closed' && (
                          <>
                            <Button variant="ghost" onClick={() => openDialog({ kind: 'reopen', period: p })} aria-label={`Reopen ${p.name}`}>
                              Reopen
                            </Button>
                            <Button variant="outline" onClick={() => openDialog({ kind: 'lock', period: p })} aria-label={`Lock ${p.name}`}>
                              Lock
                            </Button>
                          </>
                        )}
                      </div>
                    </li>
                  )
                })}
              </ul>
            </ReportSection>
          ))}
        </>
      )}

      <ReportDialog
        open={dialog !== null}
        onOpenChange={(o) => !o && setDialog(null)}
        title={
          dialog?.kind === 'reopen'
            ? `Reopen ${dialog.period.name}`
            : dialog?.kind === 'lock'
              ? `Lock ${dialog.period.name}?`
              : dialog?.kind === 'year'
                ? `Close fiscal year ${dialog.year.year}?`
                : ''
        }
        footer={
          <>
            <Button variant="ghost" onClick={() => setDialog(null)}>
              Cancel
            </Button>
            <Button onClick={confirm} disabled={command.isPending}>
              {command.isPending
                ? 'Working…'
                : dialog?.kind === 'reopen'
                  ? 'Reopen period'
                  : dialog?.kind === 'lock'
                    ? 'Lock period'
                    : 'Close the year'}
            </Button>
          </>
        }
      >
        <div className="space-y-4 text-sm">
          <IssueSummary error={dialogFailure} title="Not done" />
          {dialog?.kind === 'reopen' && (
            <>
              <p>
                Reopening lets entries be dated in {dialog.period.name} again,
                which can change figures already reported. Periods reopen
                latest first.
              </p>
              <Field id="reopen-reason" label="Reason" required error={reasonError}>
                {(props) => (
                  <Textarea {...props} rows={3} value={reason} onChange={(e) => setReason(e.target.value)} />
                )}
              </Field>
            </>
          )}
          {dialog?.kind === 'lock' && (
            <p>
              A locked period can never be reopened or take entries. Lock{' '}
              {dialog.period.name} only once its accounts are signed off.
            </p>
          )}
          {dialog?.kind === 'year' && (
            <>
              <p>The year-end close does three things:</p>
              <ol className="list-decimal space-y-1 pl-5">
                <li>
                  Posts one closing entry dated{' '}
                  {formatDate(dialog.year.periods[dialog.year.periods.length - 1].end_date)} that
                  brings every income and expense account for {dialog.year.year} to zero.
                </li>
                <li>
                  Moves the result into net assets fund by fund: unrestricted
                  funds to unrestricted net assets, restricted funds to
                  restricted net assets.
                </li>
                <li>Closes every period of {dialog.year.year} that is still open.</li>
              </ol>
              {dialog.year.drafts > 0 && (
                <p className="font-semibold">
                  {dialog.year.drafts} draft {dialog.year.drafts === 1 ? 'entry is' : 'entries are'} still
                  in {dialog.year.year}. Drafts are not included in the close; post or delete them first.
                </p>
              )}
              <p className="text-muted-foreground">
                A year can be closed only once. Check the trial balance for{' '}
                {dialog.year.year} before you continue.
              </p>
            </>
          )}
        </div>
      </ReportDialog>
    </BooksPage>
  )
}
