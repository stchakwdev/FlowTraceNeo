import { afterEach, describe, expect, it, vi } from 'vitest'
import type * as PublicSql from './ledger/public-sql'

// The module implementation, replaced so the test sees which one data.ts calls.
const moduleImpl = vi.hoisted(() => ({
  getCampaigns: vi.fn(() => Promise.resolve([{ id: 'from-module' }])),
}))
vi.mock('./ledger/spacetime-transparency', () => moduleImpl)

const publicImpl = vi.hoisted(() => ({
  getPublicCampaignBySlug: vi.fn(() =>
    Promise.resolve({ id: 'from-public-sql' }),
  ),
}))
vi.mock('./ledger/public-sql', async (original) => ({
  ...(await original<typeof PublicSql>()),
  ...publicImpl,
}))

async function load(env: { auth: 'dummy' | 'workos'; backend?: string }) {
  vi.stubEnv('VITE_AUTH_MODE', env.auth)
  vi.stubEnv('VITE_LEDGER_BACKEND', env.backend ?? '')
  vi.resetModules()
  return import('./data')
}

afterEach(() => {
  vi.unstubAllEnvs()
  vi.clearAllMocks()
})

describe('who serves campaigns, donations and the organisation profile', () => {
  it('is the module whenever VITE_LEDGER_BACKEND=spacetimedb, in either auth mode', async () => {
    for (const auth of ['dummy', 'workos'] as const) {
      const data = await load({ auth, backend: 'spacetimedb' })
      expect(await data.getCampaigns('rw1')).toEqual([{ id: 'from-module' }])
      expect(await data.getCampaignBySlug('clean-water')).toEqual({
        id: 'from-public-sql',
      })
    }
    expect(moduleImpl.getCampaigns).toHaveBeenCalledWith('rw1')
  })

  it('is the demo dataset for the demo books', async () => {
    const data = await load({ auth: 'dummy' })
    const campaigns = await data.getCampaigns(
      '550e8400-e29b-41d4-a716-446655440000',
    )
    expect(campaigns.length).toBeGreaterThan(0)
    expect(moduleImpl.getCampaigns).not.toHaveBeenCalled()
    expect(publicImpl.getPublicCampaignBySlug).not.toHaveBeenCalled()
  })

  it('is nothing with no backend: Supabase is gone, so the call says the books are not connected', async () => {
    const data = await load({ auth: 'workos' })
    expect(data.transparencySource('none')).toBe('none')
    await expect(data.getCampaigns('rw1')).rejects.toThrow(
      'The books are not connected for this organisation yet.',
    )
    await expect(data.getCampaignBySlug('x')).rejects.toThrow(
      'The books are not connected',
    )
  })
})
