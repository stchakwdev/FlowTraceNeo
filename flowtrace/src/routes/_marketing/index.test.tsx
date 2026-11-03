import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen } from '@testing-library/react'
import readability from 'text-readability'
import { LANDING_DESCRIPTION, LANDING_TITLE, LandingPage, Route } from './index'
import { renderWithRouter } from '@/test/render-with-router'
import { SITE_URL } from '@/lib/site'
import { HERO, landingCopy } from '@/lib/landing-copy'
import { HERO_FIGURES, LANDING_CAMPAIGN } from '@/lib/landing-figures'
import { formatAmount, formatCompact } from '@/lib/format'
import {
  DEMO_ORG_ID,
  campaigns,
  demoOrganization,
  flowNodes,
  transactions,
} from '@/lib/demo/dataset'

const SECTION_HEADINGS = [
  'Who is this for?',
  'One ledger, two ways in.',
  'Donors see the spending, not a summary.',
  'How proof works.',
  'Each audience sees its own depth.',
  'Funders follow the grant past your door.',
  'Works where you work.',
  'Where your data lives.',
  'Questions, answered.',
  "Open a quarter of an organisation's books, proof included.",
]

/** Copy as running prose: a line without end punctuation (a label) gets a full stop. */
const prose = () =>
  landingCopy()
    .map((line) => (/[.?!:]$/.test(line) ? line : `${line}.`))
    .join(' ')

const umami = { track: vi.fn() }
beforeEach(() => {
  umami.track.mockClear()
  vi.stubGlobal('umami', umami)
})
afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('landing page', () => {
  it('links "How the score is computed" to the Trust Score methodology page', async () => {
    await renderWithRouter(() => <LandingPage />)
    expect(
      screen
        .getByRole('link', { name: /How the score is computed/ })
        .getAttribute('href'),
    ).toBe('/docs/trust-score')
  })

  it('renders the H1 and every section H2 in order', async () => {
    await renderWithRouter(() => <LandingPage />)
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe(
      'Every payment, traced to its proof.',
    )
    const h2s = screen
      .getAllByRole('heading', { level: 2 })
      .map((h) => h.textContent)
      .filter((t) => t !== HERO.figuresLabel)
    expect(h2s).toEqual(SECTION_HEADINGS)
  })

  it('keeps the anchors the masthead and hero link to', async () => {
    const { container } = await renderWithRouter(() => <LandingPage />)
    for (const id of ['product', 'how', 'funders'])
      expect(container.querySelector(`#${id}`)).not.toBeNull()
  })

  it('links the demo CTAs to /dashboard and tracks them by location', async () => {
    await renderWithRouter(() => <LandingPage />)
    const ctas = screen.getAllByRole('link', { name: /^Explore the demo/ })
    expect(ctas.length).toBeGreaterThanOrEqual(2)
    for (const cta of ctas) expect(cta.getAttribute('href')).toBe('/dashboard')

    fireEvent.click(ctas[0])
    expect(umami.track).toHaveBeenCalledWith('cta_click', { location: 'hero' })
    expect(umami.track).toHaveBeenCalledWith('demo_started', {
      location: 'hero',
    })

    umami.track.mockClear()
    fireEvent.click(ctas[ctas.length - 1])
    expect(umami.track).toHaveBeenCalledWith('cta_click', { location: 'close' })
    expect(umami.track).toHaveBeenCalledWith('demo_started', {
      location: 'close',
    })
  })

  it('tracks the secondary hero link and the door links', async () => {
    await renderWithRouter(() => <LandingPage />)
    const proof = screen.getByRole('link', { name: HERO.secondary })
    expect(proof.getAttribute('href')).toBe('#how')
    fireEvent.click(proof)
    expect(umami.track).toHaveBeenCalledWith('cta_click', {
      location: 'hero_proof',
    })
    const funder = screen.getByRole('link', { name: /Read the funder guide/ })
    expect(funder.getAttribute('href')).toBe('/funders')
    fireEvent.click(funder)
    expect(umami.track).toHaveBeenCalledWith('cta_click', {
      location: 'door_funders_and_investors',
    })
    expect(
      screen
        .getByRole('link', { name: /See a demo campaign/ })
        .getAttribute('href'),
    ).toBe('/donate/clean-water-turkana')
  })

  it('shows hero figures equal to the demo dataset', async () => {
    const own = transactions.filter((t) => t.organization_id === DEMO_ORG_ID)
    const donorIds = new Set(
      flowNodes.filter((n) => n.type === 'donor').map((n) => n.id),
    )
    const evidenced = own
      .filter(
        (t) =>
          t.verification_status === 'verified' &&
          donorIds.has(t.from_node_id ?? ''),
      )
      .reduce((s, t) => s + t.amount, 0)
    const verified = own.filter((t) => t.verification_status === 'verified')
    const pending = own.filter((t) => t.verification_status === 'pending')
    const flagged = own.filter((t) => t.verification_status === 'flagged')

    expect(HERO_FIGURES.evidencedTotal).toBe(evidenced)
    expect(HERO_FIGURES.movements).toBe(`${verified.length}/${own.length}`)
    expect(HERO_FIGURES.awaiting).toBe(pending.length)
    expect(HERO_FIGURES.flagged).toBe(flagged.length)
    expect(HERO_FIGURES.trustScore).toBe(demoOrganization.trust_score)

    await renderWithRouter(() => <LandingPage />)
    const text = document.body.textContent
    expect(text).toContain(formatCompact(evidenced))
    expect(text).toContain(`${verified.length}/${own.length}`)
    expect(text).toContain(`KES ${formatAmount(evidenced)} evidenced`)
    expect(text).toContain(`${flagged.length} flagged`)
  })

  it('shows the demo campaign from the dataset, labelled as demo books', async () => {
    const seed = campaigns.find((c) => c.public_slug === 'clean-water-turkana')
    expect(LANDING_CAMPAIGN.campaign.current_amount).toBe(seed?.current_amount)
    await renderWithRouter(() => <LandingPage />)
    expect(screen.getByText('Live demo books')).toBeTruthy()
    expect(screen.getByText('Clean Water for Turkana')).toBeTruthy()
    expect(document.body.textContent).toContain(
      formatAmount(LANDING_CAMPAIGN.campaign.current_amount),
    )
    expect(screen.getAllByText('Demo data').length).toBeGreaterThanOrEqual(3)
  })

  it('renders every FAQ answer on the server, one open at a time', async () => {
    await renderWithRouter(() => <LandingPage />)
    const question = screen.getByRole('button', { name: 'Is it free?' })
    expect(question.getAttribute('aria-expanded')).toBe('false')
    expect(document.body.textContent).toContain('free during the pilot')
    fireEvent.click(question)
    expect(question.getAttribute('aria-expanded')).toBe('true')
  })

  it('sets its title, description, canonical and Organization JSON-LD', async () => {
    const head = await Route.options.head!({} as never)
    expect(head.meta).toContainEqual({ title: LANDING_TITLE })
    expect(LANDING_TITLE).toBe('FlowTrace · Every payment, traced to its proof')
    expect(head.meta).toContainEqual({
      name: 'description',
      content: LANDING_DESCRIPTION,
    })
    expect(LANDING_DESCRIPTION.length).toBeLessThan(160)
    expect(head.links).toContainEqual({
      rel: 'canonical',
      href: `${SITE_URL}/`,
    })
    const ld = head.scripts?.find((s) => s?.type === 'application/ld+json')
    expect(JSON.parse(String(ld?.children))).toMatchObject({
      '@type': 'Organization',
    })
  })
})

