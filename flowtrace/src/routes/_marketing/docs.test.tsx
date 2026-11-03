import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen, within } from '@testing-library/react'
import readability from 'text-readability'
import { renderWithRouter } from '@/test/render-with-router'
import { SITE_URL } from '@/lib/site'
import { DOCS, docsShell, getDoc } from '@/lib/docs/content'

const track = vi.hoisted(() => vi.fn())
vi.mock('@/lib/analytics', () => ({ track }))
// The loaders are server functions; the pages only take their result.
vi.mock('@/lib/docs/docs-server', () => ({
  getDocsShell: vi.fn(),
  getDocPage: vi.fn(),
}))

const { DocArticlePage, DocsIndexPage } =
  await import('@/components/docs/docs-page')
const {
  DOCS_DESCRIPTION,
  DOCS_TITLE,
  Route: IndexRoute,
} = await import('./docs.index')
const { Route: SlugRoute } = await import('./docs.$slug')

const shell = docsShell()

afterEach(() => {
  cleanup()
  track.mockClear()
})

type Head = {
  meta: Array<Record<string, string>>
  links: Array<Record<string, string>>
}
const metaOf = (head: Head, key: string) =>
  head.meta.find((m) => m.name === key || m.property === key)?.content

describe('/docs', () => {
  it('lists every page under its section heading', async () => {
    await renderWithRouter(() => <DocsIndexPage shell={shell} />)
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe(
      'How to use FlowTrace, screen by screen.',
    )
    expect(
      screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent),
    ).toEqual(shell.nav.map((s) => s.title))
    const main = screen.getByRole('heading', { level: 1 }).parentElement!
    for (const d of DOCS)
      expect(
        within(main)
          .getByRole('link', { name: new RegExp(`^${d.title}`) })
          .getAttribute('href'),
      ).toBe(`/docs/${d.slug}`)
    expect(track).toHaveBeenCalledWith('docs_viewed', { slug: 'index' })
  })

  it('sets the title, description and canonical', () => {
    const head = IndexRoute.options.head!({} as never) as Head
    expect(head.meta[0]).toEqual({ title: `${DOCS_TITLE} · FlowTrace` })
    expect(metaOf(head, 'description')).toBe(DOCS_DESCRIPTION)
    expect(head.links).toEqual([{ rel: 'canonical', href: `${SITE_URL}/docs` }])
  })

  it('searches titles and headings without a service', async () => {
    await renderWithRouter(() => <DocsIndexPage shell={shell} />)
    const search = screen.getByRole('searchbox', { name: 'Search the docs' })
    fireEvent.change(search, { target: { value: 'trust score' } })
    const results = within(screen.getByRole('search')).getAllByRole('link')
    expect(results[0].getAttribute('href')).toBe('/docs/trust-score')
    fireEvent.change(search, { target: { value: 'zzqx' } })
    expect(screen.getByText(/No page or heading matches/)).toBeTruthy()
  })
})

describe('/docs/$slug', () => {
  const view = getDoc('trust-score')!

  it('renders the page, its contents and previous and next links', async () => {
    await renderWithRouter(() => <DocArticlePage shell={shell} view={view} />)
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe(
      'Trust Score',
    )
    const h2s = screen
      .getAllByRole('heading', { level: 2 })
      .map((h) => h.textContent)
    for (const h of view.page.headings.filter((x) => x.depth === 2))
      expect(h2s).toContain(h.text)
    const toc = screen.getByRole('navigation', { name: 'On this page' })
    expect(within(toc).getAllByRole('link')[0].getAttribute('href')).toBe(
      `#${view.page.headings[0].id}`,
    )
    const pager = screen.getByRole('navigation', {
      name: 'Previous and next page',
    })
    expect(
      within(pager)
        .getAllByRole('link')
        .map((a) => a.getAttribute('href')),
    ).toEqual([`/docs/${view.prev!.slug}`, `/docs/${view.next!.slug}`])
    // One copy for phones (inside a disclosure), one for wide screens.
    for (const nav of screen.getAllByRole('navigation', {
      name: 'Documentation',
    }))
      expect(nav.querySelector('[aria-current="page"]')?.textContent).toBe(
        'Trust Score',
      )
    expect(track).toHaveBeenCalledWith('docs_viewed', { slug: 'trust-score' })
  })

  it.each(DOCS.map((d) => d.slug))(
    '%s: head() comes from its front matter',
    (slug) => {
      const loaded = getDoc(slug)!
      const head = SlugRoute.options.head!({
        loaderData: { shell, view: loaded },
        params: { slug },
      } as never) as Head
      expect(head.meta[0]).toEqual({
        title: `${loaded.page.title} · FlowTrace`,
      })
      expect(metaOf(head, 'description')).toBe(loaded.page.description)
      expect(metaOf(head, 'og:url')).toBe(`${SITE_URL}/docs/${slug}`)
      expect(head.links).toEqual([
        { rel: 'canonical', href: `${SITE_URL}/docs/${slug}` },
      ])
    },
  )
})

/**
 * A page's prose: each paragraph and list item as its own sentence (a line
 * without end punctuation, such as a list label, gets a full stop). Tables,
 * code and headings are not prose.
 */
function prose(slug: string): string {
  const div = document.createElement('div')
  div.innerHTML = getDoc(slug)!.page.html
  div.querySelectorAll('table, pre, h2, h3, h4').forEach((n) => n.remove())
  return [...div.querySelectorAll('p, li')]
    .filter((n) => !n.querySelector('p'))
    .map((n) => n.textContent.replace(/\s+/g, ' ').trim())
    .filter(Boolean)
    .map((t) => (/[.?!:]$/.test(t) ? t : `${t}.`))
    .join(' ')
}

describe('reading level', () => {
  it.each(DOCS.map((d) => d.slug))(
    '%s reads at grade 7 or below (Flesch-Kincaid)',
    (slug) => {
      const grade = readability.fleschKincaidGrade(prose(slug))
      console.info(`${slug}: Flesch-Kincaid grade ${grade}`)
      expect(grade).toBeLessThanOrEqual(7)
    },
  )
})
