import { useMemo } from 'react'
import { CircleNotch } from '@phosphor-icons/react'
import { generateTrustBadgeSVG, generateGradeBadgeSVG, type BadgeStyle } from '@/lib/badge-utils'
import { useTrustScore } from '@/lib/queries'

interface TrustBadgePreviewProps {
  organizationId: string
  style?: BadgeStyle
  label?: string
  showGrade?: boolean
  className?: string
}

export function TrustBadgePreview({
  organizationId,
  style = 'flat',
  label = 'Trust Score',
  showGrade = false,
  className,
}: TrustBadgePreviewProps) {
  const { data: trustData, isLoading } = useTrustScore(organizationId)

  const score = trustData?.overall_score ?? 0

  const badgeSvg = useMemo(() => {
    if (showGrade) {
      return generateGradeBadgeSVG({ label, score, style })
    }
    return generateTrustBadgeSVG({ label, score, style })
  }, [label, score, style, showGrade])

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-4">
        <CircleNotch className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div
      className={className}
      dangerouslySetInnerHTML={{ __html: badgeSvg }}
    />
  )
}

interface BadgePreviewGridProps {
  organizationId: string
  label?: string
}

export function BadgePreviewGrid({ organizationId, label = 'Trust Score' }: BadgePreviewGridProps) {
  const styles: BadgeStyle[] = ['flat', 'flat-square', 'for-the-badge', 'plastic']

  return (
    <div className="space-y-4">
      <div>
        <h4 className="text-sm font-medium mb-2">Score Badge</h4>
        <div className="flex flex-wrap gap-3">
          {styles.map((style) => (
            <TrustBadgePreview
              key={`score-${style}`}
              organizationId={organizationId}
              style={style}
              label={label}
            />
          ))}
        </div>
      </div>

      <div>
        <h4 className="text-sm font-medium mb-2">Grade Badge</h4>
        <div className="flex flex-wrap gap-3">
          {styles.map((style) => (
            <TrustBadgePreview
              key={`grade-${style}`}
              organizationId={organizationId}
              style={style}
              label="FlowTrace"
              showGrade
            />
          ))}
        </div>
      </div>
    </div>
  )
}
