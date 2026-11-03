import { useEffect } from 'react'
import { Link, useRouter } from '@tanstack/react-router'
import { ArrowLeft, ArrowRight } from '@phosphor-icons/react'
import { DocsSearch } from './docs-search'
import type { DocView, DocsShell } from '@/lib/docs/content'
import type { DocHeading } from '@/lib/docs/markdown'
import { track } from '@/lib/analytics'
import { cn } from '@/lib/utils'

const QUIET_LINK =
  'underline decoration-foreground/30 underline-offset-4 hover:decoration-foreground'

// Styles for the rendered Markdown. Scoped here so the docs need no global CSS.
const PROSE = cn(
  'max-w-[42rem] text-base leading-relaxed',
  '[&_p]:mt-4 [&_ul]:mt-4 [&_ol]:mt-4 [&_ul]:list-disc [&_ol]:list-decimal [&_ul]:pl-6 [&_ol]:pl-6 [&_li]:mt-1.5',
  '[&_h2]:mt-12 [&_h2]:scroll-mt-24 [&_h2]:border-t-[3px] [&_h2]:border-foreground [&_h2]:pt-4 [&_h2]:text-3xl [&_h2]:leading-tight',
  '[&_h3]:mt-8 [&_h3]:scroll-mt-24 [&_h3]:text-xl [&_h3]:leading-snug',
  '[&_h4]:mt-6 [&_h4]:font-semibold',
  '[&_a]:font-medium [&_a]:text-primary [&_a]:underline [&_a]:decoration-primary/40 [&_a]:underline-offset-4 [&_a:hover]:decoration-primary',
  '[&_strong]:font-semibold',
  '[&_code]:font-mono [&_code]:text-[0.8125rem]',
  '[&_pre]:mt-4 [&_pre]:overflow-x-auto [&_pre]:border [&_pre]:border-border [&_pre]:bg-card [&_pre]:p-4 [&_pre]:leading-normal',
  '[&_blockquote]:mt-4 [&_blockquote]:border-y [&_blockquote]:border-border [&_blockquote]:py-3 [&_blockquote]:text-muted-foreground',
  '[&_hr]:mt-10 [&_hr]:border-border',
  '[&_table]:mt-4 [&_table]:block [&_table]:w-full [&_table]:overflow-x-auto [&_table]:border-collapse [&_table]:text-sm',
  '[&_th]:border-b-[3px] [&_th]:border-foreground [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:align-bottom [&_th]:font-semibold',
  '[&_td]:border-b [&_td]:border-border [&_td]:px-3 [&_td]:py-2 [&_td]:align-top',
  '[&_img]:mt-6 [&_img]:h-auto [&_img]:w-full [&_img]:border [&_img]:border-border',
)

/** Sidebar, content and on-page contents. The marketing layout supplies masthead and footer. */
export function DocsFrame({
  shell,
  activeSlug,
  headings = [],
  children,
}: {
  shell: DocsShell
  activeSlug?: string
  headings?: ReadonlyArray<DocHeading>
  children: React.ReactNode
}) {
  useEffect(() => {
    track('docs_viewed', { slug: activeSlug ?? 'index' })
  }, [activeSlug])

  return (
    <div className="mx-auto grid max-w-7xl gap-10 px-4 py-10 sm:px-8 lg:grid-cols-[15rem_minmax(0,1fr)] lg:py-14 xl:grid-cols-[15rem_minmax(0,1fr)_13rem]">
      <aside className="lg:sticky lg:top-6 lg:max-h-[calc(100vh-3rem)] lg:self-start lg:overflow-y-auto">
        <DocsSearch index={shell.search} />
        <details className="mt-6 border-t-[3px] border-foreground lg:hidden">
          <summary className="flex min-h-11 cursor-pointer items-center text-sm font-semibold">
            All pages
          </summary>
          <DocsNav shell={shell} activeSlug={activeSlug} />
        </details>
        <div className="mt-8 hidden lg:block">
          <DocsNav shell={shell} activeSlug={activeSlug} />
        </div>
      </aside>
      <div className="min-w-0">{children}</div>
      {headings.length > 0 && (
        <nav
          aria-label="On this page"
          className="hidden xl:sticky xl:top-6 xl:block xl:self-start"
        >
          <OnThisPage headings={headings} />
        </nav>
      )}
    </div>
  )
}

function DocsNav({
  shell,
  activeSlug,
}: {
  shell: DocsShell
  activeSlug?: string
}) {
  return (
    <nav aria-label="Documentation">
      <Link
        to="/docs"
        aria-current={activeSlug ? undefined : 'page'}
        className={cn(
          'flex min-h-10 items-center px-3 text-sm font-medium',
          activeSlug ? 'hover:bg-muted' : 'bg-foreground text-background',
        )}
      >
        Overview
      </Link>
      {shell.nav.map((section) => (
        <div key={section.title} className="mt-6">
          <p className="section-label px-3">{section.title}</p>
          <ul className="mt-2">
            {section.items.map((item) => {
              const active = item.slug === activeSlug
              return (
                <li key={item.slug}>
                  <Link
                    to="/docs/$slug"
                    params={{ slug: item.slug }}
                    aria-current={active ? 'page' : undefined}
                    className={cn(
                      'flex min-h-10 items-center px-3 text-sm',
                      active
                        ? 'bg-foreground font-semibold text-background'
                        : 'hover:bg-muted',
                    )}
                  >
                    {item.title}
                  </Link>
                </li>
              )
            })}
          </ul>
        </div>
      ))}
    </nav>
  )
}

