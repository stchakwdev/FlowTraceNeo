import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useEffect, useMemo, useRef, useState } from 'react'
import { ArrowLeft, Plus, X } from '@phosphor-icons/react'
import { str } from './-shared'
import type { EntryInput, Issue, Ledger, LedgerState, LineInput } from '@/lib/ledger'
import { UNRESTRICTED_FUND_ID, parseMoney } from '@/lib/ledger'
import { createdId, useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
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
import { ReportNotice, ReportSection } from '@/components/report'
import {
  AccountSelect,
  BooksPage,
  BooksState,
  DimensionSelect,
  FundSelect,
  GrantSelect,
  IssueSummary,
  formatMinor,
  hasOverridable,
} from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/ledger/new')({
  validateSearch: (search: Record<string, unknown>): { draft?: string } => ({
    draft: str(search.draft),
  }),
  component: NewEntryPage,
})

type LineDraft = {
  key: number
  account: string
  fund: string
  grant: string
  dimension: string
  debit: string
  credit: string
  memo: string
  /** Kept from a loaded draft so editing does not drop them. */
  keep: Pick<LineInput, 'currency' | 'fx_rate' | 'flow_node_id'> & { extraDims: Array<string> }
}

type LineField = 'account' | 'fund' | 'grant' | 'debit' | 'credit'

// Which control an engine issue belongs to, so the message sits beside it.
const ISSUE_FIELD: Partial<Record<Issue['code'], LineField>> = {
  UNKNOWN_ACCOUNT: 'account',
  GROUP_ACCOUNT: 'account',
  INACTIVE_ACCOUNT: 'account',
  UNKNOWN_FUND: 'fund',
  UNKNOWN_GRANT: 'grant',
  GRANT_FUND_MISMATCH: 'grant',
  GRANT_NOT_STARTED: 'grant',
  GRANT_ENDED: 'grant',
  RESTRICTED_NEEDS_GRANT: 'grant',
  LINE_ZERO: 'debit',
  LINE_BOTH_SIDES: 'debit',
  NEGATIVE_AMOUNT: 'debit',
  NOT_MINOR_UNITS: 'debit',
  FX_RATE_REQUIRED: 'debit',
}

let nextKey = 1
const blankLine = (fund: string): LineDraft => ({
  key: nextKey++,
  account: '',
  fund,
  grant: '',
  dimension: '',
  debit: '',
  credit: '',
  memo: '',
  keep: { extraDims: [] },
})

const minorToText = (m: number | undefined) => (m ? (m / 100).toFixed(2) : '')

function NewEntryPage() {
  const { draft } = Route.useSearch()
  const { state, ledger, isLoading, error, refetch } = useLedger()

  const back = (
    <Link
      to={draft ? '/ledger/entries/$id' : '/ledger'}
      params={draft ? { id: draft } : undefined}
      className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeft className="size-4" />
      {draft ? 'Back to the draft' : 'Journal'}
    </Link>
  )
  const title = draft ? 'Edit draft entry' : 'New journal entry'
  const lede =
    'A manual entry moves amounts between accounts. Debits must equal credits, in total and within each fund. Save it as a draft to finish later, or post it to put it in the books.'

  if (!state || !ledger)
    return (
      <BooksPage title={title} lede={lede} nav={back} width="narrow">
        <BooksState isLoading={isLoading} error={error} refetch={refetch} label="ledger" />
      </BooksPage>
    )

  const existing = draft ? ledger.entry(draft) : undefined
  if (draft && (!existing || existing.status !== 'draft'))
    return (
      <BooksPage title={title} nav={back} width="narrow">
        <ReportNotice
          title={existing ? `${existing.number} is ${existing.status}` : 'This draft is not in the books'}
          action={
            <Link to="/ledger" className={buttonVariants({ variant: 'outline' })}>
              Back to the journal
            </Link>
          }
        >
          {existing
            ? 'Posted entries cannot be edited. Reverse it and post a correcting entry instead.'
            : 'It may have been deleted or posted from another screen.'}
        </ReportNotice>
      </BooksPage>
    )

  return (
    <BooksPage title={title} lede={lede} nav={back} width="narrow">
      <EntryForm key={draft ?? 'new'} state={state} ledger={ledger} draftId={draft} />
    </BooksPage>
  )
}

