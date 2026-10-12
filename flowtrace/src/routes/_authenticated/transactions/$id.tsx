import { useEffect, useRef, useState } from 'react'
import { Link, createFileRoute, useNavigate } from '@tanstack/react-router'
import { ArrowLeft, Plus } from '@phosphor-icons/react'
import type {
  TransactionWithNodes,
  Verification,
  VerificationType,
  VerificationVisibility,
} from '@/lib/supabase'
import type { UploadedFile } from '@/components/file-upload'
import { VERIFICATION_TRUST_POINTS } from '@/lib/supabase'
import { useTransaction, useVerifications } from '@/lib/queries'
import { formatAmount, formatDate, formatDateTimeEAT } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  PROOF_STATUS_LABEL,
  ReportNotice,
  ReportSection,
  StatusMark,
  proofStroke,
} from '@/components/report'
import {
  PROOF_TYPE_LABEL,
  VerificationGallery,
  proofTrustPoints,
} from '@/components/verification-gallery'
import { VerificationViewer } from '@/components/verification-viewer'
import {
  EnhancedUploadForm,
  FileUpload,
  useAttachProofs,
  useGPSCapture,
} from '@/components/file-upload'

// `attach=1` opens the attach-proof panel; `failed=n` reports uploads that
// did not complete when the transaction was created.
type DetailSearch = { attach?: 1; failed?: number }

export const Route = createFileRoute('/_authenticated/transactions/$id')({
  validateSearch: (search: Record<string, unknown>): DetailSearch => {
    const failed = Number(search.failed)
    return {
      attach:
        search.attach === true || search.attach === '1' || search.attach === 1
          ? 1
          : undefined,
      failed: Number.isFinite(failed) && failed > 0 ? failed : undefined,
    }
  },
  component: TransactionDetail,
})

