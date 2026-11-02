import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, screen, within } from '@testing-library/react'
import { MarketingFrame } from './_marketing'
import { renderWithRouter } from '@/test/render-with-router'
import { FOOTER_COLUMNS, MASTHEAD_NAV } from '@/lib/marketing-nav'

afterEach(() => {
  cleanup()
  delete window.umami
})

const frame = (demoFigures = false) =>
  renderWithRouter(() => (
    <MarketingFrame signedIn={false} demoFigures={demoFigures}>
      <h1>Page body</h1>
    </MarketingFrame>
  ))

describe('marketing layout', () => {
  it('starts with a skip link to the main content', async () => {
    await frame()
    const skip = screen.getByRole('link', { name: 'Skip to content' })
    expect(skip.getAttribute('href')).toBe('#main')
    expect(document.getElementById('main')?.tagName).toBe('MAIN')
    expect(screen.getByRole('main').textContent).toContain('Page body')
  })

  it('links every masthead path', async () => {
    await frame()
    const nav = screen.getByRole('navigation', { name: 'Main' })
    for (const item of MASTHEAD_NAV) {
      expect(
        within(nav)
          .getByRole('link', { name: item.label })
          .getAttribute('href'),
      ).toBe(item.href)
    }
    expect(
      screen.getAllByRole('link', { name: 'Sign in' })[0].getAttribute('href'),
    ).toBe('/dashboard')
  })

  it('renders the four footer columns', async () => {
    await frame()
    const site = screen.getByRole('navigation', { name: 'Site' })
    for (const column of FOOTER_COLUMNS) {
      expect(
        within(site).getByRole('heading', { name: column.title }),
      ).toBeTruthy()
      for (const link of column.links) {
        expect(
          within(site)
            .getByRole('link', { name: link.label })
            .getAttribute('href'),
        ).toBe(link.href)
      }
    }
    expect(screen.getByText(/© \d{4} FlowTrace/)).toBeTruthy()
  })

  it('dates demo figures only when the page shows them', async () => {
    await frame(false)
    expect(screen.queryByText(/Demo figures/)).toBeNull()
    cleanup()
    await frame(true)
    expect(screen.getByText(/Demo figures/).textContent).toContain('as of')
  })

  it('tracks the masthead and footer demo CTAs', async () => {
    const spy = vi.fn()
    window.umami = { track: spy }
    await frame()
    const ctas = screen.getAllByRole('link', { name: /Explore the demo/ })
    for (const cta of ctas) fireEvent.click(cta)
    const locations = spy.mock.calls
      .filter(([event]) => event === 'cta_click')
      .map(([, props]) => (props as { location: string }).location)
    expect(locations).toEqual(expect.arrayContaining(['masthead', 'footer']))
    expect(spy).toHaveBeenCalledWith('demo_started', { location: 'masthead' })
  })

  it('opens and closes the mobile drawer', async () => {
    await frame()
    const drawer = document.getElementById('marketing-drawer')!
    expect(drawer.hasAttribute('inert')).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'Open menu' }))
    expect(drawer.hasAttribute('inert')).toBe(false)
    expect(
      within(drawer)
        .getAllByRole('link')
        .map((a) => a.getAttribute('href')),
    ).toEqual(expect.arrayContaining(MASTHEAD_NAV.map((l) => l.href)))
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(drawer.hasAttribute('inert')).toBe(true)
  })
})