function EntryForm({
  state,
  ledger,
  draftId,
}: {
  state: LedgerState
  ledger: Ledger
  draftId?: string
}) {
  const navigate = useNavigate()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const base = state.settings.base_currency
  const defaultFund = state.funds.some((f) => f.id === UNRESTRICTED_FUND_ID)
    ? UNRESTRICTED_FUND_ID
    : (state.funds[0]?.id ?? '')

  const [date, setDate] = useState(isoToDayMonthYear(asOf))
  const [memo, setMemo] = useState('')
  const [lines, setLines] = useState<Array<LineDraft>>(() => [blankLine(defaultFund), blankLine(defaultFund)])
  const [submitted, setSubmitted] = useState(false)
  const [saveError, setSaveError] = useState<unknown>(null)
  const [needsOverride, setNeedsOverride] = useState(false)
  const [overrideReason, setOverrideReason] = useState('')
  const [overrideError, setOverrideError] = useState<string>()
  const summaryRef = useRef<HTMLDivElement>(null)
  const loaded = useRef(false)

  // Load the draft once.
  useEffect(() => {
    if (!draftId || loaded.current) return
    loaded.current = true
    const input = ledger.inputOf(draftId)
    setDate(isoToDayMonthYear(input.date))
    setMemo(input.memo)
    if (input.override_reason) setOverrideReason(input.override_reason)
    const loadedLines = input.lines.map((l): LineDraft => {
      const [first = '', ...rest] = l.dimension_ids ?? []
      return {
        key: nextKey++,
        account: l.account_id,
        fund: l.fund_id,
        grant: l.grant_id ?? '',
        dimension: first,
        debit: minorToText(l.debit),
        credit: minorToText(l.credit),
        memo: l.memo ?? '',
        keep: { currency: l.currency, fx_rate: l.fx_rate, flow_node_id: l.flow_node_id, extraDims: rest },
      }
    })
    while (loadedLines.length < 2) loadedLines.push(blankLine(defaultFund))
    setLines(loadedLines)
  }, [draftId, ledger, defaultFund])

  const parsedDate = parseDayMonthYear(date)

  // Client-side problems: what the engine cannot see because it never parses text.
  const fieldErrors = useMemo(() => {
    const e: Record<string, string> = {}
    if (!parsedDate) e.date = 'Enter the entry date.'
    else if (parsedDate.error) e.date = parsedDate.error
    if (!memo.trim()) e.memo = 'Describe the entry, as it should read in the journal.'
    lines.forEach((l, i) => {
      for (const side of ['debit', 'credit'] as const) {
        const raw = l[side].trim()
        if (!raw) continue
        const m = parseMoney(raw)
        if (m === null) e[`line-${i}-${side}`] = 'Enter a number, like 1250 or 1,250.50.'
        else if (m < 0) e[`line-${i}-${side}`] = 'Enter a positive amount; use the other column for the opposite side.'
      }
    })
    return e
  }, [parsedDate, memo, lines])

  const input = useMemo<EntryInput>(
    () => ({
      date: parsedDate?.iso ?? '',
      memo,
      override_reason: overrideReason.trim() || null,
      lines: lines.map((l) => ({
        account_id: l.account,
        debit: Math.max(0, parseMoney(l.debit) ?? 0),
        credit: Math.max(0, parseMoney(l.credit) ?? 0),
        fund_id: l.fund,
        grant_id: l.grant || null,
        dimension_ids: [...(l.dimension ? [l.dimension] : []), ...l.keep.extraDims.filter((d) => d !== l.dimension)],
        memo: l.memo.trim() || null,
        currency: l.keep.currency,
        fx_rate: l.keep.fx_rate,
        flow_node_id: l.keep.flow_node_id ?? null,
      })),
    }),
    [parsedDate?.iso, memo, overrideReason, lines],
  )

  const issues = useMemo(
    () => (submitted ? ledger.validateEntry(input, draftId) : []),
    [submitted, ledger, input, draftId],
  )

  const totals = useMemo(() => {
    let debits = 0
    let credits = 0
    for (const l of input.lines) {
      const rate = l.currency && l.currency !== base ? (l.fx_rate ?? 1) : 1
      debits += Math.round((l.debit ?? 0) * rate)
      credits += Math.round((l.credit ?? 0) * rate)
    }
    return { debits, credits, difference: debits - credits }
  }, [input.lines, base])

  const lineIssues = (i: number) => issues.filter((x) => x.line === i)
  const entryIssues = issues.filter((x) => x.line === undefined)
  const strip = (m: string) => m.replace(/^Line \d+: /, '')
  const fieldIssue = (i: number, f: LineField) => {
    const own = fieldErrors[`line-${i}-${f}`]
    if (own) return own
    const found = lineIssues(i).filter((x) => ISSUE_FIELD[x.code] === f)
    return found.length ? found.map((x) => strip(x.message)).join(' ') : undefined
  }
  const loose = (i: number) => lineIssues(i).filter((x) => !ISSUE_FIELD[x.code])

  const updateLine = (i: number, patch: Partial<LineDraft>) =>
    setLines((prev) =>
      prev.map((l, j) => {
        if (j !== i) return l
        const next = { ...l, ...patch }
        // A grant belongs to one fund; clear it when the fund changes.
        if (patch.fund !== undefined && next.grant) {
          const g = state.grants.find((x) => x.id === next.grant)
          if (g && g.fund_id !== patch.fund) next.grant = ''
        }
        // Typing a debit clears the credit and vice versa.
        if (patch.debit) next.credit = ''
        if (patch.credit) next.debit = ''
        return next
      }),
    )

  const focusSummary = () => requestAnimationFrame(() => summaryRef.current?.focus())

  const finish = (id: string | undefined) => {
    if (id) navigate({ to: '/ledger/entries/$id', params: { id } })
    else navigate({ to: '/ledger' })
  }

  const saveDraft = async () => {
    setSubmitted(true)
    setSaveError(null)
    if (fieldErrors.date || fieldErrors.memo || Object.keys(fieldErrors).some((k) => k.startsWith('line-'))) {
      focusSummary()
      return
    }
    try {
      const r = draftId
        ? await command.mutateAsync({ type: 'entry.updateDraft', entryId: draftId, input })
        : await command.mutateAsync({ type: 'entry.saveDraft', input })
      finish(createdId(r) ?? draftId)
    } catch (e) {
      setSaveError(e)
      focusSummary()
    }
  }

  const post = async (withOverride: boolean) => {
    setSubmitted(true)
    setSaveError(null)
    if (withOverride && !overrideReason.trim()) {
      setOverrideError('Say why this entry should post despite the rule.')
      return
    }
    setOverrideError(undefined)
    const reason = withOverride ? overrideReason.trim() : undefined
    const postInput: EntryInput = { ...input, override_reason: reason ?? null }
    const found = ledger.validateEntry(postInput, draftId)
    const blocking = found.filter((x) => !(x.overridable && reason))
    if (Object.keys(fieldErrors).length || blocking.length) {
      if (!Object.keys(fieldErrors).length && blocking.every((x) => x.overridable)) setNeedsOverride(true)
      focusSummary()
      return
    }
    try {
      if (draftId) {
        await command.mutateAsync({ type: 'entry.updateDraft', entryId: draftId, input: postInput })
        await command.mutateAsync({ type: 'entry.postDraft', entryId: draftId, overrideReason: reason })
        finish(draftId)
      } else {
        const r = await command.mutateAsync({ type: 'entry.post', input: postInput })
        finish(createdId(r))
      }
    } catch (e) {
      setSaveError(e)
      if (hasOverridable(e)) setNeedsOverride(true)
      focusSummary()
    }
  }

  const clientList = [
    fieldErrors.date && { id: 'entry-date', text: `Date: ${fieldErrors.date}` },
    fieldErrors.memo && { id: 'entry-memo', text: `Memo: ${fieldErrors.memo}` },
    ...lines.flatMap((_, i) =>
      (['debit', 'credit'] as const)
        .filter((s) => fieldErrors[`line-${i}-${s}`])
        .map((s) => ({ id: `line-${i}-${s}`, text: `Line ${i + 1} ${s}: ${fieldErrors[`line-${i}-${s}`]}` })),
    ),
  ].filter(Boolean) as Array<{ id: string; text: string }>
  const engineList = issues
    .filter((x) => !(x.code === 'INVALID_INPUT' && (fieldErrors.date || fieldErrors.memo)))
    .map((x) => ({
      id:
        x.line !== undefined
          ? `line-${x.line}-${ISSUE_FIELD[x.code] ?? 'account'}`
          : x.code === 'NO_PERIOD' || x.code.startsWith('PERIOD')
            ? 'entry-date'
            : 'entry-lines',
      text: x.message + (x.overridable ? ' (can be overridden with a reason)' : ''),
    }))
  const summaryItems = [...clientList, ...engineList]
  const showSummary = submitted && summaryItems.length > 0
  const pending = command.isPending

  return (
    <div className="space-y-10">
      {!!saveError && <IssueSummary error={saveError} title={draftId ? 'Not saved' : 'Not saved or posted'} />}

      {showSummary && (
        <div
          ref={summaryRef}
          tabIndex={-1}
          role="alert"
          aria-labelledby="entry-summary-title"
          className="border border-destructive p-4 outline-none focus-visible:ring-2 focus-visible:ring-destructive/40"
        >
          <p id="entry-summary-title" className="font-semibold text-destructive">
            {summaryItems.length === 1 ? 'This stops the entry posting' : `${summaryItems.length} things stop the entry posting`}
          </p>
          <ul className="mt-2 space-y-1 text-sm">
            {summaryItems.map((item, n) => (
              <li key={n}>
                <a
                  href={`#${item.id}`}
                  onClick={(e) => {
                    e.preventDefault()
                    document.getElementById(item.id)?.focus()
                  }}
                  className="underline underline-offset-4 hover:text-destructive"
                >
                  {item.text}
                </a>
              </li>
            ))}
          </ul>
          <p className="mt-2 text-xs text-muted-foreground">
            A draft can still be saved; it is checked again when posted.
          </p>
        </div>
      )}

      <form
        noValidate
        onSubmit={(e) => {
          e.preventDefault()
          void post(false)
        }}
        className="space-y-10"
      >
        <fieldset disabled={pending} className="min-w-0 space-y-10">
          <ReportSection title="Entry">
            <div className="grid gap-5 pt-5 sm:grid-cols-[14rem_minmax(0,1fr)]">
              <Field id="entry-date" label="Date" required error={submitted ? fieldErrors.date : undefined} hint="Day, month, year">
                {(props) => (
                  <DateField
                    {...props}
                    value={date}
                    onValueChange={setDate}
                    pickerLabel="Choose the entry date from a calendar"
                    className="h-10"
                  />
                )}
              </Field>
              <Field id="entry-memo" label="Memo" required error={submitted ? fieldErrors.memo : undefined} hint="e.g. Accrue December audit fee">
                {(props) => (
                  <Input {...props} value={memo} onChange={(e) => setMemo(e.target.value)} autoComplete="off" className="h-10" />
                )}
              </Field>
            </div>
          </ReportSection>

          <ReportSection
            id="entry-lines"
            title="Lines"
            meta={<span>{lines.length} lines · amounts in {base}</span>}
          >
            <ol className="divide-y divide-border">
              {lines.map((l, i) => {
                const extra = loose(i)
                return (
                  <li key={l.key} className="py-5">
                    <fieldset className="min-w-0">
                      <legend className="sr-only">Line {i + 1}</legend>
                      <div className="mb-3 flex items-center justify-between gap-3">
                        <p aria-hidden="true" className="text-sm font-semibold">Line {i + 1}</p>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-10"
                          disabled={lines.length <= 2}
                          onClick={() => setLines((prev) => prev.filter((_, j) => j !== i))}
                          aria-label={`Remove line ${i + 1}`}
                        >
                          <X />
                          Remove
                        </Button>
                      </div>
                      <div className="grid gap-3 sm:grid-cols-2">
                        <div className="sm:col-span-2">
                          <Field id={`line-${i}-account`} label="Account" required error={fieldIssue(i, 'account')}>
                            {(props) => (
                              <AccountSelect {...props} state={state} value={l.account} onValueChange={(v) => updateLine(i, { account: v })} />
                            )}
                          </Field>
                        </div>
                        <Field id={`line-${i}-fund`} label="Fund" required error={fieldIssue(i, 'fund')}>
                          {(props) => (
                            <FundSelect {...props} state={state} value={l.fund} onValueChange={(v) => updateLine(i, { fund: v })} />
                          )}
                        </Field>
                        <Field id={`line-${i}-grant`} label="Grant" error={fieldIssue(i, 'grant')}>
                          {(props) => (
                            <GrantSelect {...props} state={state} fundId={l.fund} value={l.grant} onValueChange={(v) => updateLine(i, { grant: v })} />
                          )}
                        </Field>
                        <Field id={`line-${i}-dimension`} label="Tag">
                          {(props) => (
                            <DimensionSelect {...props} state={state} value={l.dimension} onValueChange={(v) => updateLine(i, { dimension: v })} />
                          )}
                        </Field>
                        <Field id={`line-${i}-memo`} label="Line memo">
                          {(props) => (
                            <Input {...props} value={l.memo} onChange={(e) => updateLine(i, { memo: e.target.value })} autoComplete="off" className="h-10" />
                          )}
                        </Field>
                        <Field id={`line-${i}-debit`} label="Debit" error={fieldIssue(i, 'debit')}>
                          {(props) => (
                            <Input
                              {...props}
                              inputMode="decimal"
                              value={l.debit}
                              onChange={(e) => updateLine(i, { debit: e.target.value })}
                              autoComplete="off"
                              placeholder="0.00"
                              className="h-10 text-right tabular-nums"
                            />
                          )}
                        </Field>
                        <Field id={`line-${i}-credit`} label="Credit" error={fieldIssue(i, 'credit')}>
                          {(props) => (
                            <Input
                              {...props}
                              inputMode="decimal"
                              value={l.credit}
                              onChange={(e) => updateLine(i, { credit: e.target.value })}
                              autoComplete="off"
                              placeholder="0.00"
                              className="h-10 text-right tabular-nums"
                            />
                          )}
                        </Field>
                      </div>
                      {extra.length > 0 && (
                        <ul className="mt-3 space-y-1 text-sm text-destructive">
                          {extra.map((x, n) => (
                            <li key={n}>{strip(x.message)}</li>
                          ))}
                        </ul>
                      )}
                    </fieldset>
                  </li>
                )
              })}
            </ol>
            <div className="border-t border-border py-3">
              <Button type="button" variant="outline" onClick={() => setLines((prev) => [...prev, blankLine(defaultFund)])}>
                <Plus />
                Add a line
              </Button>
            </div>
            <dl
              aria-live="polite"
              className="grid grid-cols-3 border-t-2 border-foreground py-3 text-sm [&>div+div]:border-l [&>div+div]:border-border [&>div+div]:pl-3 [&>div]:pr-3"
            >
              <div>
                <dt className="text-xs text-muted-foreground">Debits</dt>
                <dd className="figure text-2xl">{formatMinor(totals.debits, { zero: '0.00' })}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Credits</dt>
                <dd className="figure text-2xl">{formatMinor(totals.credits, { zero: '0.00' })}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Difference</dt>
                <dd className={cn('figure text-2xl', totals.difference !== 0 && 'text-destructive')}>
                  {totals.difference === 0 ? 'Balanced' : formatMinor(Math.abs(totals.difference))}
                </dd>
              </div>
            </dl>
          </ReportSection>

          {entryIssues.length > 0 && (
            <p className="sr-only" aria-live="polite">
              {entryIssues.map((x) => x.message).join(' ')}
            </p>
          )}

          {needsOverride && (
            <ReportSection title="Override">
              <div className="space-y-4 pt-5">
                <p className="max-w-prose text-sm text-muted-foreground">
                  Only grant rules stop this entry, and they can be set aside
                  with a reason a funder or auditor would accept. The reason is
                  stored on the entry and in the audit trail.
                </p>
                <Field id="override-reason" label="Override reason" required error={overrideError}>
                  {(props) => (
                    <Textarea {...props} rows={3} value={overrideReason} onChange={(e) => setOverrideReason(e.target.value)} />
                  )}
                </Field>
                <div className="flex justify-end">
                  <Button type="button" onClick={() => post(true)} disabled={pending}>
                    {pending ? 'Posting…' : 'Post with override'}
                  </Button>
                </div>
              </div>
            </ReportSection>
          )}
        </fieldset>

        <div className="flex flex-col-reverse gap-2 border-t-[3px] border-foreground pt-5 sm:flex-row sm:items-center sm:justify-end">
          <Link
            to={draftId ? '/ledger/entries/$id' : '/ledger'}
            params={draftId ? { id: draftId } : undefined}
            className={buttonVariants({ variant: 'ghost', size: 'lg' })}
          >
            Cancel
          </Link>
          <Button type="button" variant="outline" size="lg" onClick={saveDraft} disabled={pending}>
            {pending ? 'Saving…' : 'Save draft'}
          </Button>
          <Button type="submit" size="lg" disabled={pending}>
            {pending ? 'Posting…' : 'Post'}
          </Button>
        </div>
      </form>
    </div>
  )
}
