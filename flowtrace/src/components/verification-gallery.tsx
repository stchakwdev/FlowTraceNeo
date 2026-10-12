import { useState } from 'react'
import {
  Bank,
  Buildings,
  CaretRight,
  EyeSlash,
  File,
  FilePdf,
  FileText,
  Fingerprint,
  Image as ImageIcon,
  MapPin,
  PencilLine,
  Plus,
  Receipt,
  Scroll,
  Shield,
  Video,
} from '@phosphor-icons/react'
import type { Icon } from '@phosphor-icons/react'
import type { Verification, VerificationType } from '@/lib/supabase'
import { VERIFICATION_TRUST_POINTS } from '@/lib/supabase'
import { formatDate } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { ReportNotice, StatusMark } from '@/components/report'

// Shared proof vocabulary, also used by the viewer and the upload panel.

export const PROOF_TYPE_ICON: Record<VerificationType, Icon> = {
  receipt: Receipt,
  photo: ImageIcon,
  gps: MapPin,
  signature: PencilLine,
  biometric: Fingerprint,
  bank_statement: Bank,
  document: File,
  invoice: FileText,
  video: Video,
  audit: Shield,
  contract: Scroll,
  government_doc: Buildings,
}

export const PROOF_TYPE_LABEL: Record<VerificationType, string> = {
  receipt: 'Receipt',
  photo: 'Photo',
  document: 'Document',
  bank_statement: 'Bank statement',
  signature: 'Signature',
  gps: 'GPS reading',
  biometric: 'Biometric',
  invoice: 'Invoice',
  video: 'Video',
  audit: 'Third-party audit',
  contract: 'Contract',
  government_doc: 'Government document',
}

export const PROOF_VISIBILITY_LABEL = {
  full: 'Full file',
  thumbnail: 'Thumbnail only',
  summary: 'Summary only',
  hidden: 'Hidden',
} as const

/** A proof's own state, worded for a document rather than a transaction. */
export const PROOF_STATE_LABEL: Record<Verification['status'], string> = {
  verified: 'Verified',
  pending: 'Awaiting review',
  rejected: 'Rejected',
}

export function proofTrustPoints(v: Verification) {
  return v.trust_points || VERIFICATION_TRUST_POINTS[v.type] || 0
}

/** What kind of file a proof holds, from its URL or the recorded MIME type. */
export function proofMediaKind(
  v: Verification,
): 'image' | 'pdf' | 'video' | null {
  const url = v.file_url ?? ''
  const mime =
    typeof v.metadata.mime_type === 'string' ? v.metadata.mime_type : ''
  if (/\.(jpe?g|png|gif|webp)(\?|$)/i.test(url) || mime.startsWith('image/'))
    return 'image'
  if (/\.pdf(\?|$)/i.test(url) || mime === 'application/pdf') return 'pdf'
  if (/\.(mp4|webm|mov)(\?|$)/i.test(url) || mime.startsWith('video/'))
    return 'video'
  return null
}

/** Square preview of a proof: the image itself, or its type drawn as an icon. */
export function ProofPreview({
  verification,
  className,
}: {
  verification: Verification
  className?: string
}) {
  const TypeIcon = PROOF_TYPE_ICON[verification.type]
  const kind = proofMediaKind(verification)
  const src = verification.thumbnail_url || verification.file_url
  const [broken, setBroken] = useState(false)
  return (
    <div
      className={cn(
        'relative flex shrink-0 items-center justify-center overflow-hidden border border-border bg-muted',
        className,
      )}
    >
      {kind === 'image' && src && !broken ? (
        <img
          src={src}
          alt=""
          onError={() => setBroken(true)}
          loading="lazy"
          className="size-full object-cover"
        />
      ) : kind === 'pdf' ? (
        <FilePdf className="size-6 text-muted-foreground" />
      ) : (
        <TypeIcon className="size-6 text-muted-foreground" />
      )}
    </div>
  )
}

interface VerificationGalleryProps {
  verifications: Array<Verification>
  onVerificationClick?: (verification: Verification) => void
  onAddClick?: () => void
  showAddButton?: boolean
  compact?: boolean
  maxItems?: number
  className?: string
}

/**
 * Proofs as a list of document entries: preview, type, who and when, the
 * trust points it carries and its verification state.
 */
