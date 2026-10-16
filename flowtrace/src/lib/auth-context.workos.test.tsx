import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'

// AuthKit, signed in, with a token getter that refreshes on expiry (useAccessToken).
vi.mock('@workos/authkit-tanstack-react-start/client', () => ({
  AuthKitProvider: ({ children }: { children: ReactNode }) => children,
  useAuth: () => ({
    user: {
      id: 'user_01ABC',
      email: 'ama@ngo.example',
      firstName: 'Ama',
      lastName: 'Owusu',
    },
    organizationId: 'org_01XYZ',
    loading: false,
    signOut: () => Promise.resolve(),
  }),
  useAccessToken: () => ({
    getAccessToken: () => Promise.resolve('workos-access-token'),
  }),
}))

afterEach(() => {
  cleanup()
  vi.unstubAllEnvs()
  vi.resetModules()
})

async function workosMode() {
  vi.stubEnv('VITE_AUTH_MODE', 'workos')
  vi.resetModules()
  const { AppAuthProvider, useAppAuth } = await import('./auth-context')
  const tokens = await import('./ledger/spacetime-auth')
  function WhoAmI() {
    const { user } = useAppAuth()
    return <p>{user?.email ?? 'signed out'}</p>
  }
  return { AppAuthProvider, WhoAmI, tokens }
}

describe('workos auth mode', () => {
  it('hands AuthKit’s access token to the SpacetimeDB connection', async () => {
    const { AppAuthProvider, WhoAmI, tokens } = await workosMode()
    render(
      <AppAuthProvider>
        <WhoAmI />
      </AppAuthProvider>,
    )
    expect(screen.getByText('ama@ngo.example')).toBeTruthy()
    expect(await tokens.workosAccessToken(100)).toBe('workos-access-token')
  })

  it('takes the token source away when the provider unmounts', async () => {
    const { AppAuthProvider, WhoAmI, tokens } = await workosMode()
    const view = render(
      <AppAuthProvider>
        <WhoAmI />
      </AppAuthProvider>,
    )
    view.unmount()
    expect(await tokens.workosAccessToken(10)).toBeUndefined()
  })
})
