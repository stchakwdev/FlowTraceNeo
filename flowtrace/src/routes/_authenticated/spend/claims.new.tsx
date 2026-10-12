import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useRef, useState } from 'react'
import { ArrowLeft, Camera, CloudSlash, Paperclip, Trash } from '@phosphor-icons/react'
import {
  ErrorSummary,
  LineEditor,
  newLine,
  toCodedLines,
  validateLines,
} from './-shared'
import type { ClaimInput } from '@/lib/offline-claims'
import type { LineDraft, LineErrors } from './-shared'
import { createdId, useLedger, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import {
  enqueueClaim,
  isNetworkError,
  isOffline,
  useOfflineClaimSync,
} from '@/lib/offline-claims'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Field } from '@/components/form'
import { ReportSection } from '@/components/report'
import { BooksState } from '@/components/ledger'

export const Route = createFileRoute('/_authenticated/spend/claims/new')({
  component: NewClaim,
})

const MAX_RECEIPTS = 8

type Errors = { date?: string; purpose?: string }

function newClaimId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return `claim-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}

function NewClaim() {
  const navigate = useNavigate()
  const { state, isLoading, error, refetch } = useLedger()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()
  const { queued } = useOfflineClaimSync()

  const [date, setDate] = useState(isoToDayMonthYear(asOf))
  const [purpose, setPurpose] = useState('')
  const [lines, setLines] = useState<Array<LineDraft>>([])
  const [receipts, setReceipts] = useState<Array<string>>([])
  const [errors, setErrors] = useState<Errors>({})
  const [lineErrors, setLineErrors] = useState<LineErrors>({})
  const [summary, setSummary] = useState<Array<{ id: string; label: string; message: string }>>([])
  const [saveError, setSaveError] = useState<unknown>(null)
  const [savedOffline, setSavedOffline] = useState(false)
  const [saving, setSaving] = useState<'draft' | 'submit' | null>(null)
  const summaryRef = useRef<HTMLDivElement>(null)
  const cameraRef = useRef<HTMLInputElement>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  // Start with one line once the books (and their funds) are here.
  if (state && lines.length === 0) setLines([newLine(state)])

  const base = state?.settings.base_currency ?? 'KES'

  const addReceipts = (files: FileList | null) => {
    const names = Array.from(files ?? []).map((f) => f.name)
    setReceipts((prev) => [...prev, ...names].slice(0, MAX_RECEIPTS))
  }

  const check = () => {
    if (!state) return false
    const e: Errors = {}
    const d = parseDayMonthYear(date)
    if (!d) e.date = 'Enter the date you spent the money.'
    else if (d.iso === undefined) e.date = d.error
    else if (d.iso > asOf) e.date = 'The date cannot be after today.'
    if (!purpose.trim()) e.purpose = 'Say what the trip or task was, e.g. Borehole site inspection, Lodwar.'
    const l = validateLines(state, lines)
    setErrors(e)
    setLineErrors(l.errors)
    const items = [
      ...(e.date ? [{ id: 'claim-date', label: 'Date', message: e.date }] : []),
      ...(e.purpose ? [{ id: 'purpose', label: 'Purpose', message: e.purpose }] : []),
      ...l.summary,
    ]
    setSummary(items)
    return items.length === 0
  }

  const reset = () => {
    setPurpose('')
    setLines(state ? [newLine(state)] : [])
    setReceipts([])
    setErrors({})
    setLineErrors({})
    setSummary([])
  }

  const save = async (submit: boolean) => {
    if (saving || !state) return
    setSaveError(null)
    setSavedOffline(false)
    if (!check()) {
      requestAnimationFrame(() => summaryRef.current?.focus())
      return
    }
    const input: ClaimInput = {
      id: newClaimId(),
      claim_date: parseDayMonthYear(date)?.iso ?? asOf,
      currency: base,
      fx_rate: 1,
      purpose: purpose.trim(),
      receipt_names: receipts,
      captured_offline: false,
      lines: toCodedLines(lines),
      submit,
    }

    const keepOnPhone = async () => {
      await enqueueClaim(input)
      reset()
      setSavedOffline(true)
      window.scrollTo({ top: 0 })
    }

    setSaving(submit ? 'submit' : 'draft')
    try {
      if (isOffline()) {
        await keepOnPhone()
        return
      }
      try {
        const r = await command.mutateAsync({ type: 'claim.create', input })
        const id = createdId(r) ?? input.id
        if (id) navigate({ to: '/spend/claims/$id', params: { id } })
      } catch (e) {
        if (isNetworkError(e)) await keepOnPhone()
        else {
          setSaveError(e)
          requestAnimationFrame(() => summaryRef.current?.focus())
        }
      }
    } catch (e) {
      setSaveError(e)
    } finally {
      setSaving(null)
    }
  }

  return (
    <div className="mx-auto max-w-3xl px-4 py-6 sm:px-8 sm:py-8">
      <Link
        to="/spend/claims"
        className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Claims
      </Link>
      <h1 className="mt-4 text-4xl sm:text-5xl">New expense claim</h1>
      <p className="mt-2 max-w-prose text-sm text-muted-foreground">
        Money you spent from your own pocket for the organisation. Once you
        submit it, a manager who is not you approves it, and then it is paid
        back to you.
      </p>

      {savedOffline && (
        <div role="status" className="mt-6 flex gap-3 border border-foreground p-4 text-sm">
          <CloudSlash aria-hidden="true" className="mt-0.5 size-5 shrink-0" />
          <div>
            <p className="font-semibold">Saved on this phone; it will be sent when you are back online.</p>
            <p className="mt-1 text-muted-foreground">
              Keep the paper receipts. You can start another claim now, or{' '}
              <Link to="/spend/claims" className="font-semibold text-foreground underline underline-offset-4">
                see claims waiting on this phone
              </Link>
              .
            </p>
          </div>
        </div>
      )}
      {!savedOffline && queued.length > 0 && (
        <p className="mt-6 flex items-center gap-2 border-y border-border py-3 text-sm">
          <CloudSlash aria-hidden="true" className="size-4 shrink-0" />
          {queued.length} {queued.length === 1 ? 'claim is' : 'claims are'} waiting on this phone to be sent.
        </p>
      )}

      {!state ? (
        <div className="mt-8">
          <BooksState isLoading={isLoading} error={error} refetch={refetch} label="claim form" />
        </div>
      ) : (
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault()
            save(true)
          }}
          className="mt-8 space-y-10"
        >
          <ErrorSummary items={summary} saveError={saveError} summaryRef={summaryRef} />

          <fieldset disabled={!!saving} className="min-w-0 space-y-10">
            <ReportSection title="The claim">
              <div className="grid gap-5 pt-5 sm:grid-cols-[14rem_1fr]">
                <Field id="claim-date" label="Date spent" required error={errors.date} hint="Day, month, year">
                  {(p) => (
                    <DateField
                      {...p}
                      max={asOf}
                      value={date}
                      onValueChange={setDate}
                      pickerLabel="Choose the date from a calendar"
                      className="h-11"
                    />
                  )}
                </Field>
                <Field id="purpose" label="Purpose" required error={errors.purpose} hint="The trip or task, as a manager would recognise it">
                  {(p) => (
                    <Input
                      {...p}
                      value={purpose}
                      onChange={(e) => setPurpose(e.target.value)}
                      autoComplete="off"
                      className="h-11"
                    />
                  )}
                </Field>
              </div>
            </ReportSection>

            <ReportSection title="What you spent" meta={<span>In {base}</span>}>
              <LineEditor
                state={state}
                lines={lines}
                onChange={setLines}
                errors={lineErrors}
                currency={base}
                accountTypes={['expense']}
                dimensions="project-location"
                disabled={!!saving}
              />
            </ReportSection>

            <ReportSection title="Receipts" meta={<span>Optional · up to {MAX_RECEIPTS}</span>}>
              <div className="space-y-4 pt-5">
                <div className="grid gap-2 sm:grid-cols-2">
                  <Button
                    type="button"
                    size="lg"
                    variant="outline"
                    className="h-12"
                    onClick={() => cameraRef.current?.click()}
                    disabled={receipts.length >= MAX_RECEIPTS}
                    aria-describedby="receipts-hint"
                  >
                    <Camera className="size-5" />
                    Take a photo of a receipt
                  </Button>
                  <Button
                    type="button"
                    size="lg"
                    variant="outline"
                    className="h-12"
                    onClick={() => fileRef.current?.click()}
                    disabled={receipts.length >= MAX_RECEIPTS}
                    aria-describedby="receipts-hint"
                  >
                    <Paperclip className="size-5" />
                    Choose saved photos or files
                  </Button>
                </div>
                <input
                  ref={cameraRef}
                  type="file"
                  accept="image/*"
                  capture="environment"
                  tabIndex={-1}
                  aria-hidden="true"
                  className="sr-only"
                  onChange={(e) => {
                    addReceipts(e.target.files)
                    e.target.value = ''
                  }}
                />
                <input
                  ref={fileRef}
                  type="file"
                  accept="image/*,application/pdf"
                  multiple
                  tabIndex={-1}
                  aria-hidden="true"
                  className="sr-only"
                  onChange={(e) => {
                    addReceipts(e.target.files)
                    e.target.value = ''
                  }}
                />
                <p id="receipts-hint" className="text-xs text-muted-foreground">
                  For now the claim records each receipt by its file name, so
                  keep the paper receipts until the claim is paid.
                </p>
                {receipts.length > 0 && (
                  <ul className="divide-y divide-border border-y border-border">
                    {receipts.map((name, i) => (
                      <li key={`${name}-${i}`} className="flex items-center gap-3 py-1.5">
                        <Paperclip aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                        <span className="min-w-0 flex-1 truncate text-sm">{name}</span>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          onClick={() => setReceipts((prev) => prev.filter((_, j) => j !== i))}
                          aria-label={`Remove ${name}`}
                        >
                          <Trash />
                        </Button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </ReportSection>
          </fieldset>

          <div className="flex flex-col-reverse gap-2 border-t-[3px] border-foreground pt-5 sm:flex-row sm:items-center sm:justify-end">
            <Link to="/spend/claims" className={cn(buttonVariants({ variant: 'ghost', size: 'lg' }))}>
              Cancel
            </Link>
            <Button type="button" variant="outline" size="lg" disabled={!!saving} onClick={() => save(false)}>
              {saving === 'draft' ? 'Saving…' : 'Save draft'}
            </Button>
            <Button type="submit" size="lg" disabled={!!saving}>
              {saving === 'submit' ? 'Submitting…' : 'Submit for approval'}
            </Button>
          </div>
        </form>
      )}
    </div>
  )
}
