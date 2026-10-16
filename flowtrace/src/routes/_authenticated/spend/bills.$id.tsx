import { Link, createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { ArrowLeft, Paperclip, Warning } from '@phosphor-icons/react'
import {
  CodedLines,
  CommandDialog,
  DocStatus,
  EntryLink,
  daysBetween,
  docTotal,
  personName,
} from './-shared'
import type { Bill, LedgerState } from '@/lib/ledger'
import { parseMoney } from '@/lib/ledger'
import { useLedger, useLedgerActor, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { DEV_MODE } from '@/lib/auth'
import { formatDate } from '@/lib/format'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Field } from '@/components/form'
import {
  AsOf,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ReportNotice,
  ReportSection,
} from '@/components/report'
import { AccountSelect, BooksPage, BooksState, Money, formatMinor } from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/spend/bills/$id')({
  component: BillPage,
})

function BillPage() {
  const { id } = Route.useParams()
  const { state, ledger, isLoading, error, refetch } = useLedger()
  const bill = state?.bills.find((b) => b.id === id)
  const vendor = bill ? state?.vendors.find((v) => v.id === bill.vendor_id) : undefined

  return (
    <BooksPage
      width="narrow"
      title={vendor?.name ?? 'Bill'}
      lede={
        bill ? (
          <>
            <DocStatus kind="bill" status={bill.status} className="mr-2 align-middle" />
            <span className="font-mono">{bill.number}</span> · dated {formatDate(bill.bill_date)} · due{' '}
            {formatDate(bill.due_date)}
          </>
        ) : undefined
      }
      nav={
        <Link
          to="/spend/bills"
          className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
          All bills
        </Link>
      }
    >
      {!state || !ledger ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="bill" />
      ) : !bill ? (
        <ReportNotice
          title="This bill is not in the books"
          action={
            <Link to="/spend/bills" className={buttonVariants({ variant: 'outline' })}>
              Back to bills
            </Link>
          }
        >
          The link may be wrong, or the bill was entered in another organisation.
        </ReportNotice>
      ) : (
        <BillDetail
          state={state}
          bill={bill}
          outstanding={bill.status === 'posted' || bill.status === 'paid' ? ledger.outstanding('bill', bill.id) : null}
        />
      )}
    </BooksPage>
  )
}

