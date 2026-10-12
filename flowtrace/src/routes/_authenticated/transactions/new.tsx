import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { useId, useMemo, useRef, useState } from 'react'
import { ArrowLeft } from '@phosphor-icons/react'
import type { FlowNode, VerificationType } from '@/lib/supabase'
import { VERIFICATION_TRUST_POINTS } from '@/lib/supabase'
import { useCreateTransaction, useNodes } from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import { formatAmount } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  DateField,
  isoToDayMonthYear,
  localTodayISO,
  parseDayMonthYear,
} from '@/components/ui/date-field'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { DEV_MODE } from '@/lib/auth'
import { transactions as demoTransactions } from '@/lib/demo/dataset'
import { Textarea } from '@/components/ui/textarea'
import { ReportSection } from '@/components/report'
import {
  FileRow,
  PROOF_ACCEPT,
  ProofDropzone,
  describeAccepted,
  useAttachProofs,
  validateProofFile,
} from '@/components/file-upload'
import { PROOF_TYPE_LABEL } from '@/components/verification-gallery'

export const Route = createFileRoute('/_authenticated/transactions/new')({
  component: NewTransaction,
})

const CATEGORIES = [
  'Funding',
  'Budget',
  'Programs',
  'Operations',
  'Salaries',
  'Transport',
  'Equipment',
  'Training',
  'Supplies',
  'Utilities',
  'Other',
]

const CURRENCIES = [
  ['KES', 'Kenyan shilling'],
  ['USD', 'US dollar'],
  ['UGX', 'Ugandan shilling'],
  ['TZS', 'Tanzanian shilling'],
  ['NGN', 'Nigerian naira'],
] as const

const PROOF_TYPES: Array<VerificationType> = [
  'receipt',
  'invoice',
  'bank_statement',
  'photo',
  'contract',
  'document',
]

const NODE_TYPE_LABEL: Record<FlowNode['type'], string> = {
  donor: 'Donors',
  account: 'Accounts',
  department: 'Departments',
  project: 'Projects',
  vendor: 'Vendors',
  beneficiary: 'Beneficiaries',
}

const MAX_PROOF_FILES = 5
const MAX_PROOF_MB = 10

type Fields = {
  description: string
  category: string
  fromNodeId: string
  toNodeId: string
  amount: string
  currency: string
  date: string
  reference: string
  notes: string
}
type FieldName = keyof Fields
type Errors = Partial<Record<FieldName | 'proof', string>>

// Order the summary lists errors in, matching the form.
const FIELD_ORDER: Array<FieldName> = [
  'description',
  'category',
  'fromNodeId',
  'toNodeId',
  'amount',
  'currency',
  'date',
  'reference',
  'notes',
]

const FIELD_LABEL: Record<FieldName, string> = {
  description: 'Description',
  category: 'Category',
  fromNodeId: 'From',
  toNodeId: 'To',
  amount: 'Amount',
  currency: 'Currency',
  date: 'Date',
  reference: 'Reference',
  notes: 'Notes',
}

// The demo is a snapshot "as of" its latest movement, so a new entry defaults
// to that day; a live organisation defaults to today in the viewer's timezone.
function defaultDateISO() {
  if (!DEV_MODE) return localTodayISO()
  let latest = ''
  for (const t of demoTransactions) {
    if (t.transaction_date > latest) latest = t.transaction_date
  }
  return latest.slice(0, 10) || localTodayISO()
}

function parseAmount(raw: string) {
  const cleaned = raw.replace(/[,\s]/g, '')
  if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) return NaN
  return Number(cleaned)
}

function validate(f: Fields): Errors {
  const e: Errors = {}
  if (!f.description.trim()) e.description = 'Describe what this money was for.'
  else if (f.description.trim().length < 3)
    e.description = 'Use at least 3 characters so it can be found later.'
  if (!f.fromNodeId) e.fromNodeId = 'Choose where the money came from.'
  if (!f.toNodeId) e.toNodeId = 'Choose where the money went.'
  else if (f.toNodeId === f.fromNodeId)
    e.toNodeId = 'Money must move between two different nodes.'
  if (!f.amount.trim()) e.amount = 'Enter the amount that moved.'
  else {
    const n = parseAmount(f.amount)
    if (Number.isNaN(n)) e.amount = 'Enter a number, like 250000 or 1,250.50.'
    else if (n <= 0) e.amount = 'The amount must be more than zero.'
  }
  const date = parseDayMonthYear(f.date)
  if (!date) e.date = 'Enter the date the money moved.'
  else if (date.iso === undefined) e.date = date.error
  else if (date.iso > localTodayISO())
    e.date = 'The date cannot be in the future.'
  if (f.reference.length > 80)
    e.reference = 'Keep the reference under 80 characters.'
  return e
}

