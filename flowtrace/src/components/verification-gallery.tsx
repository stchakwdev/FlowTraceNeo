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
  Plus,
  CaretRight,
} from '@phosphor-icons/react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import type {
  Verification,
  VerificationType,
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
  photo: 'Photo',
  document: 'Document',
  bank_statement: 'Bank Statement',
  signature: 'Signature',
  gps: 'GPS',
  biometric: 'Biometric',
  invoice: 'Invoice',
  video: 'Video',
  audit: 'Audit',
  contract: 'Contract',
  government_doc: 'Gov. Doc',
}

const STATUS_CONFIG = {
  pending: {
    icon: Clock,
    label: 'Pending',
    className: 'text-amber-600 bg-amber-50 dark:bg-amber-900/20 border-amber-200',
  },
  verified: {
    icon: CheckCircle,
    label: 'Verified',
    className: 'text-emerald-600 bg-emerald-50 dark:bg-emerald-900/20 border-emerald-200',
  },
  rejected: {
    icon: XCircle,
    label: 'Rejected',
    className: 'text-red-600 bg-red-50 dark:bg-red-900/20 border-red-200',
  },
}

interface VerificationGalleryProps {
  verifications: Verification[]
  onVerificationClick?: (verification: Verification) => void
  onAddClick?: () => void
  showAddButton?: boolean
  compact?: boolean
  maxItems?: number
  className?: string
}

export function VerificationGallery({
  verifications,
  onVerificationClick,
  onAddClick,
  showAddButton = true,
  compact = false,
  maxItems,
  className,
}: VerificationGalleryProps) {
  const displayedVerifications = maxItems
    ? verifications.slice(0, maxItems)
    : verifications

  const hasMore = maxItems && verifications.length > maxItems

  // Calculate total trust points
  const totalTrustPoints = verifications.reduce(
    (sum, v) => sum + (v.trust_points || 0),
    0,
  )

  if (verifications.length === 0 && !showAddButton) {
    return (
      <div className={cn('text-center py-8', className)}>
        <File className="mx-auto h-12 w-12 text-muted-foreground/50" />
        <p className="mt-2 text-sm text-muted-foreground">
          No verifications attached
        </p>
      </div>
    )
  }

  return (
    <div className={className}>
      {/* Summary header */}
      {verifications.length > 0 && (
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium">
              {verifications.length} Verification{verifications.length !== 1 ? 's' : ''}
            </span>
            <span className="flex items-center gap-1 text-sm text-emerald-600 dark:text-emerald-400 font-medium">
              <Shield className="h-4 w-4" weight="fill" />
              +{totalTrustPoints} points
            </span>
          </div>
          {showAddButton && onAddClick && (
            <Button size="sm" variant="outline" onClick={onAddClick}>
              <Plus className="h-4 w-4" />
              Add
            </Button>
          )}
        </div>
      )}

      {/* Gallery grid */}
      <div
        className={cn(
          'grid gap-3',
          compact ? 'grid-cols-2 sm:grid-cols-3' : 'grid-cols-2 sm:grid-cols-3 lg:grid-cols-4',
        )}
      >
        {displayedVerifications.map((verification) => (
          <VerificationCard
            key={verification.id}
            verification={verification}
            onClick={() => onVerificationClick?.(verification)}
            compact={compact}
          />
        ))}

        {/* Add button as card */}
        {showAddButton && onAddClick && verifications.length === 0 && (
          <button
            onClick={onAddClick}
            className={cn(
              'flex flex-col items-center justify-center gap-2 p-4 rounded-lg',
              'border-2 border-dashed border-muted-foreground/25',
              'hover:border-primary hover:bg-primary/5 transition-colors',
              'text-muted-foreground hover:text-primary',
              compact ? 'min-h-[100px]' : 'min-h-[140px]',
            )}
          >
            <Plus className="h-8 w-8" />
            <span className="text-sm font-medium">Add Proof</span>
          </button>
        )}
      </div>

      {/* Show more indicator */}
      {hasMore && (
        <button
          onClick={() => onVerificationClick?.(verifications[0])}
          className="flex items-center justify-center gap-1 w-full mt-3 py-2 text-sm text-muted-foreground hover:text-primary transition-colors"
        >
          +{verifications.length - maxItems} more
          <CaretRight className="h-4 w-4" />
        </button>
      )}
    </div>
  )
}

interface VerificationCardProps {
  verification: Verification
  onClick?: () => void
  compact?: boolean
}

