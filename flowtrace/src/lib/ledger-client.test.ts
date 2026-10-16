import { afterEach, describe, expect, it, vi } from 'vitest'
import { ledgerBackendKind, ledgerOrgId } from './ledger-client'

describe('ledgerBackendKind', () => {
  it('uses SpacetimeDB when asked, whatever the auth mode', () => {
    expect(ledgerBackendKind('spacetimedb', true)).toBe('spacetimedb')
    expect(ledgerBackendKind('spacetimedb', false)).toBe('spacetimedb')
  })

  it('falls back to the demo books in dummy auth mode only', () => {
    expect(ledgerBackendKind(undefined, true)).toBe('demo')
    expect(ledgerBackendKind('demo', false)).toBe('demo')
    expect(ledgerBackendKind(undefined, false)).toBe('none')
  })
})

describe('ledgerOrgId', () => {
  const base = {
    contextOrgId: 'demo-org',
    configured: undefined,
    memberOrgs: [],
  }

  it('keeps the signed-in organisation for the demo books and real auth', () => {
    expect(ledgerOrgId({ ...base, kind: 'demo', devMode: true })).toBe(
      'demo-org',
    )
    expect(
      ledgerOrgId({
        ...base,
        kind: 'spacetimedb',
        devMode: false,
        memberOrgs: ['b'],
      }),
    ).toBe('demo-org')
  })

  it('in dummy auth mode uses the configured organisation, else this device’s first', () => {
    const spacetime = { ...base, kind: 'spacetimedb' as const, devMode: true }
    expect(
      ledgerOrgId({ ...spacetime, configured: 'web1-x', memberOrgs: ['a'] }),
    ).toBe('web1-x')
    expect(ledgerOrgId({ ...spacetime, memberOrgs: ['org-b', 'org-a'] })).toBe(
      'org-a',
    )
    expect(ledgerOrgId(spacetime)).toBeNull()
  })
})

describe('getLedgerBackend', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
    vi.resetModules()
  })

  it('returns the SpacetimeDB backend in dummy auth mode when it is selected', async () => {
    vi.stubEnv('VITE_AUTH_MODE', 'dummy')
    vi.stubEnv('VITE_LEDGER_BACKEND', 'spacetimedb')
    vi.resetModules()
    const { getLedgerBackend } = await import('./ledger-client')
    expect((await getLedgerBackend())?.kind).toBe('spacetimedb')
  })

  it('returns the demo backend in dummy auth mode by default', async () => {
    vi.stubEnv('VITE_AUTH_MODE', 'dummy')
    vi.stubEnv('VITE_LEDGER_BACKEND', '')
    vi.resetModules()
    const { getLedgerBackend } = await import('./ledger-client')
    expect((await getLedgerBackend())?.kind).toBe('demo')
  })
})
