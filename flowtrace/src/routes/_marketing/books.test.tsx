import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen, within } from '@testing-library/react'
import type { BooksRelease } from '@/lib/books-release'
import { renderWithRouter } from '@/test/render-with-router'
import { SITE_URL } from '@/lib/site'

const track = vi.hoisted(() => vi.fn())
vi.mock('@/lib/analytics', () => ({ track }))
// The route's loader is a server function; the page itself only takes its result.
vi.mock('@/lib/books-release-server', () => ({ getBooksRelease: vi.fn() }))

const { BooksPage } = await import('@/components/books/books-page')
const { BOOKS_TITLE, Route } = await import('./books')

const render = (release: BooksRelease) =>
  renderWithRouter(() => <BooksPage release={release} />)

const released: BooksRelease = {
  kind: 'available',
  tag: 'v0.1.0',
  version: '0.1.0',
  publishedAt: '2026-11-03T09:30:00Z',
  url: 'https://github.com/stchakwdev/FlowTraceNeo/releases/tag/v0.1.0',
  checksumsUrl: 'https://example.test/SHA256SUMS',
  assets: [
    {
      name: 'flowtrace-books-0.1.0-linux-x86_64.tar.gz',
      platform: 'Linux',
      arch: 'x86_64',
      size: 21_654_321,
      url: 'https://example.test/linux.tar.gz',
    },
    {
      name: 'flowtrace-books-0.1.0-macos-arm64.tar.gz',
      platform: 'macOS',
      arch: 'arm64',
      size: 20_123_456,
      url: 'https://example.test/macos.tar.gz',
    },
  ],
}

afterEach(() => {
  cleanup()
  track.mockClear()
})

describe('BooksPage', () => {
  it('sets out the sections with headings in order', async () => {
    await render({ kind: 'none' })
    expect(
      screen.getByRole('heading', {
        level: 1,
        name: 'The books, offline, native.',
      }),
    ).toBeTruthy()
    expect(
      screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent),
    ).toEqual([
      'What it covers',
      'How it stays correct',
      'Not released yet',
      'Requirements and install',
      'Try the books in your browser first.',
    ])
    expect(screen.getByText('861')).toBeTruthy()
  })

  it('without a release, offers early access and never an empty download table', async () => {
    await render({ kind: 'none' })
    const early = screen.getAllByRole('link', { name: /Request early access/ })
    expect(early).toHaveLength(2)
    expect(early[0].getAttribute('href')).toMatch(/^mailto:TODO-USER\?subject=/)
    expect(screen.queryByRole('table')).toBeNull()
    expect(screen.queryByText(/Download for Linux/)).toBeNull()
    expect(screen.queryByText(/0 downloads/)).toBeNull()
    expect(
      screen
        .getByRole('link', { name: 'Source on GitHub' })
        .getAttribute('href'),
    ).toBe('https://github.com/stchakwdev/FlowTraceNeo')

    fireEvent.click(early[0])
    expect(track).toHaveBeenCalledWith('cta_click', { location: 'books-hero' })
  })

  it('with a release, links the download and lists both archives with size and checksums', async () => {
    await render(released)
    const hero = screen.getByRole('link', {
      name: /Download for Linux \/ macOS/,
    })
    expect(hero.getAttribute('href')).toBe('#download')
    expect(
      screen.queryByRole('link', { name: /Request early access/ }),
    ).toBeNull()

    const rows = within(screen.getByRole('table')).getAllByRole('row').slice(1)
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringContaining('21.7 MB'),
      expect.stringContaining('20.1 MB'),
    ])
    expect(
      screen.getByText(/Version 0\.1\.0, released 3 Nov 2026/),
    ).toBeTruthy()
    expect(
      screen
        .getByRole('link', { name: 'SHA256 checksums' })
        .getAttribute('href'),
    ).toBe('https://example.test/SHA256SUMS')

    const linux = screen.getByRole('link', {
      name: 'Download for Linux x86_64',
    })
    expect(linux.getAttribute('href')).toBe('https://example.test/linux.tar.gz')
    fireEvent.click(linux)
    expect(track).toHaveBeenCalledWith('file_download', {
      name: 'flowtrace-books-0.1.0-linux-x86_64.tar.gz',
    })
  })

  it('tracks the web demo link', async () => {
    await render({ kind: 'none' })
    const demo = screen.getAllByRole('link', { name: /See the web demo/ })
    expect(demo.every((a) => a.getAttribute('href') === '/dashboard')).toBe(
      true,
    )
    fireEvent.click(demo[0])
    expect(track).toHaveBeenCalledWith('cta_click', {
      location: 'books-hero-demo',
    })
    expect(track).toHaveBeenCalledWith('demo_started', {
      location: 'books-hero-demo',
    })
  })

  it('loads only the hero screenshot eagerly and gives every image a size and alt', async () => {
    await render({ kind: 'none' })
    const images = screen.getAllByRole('img')
    expect(images).toHaveLength(8)
    expect(
      images.filter((i) => i.getAttribute('loading') === 'eager'),
    ).toHaveLength(1)
    for (const img of images) {
      expect(img.getAttribute('alt')).toBeTruthy()
      expect(img.getAttribute('width')).toBe('960')
      expect(img.getAttribute('height')).toMatch(/^\d+$/)
    }
  })
})

describe('/books head', () => {
  it('sets the title, description, canonical and Open Graph fields', async () => {
    const head = (await Route.options.head!({} as never)) as {
      meta: Array<Record<string, string>>
      links: Array<Record<string, string>>
    }
    const meta = (key: string) =>
      head.meta.find((m) => m.name === key || m.property === key)?.content
    expect(head.meta[0]).toEqual({ title: BOOKS_TITLE })
    expect(BOOKS_TITLE).toBe('FlowTrace Books · the books, offline, native')
    expect(meta('description')).toMatch(/desktop app for double-entry books/)
    expect(meta('og:url')).toBe(`${SITE_URL}/books`)
    expect(meta('og:image')).toBe(`${SITE_URL}/books/u4-general-ledger.webp`)
    expect(meta('og:image:alt')).toBeTruthy()
    expect(meta('twitter:card')).toBe('summary_large_image')
    expect(head.links).toEqual([
      { rel: 'canonical', href: `${SITE_URL}/books` },
    ])
  })
})