function BillDetail({
  state,
  bill,
  outstanding,
}: {
  state: LedgerState
  bill: Bill
  outstanding: number | null
}) {
  const actor = useLedgerActor()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const [dialog, setDialog] = useState<'approve' | 'post' | 'pay' | null>(null)
  const [done, setDone] = useState<string | null>(null)

  const [payDate, setPayDate] = useState(isoToDayMonthYear(asOf))
  const [cashAccount, setCashAccount] = useState('')
  const [amount, setAmount] = useState('')
  const [reference, setReference] = useState('')
  const [payErrors, setPayErrors] = useState<{ date?: string; cash?: string; amount?: string }>({})

  const total = docTotal(bill.lines)
  const base = state.settings.base_currency
  const overdueDays =
    bill.status === 'posted' && (outstanding ?? 0) > 0 ? Math.max(0, daysBetween(bill.due_date, asOf)) : 0
  const payments = state.payments
    .filter((p) => p.applies_to.kind === 'bill' && p.applies_to.id === bill.id)
    .sort((a, b) => a.date.localeCompare(b.date))
  const who = (id: string) => personName(id, { actor })

  const openPay = () => {
    setAmount(outstanding ? (outstanding / 100).toFixed(2) : '')
    setPayErrors({})
    setDialog('pay')
  }

  return (
    <>
      {done && (
        <p role="status" className="border-y border-border py-3 text-sm font-semibold">
          {done}
        </p>
      )}

      <ReportSection
        title="Bill"
        meta={
          <>
            <AsOf date={asOf} />
            {DEV_MODE && <DemoTag />}
          </>
        }
      >
        <KeyFigures>
          <KeyFigure scale="lg" value={formatMinor(total, { decimals: 0 })} label={`Billed, ${bill.currency}`} />
          <KeyFigure
            value={outstanding === null ? '–' : formatMinor(outstanding, { decimals: 0, zero: '0' })}
            label="Outstanding"
            detail={outstanding === null ? <span className="text-muted-foreground">Owed once posted</span> : undefined}
          />
          <KeyFigure
            value={formatMinor(total - (outstanding ?? total), { decimals: 0, zero: '0' })}
            label="Paid"
          />
          <KeyFigure
            value={overdueDays > 0 ? String(overdueDays) : String(Math.max(0, daysBetween(asOf, bill.due_date)))}
            label={overdueDays > 0 ? 'Days overdue' : 'Days until due'}
            detail={
              overdueDays > 0 ? (
                <span className="inline-flex items-center gap-1 font-semibold text-flag">
                  <Warning weight="bold" aria-hidden="true" className="size-3.5" />
                  Past the due date
                </span>
              ) : undefined
            }
          />
        </KeyFigures>
        {bill.currency !== base && (
          <p className="border-b border-border py-3 text-xs text-muted-foreground">
            Booked at {bill.fx_rate} {base} per {bill.currency}.
          </p>
        )}
      </ReportSection>

      <ReportSection title="What happens next">
        <div className="space-y-3 py-4 text-sm">
          {bill.status === 'draft' &&
            (actor?.id === bill.created_by ? (
              <p className="text-muted-foreground">
                You entered this bill, so someone else has to approve it. The
                person who records a bill never approves it.
              </p>
            ) : actor?.can_approve ? (
              <>
                <p>Check the bill against the delivery, then approve it. Approval commits the money against its grant and budget lines.</p>
                <Button size="lg" onClick={() => setDialog('approve')}>Approve bill</Button>
              </>
            ) : (
              <p className="text-muted-foreground">Waiting for a manager with approval rights.</p>
            ))}
          {bill.status === 'approved' && (
            <>
              <p>
                Approved{bill.approved_by ? ` by ${who(bill.approved_by)}` : ''}. Posting records the costs and the amount owed to the supplier in the books.
              </p>
              <Button size="lg" onClick={() => setDialog('post')}>Post to the books</Button>
            </>
          )}
          {bill.status === 'posted' && (
            <>
              <p>
                Posted. <Money value={outstanding ?? 0} currency={bill.currency} strong /> is still owed, due{' '}
                {formatDate(bill.due_date)}.
              </p>
              <Button size="lg" onClick={openPay}>Record a payment</Button>
            </>
          )}
          {bill.status === 'paid' && <p>Paid in full. Nothing more to do.</p>}
          {bill.status === 'void' && <p className="text-muted-foreground">This bill was voided.</p>}
        </div>
      </ReportSection>

      <ReportSection title="Coding">
        <CodedLines state={state} lines={bill.lines} currency={bill.currency} caption={`Lines of bill ${bill.number}`} />
      </ReportSection>

      <ReportSection title="Proof" meta={<span>{bill.proof_ids.length}</span>}>
        {bill.proof_ids.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">No proof recorded with this bill.</p>
        ) : (
          <ul className="divide-y divide-border">
            {bill.proof_ids.map((p, i) => (
              <li key={`${p}-${i}`} className="flex items-center gap-3 py-2.5 text-sm">
                <Paperclip aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0 truncate">{p}</span>
              </li>
            ))}
          </ul>
        )}
      </ReportSection>

      <ReportSection title="Payments" meta={<span>{payments.length}</span>}>
        {payments.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">No payments recorded yet.</p>
        ) : (
          <ul className="divide-y divide-border">
            {payments.map((p) => {
              const cash = state.accounts.find((a) => a.id === p.cash_account_id)
              return (
                <li key={p.id} className="flex flex-wrap items-start justify-between gap-2 py-3 text-sm">
                  <div className="min-w-0">
                    <p className="font-semibold">
                      <Money value={p.amount} currency={p.currency} /> on {formatDate(p.date)}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      From {cash ? `${cash.code} · ${cash.name}` : 'cash account'}
                      {p.reference && (
                        <>
                          {' '}
                          · <span className="font-mono">{p.reference}</span>
                        </>
                      )}
                    </p>
                  </div>
                  <EntryLink state={state} entryId={p.entry_id} />
                </li>
              )
            })}
          </ul>
        )}
      </ReportSection>

      <ReportSection title="In the books">
        {!bill.entry_id ? (
          <p className="py-4 text-sm text-muted-foreground">Not posted yet. A journal entry is made when the bill is posted.</p>
        ) : (
          <ul className="divide-y divide-border">
            <li className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
              <span>Bill posted: costs and the amount owed</span>
              <EntryLink state={state} entryId={bill.entry_id} />
            </li>
            {payments.map((p) => (
              <li key={p.id} className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                <span>Payment of {formatDate(p.date)}</span>
                <EntryLink state={state} entryId={p.entry_id} />
              </li>
            ))}
          </ul>
        )}
      </ReportSection>

      <CommandDialog
        open={dialog === 'approve'}
        onOpenChange={(o) => setDialog(o ? 'approve' : null)}
        title="Approve this bill"
        description={`${bill.number} · ${bill.currency} ${formatMinor(total)}`}
        confirmLabel="Approve bill"
        pendingLabel="Approving…"
        onConfirm={async () => {
          await command.mutateAsync({ type: 'bill.approve', billId: bill.id })
          setDone('Approved. Post it to record the amount owed.')
        }}
      >
        <p className="text-sm">The bill is committed against its grants and budget lines until it is posted.</p>
      </CommandDialog>

      <CommandDialog
        open={dialog === 'post'}
        onOpenChange={(o) => setDialog(o ? 'post' : null)}
        title="Post this bill"
        description={`Dated ${formatDate(bill.bill_date)}, so it posts to that month.`}
        confirmLabel="Post bill"
        pendingLabel="Posting…"
        allowOverride
        onConfirm={async (overrideReason) => {
          await command.mutateAsync({ type: 'bill.post', billId: bill.id, overrideReason })
          setDone('Posted to the books.')
        }}
      >
        <p className="text-sm">
          Each line is charged to its account, fund and grant, and the total is
          owed to the supplier until it is paid.
        </p>
      </CommandDialog>

      <CommandDialog
        open={dialog === 'pay'}
        onOpenChange={(o) => setDialog(o ? 'pay' : null)}
        title="Record a payment"
        description={`${bill.currency} ${formatMinor(outstanding ?? 0)} outstanding`}
        confirmLabel="Record payment"
        pendingLabel="Recording…"
        validate={() => {
          const e: { date?: string; cash?: string; amount?: string } = {}
          const d = parseDayMonthYear(payDate)
          if (!d) e.date = 'Enter the date the money left.'
          else if (d.iso === undefined) e.date = d.error
          if (!cashAccount) e.cash = 'Choose the account the money left from.'
          const m = parseMoney(amount)
          if (!amount.trim()) e.amount = 'Enter the amount paid.'
          else if (m === null) e.amount = 'Enter a number, like 39800 or 39,800.00.'
          else if (m <= 0) e.amount = 'The amount must be more than zero.'
          else if (m > (outstanding ?? 0)) e.amount = `Only ${formatMinor(outstanding ?? 0)} is outstanding.`
          setPayErrors(e)
          return !e.date && !e.cash && !e.amount
        }}
        onConfirm={async () => {
          await command.mutateAsync({
            type: 'bill.pay',
            billId: bill.id,
            date: parseDayMonthYear(payDate)?.iso ?? asOf,
            cashAccountId: cashAccount,
            amount: parseMoney(amount) ?? 0,
            reference: reference.trim() || null,
          })
          setReference('')
          setDone('Payment recorded.')
        }}
      >
        <Field id="pay-amount" label={`Amount, ${bill.currency}`} required error={payErrors.amount} hint="Defaults to everything still owed">
          {(p) => (
            <Input
              {...p}
              inputMode="decimal"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              autoComplete="off"
              className="h-10 text-right tabular-nums"
            />
          )}
        </Field>
        <Field id="pay-date" label="Date paid" required error={payErrors.date} hint="Day, month, year">
          {(p) => (
            <DateField {...p} value={payDate} onValueChange={setPayDate} pickerLabel="Choose the date from a calendar" className="h-10" />
          )}
        </Field>
        <Field id="pay-account" label="Paid from" required error={payErrors.cash} hint="Bank, M-Pesa or petty cash">
          {(p) => <AccountSelect {...p} state={state} cashOnly value={cashAccount} onValueChange={setCashAccount} />}
        </Field>
        <Field id="pay-ref" label="Reference" hint="EFT, cheque or M-Pesa code">
          {(p) => (
            <Input {...p} value={reference} onChange={(e) => setReference(e.target.value)} autoComplete="off" className="h-10 font-mono" />
          )}
        </Field>
      </CommandDialog>
    </>
  )
}
