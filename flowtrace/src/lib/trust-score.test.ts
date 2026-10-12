import { describe, expect, it } from 'vitest'
import { calculateTrustScore, getTrustGrade } from './supabase'

describe('calculateTrustScore', () => {
  it('weights verification 40%, documentation 30%, consistency 30%', () => {
    expect(calculateTrustScore(100, 0, 0)).toBe(40)
    expect(calculateTrustScore(0, 100, 0)).toBe(30)
    expect(calculateTrustScore(0, 0, 100)).toBe(30)
    expect(calculateTrustScore(90, 80, 70)).toBe(81)
  })
})

describe('getTrustGrade', () => {
  it('matches the badge grade bands', () => {
    expect(getTrustGrade(90)).toBe('A+')
    expect(getTrustGrade(89)).toBe('A')
    expect(getTrustGrade(0)).toBe('F')
  })
})
