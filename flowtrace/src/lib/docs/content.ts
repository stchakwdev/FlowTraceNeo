/**
 * The docs pages: every `content/docs/<slug>.md`, bundled at build time with
 * `import.meta.glob` and rendered once per server instance. Order and section
 * come from front matter. Server only: the routes reach it through
 * docs-server.ts, so neither the Markdown nor the parser ships to the browser.
 */
import { parseFrontMatter, renderMarkdown, toFrontMatter } from './markdown'
import type { DocFrontMatter, DocHeading } from './markdown'

export interface DocPage extends DocFrontMatter {
  slug: string
  html: string
  headings: Array<DocHeading>
}

export interface DocNavItem {
  slug: string
  title: string
  description: string
}

export interface DocNavSection {
  title: string
  items: Array<DocNavItem>
}

/** One search entry per page: its title, section and h2/h3 headings. */
export interface DocSearchEntry {
  slug: string
  title: string
  section: string
  headings: Array<{ id: string; text: string }>
}

const FILES = import.meta.glob<string>('/content/docs/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
})

export function slugFromPath(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1).replace(/\.md$/, '')
}

/** Parses and orders a set of files; exported for tests. */
export function buildDocs(files: Record<string, string>): Array<DocPage> {
  const pages = Object.entries(files).map(([path, raw]) => {
    const { data, body } = parseFrontMatter(raw)
    return {
      slug: slugFromPath(path),
      ...toFrontMatter(data, path),
      ...renderMarkdown(body),
    }
  })
  const orders = new Set<number>()
  for (const p of pages) {
    if (orders.has(p.order))
      throw new Error(`Two docs pages share order ${p.order}`)
    orders.add(p.order)
  }
  return pages.sort((a, b) => a.order - b.order)
}

export const DOCS: ReadonlyArray<DocPage> = buildDocs(FILES)

export const DOC_PATHS: ReadonlyArray<string> = DOCS.map(
  (d) => `/docs/${d.slug}`,
)

function navItem(d: DocPage): DocNavItem {
  return { slug: d.slug, title: d.title, description: d.description }
}

/** Sections in the order their first page appears. */
export function docsNav(
  docs: ReadonlyArray<DocPage> = DOCS,
): Array<DocNavSection> {
  const sections: Array<DocNavSection> = []
  for (const d of docs) {
    let section = sections.find((s) => s.title === d.section)
    if (!section) {
      section = { title: d.section, items: [] }
      sections.push(section)
    }
    section.items.push(navItem(d))
  }
  return sections
}

export function docsSearchIndex(
  docs: ReadonlyArray<DocPage> = DOCS,
): Array<DocSearchEntry> {
  return docs.map((d) => ({
    slug: d.slug,
    title: d.title,
    section: d.section,
    headings: d.headings.map(({ id, text }) => ({ id, text })),
  }))
}

export interface DocView {
  page: DocPage
  prev: DocNavItem | null
  next: DocNavItem | null
}

export function getDoc(
  slug: string,
  docs: ReadonlyArray<DocPage> = DOCS,
): DocView | null {
  const i = docs.findIndex((d) => d.slug === slug)
  if (i < 0) return null
  const item = (d: DocPage | undefined) => (d ? navItem(d) : null)
  return { page: docs[i], prev: item(docs[i - 1]), next: item(docs[i + 1]) }
}

/** Everything the docs frame needs on every page: the sidebar and search. */
export interface DocsShell {
  nav: Array<DocNavSection>
  search: Array<DocSearchEntry>
}

export function docsShell(docs: ReadonlyArray<DocPage> = DOCS): DocsShell {
  return { nav: docsNav(docs), search: docsSearchIndex(docs) }
}
