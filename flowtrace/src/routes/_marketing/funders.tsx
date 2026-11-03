import { createFileRoute } from '@tanstack/react-router'
import { ChainGraphic } from '@/components/marketing/chain-graphic'
import {
  CtaRow,
  DemoCta,
  MailCta,
  PageHero,
  Section,
  quietLink,
} from '@/components/marketing/page-parts'
import { pageHead } from '@/lib/site'

export const FUNDERS_TITLE = 'For funders and investors'
export const FUNDERS_DESCRIPTION =
  'Follow a grant or an investment past the door of the organisation you fund: through its books and on to its partners, with the proof attached and at the depth you agree.'

export const Route = createFileRoute('/_marketing/funders')({
  head: () =>
    pageHead({
      title: FUNDERS_TITLE,
      description: FUNDERS_DESCRIPTION,
      path: '/funders',
    }),
  component: FundersPage,
})

const TALK_SUBJECT = 'FlowTrace for funders'

// The four parts and their names: routes/_authenticated/trust-score.tsx:42-60; "Not yet
// scored" is how that screen shows a part with no score.
const SCORE_PARTS: ReadonlyArray<{ name: string; means: string }> = [
  {
    name: 'Verification',
    means: 'The share of payments whose proof has been checked and accepted.',
  },
  {
    name: 'Documentation',
    means: 'The share of payments with at least one proof attached.',
  },
  {
    name: 'Consistency',
    means: 'How well amounts, dates and parties agree across the records.',
  },
  {
    name: 'Independent review',
    means:
      'A review of the records against a fixed rubric, less 5 points for each confirmed flag. Until the first review it does not move the score.',
  },
]

function FundersPage() {
  return (
    <>
      <PageHero
        eyebrow="For funders and investors"
        title="Follow your money past the grantee’s door."
        lede="A report tells you where the money went. FlowTrace shows you: each payment in the books of the organisation you fund, with its receipt, photo or bank line attached, and on through the partners it paid."
      >
        <CtaRow>
          <DemoCta location="funders-hero">
            Explore the demo as a funder
          </DemoCta>
          <MailCta location="funders-hero-talk" subject={TALK_SUBJECT}>
            Talk to us
          </MailCta>
        </CtaRow>
      </PageHero>

      <Section
        id="see"
        title="What you see"
        lede="Organisations link their FlowTrace to yours. Each link is asked for, approved, and can be ended by either side."
      >
        <ChainGraphic />
        <ol className="list-decimal space-y-3 pl-5">
          <li>
            <strong>Request.</strong> You ask an organisation you fund for a
            view of its flows.
          </li>
          <li>
            <strong>Approve.</strong> The organisation approves, and decides
            what the link shows.
          </li>
          <li>
            <strong>Follow.</strong> You follow your grant through its books,
            and on through the partners it funds when they link too.
          </li>
        </ol>
        <p>
          How deep you see is agreed with each organisation: the amounts, the
          proofs themselves, or only a summary that a proof is attached.
        </p>
      </Section>

      <Section
        id="score"
        title="One signal across your portfolio"
        lede="Every organisation on FlowTrace has a Trust Score from 0 to 100, worked out the same way, so you can compare them."
      >
        <dl className="divide-y divide-border border-y border-border">
          {SCORE_PARTS.map((p) => (
            <div
              key={p.name}
              className="grid gap-1 py-3 sm:grid-cols-[11rem_1fr] sm:gap-6"
            >
              <dt className="font-semibold">{p.name}</dt>
              <dd className="text-muted-foreground">{p.means}</dd>
            </div>
          ))}
        </dl>
        <p>
          A part with no data says “Not yet scored” instead of guessing. The
          score points you to the evidence; it does not replace reading it.
        </p>
      </Section>

      <Section
        id="ask"
        title="How to ask your grantees to use it"
        lede="Send them this page, or the demo."
      >
        <p>
          FlowTrace is free during the pilot, for you and for them. When pricing
          comes, the plan is that a funder can sponsor its grantees’ seats, so
          the organisations you fund do not pay.{' '}
          <a href="/pricing" className={quietLink}>
            Read about pricing
          </a>
          .
        </p>
        <p>
          The organisation keeps its books in FlowTrace as part of its normal
          work. Once it approves your link, you see the result without asking
          for a new report.
        </p>
      </Section>

      <Section id="limits" title="What it does not do">
        <p>
          FlowTrace does not audit anyone. It shows the evidence an organisation
          has recorded, and it shows the gaps: payments with no proof, proofs
          not yet checked, parts not yet scored. What you make of them is your
          judgement.
        </p>
      </Section>

      <Section id="investors" title="For small businesses and investors">
        <p>
          The same books work for a company. A small business that records its
          payments with their proof becomes easier to understand for an investor
          abroad, who can follow the money at the depth the company agrees to
          share.
        </p>
      </Section>

      <section
        aria-labelledby="funders-demo"
        className="border-t-[3px] border-foreground"
      >
        <div className="mx-auto grid max-w-7xl gap-8 px-4 py-14 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:py-20">
          <div className="lg:col-span-5">
            <h2 id="funders-demo" className="text-4xl leading-none sm:text-5xl">
              Try it from the funder’s side.
            </h2>
          </div>
          <div className="lg:col-span-7">
            <p className="text-lg leading-relaxed text-muted-foreground">
              The demo is a made-up NGO in Kenya with links in both directions.
              Open it, then choose <strong>Chains</strong> in the menu.
              “Incoming” shows the funders who can see its flows; “Outgoing”
              shows the partners whose flows it follows, the way you would
              follow yours.
            </p>
            <CtaRow>
              <DemoCta location="funders-close">
                Explore the demo as a funder
              </DemoCta>
              <MailCta location="funders-close-talk" subject={TALK_SUBJECT}>
                Talk to us
              </MailCta>
            </CtaRow>
          </div>
        </div>
      </section>
    </>
  )
}
