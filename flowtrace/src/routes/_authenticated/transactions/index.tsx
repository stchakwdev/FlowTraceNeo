import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { ArrowRight, MagnifyingGlass, Plus } from '@phosphor-icons/react'
import type { TransactionWithNodes } from '@/lib/types'
import { useTransactions } from '@/lib/queries'
import { formatAmount, formatDate, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  ProofLegend,
  ReportNotice,
  ReportSection,
  StatusLine,
  StatusMark,
} from '@/components/report'

// Status filter values in the URL use the stored verification states, so
// other screens can link straight to e.g. /transactions?status=pending.
const FILTERS = [
  { value: undefined, label: 'All' },
  { value: 'verified', label: 'Evidenced' },
  { value: 'pending', label: 'Awaiting proof' },
  { value: 'flagged', label: 'Flagged' },
] as const

type StatusFilter = 'verified' | 'pending' | 'flagged'

const ALIASES: Record<string, StatusFilter> = {
  verified: 'verified',
  evidenced: 'verified',
  pending: 'pending',
  'awaiting-proof': 'pending',
  awaiting: 'pending',
  flagged: 'flagged',
}

// Search params pass through unchanged (rewriting them on the server makes
// the router answer a hard load with an empty redirect); aliases such as
// ?status=evidenced or the older ?filter=pending are resolved when read.
type TransactionSearch = { status?: string; filter?: string; q?: string }

const str = (v: unknown) => (typeof v === 'string' && v ? v : undefined)

function resolveStatus(search: TransactionSearch): StatusFilter | undefined {
  const raw = search.status ?? search.filter
  return raw ? ALIASES[raw.toLowerCase()] : undefined
}

export const Route = createFileRoute('/_authenticated/transactions/')({
  validateSearch: (search: Record<string, unknown>): TransactionSearch => ({
    status: str(search.status),
    filter: str(search.filter),
    q: str(search.q),
  }),
  component: TransactionList,
})

