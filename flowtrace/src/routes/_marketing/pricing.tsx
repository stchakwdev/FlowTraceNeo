import { useEffect } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { Check } from '@phosphor-icons/react'
import {
  CtaRow,
  DemoCta,
  Faq,
  MailCta,
  Section,
  quietLink,
} from '@/components/marketing/page-parts'
import { track } from '@/lib/analytics'
import { pageHead } from '@/lib/site'

export const PRICING_TITLE = 'Pricing'
export const PRICING_DESCRIPTION =
  'FlowTrace is free during the pilot: the books, the public pages and the funder views, with no card needed. Pricing is not set yet.'

export const Route = createFileRoute('/_marketing/pricing')({
  head: () =>
    pageHead({
      title: PRICING_TITLE,
      description: PRICING_DESCRIPTION,
      path: '/pricing',
    }),
  component: PricingPage,
})

const PILOT_SUBJECT = 'Joining the FlowTrace pilot'

const YOU_GET = [
  'The books: double-entry accounts, grants, budgets, spend claims, bills and bank reconciliation.',
  'The public pages: donation pages, embeddable campaign cards and a trust badge.',
  'The funder views: link your books to a funder’s or a partner’s and choose what each one sees.',
  'A direct line to the people who build FlowTrace.',
]

const YOU_GIVE = [
  'Feedback on what works and what does not.',
  'A call with us once a month.',
]

const FAQS = [
  {
    q: 'Is it really free?',
    a: 'Yes. During the pilot every part of FlowTrace is free, and we do not ask for a card.',
  },
  {
    q: 'What happens when the pilot ends?',
    a: 'Pricing is not set yet. Our intent is to keep FlowTrace free for small organisations.',
  },
  {
    q: 'Can a funder pay for its grantees?',
    a: 'That is the plan once pricing exists: a funder sponsors the seats of the organisations it funds, so they do not pay.',
  },
  {
    q: 'Can I try it before I join?',
    a: 'Yes. The demo opens the books of a made-up NGO in Kenya, proofs included.',
  },
]

function PricingPage() {
  useEffect(() => track('pricing_viewed'), [])

  return (
    <>
      <section className="mx-auto grid max-w-7xl gap-12 px-4 pt-12 pb-16 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:pt-16 lg:pb-20">
        <div className="lg:col-span-5">
          <p className="section-label">Pricing</p>
          <h1 className="mt-4 text-[2.75rem] leading-[0.95] sm:text-6xl lg:text-7xl">
            Free during the pilot.
          </h1>
          <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
            Everything FlowTrace does is included. No card needed. In return, we
            ask pilot organisations to tell us what works.
          </p>
          <CtaRow>
            <DemoCta location="pricing-hero" />
            <MailCta location="pricing-hero-pilot" subject={PILOT_SUBJECT}>
              Join the pilot
            </MailCta>
          </CtaRow>
        </div>

        <article
          aria-labelledby="pilot-card"
          className="border-[3px] border-foreground lg:col-span-7"
        >
          <header className="flex flex-wrap items-baseline justify-between gap-3 border-b border-border px-6 py-5">
            <h2 id="pilot-card" className="text-3xl leading-none">
              The pilot
            </h2>
            <span className="border border-foreground px-2 py-0.5 text-[0.6875rem] font-semibold tracking-[0.08em] uppercase">
              Pilot stage
            </span>
          </header>
          <div className="px-6 py-5">
            <p className="text-4xl leading-none font-semibold tabular-nums">
              Free
            </p>
            <p className="mt-2 text-sm text-muted-foreground">
              Everything included. No card needed.
            </p>
          </div>
          <div className="grid gap-8 border-t border-border px-6 py-6 sm:grid-cols-2">
            <div>
              <h3 className="section-label">What you get</h3>
              <ul className="mt-3 space-y-3 text-sm leading-relaxed">
                {YOU_GET.map((item) => (
                  <li key={item} className="flex gap-3">
                    <Check
                      aria-hidden="true"
                      weight="bold"
                      className="mt-0.5 size-4 shrink-0 text-primary"
                    />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div>
              <h3 className="section-label">What you give</h3>
              <ul className="mt-3 list-disc space-y-3 pl-5 text-sm leading-relaxed">
                {YOU_GIVE.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          </div>
        </article>
      </section>

      <Section id="later" title="What will it cost later?">
        <p>
          Pricing is not set yet. Our intent is to keep FlowTrace free for small
          organisations, and to let funders sponsor the seats of the
          organisations they fund. We have no prices to share yet.
        </p>
        <p className="text-muted-foreground">
          Questions about cost?{' '}
          <MailCta
            location="pricing-later"
            subject="FlowTrace pricing question"
            quiet
          >
            Write to us
          </MailCta>
          .
        </p>
      </Section>

      <Section id="faq" title="Questions">
        <Faq items={FAQS} />
      </Section>

      <section className="border-t-[3px] border-foreground">
        <div className="mx-auto flex max-w-7xl flex-col gap-8 px-4 py-14 sm:px-8 md:flex-row md:items-end md:justify-between lg:py-20">
          <div className="max-w-2xl">
            <h2 className="text-4xl leading-none sm:text-5xl">
              Look around the demo first.
            </h2>
            <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
              The demo is a made-up NGO with a quarter of its books.{' '}
              <a href="/security" className={quietLink}>
                Read how we handle your data
              </a>
              .
            </p>
          </div>
          <CtaRow>
            <DemoCta location="pricing-close" />
            <MailCta location="pricing-close-pilot" subject={PILOT_SUBJECT}>
              Join the pilot
            </MailCta>
          </CtaRow>
        </div>
      </section>
    </>
  )
}
