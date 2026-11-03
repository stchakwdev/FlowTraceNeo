import { describe, expect, it } from 'vitest'
import { DEMO_AS_OF } from './demo/ledger'
import {
  DEMO_FIGURES_AS_OF,
  FOOTER_COLUMNS,
  MASTHEAD_NAV,
  SIGN_IN_PATH,
} from './marketing-nav'

// The paths later LP threads build pages for. Several are not routes yet;
// the masthead and footer link to them as plain anchors until they land.
describe('marketing navigation', () => {
  it('lists the masthead paths', () => {
    expect(MASTHEAD_NAV.map((l) => l.href)).toEqual([
      '/#product',
      '/books',
      '/funders',
      '/pricing',
      '/security',
      '/docs',
      '/blog',
      '/changelog',
    ])
  })

  it('lists the footer columns and their paths', () => {
    expect(
      Object.fromEntries(
        FOOTER_COLUMNS.map((c) => [c.title, c.links.map((l) => l.href)]),
      ),
    ).toEqual({
      Product: ['/#product', '/books', '/funders', '/pricing'],
      Resources: [
        '/docs',
        '/blog',
        '/changelog',
        'https://github.com/stchakwdev/FlowTraceNeo',
      ],
      Company: ['/about', '/about#contact', '/security'],
      Legal: ['/privacy', '/terms'],
    })
  })

  it('sends Sign in to /dashboard until a /sign-in route exists', () => {
    expect(SIGN_IN_PATH).toBe('/dashboard')
  })

  it('dates demo figures as the demo dataset does', () => {
    expect(DEMO_FIGURES_AS_OF).toBe(DEMO_AS_OF)
  })
})
