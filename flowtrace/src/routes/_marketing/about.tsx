import { createFileRoute } from '@tanstack/react-router'
import { GithubLogo } from '@phosphor-icons/react'
import {
  CONTACT_EMAIL,
  CtaRow,
  DemoCta,
  PageHero,
  Section,
  mailto,
  quietLink,
} from '@/components/marketing/page-parts'
import { track } from '@/lib/analytics'
import { GITHUB_URL, pageHead } from '@/lib/site'

export const ABOUT_TITLE = 'About'
export const ABOUT_DESCRIPTION =
  'Why FlowTrace exists, what it is built on, and how to reach the people who build it. The code is open on GitHub.'

export const Route = createFileRoute('/_marketing/about')({
  head: () =>
    pageHead({
      title: ABOUT_TITLE,
      description: ABOUT_DESCRIPTION,
      path: '/about',
    }),
  component: AboutPage,
})

// Facts the user has not supplied yet (brief LP3): leave the marker until they do.
const FOUNDER = 'TODO-USER'

const ISSUES_URL = `${GITHUB_URL}/issues`

function AboutPage() {
  return (
    <>
      <PageHero
        eyebrow="About"
        title="Every movement drawn with its proof."
        lede="FlowTrace keeps an organisation’s books with the evidence attached to each payment, and lets it choose who sees how much."
      >
        <CtaRow>
          <DemoCta location="about-hero" />
          <a
            href="#contact"
            className={quietLink}
            onClick={() =>
              track('cta_click', { location: 'about-hero-contact' })
            }
          >
            Contact us
          </a>
        </CtaRow>
      </PageHero>

      <Section id="why" title="Why FlowTrace exists">
        {/* DRAFT for the user to edit: the founder's name and the origin are TODO-USER. */}
        <p>
          Organisations that do good work often lose funding because they cannot
          show, quickly and plainly, where the money went. They rebuild the same
          proof in spreadsheets for every funder, and the funder still has to
          take the report on trust. FlowTrace started from a simple idea: if
          each payment carries its own proof from the moment it is recorded, the
          report writes itself, and a funder abroad can see the evidence instead
          of a claim about it. [Origin: TODO-USER]
        </p>
        <p className="text-muted-foreground">{FOUNDER}, founder</p>
      </Section>

      <Section id="built-on" title="What it is built on">
        <ul className="list-disc space-y-3 pl-5">
          <li>
            <strong>An open repository.</strong> All of FlowTrace’s code is
            public on GitHub.
          </li>
          {/* rust/crates/ledger-core: the shared posting engine, at parity with the TS engine. */}
          <li>
            <strong>A ledger core written in Rust.</strong> One double-entry
            engine keeps the books for the web app and the desktop app, and is
            tested against the same cases.
          </li>
          {/* rust/crates/ledger-module, hosted on maincloud. */}
          <li>
            <strong>SpacetimeDB.</strong> The books live in one database that
            checks every change against who is allowed to make it.
          </li>
          <li>
            <strong>One design rule.</strong> Every movement of money is drawn
            with its proof state: evidenced, awaiting proof, flagged or
            rejected, told apart by the line, never by colour alone.
          </li>
        </ul>
      </Section>

      <Section id="open" title="Built in the open">
        <p>
          You can read the code, follow the work and see what changed in each
          release.
        </p>
        <ul className="space-y-3">
          <li>
            <a
              href={GITHUB_URL}
              className={`inline-flex items-center gap-2 ${quietLink}`}
              onClick={() =>
                track('outbound_click', { location: 'about-github' })
              }
            >
              <GithubLogo aria-hidden="true" className="size-4" weight="bold" />
              The FlowTrace repository on GitHub
            </a>
          </li>
          <li>
            <a href="/changelog" className={quietLink}>
              The changelog
            </a>
          </li>
        </ul>
      </Section>

      <Section id="contact" title="Contact">
        <p>
          Email us at{' '}
          <a
            href={mailto('Hello from the FlowTrace site')}
            className={quietLink}
            onClick={() => track('cta_click', { location: 'about-contact' })}
          >
            {CONTACT_EMAIL}
          </a>
          .
        </p>
        <p>
          For a bug or a feature request, you can also{' '}
          <a
            href={ISSUES_URL}
            className={quietLink}
            onClick={() =>
              track('outbound_click', { location: 'about-issues' })
            }
          >
            open an issue on GitHub
          </a>
          . To report a security problem, see{' '}
          <a href="/security#disclosure" className={quietLink}>
            the security page
          </a>
          .
        </p>
      </Section>
    </>
  )
}
