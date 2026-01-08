import { useState } from 'react'
import {
  Receipt,
  Image as ImageIcon,
  MapPin,
  PencilLine,
  Fingerprint,
  Bank,
  File,
  FileText,
  Video,
  Shield,
  Scroll,
  Buildings,
  Eye,
  EyeSlash,
  CheckCircle,
  Clock,
  XCircle,
  X,
  User,
  Calendar,
  Download,
  MagnifyingGlassPlus,
  MagnifyingGlassMinus,
  ArrowsOut,
  ArrowsIn,
  CaretLeft,
  CaretRight,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import type {
  Verification,
  VerificationType,
  VerificationVisibility,
} from '@/lib/supabase'
import { VERIFICATION_TRUST_POINTS } from '@/lib/supabase'

// Icon map for verification types
const VERIFICATION_ICONS: Record<VerificationType, React.ElementType> = {
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

const TYPE_LABELS: Record<VerificationType, string> = {
  receipt: 'Receipt',
  photo: 'Photo Evidence',
  document: 'Document',
  bank_statement: 'Bank Statement',
  signature: 'Signature',
  gps: 'GPS Verification',
  biometric: 'Biometric',
  invoice: 'Invoice',
  video: 'Video Evidence',
  audit: 'Third-party Audit',
  contract: 'Contract',
  government_doc: 'Government Document',
}

const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    label: 'Pending Verification',
    className: 'text-amber-600 bg-amber-50 dark:bg-amber-900/20',
  },
  verified: {
    icon: CheckCircle,
    label: 'Verified',
    className: 'text-emerald-600 bg-emerald-50 dark:bg-emerald-900/20',
  },
  rejected: {
    icon: XCircle,
    label: 'Rejected',
    className: 'text-red-600 bg-red-50 dark:bg-red-900/20',
  },
}

const VISIBILITY_CONFIG: Record<VerificationVisibility, { icon: React.ElementType; label: string; description: string }> = {
  full: {
    icon: Eye,
    label: 'Full Access',
    description: 'Board & Staff can see all details',
  },
  thumbnail: {
    icon: Eye,
    label: 'Thumbnail Only',
    description: 'Donors see blurred version',
  },
  summary: {
    icon: EyeSlash,
    label: 'Summary Only',
    description: 'Partners see indicator only',
  },
  hidden: {
    icon: EyeSlash,
    label: 'Hidden',
    description: 'Not visible to public',
  },
}

interface VerificationViewerProps {
  verification: Verification | null
  verifications?: Verification[]
  onClose: () => void
  onNavigate?: (verification: Verification) => void
  className?: string
}

