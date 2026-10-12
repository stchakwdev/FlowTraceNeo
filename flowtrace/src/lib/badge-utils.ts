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

function getScoreColor(score: number): string {
  if (score >= 80) return '#22c55e' // green-500
  if (score >= 60) return '#84cc16' // lime-500
  if (score >= 40) return '#eab308' // yellow-500
  if (score >= 20) return '#f97316' // orange-500
  return '#ef4444' // red-500
}

function getScoreGrade(score: number): string {
  if (score >= 90) return 'A+'
  if (score >= 80) return 'A'
  if (score >= 70) return 'B'
  if (score >= 60) return 'C'
  if (score >= 50) return 'D'
  return 'F'
}

export function generateTrustBadgeSVG(options: BadgeOptions): string {
  const {
    label = 'Trust Score',
    score,
    style = 'flat',
    labelColor = '#555',
    color,
  } = options

  const scoreColor = color || getScoreColor(score)
  const scoreText = `${score}/100`

  // Calculate widths based on text length
  const labelWidth = label.length * 6.5 + 10
  const scoreWidth = scoreText.length * 7 + 10
  const totalWidth = labelWidth + scoreWidth

  const isForTheBadge = style === 'for-the-badge'
  const height = isForTheBadge ? 28 : 20
  const fontSize = isForTheBadge ? 11 : 11
  const textY = isForTheBadge ? 18 : 14

  // Different border radius based on style
  const rx = style === 'flat-square' ? 0 : 3

  // Gradient for plastic style
  const gradientDef = style === 'plastic' ? `
    <linearGradient id="smooth" x2="0" y2="100%">
      <stop offset="0" stop-color="#fff" stop-opacity=".7"/>
      <stop offset=".1" stop-color="#aaa" stop-opacity=".1"/>
      <stop offset=".9" stop-color="#000" stop-opacity=".3"/>
      <stop offset="1" stop-color="#000" stop-opacity=".5"/>
    </linearGradient>
  ` : ''

  const plasticOverlay = style === 'plastic' ? `
    <rect rx="${rx}" width="${totalWidth}" height="${height}" fill="url(#smooth)"/>
  ` : ''

  const safeLabel = escapeXml(label)

  return `
<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="${height}" role="img" aria-label="${safeLabel}: ${scoreText}">
  <title>${safeLabel}: ${scoreText}</title>
  <defs>${gradientDef}</defs>
  <rect rx="${rx}" width="${totalWidth}" height="${height}" fill="${labelColor}"/>
  <rect rx="${rx}" x="${labelWidth}" width="${scoreWidth}" height="${height}" fill="${scoreColor}"/>
  ${style !== 'flat-square' ? `<rect rx="${rx}" width="${totalWidth}" height="${height}" fill="url(#smooth)"/>` : ''}
  ${plasticOverlay}
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="${fontSize}" ${isForTheBadge ? 'text-transform="uppercase" letter-spacing=".5"' : ''}>
    <text x="${labelWidth / 2}" y="${textY}" fill="#010101" fill-opacity=".3">${safeLabel}</text>
    <text x="${labelWidth / 2}" y="${textY - 1}" fill="#fff">${safeLabel}</text>
    <text x="${labelWidth + scoreWidth / 2}" y="${textY}" fill="#010101" fill-opacity=".3">${scoreText}</text>
    <text x="${labelWidth + scoreWidth / 2}" y="${textY - 1}" fill="#fff">${scoreText}</text>
  </g>
</svg>
`.trim()
}

export function generateGradeBadgeSVG(options: BadgeOptions): string {
  const {
    label = 'FlowTrace',
    score,
    style = 'flat',
    labelColor = '#555',
    color,
  } = options

  const scoreColor = color || getScoreColor(score)
  const grade = getScoreGrade(score)

  const labelWidth = label.length * 6.5 + 10
  const gradeWidth = grade.length * 8 + 14
  const totalWidth = labelWidth + gradeWidth

  const isForTheBadge = style === 'for-the-badge'
  const height = isForTheBadge ? 28 : 20
  const fontSize = isForTheBadge ? 11 : 11
  const textY = isForTheBadge ? 18 : 14
  const rx = style === 'flat-square' ? 0 : 3

  const safeLabel = escapeXml(label)

  return `
<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="${height}" role="img" aria-label="${safeLabel}: ${grade}">
  <title>${safeLabel}: Grade ${grade}</title>
  <rect rx="${rx}" width="${totalWidth}" height="${height}" fill="${labelColor}"/>
  <rect rx="${rx}" x="${labelWidth}" width="${gradeWidth}" height="${height}" fill="${scoreColor}"/>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="${fontSize}" font-weight="bold" ${isForTheBadge ? 'text-transform="uppercase" letter-spacing=".5"' : ''}>
    <text x="${labelWidth / 2}" y="${textY}" fill="#010101" fill-opacity=".3">${safeLabel}</text>
    <text x="${labelWidth / 2}" y="${textY - 1}" fill="#fff">${safeLabel}</text>
    <text x="${labelWidth + gradeWidth / 2}" y="${textY}" fill="#010101" fill-opacity=".3">${grade}</text>
    <text x="${labelWidth + gradeWidth / 2}" y="${textY - 1}" fill="#fff">${grade}</text>
  </g>
</svg>
`.trim()
}

export function generateCampaignBadgeSVG(options: {
  name: string
  percentage: number
  style?: BadgeStyle
}): string {
  const { name, percentage, style = 'flat' } = options

  const displayName = name.length > 20 ? name.substring(0, 18) + '...' : name
  const percentText = `${Math.round(percentage)}%`

  const labelWidth = displayName.length * 6 + 10
  const percentWidth = percentText.length * 7 + 10
  const totalWidth = labelWidth + percentWidth

  const color = percentage >= 100 ? '#22c55e' : percentage >= 75 ? '#84cc16' : percentage >= 50 ? '#3b82f6' : '#6b7280'

  const height = 20
  const rx = style === 'flat-square' ? 0 : 3

  const safeDisplayName = escapeXml(displayName)

  return `
<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="${height}" role="img" aria-label="${safeDisplayName}: ${percentText} funded">
  <title>${safeDisplayName}: ${percentText} funded</title>
  <rect rx="${rx}" width="${totalWidth}" height="${height}" fill="#555"/>
  <rect rx="${rx}" x="${labelWidth}" width="${percentWidth}" height="${height}" fill="${color}"/>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="${labelWidth / 2}" y="14" fill="#010101" fill-opacity=".3">${safeDisplayName}</text>
    <text x="${labelWidth / 2}" y="13" fill="#fff">${safeDisplayName}</text>
    <text x="${labelWidth + percentWidth / 2}" y="14" fill="#010101" fill-opacity=".3">${percentText}</text>
    <text x="${labelWidth + percentWidth / 2}" y="13" fill="#fff">${percentText}</text>
  </g>
</svg>
`.trim()
}
