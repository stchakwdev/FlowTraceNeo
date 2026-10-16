import { useCallback, useId, useRef, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  CheckCircle,
  CloudArrowUp,
  File,
  FileImage,
  FilePdf,
  MapPin,
  SpinnerGap,
  Trash,
  Warning,
  X,
} from '@phosphor-icons/react'
import type { VerificationType, VerificationVisibility } from '@/lib/supabase'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { VERIFICATION_TRUST_POINTS } from '@/lib/supabase'
import { createVerification } from '@/lib/data'
import {
  MAX_PROOF_MB,
  proofKey,
  releaseProofFile,
  storeProofFile,
} from '@/lib/proofs'
import { queryKeys } from '@/lib/queries'
import { useOrg } from '@/lib/org-context'
import {
  PROOF_TYPE_LABEL,
  PROOF_VISIBILITY_LABEL,
} from '@/components/verification-gallery'

export type { VerificationType, VerificationVisibility }

// GPS location data
export interface GPSLocation {
  latitude: number
  longitude: number
  accuracy: number
  locationName?: string
}

interface UploadedFile {
  id: string
  name: string
  size: number
  type: string
  /** `proof:<key>` for a file on the proof store, else a session-local URL. */
  url: string
  thumbnailUrl?: string | null
  status: 'uploading' | 'success' | 'error'
  progress: number
  error?: string
}

// Extended upload result with Phase 3 metadata
export interface UploadResult extends UploadedFile {
  visibilityLevel: VerificationVisibility
  executorName?: string
  description?: string
  gpsLocation?: GPSLocation
  trustPoints: number
}

export type { UploadedFile }

/** File types each kind of proof accepts. */
export const PROOF_ACCEPT: Record<VerificationType, Array<string>> = {
  receipt: ['image/jpeg', 'image/png', 'image/webp', 'application/pdf'],
  photo: ['image/jpeg', 'image/png', 'image/webp'],
  document: [
    'application/pdf',
    'image/jpeg',
    'image/png',
    'application/msword',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  ],
  bank_statement: ['application/pdf', 'image/jpeg', 'image/png'],
  signature: ['image/jpeg', 'image/png', 'image/webp'],
  gps: ['application/json', 'text/plain', 'image/jpeg', 'image/png'],
  biometric: ['application/json'],
  invoice: ['application/pdf', 'image/jpeg', 'image/png'],
  video: ['video/mp4', 'video/webm', 'video/quicktime'],
  audit: ['application/pdf'],
  contract: ['application/pdf', 'image/jpeg', 'image/png'],
  government_doc: ['application/pdf', 'image/jpeg', 'image/png'],
}

const EXTENSION: Record<string, string> = {
  'image/jpeg': 'JPG',
  'image/png': 'PNG',
  'image/webp': 'WebP',
  'application/pdf': 'PDF',
  'application/msword': 'DOC',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document':
    'DOCX',
  'application/json': 'JSON',
  'text/plain': 'TXT',
  'video/mp4': 'MP4',
  'video/webm': 'WebM',
  'video/quicktime': 'MOV',
}

/** "PDF, JPG or PNG" */
export function describeAccepted(types: Array<string>) {
  const names = [...new Set(types.map((t) => EXTENSION[t] ?? t))]
  if (names.length <= 1) return names.join('')
  return `${names.slice(0, -1).join(', ')} or ${names[names.length - 1]}`
}

/** Returns why a file can't be used as proof, or null when it can. */
export function validateProofFile(
  file: File,
  allowedTypes: Array<string>,
  maxSizeMB: number,
): string | null {
  if (!allowedTypes.includes(file.type)) {
    return `${file.name} is not a ${describeAccepted(allowedTypes)} file.`
  }
  if (file.size > maxSizeMB * 1024 * 1024) {
    return `${file.name} is ${formatFileSize(file.size)}; the limit is ${maxSizeMB} MB.`
  }
  return null
}

export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

export function fileIcon(mimeType: string) {
  if (mimeType.startsWith('image/')) return FileImage
  if (mimeType === 'application/pdf') return FilePdf
  return File
}