function NewTransaction() {
  const navigate = useNavigate()
  const { organization } = useOrg()
  const {
    data: nodes,
    isLoading: nodesLoading,
    error: nodesError,
    refetch: refetchNodes,
  } = useNodes()
  const createTransaction = useCreateTransaction()
  const attachProofs = useAttachProofs()

  const [fields, setFields] = useState<Fields>({
    description: '',
    category: '',
    fromNodeId: '',
    toNodeId: '',
    amount: '',
    currency: organization?.currency ?? 'KES',
    date: isoToDayMonthYear(defaultDateISO()),
    reference: '',
    notes: '',
  })
  const [errors, setErrors] = useState<Errors>({})
  const [submitted, setSubmitted] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [proofType, setProofType] = useState<VerificationType>('receipt')
  const [proofFiles, setProofFiles] = useState<Array<File>>([])
  const summaryRef = useRef<HTMLDivElement>(null)

  const saving = createTransaction.isPending || attachProofs.isPending

  const nodeGroups = useMemo(() => {
    const groups = new Map<FlowNode['type'], Array<FlowNode>>()
    for (const n of nodes ?? []) {
      const list = groups.get(n.type) ?? []
      list.push(n)
      groups.set(n.type, list)
    }
    return [...groups.entries()]
  }, [nodes])

  const set = (name: FieldName, value: string) => {
    const next = { ...fields, [name]: value }
    setFields(next)
    // After a failed submit, errors update as they are fixed.
    if (submitted) {
      const proof = errors.proof
      setErrors({ ...validate(next), ...(proof ? { proof } : {}) })
    }
  }

  const addProofFiles = (chosen: Array<File>) => {
    const accept = PROOF_ACCEPT[proofType]
    const problems: Array<string> = []
    const ok: Array<File> = []
    for (const file of chosen) {
      const problem = validateProofFile(file, accept, MAX_PROOF_MB)
      if (problem) problems.push(problem)
      else ok.push(file)
    }
    const room = MAX_PROOF_FILES - proofFiles.length
    if (ok.length > room)
      problems.push(`Only ${MAX_PROOF_FILES} files can be added at once.`)
    setProofFiles((prev) => [...prev, ...ok.slice(0, Math.max(room, 0))])
    setErrors((prev) => ({
      ...prev,
      proof: problems.length ? problems.join(' ') : undefined,
    }))
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (saving) return
    setSubmitted(true)
    setSaveError(null)
    const found = validate(fields)
    setErrors(found)
    if (Object.keys(found).length > 0) {
      requestAnimationFrame(() => summaryRef.current?.focus())
      return
    }

    let created
    try {
      created = await createTransaction.mutateAsync({
        description: fields.description.trim(),
        amount: parseAmount(fields.amount),
        currency: fields.currency,
        from_node_id: fields.fromNodeId,
        to_node_id: fields.toNodeId,
        category: fields.category || null,
        transaction_date: parseDayMonthYear(fields.date)?.iso ?? '',
        notes: fields.notes.trim() || null,
        reference: fields.reference.trim() || null,
        exchange_rate: null,
        amount_usd: null,
        verification_status: 'pending',
        created_by: null,
      })
    } catch (err) {
      setSaveError(
        `The transaction was not saved${
          err instanceof Error && err.message ? `: ${err.message}` : ''
        }. Your entries are still here; try again.`,
      )
      requestAnimationFrame(() => summaryRef.current?.focus())
      return
    }

    let failed = 0
    if (proofFiles.length > 0) {
      try {
        const result = await attachProofs.mutateAsync({
          transactionId: created.id,
          type: proofType,
          files: proofFiles,
        })
        failed = result.failed
      } catch {
        failed = proofFiles.length
      }
    }

    navigate({
      to: '/transactions/$id',
      params: { id: created.id },
      search: failed ? { failed } : {},
    })
  }

  const errorList = FIELD_ORDER.filter((f) => errors[f])
  const showSummary =
    saveError !== null ||
    (submitted && (errorList.length > 0 || !!errors.proof))

  const amountPreview = parseAmount(fields.amount)

  return (
    <div className="mx-auto max-w-3xl px-4 py-6 sm:px-8 sm:py-8">
      <Link
        to="/transactions"
        className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Transactions
      </Link>
      <h1 className="mt-4 text-4xl sm:text-5xl">Record a transaction</h1>
      <p className="mt-2 max-w-prose text-sm text-muted-foreground">
        Record one movement of money between two nodes. It is marked as awaiting
        proof until receipts or statements are attached.
      </p>

      <form noValidate onSubmit={handleSubmit} className="mt-8 space-y-10">
        {showSummary && (
          <div
            ref={summaryRef}
            tabIndex={-1}
            role="alert"
            aria-labelledby="error-summary-title"
            className="border border-destructive p-4 outline-none focus-visible:ring-2 focus-visible:ring-destructive/40"
          >
            <p
              id="error-summary-title"
              className="font-semibold text-destructive"
            >
              {saveError
                ? 'Not saved'
                : `Fix ${errorList.length + (errors.proof ? 1 : 0) === 1 ? 'this' : 'these'} before saving`}
            </p>
            {saveError ? (
              <p className="mt-1 text-sm">{saveError}</p>
            ) : (
              <ul className="mt-2 space-y-1 text-sm">
                {errorList.map((f) => (
                  <li key={f}>
                    <a
                      href={`#${f}`}
                      onClick={(e) => {
                        e.preventDefault()
                        document.getElementById(f)?.focus()
                      }}
                      className="underline underline-offset-4 hover:text-destructive"
                    >
                      {FIELD_LABEL[f]}: {errors[f]}
                    </a>
                  </li>
                ))}
                {errors.proof && <li>Proof: {errors.proof}</li>}
              </ul>
            )}
          </div>
        )}

        <fieldset disabled={saving} className="min-w-0 space-y-10">
          <ReportSection title="What moved">
            <div className="grid gap-5 pt-5 sm:grid-cols-[2fr_1fr]">
              <Field
                id="description"
                label="Description"
                required
                error={errors.description}
                hint="As it should read in the report, e.g. Borehole drilling, phase 1"
              >
                {(props) => (
                  <Input
                    {...props}
                    value={fields.description}
                    onChange={(e) => set('description', e.target.value)}
                    autoComplete="off"
                    className="h-10"
                  />
                )}
              </Field>
              <Field id="category" label="Category" error={errors.category}>
                {(props) => (
                  <FormSelect
                    {...props}
                    value={fields.category}
                    onValueChange={(v) => set('category', v)}
                    options={[
                      { value: '', label: 'Uncategorised' },
                      ...CATEGORIES.map((c) => ({ value: c, label: c })),
                    ]}
                  />
                )}
              </Field>
            </div>
          </ReportSection>

          <ReportSection title="Between which nodes">
            {nodesError ? (
              <div role="alert" className="pt-5 text-sm">
                <p className="font-semibold text-destructive">
                  Nodes could not be loaded
                </p>
                <p className="mt-1 text-muted-foreground">
                  The From and To lists need your organisation's nodes.{' '}
                  <button
                    type="button"
                    onClick={() => refetchNodes()}
                    className="font-semibold text-foreground underline underline-offset-4"
                  >
                    Try again
                  </button>
                </p>
              </div>
            ) : !nodesLoading && (nodes?.length ?? 0) < 2 ? (
              <div className="pt-5 text-sm">
                <p className="font-semibold">Add nodes first</p>
                <p className="mt-1 text-muted-foreground">
                  A transaction moves money between two nodes, such as a donor
                  and a project. Add them on the{' '}
                  <Link
                    to="/flows"
                    className="font-semibold text-foreground underline underline-offset-4"
                  >
                    Flows
                  </Link>{' '}
                  page.
                </p>
              </div>
            ) : (
              <div className="grid gap-5 pt-5 sm:grid-cols-2">
                {(['fromNodeId', 'toNodeId'] as const).map((name) => (
                  <Field
                    key={name}
                    id={name}
                    label={name === 'fromNodeId' ? 'From' : 'To'}
                    required
                    error={errors[name]}
                  >
                    {(props) => (
                      <FormSelect
                        {...props}
                        value={fields[name]}
                        disabled={nodesLoading}
                        onValueChange={(v) => set(name, v)}
                        placeholder={
                          nodesLoading
                            ? 'Loading nodes…'
                            : name === 'fromNodeId'
                              ? 'Choose the source'
                              : 'Choose the destination'
                        }
                        groups={nodeGroups.map(([type, list]) => ({
                          label: NODE_TYPE_LABEL[type],
                          options: list.map((n) => ({
                            value: n.id,
                            label: n.name,
                          })),
                        }))}
                      />
                    )}
                  </Field>
                ))}
              </div>
            )}
          </ReportSection>

          <ReportSection title="Amount and date">
            <div className="grid gap-5 pt-5 sm:grid-cols-[1fr_14rem]">
              <Field
                id="amount"
                label="Amount"
                required
                error={errors.amount}
                hint={
                  !errors.amount && amountPreview > 0
                    ? `${fields.currency} ${formatAmount(amountPreview, amountPreview % 1 ? 2 : 0)}`
                    : undefined
                }
              >
                {(props) => (
                  <div className="relative">
                    <span
                      aria-hidden="true"
                      className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-sm font-medium text-muted-foreground"
                    >
                      {fields.currency}
                    </span>
                    <Input
                      {...props}
                      inputMode="decimal"
                      value={fields.amount}
                      onChange={(e) => set('amount', e.target.value)}
                      placeholder="e.g. 250,000"
                      autoComplete="off"
                      className="h-12 pl-14 text-right text-lg tabular-nums placeholder:text-sm md:text-lg"
                    />
                  </div>
                )}
              </Field>
              <Field
                id="currency"
                label="Currency"
                fixed
                error={errors.currency}
              >
                {(props) => (
                  <FormSelect
                    {...props}
                    value={fields.currency}
                    onValueChange={(v) => set('currency', v)}
                    className="h-12 data-[size=default]:h-12"
                    options={CURRENCIES.map(([code, name]) => ({
                      value: code,
                      label: `${code} · ${name}`,
                    }))}
                  />
                )}
              </Field>
              <Field
                id="date"
                label="Date"
                required
                error={errors.date}
                hint="Day, month, year"
              >
                {(props) => (
                  <DateField
                    {...props}
                    max={localTodayISO()}
                    value={fields.date}
                    onValueChange={(v) => set('date', v)}
                    pickerLabel="Choose the date from a calendar"
                    className="h-10"
                    wrapperClassName="sm:max-w-56"
                  />
                )}
              </Field>
              <div className="hidden sm:block" />
              <Field
                id="reference"
                label="Reference"
                error={errors.reference}
                hint="Bank, M-Pesa or invoice reference, if there is one"
              >
                {(props) => (
                  <Input
                    {...props}
                    value={fields.reference}
                    onChange={(e) => set('reference', e.target.value)}
                    autoComplete="off"
                    className="h-10 font-mono"
                  />
                )}
              </Field>
            </div>
          </ReportSection>

          <ReportSection title="Notes">
            <div className="pt-5">
              <Field
                id="notes"
                label="Notes"
                srOnlyLabel
                error={errors.notes}
                hint="Anything a funder or auditor should know about this movement"
              >
                {(props) => (
                  <Textarea
                    {...props}
                    rows={3}
                    value={fields.notes}
                    onChange={(e) => set('notes', e.target.value)}
                  />
                )}
              </Field>
            </div>
          </ReportSection>

          <ReportSection
            title="Proof"
            meta={<span>Optional · you can attach it later</span>}
          >
            <div className="space-y-4 pt-5">
              <Field id="proofType" label="Kind of proof" fixed>
                {(props) => (
                  <FormSelect
                    {...props}
                    value={proofType}
                    disabled={proofFiles.length > 0}
                    onValueChange={(v) => setProofType(v as VerificationType)}
                    className="sm:max-w-sm"
                    options={PROOF_TYPES.map((t) => ({
                      value: t,
                      label: `${PROOF_TYPE_LABEL[t]} (+${VERIFICATION_TRUST_POINTS[t]} trust points)`,
                    }))}
                  />
                )}
              </Field>
              <ProofDropzone
                accept={PROOF_ACCEPT[proofType]}
                maxSizeMB={MAX_PROOF_MB}
                onFiles={addProofFiles}
                disabled={saving || proofFiles.length >= MAX_PROOF_FILES}
                invalid={!!errors.proof}
                describedBy={errors.proof ? 'proof-error' : undefined}
                hint={`${describeAccepted(PROOF_ACCEPT[proofType])}, up to ${MAX_PROOF_MB} MB each. Files upload when you save.`}
              />
              {errors.proof && (
                <p id="proof-error" className="text-sm text-destructive">
                  {errors.proof}
                </p>
              )}
              {proofFiles.length > 0 && (
                <ul className="divide-y divide-border border-b border-border">
                  {proofFiles.map((file, i) => (
                    <FileRow
                      key={`${file.name}-${i}`}
                      name={file.name}
                      size={file.size}
                      type={file.type}
                      status={attachProofs.isPending ? 'uploading' : 'queued'}
                      onRemove={() =>
                        setProofFiles((prev) => prev.filter((_, j) => j !== i))
                      }
                      removeDisabled={saving}
                    />
                  ))}
                </ul>
              )}
            </div>
          </ReportSection>
        </fieldset>

        <div className="flex flex-col-reverse gap-2 border-t-[3px] border-foreground pt-5 sm:flex-row sm:items-center sm:justify-end">
          <Link
            to="/transactions"
            className={cn(buttonVariants({ variant: 'ghost', size: 'lg' }))}
          >
            Cancel
          </Link>
          <Button type="submit" size="lg" disabled={saving}>
            {createTransaction.isPending
              ? 'Saving…'
              : attachProofs.isPending
                ? 'Uploading proof…'
                : 'Save transaction'}
          </Button>
        </div>
      </form>
    </div>
  )
}

