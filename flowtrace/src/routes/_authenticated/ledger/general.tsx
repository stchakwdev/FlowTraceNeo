import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useEffect, useMemo, useState } from 'react'
import { DownloadSimple } from '@phosphor-icons/react'
import { ISO_DATE, str } from './-shared'
import type { Account, AccountType, LedgerState, Minor, ReportLine } from '@/lib/ledger'
import { natural, postedLines } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { formatDate, formatShortDate } from '@/lib/format'
import { Button, buttonVariants } from '@/components/ui/button'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Field, FormSelect } from '@/components/form'
import {
  AsOf,
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
  SubNav,
  csvAmount,
  downloadCsv,
  formatMinorCompact,
} from '@/components/ledger'
import { LEDGER_NAV } from '@/components/ledger/ledger-nav'

type GLSearch = {
  account?: string
  fund?: string
  grant?: string
  dimension?: string
  from?: string
  to?: string
  source?: string
  node?: string
}

const iso = (v: unknown) => {
  const s = str(v)
  return s && ISO_DATE.test(s) ? s : undefined
}

export const Route = createFileRoute('/_authenticated/ledger/general')({
  validateSearch: (search: Record<string, unknown>): GLSearch => ({
    account: str(search.account),
    fund: str(search.fund),
    grant: str(search.grant),
    dimension: str(search.dimension),
    from: iso(search.from),
    to: iso(search.to),
    source: str(search.source),
    node: str(search.node),
  }),
  component: GeneralLedgerPage,
})

const TYPE_LABEL: Record<AccountType, string> = {
  asset: 'Assets',
  liability: 'Liabilities',
  equity: 'Net assets',
  income: 'Income',
  expense: 'Expenses',
}

const DIMENSION_LABEL = { project: 'Projects', department: 'Departments', location: 'Locations', donor: 'Donors' }

type Group = {
  account: Account
  opening: Minor
  rows: Array<{ line: ReportLine; running: Minor }>
  closing: Minor
  debits: Minor
  credits: Minor
}

function buildLedger(state: LedgerState, s: GLSearch): Array<Group> {
  let lines = postedLines(state, {
    accountIds: s.account ? [s.account] : undefined,
    fundId: s.fund,
    grantId: s.grant,
    dimensionId: s.dimension,
  })
  if (s.source) lines = lines.filter((l) => l.entry.source_id === s.source || l.entry.id === s.source)
  if (s.node) lines = lines.filter((l) => l.flow_node_id === s.node)
  lines.sort(
    (a, b) => a.entry.date.localeCompare(b.entry.date) || a.entry.number.localeCompare(b.entry.number),
  )
  const accounts = new Map(state.accounts.map((a) => [a.id, a]))
  const groups = new Map<string, Group>()
  const groupOf = (id: string) => {
    let g = groups.get(id)
    if (!g) {
      g = { account: accounts.get(id)!, opening: 0, rows: [], closing: 0, debits: 0, credits: 0 }
      groups.set(id, g)
    }
    return g
  }
  for (const l of lines) {
    if (s.to && l.entry.date > s.to) continue
    if (!accounts.has(l.account_id)) continue
    const g = groupOf(l.account_id)
    if (s.from && l.entry.date < s.from) {
      g.opening += l.base_amount
      g.closing += l.base_amount
      continue
    }
    g.closing += l.base_amount
    if (l.base_amount > 0) g.debits += l.base_amount
    else g.credits -= l.base_amount
    g.rows.push({ line: l, running: g.closing })
  }
  return [...groups.values()]
    .filter((g) => g.rows.length || g.opening !== 0)
    .sort((a, b) => a.account.code.localeCompare(b.account.code))
}

