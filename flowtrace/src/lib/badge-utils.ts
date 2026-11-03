export type BadgeStyle = 'flat' | 'flat-square' | 'for-the-badge' | 'plastic'

interface BadgeOptions {
  label?: string
  score: number
  style?: BadgeStyle
  labelColor?: string
  color?: string
}

// Labels can come from URL params and the SVG is injected as HTML, so escape
// any user text before it goes into markup
function escapeXml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

// The report palette: ink for the label, report blue for a sound score, ink
// grey for a middling one, and signal orange only when the score is low enough
// to be a flag.
export const BADGE_COLORS = {
  ink: '#13202D',
  report: '#1C5FA8',
  middling: '#4A5866',
  flag: '#C4501B',
} as const

const BADGE_FONT = "Archivo,'Helvetica Neue',Arial,sans-serif"

function getScoreColor(score: number): string {
  if (score >= 60) return BADGE_COLORS.report
  if (score >= 40) return BADGE_COLORS.middling
  return BADGE_COLORS.flag
}

function getScoreGrade(score: number): string {
  if (score >= 90) return 'A+'
  if (score >= 80) return 'A'
  if (score >= 70) return 'B'
  if (score >= 60) return 'C'
  if (score >= 50) return 'D'
  return 'F'
}

interface BadgeParts {
  left: string
  right: string
  ariaLabel: string
  title: string
  style: BadgeStyle
  labelColor: string
  valueColor: string
  /** Approximate glyph widths for the label and value text. */
  leftChar?: number
  rightChar?: number
}

// Two flat, square-cornered fields: label on ink, value on its state colour.
// Every text input is escaped by the caller before it reaches here.
function renderBadge({
  left,
  right,
  ariaLabel,
  title,
  style,
  labelColor,
  valueColor,
  leftChar = 6.5,
  rightChar = 7.5,
}: BadgeParts): string {
  const large = style === 'for-the-badge'
  const height = large ? 28 : 20
  const pad = large ? 12 : 7
  const textY = large ? 18 : 14
  const leftWidth = Math.round(left.length * leftChar * (large ? 1.1 : 1) + pad * 2)
  const rightWidth = Math.round(right.length * rightChar * (large ? 1.1 : 1) + pad * 2)
  const totalWidth = leftWidth + rightWidth
  const caps = large ? ' letter-spacing=".08em"' : ''
  const leftText = large ? left.toUpperCase() : left
  return `
<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="${height}" role="img" aria-label="${ariaLabel}">
  <title>${title}</title>
  <rect width="${leftWidth}" height="${height}" fill="${labelColor}"/>
  <rect x="${leftWidth}" width="${rightWidth}" height="${height}" fill="${valueColor}"/>
  <g fill="#fff" text-anchor="middle" font-family="${BADGE_FONT}" font-size="11"${caps}>
    <text x="${leftWidth / 2}" y="${textY}" font-weight="600">${leftText}</text>
    <text x="${leftWidth + rightWidth / 2}" y="${textY}" font-weight="800" style="font-variant-numeric:tabular-nums">${right}</text>
  </g>
</svg>
`.trim()
}

export function generateTrustBadgeSVG(options: BadgeOptions): string {
  const {
    label = 'Trust Score',
    score,
    style = 'flat',
    labelColor = BADGE_COLORS.ink,
    color,
  } = options
  const safeLabel = escapeXml(label)
  const scoreText = `${score}/100`
  return renderBadge({
    left: safeLabel,
    right: scoreText,
    ariaLabel: `${safeLabel}: ${scoreText}`,
    title: `${safeLabel}: ${scoreText}`,
    style,
    labelColor,
    valueColor: color || getScoreColor(score),
  })
}

export function generateGradeBadgeSVG(options: BadgeOptions): string {
  const {
    label = 'FlowTrace',
    score,
    style = 'flat',
    labelColor = BADGE_COLORS.ink,
    color,
  } = options
  const safeLabel = escapeXml(label)
  const grade = getScoreGrade(score)
  return renderBadge({
    left: safeLabel,
    right: grade,
    ariaLabel: `${safeLabel}: ${grade}`,
    title: `${safeLabel}: Grade ${grade}`,
    style,
    labelColor,
    valueColor: color || getScoreColor(score),
    rightChar: 9,
  })
}

export function generateCampaignBadgeSVG(options: {
  name: string
  percentage: number
  style?: BadgeStyle
}): string {
  const { name, percentage, style = 'flat' } = options
  const displayName = name.length > 20 ? name.substring(0, 18) + '...' : name
  const safeName = escapeXml(displayName)
  const percentText = `${Math.round(percentage)}%`
  return renderBadge({
    left: safeName,
    right: percentText,
    ariaLabel: `${safeName}: ${percentText} funded`,
    title: `${safeName}: ${percentText} funded`,
    style,
    labelColor: BADGE_COLORS.ink,
    valueColor: percentage >= 100 ? BADGE_COLORS.report : BADGE_COLORS.middling,
    leftChar: 6,
  })
}

// The SVG has a fixed width and height but no viewBox, so it cannot grow by
// CSS alone. Give it one and multiply the size so it scales crisply.
export function scaleSvg(svg: string, scale: number) {
  if (scale === 1) return svg
  return svg.replace(
    /<svg([^>]*?) width="(\d+(?:\.\d+)?)" height="(\d+(?:\.\d+)?)"/,
    (_m, attrs: string, w: string, h: string) =>
      `<svg${attrs} viewBox="0 0 ${w} ${h}" width="${Number(w) * scale}" height="${Number(h) * scale}"`,
  )
}
