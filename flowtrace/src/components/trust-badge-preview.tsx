import { useMemo } from 'react'
import type { BadgeStyle } from '@/lib/badge-utils'
import { generateGradeBadgeSVG, generateTrustBadgeSVG } from '@/lib/badge-utils'
import { useTrustScore } from '@/lib/queries'
import { cn } from '@/lib/utils'

interface TrustBadgePreviewProps {
  organizationId: string
  style?: BadgeStyle
  label?: string
  showGrade?: boolean
  /** Draw the badge this many times its natural size (it stays vector). */
  scale?: number
  className?: string
}

// The SVG has a fixed width and height but no viewBox, so it cannot grow by
// CSS alone. Give it one and multiply the size so it scales crisply.
function scaleSvg(svg: string, scale: number) {
  if (scale === 1) return svg
  return svg.replace(
    /<svg([^>]*?) width="(\d+(?:\.\d+)?)" height="(\d+(?:\.\d+)?)"/,
    (_m, attrs: string, w: string, h: string) =>
      `<svg${attrs} viewBox="0 0 ${w} ${h}" width="${Number(w) * scale}" height="${Number(h) * scale}"`,
  )
}

export function TrustBadgePreview({
  organizationId,
  style = 'flat',
  label = 'Trust Score',
  showGrade = false,
  scale = 1,
  className,
}: TrustBadgePreviewProps) {
  const { data: trustData, isLoading, error } = useTrustScore(organizationId)

  const score = trustData?.overall_score ?? 0

  const badgeSvg = useMemo(() => {
    const svg = showGrade
      ? generateGradeBadgeSVG({ label, score, style })
      : generateTrustBadgeSVG({ label, score, style })
    return scaleSvg(svg, scale)
  }, [label, score, style, showGrade, scale])

  if (isLoading) {
    return (
      <div
        role="status"
        className="h-5 w-32 bg-gap"
        style={
          scale === 1 ? undefined : { height: 20 * scale, width: 128 * scale }
        }
      >
        <span className="sr-only">Loading Trust Score</span>
      </div>
    )
  }

  if (error) {
    return (
      <p role="alert" className="text-xs text-destructive">
        Trust Score could not be loaded.
      </p>
    )
  }

  return (
    <div
      // Set the badge in the app's own Archivo, loaded by the stylesheet; the
      // SVG's font-family names a face the page does not register.
      className={cn('leading-none [&_svg]:block [&_text]:font-sans', className)}
      dangerouslySetInnerHTML={{ __html: badgeSvg }}
    />
  )
}

interface BadgePreviewGridProps {
  organizationId: string
  label?: string
}

export function BadgePreviewGrid({
  organizationId,
  label = 'Trust Score',
}: BadgePreviewGridProps) {
  const styles: Array<BadgeStyle> = [
    'flat',
    'flat-square',
    'for-the-badge',
    'plastic',
  ]

  return (
    <div className="space-y-4">
      <div>
        <h4 className="mb-2 text-sm font-semibold">Score badge</h4>
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
        <h4 className="mb-2 text-sm font-semibold">Grade badge</h4>
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
