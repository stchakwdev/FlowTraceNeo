import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo } from 'react'
import { Plus } from '@phosphor-icons/react'
import { BILL_STATUS_LABEL, DocStatus, OverdueFlag, daysBetween, docTotal } from './-shared'
import type { Bill, BillStatus, Ledger, LedgerState } from '@/lib/ledger'
import { useLedger, useLedgerAsOf } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate, formatShortDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { buttonVariants } from '@/components/ui/button'
import { AsOf, DemoTag, ReportNotice, ReportSection } from '@/components/report'
import { BooksPage, BooksState, Money, SubNav } from '@/components/ledger'
import { SPEND_NAV } from '@/components/ledger/spend-nav'

const FILTERS: Array<{ value: BillStatus | undefined; label: string }> = [
  { value: undefined, label: 'All' },
  { value: 'draft', label: 'Draft' },
  { value: 'approved', label: 'Approved' },
  { value: 'posted', label: 'To pay' },
  { value: 'paid', label: 'Paid' },
  { value: 'void', label: 'Void' },
]

const STATUSES = new Set<string>(['draft', 'approved', 'posted', 'paid', 'void'])

export const Route = createFileRoute('/_authenticated/spend/bills/')({
  validateSearch: (s: Record<string, unknown>): { status?: BillStatus } => ({
    status: typeof s.status === 'string' && STATUSES.has(s.status) ? (s.status as BillStatus) : undefined,
  }),
  component: BillsPage,
})

interface Row {
  bill: Bill
  vendor: string
  total: number
  outstanding: number | null
  overdueDays: number
}

function rowsFor(state: LedgerState, ledger: Ledger, asOf: string): Array<Row> {
  return state.bills.map((bill) => {
    const owed = bill.status === 'posted' || bill.status === 'paid' ? ledger.outstanding('bill', bill.id) : null
    const late = bill.status === 'posted' && (owed ?? 0) > 0 ? daysBetween(bill.due_date, asOf) : 0
    return {
      bill,
      vendor: state.vendors.find((v) => v.id === bill.vendor_id)?.name ?? 'Vendor',
      total: docTotal(bill.lines),
      outstanding: owed,
      overdueDays: Math.max(0, late),
    }
  })
}