/**
 * A drop target for proof files with a keyboard-reachable Browse button.
 * It only hands the chosen files over; callers decide what happens next.
 */
export function ProofDropzone({
  accept,
  maxSizeMB,
  multiple = true,
  disabled = false,
  onFiles,
  hint,
  invalid = false,
  describedBy,
  className,
}: {
  accept: Array<string>
  maxSizeMB: number
  multiple?: boolean
  disabled?: boolean
  onFiles: (files: Array<File>) => void
  hint?: React.ReactNode
  invalid?: boolean
  describedBy?: string
  className?: string
}) {
  const inputRef = useRef<HTMLInputElement>(null)
  const hintId = useId()
  const [isDragging, setIsDragging] = useState(false)

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault()
        if (!disabled) setIsDragging(true)
      }}
      onDragLeave={(e) => {
        e.preventDefault()
        setIsDragging(false)
      }}
      onDrop={(e) => {
        e.preventDefault()
        setIsDragging(false)
        if (disabled) return
        const files = Array.from(e.dataTransfer.files)
        if (files.length > 0) onFiles(multiple ? files : files.slice(0, 1))
      }}
      className={cn(
        'flex flex-col items-center gap-3 border-2 border-dashed px-4 py-6 text-center transition-colors sm:flex-row sm:text-left',
        isDragging
          ? 'border-primary bg-report/25'
          : invalid
            ? 'border-destructive'
            : 'border-foreground/45',
        disabled && 'opacity-50',
        className,
      )}
    >
      <CloudArrowUp
        className={cn(
          'size-8 shrink-0',
          isDragging ? 'text-primary' : 'text-muted-foreground',
        )}
      />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-semibold">
          {isDragging ? 'Drop to add' : 'Drag files here, or browse'}
        </p>
        <p id={hintId} className="mt-0.5 text-xs text-muted-foreground">
          {hint ?? `${describeAccepted(accept)}, up to ${maxSizeMB} MB each`}
        </p>
      </div>
      <Button
        type="button"
        variant="outline"
        className="h-10"
        disabled={disabled}
        aria-describedby={cn(hintId, describedBy) || undefined}
        onClick={() => inputRef.current?.click()}
      >
        Browse files
      </Button>
      <input
        ref={inputRef}
        type="file"
        tabIndex={-1}
        aria-hidden="true"
        accept={accept.join(',')}
        multiple={multiple}
        disabled={disabled}
        className="sr-only"
        onChange={(e) => {
          const files = Array.from(e.target.files ?? [])
          if (files.length > 0) onFiles(files)
          // Allow choosing the same file again
          e.target.value = ''
        }}
      />
    </div>
  )
}

/** A chosen or uploaded file as a document row. */
export function FileRow({
  name,
  size,
  type,
  status,
  error,
  onRemove,
  removeDisabled,
}: {
  name: string
  size: number
  type: string
  status?: 'uploading' | 'success' | 'error' | 'queued'
  error?: string
  onRemove?: () => void
  removeDisabled?: boolean
}) {
  const Icon = fileIcon(type)
  return (
    <li className="flex items-center gap-3 py-2.5">
      <Icon
        className={cn(
          'size-5 shrink-0',
          status === 'error' ? 'text-destructive' : 'text-muted-foreground',
        )}
      />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{name}</p>
        <p
          className={cn(
            'text-xs',
            status === 'error' ? 'text-destructive' : 'text-muted-foreground',
          )}
        >
          {error ??
            `${formatFileSize(size)}${
              status === 'uploading'
                ? ' · Uploading…'
                : status === 'success'
                  ? ' · Uploaded'
                  : ''
            }`}
        </p>
      </div>
      {status === 'uploading' && (
        <SpinnerGap
          className="size-4 shrink-0 animate-spin text-primary"
          aria-hidden="true"
        />
      )}
      {status === 'success' && (
        <CheckCircle className="size-4 shrink-0" aria-hidden="true" />
      )}
      {status === 'error' && (
        <Warning
          className="size-4 shrink-0 text-destructive"
          aria-hidden="true"
        />
      )}
      {onRemove && (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-10"
          onClick={onRemove}
          disabled={removeDisabled}
          aria-label={`Remove ${name}`}
        >
          {status === 'error' ? <X /> : <Trash />}
        </Button>
      )}
    </li>
  )
}

