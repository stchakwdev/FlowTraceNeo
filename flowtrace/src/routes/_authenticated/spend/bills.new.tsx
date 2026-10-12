import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useRef, useState } from 'react'
import { ArrowLeft } from '@phosphor-icons/react'
import { ErrorSummary, LineEditor, newLine, toCodedLines, validateLines } from './-shared'
import type { LineDraft, LineErrors } from './-shared'
import { createdId, useLedger, useLedgerActor, useLedgerAsOf, useLedgerCommand } from '@/lib/ledger-client'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  DateField,
  isoToDayMonthYear,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import { Field, FormSelect } from '@/components/form'
import { ReportNotice, ReportSection } from '@/components/report'
import { BooksState } from '@/components/ledger'
import {
  FileRow,
  PROOF_ACCEPT,
  ProofDropzone,
  describeAccepted,
  validateProofFile,
} from '@/components/file-upload'

export const Route = createFileRoute('/_authenticated/spend/bills/new')({
  component: NewBill,
})

const CURRENCIES = [
  ['KES', 'Kenyan shilling'],
  ['USD', 'US dollar'],
  ['EUR', 'Euro'],
  ['GBP', 'British pound'],
  ['UGX', 'Ugandan shilling'],
  ['TZS', 'Tanzanian shilling'],
] as const

const MAX_PROOFS = 5
const MAX_PROOF_MB = 10

type FieldName = 'vendor' | 'vendorName' | 'number' | 'billDate' | 'dueDate' | 'fxRate'
type Errors = Partial<Record<FieldName | 'proof', string>>

const FIELD_LABEL: Record<FieldName, string> = {
  vendor: 'Vendor',
  vendorName: 'New vendor name',
  number: 'Bill number',
  billDate: 'Bill date',
  dueDate: 'Due date',
  fxRate: 'Exchange rate',
}

