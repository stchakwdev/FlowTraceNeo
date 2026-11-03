import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import {
  DOCS,
  DOC_PATHS,
  buildDocs,
  docsNav,
  docsSearchIndex,
  getDoc,
} from './content'
import { REQUIRED_FRONT_MATTER, parseFrontMatter } from './markdown'
import { navItems } from '@/lib/navigation'
import { SITEMAP_PATHS, sitemapXml } from '@/lib/site'

const DIR = 'content/docs'
const files = readdirSync(DIR).filter((f) => f.endsWith('.md'))

// The URL paths routeTree.gen.ts knows, e.g. '/', '/books', '/ledger/accounts'.
function routePaths(): Set<string> {
  const tree = readFileSync('src/routeTree.gen.ts', 'utf8')
  const block = tree.slice(
    tree.indexOf('export interface FileRoutesByFullPath'),
  )
  const body = block.slice(0, block.indexOf('}'))
  return new Set([...body.matchAll(/'([^']+)':/g)].map((m) => m[1]))
}

describe('docs content files', () => {
  it('has the guide pages in the app rail order', () => {
    const rail = navItems.map((i) => i.href.slice(1))
    const slugs = DOCS.map((d) => d.slug)
    expect(slugs[0]).toBe('getting-started')
    // Each rail entry has a page, in rail order.
    const railPages = slugs.filter((s) => rail.includes(s))
    expect(railPages).toEqual(rail)
    expect(slugs).toEqual(
      expect.arrayContaining(['flowtrace-books', 'glossary']),
    )
  })

  it.each(files)('%s has title, description, order and section', (file) => {
    const { data } = parseFrontMatter(readFileSync(`${DIR}/${file}`, 'utf8'))
    for (const key of REQUIRED_FRONT_MATTER) expect(data[key], key).toBeTruthy()
    expect(Number.isInteger(Number(data.order))).toBe(true)
    expect(data.description.length).toBeLessThanOrEqual(160)
  })

  it.each(files)(
    '%s says which source files it was verified against',
    (file) => {
      const { body } = parseFrontMatter(readFileSync(`${DIR}/${file}`, 'utf8'))
      const note = /^\s*<!-- Verified against: (.+?) -->/.exec(body)
      expect(note, 'first line is the Verified against comment').toBeTruthy()
      for (const path of note![1].split(',').map((p) => p.trim()))
        expect(existsSync(`../${path}`), path).toBe(true)
    },
  )

  it('every internal link resolves to a route, a docs page or a heading on it', () => {
    const routes = routePaths()
    const bySlug = new Map(DOCS.map((d) => [d.slug, d]))
    const broken: Array<string> = []
    for (const doc of DOCS) {
      for (const [, href] of doc.html.matchAll(/href="([^"]+)"/g)) {
        if (/^(https?:|mailto:)/.test(href)) continue
        const [path, hash] = href.split('#')
        if (!path) {
          if (!doc.headings.some((h) => h.id === hash))
            broken.push(`${doc.slug}: ${href}`)
          continue
        }
        const docMatch = /^\/docs\/([^/]+)$/.exec(path)
        if (docMatch) {
          const target = bySlug.get(docMatch[1])
          const ok =
            target && (!hash || new RegExp(`id="${hash}"`).test(target.html))
          if (!ok) broken.push(`${doc.slug}: ${href}`)
        } else if (!routes.has(path)) broken.push(`${doc.slug}: ${href}`)
      }
    }
    expect(broken).toEqual([])
  })

  it('every image is a sized, lazy WebP in public/ with alt text', () => {
    for (const doc of DOCS) {
      for (const [tag] of doc.html.matchAll(/<img [^>]+>/g)) {
        const src = /src="([^"]+)"/.exec(tag)![1]
        expect(src, doc.slug).toMatch(/^\/docs\/img\/.+\.webp$/)
        expect(existsSync(`public${src}`), src).toBe(true)
        expect(tag).toMatch(/alt="[^"]{10,}"/)
        expect(tag).toMatch(/width="\d+" height="\d+"/)
        expect(tag).toContain('loading="lazy"')
      }
      expect(
        (doc.html.match(/<img /g) ?? []).length,
        `${doc.slug} has at most two screenshots`,
      ).toBeLessThanOrEqual(2)
    }
  })

  it('has no emoji and no unresolved placeholder other than TODO-USER', () => {
    for (const doc of DOCS) {
      expect(doc.html, doc.slug).not.toMatch(/\p{Extended_Pictographic}/u)
      expect(doc.html, doc.slug).not.toMatch(/\b(TODO|TBD|FIXME)\b(?!-USER)/)
    }
  })
})

describe('docs index, navigation and search', () => {
  const sample = buildDocs({
    '/content/docs/b.md':
      '---\ntitle: B\ndescription: d\norder: 2\nsection: One\n---\n## Bee\n',
    '/content/docs/a.md':
      '---\ntitle: A\ndescription: d\norder: 1\nsection: One\n---\n## Ay\n### Ay two\n',
    '/content/docs/c.md':
      '---\ntitle: C\ndescription: d\norder: 3\nsection: Two\n---\nText\n',
  })

  it('orders pages by front matter and groups them by section', () => {
    expect(sample.map((d) => d.slug)).toEqual(['a', 'b', 'c'])
    expect(docsNav(sample).map((s) => [s.title, s.items.length])).toEqual([
      ['One', 2],
      ['Two', 1],
    ])
  })

  it('links each page to the one before and after it', () => {
    expect(getDoc('a', sample)).toMatchObject({
      prev: null,
      next: { slug: 'b' },
    })
    expect(getDoc('c', sample)).toMatchObject({
      prev: { slug: 'b' },
      next: null,
    })
    expect(getDoc('missing', sample)).toBeNull()
  })

  it('indexes titles and headings for search', () => {
    expect(docsSearchIndex(sample)[0]).toEqual({
      slug: 'a',
      title: 'A',
      section: 'One',
      headings: [
        { id: 'ay', text: 'Ay' },
        { id: 'ay-two', text: 'Ay two' },
      ],
    })
  })

  it('refuses two pages with the same order', () => {
    expect(() =>
      buildDocs({
        '/content/docs/a.md':
          '---\ntitle: A\ndescription: d\norder: 1\nsection: S\n---\n',
        '/content/docs/b.md':
          '---\ntitle: B\ndescription: d\norder: 1\nsection: S\n---\n',
      }),
    ).toThrow('share order 1')
  })

  it('puts /docs and every docs page in the sitemap', () => {
    expect(SITEMAP_PATHS).toContain('/docs')
    expect(DOC_PATHS).toHaveLength(files.length)
    const xml = sitemapXml([...SITEMAP_PATHS, ...DOC_PATHS])
    for (const p of DOC_PATHS) expect(xml).toContain(`${p}</loc>`)
    expect(readFileSync('server/routes/sitemap.xml.ts', 'utf8')).toContain(
      '...DOC_PATHS',
    )
  })

  it('has the Trust Score methodology at /docs/trust-score', () => {
    expect(getDoc('trust-score')?.page.title).toBe('Trust Score')
  })
})