function VerificationCard({ verification, onClick, compact }: VerificationCardProps) {
  const Icon = VERIFICATION_ICONS[verification.type] || File
  const status = STATUS_CONFIG[verification.status]
  const StatusIcon = status.icon
  const trustPoints = verification.trust_points || VERIFICATION_TRUST_POINTS[verification.type] || 0
  const isImage = verification.file_url?.match(/\.(jpg|jpeg|png|gif|webp)(\?|$)/i)

  return (
    <Card
      className={cn(
        'group relative overflow-hidden cursor-pointer transition-all',
        'hover:shadow-md hover:border-primary/30',
        compact ? 'p-2' : 'p-3',
      )}
      onClick={onClick}
    >
      {/* Thumbnail or icon */}
      <div
        className={cn(
          'relative rounded-lg bg-muted flex items-center justify-center overflow-hidden',
          compact ? 'h-16' : 'h-24',
        )}
      >
        {isImage && verification.file_url ? (
          <img
            src={verification.thumbnail_url || verification.file_url}
            alt={verification.type}
            className="w-full h-full object-cover"
          />
        ) : (
          <Icon
            className={cn(
              'text-muted-foreground',
              compact ? 'h-8 w-8' : 'h-12 w-12',
            )}
            weight="duotone"
          />
        )}

        {/* Visibility indicator */}
        {verification.visibility_level && verification.visibility_level !== 'full' && (
          <div className="absolute top-1 right-1 p-1 rounded bg-background/80 backdrop-blur-sm">
            {verification.visibility_level === 'hidden' ? (
              <EyeSlash className="h-3 w-3 text-muted-foreground" />
            ) : (
              <Eye className="h-3 w-3 text-muted-foreground" />
            )}
          </div>
        )}

        {/* Status badge */}
        <div
          className={cn(
            'absolute bottom-1 left-1 flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium',
            status.className,
          )}
        >
          <StatusIcon className="h-3 w-3" weight="fill" />
          {!compact && status.label}
        </div>
      </div>

      {/* Info */}
      <div className={cn('mt-2', compact ? 'space-y-0.5' : 'space-y-1')}>
        <div className="flex items-center justify-between">
          <span className={cn('font-medium truncate', compact ? 'text-xs' : 'text-sm')}>
            {TYPE_LABELS[verification.type]}
          </span>
          <span
            className={cn(
              'text-emerald-600 dark:text-emerald-400 font-medium shrink-0',
              compact ? 'text-[10px]' : 'text-xs',
            )}
          >
            +{trustPoints}
          </span>
        </div>

        {!compact && verification.description && (
          <p className="text-xs text-muted-foreground truncate">
            {verification.description}
          </p>
        )}

        {!compact && verification.executor_name && (
          <p className="text-xs text-muted-foreground truncate">
            By: {verification.executor_name}
          </p>
        )}

        {!compact && verification.location_name && (
          <p className="flex items-center gap-1 text-xs text-muted-foreground truncate">
            <MapPin className="h-3 w-3 shrink-0" />
            {verification.location_name}
          </p>
        )}
      </div>

      {/* Hover overlay */}
      <div className="absolute inset-0 bg-primary/5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />
    </Card>
  )
}

// Compact inline list view
interface VerificationListProps {
  verifications: Verification[]
  onVerificationClick?: (verification: Verification) => void
  className?: string
}

export function VerificationList({
  verifications,
  onVerificationClick,
  className,
}: VerificationListProps) {
  if (verifications.length === 0) {
    return null
  }

  return (
    <div className={cn('flex flex-wrap gap-2', className)}>
      {verifications.map((verification) => {
        const Icon = VERIFICATION_ICONS[verification.type] || File
        const status = STATUS_CONFIG[verification.status]

        return (
          <button
            key={verification.id}
            onClick={() => onVerificationClick?.(verification)}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium',
              'border transition-colors',
              status.className,
              'hover:opacity-80',
            )}
          >
            <Icon className="h-3.5 w-3.5" />
            {TYPE_LABELS[verification.type]}
            <span className="text-emerald-600 dark:text-emerald-400">
              +{verification.trust_points || VERIFICATION_TRUST_POINTS[verification.type]}
            </span>
          </button>
        )
      })}
    </div>
  )
}

// Verification count badge for compact display
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
  if (count === 0) {
    return null
  }

  return (
    <button
      onClick={onClick}
      className={cn(
        'inline-flex items-center gap-1.5 px-2 py-1 rounded-full',
        'bg-emerald-50 dark:bg-emerald-900/20 text-emerald-700 dark:text-emerald-400',
        'text-xs font-medium transition-colors',
        'hover:bg-emerald-100 dark:hover:bg-emerald-900/30',
        className,
      )}
    >
      <Shield className="h-3.5 w-3.5" weight="fill" />
      {count} proof{count !== 1 ? 's' : ''}
      <span className="font-bold">+{totalTrustPoints}</span>
    </button>
  )
}