function addDays(iso: string, n: number) {
  const d = new Date(`${iso}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() + n)
  return d.toISOString().slice(0, 10)
}

function NewBill() {
  const navigate = useNavigate()
  const { state, isLoading, error, refetch } = useLedger()
  const actor = useLedgerActor()
  const asOf = useLedgerAsOf()
  const command = useLedgerCommand()

  const [vendorMode, setVendorMode] = useState<'existing' | 'new'>('existing')
  const [vendorId, setVendorId] = useState('')
  const [vendorName, setVendorName] = useState('')
  const [vendorDetails, setVendorDetails] = useState('')
  // A vendor added on an earlier, failed attempt, so a retry does not add it twice.
  const [addedVendor, setAddedVendor] = useState<{ name: string; id: string } | null>(null)
  const [number, setNumber] = useState('')
  const [billDate, setBillDate] = useState(isoToDayMonthYear(asOf))
  const [dueDate, setDueDate] = useState(isoToDayMonthYear(addDays(asOf, 30)))
  const [currency, setCurrency] = useState('')
  const [fxRate, setFxRate] = useState('')
  const [lines, setLines] = useState<Array<LineDraft>>([])
  const [proofs, setProofs] = useState<Array<File>>([])
  const [errors, setErrors] = useState<Errors>({})
  const [lineErrors, setLineErrors] = useState<LineErrors>({})
  const [summary, setSummary] = useState<Array<{ id: string; label: string; message: string }>>([])
  const [saveError, setSaveError] = useState<unknown>(null)
  const [saving, setSaving] = useState(false)
  const summaryRef = useRef<HTMLDivElement>(null)

  if (state && lines.length === 0) setLines([newLine(state)])
  const base = state?.settings.base_currency ?? 'KES'
  const cur = currency || base
  const foreign = cur !== base

  const check = () => {
    if (!state) return false
    const e: Errors = {}
    if (vendorMode === 'existing' && !vendorId) e.vendor = 'Choose the supplier, or add a new one.'
    if (vendorMode === 'new' && !vendorName.trim()) e.vendorName = 'Enter the supplier’s name as it appears on the bill.'
    if (!number.trim()) e.number = 'Enter the invoice number printed on the bill.'
    else if (
      state.bills.some(
        (b) => b.number.trim().toLowerCase() === number.trim().toLowerCase() && b.vendor_id === vendorId && vendorMode === 'existing',
      )
    )
      e.number = 'This supplier already has a bill with this number.'
    const bd = parseDayMonthYear(billDate)
    const dd = parseDayMonthYear(dueDate)
    if (!bd) e.billDate = 'Enter the date on the bill.'
    else if (bd.iso === undefined) e.billDate = bd.error
    if (!dd) e.dueDate = 'Enter when the bill must be paid.'
    else if (dd.iso === undefined) e.dueDate = dd.error
    else if (bd?.iso && dd.iso < bd.iso) e.dueDate = 'The due date cannot be before the bill date.'
    if (foreign) {
      const r = Number(fxRate.replace(/,/g, ''))
      if (!fxRate.trim()) e.fxRate = `Enter how many ${base} one ${cur} is worth.`
      else if (!(r > 0) || !Number.isFinite(r)) e.fxRate = 'Enter a positive number, like 129.45.'
    }
    const l = validateLines(state, lines)
    setErrors((prev) => ({ ...e, proof: prev.proof }))
    setLineErrors(l.errors)
    const order: Array<FieldName> = ['vendor', 'vendorName', 'number', 'billDate', 'dueDate', 'fxRate']
    const fieldId: Record<FieldName, string> = {
      vendor: 'vendor',
      vendorName: 'vendor-name',
      number: 'bill-number',
      billDate: 'bill-date',
      dueDate: 'due-date',
      fxRate: 'fx-rate',
    }
    const items = [
      ...order.filter((f) => e[f]).map((f) => ({ id: fieldId[f], label: FIELD_LABEL[f], message: e[f]! })),
      ...l.summary,
    ]
    setSummary(items)
    return items.length === 0
  }

  const addProofs = (chosen: Array<File>) => {
    const accept = PROOF_ACCEPT.invoice
    const problems: Array<string> = []
    const ok: Array<File> = []
    for (const f of chosen) {
      const p = validateProofFile(f, accept, MAX_PROOF_MB)
      if (p) problems.push(p)
      else ok.push(f)
    }
    const room = MAX_PROOFS - proofs.length
    if (ok.length > room) problems.push(`Only ${MAX_PROOFS} files can be added.`)
    setProofs((prev) => [...prev, ...ok.slice(0, Math.max(0, room))])
    setErrors((prev) => ({ ...prev, proof: problems.length ? problems.join(' ') : undefined }))
  }

  const save = async () => {
    if (saving || !state) return
    setSaveError(null)
    if (!check()) {
      requestAnimationFrame(() => summaryRef.current?.focus())
      return
    }
    setSaving(true)
    try {
      let vid = vendorId
      if (vendorMode === 'new') {
        const name = vendorName.trim()
        if (addedVendor?.name === name) vid = addedVendor.id
        else {
          const r = await command.mutateAsync({
            type: 'vendor.add',
            input: { name, node_id: null, payment_details: vendorDetails.trim() || null },
          })
          vid = createdId(r) ?? ''
          setAddedVendor({ name, id: vid })
        }
      }
      const r = await command.mutateAsync({
        type: 'bill.create',
        input: {
          vendor_id: vid,
          number: number.trim(),
          bill_date: parseDayMonthYear(billDate)?.iso ?? asOf,
          due_date: parseDayMonthYear(dueDate)?.iso ?? asOf,
          currency: cur,
          fx_rate: foreign ? Number(fxRate.replace(/,/g, '')) : 1,
          proof_ids: proofs.map((f) => f.name),
          lines: toCodedLines(lines),
        },
      })
      const id = createdId(r)
      if (id) navigate({ to: '/spend/bills/$id', params: { id } })
      else navigate({ to: '/spend/bills', search: {} })
    } catch (e) {
      setSaveError(e)
      requestAnimationFrame(() => summaryRef.current?.focus())
    } finally {
      setSaving(false)
    }
  }

  const vendorOptions = (state?.vendors ?? [])
    .slice()
    .sort((a, b) => a.name.localeCompare(b.name))
    .map((v) => ({ value: v.id, label: v.name }))

  return (
    <div className="mx-auto max-w-3xl px-4 py-6 sm:px-8 sm:py-8">
      <Link
        to="/spend/bills"
        className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Bills
      </Link>
      <h1 className="mt-4 text-4xl sm:text-5xl">New bill</h1>
      <p className="mt-2 max-w-prose text-sm text-muted-foreground">
        Enter a supplier’s invoice as it was received. It is saved as a draft;
        once approved and posted it shows in the books as money owed.
      </p>

      {!state ? (
        <div className="mt-8">
          <BooksState isLoading={isLoading} error={error} refetch={refetch} label="bill form" />
        </div>
      ) : actor && !actor.can_post ? (
        <ReportNotice title="You cannot enter bills" className="mt-8">
          Entering bills needs permission to post to the books. Ask a finance
          administrator to enter this one, or to give you that permission.
        </ReportNotice>
      ) : (
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault()
            save()
          }}
          className="mt-8 space-y-10"
        >
          <ErrorSummary items={summary} saveError={saveError} summaryRef={summaryRef} />

          <fieldset disabled={saving} className="min-w-0 space-y-10">
            <ReportSection title="Supplier">
              <div className="space-y-5 pt-5">
                <div role="group" aria-label="Supplier" className="inline-flex gap-px border border-border bg-border">
                  {(['existing', 'new'] as const).map((m) => (
                    <button
                      key={m}
                      type="button"
                      aria-pressed={vendorMode === m}
                      onClick={() => setVendorMode(m)}
                      className={cn(
                        'h-10 px-3 text-sm whitespace-nowrap transition-colors',
                        vendorMode === m
                          ? 'bg-primary font-semibold text-primary-foreground'
                          : 'bg-background text-muted-foreground hover:bg-muted hover:text-foreground',
                      )}
                    >
                      {m === 'existing' ? 'Existing supplier' : 'Add a new supplier'}
                    </button>
                  ))}
                </div>
                {vendorMode === 'existing' ? (
                  <Field id="vendor" label="Supplier" required error={errors.vendor}>
                    {(p) => (
                      <FormSelect
                        {...p}
                        value={vendorId}
                        onValueChange={setVendorId}
                        placeholder={vendorOptions.length ? 'Choose a supplier' : 'No suppliers yet'}
                        options={vendorOptions}
                      />
                    )}
                  </Field>
                ) : (
                  <div className="grid gap-5 sm:grid-cols-2">
                    <Field id="vendor-name" label="Supplier name" required error={errors.vendorName}>
                      {(p) => (
                        <Input {...p} value={vendorName} onChange={(e) => setVendorName(e.target.value)} autoComplete="off" className="h-10" />
                      )}
                    </Field>
                    <Field id="vendor-details" label="Payment details" hint="Paybill, till or bank account">
                      {(p) => (
                        <Input {...p} value={vendorDetails} onChange={(e) => setVendorDetails(e.target.value)} autoComplete="off" className="h-10" />
                      )}
                    </Field>
                  </div>
                )}
              </div>
            </ReportSection>

            <ReportSection title="The bill">
              <div className="grid gap-5 pt-5 sm:grid-cols-2">
                <Field id="bill-number" label="Bill number" required error={errors.number} hint="As printed on the invoice">
                  {(p) => (
                    <Input {...p} value={number} onChange={(e) => setNumber(e.target.value)} autoComplete="off" className="h-10 font-mono" />
                  )}
                </Field>
                <div className="hidden sm:block" />
                <Field id="bill-date" label="Bill date" required error={errors.billDate} hint="Day, month, year">
                  {(p) => (
                    <DateField {...p} value={billDate} onValueChange={setBillDate} pickerLabel="Choose the bill date from a calendar" className="h-10" />
                  )}
                </Field>
                <Field id="due-date" label="Due date" required error={errors.dueDate} hint="Day, month, year">
                  {(p) => (
                    <DateField {...p} value={dueDate} onValueChange={setDueDate} pickerLabel="Choose the due date from a calendar" className="h-10" />
                  )}
                </Field>
                <Field id="currency" label="Currency" fixed>
                  {(p) => (
                    <FormSelect
                      {...p}
                      value={cur}
                      onValueChange={setCurrency}
                      options={CURRENCIES.map(([code, name]) => ({ value: code, label: `${code} · ${name}` }))}
                    />
                  )}
                </Field>
                {foreign ? (
                  <Field
                    id="fx-rate"
                    label={`Exchange rate, ${base} per ${cur}`}
                    required
                    error={errors.fxRate}
                    hint="The rate on the bill date. The books convert at this rate."
                  >
                    {(p) => (
                      <Input
                        {...p}
                        inputMode="decimal"
                        value={fxRate}
                        onChange={(e) => setFxRate(e.target.value)}
                        autoComplete="off"
                        placeholder="e.g. 129.45"
                        className="h-10 text-right tabular-nums"
                      />
                    )}
                  </Field>
                ) : (
                  <p className="self-end pb-2 text-xs text-muted-foreground">
                    {base} is the books’ own currency, so no exchange rate is needed.
                  </p>
                )}
              </div>
            </ReportSection>

            <ReportSection title="Coding" meta={<span>In {cur}</span>}>
              <LineEditor
                state={state}
                lines={lines}
                onChange={setLines}
                errors={lineErrors}
                currency={cur}
                accountTypes={['expense', 'asset']}
                dimensions="single"
                disabled={saving}
              />
            </ReportSection>

            <ReportSection title="Proof" meta={<span>Optional</span>}>
              <div className="space-y-4 pt-5">
                <ProofDropzone
                  accept={PROOF_ACCEPT.invoice}
                  maxSizeMB={MAX_PROOF_MB}
                  onFiles={addProofs}
                  disabled={saving || proofs.length >= MAX_PROOFS}
                  invalid={!!errors.proof}
                  describedBy={errors.proof ? 'proof-error' : undefined}
                  hint={`${describeAccepted(PROOF_ACCEPT.invoice)}, up to ${MAX_PROOF_MB} MB each. The bill records each file by name; keep the original.`}
                />
                {errors.proof && (
                  <p id="proof-error" className="text-sm text-destructive">{errors.proof}</p>
                )}
                {proofs.length > 0 && (
                  <ul className="divide-y divide-border border-b border-border">
                    {proofs.map((f, i) => (
                      <FileRow
                        key={`${f.name}-${i}`}
                        name={f.name}
                        size={f.size}
                        type={f.type}
                        status="queued"
                        onRemove={() => setProofs((prev) => prev.filter((_, j) => j !== i))}
                        removeDisabled={saving}
                      />
                    ))}
                  </ul>
                )}
              </div>
            </ReportSection>
          </fieldset>

          <div className="flex flex-col-reverse gap-2 border-t-[3px] border-foreground pt-5 sm:flex-row sm:items-center sm:justify-end">
            <Link to="/spend/bills" className={cn(buttonVariants({ variant: 'ghost', size: 'lg' }))}>
              Cancel
            </Link>
            <Button type="submit" size="lg" disabled={saving}>
              {saving ? 'Saving…' : 'Save bill'}
            </Button>
          </div>
        </form>
      )}
    </div>
  )
}
