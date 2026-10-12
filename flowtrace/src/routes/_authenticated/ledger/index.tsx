import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { MagnifyingGlass, Plus } from '@phosphor-icons/react'
import {
  ENTRY_STATUS_LABEL,
  EntryStatusMark,
  SOURCE_LABEL,
  entryTotal,
  linesByEntry,
  str,
} from './-shared'
import type { EntryStatus, JournalEntry, SourceType } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Field, FormSelect } from '@/components/form'
import {
  AsOf,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { BooksPage, BooksState, Money, SubNav } from '@/components/ledger'
import { LEDGER_NAV } from '@/components/ledger/ledger-nav'

type JournalSearch = {
  status?: string
  source?: string
  q?: string
  period?: string
}

const STATUSES: Array<EntryStatus> = ['draft', 'posted', 'reversed']

export const Route = createFileRoute('/_authenticated/ledger/')({
  validateSearch: (search: Record<string, unknown>): JournalSearch => ({
    status: str(search.status),
    source: str(search.source),
    q: str(search.q),
    period: str(search.period),
  }),
  component: JournalPage,
})

type Row = { entry: JournalEntry; total: number; lineCount: number }

function JournalPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const search = Route.useSearch()
  const navigate = useNavigate({ from: Route.fullPath })
  const status = STATUSES.includes(search.status as EntryStatus)
    ? (search.status as EntryStatus)
    : undefined
  const source = search.source as SourceType | undefined
  const q = search.q ?? ''

  const all = useMemo<Array<Row>>(() => {
    if (!state) return []
    const byEntry = linesByEntry(state)
    return state.entries
      .map((entry) => {
        const lines = byEntry.get(entry.id) ?? []
        return { entry, total: entryTotal(lines), lineCount: lines.length }
      })
      .sort(
        (a, b) =>
          b.entry.date.localeCompare(a.entry.date) ||
          b.entry.number.localeCompare(a.entry.number),
      )
  }, [state])

  const counts = useMemo(() => {
    const c = { all: all.length, draft: 0, posted: 0, reversed: 0 }
    for (const r of all) c[r.entry.status]++
    return c
  }, [all])

  const rows = useMemo(() => {
    const needle = q.trim().toLowerCase()
    return all.filter(({ entry }) => {
      if (status && entry.status !== status) return false
      if (source && entry.source_type !== source) return false
      if (search.period && entry.period_id !== search.period) return false
      if (!needle) return true
      return [entry.number, entry.memo, entry.override_reason].some((f) =>
        f?.toLowerCase().includes(needle),
      )
    })
  }, [all, status, source, search.period, q])

  const sourceOptions = useMemo(() => {
    const present = new Set(all.map((r) => r.entry.source_type))
    return [
      { value: '', label: 'All sources' },
      ...(Object.keys(SOURCE_LABEL) as Array<SourceType>)
        .filter((s) => present.has(s))
        .map((s) => ({ value: s, label: SOURCE_LABEL[s] })),
    ]
  }, [all])

  const periodOptions = useMemo(
    () => [
      { value: '', label: 'All periods' },
      ...[...(state?.periods ?? [])]
        .sort((a, b) => b.start_date.localeCompare(a.start_date))
        .map((p) => ({ value: p.id, label: p.name })),
    ],
    [state],
  )

  const setSearch = (patch: Partial<JournalSearch>) =>
    navigate({ search: (prev) => ({ ...prev, ...patch }), replace: true })

  const filtered = !!(status || source || search.period || q)
  const shownTotal = rows.reduce((t, r) => t + r.total, 0)
  const base = state?.settings.base_currency ?? 'KES'

  return (
    <BooksPage
      title="Journal"
      lede="Every entry in the books, in date order. Drafts are not in the books until they are posted; posted entries are never edited, only reversed."
      actions={
        <Link to="/ledger/new" className={buttonVariants({ size: 'lg' })}>
          <Plus />
          New journal entry
        </Link>
      }
      nav={<SubNav label="Ledger" items={LEDGER_NAV} />}
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="journal" />
      ) : (
        <>
          <ReportSection
            title="Entries in the books"
            meta={
              <>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            <KeyFigures>
              <KeyFigure scale="lg" value={counts.posted} label="Posted" />
              <KeyFigure
                value={counts.draft}
                label="Drafts"
                detail={<span className="text-muted-foreground">Not yet in the books</span>}
              />
              <KeyFigure value={counts.reversed} label="Reversed" />
              <KeyFigure value={counts.all} label="All entries" />
            </KeyFigures>
          </ReportSection>

          <ReportSection title="Entries" meta={<span>{rows.length} shown</span>}>
            <div className="space-y-3 border-b border-border py-3">
              <nav aria-label="Filter by status" className="flex">
                <ul className="flex flex-wrap gap-px border border-border bg-border">
                  {[undefined, ...STATUSES].map((s) => {
                    const active = status === s
                    const n = s ? counts[s] : counts.all
                    return (
                      <li key={s ?? 'all'} className="grow bg-background sm:grow-0">
                        <Link
                          to="/ledger"
                          search={(prev) => ({ ...prev, status: s })}
                          replace
                          aria-current={active ? 'page' : undefined}
                          className={cn(
                            'flex h-10 items-center justify-center gap-2 px-3 text-sm whitespace-nowrap transition-colors',
                            active
                              ? 'bg-primary font-semibold text-primary-foreground'
                              : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                          )}
                        >
                          {s ? ENTRY_STATUS_LABEL[s] : 'All'}
                          <span className="tabular-nums opacity-80">{n}</span>
                        </Link>
                      </li>
                    )
                  })}
                </ul>
              </nav>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-[minmax(0,1fr)_14rem_12rem]">
                <div className="relative sm:col-span-2 lg:col-span-1 lg:self-end">
                  <label htmlFor="journal-search" className="mb-1.5 block text-sm font-medium">
                    Search
                  </label>
                  <MagnifyingGlass
                    aria-hidden="true"
                    className="pointer-events-none absolute bottom-3 left-3 size-4 text-muted-foreground"
                  />
                  <Input
                    id="journal-search"
                    type="search"
                    value={q}
                    onChange={(e) => setSearch({ q: e.target.value || undefined })}
                    placeholder="Number, memo or override reason"
                    className="h-10 pl-9"
                  />
                </div>
                <Field id="journal-source" label="Source" fixed>
                  {(props) => (
                    <FormSelect
                      {...props}
                      value={source ?? ''}
                      onValueChange={(v) => setSearch({ source: v || undefined })}
                      options={sourceOptions}
                    />
                  )}
                </Field>
                <Field id="journal-period" label="Period" fixed>
                  {(props) => (
                    <FormSelect
                      {...props}
                      value={search.period ?? ''}
                      onValueChange={(v) => setSearch({ period: v || undefined })}
                      options={periodOptions}
                    />
                  )}
                </Field>
              </div>
            </div>

            {all.length === 0 ? (
              <ReportNotice
                title="No entries yet"
                action={
                  <Link to="/ledger/new" className={buttonVariants()}>
                    <Plus />
                    New journal entry
                  </Link>
                }
              >
                Entries appear here as transactions, bills and claims post, or
                when you write one by hand.
              </ReportNotice>
            ) : rows.length === 0 ? (
              <ReportNotice
                title="No entries match"
                action={
                  <Link to="/ledger" search={{}} className={buttonVariants({ variant: 'outline' })}>
                    Clear filters
                  </Link>
                }
              >
                {q ? `Nothing matches “${q}” with these filters.` : 'No entries have this combination of status, source and period.'}
              </ReportNotice>
            ) : (
              <>
                <EntryTable rows={rows} />
                <EntryStack rows={rows} />
                <p className="flex items-baseline justify-between gap-3 border-t-2 border-foreground py-3 text-xs text-muted-foreground">
                  <span>
                    {filtered ? 'Shown' : 'Total'}: {rows.length}{' '}
                    {rows.length === 1 ? 'entry' : 'entries'}
                  </span>
                  <span>
                    Debits {base}{' '}
                    <Money value={shownTotal} strong className="text-sm text-foreground" />
                  </span>
                </p>
              </>
            )}
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}

function EntryTable({ rows }: { rows: Array<Row> }) {
  const navigate = useNavigate()
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[52rem] text-sm">
        <caption className="sr-only">
          Journal entries with number, date, memo, source, status and total
        </caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">Number</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Date</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Memo</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Source</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Status</th>
            <th scope="col" className="py-2.5 text-right font-semibold">Total</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ entry, total, lineCount }) => (
            <tr
              key={entry.id}
              onClick={(e) => {
                if ((e.target as HTMLElement).closest('a')) return
                navigate({ to: '/ledger/entries/$id', params: { id: entry.id } })
              }}
              className="cursor-pointer border-b border-border align-top transition-colors hover:bg-muted"
            >
              <td className="py-3 pr-4 whitespace-nowrap">
                <Link
                  to="/ledger/entries/$id"
                  params={{ id: entry.id }}
                  className="font-mono font-medium underline-offset-4 hover:underline focus-visible:underline"
                >
                  {entry.number}
                </Link>
              </td>
              <td className="py-3 pr-4 whitespace-nowrap text-muted-foreground tabular-nums">
                <time dateTime={entry.date}>{formatDate(entry.date)}</time>
              </td>
              <td className="max-w-[26rem] py-3 pr-4">
                <p className="font-semibold">{entry.memo}</p>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {lineCount} {lineCount === 1 ? 'line' : 'lines'}
                  {entry.override_reason && entry.source_type !== 'reversal' && entry.source_type !== 'year_end_close'
                    ? ' · posted with an override'
                    : ''}
                </p>
              </td>
              <td className="py-3 pr-4 whitespace-nowrap">{SOURCE_LABEL[entry.source_type]}</td>
              <td className="py-3 pr-4">
                <EntryStatusMark status={entry.status} />
              </td>
              <td className="py-3 text-right">
                <Money value={total} strong />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function EntryStack({ rows }: { rows: Array<Row> }) {
  return (
    <ul className="divide-y divide-border md:hidden">
      {rows.map(({ entry, total }) => (
        <li key={entry.id}>
          <Link
            to="/ledger/entries/$id"
            params={{ id: entry.id }}
            className="-mx-4 block px-4 py-3.5 transition-colors hover:bg-muted"
          >
            <div className="flex items-start justify-between gap-3">
              <p className="min-w-0 font-semibold">{entry.memo}</p>
              <Money value={total} strong className="shrink-0" />
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <EntryStatusMark status={entry.status} />
              <span className="font-mono">{entry.number}</span>
              <span className="tabular-nums">{formatShortDate(entry.date)}</span>
              <span>{SOURCE_LABEL[entry.source_type]}</span>
            </div>
          </Link>
        </li>
      ))}
    </ul>
  )
}