export function VerificationViewer({
  verification,
  verifications = [],
  onClose,
  onNavigate,
  className,
}: VerificationViewerProps) {
  const [zoom, setZoom] = useState(1)
  const [isFullscreen, setIsFullscreen] = useState(false)

  if (!verification) return null

  const Icon = VERIFICATION_ICONS[verification.type] || File
  const status = STATUS_CONFIG[verification.status]
  const StatusIcon = status.icon
  const visibility = VISIBILITY_CONFIG[verification.visibility_level || 'full']
  const VisibilityIcon = visibility.icon
  const trustPoints = verification.trust_points || VERIFICATION_TRUST_POINTS[verification.type] || 0

  const isImage = verification.file_url?.match(/\.(jpg|jpeg|png|gif|webp)(\?|$)/i)
  const isPDF = verification.file_url?.match(/\.pdf(\?|$)/i)
  const isVideo = verification.file_url?.match(/\.(mp4|webm|mov)(\?|$)/i)

  // Find current index for navigation
  const currentIndex = verifications.findIndex((v) => v.id === verification.id)
  const hasPrev = currentIndex > 0
  const hasNext = currentIndex < verifications.length - 1

  const handlePrev = () => {
    if (hasPrev && onNavigate) {
      onNavigate(verifications[currentIndex - 1])
    }
  }

  const handleNext = () => {
    if (hasNext && onNavigate) {
      onNavigate(verifications[currentIndex + 1])
    }
  }

  const handleZoomIn = () => setZoom((z) => Math.min(z + 0.25, 3))
  const handleZoomOut = () => setZoom((z) => Math.max(z - 0.25, 0.5))
  const handleResetZoom = () => setZoom(1)

  const handleDownload = () => {
    if (verification.file_url) {
      window.open(verification.file_url, '_blank')
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
      {/* Close button */}
      <Button
        variant="ghost"
        size="icon"
        onClick={onClose}
        className="absolute top-4 right-4 z-10 text-white hover:bg-white/20"
      >
        <X className="h-6 w-6" />
      </Button>

      {/* Navigation arrows */}
      {hasPrev && (
        <Button
          variant="ghost"
          size="icon"
          onClick={handlePrev}
          className="absolute left-4 top-1/2 -translate-y-1/2 z-10 text-white hover:bg-white/20"
        >
          <CaretLeft className="h-8 w-8" />
        </Button>
      )}
      {hasNext && (
        <Button
          variant="ghost"
          size="icon"
          onClick={handleNext}
          className="absolute right-4 top-1/2 -translate-y-1/2 z-10 text-white hover:bg-white/20"
        >
          <CaretRight className="h-8 w-8" />
        </Button>
      )}

      <div
        className={cn(
          'flex flex-col lg:flex-row gap-4 w-full max-w-6xl max-h-[90vh] mx-4',
          isFullscreen && 'max-w-none h-full mx-0',
          className,
        )}
      >
        {/* Media viewer */}
        <div
          className={cn(
            'relative flex-1 bg-black/50 rounded-xl overflow-hidden',
            'flex items-center justify-center',
            isFullscreen ? 'rounded-none' : '',
          )}
        >
          {isImage && verification.file_url ? (
            <div
              className="relative overflow-auto max-h-full max-w-full"
              style={{ transform: `scale(${zoom})`, transformOrigin: 'center' }}
            >
              <img
                src={verification.file_url}
                alt={verification.type}
                className="max-h-[70vh] object-contain"
              />
            </div>
          ) : isVideo && verification.file_url ? (
            <video
              src={verification.file_url}
              controls
              className="max-h-[70vh] max-w-full"
            />
          ) : isPDF && verification.file_url ? (
            <iframe
              src={verification.file_url}
              className="w-full h-[70vh]"
              title="PDF viewer"
            />
          ) : (
            <div className="flex flex-col items-center justify-center py-16 text-white/70">
              <Icon className="h-24 w-24 mb-4" weight="duotone" />
              <p className="text-lg font-medium">{TYPE_LABELS[verification.type]}</p>
              {verification.file_url && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDownload}
                  className="mt-4"
                >
                  <Download className="h-4 w-4 mr-2" />
                  Download File
                </Button>
              )}
            </div>
          )}

          {/* Zoom controls for images */}
          {isImage && (
            <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-1 p-1 bg-black/50 rounded-full">
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={handleZoomOut}
                className="text-white hover:bg-white/20"
              >
                <MagnifyingGlassMinus className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleResetZoom}
                className="text-white hover:bg-white/20 px-2 min-w-[60px]"
              >
                {Math.round(zoom * 100)}%
              </Button>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={handleZoomIn}
                className="text-white hover:bg-white/20"
              >
                <MagnifyingGlassPlus className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setIsFullscreen(!isFullscreen)}
                className="text-white hover:bg-white/20"
              >
                {isFullscreen ? (
                  <ArrowsIn className="h-4 w-4" />
                ) : (
                  <ArrowsOut className="h-4 w-4" />
                )}
              </Button>
            </div>
          )}
        </div>

        {/* Details panel */}
        {!isFullscreen && (
          <Card className="w-full lg:w-80 shrink-0 p-4 overflow-y-auto max-h-[70vh]">
            {/* Header */}
            <div className="flex items-start gap-3 mb-4">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10">
                <Icon className="h-6 w-6 text-primary" weight="duotone" />
              </div>
              <div className="flex-1">
                <h3 className="text-lg font-semibold">{TYPE_LABELS[verification.type]}</h3>
                <div className="flex items-center gap-2 mt-1">
                  <span
                    className={cn(
                      'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium',
                      status.className,
                    )}
                  >
                    <StatusIcon className="h-3 w-3" weight="fill" />
                    {status.label}
                  </span>
                </div>
              </div>
            </div>

            {/* Trust points */}
            <div className="flex items-center justify-between p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 mb-4">
              <span className="text-sm font-medium text-emerald-700 dark:text-emerald-400">
                Trust Points
              </span>
              <span className="text-lg font-bold text-emerald-600 dark:text-emerald-400">
                +{trustPoints}
              </span>
            </div>

            {/* Details */}
            <div className="space-y-4">
              {/* Visibility */}
              <div className="flex items-start gap-3">
                <VisibilityIcon className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
                <div>
                  <p className="text-sm font-medium">{visibility.label}</p>
                  <p className="text-xs text-muted-foreground">{visibility.description}</p>
                </div>
              </div>

              {/* Executor */}
              {verification.executor_name && (
                <div className="flex items-start gap-3">
                  <User className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs text-muted-foreground">Executed by</p>
                    <p className="text-sm font-medium">{verification.executor_name}</p>
                  </div>
                </div>
              )}

              {/* Location */}
              {(verification.location_name || verification.gps_latitude) && (
                <div className="flex items-start gap-3">
                  <MapPin className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs text-muted-foreground">Location</p>
                    {verification.location_name && (
                      <p className="text-sm font-medium">{verification.location_name}</p>
                    )}
                    {verification.gps_latitude && verification.gps_longitude && (
                      <p className="text-xs text-muted-foreground">
                        {verification.gps_latitude.toFixed(6)}, {verification.gps_longitude.toFixed(6)}
                        {verification.gps_accuracy && ` (±${Math.round(verification.gps_accuracy)}m)`}
                      </p>
                    )}
                  </div>
                </div>
              )}

              {/* Description */}
              {verification.description && (
                <div className="flex items-start gap-3">
                  <FileText className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs text-muted-foreground">Description</p>
                    <p className="text-sm">{verification.description}</p>
                  </div>
                </div>
              )}

              {/* Created date */}
              <div className="flex items-start gap-3">
                <Calendar className="h-5 w-5 text-muted-foreground shrink-0 mt-0.5" />
                <div>
                  <p className="text-xs text-muted-foreground">Added</p>
                  <p className="text-sm font-medium">
                    {new Date(verification.created_at).toLocaleDateString('en-US', {
                      year: 'numeric',
                      month: 'long',
                      day: 'numeric',
                    })}
                  </p>
                </div>
              </div>

              {/* Verified by */}
              {verification.verified_by && verification.verified_at && (
                <div className="flex items-start gap-3">
                  <CheckCircle className="h-5 w-5 text-emerald-600 shrink-0 mt-0.5" weight="fill" />
                  <div>
                    <p className="text-xs text-muted-foreground">Verified</p>
                    <p className="text-sm font-medium">
                      {new Date(verification.verified_at).toLocaleDateString('en-US', {
                        year: 'numeric',
                        month: 'short',
                        day: 'numeric',
                      })}
                    </p>
                  </div>
                </div>
              )}
            </div>

            {/* Actions */}
            {verification.file_url && (
              <div className="mt-6 pt-4 border-t">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleDownload}
                  className="w-full"
                >
                  <Download className="h-4 w-4 mr-2" />
                  Download Original
                </Button>
              </div>
            )}

            {/* Navigation indicator */}
            {verifications.length > 1 && (
              <div className="mt-4 pt-4 border-t text-center">
                <p className="text-xs text-muted-foreground">
                  {currentIndex + 1} of {verifications.length} verifications
                </p>
              </div>
            )}
          </Card>
        )}
      </div>
    </div>
  )
}
