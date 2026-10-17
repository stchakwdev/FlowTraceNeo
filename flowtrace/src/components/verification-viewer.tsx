import { useEffect, useRef, useState } from 'react'
import {
  CaretLeft,
  CaretRight,
  DownloadSimple,
  MagnifyingGlassMinus,
  MagnifyingGlassPlus,
  SpinnerGap,
  Warning,
  X,
} from '@phosphor-icons/react'
import type { Verification } from '@/lib/supabase'
import { formatDate, formatDateTime } from '@/lib/format'
import { useProofSrc } from '@/lib/proofs'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/report'
import {
  PROOF_STATE_LABEL,
  PROOF_TYPE_ICON,
  PROOF_TYPE_LABEL,
  PROOF_VISIBILITY_LABEL,
  proofMediaKind,
  proofTrustPoints,
} from '@/components/verification-gallery'

const VISIBILITY_DESCRIPTION = {
  full: 'Board and staff see the original file.',
  thumbnail: 'Donors see a low-resolution version.',
  summary: 'Partners see that proof is attached, not the file.',
  hidden: 'Not shown outside the organisation.',
} as const

interface VerificationViewerProps {
  verification: Verification | null
  verifications?: Array<Verification>
  onClose: () => void
  onNavigate?: (verification: Verification) => void
  className?: string
}