export function VerificationGallery({
  verifications,
  onVerificationClick,
  onAddClick,
  showAddButton = true,
  compact = false,
  maxItems,
  className,
}: VerificationGalleryProps) {
  const shown = maxItems ? verifications.slice(0, maxItems) : verifications
  const hidden = verifications.length - shown.length

  if (verifications.length === 0) {
    return (
      <ReportNotice
        className={className}
        title="No proof attached yet"
        action={
          showAddButton && onAddClick ? (
            <Button onClick={onAddClick}>
              <Plus />
              Attach proof
            </Button>
          ) : undefined
        }
      >
        Receipts, photos, bank statements and GPS readings attached to this
        transaction will be listed here. Each one adds trust points.
      </ReportNotice>
    )
  }

  return (
    <div className={className}>
      <ul className="divide-y divide-border">
        {shown.map((v) => (
          <li key={v.id}>
            <ProofEntry
              verification={v}
              compact={compact}
              onClick={
                onVerificationClick ? () => onVerificationClick(v) : undefined
              }
            />
          </li>
        ))}
      </ul>
      {hidden > 0 && (
        <button
          type="button"
          onClick={() => onVerificationClick?.(verifications[shown.length])}
          className="flex min-h-10 w-full items-center justify-center gap-1 border-t border-border text-sm text-muted-foreground hover:text-foreground"
        >
          {hidden} more {hidden === 1 ? 'proof' : 'proofs'}
          <CaretRight />
        </button>
      )}
    </div>
  )
}

function ProofEntry({
  verification: v,
  compact,
  onClick,
}: {
  verification: Verification
  compact?: boolean
  onClick?: () => void
}) {
  const who = [v.executor_name, v.location_name].filter(Boolean).join(', ')
  const body = (
    <>
      <ProofPreview
        verification={v}
        className={compact ? 'size-12' : 'size-14 sm:size-16'}
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
          <p className="font-semibold">{PROOF_TYPE_LABEL[v.type]}</p>
          <p className="figure text-lg text-foreground">
            +{proofTrustPoints(v)}
            <span className="ml-1 font-sans text-xs font-normal tracking-normal text-muted-foreground font-normal-width">
              trust points
            </span>
          </p>
        </div>
        {v.description && (
          <p className="mt-0.5 truncate text-sm text-muted-foreground">
            {v.description}
          </p>
        )}
        <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <StatusMark status={v.status} label={PROOF_STATE_LABEL[v.status]} />
          <span>
            Added {formatDate(v.created_at)}
            {who && ` by ${who}`}
          </span>
          {v.visibility_level !== 'full' && (
            <span className="inline-flex items-center gap-1">
              <EyeSlash />
              {PROOF_VISIBILITY_LABEL[v.visibility_level]}
            </span>
          )}
        </div>
      </div>
    </>
  )

  if (!onClick) {
    return <div className="flex gap-4 py-4">{body}</div>
  }
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={`Open ${PROOF_TYPE_LABEL[v.type]}${v.description ? `: ${v.description}` : ''}`}
      className="-mx-2 flex w-[calc(100%+1rem)] gap-4 px-2 py-4 text-left transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring"
    >
      {body}
    </button>
  )
}

// Compact inline list of proofs, for tight spaces.
interface VerificationListProps {
  verifications: Array<Verification>
  onVerificationClick?: (verification: Verification) => void
  className?: string
}

export function VerificationList({
  verifications,
  onVerificationClick,
  className,
}: VerificationListProps) {
  if (verifications.length === 0) return null
  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      {verifications.map((v) => {
        const TypeIcon = PROOF_TYPE_ICON[v.type]
        return (
          <button
            key={v.id}
            type="button"
            onClick={() => onVerificationClick?.(v)}
            className="inline-flex min-h-8 items-center gap-1.5 border border-border px-2 text-xs font-medium hover:bg-muted"
          >
            <TypeIcon />
            {PROOF_TYPE_LABEL[v.type]}
            <span className="tabular-nums text-muted-foreground">
              +{proofTrustPoints(v)}
            </span>
          </button>
        )
      })}
    </div>
  )
}

// Proof count with trust points, for compact display.
interface VerificationBadgeProps {
  count: number
  totalTrustPoints: number
  onClick?: () => void
  className?: string
}

export function VerificationBadge({
  count,
  totalTrustPoints,
  onClick,
  className,
}: VerificationBadgeProps) {
  if (count === 0) return null
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'inline-flex min-h-8 items-center gap-1.5 border border-border px-2 text-xs font-medium tabular-nums hover:bg-muted',
        className,
      )}
    >
      {count} {count === 1 ? 'proof' : 'proofs'}
      <span className="font-bold">+{totalTrustPoints}</span>
    </button>
  )
}