function OnThisPage({ headings }: { headings: ReadonlyArray<DocHeading> }) {
  return (
    <>
      <p className="section-label">On this page</p>
      <ul className="mt-3 border-l border-border text-sm">
        {headings.map((h) => (
          <li key={h.id}>
            <a
              href={`#${h.id}`}
              className={cn(
                'block py-1.5 text-muted-foreground hover:text-foreground',
                h.depth === 3 ? 'pl-7' : 'pl-4',
              )}
            >
              {h.text}
            </a>
          </li>
        ))}
      </ul>
    </>
  )
}

const sectionId = (title: string) =>
  `docs-${title.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`

/** /docs: what the docs cover, page by page. */
export function DocsIndexPage({ shell }: { shell: DocsShell }) {
  return (
    <DocsFrame shell={shell}>
      <h1 className="text-[2.75rem] leading-[0.95] sm:text-6xl">
        How to use FlowTrace, screen by screen.
      </h1>
      <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
        Each page follows one part of the app, in the order of its menu. Open
        the demo next to it and try each step as you read.
      </p>
      <p className="mt-4 text-sm">
        <Link
          to="/docs/$slug"
          params={{ slug: 'getting-started' }}
          className={QUIET_LINK}
        >
          New here? Start with Getting started.
        </Link>
      </p>
      {shell.nav.map((section) => (
        <section
          key={section.title}
          aria-labelledby={sectionId(section.title)}
          className="mt-12 border-t-[3px] border-foreground"
        >
          <h2
            id={sectionId(section.title)}
            className="section-label flex min-h-11 items-center border-b border-border"
          >
            {section.title}
          </h2>
          <ul className="grid sm:grid-cols-2">
            {section.items.map((item) => (
              <li key={item.slug} className="border-b border-border">
                <Link
                  to="/docs/$slug"
                  params={{ slug: item.slug }}
                  className="group block h-full py-4 pr-4 hover:bg-muted sm:px-4"
                >
                  <span className="flex items-center gap-1.5 font-semibold">
                    {item.title}
                    <ArrowRight
                      aria-hidden
                      weight="bold"
                      className="size-3.5 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
                    />
                  </span>
                  <span className="mt-1 block text-sm text-muted-foreground">
                    {item.description}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </DocsFrame>
  )
}

/** /docs/$slug: one rendered page with its previous and next links. */
export function DocArticlePage({
  shell,
  view,
}: {
  shell: DocsShell
  view: DocView
}) {
  const { page, prev, next } = view
  const router = useRouter()

  // Internal links in the rendered HTML go through the router, not a full reload.
  const onClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const a = (e.target as HTMLElement).closest('a')
    const href = a?.getAttribute('href')
    if (
      !href?.startsWith('/') ||
      e.defaultPrevented ||
      e.button !== 0 ||
      e.metaKey ||
      e.ctrlKey ||
      e.shiftKey ||
      e.altKey
    )
      return
    e.preventDefault()
    void router.navigate({ href })
  }

  return (
    <DocsFrame shell={shell} activeSlug={page.slug} headings={page.headings}>
      <article>
        <p className="section-label text-muted-foreground">{page.section}</p>
        <h1 className="mt-3 text-[2.75rem] leading-[0.95] sm:text-6xl">
          {page.title}
        </h1>
        <p className="mt-5 max-w-[42rem] text-lg leading-relaxed text-muted-foreground">
          {page.description}
        </p>
        <div
          className={PROSE}
          onClick={onClick}
          dangerouslySetInnerHTML={{ __html: page.html }}
        />
      </article>
      <nav
        aria-label="Previous and next page"
        className="mt-16 grid max-w-[42rem] gap-4 border-t-[3px] border-foreground pt-4 sm:grid-cols-2"
      >
        {prev ? (
          <Link
            to="/docs/$slug"
            params={{ slug: prev.slug }}
            rel="prev"
            className="flex min-h-11 flex-col py-2 hover:bg-muted"
          >
            <span className="flex items-center gap-1 text-xs text-muted-foreground">
              <ArrowLeft aria-hidden className="size-3.5" /> Previous
            </span>
            <span className="font-semibold">{prev.title}</span>
          </Link>
        ) : (
          <span />
        )}
        {next && (
          <Link
            to="/docs/$slug"
            params={{ slug: next.slug }}
            rel="next"
            className="flex min-h-11 flex-col py-2 hover:bg-muted sm:items-end sm:text-right"
          >
            <span className="flex items-center gap-1 text-xs text-muted-foreground">
              Next <ArrowRight aria-hidden className="size-3.5" />
            </span>
            <span className="font-semibold">{next.title}</span>
          </Link>
        )}
      </nav>
    </DocsFrame>
  )
}