function TransactionList() {
  const search = Route.useSearch()
  const status = resolveStatus(search)
  const q = search.q
  const navigate = useNavigate({ from: Route.fullPath })
  const {
    data: transactions,
    isLoading,
    error,
    refetch,
    isRefetching,
  } = useTransactions()

  const counts = useMemo(() => {
    const c = { all: 0, verified: 0, pending: 0, flagged: 0 }
    for (const t of transactions ?? []) {
      c.all++
      if (t.verification_status in c) c[t.verification_status as StatusFilter]++
    }
    return c
  }, [transactions])

  const rows = useMemo(() => {
    const needle = (q ?? '').trim().toLowerCase()
    return (transactions ?? []).filter((t) => {
      if (status && t.verification_status !== status) return false
      if (!needle) return true
      return [
        t.description,
        t.reference,
        t.category,
        t.from_node?.name,
        t.to_node?.name,
      ].some((field) => field?.toLowerCase().includes(needle))
    })
  }, [transactions, status, q])

  const currencies = new Set(rows.map((t) => t.currency))
  const total =
    currencies.size === 1 ? rows.reduce((sum, t) => sum + t.amount, 0) : null

  const setSearch = (value: string) =>
    navigate({
      search: (prev) => ({ ...prev, q: value || undefined }),
      replace: true,
    })

  const filtered = status !== undefined || !!q

  return (
    <div className="mx-auto max-w-7xl px-4 py-6 sm:px-8 sm:py-8">
      <header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-4xl sm:text-5xl">Transactions</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {transactions
              ? `${counts.all} movements recorded · ${counts.pending} awaiting proof · ${counts.flagged} flagged`
              : 'Every movement of money, with the proof attached to it'}
          </p>
        </div>
        <Link
          to="/transactions/new"
          className={cn(
            buttonVariants({ size: 'lg' }),
            'self-start sm:self-auto',
          )}
        >
          <Plus />
          Record a transaction
        </Link>
      </header>

      <ReportSection
        className="mt-8"
        title="Movements"
        meta={<ProofLegend className="hidden md:flex" />}
      >
        {/* Filters */}
        <div className="flex flex-col gap-3 border-b border-border py-3 lg:flex-row lg:items-center lg:justify-between">
          <nav aria-label="Filter by proof status" className="flex">
            <ul className="flex flex-wrap gap-px border border-border bg-border">
              {FILTERS.map((f) => {
                const active = status === f.value
                const n = f.value ? counts[f.value] : counts.all
                return (
                  <li key={f.label} className="grow bg-background sm:grow-0">
                    <Link
                      to="/transactions"
                      search={(prev) => ({ q: prev.q, status: f.value })}
                      replace
                      aria-current={active ? 'page' : undefined}
                      className={cn(
                        'flex h-10 items-center justify-center gap-2 px-3 text-sm whitespace-nowrap transition-colors',
                        active
                          ? 'bg-primary font-semibold text-primary-foreground'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                      )}
                    >
                      {f.value && (
                        <StatusLine
                          status={f.value}
                          width={20}
                          className={cn(
                            active &&
                              '[&_line]:stroke-primary-foreground [&_path]:stroke-primary-foreground',
                          )}
                        />
                      )}
                      {f.label}
                      {transactions && (
                        <span className="tabular-nums opacity-80">{n}</span>
                      )}
                    </Link>
                  </li>
                )
              })}
            </ul>
          </nav>

          <div className="relative w-full lg:max-w-xs">
            <label htmlFor="txn-search" className="sr-only">
              Search transactions
            </label>
            <MagnifyingGlass
              aria-hidden="true"
              className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
            />
            <Input
              id="txn-search"
              type="search"
              value={q ?? ''}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Description, reference or node"
              className="h-10 pl-9"
            />
          </div>
        </div>

        {isLoading ? (
          <LoadingRows />
        ) : error ? (
          <ReportNotice
            tone="error"
            title="Transactions could not be loaded"
            action={
              <Button
                variant="outline"
                onClick={() => refetch()}
                disabled={isRefetching}
              >
                {isRefetching ? 'Trying again…' : 'Try again'}
              </Button>
            }
          >
            {error instanceof Error ? `${error.message}. ` : ''}Check your
            connection, then try again.
          </ReportNotice>
        ) : counts.all === 0 ? (
          <ReportNotice
            title="No transactions recorded yet"
            action={
              <Link to="/transactions/new" className={buttonVariants()}>
                <Plus />
                Record a transaction
              </Link>
            }
          >
            Each grant received, budget allocated and payment made appears here
            with its proof. Record the first one to start the report.
          </ReportNotice>
        ) : rows.length === 0 ? (
          <ReportNotice
            title="No transactions match"
            action={
              <Link
                to="/transactions"
                search={{}}
                className={buttonVariants({ variant: 'outline' })}
              >
                Clear filters
              </Link>
            }
          >
            {q
              ? `Nothing ${status ? `${FILTERS.find((f) => f.value === status)?.label.toLowerCase()} ` : ''}matches “${q}”.`
              : 'No transactions have this status.'}
          </ReportNotice>
        ) : (
          <>
            <TransactionTable rows={rows} total={total} filtered={filtered} />
            <TransactionStack rows={rows} total={total} filtered={filtered} />
          </>
        )}
      </ReportSection>
    </div>
  )
}

function TotalLabel({ count, filtered }: { count: number; filtered: boolean }) {
  return (
    <>
      {filtered ? 'Shown' : 'Total'}: {count}{' '}
      {count === 1 ? 'transaction' : 'transactions'}
    </>
  )
}

