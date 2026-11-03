/**
 * Minimal FAQ (Kokonut UI Pro `minimal-faq`, restyled to DESIGN.md)
 *
 * The heading on one side, the questions on the other, split by hairlines.
 * One answer is open at a time; while one is open the other questions step
 * back in colour.
 *
 * Every question has its own id, so `/#faq-is-it-free` opens that answer on
 * load, and an open answer can copy a link to itself.
 *
 * FlowTrace changes: Phosphor icons in place of lucide, square corners and
 * report tokens, no team avatars, and no Motion. Every answer is in the
 * server-rendered markup; a closed one collapses through a 200 ms grid-row
 * transition (none under reduced motion) and is `inert`.
 */

import { useEffect, useId, useRef, useState } from 'react'
import { Check, Link as LinkIcon } from '@phosphor-icons/react'
import { cn } from '@/lib/utils'

interface FaqItem {
  question: string
  answer: string
  /** Used for the link to this answer. Derived from the question when left out. */
  slug?: string
}

export interface MinimalFaqProps {
  title: string
  description?: string
  items: ReadonlyArray<FaqItem>
  /** Where "Ask us" points. Hidden when empty. */
  contactHref?: string
  contactLabel?: string
  /** Slug of the answer open on first render, with or without the `faq-` prefix. */
  defaultOpen?: string | null
  className?: string
}

const SLUG_STRIP = /[^a-z0-9]+/g
const SLUG_TRIM = /^-+|-+$/g

const slugify = (text: string) =>
  text.toLowerCase().replace(SLUG_STRIP, '-').replace(SLUG_TRIM, '')

const SLUG_PREFIX = 'faq-'

const slugOf = (item: FaqItem) =>
  `${SLUG_PREFIX}${item.slug ?? slugify(item.question).split('-').slice(0, 4).join('-')}`

/** Accepts "is-it-free" or "faq-is-it-free" for `defaultOpen`. */
const toSlug = (value: string | null) => {
  if (!value) return null
  return value.startsWith(SLUG_PREFIX) ? value : `${SLUG_PREFIX}${value}`
}

const COPIED_MS = 1600

const focusRing =
  'outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring'

/** A plus whose vertical stroke turns flat to make a minus. */
function PlusMinus({ open }: { open: boolean }) {
  return (
    <span
      aria-hidden="true"
      className="relative flex size-6 shrink-0 items-center justify-center"
    >
      <span className="absolute h-0.5 w-3 bg-current" />
      <span
        className={cn(
          'absolute h-3 w-0.5 bg-current transition-[rotate] duration-200 motion-reduce:transition-none',
          open && 'rotate-90',
        )}
      />
    </span>
  )
}

type CopyState = 'idle' | 'copied' | 'fallback'

const COPY_LABEL: Record<CopyState, string> = {
  idle: 'Copy link',
  copied: 'Link copied',
  fallback: 'Link is in the address bar',
}

function CopyLink({ slug }: { slug: string }) {
  const [state, setState] = useState<CopyState>('idle')
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current)
    },
    [],
  )

  const copy = async () => {
    const url = `${window.location.origin}${window.location.pathname}${window.location.search}#${slug}`
    let next: CopyState = 'copied'
    try {
      await navigator.clipboard.writeText(url)
    } catch {
      // Clipboard blocked (permissions, insecure context): put the link in
      // the address bar instead, without scrolling or adding history.
      window.history.replaceState(window.history.state, '', `#${slug}`)
      next = 'fallback'
    }
    setState(next)
    if (timer.current) clearTimeout(timer.current)
    timer.current = setTimeout(() => setState('idle'), COPIED_MS)
  }

  const Icon = state === 'idle' ? LinkIcon : Check

  return (
    <button
      className={cn(
        '-ml-2 flex h-7 min-w-6 items-center gap-1.5 px-2 text-xs text-muted-foreground transition-colors duration-150 hover:bg-wash hover:text-foreground',
        focusRing,
      )}
      onClick={copy}
      type="button"
    >
      <Icon aria-hidden="true" className="size-3.5" weight="bold" />
      <span aria-live="polite">{COPY_LABEL[state]}</span>
    </button>
  )
}

