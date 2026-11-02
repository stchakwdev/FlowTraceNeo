import { Link } from '@tanstack/react-router'
import { ArrowRight, DownloadSimple, GithubLogo } from '@phosphor-icons/react'
import { BOOKS_FEATURES, HERO_SCREEN } from './books-screens'
import type { BooksScreen } from './books-screens'
import type { BooksRelease } from '@/lib/books-release'
import { demoStarted } from '@/components/marketing/marketing-masthead'
import { AsOf, DemoTag } from '@/components/report'
import { buttonVariants } from '@/components/ui/button'
import { track } from '@/lib/analytics'
import { BOOKS_REPO_URL, formatBytes } from '@/lib/books-release'
import { formatDate } from '@/lib/format'
import { DEMO_FIGURES_AS_OF, DEMO_PATH } from '@/lib/marketing-nav'
import { cn } from '@/lib/utils'

// TODO-USER: the contact address for early access requests.
const CONTACT_EMAIL = 'TODO-USER'
const EARLY_ACCESS_HREF = `mailto:${CONTACT_EMAIL}?subject=${encodeURIComponent('FlowTrace Books early access')}`

// Measured, not rounded: `cargo test --workspace --locked` on main 1ffec82 passed 861
// (13 ignored). Update both lines together.
const RUST_TESTS = '861'
const RUST_TESTS_DATE = '2026-11-01'
const PARITY_FIXTURES = 32

const arrow =
  'size-4 transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none motion-reduce:group-hover:translate-x-0'
const quietLink =
  'underline decoration-foreground/40 underline-offset-4 hover:decoration-foreground'

/** The /books page's sections; the marketing layout supplies masthead, main and footer. */
export function BooksPage({ release }: { release: BooksRelease }) {
  return (
    <>
      <Hero release={release} />
      <Features />
      <Correctness />
      <Downloads release={release} />
      <Requirements />
      <Close />
    </>
  )
}

function DemoLink({
  location,
  className,
  children,
}: {
  location: string
  className: string
  children: React.ReactNode
}) {
  return (
    <Link
      to={DEMO_PATH}
      className={className}
      onClick={() => demoStarted(location)}
    >
      {children}
    </Link>
  )
}

function EarlyAccessLink({
  location,
  className,
}: {
  location: string
  className?: string
}) {
  return (
    <a
      href={EARLY_ACCESS_HREF}
      className={cn(buttonVariants({ size: 'lg' }), 'group', className)}
      onClick={() => track('cta_click', { location })}
    >
      Request early access
      <ArrowRight className={arrow} weight="bold" />
    </a>
  )
}

function Hero({ release }: { release: BooksRelease }) {
  return (
    <section className="mx-auto grid max-w-7xl gap-12 px-4 pt-12 pb-16 sm:px-8 lg:grid-cols-12 lg:gap-14 lg:pt-16 lg:pb-20">
      <div className="lg:col-span-5 lg:pt-4">
        <h1 className="text-[3.25rem] leading-[0.95] sm:text-7xl lg:text-[5.25rem]">
          The books, offline, native.
        </h1>
        <p className="mt-6 max-w-[34rem] text-lg leading-relaxed text-muted-foreground">
          FlowTrace Books is double-entry bookkeeping for organisations that
          work where the connection drops. It runs on your own computer, keeps
          your books on screen when the line goes down, and picks up the live
          books again when it comes back.
        </p>
        <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-4">
          {release.kind === 'available' ? (
            <a
              href="#download"
              className={cn(buttonVariants({ size: 'lg' }), 'group')}
              onClick={() => track('cta_click', { location: 'books-hero' })}
            >
              Download for Linux / macOS
              <DownloadSimple className="size-4" weight="bold" />
            </a>
          ) : (
            <EarlyAccessLink location="books-hero" />
          )}
          <DemoLink
            location="books-hero-demo"
            className={cn('text-base font-medium', quietLink)}
          >
            See the web demo
          </DemoLink>
        </div>
        <p className="mt-4 text-sm text-muted-foreground">
          {release.kind === 'available'
            ? `Version ${release.version}, free during the pilot.`
            : 'Not released yet. Free during the pilot.'}
        </p>
      </div>
      <div className="lg:col-span-7">
        <Shot screen={HERO_SCREEN} eager />
      </div>
    </section>
  )
}

function Shot({ screen, eager }: { screen: BooksScreen; eager?: boolean }) {
  return (
    <figure className="rule-heavy">
      <figcaption className="flex min-h-11 items-center justify-between gap-3 border-b border-border py-2 text-xs text-muted-foreground">
        <span>
          {screen.demo ? (
            <>
              Demo NGO Kenya ·{' '}
              <AsOf date={DEMO_FIGURES_AS_OF} className="text-xs" />
            </>
          ) : (
            'FlowTrace Books on Linux'
          )}
        </span>
        {screen.demo && <DemoTag />}
      </figcaption>
      <img
        src={screen.src}
        width={screen.width}
        height={screen.height}
        alt={screen.alt}
        loading={eager ? 'eager' : 'lazy'}
        decoding="async"
        fetchPriority={eager ? 'high' : undefined}
        className="h-auto w-full border-b border-border"
      />
    </figure>
  )
}

