import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { Organization } from '@/lib/types'

const updateOrganization = vi.hoisted(() =>
  vi.fn((id: string, u: { is_public?: boolean }) =>
    Promise.resolve({ id, is_public: u.is_public } as Organization),
  ),
)
vi.mock('@/lib/data', () => ({ updateOrganization }))

const { PublicPagesSwitch } = await import('./public-pages-switch')

const org = (isPublic: boolean): Organization => ({
  id: 'rw1',
  name: 'RW1',
  type: 'NGO',
  country: 'KE',
  currency: 'KES',
  trust_score: 0,
  verified: false,
  public_slug: 'rw1',
  logo_url: null,
  is_public: isPublic,
  created_at: 'x',
  updated_at: 'x',
})

function show(o: Organization, onSaved = vi.fn()) {
  const client = new QueryClient()
  render(
    <QueryClientProvider client={client}>
      <PublicPagesSwitch org={o} onSaved={onSaved} />
    </QueryClientProvider>,
  )
  return onSaved
}

afterEach(() => {
  cleanup()
  updateOrganization.mockClear()
})

describe('Show on public pages', () => {
  it('shows the profile’s is_public', () => {
    show(org(false))
    expect(
      screen.getByRole('switch', { name: /show on public pages/i }).getAttribute('aria-checked'),
    ).toBe('false')
    expect(screen.getByText(/hidden/i)).toBeTruthy()
  })

  it('sends is_public alone through the profile update', async () => {
    const onSaved = show(org(false))
    fireEvent.click(screen.getByRole('switch', { name: /show on public pages/i }))
    await waitFor(() => expect(onSaved).toHaveBeenCalled())
    expect(updateOrganization).toHaveBeenCalledWith('rw1', { is_public: true })
  })

  it('turns publishing off again', async () => {
    show(org(true))
    fireEvent.click(screen.getByRole('switch', { name: /show on public pages/i }))
    await waitFor(() =>
      expect(updateOrganization).toHaveBeenCalledWith('rw1', { is_public: false }),
    )
  })
})
