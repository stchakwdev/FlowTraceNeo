import { Link } from '@tanstack/react-router'
import { ArrowRight } from '@phosphor-icons/react'
import { demoStarted } from '@/components/marketing/marketing-masthead'
import { buttonVariants } from '@/components/ui/button'
import { track } from '@/lib/analytics'
import { DEMO_PATH } from '@/lib/marketing-nav'
import { cn } from '@/lib/utils'

/**
 * Building blocks of the text-led marketing pages (/pricing, /security,
 * /funders, /about): a hero, ruled sections, the demo and mailto CTAs and a
 * no-JavaScript FAQ. The layout supplies masthead, main and footer.
 */

// The contact address for pilot requests and questions (coordinator, 2026-11-03).
export const CONTACT_EMAIL = 'stchakwera@gmail.com'

export function mailto(subject: string, email = CONTACT_EMAIL): string {
  return `mailto:${email}?subject=${encodeURIComponent(subject)}`
}

export const arrow =
  'size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0'
export const quietLink =
  'underline decoration-foreground/40 underline-offset-4 hover:decoration-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring'

export function PageHero({
  eyebrow,
  title,
  lede,
  children,
}: {
  eyebrow: string
  title: string
  lede: React.ReactNode
  children?: React.ReactNode
}) {
  return (
    <section className="mx-auto max-w-7xl px-4 pt-12 pb-14 sm:px-8 lg:pt-16 lg:pb-20">
      <p className="section-label">{eyebrow}</p>
      <h1 className="mt-4 max-w-4xl text-[2.75rem] leading-[0.95] sm:text-6xl lg:text-7xl">
        {title}
      </h1>
      <div className="mt-6 max-w-[38rem] text-lg leading-relaxed text-muted-foreground">
        {lede}
      </div>
      {children}
    </section>
  )
}

/** A section under a heavy rule: heading and lede on the left, body on the right. */
export function Section({
  id,
  title,
  lede,
  children,
}: {
  id: string
  title: string
  lede?: string
  children: React.ReactNode
}) {
  return (
    <section
      id={id}
      aria-labelledby={`${id}-title`}
      className="scroll-mt-28 border-t-[3px] border-foreground"
    >
      <div className="mx-auto grid max-w-7xl gap-8 px-4 py-14 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:py-20">
        <div className="lg:col-span-4">
          <h2 id={`${id}-title`} className="text-3xl leading-none sm:text-4xl">
            {title}
          </h2>
          {lede && (
            <p className="mt-4 text-base leading-relaxed text-muted-foreground">
              {lede}
            </p>
          )}
        </div>
        <div className="max-w-2xl space-y-5 text-base leading-relaxed lg:col-span-8">
          {children}
        </div>
      </div>
    </section>
  )
}

export function DemoCta({
  location,
  children = 'Explore the demo',
  quiet,
}: {
  location: string
  children?: React.ReactNode
  quiet?: boolean
}) {
  return (
    <Link
      to={DEMO_PATH}
      onClick={() => demoStarted(location)}
      className={
        quiet
          ? cn('text-base font-medium', quietLink)
          : cn(buttonVariants({ size: 'lg' }), 'group')
      }
    >
      {children}
      {!quiet && <ArrowRight className={arrow} weight="bold" />}
    </Link>
  )
}

export function MailCta({
  location,
  subject,
  children,
  quiet,
}: {
  location: string
  subject: string
  children: React.ReactNode
  quiet?: boolean
}) {
  return (
    <a
      href={mailto(subject)}
      onClick={() => track('cta_click', { location })}
      className={
        quiet
          ? cn('text-base font-medium', quietLink)
          : cn(buttonVariants({ size: 'lg', variant: 'outline' }), 'group')
      }
    >
      {children}
    </a>
  )
}

export function CtaRow({ children }: { children: React.ReactNode }) {
  return (
    <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-4">
      {children}
    </div>
  )
}

/** Questions as native disclosure widgets: they open without JavaScript. */
export function Faq({
  items,
}: {
  items: ReadonlyArray<{ q: string; a: React.ReactNode }>
}) {
  return (
    <div className="border-b border-border">
      {items.map((item) => (
        <details key={item.q} className="group border-t border-border">
          <summary className="flex min-h-12 cursor-pointer list-none items-center justify-between gap-4 py-3 font-semibold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
            {item.q}
            <span
              aria-hidden="true"
              className="font-mono text-lg leading-none text-muted-foreground group-open:hidden"
            >
              +
            </span>
            <span
              aria-hidden="true"
              className="hidden font-mono text-lg leading-none text-muted-foreground group-open:inline"
            >
              -
            </span>
          </summary>
          <div className="pb-5 leading-relaxed text-muted-foreground">
            {item.a}
          </div>
        </details>
      ))}
    </div>
  )
}
