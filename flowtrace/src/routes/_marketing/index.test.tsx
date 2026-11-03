import { afterEach, describe, expect, it } from 'vitest'
import { cleanup, screen } from '@testing-library/react'
import { LANDING_DESCRIPTION, LANDING_TITLE, LandingPage, Route } from './index'
import { renderWithRouter } from '@/test/render-with-router'
import { SITE_URL } from '@/lib/site'

afterEach(cleanup)

describe('landing page', () => {
  it('renders its headings and the demo CTA', async () => {
    await renderWithRouter(() => <LandingPage />)
    expect(screen.getByRole('heading', { level: 1 }).textContent).toContain(
      'traced to its proof',
    )
    for (const name of [
      'One borehole, followed from grant to drill.',
      'Each audience sees its own depth.',
      'Funders follow the grant past your door.',
      "Open a quarter of an NGO's books, proof included.",
    ]) {
      expect(screen.getByRole('heading', { level: 2, name })).toBeTruthy()
    }
    expect(
      screen
        .getAllByRole('link', { name: /Explore the demo/ })[0]
        .getAttribute('href'),
    ).toBe('/dashboard')
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
