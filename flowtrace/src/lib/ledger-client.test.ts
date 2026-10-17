import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  ledgerBackendKind,
  ledgerOrgId,
  mastheadOrgName,
} from './ledger-client'

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

  it('keeps the signed-in organisation for the demo books', () => {
    expect(ledgerOrgId({ ...base, kind: 'demo' })).toBe('demo-org')
  })

  it('on SpacetimeDB uses the configured organisation, else the identity’s first', () => {
    const spacetime = { ...base, kind: 'spacetimedb' as const }
    expect(
      ledgerOrgId({ ...spacetime, configured: 'web1-x', memberOrgs: ['a'] }),
    ).toBe('web1-x')
    expect(ledgerOrgId({ ...spacetime, memberOrgs: ['org-b', 'org-a'] })).toBe(
      'org-a',
    )
    expect(ledgerOrgId(spacetime)).toBeNull()
  })

  it('with WorkOS on SpacetimeDB opens the books the person is a member of, not the WorkOS organisation', () => {
    expect(
      ledgerOrgId({
        kind: 'spacetimedb',
        contextOrgId: 'org_01WORKOS',
        configured: undefined,
        memberOrgs: ['web1-20261016'],
      }),
    ).toBe('web1-20261016')
  })

  it('prefers the signed-in organisation when the person is a member of it', () => {
    expect(
      ledgerOrgId({
        kind: 'spacetimedb',
        contextOrgId: 'org-z',
        configured: undefined,
        memberOrgs: ['org-a', 'org-z'],
      }),
    ).toBe('org-z')
  })
})

describe('mastheadOrgName', () => {
  it('names the books’ organisation id when WorkOS users read them from SpacetimeDB', () => {
    expect(
      mastheadOrgName({
        kind: 'spacetimedb',
        devMode: false,
        ledgerOrgId: 'web1-20261016',
        ledgerOrgName: null,
        organizationName: 'Demo NGO Kenya',
      }),
    ).toBe('web1-20261016')
  })

  it('names the organisation by its profile name once the module holds one (MOD1)', () => {
    expect(
      mastheadOrgName({
        kind: 'spacetimedb',
        devMode: false,
        ledgerOrgId: 'web1-20261016',
        ledgerOrgName: 'Web One NGO',
        organizationName: 'Demo NGO Kenya',
      }),
    ).toBe('Web One NGO')
  })

  it('says “Your organisation” while a WorkOS user belongs to no books yet', () => {
    expect(
      mastheadOrgName({
        kind: 'spacetimedb',
        devMode: false,
        ledgerOrgId: null,
        ledgerOrgName: null,
        organizationName: 'Demo NGO Kenya',
      }),
    ).toBe('Your organisation')
  })

  it('keeps the organisation’s name in dummy mode and on the demo books', () => {
    const named = {
      ledgerOrgId: 'web1-x',
      ledgerOrgName: 'Web One',
      organizationName: 'Demo NGO Kenya',
    }
    expect([
      mastheadOrgName({ ...named, kind: 'spacetimedb', devMode: true }),
      mastheadOrgName({ ...named, kind: 'demo', devMode: true }),
      mastheadOrgName({ ...named, kind: 'none', devMode: false }),
    ]).toEqual(['Demo NGO Kenya', 'Demo NGO Kenya', 'Demo NGO Kenya'])
  })

  it('falls back to “Your organisation” when there is no name', () => {
    expect(
      mastheadOrgName({
        kind: 'demo',
        devMode: true,
        ledgerOrgId: null,
        ledgerOrgName: null,
        organizationName: undefined,
      }),
    ).toBe('Your organisation')
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

  it('returns the SpacetimeDB backend in workos auth mode when it is selected', async () => {
    vi.stubEnv('VITE_AUTH_MODE', 'workos')
    vi.stubEnv('VITE_LEDGER_BACKEND', 'spacetimedb')
    vi.resetModules()
    const { getLedgerBackend } = await import('./ledger-client')
    expect((await getLedgerBackend())?.kind).toBe('spacetimedb')
  })

  it('has no books in workos auth mode unless a backend is selected', async () => {
    vi.stubEnv('VITE_AUTH_MODE', 'workos')
    vi.stubEnv('VITE_LEDGER_BACKEND', '')
    vi.resetModules()
    const { getLedgerBackend } = await import('./ledger-client')
    expect(await getLedgerBackend()).toBeNull()
  })

  it('returns the demo backend in dummy auth mode by default', async () => {
    vi.stubEnv('VITE_AUTH_MODE', 'dummy')
    vi.stubEnv('VITE_LEDGER_BACKEND', '')
    vi.resetModules()
    const { getLedgerBackend } = await import('./ledger-client')
    expect((await getLedgerBackend())?.kind).toBe('demo')
  })
})
