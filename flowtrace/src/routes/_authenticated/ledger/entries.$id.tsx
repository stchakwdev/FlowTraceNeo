import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMemo, useState } from 'react'
import { ArrowLeft, ArrowRight, PencilSimple, Trash } from '@phosphor-icons/react'
import {
  AUDIT_ACTION_LABEL,
  EntryStatusMark,
  SOURCE_LABEL,
  actorName,
  entryTotal,
} from './-shared'
import type { JournalEntry, LedgerState } from '@/lib/ledger'
import { DEMO_BOOKS, useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { formatDate, formatDateTime } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Field } from '@/components/form'
import { ReportDialog } from '@/components/report-dialog'
import {
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
  hasOverridable,
} from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/ledger/entries/$id')({
  component: EntryPage,
})

function EntryPage() {
  const { id } = Route.useParams()
  const { state, ledger, isLoading, error, refetch } = useLedger()

  const back = (
    <Link
      to="/ledger"
      className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeft className="size-4" />
      Journal
    </Link>
  )

  if (!state || !ledger)
    return (
      <BooksPage title="Journal entry" nav={back}>
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="entry" />
      </BooksPage>
    )

  const entry = ledger.entry(id)
  if (!entry)
    return (
      <BooksPage title="Journal entry" nav={back}>
        <ReportNotice
          title="This entry is not in the books"
          action={
            <Link to="/ledger" className={buttonVariants({ variant: 'outline' })}>
              Back to the journal
            </Link>
          }
        >
          It may have been a draft that was deleted, or the link is wrong.
        </ReportNotice>
      </BooksPage>
    )

  return <EntryDetail state={state} entry={entry} back={back} />
}

