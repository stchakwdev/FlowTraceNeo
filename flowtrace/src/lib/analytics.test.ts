import { afterEach, describe, expect, it, vi } from 'vitest'
import { UMAMI_SCRIPT_URL, track, umamiScript } from './analytics'

describe('track', () => {
  afterEach(() => {
    delete window.umami
  })

  it('does nothing when Umami is not loaded', () => {
    expect(() => track('demo_started')).not.toThrow()
  })

  it('calls through to Umami when it is loaded', () => {
    const spy = vi.fn()
    window.umami = { track: spy }
    track('cta_click', { location: 'masthead' })
    expect(spy).toHaveBeenCalledWith('cta_click', { location: 'masthead' })
  })

  it('swallows an error thrown by Umami', () => {
    window.umami = {
      track: () => {
        throw new Error('blocked')
      },
    }
    expect(() => track('demo_started')).not.toThrow()
  })
})

describe('umamiScript', () => {
  it('adds no script without a website id', () => {
    expect(umamiScript(undefined)).toEqual([])
    expect(umamiScript('')).toEqual([])
  })

  it('adds the deferred cloud script with the website id', () => {
    expect(umamiScript('abc-123')).toEqual([
      { src: UMAMI_SCRIPT_URL, defer: true, 'data-website-id': 'abc-123' },
    ])
  })
})