type ControlProps = {
  id: string
  'aria-invalid'?: true
  'aria-describedby'?: string
  'aria-required'?: true
}

function Field({
  id,
  label,
  required,
  error,
  hint,
  srOnlyLabel,
  fixed,
  children,
}: {
  id: string
  label: string
  required?: boolean
  error?: string
  hint?: string
  srOnlyLabel?: boolean
  /** Always has a value, so it is neither required nor optional. */
  fixed?: boolean
  children: (props: ControlProps) => React.ReactNode
}) {
  const hintId = useId()
  const describedBy = [error && `${id}-error`, hint && hintId]
    .filter(Boolean)
    .join(' ')
  return (
    <div className="min-w-0">
      <label
        htmlFor={id}
        className={cn(
          'mb-1.5 block text-sm font-medium',
          srOnlyLabel && 'sr-only',
        )}
      >
        {label}
        {required ? (
          <span className="sr-only"> (required)</span>
        ) : fixed ? null : (
          <span className="ml-1.5 text-xs font-normal text-muted-foreground">
            optional
          </span>
        )}
      </label>
      {children({
        id,
        'aria-invalid': error ? true : undefined,
        'aria-describedby': describedBy || undefined,
        'aria-required': required ? true : undefined,
      })}
      {error && (
        <p id={`${id}-error`} className="mt-1.5 text-sm text-destructive">
          {error}
        </p>
      )}
      {hint && (
        <p
          id={hintId}
          className={cn(
            'mt-1 text-xs text-muted-foreground',
            id === 'amount' && 'tabular-nums',
          )}
        >
          {hint}
        </p>
      )}
    </div>
  )
}