/** Full-screen proof viewer: the file on the left, its record on the right. */
export function VerificationViewer({
  verification,
  verifications = [],
  onClose,
  onNavigate,
  className,
}: VerificationViewerProps) {
  const [zoom, setZoom] = useState(1)
  const dialogRef = useRef<HTMLDivElement>(null)
  const closeRef = useRef<HTMLButtonElement>(null)

  const index = verification
    ? verifications.findIndex((v) => v.id === verification.id)
    : -1
  const prev = index > 0 ? verifications[index - 1] : null
  const next =
    index >= 0 && index < verifications.length - 1
      ? verifications[index + 1]
      : null

  // Latest handlers, read by the key listener without re-subscribing.
  const keys = useRef({ prev, next, onClose, onNavigate })
  keys.current = { prev, next, onClose, onNavigate }
  const isOpen = verification !== null

  // Focus the dialog, trap Tab inside it, close on Escape, arrows navigate.
  useEffect(() => {
    if (!isOpen) return
    const opener = document.activeElement as HTMLElement | null
    closeRef.current?.focus()
    const onKey = (e: KeyboardEvent) => {
      const k = keys.current
      if (e.key === 'Escape') k.onClose()
      else if (e.key === 'ArrowLeft' && k.prev && k.onNavigate)
        k.onNavigate(k.prev)
      else if (e.key === 'ArrowRight' && k.next && k.onNavigate)
        k.onNavigate(k.next)
      else if (e.key === 'Tab' && dialogRef.current) {
        const items = dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), a[href], iframe, video',
        )
        if (items.length === 0) return
        const first = items[0]
        const last = items[items.length - 1]
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault()
          last.focus()
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault()
          first.focus()
        }
      }
    }
    document.addEventListener('keydown', onKey)
    const overflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      document.removeEventListener('keydown', onKey)
      document.body.style.overflow = overflow
      opener?.focus()
    }
  }, [isOpen])

  useEffect(() => setZoom(1), [verification?.id])

  // A stored file's `proof:` reference becomes a signed URL; others pass through.
  const file = useProofSrc(verification?.file_url)

  if (!verification) return null

  const v = verification
  const TypeIcon = PROOF_TYPE_ICON[v.type]
  const kind = proofMediaKind(v)
  const label = PROOF_TYPE_LABEL[v.type]

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-label={`${label}${v.description ? `: ${v.description}` : ''}`}
      className="fixed inset-0 z-50 flex flex-col bg-foreground/97 text-background lg:flex-row"
    >
      {/* The file */}
      <div className="relative flex min-h-0 flex-1 flex-col">
        <div className="flex h-14 shrink-0 items-center justify-between gap-2 px-4">
          <p className="text-sm tabular-nums">
            {verifications.length > 1 &&
              `Proof ${index + 1} of ${verifications.length}`}
          </p>
          <div className="flex items-center gap-1">
            {kind === 'image' && (
              <>
                <ViewerButton
                  label="Zoom out"
                  onClick={() => setZoom((z) => Math.max(z - 0.25, 0.5))}
                  disabled={zoom <= 0.5}
                >
                  <MagnifyingGlassMinus />
                </ViewerButton>
                <button
                  type="button"
                  onClick={() => setZoom(1)}
                  className="h-10 min-w-14 px-2 text-sm tabular-nums hover:bg-background/15"
                  aria-label="Reset zoom"
                >
                  {Math.round(zoom * 100)}%
                </button>
                <ViewerButton
                  label="Zoom in"
                  onClick={() => setZoom((z) => Math.min(z + 0.25, 3))}
                  disabled={zoom >= 3}
                >
                  <MagnifyingGlassPlus />
                </ViewerButton>
              </>
            )}
            <button
              ref={closeRef}
              type="button"
              onClick={onClose}
              aria-label="Close proof viewer"
              className="ml-2 flex size-10 items-center justify-center hover:bg-background/15 focus-visible:outline-2 focus-visible:outline-background"
            >
              <X className="size-5" />
            </button>
          </div>
        </div>

        <div className="relative flex min-h-0 flex-1 items-center justify-center overflow-auto px-4 pb-4 sm:px-16">
          {file.status === 'loading' ? (
            <div
              role="status"
              className="flex max-w-sm flex-col items-center text-center"
            >
              <SpinnerGap className="size-10 animate-spin opacity-80" />
              <p className="mt-4 text-sm opacity-80">Opening the file…</p>
            </div>
          ) : file.status === 'error' ? (
            <div className="flex max-w-sm flex-col items-center text-center">
              <Warning className="size-16 opacity-80" />
              <p className="mt-4 font-semibold">The file could not be loaded</p>
              <p className="mt-1 text-sm opacity-80">
                {file.error} Check your connection and try again.
              </p>
            </div>
          ) : kind === 'image' && file.src ? (
            <img
              src={file.src}
              alt={v.description ?? label}
              className="max-h-full max-w-full object-contain transition-transform duration-150"
              style={{ transform: `scale(${zoom})` }}
            />
          ) : kind === 'video' && file.src ? (
            <video src={file.src} controls className="max-h-full max-w-full" />
          ) : kind === 'pdf' && file.src ? (
            <iframe
              src={file.src}
              className="size-full bg-background"
              title={`${label} document`}
            />
          ) : (
            <div className="flex max-w-sm flex-col items-center text-center">
              <TypeIcon className="size-16 opacity-80" />
              <p className="mt-4 font-semibold">{label}</p>
              <p className="mt-1 text-sm opacity-80">
                {v.file_url
                  ? 'This file type cannot be previewed here. Download it to open it.'
                  : 'This proof is a record without an attached file.'}
              </p>
            </div>
          )}

          {prev && onNavigate && (
            <ViewerButton
              label="Previous proof"
              onClick={() => onNavigate(prev)}
              className="absolute top-1/2 left-2 -translate-y-1/2"
            >
              <CaretLeft className="size-6" />
            </ViewerButton>
          )}
          {next && onNavigate && (
            <ViewerButton
              label="Next proof"
              onClick={() => onNavigate(next)}
              className="absolute top-1/2 right-2 -translate-y-1/2"
            >
              <CaretRight className="size-6" />
            </ViewerButton>
          )}
        </div>
      </div>

      {/* The record */}
      <aside
        className={cn(
          'max-h-[45vh] shrink-0 overflow-y-auto border-t-[3px] border-foreground bg-background p-5 text-foreground lg:max-h-none lg:w-96 lg:border-t-0 lg:border-l lg:border-l-border',
          className,
        )}
      >
        <h2 className="text-2xl">{label}</h2>
        {v.description && (
          <p className="mt-1 text-sm text-muted-foreground">{v.description}</p>
        )}
        <div className="mt-3">
          <StatusMark status={v.status} label={PROOF_STATE_LABEL[v.status]} />
        </div>

        <p className="figure mt-5 text-4xl">+{proofTrustPoints(v)}</p>
        <p className="mt-1 text-sm text-muted-foreground">
          trust points toward the organisation's score
        </p>

        <dl className="mt-5 divide-y divide-border border-t border-border text-sm">
          {v.executor_name && <Fact term="Captured by">{v.executor_name}</Fact>}
          {(v.location_name || v.gps_latitude != null) && (
            <Fact term="Location">
              {v.location_name}
              {v.gps_latitude != null && v.gps_longitude != null && (
                <span className="block font-mono text-xs text-muted-foreground">
                  {v.gps_latitude.toFixed(5)}, {v.gps_longitude.toFixed(5)}
                  {v.gps_accuracy != null &&
                    ` (±${Math.round(v.gps_accuracy)} m)`}
                </span>
              )}
            </Fact>
          )}
          <Fact term="Added">{formatDateTime(v.created_at)}</Fact>
          {v.verified_at && v.status === 'verified' && (
            <Fact term="Verified">{formatDate(v.verified_at)}</Fact>
          )}
          <Fact term="Who can see it">
            {PROOF_VISIBILITY_LABEL[v.visibility_level]}
            <span className="block text-xs text-muted-foreground">
              {VISIBILITY_DESCRIPTION[v.visibility_level]}
            </span>
          </Fact>
        </dl>

        {file.src && (
          <a
            href={file.src}
            target="_blank"
            rel="noreferrer"
            download
            className="mt-5 inline-flex h-10 w-full items-center justify-center gap-2 border border-foreground/70 text-sm font-semibold hover:bg-muted"
          >
            <DownloadSimple className="size-4" />
            Download original
          </a>
        )}
      </aside>
    </div>
  )
}

function Fact({ term, children }: { term: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[7.5rem_1fr] gap-3 py-2.5">
      <dt className="text-muted-foreground">{term}</dt>
      <dd className="min-w-0 font-medium">{children}</dd>
    </div>
  )
}

function ViewerButton({
  label,
  onClick,
  disabled,
  className,
  children,
}: {
  label: string
  onClick: () => void
  disabled?: boolean
  className?: string
  children: React.ReactNode
}) {
  return (
    <Button
      variant="ghost"
      size="icon-lg"
      aria-label={label}
      onClick={onClick}
      disabled={disabled}
      className={cn(
        'text-background hover:bg-background/15 hover:text-background',
        className,
      )}
    >
      {children}
    </Button>
  )
}
