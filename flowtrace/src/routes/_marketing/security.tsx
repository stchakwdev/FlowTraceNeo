import { useEffect } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { PageHero, Section, quietLink } from '@/components/marketing/page-parts'
import { track } from '@/lib/analytics'
import { formatDate } from '@/lib/format'
import { GITHUB_URL, pageHead } from '@/lib/site'

export const SECURITY_TITLE = 'Security'
export const SECURITY_DESCRIPTION =
  'Where FlowTrace keeps your data, who can see what, what is recorded, and what has not been done yet: no third-party audit and no certification. How to report a vulnerability.'

/**
 * The date this page's facts were last checked against the repository. A constant, not
 * `Date.now()`, so the server and the browser render the same text. Change it whenever a
 * statement below changes.
 */
export const SECURITY_LAST_UPDATED = '2026-11-03'

// Facts the user has not supplied yet (brief LP3): leave the marker until they do.
const OWNER = 'TODO-USER'
const DISCLOSURE_EMAIL = 'TODO-USER'

export const Route = createFileRoute('/_marketing/security')({
  head: () =>
    pageHead({
      title: SECURITY_TITLE,
      description: SECURITY_DESCRIPTION,
      path: '/security',
    }),
  component: SecurityPage,
})

/** One provider's own statement about encryption at rest, linked, and nothing more. */
export const AT_REST: ReadonlyArray<{
  provider: string
  what: string
  statement: string
  href: string
}> = [
  // Read on 2026-11-03.
  {
    provider: 'Cloudflare R2',
    what: 'proof files',
    statement:
      '“All objects stored in R2, including their metadata, are encrypted at rest.”',
    href: 'https://developers.cloudflare.com/r2/reference/data-security/',
  },
  {
    provider: 'Vercel',
    what: 'the web app',
    statement:
      '“Vercel encrypts data at rest (when on disk) with 256 bit Advanced Encryption Standard (AES-256).”',
    href: 'https://vercel.com/docs/security/compliance',
  },
]

// No public at-rest statement found on 2026-11-03 (spacetimedb.com/maincloud and its deploy
// docs; workos.com/security). Say so rather than assume.
const NO_STATEMENT = ['Clockwork Labs (SpacetimeDB maincloud)', 'WorkOS']

// Who handles data for FlowTrace, and for what.
const SUBPROCESSORS: ReadonlyArray<{ name: string; role: string }> = [
  {
    name: 'Clockwork Labs (SpacetimeDB maincloud)',
    role: 'Hosts the ledger database: the books, proofs records, access groups and audit log. United States.',
  },
  {
    name: 'Vercel',
    role: 'Hosts and serves the web app.',
  },
  {
    name: 'Cloudflare (Workers and R2)',
    role: 'Stores proof files and hands them out to people allowed to see them.',
  },
  {
    name: 'WorkOS',
    role: 'Signs people in.',
  },
  {
    name: 'Umami',
    role: 'Counts page views and clicks on the public pages, when analytics is switched on.',
  },
  {
    name: 'GitHub',
    role: 'Hosts the source code, the desktop app’s downloads and public issues.',
  },
]

// The five groups every organisation starts with: rust/crates/ledger-module/src/transparency/plan.rs:247
// (default_groups), ordered board, staff, donors, partners, public (plan_tests.rs:334).
const GROUPS: ReadonlyArray<{ name: string; sees: string }> = [
  { name: 'Board', sees: 'Everything, including original proof files.' },
  { name: 'Staff', sees: 'Everything, with small copies of proofs.' },
  {
    name: 'Donors',
    sees: 'Their own money, followed downstream, with small copies of proofs.',
  },
  {
    name: 'Partners',
    sees: 'Only the parts of the flow assigned to them, with amounts but no proofs.',
  },
  { name: 'Public', sees: 'The flows by name, without amounts or proofs.' },
]