describe('landing copy', () => {
  it('reads below grade 8 and stays between 600 and 900 words', () => {
    const text = prose()
    expect(readability.fleschKincaidGrade(text)).toBeLessThan(8)
    const words = text.split(/\s+/).filter(Boolean).length
    expect(words).toBeGreaterThanOrEqual(600)
    expect(words).toBeLessThanOrEqual(900)
  })

  it('keeps the hero subhead under 25 words', () => {
    expect(HERO.lede.split(/\s+/).length).toBeLessThan(25)
  })

  it('has no emoji', () => {
    for (const line of landingCopy())
      expect(line).not.toMatch(/\p{Extended_Pictographic}/u)
  })

  it('says shilling only inside demo figures', () => {
    for (const line of landingCopy()) expect(line).not.toMatch(/shilling/i)
  })
})

describe('marketing code', () => {
  const files = (dir: string): Array<string> =>
    readdirSync(dir).flatMap((name) => {
      const path = join(dir, name)
      return statSync(path).isDirectory() ? files(path) : [path]
    })

  it('imports no lucide-react icons', () => {
    const root = join(__dirname, '..', '..')
    const offenders = files(root)
      .filter((f) => /\.(ts|tsx)$/.test(f) && !f.endsWith('index.test.tsx'))
      .filter((f) => /from ['"]lucide-react['"]/.test(readFileSync(f, 'utf8')))
    expect(offenders).toEqual([])
  })
})
