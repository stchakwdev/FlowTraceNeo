/**
 * Sitemap Footer (Kokonut UI Pro `sitemap-footer`, restyled to DESIGN.md)
 *
 * The site's columns of links and a way through them: type in "Jump to" (or
 * press / while the footer is on screen) and the matching links stay lit
 * with the match underlined while the rest fade. Enter or the down arrow
 * moves to the first match, Escape clears.
 *
 * FlowTrace changes: report tokens instead of zinc, square corners, Phosphor
 * icons, plain anchors (several targets are future pages), English only (no
 * locale menu), one social link (the GitHub repository), a demo CTA and an
 * optional "Demo figures as of" note.
 */

import { useEffect, useId, useMemo, useRef, useState } from 'react'
import { Link } from '@tanstack/react-router'
import { ArrowRight, GithubLogo } from '@phosphor-icons/react'
import type { KeyboardEvent } from 'react'
import { AsOf } from '@/components/report'
import { Wordmark } from '@/components/layout/wordmark'
import { buttonVariants } from '@/components/ui/button'
import { track } from '@/lib/analytics'
import { cn } from '@/lib/utils'

export interface SitemapLink {
  label: string
  href: string
  external?: boolean
}

export interface SitemapColumn {
  title: string
  links: ReadonlyArray<SitemapLink>
}

interface SitemapFooterProps {
  columns: ReadonlyArray<SitemapColumn>
  /** Where "Explore the demo" goes. */
  demoPath: string
  onDemo?: () => void
  githubUrl: string
  /** Shown as "Demo figures as of <date>" when the page shows demo figures. */
  demoAsOf?: string
  className?: string
}

const FOCUS =
  'outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring'

/** Underlines the matched part of a link name. */
function Highlight({ text, query }: { text: string; query: string }) {
  const at = query ? text.toLowerCase().indexOf(query.toLowerCase()) : -1
  if (at < 0) {
    return <>{text}</>
  }
  return (
    <>
      {text.slice(0, at)}
      <span className="underline decoration-2 decoration-primary underline-offset-[3px]">
        {text.slice(at, at + query.length)}
      </span>
      {text.slice(at + query.length)}
    </>
  )
}

