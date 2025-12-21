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
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { supabase, type Verification } from '@/lib/supabase'

export type VerificationType = Verification['type']

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
}

const TYPE_LABELS: Record<VerificationType, string> = {
  receipt: 'Receipt',
  photo: 'Photo',
  document: 'Document',
  bank_statement: 'Bank Statement',
  signature: 'Signature',
  gps: 'GPS Data',
  biometric: 'Biometric',
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
        const {
          data: { signedUrl },
        } = await supabase.storage
          .from('verifications')
          .createSignedUrl(data.path, 60 * 60 * 24 * 7) // 7 days

        const successFile: UploadedFile = {
          ...uploadedFile,
          url: signedUrl || '',
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

      const {
        data: { signedUrl },
      } = await supabase.storage
        .from('verifications')
        .createSignedUrl(data.path, 60 * 60 * 24 * 7)

      onUploadComplete?.(signedUrl || '')
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
