import { describe, expect, it } from 'vitest'
import { formatCompact, formatDate, formatMoney } from './format'

describe('format', () => {
  it('shows a calendar date on its own day regardless of timezone', () => {
    expect(formatDate('2026-01-18')).toBe('18 Jan 2026')
  })

  it('formats money with currency and grouping', () => {
    expect(formatMoney(2_500_000)).toBe('KES 2,500,000')
  })

  it('compacts large figures', () => {
    expect(formatCompact(15_700_000)).toBe('15.7M')
    expect(formatCompact(8_000_000)).toBe('8M')
    expect(formatCompact(850_000)).toBe('850K')
  })
})
