import { afterEach, describe, expect, it, vi } from 'vitest'
import { randomId } from './random-id'

const UUID_V4 =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
const real = globalThis.crypto

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('randomId', () => {
  it('uses crypto.randomUUID in a secure context', () => {
    vi.stubGlobal('crypto', { randomUUID: () => 'from-random-uuid' })
    expect(randomId()).toBe('from-random-uuid')
  })

  it('mints a UUID v4 from getRandomValues on plain HTTP, where randomUUID is missing', () => {
    vi.stubGlobal('crypto', {
      getRandomValues: real.getRandomValues.bind(real),
    })
    const ids = Array.from({ length: 50 }, randomId)
    for (const id of ids) expect(id).toMatch(UUID_V4)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it('still mints a UUID v4 with no crypto at all', () => {
    vi.stubGlobal('crypto', undefined)
    expect(randomId()).toMatch(UUID_V4)
  })
})