function GeneralLedgerPage() {
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const search = Route.useSearch()
  const navigate = useNavigate({ from: Route.fullPath })

  const setSearch = (patch: Partial<GLSearch>) =>
    navigate({ search: (prev) => ({ ...prev, ...patch }), replace: true })

  const groups = useMemo(() => (state ? buildLedger(state, search) : []), [state, search])
  const totals = useMemo(
    () =>
      groups.reduce(
        (t, g) => ({ lines: t.lines + g.rows.length, debits: t.debits + g.debits, credits: t.credits + g.credits }),
        { lines: 0, debits: 0, credits: 0 },
      ),
    [groups],
  )
  const base = state?.settings.base_currency ?? 'KES'

  const scopeNote = useMemo(() => {
    if (!state) return null
    if (search.source) {
      const e = state.entries.find((x) => x.id === search.source)
      return e ? `Lines of ${e.number} and anything it reverses or is reversed by` : 'Lines from one source document'
    }
    if (search.node) {
      const d = state.dimensions.find((x) => x.node_id === search.node)
      const g = state.grants.find((x) => x.donor_node_id === search.node)
      return `Lines recorded against the flow node ${d?.name ?? g?.donor_name ?? 'selected on the flow map'}`
    }
    return null
  }, [state, search.source, search.node])

  const exportCsv = () => {
    if (!state) return
    const fund = (id: string) => state.funds.find((f) => f.id === id)?.name ?? ''
    const grant = (id: string | null) => (id ? (state.grants.find((g) => g.id === id)?.name ?? '') : '')
    const rows: Array<Array<unknown>> = [
      ['Account code', 'Account', 'Date', 'Entry', 'Memo', 'Fund', 'Grant', `Debit (${base})`, `Credit (${base})`, `Balance (${base})`],
    ]
    for (const g of groups) {
      rows.push([g.account.code, g.account.name, search.from ?? '', '', 'Opening balance', '', '', '', '', csvAmount(natural(g.account, g.opening))])
      for (const { line, running } of g.rows)
        rows.push([
          g.account.code,
          g.account.name,
          line.entry.date,
          line.entry.number,
          line.memo ?? line.entry.memo,
          fund(line.fund_id),
          grant(line.grant_id),
          line.base_amount > 0 ? csvAmount(line.base_amount) : '',
          line.base_amount < 0 ? csvAmount(-line.base_amount) : '',
          csvAmount(natural(g.account, running)),
        ])
    }
    downloadCsv(`general-ledger-${search.from ?? 'start'}-to-${search.to ?? asOf}`, rows)
  }

  return (
    <BooksPage
      title="General ledger"
      lede="Every posted line, account by account, with the balance after each one. Header accounts include all of their sub-accounts."
      actions={
        <Button variant="outline" onClick={exportCsv} disabled={!state || groups.length === 0}>
          <DownloadSimple />
          Export CSV
        </Button>
      }
      nav={<SubNav label="Ledger" items={LEDGER_NAV} />}
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="general ledger" />
      ) : (
        <>
          <ReportSection title="Filters">
            <Filters state={state} search={search} setSearch={setSearch} />
            {scopeNote && (
              <p className="flex flex-wrap items-center justify-between gap-2 border-b border-border py-3 text-sm">
                <span className="font-semibold">{scopeNote}</span>
                <Link
                  to="/ledger/general"
                  search={(prev) => ({ ...prev, source: undefined, node: undefined })}
                  className={buttonVariants({ variant: 'ghost' })}
                >
                  Show all lines
                </Link>
              </p>
            )}
          </ReportSection>

          <ReportSection
            title="Ledger"
            meta={
              <>
                <AsOf date={search.to ?? asOf} />
                {DEMO_BOOKS && <DemoTag />}
              </>
            }
          >
            <KeyFigures>
              <KeyFigure scale="lg" value={formatMinorCompact(totals.debits)} label={`Debits, ${base}`} />
              <KeyFigure value={formatMinorCompact(totals.credits)} label={`Credits, ${base}`} />
              <KeyFigure value={totals.lines} label="Lines" />
              <KeyFigure value={groups.length} label="Accounts" />
            </KeyFigures>
            {groups.length === 0 ? (
              <ReportNotice
                title="No posted lines match"
                action={
                  <Link to="/ledger/general" search={{}} className={buttonVariants({ variant: 'outline' })}>
                    Clear filters
                  </Link>
                }
              >
                Drafts are not shown here. Widen the dates or remove a filter.
              </ReportNotice>
            ) : (
              <>
                <LedgerTable groups={groups} state={state} from={search.from} />
                <LedgerStack groups={groups} from={search.from} />
              </>
            )}
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}

function DateFilter({
  id,
  label,
  value,
  onCommit,
}: {
  id: string
  label: string
  value?: string
  onCommit: (iso: string | undefined) => void
}) {
  const [text, setText] = useState(isoToDayMonthYear(value))
  const parsed = parseDayMonthYear(text)
  // Clear the text when the filter is cleared elsewhere (e.g. Clear filters).
  useEffect(() => {
    if (!value) setText((t) => (parseDayMonthYear(t)?.iso ? '' : t))
    else setText((t) => (parseDayMonthYear(t)?.iso === value ? t : isoToDayMonthYear(value)))
  }, [value])
  return (
    <Field id={id} label={label} error={parsed?.error}>
      {(props) => (
        <DateField
          {...props}
          value={text}
          onValueChange={(t) => {
            setText(t)
            const p = parseDayMonthYear(t)
            if (!p) onCommit(undefined)
            else if (p.iso) onCommit(p.iso)
          }}
          pickerLabel={`Choose the ${label.toLowerCase()} date from a calendar`}
          className="h-10"
        />
      )}
    </Field>
  )
}

function Filters({
  state,
  search,
  setSearch,
}: {
  state: LedgerState
  search: GLSearch
  setSearch: (patch: Partial<GLSearch>) => void
}) {
  // Every account, headers included: choosing a header shows its sub-accounts.
  const accountGroups = useMemo(
    () =>
      (Object.keys(TYPE_LABEL) as Array<AccountType>)
        .map((type) => ({
          label: TYPE_LABEL[type],
          options: state.accounts
            .filter((a) => a.type === type)
            .sort((a, b) => a.code.localeCompare(b.code))
            .map((a) => ({
              value: a.id,
              label: `${a.code} · ${a.name}${a.is_group ? ' (with sub-accounts)' : ''}`,
            })),
        }))
        .filter((g) => g.options.length),
    [state.accounts],
  )

  const dimensionGroups = useMemo(
    () =>
      (['project', 'department', 'location', 'donor'] as const)
        .map((k) => ({
          label: DIMENSION_LABEL[k],
          options: state.dimensions.filter((d) => d.kind === k).map((d) => ({ value: d.id, label: d.name })),
        }))
        .filter((g) => g.options.length),
    [state.dimensions],
  )

  return (
    <div className="grid gap-4 border-b border-border py-4 sm:grid-cols-2 lg:grid-cols-3">
      <Field id="gl-account" label="Account" fixed>
        {(props) => (
          <FormSelect
            {...props}
            value={search.account ?? ''}
            onValueChange={(v) => setSearch({ account: v || undefined })}
            options={[{ value: '', label: 'All accounts' }]}
            groups={accountGroups}
          />
        )}
      </Field>
      <Field id="gl-fund" label="Fund" fixed>
        {(props) => (
          <FormSelect
            {...props}
            value={search.fund ?? ''}
            onValueChange={(v) => {
              const g = state.grants.find((x) => x.id === search.grant)
              setSearch({ fund: v || undefined, grant: g && v && g.fund_id !== v ? undefined : search.grant })
            }}
            options={[{ value: '', label: 'All funds' }, ...state.funds.map((f) => ({ value: f.id, label: f.name }))]}
          />
        )}
      </Field>
      <Field id="gl-grant" label="Grant" fixed>
        {(props) => (
          <FormSelect
            {...props}
            value={search.grant ?? ''}
            onValueChange={(v) => setSearch({ grant: v || undefined })}
            options={[
              { value: '', label: 'All grants' },
              ...state.grants
                .filter((g) => !search.fund || g.fund_id === search.fund)
                .map((g) => ({ value: g.id, label: g.name })),
            ]}
          />
        )}
      </Field>
      <Field id="gl-dimension" label="Tag" fixed>
        {(props) => (
          <FormSelect
            {...props}
            value={search.dimension ?? ''}
            onValueChange={(v) => setSearch({ dimension: v || undefined })}
            options={[{ value: '', label: 'All tags' }]}
            groups={dimensionGroups}
          />
        )}
      </Field>
      <DateFilter id="gl-from" label="From" value={search.from} onCommit={(v) => setSearch({ from: v })} />
      <DateFilter id="gl-to" label="To" value={search.to} onCommit={(v) => setSearch({ to: v })} />
    </div>
  )
}

function LedgerTable({ groups, state, from }: { groups: Array<Group>; state: LedgerState; from?: string }) {
  const fund = (id: string) => state.funds.find((f) => f.id === id)?.name ?? ''
  const grant = (id: string | null) => (id ? state.grants.find((g) => g.id === id)?.name : undefined)
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[60rem] text-sm">
        <caption className="sr-only">
          General ledger by account, with opening balance, each posted line, running balance and closing balance
        </caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">Date</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Entry</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Memo</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Fund and grant</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Debit</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Credit</th>
            <th scope="col" className="py-2.5 text-right font-semibold">Balance</th>
          </tr>
        </thead>
        {groups.map((g) => (
          <tbody key={g.account.id}>
            <tr className="border-t-2 border-foreground">
              <th scope="rowgroup" colSpan={6} className="pt-4 pb-2 pr-4 text-left">
                <span className="font-mono text-xs text-muted-foreground">{g.account.code}</span>{' '}
                <span className="font-semibold">{g.account.name}</span>
              </th>
              <td />
            </tr>
            <tr className="border-b border-border text-muted-foreground">
              <td colSpan={6} className="py-2 pr-4 text-xs">
                Opening balance{from ? ` at ${formatDate(from)}` : ''}
              </td>
              <td className="py-2 text-right"><Money value={natural(g.account, g.opening)} zero="0.00" /></td>
            </tr>
            {g.rows.map(({ line, running }) => (
              <tr key={line.id} className="border-b border-border align-top">
                <td className="py-2.5 pr-4 whitespace-nowrap text-muted-foreground tabular-nums">
                  <time dateTime={line.entry.date}>{formatShortDate(line.entry.date)}</time>
                </td>
                <td className="py-2.5 pr-4 whitespace-nowrap">
                  <Link
                    to="/ledger/entries/$id"
                    params={{ id: line.entry.id }}
                    className="inline-flex min-h-6 items-center font-mono underline-offset-4 hover:underline focus-visible:underline"
                  >
                    {line.entry.number}
                  </Link>
                </td>
                <td className="max-w-[22rem] py-2.5 pr-4">{line.memo ?? line.entry.memo}</td>
                <td className="py-2.5 pr-4 text-muted-foreground">
                  {fund(line.fund_id)}
                  {grant(line.grant_id) && <span className="block text-xs">{grant(line.grant_id)}</span>}
                </td>
                <td className="py-2.5 pr-4 text-right"><Money value={Math.max(0, line.base_amount)} /></td>
                <td className="py-2.5 pr-4 text-right"><Money value={Math.max(0, -line.base_amount)} /></td>
                <td className="py-2.5 text-right"><Money value={natural(g.account, running)} zero="0.00" /></td>
              </tr>
            ))}
            <tr>
              <td colSpan={4} className="py-2.5 pr-4 text-xs text-muted-foreground">Closing balance</td>
              <td className="py-2.5 pr-4 text-right"><Money value={g.debits} strong /></td>
              <td className="py-2.5 pr-4 text-right"><Money value={g.credits} strong /></td>
              <td className="py-2.5 text-right"><Money value={natural(g.account, g.closing)} zero="0.00" strong /></td>
            </tr>
          </tbody>
        ))}
      </table>
    </div>
  )
}

