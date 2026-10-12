import { Fragment, useState } from 'react'
import { Link, createFileRoute } from '@tanstack/react-router'
import { ArrowRight, Check, Minus } from '@phosphor-icons/react'
import { ChainLine } from '@/components/chain-status-badge'
import { ThemeToggle } from '@/components/layout/theme-toggle'
import { buttonVariants } from '@/components/ui/button'
import {
  AsOf,
  CoverageBar,
  DemoTag,
  KeyFigure,
  KeyFigures,
  ProofLegend,
  StatusLine,
} from '@/components/report'
import { Wordmark } from '@/components/layout/wordmark'
import { useAppAuth } from '@/lib/auth-context'
import { cn } from '@/lib/utils'

export const Route = createFileRoute('/')({
  component: LandingPage,
})

// Figures from the bundled demo dataset (Demo NGO Kenya, Q1 2026).
const REPORT_DATE = '2026-02-04'
const DONORS = [
  // Same order as the dashboard: evidenced by amount, awaiting proof last.
  {
    name: 'Harambee Health Fund',
    amount: 8_500_000,
    status: 'verified' as const,
  },
  { name: 'Kisima Water Trust', amount: 4_000_000, status: 'verified' as const },
  {
    name: 'Northlight Foundation',
    amount: 3_200_000,
    status: 'verified' as const,
  },
  {
    name: 'Pan-African Education Fund',
    amount: 6_000_000,
    status: 'pending' as const,
  },
]
const LARGEST = Math.max(...DONORS.map((d) => d.amount))

const TRACE = [
  {
    step: 'Received',
    node: 'Kisima Water Trust',
    detail: 'KES 4,000,000 into the Project Fund, 10 Jan',
    proof: 'Bank statement, signed award letter',
    status: 'verified' as const,
  },
  {
    step: 'Allocated',
    node: 'Project Fund',
    detail: 'KES 2,500,000 to Clean Water Turkana, 18 Jan',
    proof: 'Board approval minute',
    status: 'verified' as const,
  },
  {
    step: 'Spent',
    node: 'Borehole, Phase 1',
    detail: 'Contractor paid for drilling in Turkana',
    proof: 'Contractor invoice, site photo, GPS fix',
    status: 'verified' as const,
  },
]

const AUDIENCES: Array<{
  group: string
  scope: string
  amounts: boolean
  proofs: 'full' | 'summary' | false
  edit: boolean
}> = [
  { group: 'Board', scope: 'Everything', amounts: true, proofs: 'full', edit: true },
  { group: 'Staff', scope: 'Everything', amounts: true, proofs: 'summary', edit: true },
  { group: 'Donors', scope: 'Their money, downstream', amounts: true, proofs: 'summary', edit: false },
  { group: 'Partners', scope: 'Assigned nodes only', amounts: true, proofs: false, edit: false },
  { group: 'Public', scope: 'All flows, names only', amounts: false, proofs: false, edit: false },
]

const fmt = (n: number) => n.toLocaleString('en-KE')

function LandingPage() {
  const { user } = useAppAuth()

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Masthead signedIn={!!user} />
      <main>
        <Hero />
        <TraceSection />
        <AudienceSection />
        <FundersSection />
        <Close />
      </main>
      <Footer />
    </div>
  )
}

function Masthead({ signedIn }: { signedIn: boolean }) {
  return (
    <header className="sticky top-0 z-40 border-b-[3px] border-foreground bg-background">
      <div className="mx-auto flex h-14 max-w-7xl items-center gap-6 px-4 sm:px-8">
        <Link to="/" aria-label="FlowTrace home">
          <Wordmark />
        </Link>
        <p className="hidden text-xs tracking-[0.14em] text-muted-foreground uppercase lg:block [font-stretch:87%]">
          Situation report · Demo NGO Kenya ·{' '}
          <AsOf date={REPORT_DATE} className="text-xs" />
        </p>
        <nav className="ml-auto flex items-center gap-1 sm:gap-4">
          <a
            href="#how"
            className="hidden px-2 py-1 text-sm hover:underline md:inline"
          >
            How proof works
          </a>
          <a
            href="#funders"
            className="hidden px-2 py-1 text-sm hover:underline md:inline"
          >
            For funders
          </a>
          <ThemeToggle className="hidden sm:inline-flex" />
          <Link
            to="/dashboard"
            className="px-1 py-1 text-sm whitespace-nowrap underline decoration-foreground/30 underline-offset-4 hover:decoration-foreground sm:px-2"
          >
            {signedIn ? 'Open app' : 'Sign in'}
          </Link>
          <Link to="/dashboard" className={buttonVariants({ size: 'sm' })}>
            Explore the demo
          </Link>
        </nav>
      </div>
    </header>
  )
}

