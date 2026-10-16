import { Link } from '@tanstack/react-router'
// Pieces shared by the Spend screens: status words, coded-line display and
// editing, links to journal entries, and the dialog that runs one ledger
// command with the override-with-a-reason flow. The leading dash keeps the
// router from treating this file as a route.
import { useEffect, useId, useMemo, useState } from 'react'
import { Plus, Trash, Warning } from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import type {
  AccountType,
  Actor,
  BillStatus,
  ClaimStatus,
  CodedLine,
  ISODate,
  LedgerState,
  Minor,
} from '@/lib/ledger'
import { UNRESTRICTED_FUND_ID, parseMoney, sum } from '@/lib/ledger'
import { cn } from '@/lib/utils'
import { formatDate } from '@/lib/format'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Field, FormSelect } from '@/components/form'
import { ReportDialog } from '@/components/report-dialog'
import {
  AccountSelect,
  DimensionSelect,
  FundSelect,
  GrantSelect,
  IssueSummary,
  Money,
  hasOverridable,
} from '@/components/ledger'

// ---------------------------------------------------------------------------
// Status words. The word carries the state; the frame only confirms it.
// ---------------------------------------------------------------------------

export const CLAIM_STATUS_LABEL: Record<ClaimStatus, string> = {
  draft: 'Draft',
  submitted: 'Submitted',
  approved: 'Approved',
  posted: 'Posted, to reimburse',
  reimbursed: 'Reimbursed',
  rejected: 'Rejected',
}

export const BILL_STATUS_LABEL: Record<BillStatus, string> = {
  draft: 'Draft',
  approved: 'Approved',
  posted: 'Posted, to pay',
  paid: 'Paid',
  void: 'Void',
}

type Tone = 'quiet' | 'open' | 'booked' | 'done' | 'rejected'

const CLAIM_TONE: Record<ClaimStatus, Tone> = {
  draft: 'quiet',
  submitted: 'open',
  approved: 'open',
  posted: 'booked',
  reimbursed: 'done',
  rejected: 'rejected',
}

const BILL_TONE: Record<BillStatus, Tone> = {
  draft: 'quiet',
  approved: 'open',
  posted: 'booked',
  paid: 'done',
  void: 'rejected',
}

/** A square status tag: dashed for drafts, dotted for rejected, filled when settled. */
export function DocStatus({
  kind,
  status,
  className,
}: {
  kind: 'claim' | 'bill'
  status: ClaimStatus | BillStatus
  className?: string
}) {
  const tone =
    kind === 'claim' ? CLAIM_TONE[status as ClaimStatus] : BILL_TONE[status as BillStatus]
  const label =
    kind === 'claim'
      ? CLAIM_STATUS_LABEL[status as ClaimStatus]
      : BILL_STATUS_LABEL[status as BillStatus]
  return (
    <span
      className={cn(
        'inline-flex h-5 items-center px-1.5 text-[0.6875rem] font-bold tracking-[0.08em] whitespace-nowrap uppercase',
        tone === 'quiet' && 'border border-dashed border-muted-foreground text-muted-foreground',
        tone === 'open' && 'border border-foreground text-foreground',
        tone === 'booked' && 'border border-foreground bg-foreground text-background',
        tone === 'done' && 'border border-primary bg-primary text-primary-foreground',
        tone === 'rejected' && 'border-2 border-dotted border-destructive text-destructive',
        className,
      )}
    >
      {label}
    </span>
  )
}

/** A flag for money past its due date. Orange is for flags only. */
export function OverdueFlag({ days, className }: { days: number; className?: string }) {
  return (
    <span className={cn('inline-flex items-center gap-1 text-xs font-semibold text-flag', className)}>
      <Warning weight="bold" aria-hidden="true" className="size-3.5" />
      Overdue {days} {days === 1 ? 'day' : 'days'}
    </span>
  )
}

