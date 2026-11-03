import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen } from '@testing-library/react'
import { renderWithRouter } from '@/test/render-with-router'
import { SITE_URL } from '@/lib/site'

const track = vi.hoisted(() => vi.fn())
vi.mock('@/lib/analytics', () => ({ track }))

const pricing = await import('./pricing')
const security = await import('./security')
const funders = await import('./funders')
const about = await import('./about')

afterEach(() => {
  cleanup()
  track.mockClear()
})

type Head = {
  meta: Array<Record<string, string>>
  links: Array<Record<string, string>>
}

const PAGES = [
  {
    path: '/pricing',
    route: pricing.Route,
    title: 'Pricing · FlowTrace',
    description: pricing.PRICING_DESCRIPTION,
  },
  {
    path: '/security',
    route: security.Route,
    title: 'Security · FlowTrace',
    description: security.SECURITY_DESCRIPTION,
  },
  {
    path: '/funders',
    route: funders.Route,
    title: 'For funders and investors · FlowTrace',
    description: funders.FUNDERS_DESCRIPTION,
  },
  {
    path: '/about',
    route: about.Route,
    title: 'About · FlowTrace',
    description: about.ABOUT_DESCRIPTION,
  },
]

const renderPage = (route: (typeof PAGES)[number]['route']) => {
  const Page = route.options.component as React.ComponentType
  return renderWithRouter(() => <Page />)
}

describe.each(PAGES)('$path head', ({ path, route, title, description }) => {
  it('sets title, description, canonical and Open Graph url', async () => {
    const head = (await route.options.head!({} as never)) as Head
    const meta = (key: string) =>
      head.meta.find((m) => m.name === key || m.property === key)?.content
    expect(head.meta[0]).toEqual({ title })
    expect(meta('description')).toBe(description)
    expect(description.length).toBeGreaterThan(50)
    expect(meta('og:url')).toBe(`${SITE_URL}${path}`)
    expect(meta('twitter:card')).toBe('summary_large_image')
    expect(head.links).toEqual([
      { rel: 'canonical', href: `${SITE_URL}${path}` },
    ])
  })
})

describe('no page makes a claim we cannot back', () => {
  it.each(PAGES)('$path', async ({ route }) => {
    const { container } = await renderPage(route)
    const text = container.textContent.toLowerCase()
    for (const banned of [
      'trusted by',
      'bank-grade',
      'soc 2 compliant',
      'certified',
    ]) {
      expect(text).not.toContain(banned)
    }
    expect(text).not.toMatch(/ai-powered/)
  })
})

describe('/pricing', () => {
  it('says free during the pilot in one card, with no prices or payment rails', async () => {
    await renderPage(pricing.Route)
    expect(
      screen.getByRole('heading', { level: 1, name: 'Free during the pilot.' }),
    ).toBeTruthy()
    expect(screen.getAllByRole('article')).toHaveLength(1)
    expect(screen.getByText('Pilot stage')).toBeTruthy()
    expect(
      screen.getAllByText(/Pricing is not set yet/).length,
    ).toBeGreaterThan(0)
    const text = document.body.textContent
    expect(text).not.toMatch(/M-Pesa|bank transfer|\$|KES|per month/)
    expect(screen.getAllByRole('group')).toHaveLength(4) // the four FAQs (<details>)
  })

  it('tracks the view on mount and both CTAs with their location', async () => {
    await renderPage(pricing.Route)
    expect(track).toHaveBeenCalledWith('pricing_viewed')

    const demo = screen.getAllByRole('link', { name: /Explore the demo/ })[0]
    expect(demo.getAttribute('href')).toBe('/dashboard')
    fireEvent.click(demo)
    expect(track).toHaveBeenCalledWith('cta_click', {
      location: 'pricing-hero',
    })
    expect(track).toHaveBeenCalledWith('demo_started', {
      location: 'pricing-hero',
    })

    const join = screen.getAllByRole('link', { name: 'Join the pilot' })[0]
    expect(join.getAttribute('href')).toMatch(
      /^mailto:stchakwera@gmail\.com\?subject=/,
    )
    fireEvent.click(join)
    expect(track).toHaveBeenCalledWith('cta_click', {
      location: 'pricing-hero-pilot',
    })
  })
})

