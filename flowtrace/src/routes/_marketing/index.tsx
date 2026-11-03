import { Fragment, useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import { ArrowRight, Check, Minus } from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import { ChainLine } from '@/components/chain-status-badge'
import { CampaignCoverage } from '@/components/campaign-progress-bar'
import MinimalFaq from '@/components/kokonutui-pro/minimal-faq'
import { demoStarted } from '@/components/marketing/marketing-masthead'
import { ProofSpendingList } from '@/components/public-campaign-header'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ProofLegend,
  StatusLine,
  StatusMark,
} from '@/components/report'
import { buttonVariants } from '@/components/ui/button'
import { track } from '@/lib/analytics'
import { generateTrustBadgeSVG, scaleSvg } from '@/lib/badge-utils'
import { accessGroups } from '@/lib/demo/dataset'
import { formatAmount, formatShortDate } from '@/lib/format'
import {
  AUDIENCE,
  CLOSE,
  DOORS,
  FAQ,
  FUNDERS,
  HERO,
  HOW,
  LIVE_LEDGER,
  PRODUCTS,
  TRUST,
  WORKS,
} from '@/lib/landing-copy'
import {
  BOREHOLE_TRACE,
  DEMO_SUMMARY,
  HERO_FIGURES,
  LANDING_CAMPAIGN,
  LANDING_ORG,
  PROOF_KINDS,
  TRUST_GRADES,
  TRUST_PARTS,
} from '@/lib/landing-figures'
import {
  DEMO_FIGURES_AS_OF,
  DEMO_PATH,
  GITHUB_REPO_URL,
} from '@/lib/marketing-nav'
import { PROOF_TYPE_LABEL } from '@/lib/proof-labels'
import { DEFAULT_DESCRIPTION, organizationJsonLd, pageHead } from '@/lib/site'
import { cn } from '@/lib/utils'

export const LANDING_TITLE = 'FlowTrace · Every payment, traced to its proof'
export const LANDING_DESCRIPTION = DEFAULT_DESCRIPTION

export const Route = createFileRoute('/_marketing/')({
  head: () => ({
    ...pageHead({
      title: LANDING_TITLE,
      description: LANDING_DESCRIPTION,
      path: '/',
      absoluteTitle: true,
    }),
    scripts: [{ type: 'application/ld+json', children: organizationJsonLd() }],
  }),
  staticData: { demoFigures: true },
  component: LandingPage,
})

