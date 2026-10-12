import { Link, createFileRoute } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { ArrowRight, CloudSlash, Plus } from '@phosphor-icons/react'
import { DocStatus, OverdueFlag, daysBetween, docTotal } from './-shared'
import type { Bill, ExpenseClaim, LedgerCommand } from '@/lib/ledger'
import { useLedger, useLedgerActor, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { useOfflineClaimSync } from '@/lib/offline-claims'
import { DEV_MODE } from '@/lib/auth'
import { formatShortDate } from '@/lib/format'
import { Button, buttonVariants } from '@/components/ui/button'
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
  IssueSummary,
  Money,
  SubNav,
  formatMinorCompact,
  hasOverridable,
} from '@/components/ledger'
import { SPEND_NAV } from '@/components/ledger/spend-nav'

export const Route = createFileRoute('/_authenticated/spend/')({
  component: SpendOverview,
})

type Waiting =
  | { kind: 'claim'; doc: ExpenseClaim; total: number }
  | { kind: 'bill'; doc: Bill; total: number; vendor: string }

interface ToPay {
  kind: 'claim' | 'bill'
  id: string
  title: string
  who: string
  owed: number
  currency: string
  due: string | null
  overdueDays: number
}

function SpendOverview() {
  const { state, ledger, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const sync = useOfflineClaimSync()

  const data = useMemo(() => {
    if (!state || !ledger) return null
    const vendorName = (id: string) => state.vendors.find((v) => v.id === id)?.name ?? 'Supplier'
    const waiting: Array<Waiting> = [
      ...state.claims
        .filter((c) => c.status === 'submitted')
        .map((c) => ({ kind: 'claim' as const, doc: c, total: docTotal(c.lines) })),
      ...state.bills
        .filter((b) => b.status === 'draft' || b.status === 'approved')
        .map((b) => ({ kind: 'bill' as const, doc: b, total: docTotal(b.lines), vendor: vendorName(b.vendor_id) })),
    ]
    const toPay: Array<ToPay> = [
      ...state.claims
        .filter((c) => c.status === 'posted')
        .map((c) => ({
          kind: 'claim' as const,
          id: c.id,
          title: c.purpose,
          who: c.claimant_name,
          owed: ledger.outstanding('expense_claim', c.id),
          currency: c.currency,
          due: null,
          overdueDays: 0,
        })),
      ...state.bills
        .filter((b) => b.status === 'posted')
        .map((b) => {
          const owed = ledger.outstanding('bill', b.id)
          return {
            kind: 'bill' as const,
            id: b.id,
            title: b.number,
            who: vendorName(b.vendor_id),
            owed,
            currency: b.currency,
            due: b.due_date,
            overdueDays: owed > 0 ? Math.max(0, daysBetween(b.due_date, asOf)) : 0,
          }
        }),
    ].sort((a, b) => b.overdueDays - a.overdueDays || (a.due ?? '').localeCompare(b.due ?? ''))
    const base = state.settings.base_currency
    // Headline figures are in base currency; foreign documents are converted at their booked rate.
    const inBase = (amount: number, currency: string, rate: number) =>
      currency === base ? amount : Math.round(amount * rate)
    const rateOf = (kind: 'claim' | 'bill', id: string) =>
      (kind === 'claim' ? state.claims.find((c) => c.id === id)?.fx_rate : state.bills.find((b) => b.id === id)?.fx_rate) ?? 1
    return {
      waiting,
      toPay,
      waitingTotal: waiting.reduce((t, w) => t + inBase(w.total, w.doc.currency, w.doc.fx_rate), 0),
      toPayTotal: toPay.reduce((t, p) => t + inBase(p.owed, p.currency, rateOf(p.kind, p.id)), 0),
      overdueTotal: toPay
        .filter((p) => p.overdueDays > 0)
        .reduce((t, p) => t + inBase(p.owed, p.currency, rateOf(p.kind, p.id)), 0),
      overdueCount: toPay.filter((p) => p.overdueDays > 0).length,
      dueSoon: toPay.filter((p) => p.due && p.overdueDays === 0 && daysBetween(asOf, p.due) <= 14).length,
      base,
    }
  }, [state, ledger, asOf])

  return (
    <BooksPage
      title="Spend"
      lede="Staff expense claims and supplier bills: what is waiting for a decision, and what is owed and when."
      nav={<SubNav label="Spend" items={SPEND_NAV} />}
      actions={
        <>
          <Link to="/spend/bills/new" className={buttonVariants({ variant: 'outline', size: 'lg' })}>
            New bill
          </Link>
          <Link to="/spend/claims/new" className={buttonVariants({ size: 'lg' })}>
            <Plus />
            New expense claim
          </Link>
        </>
      }
    >
      {sync.queued.length > 0 && (
        <p className="flex items-center gap-2 border-y border-border py-3 text-sm">
          <CloudSlash aria-hidden="true" className="size-4 shrink-0" />
          <span>
            {sync.queued.length} {sync.queued.length === 1 ? 'claim is' : 'claims are'} waiting on this phone to be sent.{' '}
            <Link to="/spend/claims" className="font-semibold underline underline-offset-4">
              See them
            </Link>
          </span>
        </p>
      )}

      {!state || !data ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="spend" />
      ) : (
        <>
          <ReportSection
            title="Owed and waiting"
            meta={
              <>
                <AsOf date={asOf} />
                {DEV_MODE && <DemoTag />}
              </>
            }
          >
            <KeyFigures>
              <KeyFigure scale="lg" value={formatMinorCompact(data.toPayTotal)} label={`To pay, ${data.base}`} detail={<span className="text-muted-foreground">{data.toPay.length} {data.toPay.length === 1 ? 'claim or bill' : 'claims and bills'}</span>} />
              <KeyFigure value={formatMinorCompact(data.waitingTotal)} label="Waiting for a decision" detail={<span className="text-muted-foreground">{data.waiting.length} to approve or post</span>} />
              <KeyFigure
                value={formatMinorCompact(data.overdueTotal)}
                label="Overdue"
                detail={
                  data.overdueCount > 0 ? (
                    <span className="font-semibold text-flag">
                      {data.overdueCount} past the due date
                    </span>
                  ) : (
                    <span className="text-muted-foreground">Nothing past due</span>
                  )
                }
              />
              <KeyFigure value={String(data.dueSoon)} label="Due in the next 14 days" />
            </KeyFigures>
          </ReportSection>

          <WaitingSection items={data.waiting} />

          <ReportSection title="To pay" meta={<span>{data.toPay.length}</span>}>
            {data.toPay.length === 0 ? (
              <ReportNotice title="Nothing is owed">
                Every posted claim and bill has been paid.
              </ReportNotice>
            ) : (
              <ul className="divide-y divide-border">
                {data.toPay.map((p) => (
                  <li key={`${p.kind}-${p.id}`} className="flex flex-col gap-3 py-3.5 sm:flex-row sm:items-center sm:justify-between">
                    <div className="min-w-0">
                      <p className="font-semibold">
                        {p.who}
                        <span className="ml-2 text-xs font-normal text-muted-foreground">
                          {p.kind === 'claim' ? 'Expense claim' : 'Bill'}
                        </span>
                      </p>
                      <p className="text-sm text-muted-foreground">
                        <span className={p.kind === 'bill' ? 'font-mono' : undefined}>{p.title}</span>
                        {p.due && <> · due {formatShortDate(p.due)}</>}
                        {!p.due && ' · owed to the claimant'}
                      </p>
                      {p.overdueDays > 0 && <OverdueFlag days={p.overdueDays} className="mt-1" />}
                    </div>
                    <div className="flex items-center justify-between gap-4 sm:justify-end">
                      <Money value={p.owed} currency={p.currency} strong />
                      {p.kind === 'claim' ? (
                        <Link to="/spend/claims/$id" params={{ id: p.id }} className={buttonVariants({ variant: 'outline' })}>
                          Reimburse
                        </Link>
                      ) : (
                        <Link to="/spend/bills/$id" params={{ id: p.id }} className={buttonVariants({ variant: 'outline' })}>
                          Pay
                        </Link>
                      )}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </ReportSection>
        </>
      )}
    </BooksPage>
  )
}

function WaitingSection({ items }: { items: Array<Waiting> }) {
  const actor = useLedgerActor()
  const command = useLedgerCommand()
  const [busy, setBusy] = useState<string | null>(null)
  const [errors, setErrors] = useState<Record<string, unknown>>({})
  const [done, setDone] = useState<string | null>(null)

  const run = async (key: string, cmd: LedgerCommand, message: string) => {
    setBusy(key)
    setDone(null)
    setErrors((e) => ({ ...e, [key]: undefined }))
    try {
      await command.mutateAsync(cmd)
      setDone(message)
    } catch (e) {
      setErrors((prev) => ({ ...prev, [key]: e }))
    } finally {
      setBusy(null)
    }
  }

  return (
    <ReportSection title="Waiting for approval" meta={<span>{items.length}</span>}>
      {done && (
        <p role="status" className="border-b border-border py-3 text-sm font-semibold">
          {done}
        </p>
      )}
      {items.length === 0 ? (
        <ReportNotice title="Nothing is waiting">
          Submitted claims and bills that need approving or posting appear here.
        </ReportNotice>
      ) : (
        <ul className="divide-y divide-border">
          {items.map((w) => {
            const key = `${w.kind}-${w.doc.id}`
            const err = errors[key]
            if (w.kind === 'claim') {
              const c = w.doc
              const mine = actor?.id === c.claimant
              return (
                <li key={key} className="py-3.5">
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div className="min-w-0">
                      <Link
                        to="/spend/claims/$id"
                        params={{ id: c.id }}
                        className="group inline-flex items-center gap-2 font-semibold hover:underline"
                      >
                        {c.purpose}
                        <ArrowRight aria-hidden="true" className="size-3.5 text-muted-foreground" />
                      </Link>
                      <p className="text-sm text-muted-foreground">
                        Expense claim · {c.claimant_name} · {formatShortDate(c.claim_date)}
                      </p>
                      <DocStatus kind="claim" status={c.status} className="mt-1.5" />
                    </div>
                    <div className="flex items-center justify-between gap-4 sm:justify-end">
                      <Money value={w.total} currency={c.currency} strong />
                      {mine ? (
                        <span className="max-w-48 text-xs text-muted-foreground">
                          Your own claim: someone else approves it
                        </span>
                      ) : actor?.can_approve ? (
                        <Button
                          disabled={busy !== null}
                          onClick={() => run(key, { type: 'claim.approve', claimId: c.id }, `Approved and posted ${c.number}.`)}
                        >
                          {busy === key ? 'Posting…' : 'Approve'}
                        </Button>
                      ) : null}
                    </div>
                  </div>
                  {!!err && <QuickError error={err} href={{ kind: 'claim', id: c.id }} />}
                </li>
              )
            }
            const b = w.doc
            const action =
              b.status === 'draft'
                ? actor?.can_approve
                  ? { label: 'Approve', cmd: { type: 'bill.approve', billId: b.id } as LedgerCommand, msg: `Approved bill ${b.number}.` }
                  : null
                : { label: 'Post', cmd: { type: 'bill.post', billId: b.id } as LedgerCommand, msg: `Posted bill ${b.number}.` }
            return (
              <li key={key} className="py-3.5">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <div className="min-w-0">
                    <Link
                      to="/spend/bills/$id"
                      params={{ id: b.id }}
                      className="group inline-flex items-center gap-2 font-semibold hover:underline"
                    >
                      {w.vendor}
                      <ArrowRight aria-hidden="true" className="size-3.5 text-muted-foreground" />
                    </Link>
                    <p className="text-sm text-muted-foreground">
                      Bill <span className="font-mono">{b.number}</span> · due {formatShortDate(b.due_date)}
                    </p>
                    <p className="mt-1.5 flex items-center gap-2 text-xs text-muted-foreground">
                      <DocStatus kind="bill" status={b.status} />
                      {b.status === 'approved' ? 'Approved, not yet posted' : 'Needs approval'}
                    </p>
                  </div>
                  <div className="flex items-center justify-between gap-4 sm:justify-end">
                    <Money value={w.total} currency={b.currency} strong />
                    {action && (
                      <Button disabled={busy !== null} onClick={() => run(key, action.cmd, action.msg)}>
                        {busy === key ? 'Saving…' : action.label}
                      </Button>
                    )}
                  </div>
                </div>
                {!!err && <QuickError error={err} href={{ kind: 'bill', id: b.id }} />}
              </li>
            )
          })}
        </ul>
      )}
    </ReportSection>
  )
}

/** A quick action's failure, with the way through when it can be overridden. */
function QuickError({ error, href }: { error: unknown; href: { kind: 'claim' | 'bill'; id: string } }) {
  return (
    <div className="mt-3">
      <IssueSummary error={error} title="Not done" />
      {hasOverridable(error) && (
        <p className="mt-2 text-sm">
          {href.kind === 'claim' ? (
            <Link to="/spend/claims/$id" params={{ id: href.id }} className="font-semibold underline underline-offset-4">
              Open the claim to override with a reason
            </Link>
          ) : (
            <Link to="/spend/bills/$id" params={{ id: href.id }} className="font-semibold underline underline-offset-4">
              Open the bill to override with a reason
            </Link>
          )}
        </p>
      )}
    </div>
  )
}