export function daysBetween(a: ISODate, b: ISODate) {
  const da = Date.parse(`${a}T00:00:00Z`)
  const db = Date.parse(`${b}T00:00:00Z`)
  return Math.round((db - da) / 86_400_000)
}

export const docTotal = (lines: Array<Pick<CodedLine, 'amount'>>) => sum(lines.map((l) => l.amount))

// ---------------------------------------------------------------------------
// People and entries
// ---------------------------------------------------------------------------

/** A readable name for an actor id seen in approvals and audit trails. */
export function personName(
  id: string,
  known: { claimant?: { id: string; name: string }; actor?: Actor | null },
) {
  if (known.actor && id === known.actor.id) return `${known.actor.name} (you)`
  if (known.claimant && id === known.claimant.id) return known.claimant.name
  if (id === 'system') return 'FlowTrace'
  return id
}

/**
 * Link to a journal entry. The entry screen is a sibling section, so this
 * is a plain link rather than a typed route.
 */
export function EntryLink({
  state,
  entryId,
  className,
}: {
  state: LedgerState
  entryId: string
  className?: string
}) {
  const entry = state.entries.find((e) => e.id === entryId)
  return (
    <Link
      to="/ledger/entries/$id"
      params={{ id: entryId }}
      className={cn(
        'font-mono text-sm font-medium underline underline-offset-4 hover:text-primary',
        className,
      )}
    >
      {entry?.number ?? 'Journal entry'}
    </Link>
  )
}

// ---------------------------------------------------------------------------
// Coded lines: read-only display
// ---------------------------------------------------------------------------

export function CodedLines({
  state,
  lines,
  currency,
  caption,
}: {
  state: LedgerState
  lines: Array<CodedLine>
  currency: string
  caption: string
}) {
  const account = (id: string) => state.accounts.find((a) => a.id === id)
  const fund = (id: string) => state.funds.find((f) => f.id === id)
  const grant = (id: string | null) => (id ? state.grants.find((g) => g.id === id) : null)
  const dims = (ids: Array<string>) =>
    ids.map((id) => state.dimensions.find((d) => d.id === id)?.name ?? id).join(', ')
  const total = docTotal(lines)

  return (
    <>
      <div className="hidden overflow-x-auto md:block">
        <table className="w-full min-w-[48rem] text-sm">
          <caption className="sr-only">{caption}</caption>
          <thead>
            <tr className="border-b border-border text-left text-xs text-muted-foreground">
              <th scope="col" className="py-2.5 pr-4 font-semibold">Description</th>
              <th scope="col" className="py-2.5 pr-4 font-semibold">Account</th>
              <th scope="col" className="py-2.5 pr-4 font-semibold">Fund and grant</th>
              <th scope="col" className="py-2.5 pr-4 font-semibold">Project, place</th>
              <th scope="col" className="py-2.5 text-right font-semibold">Amount</th>
            </tr>
          </thead>
          <tbody>
            {lines.map((l) => {
              const a = account(l.account_id)
              return (
                <tr key={l.id} className="border-b border-border align-top">
                  <td className="py-3 pr-4 font-medium">{l.description || 'No description'}</td>
                  <td className="py-3 pr-4">
                    {a ? (
                      <>
                        <span className="font-mono text-xs text-muted-foreground">{a.code}</span> {a.name}
                      </>
                    ) : (
                      'Unknown account'
                    )}
                  </td>
                  <td className="py-3 pr-4">
                    {fund(l.fund_id)?.name ?? 'Fund'}
                    <span className="block text-xs text-muted-foreground">
                      {grant(l.grant_id)?.name ?? 'No grant'}
                    </span>
                  </td>
                  <td className="py-3 pr-4 text-muted-foreground">{dims(l.dimension_ids) || '–'}</td>
                  <td className="py-3 text-right"><Money value={l.amount} strong /></td>
                </tr>
              )
            })}
          </tbody>
          <tfoot>
            <tr className="border-t-2 border-foreground">
              <td colSpan={4} className="py-3 pr-4 text-xs text-muted-foreground">
                Total, {lines.length} {lines.length === 1 ? 'line' : 'lines'}
              </td>
              <td className="py-3 text-right font-bold"><Money value={total} currency={currency} /></td>
            </tr>
          </tfoot>
        </table>
      </div>
      <div className="md:hidden">
        <ul className="divide-y divide-border">
          {lines.map((l) => {
            const a = account(l.account_id)
            return (
              <li key={l.id} className="py-3">
                <div className="flex items-start justify-between gap-3">
                  <p className="min-w-0 font-semibold">{l.description || 'No description'}</p>
                  <Money value={l.amount} strong className="shrink-0" />
                </div>
                <p className="mt-1 text-sm text-muted-foreground">
                  {a ? `${a.code} · ${a.name}` : 'Unknown account'}
                </p>
                <p className="text-xs text-muted-foreground">
                  {fund(l.fund_id)?.name ?? 'Fund'} · {grant(l.grant_id)?.name ?? 'No grant'}
                  {l.dimension_ids.length > 0 && ` · ${dims(l.dimension_ids)}`}
                </p>
              </li>
            )
          })}
        </ul>
        <p className="flex items-baseline justify-between gap-3 border-t-2 border-foreground py-3 text-xs text-muted-foreground">
          <span>Total, {lines.length} {lines.length === 1 ? 'line' : 'lines'}</span>
          <Money value={total} currency={currency} strong className="text-sm text-foreground" />
        </p>
      </div>
    </>
  )
}