export default function SitemapFooter({
  columns,
  demoPath,
  onDemo,
  githubUrl,
  demoAsOf,
  className,
}: SitemapFooterProps) {
  const [query, setQuery] = useState('')
  const footerRef = useRef<HTMLElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const visibleRef = useRef(false)
  const inputId = useId()
  const countId = useId()

  const q = query.trim().toLowerCase()
  const total = useMemo(
    () => columns.reduce((sum, column) => sum + column.links.length, 0),
    [columns],
  )
  const isMatch = (link: SitemapLink, column: SitemapColumn) =>
    !q ||
    link.label.toLowerCase().includes(q) ||
    column.title.toLowerCase().includes(q)
  const matches = q
    ? columns.reduce(
        (sum, column) =>
          sum + column.links.filter((link) => isMatch(link, column)).length,
        0,
      )
    : total

  // "/" focuses the filter, but only while this footer is on screen, so it
  // doesn't steal the key from the rest of the page.
  useEffect(() => {
    const footer = footerRef.current
    if (!footer || typeof IntersectionObserver === 'undefined') {
      return
    }
    const observer = new IntersectionObserver(([entry]) => {
      visibleRef.current = entry.isIntersecting
    })
    observer.observe(footer)
    const onKey = (event: globalThis.KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      const typing =
        target?.isContentEditable ||
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target?.tagName ?? '')
      if (event.key === '/' && visibleRef.current && !typing) {
        event.preventDefault()
        inputRef.current?.focus()
      }
    }
    document.addEventListener('keydown', onKey)
    return () => {
      observer.disconnect()
      document.removeEventListener('keydown', onKey)
    }
  }, [])

  const focusFirstMatch = () => {
    footerRef.current
      ?.querySelector<HTMLAnchorElement>("[data-sitemap-match='true']")
      ?.focus()
  }

  const onInputKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape') {
      if (query) {
        event.preventDefault()
        setQuery('')
      } else {
        inputRef.current?.blur()
      }
    } else if ((event.key === 'Enter' || event.key === 'ArrowDown') && q) {
      event.preventDefault()
      focusFirstMatch()
    }
  }

  return (
    <footer
      className={cn(
        '@container w-full border-t-[3px] border-foreground bg-background',
        className,
      )}
      ref={footerRef}
    >
      <div className="mx-auto max-w-7xl px-4 sm:px-8">
        <div className="flex flex-col justify-between gap-6 pt-12 pb-10 @3xl:flex-row @3xl:items-center @3xl:pt-14">
          <a
            href="/"
            aria-label="FlowTrace home"
            className={cn('w-fit', FOCUS)}
          >
            <Wordmark />
          </a>

          <div className="flex flex-col gap-4 @lg:flex-row @lg:items-center">
            <div className="relative w-full @3xl:w-64">
              <label className="sr-only" htmlFor={inputId}>
                Jump to a page
              </label>
              {/* text-base keeps iOS Safari from zooming on focus. */}
              <input
                aria-describedby={countId}
                autoComplete="off"
                className="h-10 w-full border border-input bg-background pr-16 pl-3 text-base text-foreground outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 @3xl:text-sm"
                id={inputId}
                onChange={(event) => setQuery(event.target.value)}
                onKeyDown={onInputKey}
                placeholder="Jump to…"
                ref={inputRef}
                spellCheck={false}
                type="search"
                value={query}
              />
              <span
                aria-live="polite"
                className="pointer-events-none absolute inset-y-0 right-3 flex items-center font-mono text-xs text-muted-foreground tabular-nums"
                id={countId}
              >
                {q ? (
                  <span>
                    {matches}
                    <span className="sr-only"> of {total} pages match</span>
                    <span aria-hidden="true">/{total}</span>
                  </span>
                ) : (
                  <kbd className="grid h-5 min-w-5 place-items-center border border-border px-1 font-mono text-[11px]">
                    /
                  </kbd>
                )}
              </span>
            </div>
            <Link
              to={demoPath}
              onClick={onDemo}
              className={cn(buttonVariants(), 'group w-fit shrink-0')}
            >
              Explore the demo
              <ArrowRight
                className="size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
                weight="bold"
              />
            </Link>
          </div>
        </div>

        <nav
          aria-label="Site"
          className="grid grid-cols-2 gap-x-8 gap-y-10 border-t border-border pt-10 pb-12 @lg:grid-cols-4"
        >
          {columns.map((column) => {
            const columnHit = column.links.some((link) => isMatch(link, column))
            return (
              <div className="flex flex-col gap-3" key={column.title}>
                <h2
                  className={cn(
                    'section-label transition-opacity duration-200 motion-reduce:transition-none',
                    !columnHit && 'opacity-30',
                  )}
                >
                  {column.title}
                </h2>
                <ul className="flex flex-col gap-1">
                  {column.links.map((link) => {
                    const hit = isMatch(link, column)
                    return (
                      <li key={link.label}>
                        <a
                          className={cn(
                            'inline-flex min-h-6 items-center text-sm transition-[color,opacity] duration-200 hover:text-foreground hover:underline motion-reduce:transition-none',
                            q && hit
                              ? 'text-foreground'
                              : 'text-muted-foreground',
                            !hit && 'opacity-25',
                            FOCUS,
                          )}
                          data-sitemap-match={q && hit ? 'true' : undefined}
                          href={link.href}
                          {...(link.external
                            ? {
                                rel: 'noopener noreferrer',
                                onClick: () =>
                                  track('outbound_click', { href: link.href }),
                              }
                            : {})}
                        >
                          <Highlight query={q} text={link.label} />
                        </a>
                      </li>
                    )
                  })}
                </ul>
              </div>
            )
          })}
        </nav>

        <div className="flex flex-col-reverse justify-between gap-4 border-t border-border py-6 text-xs text-muted-foreground @3xl:flex-row @3xl:items-center">
          <p>
            &copy; {new Date().getFullYear()} FlowTrace
            {demoAsOf && (
              <>
                {' · '}Demo figures <AsOf date={demoAsOf} className="text-xs" />
              </>
            )}
          </p>
          <a
            aria-label="FlowTrace on GitHub"
            className={cn(
              'grid size-9 place-items-center text-muted-foreground hover:text-foreground',
              FOCUS,
            )}
            href={githubUrl}
            rel="noopener noreferrer"
            onClick={() => track('outbound_click', { href: githubUrl })}
          >
            <GithubLogo aria-hidden="true" className="size-5" />
          </a>
        </div>
      </div>
    </footer>
  )
}
