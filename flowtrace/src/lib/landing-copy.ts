// Every sentence of the landing page (routes/_marketing/index.tsx), in one place so the copy can
// be read, counted and graded on its own (decision Q8: strings centralised). Figures are not here:
// they come from lib/landing-figures.ts.
import { DEMO_PATH } from './marketing-nav'
import { LANDING_CAMPAIGN_SLUG } from './landing-figures'

export const HERO = {
  title: 'Every payment, traced to its proof.',
  lede: 'FlowTrace keeps every grant, sale and payment with its receipt, photo or bank statement. Funders, investors and donors see as much as you choose.',
  primary: 'Explore the demo',
  secondary: 'See how proof works',
  note: 'The demo opens as Demo NGO Kenya. No account needed.',
  figuresLabel: 'Key figures, Q1 2026',
  donorsLabel: 'Funding received, by donor',
}

export const DOORS = {
  title: 'Who is this for?',
  items: [
    {
      name: 'NGOs',
      text: 'Keep your books and prove each payment to every funder.',
      outcome: 'One record, not one report per funder.',
      link: { label: 'Open the demo books', href: DEMO_PATH },
    },
    {
      name: 'Small businesses',
      text: 'Keep books that make you fundable: a clear record an investor can read.',
      outcome: 'Show your numbers with the proof behind them.',
      link: { label: 'See FlowTrace Books', href: '/books' },
    },
    {
      name: 'Funders and investors',
      text: 'Follow your money past the door of the group you fund.',
      outcome: 'See where it went, not a summary of it.',
      link: { label: 'Read the funder guide', href: '/funders' },
    },
    {
      name: 'Donors',
      text: 'Give to a campaign and watch where the money goes.',
      outcome: 'Each payment shows its proof.',
      link: {
        label: 'See a demo campaign',
        href: `/donate/${LANDING_CAMPAIGN_SLUG}`,
      },
    },
  ],
} as const

export const PRODUCTS = {
  title: 'One ledger, two ways in.',
  lede: 'Both read and write the same books, so a payment entered in one shows in the other.',
  items: [
    {
      name: 'FlowTrace web',
      text: 'Show the money. Flow maps, funder chains, the Trust Score, public pages and campaigns, in any browser.',
      image: {
        src: '/screens/web-flow-map.webp',
        width: 1200,
        height: 677,
        alt: 'The FlowTrace web flow map for Demo NGO Kenya: donors on the left, lines to accounts, projects and vendors, with dashed lines for payments awaiting proof.',
      },
      link: { label: 'Explore the demo', href: DEMO_PATH },
    },
    {
      name: 'FlowTrace Books',
      text: 'Keep the books. Double-entry books on your own computer that work offline and sync when you reconnect.',
      image: {
        src: '/screens/books-trial-balance.webp',
        width: 1200,
        height: 678,
        alt: 'The FlowTrace Books desktop app showing the trial balance for Demo NGO Kenya, with accounts, debits and credits.',
      },
      link: { label: 'About FlowTrace Books', href: '/books' },
    },
  ],
} as const

export const LIVE_LEDGER = {
  title: 'Donors see the spending, not a summary.',
  label: 'Live demo books',
  raised: 'Raised so far',
  spending: 'Recent spending, with proof',
  caption:
    "Every organisation on FlowTrace gets a page like this; the pilot organisation's will replace this one.",
  link: 'Open the full campaign page',
}

export const HOW = {
  title: 'How proof works.',
  lede: 'Each payment is recorded once, with its proof attached when it happens. Nobody rebuilds the trail at report time. The trail is the record.',
  traceLabel: 'One borehole, from grant to drill',
  donorStation: 'The donor. It makes the grant.',
  traceNote: 'From the demo books. In the app, every line opens to its files.',
  kindsTitle: 'Eleven kinds of proof',
  kindsLede: 'Each kind adds trust points to the payment it backs.',
  scoreTitle: 'The Trust Score',
  scoreLede:
    'A score from 0 to 100, made of four parts. The weights are fixed and the same for everyone.',
  parts: {
    Verification: 'Share of payments whose proof was checked.',
    Documentation: 'Share of payments with proof attached.',
    Consistency: 'Amounts, dates and names agree.',
    'Independent review':
      'A fixed review of the records, less 5 points for each confirmed flag.',
  },
  gradesLabel: 'Grades',
  badgeLabel: 'The demo badge',
  method: 'How the score is computed',
} as const

export const AUDIENCE = {
  title: 'Each audience sees its own depth.',
  lede: 'Your board sees the original receipts. Funders and donors see where their money went. Partners see their own part. The public sees names, not amounts. You set the depth for each group.',
  note: 'The five groups every organisation starts with. Add your own, and make access end for an auditor.',
  scope: {
    Board: 'Everything',
    Staff: 'Everything',
    'Funders and donors': 'Their money, all the way down',
    Partners: 'The parts they work on',
    Public: 'All flows, names only',
  } as Record<string, string>,
}

