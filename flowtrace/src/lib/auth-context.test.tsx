import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { AppAuthProvider, useAppAuth } from './auth-context'
import { AUTH_MODE, DEV_MODE } from './auth'

function WhoAmI() {
  const { user, loading } = useAppAuth()
  return <p>{loading ? 'loading' : (user?.email ?? 'signed out')}</p>
}

// VITE_AUTH_MODE is unset in tests, which is the default for builds too: the
// app must run signed in as the dummy user without any WorkOS config.
describe('dummy auth mode', () => {
  it('is the default when VITE_AUTH_MODE is unset', () => {
    expect(AUTH_MODE).toBe('dummy')
    expect(DEV_MODE).toBe(true)
  })

  it('provides the dummy user without mounting AuthKit', () => {
    render(
      <AppAuthProvider>
        <WhoAmI />
      </AppAuthProvider>,
    )
    expect(screen.getByText('demo@flowtrace.dev')).toBeTruthy()
  })
})
