import { useCallback, useState } from 'react'
import {
  CloudArrowUp,
  File,
  FileImage,
  FilePdf,
  Trash,
  X,
  CheckCircle,
  Warning,
  SpinnerGap,
  MapPin,
  Eye,
  EyeSlash,
  User,
  CaretDown,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  supabase,
  type VerificationType,
  type VerificationVisibility,
  VERIFICATION_TRUST_POINTS,
} from '@/lib/supabase'

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
  url: string
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
  // New Phase 3 options
  showVisibilitySelector?: boolean
  showGPSCapture?: boolean
  showExecutorName?: boolean
  showDescription?: boolean
  defaultVisibility?: VerificationVisibility
  createdBy?: string
}

const ALLOWED_TYPES: Record<string, string[]> = {
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
  gps: ['application/json', 'text/plain'],
  biometric: ['application/json'],
  // New Phase 3 types
  invoice: ['application/pdf', 'image/jpeg', 'image/png'],
  video: ['video/mp4', 'video/webm', 'video/quicktime'],
  audit: ['application/pdf'],
  contract: ['application/pdf', 'image/jpeg', 'image/png'],
  government_doc: ['application/pdf', 'image/jpeg', 'image/png'],
}

const TYPE_LABELS: Record<VerificationType, string> = {
  receipt: 'Receipt',
  photo: 'Photo Evidence',
  document: 'Document',
  bank_statement: 'Bank Statement',
  signature: 'Signature',
  gps: 'GPS Verification',
  biometric: 'Biometric',
  // New Phase 3 types
  invoice: 'Invoice',
  video: 'Video Evidence',
  audit: 'Third-party Audit',
  contract: 'Contract',
  government_doc: 'Government Document',
}

const VISIBILITY_LABELS: Record<VerificationVisibility, string> = {
  full: 'Full Access',
  thumbnail: 'Thumbnail Only',
  summary: 'Summary Only',
  hidden: 'Hidden',
}

