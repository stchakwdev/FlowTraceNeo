import { existsSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { BOOKS_SCREENS } from './books-screens'

const web = join(__dirname, '..', '..', '..')
const screens = join(web, '..', 'rust', 'docs', 'screens')

// VP8 / VP8L / VP8X headers (https://developers.google.com/speed/webp/docs/riff_container).
function webpSize(file: string) {
  const b = readFileSync(file)
  expect(b.toString('ascii', 0, 4)).toBe('RIFF')
  expect(b.toString('ascii', 8, 12)).toBe('WEBP')
  const chunk = b.toString('ascii', 12, 16)
  if (chunk === 'VP8X') {
    return { width: b.readUIntLE(24, 3) + 1, height: b.readUIntLE(27, 3) + 1 }
  }
  if (chunk === 'VP8L') {
    const bits = b.readUInt32LE(21)
    return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 }
  }
  return {
    width: b.readUInt16LE(26) & 0x3fff,
    height: b.readUInt16LE(28) & 0x3fff,
  }
}

describe('Books page screenshots', () => {
  it('uses eight distinct screenshots', () => {
    expect(BOOKS_SCREENS).toHaveLength(8)
    expect(new Set(BOOKS_SCREENS.map((s) => s.src)).size).toBe(8)
  })

  it.each(BOOKS_SCREENS)(
    '$shot exists at its stated size, from a repo screenshot',
    (s) => {
      const file = join(web, 'public', s.src)
      expect(existsSync(file)).toBe(true)
      expect(existsSync(join(screens, `${s.shot}.png`))).toBe(true)
      expect(webpSize(file)).toEqual({ width: s.width, height: s.height })
    },
  )

  it.each(BOOKS_SCREENS)('$shot has alt text describing the screen', (s) => {
    expect(s.alt.length).toBeGreaterThan(40)
    expect(s.alt).not.toMatch(/screenshot|image of/i)
  })

  it('keeps the eight under 600 KB together', () => {
    const total = BOOKS_SCREENS.reduce(
      (sum, s) => sum + statSync(join(web, 'public', s.src)).size,
      0,
    )
    expect(total).toBeLessThan(600 * 1024)
  })
})