describe('/security', () => {
  it('renders every section, tracks the view and shows the fixed last-updated date', async () => {
    await renderPage(security.Route)
    expect(track).toHaveBeenCalledWith('security_viewed')
    expect(
      screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent),
    ).toEqual([
      'Where data lives',
      'Who can see what',
      'Audit trail',
      'Encryption',
      'Export and deletion',
      'Audits and certifications',
      'Report a vulnerability',
      'Who handles data for us',
    ])
    expect(security.SECURITY_LAST_UPDATED).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    const updated = screen.getByText(/Last updated/)
    expect(updated.textContent).toContain('3 Nov 2026')
    expect(updated.querySelector('time')?.getAttribute('dateTime')).toBe(
      security.SECURITY_LAST_UPDATED,
    )
    expect(updated.textContent).toContain('Maintained by TODO-USER')
  })

  it('states plainly what has not been done', async () => {
    await renderPage(security.Route)
    expect(screen.getByText(/hosted outside Kenya/)).toBeTruthy()
    expect(screen.getByText(/Data Protection\s+Act, 2019/)).toBeTruthy()
    expect(
      screen.getByText(/No independent security audit has been done/),
    ).toBeTruthy()
    expect(screen.getByText(/not self-service yet/)).toBeTruthy()
    expect(screen.getByText(/We acknowledge every report/)).toBeTruthy()
    for (const name of ['Board', 'Staff', 'Donors', 'Partners', 'Public']) {
      expect(screen.getByText(name, { selector: 'dt' })).toBeTruthy()
    }
    for (const level of ['Full', 'Thumbnail', 'Summary', 'Hidden']) {
      expect(screen.getByText(level, { selector: 'dt' })).toBeTruthy()
    }
    for (const sub of [
      'Clockwork Labs',
      'Vercel',
      'Cloudflare',
      'WorkOS',
      'Umami',
      'GitHub',
    ]) {
      expect(
        screen.getAllByText(new RegExp(sub), { selector: 'dt' }).length,
      ).toBeGreaterThan(0)
    }
  })

  it('links every at-rest statement to the provider', () => {
    expect(security.AT_REST.length).toBeGreaterThan(0)
    for (const s of security.AT_REST) {
      expect(s.href).toMatch(/^https:\/\//)
    }
  })
})

describe('/funders', () => {
  it('renders the headline, the chain graphic and the demo directions', async () => {
    await renderPage(funders.Route)
    expect(
      screen.getByRole('heading', {
        level: 1,
        name: 'Follow your money past the grantee’s door.',
      }),
    ).toBeTruthy()
    expect(
      screen.getByRole('list', { name: /A chain of organisations/ }),
    ).toBeTruthy()
    expect(screen.getByText('Independent review')).toBeTruthy()
    expect(screen.getByText(/does not audit anyone/)).toBeTruthy()
    expect(screen.getByText('Chains')).toBeTruthy()
  })

  it('links the demo and Talk to us, tracking each with its location', async () => {
    await renderPage(funders.Route)
    const demo = screen.getAllByRole('link', {
      name: /Explore the demo as a funder/,
    })
    expect(demo.every((a) => a.getAttribute('href') === '/dashboard')).toBe(
      true,
    )
    fireEvent.click(demo[1])
    expect(track).toHaveBeenCalledWith('cta_click', {
      location: 'funders-close',
    })
    const talk = screen.getAllByRole('link', { name: 'Talk to us' })[0]
    expect(talk.getAttribute('href')).toMatch(/^mailto:stchakwera@gmail\.com/)
    fireEvent.click(talk)
    expect(track).toHaveBeenCalledWith('cta_click', {
      location: 'funders-hero-talk',
    })
  })
})

describe('/about', () => {
  it('renders the sections, the repository link and the contact anchor', async () => {
    const { container } = await renderPage(about.Route)
    expect(
      screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent),
    ).toEqual([
      'Why FlowTrace exists',
      'What it is built on',
      'Built in the open',
      'Contact',
    ])
    expect(container.querySelector('#contact')).toBeTruthy()
    expect(
      screen
        .getByRole('link', { name: /FlowTrace repository on GitHub/ })
        .getAttribute('href'),
    ).toBe('https://github.com/stchakwdev/FlowTraceNeo')
    expect(
      screen.getByRole('link', { name: 'The changelog' }).getAttribute('href'),
    ).toBe('/changelog')
    expect(
      screen
        .getByRole('link', { name: 'stchakwera@gmail.com' })
        .getAttribute('href'),
    ).toMatch(/^mailto:stchakwera@gmail\.com/)
    expect(container.textContent).toContain('TODO-USER, founder')
  })
})