export default function MinimalFaq({
  title,
  description,
  items,
  contactHref = '',
  contactLabel = 'Ask us',
  defaultOpen = null,
  className,
}: MinimalFaqProps) {
  const id = useId()
  const slugs = items.map(slugOf)
  const [open, setOpen] = useState<string | null>(() => toSlug(defaultOpen))
  const listRef = useRef<HTMLUListElement>(null)

  // Open the answer a link points at. Runs after hydration so the server
  // markup never depends on the URL.
  useEffect(() => {
    const fromHash = window.location.hash.slice(1)
    if (!fromHash) return
    const target = document.getElementById(fromHash)
    if (target && listRef.current?.contains(target)) {
      setOpen(fromHash)
      target.scrollIntoView({ block: 'center' })
    }
  }, [])

  return (
    <section
      aria-labelledby={`${id}-title`}
      className={cn('border-t-[3px] border-foreground', className)}
    >
      <div className="mx-auto grid max-w-7xl gap-10 px-4 py-16 sm:px-8 lg:grid-cols-12 lg:py-20">
        <div className="lg:sticky lg:top-24 lg:col-span-5 lg:self-start">
          <h2 className="text-4xl leading-none sm:text-5xl" id={`${id}-title`}>
            {title}
          </h2>
          {description && (
            <p className="mt-5 max-w-sm text-base leading-relaxed text-muted-foreground">
              {description}
            </p>
          )}
          {contactHref && (
            <a
              className={cn(
                'mt-5 inline-flex h-10 items-center px-4 text-sm font-semibold underline decoration-foreground/40 hover:decoration-foreground',
                focusRing,
              )}
              href={contactHref}
            >
              {contactLabel}
            </a>
          )}
        </div>

        <ul className="border-t border-border lg:col-span-7" ref={listRef}>
          {items.map((item, index) => {
            const slug = slugs[index]
            const isOpen = open === slug
            const dimmed = open !== null && !isOpen
            return (
              <li
                className="scroll-mt-24 border-b border-border"
                id={slug}
                key={slug}
              >
                <h3 className="font-sans text-base [font-variation-settings:'wdth'_100]">
                  <button
                    aria-controls={`${slug}-answer`}
                    aria-expanded={isOpen}
                    className={cn(
                      'group flex min-h-12 w-full items-center justify-between gap-6 py-4 text-left',
                      focusRing,
                    )}
                    id={`${slug}-question`}
                    onClick={() => setOpen(isOpen ? null : slug)}
                    type="button"
                  >
                    <span
                      className={cn(
                        'text-base leading-snug font-semibold transition-colors duration-200 motion-reduce:transition-none',
                        dimmed
                          ? 'text-muted-foreground group-hover:text-foreground'
                          : 'text-foreground',
                      )}
                    >
                      {item.question}
                    </span>
                    <PlusMinus open={isOpen} />
                  </button>
                </h3>
                <div
                  aria-labelledby={`${slug}-question`}
                  className={cn(
                    'grid transition-[grid-template-rows] duration-200 motion-reduce:transition-none',
                    isOpen ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]',
                  )}
                  id={`${slug}-answer`}
                  inert={!isOpen}
                  role="region"
                >
                  <div className="overflow-hidden">
                    <div className="flex max-w-[60ch] flex-col items-start gap-3 pr-10 pb-6">
                      <p className="text-base leading-relaxed text-muted-foreground">
                        {item.answer}
                      </p>
                      <CopyLink slug={slug} />
                    </div>
                  </div>
                </div>
              </li>
            )
          })}
        </ul>
      </div>
    </section>
  )
}