function BillsPage() {
  const { status } = Route.useSearch()
  const { state, ledger, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()

  const all = useMemo(() => (state && ledger ? rowsFor(state, ledger, asOf) : []), [state, ledger, asOf])
  const counts = useMemo(() => {
    const c: Record<string, number> = { all: all.length }
    for (const r of all) c[r.bill.status] = (c[r.bill.status] ?? 0) + 1
    return c
  }, [all])
  const rows = useMemo(
    () =>
      all
        .filter((r) => !status || r.bill.status === status)
        .sort((a, b) => b.overdueDays - a.overdueDays || b.bill.bill_date.localeCompare(a.bill.bill_date)),
    [all, status],
  )

  return (
    <BooksPage
      title="Bills"
      lede="What suppliers have invoiced: approved, posted to the books as money owed, then paid."
      nav={<SubNav label="Spend" items={SPEND_NAV} />}
      actions={
        <Link to="/spend/bills/new" className={buttonVariants({ size: 'lg' })}>
          <Plus />
          New bill
        </Link>
      }
    >
      {!state ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="bills" />
      ) : (
        <ReportSection
          title="Bills"
          meta={
            <>
              <AsOf date={asOf} />
              {DEV_MODE && <DemoTag />}
            </>
          }
        >
          <nav aria-label="Filter by bill status" className="border-b border-border py-3">
            <ul className="flex flex-wrap gap-px border border-border bg-border">
              {FILTERS.map((f) => {
                const active = status === f.value
                return (
                  <li key={f.label} className="grow bg-background sm:grow-0">
                    <Link
                      to="/spend/bills"
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

          {all.length === 0 ? (
            <ReportNotice
              title="No bills yet"
              action={
                <Link to="/spend/bills/new" className={buttonVariants()}>
                  <Plus />
                  New bill
                </Link>
              }
            >
              Record a supplier’s invoice here to approve it, post it and pay it.
            </ReportNotice>
          ) : rows.length === 0 ? (
            <ReportNotice
              title="No bills match"
              action={
                <Link to="/spend/bills" search={{}} className={buttonVariants({ variant: 'outline' })}>
                  Show all bills
                </Link>
              }
            >
              No bills are {status ? BILL_STATUS_LABEL[status].toLowerCase() : 'here'}.
            </ReportNotice>
          ) : (
            <>
              <BillTable rows={rows} />
              <BillStack rows={rows} />
            </>
          )}
        </ReportSection>
      )}
    </BooksPage>
  )
}

function BillTable({ rows }: { rows: Array<Row> }) {
  const navigate = useNavigate()
  return (
    <div className="hidden overflow-x-auto md:block">
      <table className="w-full min-w-[58rem] text-sm">
        <caption className="sr-only">Bills with vendor, dates, amount, amount outstanding and status</caption>
        <thead>
          <tr className="border-b border-border text-left text-xs text-muted-foreground">
            <th scope="col" className="py-2.5 pr-4 font-semibold">Bill</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Dated</th>
            <th scope="col" className="py-2.5 pr-4 font-semibold">Due</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Amount</th>
            <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Outstanding</th>
            <th scope="col" className="py-2.5 font-semibold">Status</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ bill, vendor, total, outstanding, overdueDays }) => (
            <tr
              key={bill.id}
              onClick={(e) => {
                if ((e.target as HTMLElement).closest('a')) return
                navigate({ to: '/spend/bills/$id', params: { id: bill.id } })
              }}
              className="cursor-pointer border-b border-border align-top transition-colors hover:bg-muted"
            >
              <td className="py-3 pr-4">
                <Link
                  to="/spend/bills/$id"
                  params={{ id: bill.id }}
                  className="font-semibold underline-offset-4 hover:underline focus-visible:underline"
                >
                  {vendor}
                </Link>
                <p className="mt-0.5 font-mono text-xs text-muted-foreground">{bill.number}</p>
              </td>
              <td className="py-3 pr-4 whitespace-nowrap text-muted-foreground">{formatDate(bill.bill_date)}</td>
              <td className="py-3 pr-4 whitespace-nowrap">
                {formatDate(bill.due_date)}
                {overdueDays > 0 && <OverdueFlag days={overdueDays} className="mt-0.5 flex" />}
              </td>
              <td className="py-3 pr-4 text-right">
                <Money value={total} currency={bill.currency} />
              </td>
              <td className="py-3 pr-4 text-right">
                {outstanding === null ? (
                  <span className="text-xs text-muted-foreground">Not posted</span>
                ) : (
                  <Money value={outstanding} strong />
                )}
              </td>
              <td className="py-3">
                <DocStatus kind="bill" status={bill.status} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function BillStack({ rows }: { rows: Array<Row> }) {
  return (
    <ul className="divide-y divide-border md:hidden">
      {rows.map(({ bill, vendor, total, outstanding, overdueDays }) => (
        <li key={bill.id}>
          <Link
            to="/spend/bills/$id"
            params={{ id: bill.id }}
            className="-mx-4 block px-4 py-3.5 transition-colors hover:bg-muted"
          >
            <div className="flex items-start justify-between gap-3">
              <p className="min-w-0 font-semibold">{vendor}</p>
              <Money value={total} currency={bill.currency} strong className="shrink-0" />
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              <span className="font-mono">{bill.number}</span> · due {formatShortDate(bill.due_date)}
              {outstanding !== null && outstanding > 0 && (
                <>
                  {' '}
                  · <Money value={outstanding} /> outstanding
                </>
              )}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <DocStatus kind="bill" status={bill.status} />
              {overdueDays > 0 && <OverdueFlag days={overdueDays} />}
            </div>
          </Link>
        </li>
      ))}
    </ul>
  )
}