export function LandingPage() {
  return (
    <>
      <Hero />
      <Doors />
      <Products />
      <LiveLedger />
      <HowProofWorks />
      <AudienceSection />
      <FundersSection />
      <WorksSection />
      <TrustSection />
      <MinimalFaq title={FAQ.title} description={FAQ.lede} items={FAQ.items} />
      <Close />
    </>
  )
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

const QUIET_LINK =
  'font-medium underline decoration-foreground/40 underline-offset-4 hover:decoration-foreground'

function Section({
  id,
  className,
  children,
}: {
  id?: string
  className?: string
  children: ReactNode
}) {
  return (
    <section
      id={id}
      className={cn('scroll-mt-20 border-t-[3px] border-foreground', className)}
    >
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        {children}
      </div>
    </section>
  )
}

function SectionTitle({ children }: { children: ReactNode }) {
  return <h2 className="text-4xl leading-none sm:text-5xl">{children}</h2>
}

function Arrow() {
  return (
    <ArrowRight
      className="size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
      weight="bold"
    />
  )
}

/** A text link with the trailing arrow; app routes go through the router, the rest are anchors. */
function ArrowLink({
  href,
  location,
  children,
}: {
  href: string
  location: string
  children: ReactNode
}) {
  const className = cn(
    'group inline-flex min-h-6 items-center gap-1.5 text-sm',
    QUIET_LINK,
  )
  if (href === DEMO_PATH)
    return (
      <Link
        to={DEMO_PATH}
        onClick={() => demoStarted(location)}
        className={className}
      >
        {children}
        <Arrow />
      </Link>
    )
  return (
    <a
      href={href}
      onClick={() => track('cta_click', { location })}
      className={className}
    >
      {children}
      <Arrow />
    </a>
  )
}

function DemoCta({ location }: { location: string }) {
  return (
    <Link
      to={DEMO_PATH}
      onClick={() => demoStarted(location)}
      className={cn(buttonVariants({ size: 'lg' }), 'group shrink-0')}
    >
      {HERO.primary}
      <Arrow />
    </Link>
  )
}

const fmt = (n: number) => formatAmount(n)

// ---------------------------------------------------------------------------
// 1. Hero
// ---------------------------------------------------------------------------

function Hero() {
  const [focused, setFocused] = useState<string | null>(null)

  return (
    <section className="mx-auto grid max-w-7xl gap-12 px-4 pt-12 pb-16 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:pt-16 lg:pb-20">
      <div className="lg:col-span-5 lg:pt-4">
        <h1 className="text-[3.25rem] leading-[0.95] sm:text-7xl lg:text-[5.25rem]">
          {HERO.title}
        </h1>
        <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
          {HERO.lede}
        </p>
        <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-4">
          <DemoCta location="hero" />
          <a
            href="#how"
            onClick={() => track('cta_click', { location: 'hero_proof' })}
            className={cn('text-base', QUIET_LINK)}
          >
            {HERO.secondary}
          </a>
        </div>
        <p className="mt-4 text-sm text-muted-foreground">{HERO.note}</p>
      </div>

      <div className="lg:col-span-7">
        <div className="rule-heavy">
          <div className="flex min-h-11 items-center justify-between gap-3 border-b border-border py-2">
            <h2 className="section-label">{HERO.figuresLabel}</h2>
            <span className="flex items-center gap-3">
              <AsOf date={DEMO_FIGURES_AS_OF} className="hidden sm:inline" />
              <DemoTag />
            </span>
          </div>
          <KeyFigures>
            <KeyFigure
              scale="lg"
              value={HERO_FIGURES.evidencedCompact}
              label="KES received with proof"
            />
            <KeyFigure
              value={HERO_FIGURES.movements}
              label="movements evidenced"
            />
            <KeyFigure
              value={HERO_FIGURES.awaiting}
              label="awaiting proof"
              detail={
                HERO_FIGURES.flagged > 0 ? (
                  <StatusMark
                    status="flagged"
                    label={`${HERO_FIGURES.flagged} flagged`}
                  />
                ) : undefined
              }
            />
            <KeyFigure
              value={HERO_FIGURES.trustScore}
              label={`Trust Score, grade ${HERO_FIGURES.trustGrade}`}
            />
          </KeyFigures>

          <h3 className="section-label mt-6 mb-3">{HERO.donorsLabel}</h3>
          <ul
            className="space-y-1"
            onMouseLeave={() => setFocused(null)}
            aria-label="Funding by donor"
          >
            {DEMO_SUMMARY.donorLines.map((donor) => {
              const dimmed = focused !== null && focused !== donor.id
              const onlyPending = donor.received === 0 && donor.awaiting > 0
              const amount = donor.received + donor.awaiting
              return (
                <li key={donor.id}>
                  <button
                    type="button"
                    onMouseEnter={() => setFocused(donor.id)}
                    onFocus={() => setFocused(donor.id)}
                    onBlur={() => setFocused(null)}
                    aria-label={`${donor.name}: KES ${fmt(amount)}, ${
                      onlyPending ? 'awaiting proof' : 'evidenced'
                    }`}
                    className={cn(
                      'grid w-full grid-cols-[1fr_6.5rem] items-center gap-x-3 gap-y-1 py-1.5 text-left text-sm transition-opacity duration-200 motion-reduce:transition-none sm:grid-cols-[12rem_1fr_7.5rem] sm:gap-4',
                      dimmed && 'opacity-30',
                    )}
                  >
                    <span className="col-span-2 truncate font-medium sm:col-span-1">
                      {donor.name}
                    </span>
                    <CoverageBar
                      total={DEMO_SUMMARY.largest}
                      filled={onlyPending ? amount : donor.received}
                      evidenced={onlyPending ? undefined : donor.evidenced}
                      pending={onlyPending}
                    />
                    <span className="text-right font-semibold tabular-nums">
                      {fmt(amount)}
                    </span>
                  </button>
                </li>
              )
            })}
          </ul>
          <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
            <ProofLegend />
            <p className="text-xs text-muted-foreground">
              KES {fmt(DEMO_SUMMARY.evidencedTotal)} evidenced of{' '}
              {fmt(DEMO_SUMMARY.committedTotal)} committed
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}

// ---------------------------------------------------------------------------
// 2. Four doors
// ---------------------------------------------------------------------------

function Doors() {
  return (
    <Section id="who">
      <SectionTitle>{DOORS.title}</SectionTitle>
      <ul className="mt-10 grid border-t border-border sm:grid-cols-2 lg:grid-cols-4">
        {DOORS.items.map((door, i) => (
          <li
            key={door.name}
            className={cn(
              'flex flex-col border-b border-border py-6 sm:px-6 lg:border-b-0',
              i % 2 === 1 && 'sm:border-l',
              i > 0 && 'lg:border-l',
              i % 2 === 0 && 'sm:pl-0',
              i === 0 && 'lg:pl-0',
              i === 2 && 'lg:pl-6',
            )}
          >
            <h3 className="text-2xl leading-tight">{door.name}</h3>
            <p className="mt-3 text-base leading-relaxed">{door.text}</p>
            <p className="mt-2 text-sm text-muted-foreground">{door.outcome}</p>
            <p className="mt-auto pt-5">
              <ArrowLink
                href={door.link.href}
                location={`door_${door.name.toLowerCase().replace(/\W+/g, '_')}`}
              >
                {door.link.label}
              </ArrowLink>
            </p>
          </li>
        ))}
      </ul>
    </Section>
  )
}

// ---------------------------------------------------------------------------
// 3. Two products
// ---------------------------------------------------------------------------

function Products() {
  return (
    <Section id="product">
      <div className="grid gap-6 lg:grid-cols-12">
        <div className="lg:col-span-5">
          <SectionTitle>{PRODUCTS.title}</SectionTitle>
        </div>
        <p className="max-w-xl text-lg leading-relaxed text-muted-foreground lg:col-span-6 lg:col-start-7">
          {PRODUCTS.lede}
        </p>
      </div>
      <div className="mt-12 grid gap-12 md:grid-cols-2 md:gap-8">
        {PRODUCTS.items.map((product) => (
          <article key={product.name} className="flex flex-col">
            <img
              src={product.image.src}
              width={product.image.width}
              height={product.image.height}
              alt={product.image.alt}
              loading="lazy"
              decoding="async"
              className="h-auto w-full border border-border"
            />
            <h3 className="mt-6 text-3xl">{product.name}</h3>
            <p className="mt-3 max-w-xl text-base leading-relaxed">
              {product.text}
            </p>
            <p className="mt-5">
              <ArrowLink
                href={product.link.href}
                location={`product_${product.name.split(' ')[1].toLowerCase()}`}
              >
                {product.link.label}
              </ArrowLink>
            </p>
          </article>
        ))}
      </div>
    </Section>
  )
}

// ---------------------------------------------------------------------------
// 4. Live public ledger (decision Q18)
// ---------------------------------------------------------------------------

function LiveLedger() {
  const { campaign, organization, evidenced_amount, spending } =
    LANDING_CAMPAIGN
  return (
    <Section id="live">
      <SectionTitle>{LIVE_LEDGER.title}</SectionTitle>
      <div className="mt-10 rule-heavy">
        <div className="flex min-h-11 flex-wrap items-center justify-between gap-3 border-b border-border py-2">
          <h3 className="section-label">{LIVE_LEDGER.label}</h3>
          <span className="flex items-center gap-3">
            <AsOf date={DEMO_FIGURES_AS_OF} />
            <DemoTag />
          </span>
        </div>
        <div className="grid gap-x-10 gap-y-8 py-6 lg:grid-cols-12">
          <div className="lg:col-span-5">
            <p className="text-3xl leading-tight font-bold [font-variation-settings:'wdth'_78]">
              {campaign.name}
            </p>
            <p className="mt-2 text-sm text-muted-foreground">
              A campaign by {organization.name}
            </p>
            <p className="section-label mt-6 mb-3">{LIVE_LEDGER.raised}</p>
            <CampaignCoverage
              raised={campaign.current_amount}
              target={campaign.target_amount}
              evidenced={evidenced_amount}
              currency={campaign.currency}
              size="lg"
            />
          </div>
          <div className="lg:col-span-7">
            <p className="section-label border-b border-border pb-2">
              {LIVE_LEDGER.spending}
            </p>
            <ProofSpendingList spending={spending} />
            <ProofLegend className="border-t border-border pt-3" />
          </div>
        </div>
      </div>
      <div className="mt-4 flex flex-wrap items-center justify-between gap-4">
        <p className="max-w-xl text-sm text-muted-foreground">
          {LIVE_LEDGER.caption}
        </p>
        <Link
          to="/donate/$slug"
          params={{ slug: campaign.public_slug ?? '' }}
          onClick={() => track('cta_click', { location: 'live_ledger' })}
          className={cn(
            'group inline-flex items-center gap-1.5 text-sm',
            QUIET_LINK,
          )}
        >
          {LIVE_LEDGER.link}
          <Arrow />
        </Link>
      </div>
    </Section>
  )
}

// ---------------------------------------------------------------------------
// 5. How proof works
// ---------------------------------------------------------------------------

function HowProofWorks() {
  const stations = [
    { name: BOREHOLE_TRACE[0].from, step: null },
    ...BOREHOLE_TRACE.map((step) => ({ name: step.to, step })),
  ]
  const badge = scaleSvg(
    generateTrustBadgeSVG({ score: LANDING_ORG.trust_score }),
    2,
  )

  return (
    <Section id="how">
      <div className="grid gap-6 lg:grid-cols-12">
        <div className="lg:col-span-5">
          <SectionTitle>{HOW.title}</SectionTitle>
        </div>
        <p className="max-w-xl text-lg leading-relaxed text-muted-foreground lg:col-span-6 lg:col-start-7">
          {HOW.lede}
        </p>
      </div>

      <p className="section-label mt-12">{HOW.traceLabel}</p>
      <ol
        className="mt-5 grid gap-10 md:grid-cols-3 md:gap-0"
        aria-label={HOW.traceLabel}
      >
        {stations.map((station, i) => (
          <li key={station.name} className="relative md:pr-8">
            <div className="flex items-center" aria-hidden="true">
              <span className="size-3.5 shrink-0 border-[3px] border-foreground bg-background" />
              {i < BOREHOLE_TRACE.length && (
                <StatusLine
                  status={BOREHOLE_TRACE[i].status}
                  width={600}
                  className="hidden h-2.5 w-full md:block"
                />
              )}
            </div>
            <p className="mt-4 text-3xl leading-tight font-bold [font-variation-settings:'wdth'_78]">
              {station.name}
            </p>
            {station.step ? (
              <>
                <p className="mt-2 text-sm">
                  <span className="font-bold">
                    KES {fmt(station.step.amount)}
                  </span>
                  , {formatShortDate(station.step.date)}.{' '}
                  {station.step.description}.
                </p>
                <p className="mt-4 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3 text-sm text-muted-foreground">
                  <span>
                    <span className="font-semibold text-foreground">
                      Proof:{' '}
                    </span>
                    {station.step.proofs
                      .map((type) => PROOF_TYPE_LABEL[type])
                      .join(', ')}
                  </span>
                  <StatusMark status={station.step.status} />
                </p>
              </>
            ) : (
              <p className="mt-2 text-sm">{HOW.donorStation}</p>
            )}
          </li>
        ))}
      </ol>
      <p className="mt-8 flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
        {HOW.traceNote} <DemoTag />
      </p>

      <div className="mt-16 grid gap-12 lg:grid-cols-12">
        <div className="lg:col-span-5">
          <h3 className="text-3xl">{HOW.kindsTitle}</h3>
          <p className="mt-3 text-base text-muted-foreground">
            {HOW.kindsLede}
          </p>
          <ul className="mt-6 grid grid-cols-2 border-t border-border text-sm">
            {PROOF_KINDS.map((kind) => (
              <li
                key={kind.type}
                className="flex items-baseline justify-between gap-3 border-b border-border py-2 pr-4 odd:border-r even:pl-4"
              >
                <span>{PROOF_TYPE_LABEL[kind.type]}</span>
                <span className="font-semibold tabular-nums">
                  {kind.points}
                  <span className="sr-only"> trust points</span>
                </span>
              </li>
            ))}
          </ul>
        </div>

        <div className="lg:col-span-6 lg:col-start-7">
          <h3 className="text-3xl">{HOW.scoreTitle}</h3>
          <p className="mt-3 text-base text-muted-foreground">
            {HOW.scoreLede}
          </p>
          <dl className="mt-6 border-t border-border">
            {TRUST_PARTS.map((part) => (
              <div
                key={part.label}
                className="grid grid-cols-[1fr_auto] gap-x-4 border-b border-border py-3"
              >
                <dt className="font-semibold">{part.label}</dt>
                <dd className="row-span-2 self-center text-3xl font-extrabold tabular-nums [font-variation-settings:'wdth'_75]">
                  {part.weight}
                </dd>
                <dd className="text-sm text-muted-foreground">
                  {HOW.parts[part.label]}
                </dd>
              </div>
            ))}
          </dl>
          <div className="mt-6 flex flex-wrap items-end justify-between gap-6">
            <div>
              <p className="section-label mb-2">{HOW.gradesLabel}</p>
              <ul className="flex flex-wrap gap-x-4 gap-y-1 text-sm tabular-nums">
                {TRUST_GRADES.map((g) => (
                  <li key={g.grade}>
                    <span className="font-bold">{g.grade}</span>{' '}
                    {g.min > 0 ? `${g.min}+` : 'below 50'}
                  </li>
                ))}
              </ul>
            </div>
            <div>
              <p className="section-label mb-2 flex items-center gap-2">
                {HOW.badgeLabel} <DemoTag />
              </p>
              <Link
                to="/badge/trust/$orgId"
                params={{ orgId: LANDING_ORG.id }}
                search={{ format: 'preview' }}
                aria-label={`${LANDING_ORG.name} Trust Score badge: ${LANDING_ORG.trust_score} of 100`}
                className="inline-block focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
              >
                {/* Our own SVG, generated from a number; badge-utils escapes its label. */}
                <span
                  aria-hidden="true"
                  dangerouslySetInnerHTML={{ __html: badge }}
                />
              </Link>
            </div>
          </div>
          <p className="mt-6">
            <ArrowLink href="/docs" location="trust_method">
              {HOW.method}
            </ArrowLink>
          </p>
        </div>
      </div>
    </Section>
  )
}

// ---------------------------------------------------------------------------
// 6. Each audience sees its own depth
// ---------------------------------------------------------------------------

const GROUP_LABEL: Record<string, string> = {
  'Board of Directors': 'Board',
  Donors: 'Funders and donors',
}

/** The demo organisation's five default groups, read from their permissions. */
const AUDIENCES = accessGroups
  .filter((g) => g.organization_id === LANDING_ORG.id)
  .map((g) => {
    const group = GROUP_LABEL[g.name] ?? g.name
    const p = g.permissions
    const proofs: 'full' | 'summary' | false = p.can_view_full_proofs
      ? 'full'
      : p.can_view_proofs
        ? 'summary'
        : false
    return {
      group,
      scope: AUDIENCE.scope[group] ?? '',
      amounts: p.can_view_amounts,
      proofs,
      edit: p.can_edit_transactions,
    }
  })

function AudienceSection() {
  return (
    <section className="bg-masthead text-masthead-foreground">
      <div className="mx-auto grid max-w-7xl gap-10 px-4 py-16 sm:px-8 lg:grid-cols-12 lg:py-20">
        <div className="lg:col-span-4">
          <SectionTitle>{AUDIENCE.title}</SectionTitle>
          <p className="mt-5 text-base leading-relaxed opacity-80">
            {AUDIENCE.lede}
          </p>
        </div>
        <div className="lg:col-span-8">
          <dl className="border-t-[3px] border-masthead-foreground sm:hidden">
            {AUDIENCES.map((row) => (
              <div
                key={row.group}
                className="border-b border-masthead-foreground/15 py-4"
              >
                <dt className="text-base font-semibold">{row.group}</dt>
                <dd className="mt-2 grid grid-cols-[6rem_1fr] gap-x-4 gap-y-1.5 text-sm">
                  <span className="opacity-70">Sees</span>
                  <span>{row.scope}</span>
                  <span className="opacity-70">Amounts</span>
                  <YesNo value={row.amounts} withText />
                  <span className="opacity-70">Proofs</span>
                  <ProofDepth value={row.proofs} withText />
                  <span className="opacity-70">Can edit</span>
                  <YesNo value={row.edit} withText />
                </dd>
              </div>
            ))}
          </dl>
          <table className="hidden w-full border-t-[3px] border-masthead-foreground text-left text-sm sm:table">
            <thead>
              <tr className="border-b border-masthead-foreground/30 text-xs tracking-[0.12em] uppercase">
                <th scope="col" className="py-3 pr-4 font-bold">
                  Group
                </th>
                <th scope="col" className="py-3 pr-4 font-bold">
                  Sees
                </th>
                <th scope="col" className="py-3 pr-4 font-bold">
                  Amounts
                </th>
                <th scope="col" className="py-3 pr-4 font-bold">
                  Proofs
                </th>
                <th scope="col" className="py-3 font-bold">
                  Can edit
                </th>
              </tr>
            </thead>
            <tbody>
              {AUDIENCES.map((row) => (
                <tr
                  key={row.group}
                  className="border-b border-masthead-foreground/15"
                >
                  <th scope="row" className="py-3 pr-4 text-base font-semibold">
                    {row.group}
                  </th>
                  <td className="py-3 pr-4 opacity-80">{row.scope}</td>
                  <td className="py-3 pr-4">
                    <YesNo value={row.amounts} />
                  </td>
                  <td className="py-3 pr-4">
                    <ProofDepth value={row.proofs} />
                  </td>
                  <td className="py-3">
                    <YesNo value={row.edit} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="mt-3 text-xs opacity-70">{AUDIENCE.note}</p>
        </div>
      </div>
    </section>
  )
}

function YesNo({ value, withText }: { value: boolean; withText?: boolean }) {
  const mark = value ? (
    <Check
      weight="bold"
      className="size-4 shrink-0"
      aria-label={withText ? undefined : 'Yes'}
      aria-hidden={withText ? true : undefined}
    />
  ) : (
    <Minus
      weight="bold"
      className="size-4 shrink-0 opacity-50"
      aria-label={withText ? undefined : 'No'}
      aria-hidden={withText ? true : undefined}
    />
  )
  if (!withText) return mark
  return (
    <span className="flex items-center gap-1.5">
      {mark}
      {value ? 'Yes' : 'No'}
    </span>
  )
}

function ProofDepth({
  value,
  withText,
}: {
  value: 'full' | 'summary' | false
  withText?: boolean
}) {
  if (value === 'full') return <span>Originals</span>
  if (value === 'summary') return <span>Summaries</span>
  return <YesNo value={false} withText={withText} />
}

// ---------------------------------------------------------------------------
// 7. For funders
// ---------------------------------------------------------------------------

const CHAIN: Array<{ name: string; note: string }> = [
  { name: 'Funder', note: 'Follows its grant' },
  { name: 'Your organisation', note: 'Records and proves' },
  { name: 'Implementing partner', note: 'Delivers on the ground' },
]
const CHAIN_LINKS: Array<{ status: 'active' | 'pending'; label: string }> = [
  { status: 'active', label: 'Approved' },
  { status: 'pending', label: 'Requested' },
]

function FundersSection() {
  return (
    <section id="funders" className="scroll-mt-20">
      <div className="mx-auto grid max-w-7xl gap-10 px-4 py-16 sm:px-8 lg:grid-cols-12 lg:py-20">
        <div className="lg:col-span-5">
          <SectionTitle>{FUNDERS.title}</SectionTitle>
        </div>
        <div className="lg:col-span-7">
          <p className="max-w-xl text-lg leading-relaxed text-muted-foreground">
            {FUNDERS.lede}
          </p>
          <ol
            className="mt-10 flex flex-col border-t-[3px] border-foreground pt-5 md:grid md:grid-cols-[minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)] md:items-start md:gap-x-3"
            aria-label="A chain of organisations: funder to your organisation approved, your organisation to implementing partner requested"
          >
            {CHAIN.map((org, i) => (
              <Fragment key={org.name}>
                <li className="min-w-0">
                  <p className="leading-tight font-semibold">{org.name}</p>
                  <p className="mt-1 text-sm text-muted-foreground">
                    {org.note}
                  </p>
                </li>
                {i < CHAIN_LINKS.length && (
                  <li
                    aria-hidden="true"
                    className="flex items-center gap-3 py-3 pl-1.5 md:flex-col md:items-stretch md:gap-1 md:py-0 md:pt-0.5 md:pl-0"
                  >
                    <svg
                      width="3"
                      height="40"
                      className="block shrink-0 overflow-visible md:hidden"
                    >
                      <line
                        x1="1.5"
                        y1="0"
                        x2="1.5"
                        y2="40"
                        strokeWidth="3"
                        strokeDasharray={
                          CHAIN_LINKS[i].status === 'pending'
                            ? '7 5'
                            : undefined
                        }
                        className={
                          CHAIN_LINKS[i].status === 'pending'
                            ? 'stroke-muted-foreground'
                            : 'stroke-foreground'
                        }
                      />
                    </svg>
                    <span
                      className={cn(
                        'text-[0.6875rem] font-semibold tracking-[0.08em] uppercase md:text-center',
                        CHAIN_LINKS[i].status === 'active'
                          ? 'text-foreground'
                          : 'text-muted-foreground',
                      )}
                    >
                      {CHAIN_LINKS[i].label}
                    </span>
                    <ChainLine
                      status={CHAIN_LINKS[i].status}
                      className="hidden md:block"
                    />
                  </li>
                )}
              </Fragment>
            ))}
          </ol>
          <p className="mt-6 text-sm text-muted-foreground">{FUNDERS.key}</p>
          <p className="mt-6 max-w-xl text-base">{FUNDERS.mandate}</p>
          <p className="mt-4">
            <ArrowLink href="/funders" location="funders">
              {FUNDERS.link}
            </ArrowLink>
          </p>
        </div>
      </div>
    </section>
  )
}

// ---------------------------------------------------------------------------
// 8. Works where you work, 9. Trust block
// ---------------------------------------------------------------------------

function WorksSection() {
  return (
    <Section>
      <SectionTitle>{WORKS.title}</SectionTitle>
      <ul className="mt-10 grid border-t border-border sm:grid-cols-2 lg:grid-cols-4">
        {WORKS.items.map((item) => (
          <li
            key={item.name}
            className="border-b border-border py-5 sm:pr-6 lg:border-b-0"
          >
            <h3 className="text-xl leading-tight">{item.name}</h3>
            <p className="mt-2 text-sm leading-relaxed">{item.text}</p>
          </li>
        ))}
      </ul>
    </Section>
  )
}

function TrustSection() {
  return (
    <Section id="trust">
      <div className="grid gap-10 lg:grid-cols-12">
        <div className="lg:col-span-5">
          <SectionTitle>{TRUST.title}</SectionTitle>
          <p className="mt-6 inline-flex border border-foreground px-3 py-2 text-sm font-semibold">
            {TRUST.stage}
          </p>
          <p className="mt-6 flex flex-wrap gap-x-6 gap-y-3">
            <ArrowLink href="/security" location="trust_security">
              {TRUST.security}
            </ArrowLink>
            <ArrowLink href="/pricing" location="trust_pricing">
              {TRUST.pricing}
            </ArrowLink>
          </p>
        </div>
        <dl className="border-t border-border text-sm lg:col-span-7">
          {TRUST.items.map((item) => (
            <div
              key={item.name}
              className="grid grid-cols-[9rem_1fr] gap-x-4 border-b border-border py-3 sm:grid-cols-[11rem_1fr]"
            >
              <dt className="font-semibold">{item.name}</dt>
              <dd>
                {item.name === 'Source code' ? (
                  <a
                    href={GITHUB_REPO_URL}
                    rel="noopener"
                    onClick={() => track('outbound_click', { name: 'github' })}
                    className={QUIET_LINK}
                  >
                    {item.text}
                  </a>
                ) : (
                  item.text
                )}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </Section>
  )
}

// ---------------------------------------------------------------------------
// 11. Close
// ---------------------------------------------------------------------------

function Close() {
  return (
    <section className="border-t-[3px] border-foreground">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 px-4 py-16 sm:px-8 md:flex-row md:items-end md:justify-between lg:py-20">
        <h2 className="max-w-2xl text-4xl leading-none sm:text-6xl">
          {CLOSE.title}
        </h2>
        <DemoCta location="close" />
      </div>
    </section>
  )
}
