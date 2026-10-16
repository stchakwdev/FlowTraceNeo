import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { CloudSlash, Plus } from '@phosphor-icons/react'
import { CLAIM_STATUS_LABEL, DocStatus, docTotal } from './-shared'
import type { ClaimStatus, ExpenseClaim, LedgerState } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { useOfflineClaimSync } from '@/lib/offline-claims'
import { formatDate, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { AsOf, DemoTag, ReportNotice, ReportSection } from '@/components/report'
import { BooksPage, BooksState, Money, SubNav } from '@/components/ledger'
import { SPEND_NAV } from '@/components/ledger/spend-nav'

const FILTERS: Array<{ value: ClaimStatus | undefined; label: string }> = [
  { value: undefined, label: 'All' },
  { value: 'draft', label: 'Draft' },
  { value: 'submitted', label: 'Submitted' },
  { value: 'posted', label: 'To reimburse' },
  { value: 'reimbursed', label: 'Reimbursed' },
  { value: 'rejected', label: 'Rejected' },
]

const STATUSES = new Set<string>(['draft', 'submitted', 'approved', 'posted', 'reimbursed', 'rejected'])

export const Route = createFileRoute('/_authenticated/spend/claims/')({
  validateSearch: (s: Record<string, unknown>): { status?: ClaimStatus } => ({
    status: typeof s.status === 'string' && STATUSES.has(s.status) ? (s.status as ClaimStatus) : undefined,
  }),
  component: ClaimsPage,
})

function ClaimsPage() {
  const { status } = Route.useSearch()
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const sync = useOfflineClaimSync()

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: 0 }
    for (const cl of state?.claims ?? []) {
      c.all++
      c[cl.status] = (c[cl.status] ?? 0) + 1
    }
    return c
  }, [state])

  const rows = useMemo(
    () =>
      (state?.claims ?? [])
        .filter((c) => !status || c.status === status)
        .sort((a, b) => b.claim_date.localeCompare(a.claim_date) || b.number.localeCompare(a.number)),
    [state, status],
  )

  return (
    <BooksPage
      title="Expense claims"
      lede="Money staff spent from their own pockets: submitted, approved by someone else, posted to the books and paid back."
      nav={<SubNav label="Spend" items={SPEND_NAV} />}
      actions={
        <Link to="/spend/claims/new" className={buttonVariants({ size: 'lg' })}>
          <Plus />
          New expense claim
        </Link>
      }
    >
      {sync.queued.length > 0 && (
        <ReportSection
          title="Waiting on this phone"
          meta={<span>{sync.syncing ? 'Sending…' : `${sync.queued.length} not sent yet`}</span>}
          actions={
            <Button variant="outline" size="sm" onClick={() => sync.flush()} disabled={sync.syncing}>
              Send now
            </Button>
          }
        >
          <p className="flex items-start gap-2 py-3 text-sm text-muted-foreground">
            <CloudSlash aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-foreground" />
            These were captured without a connection. They are sent on their
            own when this phone is back online.
          </p>
          <ul className="divide-y divide-border border-t border-border">
            {sync.queued.map((q) => (
              <li key={q.key} className="flex flex-wrap items-start justify-between gap-3 py-3">
                <div className="min-w-0">
                  <p className="font-semibold">{q.input.purpose}</p>
                  <p className="text-xs text-muted-foreground">
                    {formatDate(q.input.claim_date)} · {q.input.lines.length}{' '}
                    {q.input.lines.length === 1 ? 'line' : 'lines'} · {q.input.submit ? 'to submit' : 'draft'}
                  </p>
                  {q.last_error && (
                    <p className="mt-1 text-xs font-semibold text-destructive">Last try: {q.last_error}</p>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <Money value={docTotal(q.input.lines)} currency={q.input.currency} strong />
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-10"
                    onClick={() => sync.discard(q.key)}
                    aria-label={`Discard the claim for ${q.input.purpose}`}
                  >
                    Discard
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        </ReportSection>
      )}
      {sync.lastResult && sync.lastResult.sent.length > 0 && (
        <p role="status" className="border-y border-border py-3 text-sm">
          {sync.lastResult.sent.length} {sync.lastResult.sent.length === 1 ? 'claim' : 'claims'} sent from this phone.
        </p>
      )}

      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="claims" />
      ) : (
        <ReportSection
          title="Claims"
          meta={
            <>
              <AsOf date={asOf} />
              {DEMO_BOOKS && <DemoTag />}
            </>
          }
        >
          <nav aria-label="Filter by claim status" className="border-b border-border py-3">
            <ul className="flex flex-wrap gap-px border border-border bg-border">
              {FILTERS.map((f) => {
                const active = status === f.value
                return (
                  <li key={f.label} className="grow bg-background sm:grow-0">
                    <Link
                      to="/spend/claims"
                      search={{ status: f.value }}
                      replace
                      aria-current={active ? 'page' : undefined}
                      className={cn(
                        'flex h-10 items-center justify-center gap-2 px-3 text-sm whitespace-nowrap transition-colors',
                        active
                          ? 'bg-primary font-semibold text-primary-foreground'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground',
                      )}
                    >
                      {f.label}
                      <span className="tabular-nums opacity-80">{counts[f.value ?? 'all'] ?? 0}</span>
                    </Link>
                  </li>
                )
              })}
            </ul>
          </nav>

          {counts.all === 0 ? (
            <ReportNotice
              title="No claims yet"
              action={
                <Link to="/spend/claims/new" className={buttonVariants()}>
                  <Plus />
                  New expense claim
                </Link>
              }
            >
              When staff pay for work costs themselves, they claim the money
              back here with the receipts.
            </ReportNotice>
          ) : rows.length === 0 ? (
            <ReportNotice
              title="No claims match"
              action={
                <Link to="/spend/claims" search={{}} className={buttonVariants({ variant: 'outline' })}>
                  Show all claims
                </Link>
              }
            >
              No claims are {status ? CLAIM_STATUS_LABEL[status].toLowerCase() : 'here'}.
            </ReportNotice>
          ) : (
            <>
              <ClaimTable state={state} rows={rows} />
              <ClaimStack state={state} rows={rows} />
            </>
          )}
        </ReportSection>
      )}
    </BooksPage>
  )
}

function grantNames(state: LedgerState, c: ExpenseClaim) {
  const ids = [...new Set(c.lines.map((l) => l.grant_id).filter(Boolean))]
  if (ids.length === 0) return 'No grant'
  return ids.map((id) => state.grants.find((g) => g.id === id)?.name ?? 'Grant').join(', ')
}

function OfflineMark() {
  return (
    <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
      <CloudSlash aria-hidden="true" className="size-3.5" />
      Captured offline
    </span>
  )
}

function ClaimTable({ state, rows }: { state: LedgerState; rows: Array<ExpenseClaim> }) {
  const navigate = useNavigate()
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[56rem] text-sm">
        <caption className="sr-only">Expense claims with claimant, purpose, grant, amount and status</caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">Date</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Claim</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Claimant</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Grant</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Amount</th>
            <th scope="col" className="py-2.5 font-semibold">Status</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((c) => (
            <tr
              key={c.id}
              onClick={(e) => {
                if ((e.target as HTMLElement).closest('a')) return
                navigate({ to: '/spend/claims/$id', params: { id: c.id } })
              }}
              className="cursor-pointer border-b border-border align-top transition-colors hover:bg-muted"
            >
              <td className="py-3 pr-4 whitespace-nowrap text-muted-foreground">
                <time dateTime={c.claim_date}>{formatDate(c.claim_date)}</time>
              </td>
              <td className="max-w-[22rem] py-3 pr-4">
                <Link
                  to="/spend/claims/$id"
                  params={{ id: c.id }}
                  className="font-semibold underline-offset-4 hover:underline focus-visible:underline"
                >
                  {c.purpose}
                </Link>
                <p className="mt-0.5 flex flex-wrap items-center gap-x-3 text-xs text-muted-foreground">
                  <span className="font-mono">{c.number}</span>
                  {c.captured_offline && <OfflineMark />}
                </p>
              </td>
              <td className="py-3 pr-4">{c.claimant_name}</td>
              <td className="py-3 pr-4 text-muted-foreground">{grantNames(state, c)}</td>
              <td className="py-3 pr-4 text-right">
                <Money value={docTotal(c.lines)} currency={c.currency} strong />
              </td>
              <td className="py-3">
                <DocStatus kind="claim" status={c.status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function ClaimStack({ state, rows }: { state: LedgerState; rows: Array<ExpenseClaim> }) {
  return (
    <ul className="divide-y divide-border md:hidden">
      {rows.map((c) => (
        <li key={c.id}>
          <Link
            to="/spend/claims/$id"
            params={{ id: c.id }}
            className="-mx-4 block px-4 py-3.5 transition-colors hover:bg-muted"
          >
            <div className="flex items-start justify-between gap-3">
              <p className="min-w-0 font-semibold">{c.purpose}</p>
              <Money value={docTotal(c.lines)} currency={c.currency} strong className="shrink-0" />
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {c.claimant_name} · {grantNames(state, c)}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <DocStatus kind="claim" status={c.status} />
              <span>{formatShortDate(c.claim_date)}</span>
              <span className="font-mono">{c.number}</span>
              {c.captured_offline && <OfflineMark />}
            </div>
          </Link>
        </li>
      ))}
    </ul>
  )
}