function TransactionTable({
  rows,
  total,
  filtered,
}: {
  rows: Array<TransactionWithNodes>
  total: number | null
  filtered: boolean
}) {
  const navigate = useNavigate()
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[56rem] text-sm">
        <caption className="sr-only">
          Transactions with amount, nodes and proof status
        </caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">
              Date
            </th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">
              Description
            </th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">
              From → To
            </th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">
              Amount
            </th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">
              Proofs
            </th>
            <th scope="col" className="py-2.5 font-semibold">
              Status
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((t) => (
            <tr
              key={t.id}
              onClick={(e) => {
                if ((e.target as HTMLElement).closest('a')) return
                navigate({ to: '/transactions/$id', params: { id: t.id } })
              }}
              className="cursor-pointer border-b border-border align-top transition-colors hover:bg-muted"
            >
              <td className="py-3 pr-4 whitespace-nowrap tabular-nums text-muted-foreground">
                <time dateTime={t.transaction_date}>
                  {formatDate(t.transaction_date)}
                </time>
              </td>
              <td className="max-w-[22rem] py-3 pr-4">
                <Link
                  to="/transactions/$id"
                  params={{ id: t.id }}
                  className="font-semibold underline-offset-4 hover:underline focus-visible:underline"
                >
                  {t.description}
                </Link>
                <p className="mt-0.5 font-mono text-xs text-muted-foreground">
                  {t.reference ?? 'No reference'}
                </p>
              </td>
              <td className="py-3 pr-4">
                <span className="inline-flex flex-wrap items-center gap-x-1.5">
                  <span>{t.from_node?.name ?? 'Unassigned'}</span>
                  <ArrowRight
                    aria-label="to"
                    className="size-3.5 text-muted-foreground"
                  />
                  <span>{t.to_node?.name ?? 'Unassigned'}</span>
                </span>
              </td>
              <td className="py-3 pr-4 text-right whitespace-nowrap">
                <span className="text-xs text-muted-foreground">
                  {t.currency}
                </span>{' '}
                <span className="font-semibold tabular-nums">
                  {formatAmount(t.amount)}
                </span>
              </td>
              <td
                className={cn(
                  'py-3 pr-4 text-right tabular-nums',
                  t.verification_count === 0 && 'text-muted-foreground',
                )}
              >
                {t.verification_count}
              </td>
              <td className="py-3">
                <StatusMark status={t.verification_status} />
              </td>
            </tr>
          ))}
        </tbody>
        {total !== null && (
          <tfoot>
            <tr className="border-t-2 border-foreground">
              <td
                colSpan={3}
                className="py-3 pr-4 text-xs text-muted-foreground"
              >
                <TotalLabel count={rows.length} filtered={filtered} />
              </td>
              <td className="py-3 pr-4 text-right whitespace-nowrap">
                <span className="text-xs text-muted-foreground">
                  {rows[0].currency}
                </span>{' '}
                <span className="font-bold tabular-nums">
                  {formatAmount(total)}
                </span>
              </td>
              <td colSpan={2} />
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  )
}

function TransactionStack({
  rows,
  total,
  filtered,
}: {
  rows: Array<TransactionWithNodes>
  total: number | null
  filtered: boolean
}) {
  return (
    <div className="md:hidden">
      <ul className="divide-y divide-border">
        {rows.map((t) => (
          <li key={t.id}>
            <Link
              to="/transactions/$id"
              params={{ id: t.id }}
              className="-mx-4 block px-4 py-3.5 transition-colors hover:bg-muted"
            >
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 font-semibold">{t.description}</p>
                <p className="shrink-0 text-right whitespace-nowrap">
                  <span className="text-xs text-muted-foreground">
                    {t.currency}
                  </span>{' '}
                  <span className="font-semibold tabular-nums">
                    {formatAmount(t.amount)}
                  </span>
                </p>
              </div>
              <p className="mt-1 text-sm text-muted-foreground">
                {t.from_node?.name ?? 'Unassigned'} →{' '}
                {t.to_node?.name ?? 'Unassigned'}
              </p>
              <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                <StatusMark status={t.verification_status} />
                <span className="tabular-nums">
                  {formatShortDate(t.transaction_date)}
                </span>
                <span className="tabular-nums">
                  {t.verification_count}{' '}
                  {t.verification_count === 1 ? 'proof' : 'proofs'}
                </span>
                {t.reference && (
                  <span className="font-mono">{t.reference}</span>
                )}
              </div>
            </Link>
          </li>
        ))}
      </ul>
      {total !== null && (
        <p className="flex items-baseline justify-between gap-3 border-t-2 border-foreground py-3 text-xs text-muted-foreground">
          <span>
            <TotalLabel count={rows.length} filtered={filtered} />
          </span>
          <span>
            {rows[0].currency}{' '}
            <span className="text-sm font-bold text-foreground tabular-nums">
              {formatAmount(total)}
            </span>
          </span>
        </p>
      )}
    </div>
  )
}

function LoadingRows() {
  return (
    <div aria-busy="true" aria-label="Loading transactions">
      {Array.from({ length: 6 }, (_, i) => (
        <div
          key={i}
          className="flex items-center gap-6 border-b border-border py-4"
        >
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