interface FileUploadProps {
  organizationId: string
  transactionId: string
  verificationType: VerificationType
  onUploadComplete?: (file: UploadedFile) => void
  onFileRemove?: (fileId: string) => void
  maxFiles?: number
  maxSizeMB?: number
  accept?: string
  className?: string
  disabled?: boolean
  showVisibilitySelector?: boolean
  showGPSCapture?: boolean
  showExecutorName?: boolean
  showDescription?: boolean
  defaultVisibility?: VerificationVisibility
  createdBy?: string
}

/** Uploads proof files to the proof store as soon as they are chosen. */
export function FileUpload({
  organizationId,
  transactionId,
  verificationType,
  onUploadComplete,
  onFileRemove,
  maxFiles = 5,
  maxSizeMB = 10,
  accept,
  className,
  disabled = false,
}: FileUploadProps) {
  const [files, setFiles] = useState<Array<UploadedFile>>([])

  const allowedTypes = accept
    ? accept.split(',')
    : PROOF_ACCEPT[verificationType]
  // The proof store takes nothing over 25 MB.
  const sizeLimitMB = Math.min(maxSizeMB, MAX_PROOF_MB)

  const uploadFile = useCallback(
    async (file: File): Promise<UploadedFile> => {
      const entry: UploadedFile = {
        id: `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
        name: file.name,
        size: file.size,
        type: file.type,
        url: '',
        status: 'uploading',
        progress: 0,
      }
      setFiles((prev) => [...prev, entry])

      let result: UploadedFile
      const problem = validateProofFile(file, allowedTypes, sizeLimitMB)
      if (problem) {
        result = { ...entry, status: 'error', error: problem }
      } else {
        try {
          const stored = await storeProofFile(
            organizationId,
            transactionId,
            file,
          )
          result = {
            ...entry,
            url: stored.url,
            thumbnailUrl: stored.thumbnailUrl,
            status: 'success',
            progress: 100,
          }
        } catch (err) {
          result = {
            ...entry,
            status: 'error',
            error: `${file.name} did not upload${
              err instanceof Error && err.message ? `: ${err.message}` : ''
            }. Check your connection and try again.`,
          }
        }
      }
      setFiles((prev) => prev.map((f) => (f.id === entry.id ? result : f)))
      return result
    },
    [organizationId, transactionId, allowedTypes, sizeLimitMB],
  )

  const handleFiles = useCallback(
    async (chosen: Array<File>) => {
      for (const file of chosen.slice(0, maxFiles - files.length)) {
        const result = await uploadFile(file)
        if (result.status === 'success') onUploadComplete?.(result)
      }
    },
    [files.length, maxFiles, uploadFile, onUploadComplete],
  )

  const removeFile = useCallback(
    async (fileId: string) => {
      const file = files.find((f) => f.id === fileId)
      if (!file) return
      if (file.status === 'success' && file.url) {
        // A session-local file is released. A file on the proof store is only
        // dropped from this list: objects there are content-addressed (another
        // proof may share the key) and the Worker has no delete route.
        try {
          await releaseProofFile(file.url)
        } catch {
          // Nothing references it any more.
        }
      }
      setFiles((prev) => prev.filter((f) => f.id !== fileId))
      onFileRemove?.(fileId)
    },
    [files, onFileRemove],
  )

  const full = files.length >= maxFiles

  return (
    <div className={cn('space-y-2', className)}>
      <ProofDropzone
        accept={allowedTypes}
        maxSizeMB={sizeLimitMB}
        multiple={maxFiles > 1}
        disabled={disabled || full}
        onFiles={handleFiles}
        hint={
          full
            ? `That's the limit of ${maxFiles} files at once.`
            : `${PROOF_TYPE_LABEL[verificationType]}: ${describeAccepted(allowedTypes)}, up to ${sizeLimitMB} MB each`
        }
      />
      {files.length > 0 && (
        <ul className="divide-y divide-border border-b border-border">
          {files.map((file) => (
            <FileRow
              key={file.id}
              name={file.name}
              size={file.size}
              type={file.type}
              status={file.status}
              error={file.error}
              onRemove={() => removeFile(file.id)}
              removeDisabled={file.status === 'uploading'}
            />
          ))}
        </ul>
      )}
    </div>
  )
}

export interface AttachProofsInput {
  transactionId: string
  type: VerificationType
  /** Files still on the device; they are uploaded first. */
  files?: Array<File>
  /** Files already stored (from FileUpload). */
  uploaded?: Array<
    Pick<UploadedFile, 'url' | 'name' | 'type' | 'size' | 'thumbnailUrl'>
  >
  description?: string
  executorName?: string
  visibilityLevel?: VerificationVisibility
  location?: GPSLocation | null
}

/**
 * Creates one proof record per file (or a single record with no file, e.g. a
 * GPS reading), uploading local files first. Resolves with how many failed.
 */
export function useAttachProofs() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (input: AttachProofsInput) => {
      const record = (file?: {
        url: string
        thumbnailUrl?: string | null
        name: string
        type: string
        size: number
      }) => {
        // A proof-store key is org/<org>/<sha256>.
        const objectKey = proofKey(file?.url)
        return createVerification({
          transaction_id: input.transactionId,
          type: input.type,
          file_url: file?.url ?? null,
          ocr_data: null,
          metadata: file
            ? {
                file_name: file.name,
                mime_type: file.type,
                size: file.size,
                ...(objectKey && {
                  object_key: objectKey,
                  sha256: objectKey.split('/').at(-1),
                }),
              }
            : {},
          status: 'pending',
          verified_by: null,
          visibility_level: input.visibilityLevel ?? 'full',
          thumbnail_url: file?.thumbnailUrl ?? null,
          executor_name: input.executorName || null,
          description: input.description || file?.name || null,
          gps_latitude: input.location?.latitude ?? null,
          gps_longitude: input.location?.longitude ?? null,
          gps_accuracy: input.location?.accuracy ?? null,
          location_name: input.location?.locationName ?? null,
          created_by: null,
        })
      }

      const ready = [...(input.uploaded ?? [])]
      let failed = 0
      for (const file of input.files ?? []) {
        try {
          const stored = await storeProofFile(
            organizationId!,
            input.transactionId,
            file,
          )
          ready.push({
            url: stored.url,
            thumbnailUrl: stored.thumbnailUrl,
            name: file.name,
            type: file.type,
            size: file.size,
          })
        } catch {
          failed++
        }
      }
      if (ready.length === 0 && failed === 0) {
        await record()
      }
      for (const file of ready) {
        await record(file)
      }
      return { attached: ready.length || (failed ? 0 : 1), failed }
    },
    onSettled: (_data, _error, input) => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.verifications(input.transactionId),
      })
      queryClient.invalidateQueries({
        predicate: (q) =>
          (q.queryKey[0] === 'transaction' &&
            q.queryKey[1] === input.transactionId) ||
          q.queryKey[0] === 'transactions' ||
          q.queryKey[0] === 'org-stats',
      })
    },
  })
}