function Hero() {
  const [focused, setFocused] = useState<string | null>(null)
  const evidenced = DONORS.filter((d) => d.status === 'verified').reduce(
    (sum, d) => sum + d.amount,
    0,
  )
  const committed = DONORS.reduce((sum, d) => sum + d.amount, 0)

  return (
    <section className="mx-auto grid max-w-7xl gap-12 px-4 pt-12 pb-16 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:pt-16 lg:pb-20">
      <div className="lg:col-span-5 lg:pt-4">
        <h1 className="text-[3.25rem] leading-[0.95] sm:text-7xl lg:text-[5.25rem]">
          Every shilling, traced to its proof.
        </h1>
        <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
          FlowTrace records each grant, budget transfer and payment with its
          receipt, photo or bank statement attached. Then it gives every funder
          a live view of the money, at exactly the depth you choose.
        </p>
        <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-4">
          <Link
            to="/dashboard"
            className={cn(buttonVariants({ size: 'lg' }), 'group')}
          >
            Explore the demo
            <ArrowRight
              className="size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
              weight="bold"
            />
          </Link>
          <a
            href="#how"
            className="text-base font-medium underline decoration-foreground/40 hover:decoration-foreground"
          >
            How proof is attached
          </a>
        </div>
        <p className="mt-4 text-sm text-muted-foreground">
          The demo opens as Demo NGO Kenya. No account needed.
        </p>
      </div>

      <div className="lg:col-span-7">
        <div className="rule-heavy">
          <div className="flex items-center justify-between border-b border-border py-2">
            <h2 className="section-label font-sans [font-stretch:100%]">
              Key figures, Q1 2026
            </h2>
            <DemoTag />
          </div>
          <KeyFigures>
            <KeyFigure
              scale="lg"
              value="15.7M"
              label="KES received with proof"
            />
            <KeyFigure value="12/16" label="movements evidenced" />
            <KeyFigure value="4" label="awaiting proof or flagged" />
            <KeyFigure value="87" label="Trust Score" />
          </KeyFigures>

          <h3 className="section-label mt-6 mb-3 font-sans [font-stretch:100%]">
            Funding received, by donor
          </h3>
          <ul
            className="space-y-1"
            onMouseLeave={() => setFocused(null)}
            aria-label="Funding by donor"
          >
            {DONORS.map((donor) => {
              const dimmed = focused !== null && focused !== donor.name
              return (
                <li key={donor.name}>
                  <button
                    type="button"
                    onMouseEnter={() => setFocused(donor.name)}
                    onFocus={() => setFocused(donor.name)}
                    onBlur={() => setFocused(null)}
                    aria-label={`${donor.name}: KES ${fmt(donor.amount)}, ${
                      donor.status === 'pending'
                        ? 'awaiting proof'
                        : 'evidenced'
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
                      total={LARGEST}
                      filled={donor.amount}
                      evidenced={
                        donor.status === 'verified' ? donor.amount : undefined
                      }
                      pending={donor.status === 'pending'}
                    />
                    <span className="text-right font-semibold tabular-nums">
                      {fmt(donor.amount)}
                    </span>
                  </button>
                </li>
              )
            })}
          </ul>
          <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
            <ProofLegend />
            <p className="text-xs text-muted-foreground">
              KES {fmt(evidenced)} evidenced of {fmt(committed)}{' '}
              committed
            </p>
          </div>
        </div>
      </div>
    </section>
  )
}

function TraceSection() {
  return (
    <section id="how" className="scroll-mt-20 border-t-[3px] border-foreground">
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        <div className="grid gap-6 lg:grid-cols-12">
          <h2 className="text-4xl leading-none sm:text-5xl lg:col-span-5">
            One borehole, followed from grant to drill.
          </h2>
          <p className="max-w-xl text-lg leading-relaxed text-muted-foreground lg:col-span-6 lg:col-start-7">
            Each movement is recorded once, with its evidence attached at the
            moment it happens. Nobody rebuilds the trail at reporting time,
            because the trail is the record.
          </p>
        </div>

        <ol className="relative mt-12 grid gap-10 md:grid-cols-3 md:gap-0">
          {TRACE.map((item, i) => (
            <li key={item.step} className="relative md:pr-8">
              <div className="flex items-center" aria-hidden="true">
                <span className="size-3.5 shrink-0 border-[3px] border-foreground bg-background" />
                {i < TRACE.length - 1 && (
                  <StatusLine
                    status={item.status}
                    width={600}
                    className="hidden h-2.5 w-full md:block"
                  />
                )}
              </div>
              <h3 className="mt-4 text-3xl">{item.node}</h3>
              <p className="mt-2 text-sm">
                <span className="font-bold">{item.step}.</span> {item.detail}
              </p>
              <p className="mt-4 border-t border-border pt-3 text-sm text-muted-foreground">
                <span className="font-semibold text-foreground">Proof: </span>
                {item.proof}
              </p>
            </li>
          ))}
        </ol>
        <p className="mt-10 text-sm text-muted-foreground">
          From the demo dataset. In the app, every line can be opened to its
          documents.
        </p>
      </div>
    </section>
  )
}

function AudienceSection() {
  return (
    <section className="bg-masthead text-masthead-foreground">
      <div className="mx-auto grid max-w-7xl gap-10 px-4 py-16 sm:px-8 lg:grid-cols-12 lg:py-20">
        <div className="lg:col-span-4">
          <h2 className="text-4xl leading-none sm:text-5xl">
            Each audience sees its own depth.
          </h2>
          <p className="mt-5 text-base leading-relaxed opacity-80">
            Your board sees the original receipts. A donor sees how their
            grant was spent. The public sees names, not salaries. You set the
            depth per group, and FlowTrace holds the line.
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
          <p className="mt-3 text-xs opacity-70">
            The five default groups every organisation starts with. Add your
            own, and make access expire for auditors.
          </p>
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
          <h2 className="text-4xl leading-none sm:text-5xl">
            Funders follow the grant past your door.
          </h2>
        </div>
        <div className="lg:col-span-7">
          <p className="max-w-xl text-lg leading-relaxed text-muted-foreground">
            Link your FlowTrace to a funder's, and to your partners'. With each
            side's approval, a foundation can follow its grant through your
            organisation to the partner that delivered it, seeing only what
            each link grants.
          </p>
          <ol
            className="mt-10 flex flex-col border-t-[3px] border-foreground pt-5 md:grid md:grid-cols-[minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)_minmax(4rem,7rem)_minmax(0,1fr)] md:items-start md:gap-x-3"
            aria-label="A chain of organisations: funder to your organisation approved, your organisation to implementing partner requested"
          >
            {CHAIN.map((org, i) => (
              <Fragment key={org.name}>
                <li className="min-w-0">
                  <p className="font-semibold leading-tight">{org.name}</p>
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
                          CHAIN_LINKS[i].status === 'pending' ? '7 5' : undefined
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
          <p className="mt-6 text-sm text-muted-foreground">
            A solid line is an approved link; a dashed line is a request
            awaiting the other side. Either side can revoke a link at any time.
          </p>
        </div>
      </div>
    </section>
  )
}

function Close() {
  return (
    <section className="border-t-[3px] border-foreground">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 px-4 py-16 sm:px-8 md:flex-row md:items-end md:justify-between lg:py-20">
        <h2 className="max-w-2xl text-4xl leading-none sm:text-6xl">
          Open a quarter of an NGO's books, proof included.
        </h2>
        <Link
          to="/dashboard"
          className={cn(buttonVariants({ size: 'lg' }), 'group shrink-0')}
        >
          Explore the demo
          <ArrowRight
            className="size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0"
            weight="bold"
          />
        </Link>
      </div>
    </section>
  )
}

function Footer() {
  return (
    <footer className="border-t border-border">
      <div className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-4 px-4 py-6 text-sm text-muted-foreground sm:px-8">
        <Wordmark className="text-foreground" />
        <p>&copy; {new Date().getFullYear()} FlowTrace</p>
      </div>
    </footer>
  )
}
