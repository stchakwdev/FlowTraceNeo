import { useId, useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { MagnifyingGlass } from '@phosphor-icons/react'
import type { DocSearchEntry } from '@/lib/docs/content'

export interface DocSearchHit {
  slug: string
  hash?: string
  title: string
  context: string
}

const MAX_HITS = 8

/**
 * Matches every word of the query against page titles and headings, page
 * titles first. The index is built with the pages, so search needs no service.
 */
export function searchDocs(
  index: ReadonlyArray<DocSearchEntry>,
  query: string,
): Array<DocSearchHit> {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  if (!words.length) return []
  const matches = (text: string) => {
    const t = text.toLowerCase()
    return words.every((w) => t.includes(w))
  }
  const pages: Array<DocSearchHit> = []
  const headings: Array<DocSearchHit> = []
  for (const entry of index) {
    if (matches(entry.title))
      pages.push({
        slug: entry.slug,
        title: entry.title,
        context: entry.section,
      })
    for (const h of entry.headings) {
      if (matches(h.text))
        headings.push({
          slug: entry.slug,
          hash: h.id,
          title: h.text,
          context: entry.title,
        })
    }
  }
  return [...pages, ...headings].slice(0, MAX_HITS)
}

export function DocsSearch({
  index,
}: {
  index: ReadonlyArray<DocSearchEntry>
}) {
  const [query, setQuery] = useState('')
  const id = useId()
  const hits = useMemo(() => searchDocs(index, query), [index, query])
  const searching = query.trim().length > 0

  return (
    <div role="search">
      <label htmlFor={id} className="section-label block">
        Search the docs
      </label>
      <div className="relative mt-2">
        <MagnifyingGlass
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
        />
        <input
          id={id}
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Proofs, periods, M-Pesa"
          autoComplete="off"
          className="h-10 w-full border border-input bg-card pr-3 pl-9 text-sm outline-none focus-visible:border-primary focus-visible:ring-1 focus-visible:ring-primary/50"
        />
      </div>
      <p aria-live="polite" className="sr-only">
        {searching ? `${hits.length} results` : ''}
      </p>
      {searching && (
        <ul className="mt-2 border-t border-border">
          {hits.length === 0 ? (
            <li className="py-3 text-sm text-muted-foreground">
              No page or heading matches “{query.trim()}”.
            </li>
          ) : (
            hits.map((hit) => (
              <li
                key={`${hit.slug}#${hit.hash ?? ''}`}
                className="border-b border-border"
              >
                <Link
                  to="/docs/$slug"
                  params={{ slug: hit.slug }}
                  hash={hit.hash}
                  onClick={() => setQuery('')}
                  className="block min-h-11 py-2 hover:bg-muted"
                >
                  <span className="block text-sm font-medium">{hit.title}</span>
                  <span className="block text-xs text-muted-foreground">
                    {hit.context}
                  </span>
                </Link>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  )
}
