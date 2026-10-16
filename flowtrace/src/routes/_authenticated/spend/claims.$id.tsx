import { Link, createFileRoute } from '@tanstack/react-router'
import { useState } from 'react'
import { ArrowLeft, CloudSlash, Paperclip } from '@phosphor-icons/react'
import {
  CodedLines,
  CommandDialog,
  DocStatus,
  EntryLink,
  docTotal,
  personName,
} from './-shared'
import type { ApprovalStep, ExpenseClaim, LedgerState } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerActor, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { formatDate, formatDateTime } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
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

export const Route = createFileRoute('/_authenticated/spend/claims/$id')({
  component: ClaimPage,
})

const STEP_LABEL: Record<ApprovalStep['action'], string> = {
  submitted: 'Submitted for approval',
  approved: 'Approved',
  rejected: 'Rejected',
  posted: 'Posted to the books',
  reimbursed: 'Paid back',
}

function ClaimPage() {
  const { id } = Route.useParams()
  const { state, ledger, isLoading, error, refetch } = useLedger()
  const claim = state?.claims.find((c) => c.id === id)

  return (
    <BooksPage
      width="narrow"
      title={claim ? claim.purpose : 'Expense claim'}
      lede={
        claim ? (
          <>
            <DocStatus kind="claim" status={claim.status} className="mr-2 align-middle" />
            <span className="font-mono">{claim.number}</span> · {claim.claimant_name} · spent{' '}
            {formatDate(claim.claim_date)}
          </>
        ) : undefined
      }
      nav={
        <Link
          to="/spend/claims"
          className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
          All claims
        </Link>
      }
    >
      {!state || !ledger ? (
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="claim" />
      ) : !claim ? (
        <ReportNotice
          title="This claim is not in the books"
          action={
            <Link to="/spend/claims" className={buttonVariants({ variant: 'outline' })}>
              Back to claims
            </Link>
          }
        >
          It may have been captured on another phone and not sent yet, or the
          link is wrong.
        </ReportNotice>
      ) : (
        <ClaimDetail state={state} claim={claim} outstanding={claim.status === 'posted' ? ledger.outstanding('expense_claim', claim.id) : 0} />
      )}
    </BooksPage>
  )
}