function EntryDetail({
  state,
  entry,
  back,
}: {
  state: LedgerState
  entry: JournalEntry
  back: React.ReactNode
}) {
  const navigate = useNavigate()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const [actionError, setActionError] = useState<unknown>(null)
  const [overrideOpen, setOverrideOpen] = useState(false)
  const [overrideReason, setOverrideReason] = useState('')
  const [overrideError, setOverrideError] = useState<string>()
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [reverseOpen, setReverseOpen] = useState(false)

  const lines = useMemo(() => state.lines.filter((l) => l.entry_id === entry.id), [state, entry.id])
  const base = state.settings.base_currency
  const total = entryTotal(lines)
  const debits = lines.reduce((t, l) => t + Math.max(0, l.base_amount), 0)
  const credits = lines.reduce((t, l) => t + Math.max(0, -l.base_amount), 0)
  const period = state.periods.find((p) => p.id === entry.period_id)
  const reversalOf = entry.reversal_of ? state.entries.find((e) => e.id === entry.reversal_of) : undefined
  const reversedBy = entry.reversed_by ? state.entries.find((e) => e.id === entry.reversed_by) : undefined
  const foreign = lines.some((l) => l.currency !== base)

  const audit = useMemo(
    () =>
      state.audit
        .filter((a) => a.subject === entry.id)
        .sort((a, b) => a.at.localeCompare(b.at)),
    [state.audit, entry.id],
  )

  const accountName = (id: string) => {
    const a = state.accounts.find((x) => x.id === id)
    return a ? `${a.code} · ${a.name}` : 'Unknown account'
  }
  const fundName = (id: string) => state.funds.find((f) => f.id === id)?.name ?? 'Unknown fund'
  const grantName = (id: string | null) => (id ? state.grants.find((g) => g.id === id)?.name ?? 'Unknown grant' : null)
  const dimNames = (ids: Array<string>) =>
    ids.map((d) => state.dimensions.find((x) => x.id === d)?.name).filter(Boolean).join(', ')

  const post = async (reason?: string) => {
    setActionError(null)
    try {
      await command.mutateAsync({ type: 'entry.postDraft', entryId: entry.id, overrideReason: reason })
      setOverrideOpen(false)
      setOverrideReason('')
    } catch (e) {
      setActionError(e)
      if (hasOverridable(e)) setOverrideOpen(true)
    }
  }

  const postWithOverride = () => {
    if (!overrideReason.trim()) {
      setOverrideError('Say why this entry should post despite the rule.')
      return
    }
    setOverrideError(undefined)
    void post(overrideReason.trim())
  }

  const remove = async () => {
    setActionError(null)
    try {
      await command.mutateAsync({ type: 'entry.deleteDraft', entryId: entry.id })
      setDeleteOpen(false)
      navigate({ to: '/ledger' })
    } catch (e) {
      setDeleteOpen(false)
      setActionError(e)
    }
  }

  return (
    <BooksPage
      title={<span className="font-mono">{entry.number}</span>}
      lede={entry.memo}
      nav={back}
      actions={
        entry.status === 'draft' ? (
          <>
            <Link
              to="/ledger/new"
              search={{ draft: entry.id }}
              className={buttonVariants({ variant: 'outline' })}
            >
              <PencilSimple />
              Edit
            </Link>
            <Button variant="destructive" onClick={() => setDeleteOpen(true)}>
              <Trash />
              Delete
            </Button>
            <Button onClick={() => post()} disabled={command.isPending}>
              {command.isPending ? 'Posting…' : 'Post'}
            </Button>
          </>
        ) : entry.status === 'posted' ? (
          <Button variant="outline" onClick={() => setReverseOpen(true)}>
            Reverse
          </Button>
        ) : null
      }
    >
      {!!actionError && (
        <IssueSummary error={actionError} title={entry.status === 'draft' ? 'Not posted' : 'Not done'} />
      )}

      {overrideOpen && entry.status === 'draft' && (
        <ReportSection title="Post with an override">
          <div className="space-y-4 pt-5">
            <p className="max-w-prose text-sm text-muted-foreground">
              The rules above can be set aside when there is a reason a funder
              or auditor would accept. The reason is stored on the entry and in
              the audit trail.
            </p>
            <Field id="override-reason" label="Override reason" required error={overrideError}>
              {(props) => (
                <Textarea
                  {...props}
                  rows={3}
                  value={overrideReason}
                  onChange={(e) => setOverrideReason(e.target.value)}
                />
              )}
            </Field>
            <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
              <Button variant="ghost" onClick={() => setOverrideOpen(false)}>
                Cancel
              </Button>
              <Button onClick={postWithOverride} disabled={command.isPending}>
                {command.isPending ? 'Posting…' : 'Post with override'}
              </Button>
            </div>
          </div>
        </ReportSection>
      )}

      <ReportSection
        title="Entry"
        meta={
          <>
            <EntryStatusMark status={entry.status} />
            {DEMO_BOOKS && <DemoTag />}
          </>
        }
      >
        <KeyFigures>
          <KeyFigure scale="lg" value={<Money value={total} decimals={0} />} label={`Total, ${base}`} />
          <KeyFigure value={formatDate(entry.date)} label={period ? `Date · ${period.name}` : 'Date'} scale="sm" />
          <KeyFigure value={lines.length} label="Lines" scale="sm" />
          <KeyFigure
            value={SOURCE_LABEL[entry.source_type]}
            label="Source"
            scale="sm"
            detail={<SourceLink entry={entry} state={state} />}
          />
        </KeyFigures>
        <dl className="grid gap-x-6 gap-y-3 border-b border-border py-4 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-xs text-muted-foreground">Created</dt>
            <dd>
              {formatDateTime(entry.created_at)} by {actorName(entry.created_by)}
            </dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground">Posted</dt>
            <dd>
              {entry.posted_at && entry.posted_by
                ? `${formatDateTime(entry.posted_at)} by ${actorName(entry.posted_by)}`
                : 'Not posted. Drafts do not count in any balance or report.'}
            </dd>
          </div>
          {entry.override_reason && entry.source_type !== 'reversal' && entry.source_type !== 'year_end_close' && (
            <div className="sm:col-span-2">
              <dt className="text-xs text-muted-foreground">Override reason</dt>
              <dd className="font-semibold">{entry.override_reason}</dd>
            </div>
          )}
          {reversalOf && (
            <div className="sm:col-span-2">
              <dt className="text-xs text-muted-foreground">Reverses</dt>
              <dd>
                <Link
                  to="/ledger/entries/$id"
                  params={{ id: reversalOf.id }}
                  className="inline-flex min-h-10 items-center gap-1.5 font-semibold underline underline-offset-4"
                >
                  <span className="font-mono">{reversalOf.number}</span> {reversalOf.memo}
                </Link>
              </dd>
            </div>
          )}
          {reversedBy && (
            <div className="sm:col-span-2">
              <dt className="text-xs text-muted-foreground">Reversed by</dt>
              <dd>
                <Link
                  to="/ledger/entries/$id"
                  params={{ id: reversedBy.id }}
                  className="inline-flex min-h-10 items-center gap-1.5 font-semibold underline underline-offset-4"
                >
                  <span className="font-mono">{reversedBy.number}</span> on {formatDate(reversedBy.date)}
                </Link>
              </dd>
            </div>
          )}
        </dl>
      </ReportSection>

      <ReportSection
        title="Lines"
        meta={
          <Link
            to="/ledger/general"
            search={{ source: entry.id }}
            className="inline-flex min-h-10 items-center gap-1 underline-offset-4 hover:text-foreground hover:underline"
          >
            In the general ledger
            <ArrowRight aria-hidden="true" className="size-3.5" />
          </Link>
        }
      >
        <div className="hidden overflow-x-auto md:block">
          <table className="w-full min-w-[56rem] text-sm">
            <caption className="sr-only">
              Lines of {entry.number} with account, fund, grant, tags, debit, credit and base amount
            </caption>
            <thead>
              <tr className="border-b border-border text-left text-xs text-muted-foreground">
                <th scope="col" className="py-2.5 pr-4 font-semibold">Account</th>
                <th scope="col" className="py-2.5 pr-4 font-semibold">Fund and grant</th>
                <th scope="col" className="py-2.5 pr-4 font-semibold">Tags</th>
                <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Debit</th>
                <th scope="col" className="py-2.5 pr-4 text-right font-semibold">Credit</th>
                <th scope="col" className="py-2.5 text-right font-semibold">Base, {base}</th>
              </tr>
            </thead>
            <tbody>
              {lines.map((l) => (
                <tr key={l.id} className="border-b border-border align-top">
                  <td className="py-3 pr-4">
                    <p className="font-semibold">{accountName(l.account_id)}</p>
                    {l.memo && <p className="mt-0.5 text-xs text-muted-foreground">{l.memo}</p>}
                  </td>
                  <td className="py-3 pr-4">
                    <p>{fundName(l.fund_id)}</p>
                    <p className="text-xs text-muted-foreground">{grantName(l.grant_id) ?? 'No grant'}</p>
                  </td>
                  <td className="py-3 pr-4 text-muted-foreground">{dimNames(l.dimension_ids) || '–'}</td>
                  <td className="py-3 pr-4 text-right">
                    <Money value={l.debit} currency={foreign ? l.currency : undefined} />
                  </td>
                  <td className="py-3 pr-4 text-right">
                    <Money value={l.credit} currency={foreign ? l.currency : undefined} />
                  </td>
                  <td className="py-3 text-right">
                    <Money value={l.base_amount} signed />
                    {l.currency !== base && (
                      <p className="text-xs text-muted-foreground">at {l.fx_rate}</p>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr className="border-t-2 border-foreground">
                <td colSpan={3} className="py-3 pr-4 text-xs text-muted-foreground">
                  Totals in {base}
                </td>
                <td className="py-3 pr-4 text-right"><Money value={debits} strong /></td>
                <td className="py-3 pr-4 text-right"><Money value={credits} strong /></td>
                <td className="py-3 text-right"><Money value={debits - credits} signed zero="0.00" strong /></td>
              </tr>
            </tfoot>
          </table>
        </div>
        <ul className="divide-y divide-border md:hidden">
          {lines.map((l) => (
            <li key={l.id} className="py-3.5">
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 font-semibold">{accountName(l.account_id)}</p>
                <p className="shrink-0 text-right text-sm">
                  <span className="text-xs text-muted-foreground">{l.debit ? 'Dr' : 'Cr'} </span>
                  <Money value={l.debit || l.credit} strong currency={foreign ? l.currency : undefined} />
                </p>
              </div>
              <p className="mt-1 text-sm text-muted-foreground">
                {fundName(l.fund_id)} · {grantName(l.grant_id) ?? 'No grant'}
                {l.dimension_ids.length > 0 && ` · ${dimNames(l.dimension_ids)}`}
              </p>
              {l.memo && <p className="mt-1 text-xs text-muted-foreground">{l.memo}</p>}
            </li>
          ))}
          <li className="flex justify-between gap-3 border-t-2 border-foreground py-3 text-xs text-muted-foreground">
            <span>Debits <Money value={debits} strong className="text-sm text-foreground" /></span>
            <span>Credits <Money value={credits} strong className="text-sm text-foreground" /></span>
          </li>
        </ul>
      </ReportSection>

      <ReportSection title="Audit trail" meta={<span>{audit.length} events</span>}>
        {audit.length === 0 ? (
          <ReportNotice title="No recorded events">
            This entry was created before the audit trail began.
          </ReportNotice>
        ) : (
          <ol className="divide-y divide-border">
            {audit.map((a) => (
              <li key={a.id} className="grid gap-1 py-3 text-sm sm:grid-cols-[12rem_minmax(0,1fr)] sm:gap-4">
                <time dateTime={a.at} className="text-muted-foreground tabular-nums">
                  {formatDateTime(a.at)}
                </time>
                <div className="min-w-0">
                  <p className="font-semibold">
                    {AUDIT_ACTION_LABEL[a.action] ?? a.action}
                    <span className="font-normal text-muted-foreground"> by {actorName(a.by)}</span>
                  </p>
                  {a.detail && a.action === 'entry.reverse' && (
                    <p className="text-muted-foreground">Reason: {a.detail}</p>
                  )}
                </div>
              </li>
            ))}
          </ol>
        )}
      </ReportSection>

      <ReportDialog
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        title={`Delete ${entry.number}?`}
        description="The draft and its lines are removed. Nothing posted is affected."
        footer={
          <>
            <Button variant="ghost" onClick={() => setDeleteOpen(false)}>
              Keep the draft
            </Button>
            <Button variant="destructive" onClick={remove} disabled={command.isPending}>
              {command.isPending ? 'Deleting…' : 'Delete draft'}
            </Button>
          </>
        }
      >
        <p className="text-sm">{entry.memo}</p>
      </ReportDialog>

      <ReverseDialog
        open={reverseOpen}
        onOpenChange={setReverseOpen}
        entry={entry}
        defaultDate={asOf}
        onDone={(revId) => {
          setReverseOpen(false)
          if (revId) navigate({ to: '/ledger/entries/$id', params: { id: revId } })
        }}
      />
    </BooksPage>
  )
}

function SourceLink({ entry, state }: { entry: JournalEntry; state: LedgerState }) {
  const cls = 'inline-flex min-h-10 items-center gap-1 font-semibold underline underline-offset-4'
  if (!entry.source_id) return null
  switch (entry.source_type) {
    case 'transaction':
      return (
        <Link to="/transactions/$id" params={{ id: entry.source_id }} className={cls}>
          Open the transaction
        </Link>
      )
    case 'bill': {
      const bill = state.bills.find((b) => b.id === entry.source_id)
      return (
        <Link to="/spend/bills/$id" params={{ id: entry.source_id }} className={cls}>
          Open bill {bill?.number ?? ''}
        </Link>
      )
    }
    case 'expense_claim': {
      const claim = state.claims.find((c) => c.id === entry.source_id)
      return (
        <Link to="/spend/claims/$id" params={{ id: entry.source_id }} className={cls}>
          Open claim {claim?.number ?? ''}
        </Link>
      )
    }
    case 'payment': {
      const p = state.payments.find((x) => x.entry_id === entry.id)
      if (!p) return null
      return (
        <Link to={p.applies_to.kind === 'bill' ? '/spend/bills/$id' : '/spend/claims/$id'} params={{ id: p.applies_to.id }} className={cls}>
          Open the {p.applies_to.kind === 'bill' ? 'bill' : 'claim'}
        </Link>
      )
    }
    case 'reimbursement':
      return (
        <Link to="/spend/claims/$id" params={{ id: entry.source_id }} className={cls}>
          Open the claim
        </Link>
      )
    default:
      return null
  }
}

function ReverseDialog({
  open,
  onOpenChange,
  entry,
  defaultDate,
  onDone,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  entry: JournalEntry
  defaultDate: string
  onDone: (reversalId: string | undefined) => void
}) {
  const command = useLedgerCommand()
  const [date, setDate] = useState(isoToDayMonthYear(defaultDate))
  const [reason, setReason] = useState('')
  const [errors, setErrors] = useState<{ date?: string; reason?: string }>({})
  const [failure, setFailure] = useState<unknown>(null)

  const submit = async () => {
    const parsed = parseDayMonthYear(date)
    const e: typeof errors = {}
    if (!parsed) e.date = 'Enter the date of the reversal.'
    else if (parsed.error) e.date = parsed.error
    if (!reason.trim()) e.reason = 'Say why the entry is being reversed.'
    setErrors(e)
    if (Object.keys(e).length || !parsed?.iso) return
    setFailure(null)
    try {
      const r = await command.mutateAsync({ type: 'entry.reverse', entryId: entry.id, date: parsed.iso, reason: reason.trim() })
      setReason('')
      onDone(r.type === 'id' ? r.id : undefined)
    } catch (err) {
      setFailure(err)
    }
  }

  return (
    <ReportDialog
      open={open}
      onOpenChange={onOpenChange}
      title={`Reverse ${entry.number}`}
      description="A mirror-image entry is posted on the date you choose. The original stays in the books, marked reversed."
      footer={
        <>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={submit} disabled={command.isPending}>
            {command.isPending ? 'Reversing…' : 'Post reversal'}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <IssueSummary error={failure} title="Not reversed" />
        <Field id="reverse-date" label="Reversal date" required error={errors.date} hint="Day, month, year. Must fall in an open period.">
          {(props) => (
            <DateField
              {...props}
              value={date}
              onValueChange={setDate}
              pickerLabel="Choose the reversal date from a calendar"
              className="h-10"
              wrapperClassName={cn('sm:max-w-56')}
            />
          )}
        </Field>
        <Field id="reverse-reason" label="Reason" required error={errors.reason}>
          {(props) => (
            <Textarea {...props} rows={3} value={reason} onChange={(e) => setReason(e.target.value)} />
          )}
        </Field>
      </div>
    </ReportDialog>
  )
}