function TransactionDetail() {
  const { id } = Route.useParams()
  const { attach, failed } = Route.useSearch()
  const navigate = useNavigate({ from: Route.fullPath })
  const [selected, setSelected] = useState<Verification | null>(null)

  const { data: transaction, isLoading, error, refetch } = useTransaction(id)
  const {
    data: verifications,
    isLoading: proofsLoading,
    error: proofsError,
    refetch: refetchProofs,
  } = useVerifications(id)

  const setAttach = (open: boolean) =>
    navigate({
      search: (prev) => ({
        attach: open ? 1 : undefined,
        failed: open ? prev.failed : undefined,
      }),
      replace: true,
      resetScroll: false,
    })

  if (isLoading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
        <div className="skeleton-shimmer h-4 w-28" />
        <div className="skeleton-shimmer mt-6 h-12 w-2/3" />
        <div className="skeleton-shimmer mt-8 h-16 w-64" />
        <div className="skeleton-shimmer mt-10 h-40 w-full" />
      </div>
    )
  }

  if (error || !transaction) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
        <BackLink />
        <ReportNotice
          tone={error ? 'error' : 'neutral'}
          className="rule-heavy mt-6"
          title={
            error
              ? 'This transaction could not be loaded'
              : 'Transaction not found'
          }
          action={
            error ? (
              <Button variant="outline" onClick={() => refetch()}>
                Try again
              </Button>
            ) : (
              <Link to="/transactions" className={buttonVariants()}>
                See all transactions
              </Link>
            )
          }
        >
          {error
            ? 'Check your connection, then try again.'
            : 'It may have been removed, or the link is incomplete.'}
        </ReportNotice>
      </div>
    )
  }

  const proofs = verifications ?? []
  const trustPoints = proofs.reduce((sum, v) => sum + proofTrustPoints(v), 0)

  return (
    <div className="mx-auto max-w-6xl px-4 py-6 sm:px-8 sm:py-8">
      <BackLink />

      <header className="mt-4">
        <h1 className="text-4xl text-balance sm:text-5xl">
          {transaction.description}
        </h1>
        <p className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-muted-foreground">
          <StatusMark status={transaction.verification_status} />
          <time dateTime={transaction.transaction_date}>
            {formatDate(transaction.transaction_date)}
          </time>
          {transaction.reference && (
            <span className="font-mono text-xs">{transaction.reference}</span>
          )}
        </p>
      </header>

      <div className="mt-8 grid gap-x-10 gap-y-10 lg:grid-cols-12">
        <div className="min-w-0 space-y-10 lg:col-span-8">
          <ReportSection title="Movement">
            <div className="py-5">
              <p className="figure text-5xl sm:text-6xl">
                <span className="mr-2 text-2xl text-muted-foreground sm:text-3xl">
                  {transaction.currency}
                </span>
                {formatAmount(transaction.amount)}
              </p>
              {transaction.amount_usd != null && (
                <p className="mt-2 text-sm text-muted-foreground tabular-nums">
                  about USD {formatAmount(transaction.amount_usd)}
                </p>
              )}
              <FlowDrawing transaction={transaction} />
            </div>
          </ReportSection>

          {failed && (
            <ReportNotice
              tone="error"
              className="border border-destructive/40 py-4"
              title={`${failed} ${failed === 1 ? 'file' : 'files'} did not upload`}
              action={
                <Button variant="outline" onClick={() => setAttach(true)}>
                  Attach proof again
                </Button>
              }
            >
              The transaction was saved. Attach the missing proof below.
            </ReportNotice>
          )}

          <ReportSection
            title="Proofs"
            meta={
              proofs.length > 0 && (
                <span className="tabular-nums">
                  {proofs.length} {proofs.length === 1 ? 'proof' : 'proofs'} · +
                  {trustPoints} trust points
                </span>
              )
            }
            actions={
              !attach &&
              proofs.length > 0 && (
                <Button size="sm" onClick={() => setAttach(true)}>
                  <Plus />
                  Attach proof
                </Button>
              )
            }
          >
            {attach && (
              <AttachProof
                organizationId={transaction.organization_id}
                transactionId={id}
                onDone={() => setAttach(false)}
              />
            )}
            {proofsLoading ? (
              <div className="space-y-3 py-4">
                <div className="skeleton-shimmer h-16 w-full" />
                <div className="skeleton-shimmer h-16 w-full" />
              </div>
            ) : proofsError ? (
              <ReportNotice
                tone="error"
                title="Proofs could not be loaded"
                action={
                  <Button variant="outline" onClick={() => refetchProofs()}>
                    Try again
                  </Button>
                }
              >
                The transaction loaded, but its attached proof did not. Check
                your connection, then try again.
              </ReportNotice>
            ) : (
              <VerificationGallery
                verifications={proofs}
                onVerificationClick={setSelected}
                onAddClick={attach ? undefined : () => setAttach(true)}
                showAddButton={!attach}
              />
            )}
          </ReportSection>
        </div>

        <aside className="min-w-0 space-y-10 lg:col-span-4">
          <ReportSection title="Record">
            <dl className="divide-y divide-border text-sm">
              <Fact term="Date">
                {formatDate(transaction.transaction_date)}
              </Fact>
              <Fact term="Reference">
                {transaction.reference ? (
                  <span className="font-mono text-xs">
                    {transaction.reference}
                  </span>
                ) : (
                  <span className="text-muted-foreground">None recorded</span>
                )}
              </Fact>
              <Fact term="Category">
                {transaction.category ?? (
                  <span className="text-muted-foreground">Uncategorised</span>
                )}
              </Fact>
              <Fact term="Status">
                {PROOF_STATUS_LABEL[transaction.verification_status]}
              </Fact>
              <Fact term="Recorded">
                {formatDateTimeEAT(transaction.created_at)}
              </Fact>
              {transaction.updated_at !== transaction.created_at && (
                <Fact term="Last changed">
                  {formatDateTimeEAT(transaction.updated_at)}
                </Fact>
              )}
            </dl>
          </ReportSection>

          <ReportSection title="Notes">
            <p
              className={cn(
                'max-w-prose py-3 text-sm leading-relaxed',
                !transaction.notes && 'text-muted-foreground',
              )}
            >
              {transaction.notes ?? 'No notes recorded.'}
            </p>
          </ReportSection>
        </aside>
      </div>

      {selected && (
        <VerificationViewer
          verification={selected}
          verifications={proofs}
          onClose={() => setSelected(null)}
          onNavigate={setSelected}
        />
      )}
    </div>
  )
}