type Option = { value: string; label: string }

// The themed select, wired to the Field's id and aria props. An empty string
// stands for "no choice" and shows the placeholder (or a '' option's label).
function FormSelect({
  value,
  onValueChange,
  options = [],
  groups = [],
  placeholder,
  disabled,
  className,
  ...control
}: ControlProps & {
  value: string
  onValueChange: (value: string) => void
  options?: Array<Option>
  groups?: Array<{ label: string; options: Array<Option> }>
  placeholder?: string
  disabled?: boolean
  className?: string
}) {
  const all = [...options, ...groups.flatMap((g) => g.options)]
  const labelOf = (v: string) => all.find((o) => o.value === v)?.label
  const item = (o: Option) => (
    <SelectItem key={o.value || '__none'} value={o.value || null}>
      {o.label}
    </SelectItem>
  )
  return (
    <Select
      value={value || null}
      disabled={disabled}
      onValueChange={(v) => onValueChange(v ?? '')}
    >
      <SelectTrigger {...control} className={cn('h-10 w-full', className)}>
        <SelectValue>
          {(v: string | null) => labelOf(v ?? '') ?? placeholder ?? ''}
        </SelectValue>
      </SelectTrigger>
      <SelectContent>
        {options.map(item)}
        {groups.map((g) => (
          <SelectGroup key={g.label}>
            <SelectLabel>{g.label}</SelectLabel>
            {g.options.map(item)}
          </SelectGroup>
        ))}
      </SelectContent>
    </Select>
  )
}