// ---------------------------------------------------------------------------
// Coded lines: editing
// ---------------------------------------------------------------------------

export interface LineDraft {
  key: string
  description: string
  amount: string
  account_id: string
  fund_id: string
  grant_id: string
  /** Claims: project and location. Bills: one dimension in `dimension`. */
  project: string
  location: string
  dimension: string
}

let lineSeq = 0
export function newLine(state: LedgerState | null, from?: Partial<LineDraft>): LineDraft {
  const fund =
    from?.fund_id ??
    (state?.funds.some((f) => f.id === UNRESTRICTED_FUND_ID)
      ? UNRESTRICTED_FUND_ID
      : (state?.funds[0]?.id ?? ''))
  return {
    key: `l${++lineSeq}`,
    description: '',
    amount: '',
    account_id: '',
    fund_id: fund,
    grant_id: '',
    project: '',
    location: '',
    dimension: '',
    ...from,
  }
}

export type LineField = 'description' | 'amount' | 'account_id' | 'fund_id' | 'grant_id'
export type LineErrors = Record<string, string>
export const lineFieldId = (key: string, f: LineField) => `${key}-${f}`

const LINE_FIELD_LABEL: Record<LineField, string> = {
  description: 'description',
  amount: 'amount',
  account_id: 'account',
  fund_id: 'fund',
  grant_id: 'grant',
}

/** Field errors for coded lines, keyed by control id, with summary labels. */
export function validateLines(state: LedgerState, lines: Array<LineDraft>) {
  const errors: LineErrors = {}
  const summary: Array<{ id: string; label: string; message: string }> = []
  const add = (l: LineDraft, n: number, f: LineField, message: string) => {
    const id = lineFieldId(l.key, f)
    errors[id] = message
    summary.push({ id, label: `Line ${n + 1} ${LINE_FIELD_LABEL[f]}`, message })
  }
  lines.forEach((l, n) => {
    if (!l.description.trim()) add(l, n, 'description', 'Say what this was for.')
    const amt = parseMoney(l.amount)
    if (!l.amount.trim()) add(l, n, 'amount', 'Enter the amount.')
    else if (amt === null) add(l, n, 'amount', 'Enter a number, like 1250 or 1,250.50.')
    else if (amt <= 0) add(l, n, 'amount', 'The amount must be more than zero.')
    if (!l.account_id) add(l, n, 'account_id', 'Choose what kind of cost this is.')
    const fund = state.funds.find((f) => f.id === l.fund_id)
    if (!fund) add(l, n, 'fund_id', 'Choose the fund that pays for this.')
    else if (fund.restriction !== 'unrestricted' && !l.grant_id)
      add(l, n, 'grant_id', 'Restricted money needs the grant it comes from.')
  })
  return { errors, summary }
}