function SectionHead({
  id,
  title,
  lede,
}: {
  id: string
  title: string
  lede: string
}) {
  return (
    <div className="max-w-2xl">
      <h2 id={id} className="text-4xl leading-none sm:text-5xl">
        {title}
      </h2>
      <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
        {lede}
      </p>
    </div>
  )
}

function Features() {
  return (
    <section
      aria-labelledby="books-covers"
      className="border-t-[3px] border-foreground"
    >
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        <SectionHead
          id="books-covers"
          title="What it covers"
          lede="The books a grant-funded organisation keeps, from the first entry to the statements your board signs."
        />
        <ol className="mt-12">
          {BOOKS_FEATURES.map((feature, i) => (
            <li
              key={feature.title}
              className="grid gap-6 border-t border-border py-10 lg:grid-cols-12 lg:gap-14"
            >
              <div
                className={cn(
                  'lg:col-span-4',
                  i % 2 === 1 && feature.screen && 'lg:order-last',
                )}
              >
                <h3 className="text-2xl leading-tight sm:text-3xl">
                  {feature.title}
                </h3>
                <p className="mt-3 max-w-[34rem] leading-relaxed text-muted-foreground">
                  {feature.body}
                </p>
              </div>
              <div className="lg:col-span-8">
                {feature.screen ? (
                  <Shot screen={feature.screen} />
                ) : (
                  <p className="border-t border-dashed border-border pt-4 text-sm text-muted-foreground">
                    These screens are in the desktop app. Pictures of them are
                    coming.
                  </p>
                )}
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  )
}

function Correctness() {
  const points = [
    {
      figure: 'One',
      label: 'ledger core',
      body: 'The desktop app and the FlowTrace server run the same Rust ledger core, checked against the web app’s own engine.',
    },
    {
      figure: String(PARITY_FIXTURES),
      label: 'parity fixtures',
      body: 'Each one is exported from the web engine and reproduced by the Rust core to the cent, on every change.',
    },
    {
      figure: RUST_TESTS,
      label: `Rust tests, ${formatDate(RUST_TESTS_DATE)}`,
      body: 'Every one passes before a change is merged.',
    },
    {
      figure: 'Live',
      label: 'sync',
      body: 'Books sync through SpacetimeDB. Offline, each screen keeps the last books it loaded; a change waits for you to reconnect, so nothing is half-saved.',
    },
  ]
  return (
    <section
      aria-labelledby="books-correct"
      className="border-t-[3px] border-foreground"
    >
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        <SectionHead
          id="books-correct"
          title="How it stays correct"
          lede="Desktop and web give the same answer for the same books, because they share one engine and are tested against each other."
        />
        <dl className="mt-12 grid border-t border-border sm:grid-cols-2 lg:grid-cols-4">
          {points.map((p) => (
            <div
              key={p.label}
              className="border-b border-border py-6 sm:odd:pr-4 sm:even:border-l sm:even:pl-4 lg:border-l lg:px-4 lg:first:border-l-0 lg:first:pl-0"
            >
              <dt>
                <span className="block text-[2.75rem] leading-none font-extrabold font-narrow tabular-nums">
                  {p.figure}
                </span>
                <span className="mt-2 block text-sm text-muted-foreground">
                  {p.label}
                </span>
              </dt>
              <dd className="mt-3 text-sm leading-relaxed">{p.body}</dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  )
}

function Downloads({ release }: { release: BooksRelease }) {
  return (
    <section
      id="download"
      aria-labelledby="books-download"
      className="scroll-mt-16 border-t-[3px] border-foreground"
    >
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        {release.kind === 'available' ? (
          <ReleaseTable release={release} />
        ) : (
          <EarlyAccess />
        )}
      </div>
    </section>
  )
}

function ReleaseTable({
  release,
}: {
  release: Extract<BooksRelease, { kind: 'available' }>
}) {
  return (
    <>
      <SectionHead
        id="books-download"
        title="Download"
        lede={`Version ${release.version}${release.publishedAt ? `, released ${formatDate(release.publishedAt)}` : ''}. Pick the archive for your computer.`}
      />
      <table className="mt-10 w-full border-t border-border text-sm">
        <thead>
          <tr className="border-b border-border text-left text-xs tracking-[0.12em] text-muted-foreground uppercase">
            <th scope="col" className="py-2 pr-4 font-bold">
              System
            </th>
            <th
              scope="col"
              className="hidden py-2 pr-4 font-bold sm:table-cell"
            >
              File
            </th>
            <th scope="col" className="py-2 pr-4 text-right font-bold">
              Size
            </th>
            <th scope="col" className="py-2 text-right font-bold">
              <span className="sr-only">Download</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {release.assets.map((asset) => (
            <tr key={asset.name} className="border-b border-border">
              <th scope="row" className="py-3 pr-4 text-left font-semibold">
                {asset.platform}{' '}
                <span className="font-normal text-muted-foreground">
                  {asset.arch}
                </span>
              </th>
              <td className="hidden py-3 pr-4 font-mono text-[0.8125rem] sm:table-cell">
                {asset.name}
              </td>
              <td className="py-3 pr-4 text-right tabular-nums">
                {formatBytes(asset.size)}
              </td>
              <td className="py-3 text-right">
                <a
                  href={asset.url}
                  aria-label={`Download for ${asset.platform} ${asset.arch}`}
                  className={buttonVariants({ size: 'sm' })}
                  onClick={() => track('file_download', { name: asset.name })}
                >
                  <DownloadSimple className="size-4" weight="bold" />
                  Download
                </a>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="mt-4 text-sm text-muted-foreground">
        {release.checksumsUrl && (
          <>
            <a
              href={release.checksumsUrl}
              className={quietLink}
              onClick={() => track('file_download', { name: 'SHA256SUMS' })}
            >
              SHA256 checksums
            </a>{' '}
            ·{' '}
          </>
        )}
        <a
          href={release.url}
          className={quietLink}
          onClick={() => track('outbound_click', { location: 'books-release' })}
        >
          Release notes
        </a>
      </p>
    </>
  )
}

function EarlyAccess() {
  return (
    <div className="grid gap-8 lg:grid-cols-12 lg:items-end">
      <div className="lg:col-span-7">
        <SectionHead
          id="books-download"
          title="Not released yet"
          lede="The first public version is being prepared. Ask for early access and we will send it to you when it is ready."
        />
      </div>
      <div className="flex flex-col gap-4 lg:col-span-5 lg:items-end">
        <EarlyAccessLink location="books-download" />
        <a
          href={BOOKS_REPO_URL}
          className={cn('inline-flex items-center gap-2 text-sm', quietLink)}
          onClick={() => track('outbound_click', { location: 'books-source' })}
        >
          <GithubLogo className="size-4" weight="bold" />
          Source on GitHub
        </a>
      </div>
    </div>
  )
}

function Requirements() {
  return (
    <section
      aria-labelledby="books-install"
      className="border-t-[3px] border-foreground"
    >
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-8 lg:py-20">
        <SectionHead
          id="books-install"
          title="Requirements and install"
          lede="One archive per system. Unpack it and run the app; the archive’s INSTALL.txt has the details."
        />
        <div className="mt-12 grid gap-10 border-t border-border pt-8 lg:grid-cols-3">
          <div>
            <h3 className="section-label">Linux</h3>
            <ul className="mt-3 list-disc space-y-2 pl-5 text-sm leading-relaxed">
              <li>64-bit Intel or AMD (x86_64).</li>
              <li>A Wayland or X11 desktop and a Vulkan graphics driver.</li>
              <li>
                Common system libraries; INSTALL.txt lists the one{' '}
                <span className="font-mono text-[0.8125rem] whitespace-nowrap">
                  apt-get
                </span>{' '}
                line for Debian and Ubuntu.
              </li>
            </ul>
          </div>
          <div>
            <h3 className="section-label">macOS</h3>
            <ul className="mt-3 list-disc space-y-2 pl-5 text-sm leading-relaxed">
              <li>Apple silicon (arm64).</li>
              <li>
                The app is not signed yet: open it the first time with
                right-click, Open.
              </li>
            </ul>
          </div>
          <div>
            <h3 className="section-label">Demo or your own books</h3>
            <ul className="mt-3 list-disc space-y-2 pl-5 text-sm leading-relaxed">
              <li>
                Started as it is, the app opens demo books in memory. Nothing is
                saved.
              </li>
              <li>
                The archive also holds a small{' '}
                <span className="font-mono text-[0.8125rem]">config.toml</span>.
                It names the FlowTrace server and the production database, and
                holds no password or key. Copy it into the app’s settings folder
                to open real books.
              </li>
              <li>
                On first connect the app makes a token file, which is your
                login. Keep it private. Someone in your organisation then adds
                you.
              </li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  )
}

function Close() {
  return (
    <section className="border-t-[3px] border-foreground">
      <div className="mx-auto flex max-w-7xl flex-col gap-8 px-4 py-16 sm:px-8 md:flex-row md:items-end md:justify-between lg:py-20">
        <div className="max-w-2xl">
          <h2 className="text-4xl leading-none sm:text-6xl">
            Try the books in your browser first.
          </h2>
          <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
            The web demo opens a quarter of an NGO’s books, proof included.{' '}
            <Link to="/" className={quietLink}>
              See what FlowTrace does
            </Link>{' '}
            for NGOs, small businesses, funders and investors, and donors.
          </p>
        </div>
        <DemoLink
          location="books-close"
          className={cn(buttonVariants({ size: 'lg' }), 'group shrink-0')}
        >
          See the web demo
          <ArrowRight className={arrow} weight="bold" />
        </DemoLink>
      </div>
    </section>
  )
}