// Visibility levels: `Visibility` in rust/crates/ledger-module/src/transparency/types.rs:255.
const LEVELS: ReadonlyArray<{ name: string; means: string }> = [
  { name: 'Full', means: 'The original file.' },
  { name: 'Thumbnail', means: 'A small, low-resolution copy.' },
  { name: 'Summary', means: 'That a proof is attached, but not the file.' },
  { name: 'Hidden', means: 'Nothing at all.' },
]

const dl = 'mt-2 divide-y divide-border border-y border-border'
const dlRow = 'grid gap-1 py-3 sm:grid-cols-[10rem_1fr] sm:gap-6'

function SecurityPage() {
  useEffect(() => track('security_viewed'), [])

  return (
    <>
      <PageHero
        eyebrow="Security"
        title="What we do with your data, and what we have not done yet."
        lede="This page says where FlowTrace keeps data, who can see it, and what is still missing. If something here is out of date, tell us."
      >
        <p className="mt-6 text-sm text-muted-foreground">
          Last updated{' '}
          <time dateTime={SECURITY_LAST_UPDATED}>
            {formatDate(SECURITY_LAST_UPDATED)}
          </time>
          {' · '}Maintained by {OWNER}
        </p>
      </PageHero>

      <Section
        id="where"
        title="Where data lives"
        lede="All of it is hosted outside Kenya."
      >
        <ul className="list-disc space-y-3 pl-5">
          {/* rust/crates/ledger-module (the SpacetimeDB module); host in
              workers/proofs/wrangler.toml (SPACETIMEDB_URL = maincloud). */}
          <li>
            <strong>The books</strong> live in a SpacetimeDB database on
            maincloud, run by Clockwork Labs in the United States.
          </li>
          <li>
            <strong>The web app</strong> runs on Vercel.
          </li>
          {/* workers/proofs/README.md and wrangler.toml (R2 bucket flowtrace-proofs). */}
          <li>
            <strong>Proof files</strong> (receipts, invoices, photos) are kept
            in Cloudflare R2, behind a Cloudflare Worker that checks who is
            asking.
          </li>
          <li>
            <strong>Sign-in</strong> is handled by WorkOS.
          </li>
        </ul>
        <p>
          If your organisation is in Kenya, treat using FlowTrace as a transfer
          of personal data out of the country under the Kenya Data Protection
          Act, 2019. Your organisation decides what data goes in, so it is the
          data controller. We do not give legal advice; ask your own adviser
          what the Act asks of you.
        </p>
      </Section>

      <Section
        id="access"
        title="Who can see what"
        lede="Each organisation decides who sees which parts of its books, and how much of each proof."
      >
        <p>
          People see an organisation’s books through access groups. Every
          organisation starts with five:
        </p>
        <dl className={dl}>
          {GROUPS.map((g) => (
            <div key={g.name} className={dlRow}>
              <dt className="font-semibold">{g.name}</dt>
              <dd className="text-muted-foreground">{g.sees}</dd>
            </div>
          ))}
        </dl>
        <p>For each proof, a group gets one of four levels:</p>
        <dl className={dl}>
          {LEVELS.map((l) => (
            <div key={l.name} className={dlRow}>
              <dt className="font-semibold">{l.name}</dt>
              <dd className="text-muted-foreground">{l.means}</dd>
            </div>
          ))}
        </dl>
        {/* public tables: transparency/tables.rs:419-460 (the only tables declared `public`);
            every other table is private, tables.rs:3 and transparency/tables.rs:3. */}
        <p>
          Three things can be read without signing in, and only once an
          organisation publishes them: its public profile, its donation
          campaigns and its trust badge. Everything else is private and is read
          through views that check membership.
        </p>
        {/* rules.rs:137 (actor_of) and transparency/chains.rs:145 refuse a non-member. */}
        <p>
          Every change goes through the database module, which checks that the
          person is a member of the organisation, with the right permission,
          before it writes anything.
        </p>
        {/* workers/proofs/README.md, Routes: 10-minute signed links; 404 for files you may not see. */}
        <p>
          Proof files are handed out as links that stop working after 10
          minutes. A file you may not see looks the same as a file that does not
          exist.
        </p>
      </Section>

      <Section id="audit" title="Audit trail">
        {/* flowtrace/src/lib/ledger/engine.ts:143 (audit: at, by, action, subject); the
            audit_event table is append-only, rust/crates/ledger-module/src/tables.rs:491. */}
        <p>
          Every change to the books is written to an audit log: who made it,
          when, what they did and to which record. Entries are only added, never
          edited. A posted entry is never changed; a correction is a reversal
          plus a new entry.
        </p>
        {/* verification_access_log and chain_activity_log, transparency/tables.rs:74 and :386. */}
        <p>
          FlowTrace also logs who opened which proof, and every request,
          approval and revocation of a link between two organisations.
        </p>
      </Section>

      <Section id="encryption" title="Encryption">
        <p>Data moves between your device and FlowTrace over HTTPS (TLS).</p>
        {AT_REST.length > 0 && (
          <>
            <p>
              For data at rest, here is what each provider says about its own
              service. We add nothing to it.
            </p>
            <ul className="list-disc space-y-3 pl-5">
              {AT_REST.map((p) => (
                <li key={p.provider}>
                  <strong>{p.provider}</strong> ({p.what}): {p.statement}{' '}
                  <a
                    href={p.href}
                    className={quietLink}
                    onClick={() =>
                      track('outbound_click', { location: 'security-at-rest' })
                    }
                  >
                    Source
                  </a>
                </li>
              ))}
            </ul>
            <p>
              We found no public statement about encryption at rest from{' '}
              {NO_STATEMENT.join(' or ')}, so we make no claim for them.
            </p>
          </>
        )}
      </Section>

      <Section id="export" title="Export and deletion">
        {/* flowtrace/src/components/ledger/export.ts (toCsv). */}
        <p>You can export your books and reports as CSV files at any time.</p>
        {/* No delete-organisation reducer in rust/crates/ledger-module/src (checked 2026-11-03). */}
        <p>
          Deleting a whole organisation is not self-service yet. To have your
          organisation’s data deleted, write to us and we will do it by hand.
        </p>
      </Section>

      <Section id="status" title="Audits and certifications">
        <p>
          No independent security audit has been done, and none is scheduled.
          FlowTrace holds no SOC 2 report and no ISO 27001 certificate, and is
          not registered with Kenya’s Office of the Data Protection
          Commissioner. We will update this page when that changes.
        </p>
        <p>
          The code is open, so you can read it yourself:{' '}
          <a
            href={GITHUB_URL}
            className={quietLink}
            onClick={() =>
              track('outbound_click', { location: 'security-github' })
            }
          >
            the FlowTrace repository on GitHub
          </a>
          .
        </p>
      </Section>

      <Section id="disclosure" title="Report a vulnerability">
        <p>
          If you find a security problem, email{' '}
          <span className="font-mono">{DISCLOSURE_EMAIL}</span>. Please do not
          open a public issue for it.
        </p>
        <p>Tell us:</p>
        <ul className="list-disc space-y-2 pl-5">
          <li>what you found and where (the page, address or file);</li>
          <li>the steps to see it happen;</li>
          <li>what someone could do with it;</li>
          <li>how to reach you.</li>
        </ul>
        <p>
          We acknowledge every report and tell you what we will do about it.
          Please give us time to fix the problem before you talk about it in
          public.
        </p>
      </Section>

      <Section
        id="subprocessors"
        title="Who handles data for us"
        lede="The services FlowTrace runs on."
      >
        <dl className={dl}>
          {SUBPROCESSORS.map((s) => (
            <div key={s.name} className="grid gap-1 py-3">
              <dt className="font-semibold">{s.name}</dt>
              <dd className="text-muted-foreground">{s.role}</dd>
            </div>
          ))}
        </dl>
      </Section>
    </>
  )
}