function ClaimDetail({
  state,
  claim,
  outstanding,
}: {
  state: LedgerState
  claim: ExpenseClaim
  outstanding: number
}) {
  const actor = useLedgerActor()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const [dialog, setDialog] = useState<'approve' | 'reject' | 'reimburse' | null>(null)
  const [done, setDone] = useState<string | null>(null)
  const [submitError, setSubmitError] = useState<string | null>(null)

  // Approve
  const [note, setNote] = useState('')
  // Reject
  const [rejectNote, setRejectNote] = useState('')
  const [rejectError, setRejectError] = useState<string | undefined>()
  // Reimburse
  const [payDate, setPayDate] = useState(isoToDayMonthYear(asOf))
  const [cashAccount, setCashAccount] = useState('')
  const [reference, setReference] = useState('')
  const [payErrors, setPayErrors] = useState<{ date?: string; cash?: string }>({})

  const total = docTotal(claim.lines)
  const isClaimant = !!actor && actor.id === claim.claimant
  const canApprove = !!actor?.can_approve
  const who = (id: string) =>
    personName(id, { actor, claimant: { id: claim.claimant, name: claim.claimant_name } })
  const payments = state.payments.filter(
    (p) => p.applies_to.kind === 'expense_claim' && p.applies_to.id === claim.id,
  )

  const submit = async () => {
    setSubmitError(null)
    try {
      await command.mutateAsync({ type: 'claim.submit', claimId: claim.id })
      setDone('Submitted. A manager can now approve it.')
    } catch (e) {
      setSubmitError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <>
      {done && (
        <p role="status" className="border-y border-border py-3 text-sm font-semibold">
          {done}
        </p>
      )}

      <ReportSection
        title="Claim"
        meta={
          <>
            <AsOf date={asOf} />
            {DEMO_BOOKS && <DemoTag />}
          </>
        }
      >
        <KeyFigures>
          <KeyFigure scale="lg" value={formatMinor(total, { decimals: 0 })} label={`Claimed, ${claim.currency}`} />
          <KeyFigure value={String(claim.lines.length)} label={claim.lines.length === 1 ? 'Line' : 'Lines'} />
          <KeyFigure
            value={formatMinor(claim.status === 'posted' ? outstanding : 0, { decimals: 0 })}
            label="Still to pay back"
          />
          <KeyFigure value={String(claim.receipt_names.length)} label={claim.receipt_names.length === 1 ? 'Receipt' : 'Receipts'} />
        </KeyFigures>
        {claim.captured_offline && (
          <p className="flex items-center gap-2 border-b border-border py-3 text-sm text-muted-foreground">
            <CloudSlash aria-hidden="true" className="size-4 text-foreground" />
            Captured offline on a phone and sent when it reconnected.
          </p>
        )}
      </ReportSection>

      <ReportSection title="What happens next">
        <div className="space-y-3 py-4 text-sm">
          {claim.status === 'draft' &&
            (isClaimant ? (
              <>
                <p>This is a draft. Submit it when the lines and receipts are complete.</p>
                <Button size="lg" onClick={submit} disabled={command.isPending}>
                  {command.isPending ? 'Submitting…' : 'Submit for approval'}
                </Button>
                {submitError && (
                  <p role="alert" className="text-sm font-semibold text-destructive">{submitError}</p>
                )}
              </>
            ) : (
              <p className="text-muted-foreground">
                This is {claim.claimant_name}’s draft. Only they can submit it.
              </p>
            ))}

          {claim.status === 'submitted' &&
            (isClaimant ? (
              <p>
                You made this claim, so someone else has to approve it. The
                books do not let anyone approve their own spending.
              </p>
            ) : !canApprove ? (
              <p className="text-muted-foreground">Waiting for a manager with approval rights.</p>
            ) : (
              <>
                <p>
                  Approving posts the claim to the books straight away: the
                  costs land on their grants and budget lines, and the money is
                  owed to {claim.claimant_name}.
                </p>
                <div className="flex flex-col gap-2 sm:flex-row">
                  <Button size="lg" onClick={() => setDialog('approve')}>Approve and post</Button>
                  <Button size="lg" variant="outline" onClick={() => setDialog('reject')}>Reject</Button>
                </div>
              </>
            ))}

          {claim.status === 'posted' && (
            <>
              <p>
                Approved and posted. <Money value={outstanding} currency={claim.currency} strong /> is owed to{' '}
                {claim.claimant_name}.
              </p>
              <Button size="lg" onClick={() => setDialog('reimburse')}>Record the reimbursement</Button>
            </>
          )}
          {claim.status === 'reimbursed' && <p>Paid back in full. Nothing more to do.</p>}
          {claim.status === 'rejected' && (
            <p>Rejected. The reason is in the approval trail below; the claimant can make a new claim.</p>
          )}
          {claim.status === 'approved' && <p>Approved, waiting to be posted.</p>}
        </div>
      </ReportSection>

      <ReportSection title="Lines">
        <CodedLines state={state} lines={claim.lines} currency={claim.currency} caption={`Lines of claim ${claim.number}`} />
      </ReportSection>

      <ReportSection title="Receipts" meta={<span>{claim.receipt_names.length}</span>}>
        {claim.receipt_names.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">No receipts recorded with this claim.</p>
        ) : (
          <ul className="divide-y divide-border">
            {claim.receipt_names.map((r, i) => (
              <li key={`${r}-${i}`} className="flex items-center gap-3 py-2.5 text-sm">
                <Paperclip aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0 truncate">{r}</span>
              </li>
            ))}
          </ul>
        )}
      </ReportSection>

      <ReportSection title="Approval trail">
        {claim.approvals.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">Nothing yet. The trail starts when the claim is submitted.</p>
        ) : (
          <ol className="py-2">
            {claim.approvals.map((s, i) => (
              <li key={i} className="relative grid grid-cols-[1.25rem_1fr] gap-3 pb-4 last:pb-0">
                <span aria-hidden="true" className="relative flex justify-center">
                  <span
                    className={cn(
                      'mt-1 size-3.5 border-[3px]',
                      s.action === 'rejected' ? 'border-dotted border-destructive' : 'border-foreground',
                      s.action === 'reimbursed' && 'bg-foreground',
                    )}
                  />
                  {i < claim.approvals.length - 1 && (
                    <span className="absolute top-5 bottom-[-0.25rem] w-px bg-border" />
                  )}
                </span>
                <div className="min-w-0 text-sm">
                  <p className={cn('font-semibold', s.action === 'rejected' && 'text-destructive')}>
                    {STEP_LABEL[s.action]}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {who(s.by)} · <time dateTime={s.at}>{formatDateTime(s.at)}</time>
                  </p>
                  {s.note && s.action !== 'posted' && s.action !== 'reimbursed' && (
                    <p className="mt-1">“{s.note}”</p>
                  )}
                  {s.action === 'reimbursed' && s.note && (
                    <p className="mt-1 font-mono text-xs">Reference {s.note}</p>
                  )}
                </div>
              </li>
            ))}
          </ol>
        )}
      </ReportSection>

      <ReportSection title="In the books">
        {!claim.entry_id && payments.length === 0 ? (
          <p className="py-4 text-sm text-muted-foreground">
            Not posted yet. A journal entry is made when the claim is approved.
          </p>
        ) : (
          <ul className="divide-y divide-border">
            {claim.entry_id && (
              <li className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                <span>Claim posted: costs and the amount owed</span>
                <EntryLink state={state} entryId={claim.entry_id} />
              </li>
            )}
            {payments.map((p) => (
              <li key={p.id} className="flex flex-wrap items-center justify-between gap-2 py-3 text-sm">
                <span>
                  Paid back {formatDate(p.date)} · <Money value={p.amount} currency={p.currency} />
                  {p.reference && <span className="ml-1 font-mono text-xs text-muted-foreground">{p.reference}</span>}
                </span>
                <EntryLink state={state} entryId={p.entry_id} />
              </li>
            ))}
          </ul>
        )}
      </ReportSection>

      <CommandDialog
        open={dialog === 'approve'}
        onOpenChange={(o) => setDialog(o ? 'approve' : null)}
        title="Approve this claim"
        description={`${claim.number} · ${claim.claimant_name} · ${claim.currency} ${formatMinor(total)}`}
        confirmLabel="Approve and post"
        pendingLabel="Posting…"
        allowOverride
        onConfirm={async (overrideReason) => {
          await command.mutateAsync({
            type: 'claim.approve',
            claimId: claim.id,
            note: note.trim() || undefined,
            overrideReason,
          })
          setNote('')
          setDone('Approved and posted to the books.')
        }}
      >
        <Field id="approve-note" label="Note to the claimant" hint="For example, which receipts you checked">
          {(p) => <Textarea {...p} rows={3} value={note} onChange={(e) => setNote(e.target.value)} />}
        </Field>
      </CommandDialog>

      <CommandDialog
        open={dialog === 'reject'}
        onOpenChange={(o) => {
          setDialog(o ? 'reject' : null)
          setRejectError(undefined)
        }}
        title="Reject this claim"
        description="The claimant sees your reason. Nothing is posted to the books."
        confirmLabel="Reject claim"
        pendingLabel="Rejecting…"
        destructive
        validate={() => {
          if (rejectNote.trim()) return true
          setRejectError('Tell the claimant why, so they can put it right.')
          return false
        }}
        onConfirm={async () => {
          await command.mutateAsync({ type: 'claim.reject', claimId: claim.id, note: rejectNote.trim() })
          setRejectNote('')
          setDone('Rejected. The claimant can see your reason.')
        }}
      >
        <Field id="reject-note" label="Reason" required error={rejectError}>
          {(p) => (
            <Textarea
              {...p}
              rows={3}
              value={rejectNote}
              onChange={(e) => {
                setRejectNote(e.target.value)
                if (rejectError) setRejectError(undefined)
              }}
            />
          )}
        </Field>
      </CommandDialog>

      <CommandDialog
        open={dialog === 'reimburse'}
        onOpenChange={(o) => {
          setDialog(o ? 'reimburse' : null)
          setPayErrors({})
        }}
        title="Record the reimbursement"
        description={`Pays ${claim.claimant_name} the full ${claim.currency} ${formatMinor(outstanding)} still owed.`}
        confirmLabel="Record payment"
        pendingLabel="Recording…"
        validate={() => {
          const e: { date?: string; cash?: string } = {}
          const d = parseDayMonthYear(payDate)
          if (!d) e.date = 'Enter the date the money was sent.'
          else if (d.iso === undefined) e.date = d.error
          if (!cashAccount) e.cash = 'Choose the account the money left from.'
          setPayErrors(e)
          return !e.date && !e.cash
        }}
        onConfirm={async () => {
          await command.mutateAsync({
            type: 'claim.reimburse',
            claimId: claim.id,
            date: parseDayMonthYear(payDate)?.iso ?? asOf,
            cashAccountId: cashAccount,
            reference: reference.trim() || null,
          })
          setReference('')
          setDone('Reimbursement recorded.')
        }}
      >
        <Field id="pay-date" label="Date paid" required error={payErrors.date} hint="Day, month, year">
          {(p) => (
            <DateField {...p} value={payDate} onValueChange={setPayDate} pickerLabel="Choose the date from a calendar" className="h-10" />
          )}
        </Field>
        <Field id="pay-account" label="Paid from" required error={payErrors.cash} hint="Bank, M-Pesa or petty cash">
          {(p) => <AccountSelect {...p} state={state} cashOnly value={cashAccount} onValueChange={setCashAccount} />}
        </Field>
        <Field id="pay-ref" label="Reference" hint="M-Pesa code or bank reference, e.g. RAN4K2PQ7Z">
          {(p) => (
            <Input {...p} value={reference} onChange={(e) => setReference(e.target.value)} autoComplete="off" className="h-10 font-mono" />
          )}
        </Field>
      </CommandDialog>
    </>
  )
}