export const FUNDERS = {
  title: 'Funders follow the grant past your door.',
  lede: 'Link your FlowTrace to a funder, and to your partners. Once each side agrees, a funder can follow its grant through you to the partner that did the work. Each link shows only what it allows.',
  mandate:
    'A funder that asks all its grantees for open books gets one view of the whole chain, with no extra reports.',
  key: 'A solid line is an approved link. A dashed line is a request the other side has not answered. Either side can end a link at any time.',
  link: 'For funders',
}

export const WORKS = {
  title: 'Works where you work.',
  items: [
    {
      name: 'Offline books',
      text: 'FlowTrace Books keeps working with no connection and syncs when you are back online.',
    },
    {
      name: 'Mobile money and bank statements',
      text: 'Import M-Pesa, MTN MoMo, Airtel Money or bank statement files. FlowTrace matches each line to your books.',
    },
    {
      name: 'Your currency',
      text: 'The demo keeps its books in KES. Your books can use any base currency, and a payment in another currency keeps its rate.',
    },
    {
      name: 'Light pages',
      text: 'Pages are kept small, so they open on a slow mobile network.',
    },
  ],
} as const

export const TRUST = {
  title: 'Where your data lives.',
  items: [
    {
      name: 'Books',
      text: 'SpacetimeDB maincloud, run by Clockwork Labs in the US.',
    },
    { name: 'Web app', text: 'Hosted on Vercel.' },
    { name: 'Proof files', text: 'Stored with Cloudflare.' },
    { name: 'Sign-in', text: 'WorkOS.' },
    {
      name: 'Audit trail',
      text: 'Every change is kept, with who made it and when.',
    },
    { name: 'Export', text: 'Reports and journals download as CSV.' },
    { name: 'Source code', text: 'Open on GitHub.' },
    { name: 'Security audit', text: 'Not done yet. We say so until one is.' },
  ],
  security: 'Read the security page',
  stage: 'Pilot stage. Free during the pilot.',
  pricing: 'Pricing',
} as const

export const FAQ = {
  title: 'Questions, answered.',
  lede: 'The short version. The docs have the long one.',
  items: [
    {
      question: 'Is it free?',
      answer:
        'Yes. FlowTrace is free during the pilot. Any future prices will be on the pricing page first.',
    },
    {
      question: 'Do I need an account to look?',
      answer:
        'No. The demo opens as Demo NGO Kenya, with its books, proofs and reports. Your changes reset when you reload.',
    },
    {
      question: 'What is a proof?',
      answer:
        'A file or record attached to a payment: a receipt, invoice, photo, bank statement, contract, signature, GPS reading, video, audit or official letter.',
    },
    {
      question: 'Who can see my numbers?',
      answer:
        'Only the people you let in. Each group sees what you set. Your board can open the receipts, and the public sees names but no amounts.',
    },
    {
      question: 'Does it work offline?',
      answer:
        'FlowTrace Books does. It keeps your books on your computer and syncs when you reconnect. Spend claims in the web app also wait and send once you are online.',
    },
    {
      question: 'Can I export my data?',
      answer: 'Yes. Reports and journals download as CSV files.',
    },
  ],
} as const

export const CLOSE = {
  title: "Open a quarter of an organisation's books, proof included.",
  cta: 'Explore the demo',
}

/** Every sentence a reader sees, for the word count and readability checks. */
export function landingCopy(): Array<string> {
  return [
    HERO.title,
    HERO.lede,
    HERO.note,
    DOORS.title,
    ...DOORS.items.flatMap((d) => [d.name, d.text, d.outcome, d.link.label]),
    PRODUCTS.title,
    PRODUCTS.lede,
    ...PRODUCTS.items.flatMap((p) => [p.name, p.text, p.link.label]),
    LIVE_LEDGER.title,
    LIVE_LEDGER.caption,
    HOW.title,
    HOW.lede,
    HOW.donorStation,
    HOW.traceNote,
    HOW.kindsTitle,
    HOW.kindsLede,
    HOW.scoreTitle,
    HOW.scoreLede,
    ...Object.values(HOW.parts),
    AUDIENCE.title,
    AUDIENCE.lede,
    AUDIENCE.note,
    FUNDERS.title,
    FUNDERS.lede,
    FUNDERS.mandate,
    FUNDERS.key,
    WORKS.title,
    ...WORKS.items.flatMap((w) => [w.name, w.text]),
    TRUST.title,
    ...TRUST.items.flatMap((t) => [t.name, t.text]),
    TRUST.stage,
    FAQ.title,
    FAQ.lede,
    ...FAQ.items.flatMap((f) => [f.question, f.answer]),
    CLOSE.title,
  ]
}