const VISIBILITY_DESCRIPTIONS: Record<VerificationVisibility, string> = {
  full: 'Board & Staff can see original file with all details',
  thumbnail: 'Donors see blurred/low-res version',
  summary: 'Partners see "Proof attached" indicator only',
  hidden: 'Not visible to public',
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

function getFileIcon(mimeType: string) {
  if (mimeType.startsWith('image/')) {
    return FileImage
  }
  if (mimeType === 'application/pdf') {
    return FilePdf
  }
  return File
}

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
  const [files, setFiles] = useState<UploadedFile[]>([])
  const [isDragging, setIsDragging] = useState(false)

  const allowedTypes = ALLOWED_TYPES[verificationType] || ALLOWED_TYPES.document
  const acceptStr = accept || allowedTypes.join(',')
  const maxSizeBytes = maxSizeMB * 1024 * 1024

  const uploadFile = useCallback(
    async (file: File): Promise<UploadedFile> => {
      const fileId = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
      const uploadedFile: UploadedFile = {
        id: fileId,
        name: file.name,
        size: file.size,
        type: file.type,
        url: '',
        status: 'uploading',
        progress: 0,
      }

      // Add to state immediately
      setFiles((prev) => [...prev, uploadedFile])

      try {
        // Validate file type
        if (!allowedTypes.includes(file.type)) {
          throw new Error(
            `Invalid file type. Allowed: ${allowedTypes.map((t) => t.split('/')[1]).join(', ')}`,
          )
        }

        // Validate file size
        if (file.size > maxSizeBytes) {
          throw new Error(`File too large. Maximum size: ${maxSizeMB}MB`)
        }

        // Generate unique filename
        const fileExt = file.name.split('.').pop() || 'bin'
        const fileName = `${organizationId}/${transactionId}/${fileId}.${fileExt}`

        // Simulate progress updates (Supabase doesn't provide real progress)
        const progressInterval = setInterval(() => {
          setFiles((prev) =>
            prev.map((f) =>
              f.id === fileId && f.progress < 90
                ? { ...f, progress: f.progress + 10 }
                : f,
            ),
          )
        }, 100)

        // Upload to Supabase Storage
        const { data, error } = await supabase.storage
          .from('verifications')
          .upload(fileName, file, {
            cacheControl: '3600',
            upsert: false,
          })

        clearInterval(progressInterval)

        if (error) throw error

        // Get the URL (signed URL for private bucket)
        const { data: signed } = await supabase.storage
          .from('verifications')
          .createSignedUrl(data.path, 60 * 60 * 24 * 7) // 7 days

        const successFile: UploadedFile = {
          ...uploadedFile,
          url: signed?.signedUrl || '',
          status: 'success',
          progress: 100,
        }

        setFiles((prev) =>
          prev.map((f) => (f.id === fileId ? successFile : f)),
        )

        return successFile
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : 'Upload failed'
        const errorFile: UploadedFile = {
          ...uploadedFile,
          status: 'error',
          progress: 0,
          error: errorMessage,
        }

        setFiles((prev) =>
          prev.map((f) => (f.id === fileId ? errorFile : f)),
        )

        return errorFile
      }
    },
    [organizationId, transactionId, allowedTypes, maxSizeBytes, maxSizeMB],
  )

  const handleFiles = useCallback(
    async (fileList: FileList | File[]) => {
      const filesToUpload = Array.from(fileList).slice(
        0,
        maxFiles - files.length,
      )

      for (const file of filesToUpload) {
        const result = await uploadFile(file)
        if (result.status === 'success' && onUploadComplete) {
          onUploadComplete(result)
        }
      }
    },
    [files.length, maxFiles, uploadFile, onUploadComplete],
  )

  const handleDrop = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      e.preventDefault()
      setIsDragging(false)

      if (disabled) return

      const droppedFiles = e.dataTransfer.files
      if (droppedFiles.length > 0) {
        handleFiles(droppedFiles)
      }
    },
    [disabled, handleFiles],
  )

  const handleDragOver = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      e.preventDefault()
      if (!disabled) {
        setIsDragging(true)
      }
    },
    [disabled],
  )

  const handleDragLeave = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    setIsDragging(false)
  }, [])

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const selectedFiles = e.target.files
      if (selectedFiles && selectedFiles.length > 0) {
        handleFiles(selectedFiles)
      }
      // Reset input value to allow selecting the same file again
      e.target.value = ''
    },
    [handleFiles],
  )

  const removeFile = useCallback(
    async (fileId: string) => {
      const file = files.find((f) => f.id === fileId)
      if (!file) return

      // If file was uploaded successfully, delete from storage
      if (file.status === 'success' && file.url) {
        try {
          const pathMatch = file.url.match(/verifications\/(.+)\?/)
          if (pathMatch) {
            await supabase.storage.from('verifications').remove([pathMatch[1]])
          }
        } catch {
          // Ignore deletion errors
        }
      }

      setFiles((prev) => prev.filter((f) => f.id !== fileId))
      onFileRemove?.(fileId)
    },
    [files, onFileRemove],
  )

  const canUploadMore = files.length < maxFiles && !disabled

  return (
    <div className={cn('space-y-3', className)}>
      {/* Drop zone */}
      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        className={cn(
          'relative border-2 border-dashed rounded-lg p-6 text-center transition-colors',
          isDragging
            ? 'border-primary bg-primary/5'
            : 'border-muted-foreground/25 hover:border-muted-foreground/50',
          disabled && 'opacity-50 cursor-not-allowed',
          !disabled && 'cursor-pointer',
        )}
      >
        <input
          type="file"
          accept={acceptStr}
          multiple={maxFiles > 1}
          onChange={handleInputChange}
          disabled={!canUploadMore}
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-not-allowed"
        />

        <div className="flex flex-col items-center gap-2">
          <div
            className={cn(
              'p-3 rounded-full',
              isDragging ? 'bg-primary/10' : 'bg-muted',
            )}
          >
            <CloudArrowUp
              className={cn(
                'size-6',
                isDragging ? 'text-primary' : 'text-muted-foreground',
              )}
            />
          </div>

          <div className="space-y-1">
            <p className="text-sm font-medium">
              {isDragging ? (
                'Drop files here'
              ) : (
                <>
                  <span className="text-primary">Click to upload</span> or drag
                  and drop
                </>
              )}
            </p>
            <p className="text-xs text-muted-foreground">
              {TYPE_LABELS[verificationType]} - Max {maxSizeMB}MB
            </p>
            <p className="text-xs text-muted-foreground">
              {files.length}/{maxFiles} files
            </p>
          </div>
        </div>
      </div>

      {/* File list */}
      {files.length > 0 && (
        <ul className="space-y-2">
          {files.map((file) => {
            const FileIcon = getFileIcon(file.type)

            return (
              <li
                key={file.id}
                className={cn(
                  'flex items-center gap-3 p-3 rounded-lg border bg-card',
                  file.status === 'error' && 'border-destructive/50 bg-destructive/5',
                  file.status === 'success' && 'border-green-500/50',
                )}
              >
                <div
                  className={cn(
                    'shrink-0 p-2 rounded',
                    file.status === 'error' ? 'bg-destructive/10' : 'bg-muted',
                  )}
                >
                  <FileIcon
                    className={cn(
                      'size-5',
                      file.status === 'error'
                        ? 'text-destructive'
                        : 'text-muted-foreground',
                    )}
                  />
                </div>

                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{file.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {formatFileSize(file.size)}
                    {file.error && (
                      <span className="text-destructive ml-2">{file.error}</span>
                    )}
                  </p>

                  {/* Progress bar */}
                  {file.status === 'uploading' && (
                    <div className="mt-1.5 h-1 w-full bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full bg-primary transition-all duration-200"
                        style={{ width: `${file.progress}%` }}
                      />
                    </div>
                  )}
                </div>

                <div className="shrink-0 flex items-center gap-1">
                  {file.status === 'uploading' && (
                    <SpinnerGap className="size-4 text-primary animate-spin" />
                  )}
                  {file.status === 'success' && (
                    <CheckCircle className="size-4 text-green-500" weight="fill" />
                  )}
                  {file.status === 'error' && (
                    <Warning className="size-4 text-destructive" weight="fill" />
                  )}

                  <Button
                    variant="ghost"
                    size="icon-xs"
                    onClick={() => removeFile(file.id)}
                    disabled={file.status === 'uploading'}
                  >
                    {file.status === 'error' ? (
                      <X className="size-3.5" />
                    ) : (
                      <Trash className="size-3.5" />
                    )}
                  </Button>
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}

// GPS capture hook
export function useGPSCapture() {
  const [location, setLocation] = useState<GPSLocation | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const captureLocation = useCallback(() => {
    if (!navigator.geolocation) {
      setError('Geolocation is not supported by your browser')
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

        // Try to get location name via reverse geocoding (optional)
        try {
          const response = await fetch(
            `https://nominatim.openstreetmap.org/reverse?lat=${loc.latitude}&lon=${loc.longitude}&format=json`,
          )
          if (response.ok) {
            const data = await response.json()
            loc.locationName = data.display_name?.split(',').slice(0, 3).join(',')
          }
        } catch {
          // Ignore geocoding errors
        }

        setLocation(loc)
        setLoading(false)
      },
      (err) => {
        setError(err.message || 'Failed to get location')
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

// Visibility selector component
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
  const [isOpen, setIsOpen] = useState(false)

  return (
    <div className={cn('relative', className)}>
      <Label className="text-sm font-medium mb-1.5 block">Visibility Level</Label>
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className={cn(
          'flex items-center justify-between w-full px-3 py-2 text-sm border rounded-lg bg-background',
          'hover:bg-muted transition-colors',
          disabled && 'opacity-50 cursor-not-allowed',
        )}
      >
        <span className="flex items-center gap-2">
          {value === 'hidden' ? (
            <EyeSlash className="size-4 text-muted-foreground" />
          ) : (
            <Eye className="size-4 text-muted-foreground" />
          )}
          {VISIBILITY_LABELS[value]}
        </span>
        <CaretDown className="size-4 text-muted-foreground" />
      </button>

      {isOpen && (
        <div className="absolute z-10 w-full mt-1 border rounded-lg bg-card shadow-lg">
          {(Object.keys(VISIBILITY_LABELS) as VerificationVisibility[]).map((level) => (
            <button
              key={level}
              type="button"
              onClick={() => {
                onChange(level)
                setIsOpen(false)
              }}
              className={cn(
                'flex flex-col w-full px-3 py-2 text-left hover:bg-muted transition-colors',
                'first:rounded-t-lg last:rounded-b-lg',
                value === level && 'bg-primary/10',
              )}
            >
              <span className="flex items-center gap-2 text-sm font-medium">
                {level === 'hidden' ? (
                  <EyeSlash className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
                {VISIBILITY_LABELS[level]}
              </span>
              <span className="text-xs text-muted-foreground mt-0.5">
                {VISIBILITY_DESCRIPTIONS[level]}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

// GPS capture button component
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
    <div className={cn('space-y-2', className)}>
      <Label className="text-sm font-medium">GPS Location</Label>
      {!location ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onCapture}
          disabled={disabled || loading}
          className="w-full"
        >
          {loading ? (
            <SpinnerGap className="size-4 animate-spin mr-2" />
          ) : (
            <MapPin className="size-4 mr-2" />
          )}
          {loading ? 'Getting location...' : 'Capture GPS Location'}
        </Button>
      ) : (
        <div className="flex items-start gap-2 p-2 rounded-lg border bg-muted/30">
          <MapPin className="size-4 text-emerald-600 shrink-0 mt-0.5" weight="fill" />
          <div className="flex-1 min-w-0">
            <p className="text-xs font-medium">
              {location.latitude.toFixed(6)}, {location.longitude.toFixed(6)}
            </p>
            {location.locationName && (
              <p className="text-xs text-muted-foreground truncate">
                {location.locationName}
              </p>
            )}
            <p className="text-xs text-muted-foreground">
              Accuracy: ±{Math.round(location.accuracy)}m
            </p>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            onClick={onClear}
            disabled={disabled}
          >
            <X className="size-3.5" />
          </Button>
        </div>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  )
}

// Enhanced verification upload form with all Phase 3 fields
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
  const trustPoints = VERIFICATION_TRUST_POINTS[verificationType] || 0

  return (
    <div className={cn('space-y-4', className)}>
      {/* Trust points indicator */}
      <div className="flex items-center justify-between p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800">
        <span className="text-sm font-medium text-emerald-700 dark:text-emerald-400">
          {TYPE_LABELS[verificationType]}
        </span>
        <span className="text-sm font-bold text-emerald-600 dark:text-emerald-400">
          +{trustPoints} Trust Points
        </span>
      </div>

      {/* Visibility selector */}
      <VisibilitySelector
        value={visibilityLevel}
        onChange={onVisibilityChange}
        disabled={disabled}
      />

      {/* Executor name */}
      <div>
        <Label htmlFor="executor-name" className="text-sm font-medium">
          Executor Name
        </Label>
        <div className="relative mt-1.5">
          <User className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          <Input
            id="executor-name"
            value={executorName}
            onChange={(e) => onExecutorNameChange(e.target.value)}
            placeholder="Who executed this transaction?"
            disabled={disabled}
            className="pl-9"
          />
        </div>
        <p className="text-xs text-muted-foreground mt-1">
          Visibility controlled by access group permissions
        </p>
      </div>

      {/* Description */}
      <div>
        <Label htmlFor="description" className="text-sm font-medium">
          Description
        </Label>
        <Textarea
          id="description"
          value={description}
          onChange={(e) => onDescriptionChange(e.target.value)}
          placeholder="Add notes about this verification..."
          disabled={disabled}
          className="mt-1.5 min-h-[80px]"
        />
      </div>

      {/* GPS capture */}
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

// Compact version for inline use
interface CompactFileUploadProps {
  organizationId: string
  transactionId: string
  verificationType: VerificationType
  onUploadComplete?: (url: string) => void
  disabled?: boolean
  className?: string
}

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

  const allowedTypes = ALLOWED_TYPES[verificationType] || ALLOWED_TYPES.document

  const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setIsUploading(true)
    setError(null)

    try {
      if (!allowedTypes.includes(file.type)) {
        throw new Error('Invalid file type')
      }

      if (file.size > 10 * 1024 * 1024) {
        throw new Error('File too large (max 10MB)')
      }

      const fileExt = file.name.split('.').pop() || 'bin'
      const fileId = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
      const fileName = `${organizationId}/${transactionId}/${fileId}.${fileExt}`

      const { data, error: uploadError } = await supabase.storage
        .from('verifications')
        .upload(fileName, file, { cacheControl: '3600', upsert: false })

      if (uploadError) throw uploadError

      const { data: signed } = await supabase.storage
        .from('verifications')
        .createSignedUrl(data.path, 60 * 60 * 24 * 7)

      onUploadComplete?.(signed?.signedUrl || '')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed')
    } finally {
      setIsUploading(false)
      e.target.value = ''
    }
  }

  return (
    <div className={cn('inline-flex items-center gap-2', className)}>
      <label
        className={cn(
          'inline-flex items-center gap-1.5 px-2.5 h-8 text-xs font-medium rounded-none border cursor-pointer transition-colors',
          'border-border bg-background hover:bg-muted',
          disabled && 'opacity-50 cursor-not-allowed',
          isUploading && 'pointer-events-none',
        )}
      >
        {isUploading ? (
          <SpinnerGap className="size-4 animate-spin" />
        ) : (
          <CloudArrowUp className="size-4" />
        )}
        <span>{isUploading ? 'Uploading...' : 'Upload'}</span>
        <input
          type="file"
          accept={allowedTypes.join(',')}
          onChange={handleChange}
          disabled={disabled || isUploading}
          className="sr-only"
        />
      </label>
      {error && <span className="text-xs text-destructive">{error}</span>}
    </div>
  )
}