function BackLink() {
  return (
    <Link
      to="/transactions"
      className="inline-flex min-h-10 items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
    >
      <ArrowLeft className="size-4" />
      Transactions
    </Link>
  )
}

function Fact({ term, children }: { term: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[7rem_1fr] gap-3 py-2.5">
      <dt className="text-muted-foreground">{term}</dt>
      <dd className="min-w-0 font-medium">{children}</dd>
    </div>
  )
}

/** From → to, joined by a line whose pattern carries the proof state. */
function FlowDrawing({ transaction }: { transaction: TransactionWithNodes }) {
  const status = transaction.verification_status
  const stroke = proofStroke(status)
  const node = (
    n: TransactionWithNodes['from_node'],
    role: string,
    align: 'left' | 'right',
  ) => (
    <div className={cn('min-w-0', align === 'right' && 'sm:text-right')}>
      <p className="text-xs text-muted-foreground">{role}</p>
      <p className="mt-0.5 font-semibold text-balance">
        {n?.name ?? 'Not assigned'}
      </p>
      {n?.type && (
        <p className="text-xs text-muted-foreground capitalize">
          {n.type.replace(/_/g, ' ')}
        </p>
      )}
    </div>
  )

  const line = (vertical: boolean) => (
    <svg
      aria-hidden="true"
      className={cn(
        'overflow-visible',
        vertical ? 'ml-1 h-10 w-3 sm:hidden' : 'hidden h-4 w-full sm:block',
      )}
    >
      <line
        x1={vertical ? 6 : 0}
        y1={vertical ? 0 : 8}
        x2={vertical ? 6 : '100%'}
        y2={vertical ? '100%' : 8}
        strokeWidth="4"
        strokeDasharray={stroke.strokeDasharray}
        className={stroke.className}
      />
      {status === 'flagged' && (
        <g
          className="stroke-flag"
          strokeWidth="2.5"
          transform={vertical ? 'translate(6 20)' : undefined}
        >
          {vertical ? (
            <path d="M-5 -5 L5 5 M5 -5 L-5 5" />
          ) : (
            <svg x="50%" y="8" overflow="visible">
              <path d="M-5 -5 L5 5 M5 -5 L-5 5" />
            </svg>
          )}
        </g>
      )}
    </svg>
  )

  return (
    <div
      className="mt-6 grid gap-1 border-t border-border pt-5 sm:grid-cols-[minmax(0,1fr)_minmax(6rem,1.4fr)_minmax(0,1fr)] sm:items-center sm:gap-5"
      aria-label={`${transaction.from_node?.name ?? 'Not assigned'} to ${transaction.to_node?.name ?? 'Not assigned'}, ${PROOF_STATUS_LABEL[status].toLowerCase()}`}
      role="group"
    >
      {node(transaction.from_node, 'From', 'left')}
      <div className="flex flex-col sm:items-stretch">
        <div className="flex items-center gap-3 sm:hidden">
          {line(true)}
          <span className="text-xs text-muted-foreground">
            {PROOF_STATUS_LABEL[status]}
          </span>
        </div>
        {line(false)}
        <p className="mt-1 hidden text-center text-xs text-muted-foreground sm:block">
          {PROOF_STATUS_LABEL[status]}
        </p>
      </div>
      {node(transaction.to_node, 'To', 'right')}
    </div>
  )
}

const ATTACH_TYPES: Array<VerificationType> = [
  'receipt',
  'invoice',
  'photo',
  'bank_statement',
  'contract',
  'signature',
  'gps',
  'video',
  'audit',
  'government_doc',
  'document',
]