function LedgerStack({ groups, from }: { groups: Array<Group>; from?: string }) {
  return (
    <div className="md:hidden">
      {groups.map((g) => (
        <section key={g.account.id} aria-labelledby={`gl-${g.account.id}`} className="border-t-2 border-foreground py-3">
          <h3 id={`gl-${g.account.id}`} className="text-base font-semibold [font-variation-settings:'wdth'_100]">
            <span className="font-mono text-xs font-normal text-muted-foreground">{g.account.code}</span> {g.account.name}
          </h3>
          <p className="mt-1 flex justify-between text-xs text-muted-foreground">
            <span>Opening{from ? ` at ${formatShortDate(from)}` : ''}</span>
            <Money value={natural(g.account, g.opening)} zero="0.00" />
          </p>
          <ul className="mt-2 divide-y divide-border border-y border-border">
            {g.rows.map(({ line, running }) => (
              <li key={line.id}>
                <Link
                  to="/ledger/entries/$id"
                  params={{ id: line.entry.id }}
                  className="-mx-4 block px-4 py-2.5 transition-colors hover:bg-muted"
                >
                  <div className="flex items-start justify-between gap-3 text-sm">
                    <p className="min-w-0">{line.memo ?? line.entry.memo}</p>
                    <p className="shrink-0 text-right">
                      <span className="text-xs text-muted-foreground">{line.base_amount > 0 ? 'Dr ' : 'Cr '}</span>
                      <Money value={Math.abs(line.base_amount)} strong />
                    </p>
                  </div>
                  <p className="mt-1 flex justify-between gap-3 text-xs text-muted-foreground">
                    <span>
                      <span className="tabular-nums">{formatShortDate(line.entry.date)}</span> ·{' '}
                      <span className="font-mono">{line.entry.number}</span>
                    </span>
                    <span>
                      Balance <Money value={natural(g.account, running)} zero="0.00" />
                    </span>
                  </p>
                </Link>
              </li>
            ))}
          </ul>
          <p className="mt-2 flex justify-between text-xs text-muted-foreground">
            <span>Closing</span>
            <Money value={natural(g.account, g.closing)} zero="0.00" strong className="text-sm text-foreground" />
          </p>
        </section>
      ))}
    </div>
  )
}