export function toCodedLines(lines: Array<LineDraft>): Array<Omit<CodedLine, 'id'>> {
  return lines.map((l) => ({
    description: l.description.trim(),
    account_id: l.account_id,
    amount: parseMoney(l.amount) ?? 0,
    fund_id: l.fund_id,
    grant_id: l.grant_id || null,
    dimension_ids: [l.project, l.location, l.dimension].filter(Boolean),
  }))
}

export function linesTotal(lines: Array<LineDraft>): Minor {
  return sum(lines.map((l) => Math.max(0, parseMoney(l.amount) ?? 0)))
}

/** Changing the fund drops a grant from another fund and picks the only one. */
function withFund(state: LedgerState, l: LineDraft, fund_id: string): LineDraft {
  const grants = state.grants.filter((g) => g.fund_id === fund_id)
  const keep = grants.some((g) => g.id === l.grant_id)
  return { ...l, fund_id, grant_id: keep ? l.grant_id : grants.length === 1 ? grants[0].id : '' }
}

export function LineEditor({
  state,
  lines,
  onChange,
  errors,
  currency,
  accountTypes,
  dimensions,
  disabled,
}: {
  state: LedgerState
  lines: Array<LineDraft>
  onChange: (lines: Array<LineDraft>) => void
  errors: LineErrors
  currency: string
  accountTypes: Array<AccountType>
  /** 'project-location' shows two pickers (claims); 'single' one (bills). */
  dimensions: 'project-location' | 'single'
  disabled?: boolean
}) {
  const set = (key: string, patch: Partial<LineDraft>) =>
    onChange(lines.map((l) => (l.key === key ? { ...l, ...patch } : l)))
  const projects = useMemo(
    () => state.dimensions.filter((d) => d.kind === 'project').map((d) => ({ value: d.id, label: d.name })),
    [state.dimensions],
  )
  const locations = useMemo(
    () => state.dimensions.filter((d) => d.kind === 'location').map((d) => ({ value: d.id, label: d.name })),
    [state.dimensions],
  )

  return (
    <div>
      <ol className="divide-y divide-border">
        {lines.map((l, n) => {
          const id = (f: LineField) => lineFieldId(l.key, f)
          return (
            <li key={l.key} className="py-5">
              <fieldset className="min-w-0">
                <div className="flex min-h-10 items-center justify-between gap-3">
                  <legend className="float-left text-sm font-semibold">Line {n + 1}</legend>
                  {lines.length > 1 && (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      disabled={disabled}
                      onClick={() => onChange(lines.filter((x) => x.key !== l.key))}
                      aria-label={`Remove line ${n + 1}`}
                    >
                      <Trash />
                    </Button>
                  )}
                </div>
                <div className="mt-2 grid gap-4 sm:grid-cols-[2fr_1fr]">
                  <Field id={id('description')} label="What it was for" required error={errors[id('description')]}>
                    {(p) => (
                      <Input
                        {...p}
                        value={l.description}
                        onChange={(e) => set(l.key, { description: e.target.value })}
                        autoComplete="off"
                        placeholder="e.g. Fuel, motorbike hire"
                        className="h-11"
                      />
                    )}
                  </Field>
                  <Field
                    id={id('amount')}
                    label={`Amount, ${currency}`}
                    required
                    error={errors[id('amount')]}
                  >
                    {(p) => (
                      <Input
                        {...p}
                        inputMode="decimal"
                        value={l.amount}
                        onChange={(e) => set(l.key, { amount: e.target.value })}
                        autoComplete="off"
                        placeholder="0.00"
                        className="h-11 text-right text-lg tabular-nums md:text-lg"
                      />
                    )}
                  </Field>
                  <Field id={id('account_id')} label="Kind of cost" required error={errors[id('account_id')]}>
                    {(p) => (
                      <AccountSelect
                        {...p}
                        state={state}
                        types={accountTypes}
                        value={l.account_id}
                        onValueChange={(v) => set(l.key, { account_id: v })}
                        disabled={disabled}
                        className="h-11"
                      />
                    )}
                  </Field>
                  <Field id={id('fund_id')} label="Fund" required error={errors[id('fund_id')]}>
                    {(p) => (
                      <FundSelect
                        {...p}
                        state={state}
                        value={l.fund_id}
                        onValueChange={(v) => onChange(lines.map((x) => (x.key === l.key ? withFund(state, x, v) : x)))}
                        disabled={disabled}
                        className="h-11"
                      />
                    )}
                  </Field>
                  <Field id={id('grant_id')} label="Grant" error={errors[id('grant_id')]} hint="Only grants on the chosen fund are listed">
                    {(p) => (
                      <GrantSelect
                        {...p}
                        state={state}
                        fundId={l.fund_id}
                        value={l.grant_id}
                        onValueChange={(v) => set(l.key, { grant_id: v })}
                        disabled={disabled}
                        className="h-11"
                      />
                    )}
                  </Field>
                  {dimensions === 'project-location' ? (
                    <div className="grid gap-4 sm:col-span-2 sm:grid-cols-2">
                      <Field id={`${l.key}-project`} label="Project">
                        {(p) => (
                          <FormSelect
                            {...p}
                            value={l.project}
                            onValueChange={(v) => set(l.key, { project: v })}
                            options={[{ value: '', label: 'No project' }, ...projects]}
                            disabled={disabled}
                            className="h-11"
                          />
                        )}
                      </Field>
                      <Field id={`${l.key}-location`} label="Place">
                        {(p) => (
                          <FormSelect
                            {...p}
                            value={l.location}
                            onValueChange={(v) => set(l.key, { location: v })}
                            options={[{ value: '', label: 'No place' }, ...locations]}
                            disabled={disabled}
                            className="h-11"
                          />
                        )}
                      </Field>
                    </div>
                  ) : (
                    <Field id={`${l.key}-dimension`} label="Project, department or place">
                      {(p) => (
                        <DimensionSelect
                          {...p}
                          state={state}
                          value={l.dimension}
                          onValueChange={(v) => set(l.key, { dimension: v })}
                          disabled={disabled}
                          className="h-11"
                        />
                      )}
                    </Field>
                  )}
                </div>
              </fieldset>
            </li>
          )
        })}
      </ol>
      <div className="flex flex-wrap items-center justify-between gap-3 border-t-2 border-foreground py-3">
        <Button
          type="button"
          variant="outline"
          disabled={disabled}
          onClick={() => {
            const last = lines.at(-1)
            onChange([
              ...lines,
              newLine(state, last && {
                account_id: last.account_id,
                fund_id: last.fund_id,
                grant_id: last.grant_id,
                project: last.project,
                location: last.location,
                dimension: last.dimension,
              }),
            ])
          }}
        >
          <Plus />
          Add another line
        </Button>
        <p className="text-sm">
          <span className="text-muted-foreground">Total </span>
          <Money value={linesTotal(lines)} currency={currency} strong zero="0.00" className="text-base" />
        </p>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Error summary for forms
// ---------------------------------------------------------------------------

export function ErrorSummary({
  items,
  saveError,
  summaryRef,
}: {
  items: Array<{ id: string; label: string; message: string }>
  saveError?: unknown
  summaryRef: React.RefObject<HTMLDivElement | null>
}) {
  const titleId = useId()
  if (saveError) {
    return (
      <div ref={summaryRef} tabIndex={-1} className="outline-none">
        <IssueSummary error={saveError} />
      </div>
    )
  }
  if (items.length === 0) return null
  return (
    <div
      ref={summaryRef}
      tabIndex={-1}
      role="alert"
      aria-labelledby={titleId}
      className="border border-destructive p-4 outline-none focus-visible:ring-2 focus-visible:ring-destructive/40"
    >
      <p id={titleId} className="font-semibold text-destructive">
        Fix {items.length === 1 ? 'this' : 'these'} before saving
      </p>
      <ul className="mt-2 space-y-1 text-sm">
        {items.map((i) => (
          <li key={i.id}>
            <a
              href={`#${i.id}`}
              onClick={(e) => {
                e.preventDefault()
                document.getElementById(i.id)?.focus()
              }}
              className="underline underline-offset-4 hover:text-destructive"
            >
              {i.label}: {i.message}
            </a>
          </li>
        ))}
      </ul>
    </div>
  )
}

// ---------------------------------------------------------------------------
// One command in a dialog, with the override-with-a-reason flow
// ---------------------------------------------------------------------------

export function CommandDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  pendingLabel = 'Saving…',
  destructive,
  allowOverride,
  validate,
  onConfirm,
  children,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: ReactNode
  description?: ReactNode
  confirmLabel: string
  pendingLabel?: string
  destructive?: boolean
  /** Offer an override with a reason when every issue allows one. */
  allowOverride?: boolean
  /** Return false to stop; the caller shows its own field errors. */
  validate?: () => boolean
  onConfirm: (overrideReason?: string) => Promise<unknown>
  children?: ReactNode
}) {
  const [error, setError] = useState<unknown>(null)
  const [pending, setPending] = useState(false)
  const [reason, setReason] = useState('')
  const [reasonError, setReasonError] = useState<string | undefined>()

  useEffect(() => {
    if (open) {
      setError(null)
      setReason('')
      setReasonError(undefined)
    }
  }, [open])

  const overriding = !!allowOverride && hasOverridable(error)

  const confirm = async () => {
    if (pending) return
    if (validate && !validate()) return
    if (overriding && !reason.trim()) {
      setReasonError('Say why this should go through anyway. It is kept with the entry.')
      return
    }
    setPending(true)
    try {
      await onConfirm(overriding ? reason.trim() : undefined)
      onOpenChange(false)
    } catch (e) {
      setError(e)
    } finally {
      setPending(false)
    }
  }

  return (
    <ReportDialog
      open={open}
      onOpenChange={(o) => !pending && onOpenChange(o)}
      title={title}
      description={description}
      footer={
        <>
          <Button variant="ghost" size="lg" type="button" onClick={() => onOpenChange(false)} disabled={pending}>
            Cancel
          </Button>
          <Button
            size="lg"
            type="button"
            variant={destructive ? 'destructive' : 'default'}
            onClick={confirm}
            disabled={pending}
          >
            {pending ? pendingLabel : overriding ? `Override and ${confirmLabel.toLowerCase()}` : confirmLabel}
          </Button>
        </>
      }
    >
      <form
        noValidate
        onSubmit={(e) => {
          e.preventDefault()
          confirm()
        }}
        className="space-y-4"
      >
        {error ? (
          <IssueSummary
            error={error}
            title={overriding ? 'The ledger flagged this' : 'Not done'}
          />
        ) : null}
        <fieldset disabled={pending} className="min-w-0 space-y-4">
          {children}
          {overriding && (
            <Field id="override-reason" label="Reason for overriding" required error={reasonError} hint="Recorded on the journal entry for auditors">
              {(p) => (
                <Textarea
                  {...p}
                  rows={3}
                  value={reason}
                  onChange={(e) => {
                    setReason(e.target.value)
                    if (reasonError) setReasonError(undefined)
                  }}
                />
              )}
            </Field>
          )}
        </fieldset>
        <button type="submit" className="sr-only" tabIndex={-1} aria-hidden="true">
          {confirmLabel}
        </button>
      </form>
    </ReportDialog>
  )
}

/** Date written as the books do: 3 Feb 2026, or a dash. */
export const dateOrDash = (d: string | null | undefined) => (d ? formatDate(d) : '–')