/** Inline panel: choose the kind of proof, add the file and its details. */
function AttachProof({
  organizationId,
  transactionId,
  onDone,
}: {
  organizationId: string
  transactionId: string
  onDone: () => void
}) {
  const [type, setType] = useState<VerificationType>('receipt')
  const [uploads, setUploads] = useState<Array<UploadedFile>>([])
  const [description, setDescription] = useState('')
  const [executorName, setExecutorName] = useState('')
  const [visibility, setVisibility] = useState<VerificationVisibility>('full')
  const [problem, setProblem] = useState<string | null>(null)
  const gps = useGPSCapture()
  const attachProofs = useAttachProofs()
  const panelRef = useRef<HTMLDivElement>(null)
  const saving = attachProofs.isPending

  useEffect(() => {
    panelRef.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    panelRef.current?.querySelector('select')?.focus({ preventScroll: true })
  }, [])

  const save = async () => {
    const gpsOnly = type === 'gps' && gps.location
    if (uploads.length === 0 && !gpsOnly) {
      setProblem(
        type === 'gps'
          ? 'Add a file or your current location before attaching.'
          : 'Add at least one file before attaching.',
      )
      return
    }
    setProblem(null)
    try {
      await attachProofs.mutateAsync({
        transactionId,
        type,
        uploaded: uploads,
        visibilityLevel: visibility,
        executorName: executorName.trim() || undefined,
        description: description.trim() || undefined,
        location: gps.location,
      })
      onDone()
    } catch (err) {
      setProblem(
        `The proof was not attached${
          err instanceof Error && err.message ? `: ${err.message}` : ''
        }. Your files are still here; try again.`,
      )
    }
  }

  return (
    <div
      ref={panelRef}
      className="my-4 border border-foreground/30 bg-card p-4 sm:p-5"
      role="region"
      aria-label="Attach proof"
    >
      <div className="flex items-baseline justify-between gap-3">
        <h3 className="text-2xl">Attach proof</h3>
        <p className="text-xs text-muted-foreground tabular-nums">
          +{VERIFICATION_TRUST_POINTS[type]} trust points each
        </p>
      </div>

      <div className="mt-4 space-y-5">
        <div>
          <Label htmlFor="proof-type" className="mb-1.5 text-sm font-medium">
            Kind of proof
          </Label>
          <Select
            value={type}
            disabled={saving || uploads.length > 0}
            items={Object.fromEntries(
              ATTACH_TYPES.map((t) => [
                t,
                `${PROOF_TYPE_LABEL[t]} (+${VERIFICATION_TRUST_POINTS[t]})`,
              ]),
            )}
            onValueChange={(v) => v && setType(v as VerificationType)}
          >
            <SelectTrigger id="proof-type" className="h-10 w-full sm:max-w-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ATTACH_TYPES.map((t) => (
                <SelectItem key={t} value={t}>
                  {PROOF_TYPE_LABEL[t]} (+{VERIFICATION_TRUST_POINTS[t]})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {uploads.length > 0 && (
            <p className="mt-1 text-xs text-muted-foreground">
              Remove the files to change the kind of proof.
            </p>
          )}
        </div>

        <FileUpload
          key={type}
          organizationId={organizationId}
          transactionId={transactionId}
          verificationType={type}
          disabled={saving}
          onUploadComplete={(file) => {
            setUploads((prev) => [...prev, file])
            setProblem(null)
          }}
          onFileRemove={(fileId) =>
            setUploads((prev) => prev.filter((f) => f.id !== fileId))
          }
        />

        <EnhancedUploadForm
          verificationType={type}
          visibilityLevel={visibility}
          onVisibilityChange={setVisibility}
          executorName={executorName}
          onExecutorNameChange={setExecutorName}
          description={description}
          onDescriptionChange={setDescription}
          gpsLocation={gps.location}
          gpsLoading={gps.loading}
          gpsError={gps.error}
          onCaptureGPS={gps.captureLocation}
          onClearGPS={gps.clearLocation}
          disabled={saving}
        />

        {problem && (
          <p role="alert" className="text-sm font-medium text-destructive">
            {problem}
          </p>
        )}

        <div className="flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:justify-end">
          <Button
            type="button"
            variant="ghost"
            className="h-10"
            onClick={onDone}
            disabled={saving}
          >
            Cancel
          </Button>
          <Button
            type="button"
            className="h-10"
            onClick={save}
            disabled={saving}
          >
            {saving
              ? 'Attaching…'
              : uploads.length > 1
                ? `Attach ${uploads.length} proofs`
                : 'Attach proof'}
          </Button>
        </div>
      </div>
    </div>
  )
}