// GPS capture hook. Reverse geocoding is a nicety: if it fails or is slow,
// the coordinates are kept and no error is shown.
export function useGPSCapture() {
  const [location, setLocation] = useState<GPSLocation | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const captureLocation = useCallback(() => {
    if (typeof navigator === 'undefined' || !('geolocation' in navigator)) {
      setError('This browser cannot share its location.')
      return
    }
    setLoading(true)
    setError(null)

    navigator.geolocation.getCurrentPosition(
      async (position) => {
        const loc: GPSLocation = {
          latitude: position.coords.latitude,
          longitude: position.coords.longitude,
          accuracy: position.coords.accuracy,
        }
        try {
          const controller = new AbortController()
          const timer = setTimeout(() => controller.abort(), 5000)
          const response = await fetch(
            `https://nominatim.openstreetmap.org/reverse?lat=${loc.latitude}&lon=${loc.longitude}&format=json`,
            { signal: controller.signal },
          )
          clearTimeout(timer)
          if (response.ok) {
            const data = (await response.json()) as { display_name?: string }
            const name = data.display_name?.split(',').slice(0, 3).join(',')
            if (name) loc.locationName = name
          }
        } catch {
          // Place name is optional; coordinates are enough.
        }
        setLocation(loc)
        setLoading(false)
      },
      (err) => {
        setError(
          err.code === err.PERMISSION_DENIED
            ? 'Location access was declined. Allow it in your browser settings to attach a GPS reading.'
            : 'Could not read your location. Move to open sky or try again.',
        )
        setLoading(false)
      },
      { enableHighAccuracy: true, timeout: 10000 },
    )
  }, [])

  const clearLocation = useCallback(() => {
    setLocation(null)
    setError(null)
  }, [])

  return { location, loading, error, captureLocation, clearLocation }
}

