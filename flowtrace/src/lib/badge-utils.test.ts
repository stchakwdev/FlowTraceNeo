import { describe, expect, it } from 'vitest'
import {
  BADGE_COLORS,
  generateCampaignBadgeSVG,
  generateGradeBadgeSVG,
  generateTrustBadgeSVG,
} from './badge-utils'

// The badge route renders these via dangerouslySetInnerHTML with a label taken
// from the URL, so any markup in user input must come out escaped.
const payload = '<img src=x onerror="alert(1)">'

describe('badge SVG escaping', () => {
  it.each([
    ['trust', () => generateTrustBadgeSVG({ label: payload, score: 80 })],
    ['grade', () => generateGradeBadgeSVG({ label: payload, score: 80 })],
    ['campaign', () => generateCampaignBadgeSVG({ name: payload, percentage: 50 })],
  ])('escapes user text in the %s badge', (_name, render) => {
    const svg = render()
    expect(svg).not.toContain('<img')
    expect(svg).toContain('&lt;img')
  })
})

describe('generateTrustBadgeSVG', () => {
  it('shows the score out of 100', () => {
    expect(generateTrustBadgeSVG({ score: 87 })).toContain('87/100')
  })

  it('colors by score band unless a color is given', () => {
    expect(generateTrustBadgeSVG({ score: 85 })).toContain(BADGE_COLORS.report)
    expect(generateTrustBadgeSVG({ score: 50 })).toContain(BADGE_COLORS.middling)
    expect(generateTrustBadgeSVG({ score: 10 })).toContain(BADGE_COLORS.flag)
    expect(generateTrustBadgeSVG({ score: 10, color: '#123456' })).toContain('#123456')
  })
})

describe('generateGradeBadgeSVG', () => {
  it.each([
    [95, 'A+'],
    [80, 'A'],
    [70, 'B'],
    [60, 'C'],
    [50, 'D'],
    [49, 'F'],
  ])('score %i is grade %s', (score, grade) => {
    expect(generateGradeBadgeSVG({ score })).toContain(`Grade ${grade}`)
  })
})