const VISIBILITY_DESCRIPTIONS: Record<VerificationVisibility, string> = {
  full: 'Board and staff see the original file.',
  thumbnail: 'Donors see a low-resolution version.',
  summary: 'Partners see that proof is attached, not the file.',
  hidden: 'Not shown outside the organisation.',
}

interface VisibilitySelectorProps {
  value: VerificationVisibility
  onChange: (value: VerificationVisibility) => void
  disabled?: boolean
  className?: string
}

export function VisibilitySelector({
  value,
  onChange,
  disabled,
  className,
}: VisibilitySelectorProps) {
  const id = useId()
  return (
    <div className={className}>
      <Label htmlFor={id} className="mb-1.5 text-sm font-medium">
        Who can see the file
      </Label>
      <Select
        value={value}
        disabled={disabled}
        items={PROOF_VISIBILITY_LABEL}
        onValueChange={(v) => v && onChange(v as VerificationVisibility)}
      >
        <SelectTrigger
          id={id}
          aria-describedby={`${id}-desc`}
          className="h-10 w-full"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {(
            Object.keys(PROOF_VISIBILITY_LABEL) as Array<VerificationVisibility>
          ).map((level) => (
            <SelectItem key={level} value={level}>
              {PROOF_VISIBILITY_LABEL[level]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p id={`${id}-desc`} className="mt-1 text-xs text-muted-foreground">
        {VISIBILITY_DESCRIPTIONS[value]}
      </p>
    </div>
  )
}

interface GPSCaptureButtonProps {
  location: GPSLocation | null
  loading: boolean
  error: string | null
  onCapture: () => void
  onClear: () => void
  disabled?: boolean
  className?: string
}

export function GPSCaptureButton({
  location,
  loading,
  error,
  onCapture,
  onClear,
  disabled,
  className,
}: GPSCaptureButtonProps) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <p className="text-sm font-medium">GPS reading</p>
      {!location ? (
        <Button
          type="button"
          variant="outline"
          onClick={onCapture}
          disabled={disabled || loading}
          className="h-10 w-full"
        >
          {loading ? <SpinnerGap className="animate-spin" /> : <MapPin />}
          {loading ? 'Reading location…' : 'Add my current location'}
        </Button>
      ) : (
        <div className="flex items-start gap-2 border border-border p-2">
          <MapPin className="mt-0.5 size-4 shrink-0" />
          <div className="min-w-0 flex-1 text-xs">
            <p className="font-mono">
              {location.latitude.toFixed(5)}, {location.longitude.toFixed(5)}
            </p>
            {location.locationName && (
              <p className="truncate text-muted-foreground">
                {location.locationName}
              </p>
            )}
            <p className="text-muted-foreground">
              Accurate to {Math.round(location.accuracy)} m
            </p>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-10"
            onClick={onClear}
            disabled={disabled}
            aria-label="Remove location"
          >
            <X />
          </Button>
        </div>
      )}
      {error && (
        <p role="alert" className="text-xs text-destructive">
          {error}
        </p>
      )}
    </div>
  )
}

interface EnhancedUploadFormProps {
  verificationType: VerificationType
  visibilityLevel: VerificationVisibility
  onVisibilityChange: (level: VerificationVisibility) => void
  executorName: string
  onExecutorNameChange: (name: string) => void
  description: string
  onDescriptionChange: (desc: string) => void
  gpsLocation: GPSLocation | null
  gpsLoading: boolean
  gpsError: string | null
  onCaptureGPS: () => void
  onClearGPS: () => void
  disabled?: boolean
  className?: string
}

/** The details that travel with a proof: who, what, where, who may see it. */
export function EnhancedUploadForm({
  verificationType,
  visibilityLevel,
  onVisibilityChange,
  executorName,
  onExecutorNameChange,
  description,
  onDescriptionChange,
  gpsLocation,
  gpsLoading,
  gpsError,
  onCaptureGPS,
  onClearGPS,
  disabled,
  className,
}: EnhancedUploadFormProps) {
  const ids = useId()
  const trustPoints = VERIFICATION_TRUST_POINTS[verificationType] || 0

  return (
    <div className={cn('space-y-4', className)}>
      <p className="flex items-baseline justify-between gap-3 border-b border-border pb-2 text-sm">
        <span className="font-medium">
          {PROOF_TYPE_LABEL[verificationType]}
        </span>
        <span className="tabular-nums">
          <span className="font-bold">+{trustPoints}</span>{' '}
          <span className="text-muted-foreground">trust points</span>
        </span>
      </p>

      <div>
        <Label htmlFor={`${ids}-desc`} className="mb-1.5 text-sm font-medium">
          What it shows
        </Label>
        <Textarea
          id={`${ids}-desc`}
          value={description}
          onChange={(e) => onDescriptionChange(e.target.value)}
          placeholder="e.g. Receipt from Lokichar Hardware for drilling pipes"
          disabled={disabled}
        />
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <div>
          <Label htmlFor={`${ids}-who`} className="mb-1.5 text-sm font-medium">
            Captured by
          </Label>
          <Input
            id={`${ids}-who`}
            value={executorName}
            onChange={(e) => onExecutorNameChange(e.target.value)}
            placeholder="Name of the person in the field"
            disabled={disabled}
            className="h-10"
          />
        </div>
        <VisibilitySelector
          value={visibilityLevel}
          onChange={onVisibilityChange}
          disabled={disabled}
        />
      </div>

      <GPSCaptureButton
        location={gpsLocation}
        loading={gpsLoading}
        error={gpsError}
        onCapture={onCaptureGPS}
        onClear={onClearGPS}
        disabled={disabled}
      />
    </div>
  )
}

interface CompactFileUploadProps {
  organizationId: string
  transactionId: string
  verificationType: VerificationType
  onUploadComplete?: (url: string) => void
  disabled?: boolean
  className?: string
}

/** One-button upload for tight spaces. */
export function CompactFileUpload({
  organizationId,
  transactionId,
  verificationType,
  onUploadComplete,
  disabled = false,
  className,
}: CompactFileUploadProps) {
  const [isUploading, setIsUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const allowedTypes = PROOF_ACCEPT[verificationType]

  const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (!file) return
    const problem = validateProofFile(file, allowedTypes, 10)
    if (problem) {
      setError(problem)
      return
    }
    setIsUploading(true)
    setError(null)
    try {
      const stored = await storeProofFile(organizationId, transactionId, file)
      onUploadComplete?.(stored.url)
    } catch {
      setError('Upload failed. Check your connection and try again.')
    } finally {
      setIsUploading(false)
    }
  }

  return (
    <div className={cn('inline-flex items-center gap-2', className)}>
      <label
        className={cn(
          'inline-flex h-10 cursor-pointer items-center gap-1.5 border border-foreground/70 px-3 text-xs font-semibold transition-colors hover:bg-muted has-[input:focus-visible]:ring-2 has-[input:focus-visible]:ring-ring/50',
          (disabled || isUploading) && 'pointer-events-none opacity-50',
        )}
      >
        {isUploading ? (
          <SpinnerGap className="size-4 animate-spin" />
        ) : (
          <CloudArrowUp className="size-4" />
        )}
        {isUploading ? 'Uploading…' : 'Upload'}
        <input
          type="file"
          accept={allowedTypes.join(',')}
          onChange={handleChange}
          disabled={disabled || isUploading}
          className="sr-only"
        />
      </label>
      {error && (
        <span role="alert" className="text-xs text-destructive">
          {error}
        </span>
      )}
    </div>
  )
}
