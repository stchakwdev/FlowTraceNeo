import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

// The generated bindings, replaced by a builder that records the token it was given and
// "connects" on build, the host handing a token back as SpacetimeDB does on connect.
const fake = vi.hoisted(() => {
  interface Built {
    token: string | undefined
    onDisconnect: () => void
  }
  const built: Array<Built> = []
  // When set, the host refuses: the SDK closes the socket without ever calling onConnect.
  const state = { refuse: false }
  // The request ids run_command was called with; each call is answered with a result row.
  const requestIds: Array<string> = []
  const results: Array<{ requestId: string; resultJson: string }> = []
  const view = (rows: Array<unknown>) => ({
    iter: () => rows[Symbol.iterator](),
    onInsert: () => {},
    onUpdate: () => {},
    removeOnInsert: () => {},
    removeOnUpdate: () => {},
  })
  const conn = (identity: unknown) => ({
    identity,
    db: new Proxy({} as Record<string, unknown>, {
      get: (_t, name) => view(name === 'myCommandResult' ? results : []),
    }),
    subscriptionBuilder: () => {
      const sb = {
        onApplied: (cb: () => void) => {
          queueMicrotask(cb)
          return sb
        },
        onError: () => sb,
        subscribe: () => sb,
      }
      return sb
    },
    reducers: {
      runCommand: ({ requestId }: { requestId: string }) => {
        requestIds.push(requestId)
        results.push({
          requestId,
          resultJson: JSON.stringify({ type: 'none' }),
        })
        return Promise.resolve()
      },
    },
  })
  const builder = () => {
    let token: string | undefined
    let onConnect: (
      conn: unknown,
      identity: unknown,
      token: string,
    ) => void = () => {}
    let onDisconnect: () => void = () => {}
    const b = {
      withUri: () => b,
      withDatabaseName: () => b,
      withToken: (t: string | undefined) => {
        token = t
        return b
      },
      onConnect: (cb: typeof onConnect) => {
        onConnect = cb
        return b
      },
      onConnectError: () => b,
      onDisconnect: (cb: () => void) => {
        onDisconnect = cb
        return b
      },
      build: () => {
        built.push({ token, onDisconnect: () => onDisconnect() })
        const identity = { toHexString: () => 'c200identity' }
        if (state.refuse) {
          queueMicrotask(() => onDisconnect())
          return
        }
        queueMicrotask(() =>
          onConnect(conn(identity), identity, token ?? 'host-minted-token'),
        )
      },
    }
    return b
  }
  return { built, builder, state, requestIds, results }
})

vi.mock('./spacetime-bindings', () => ({
  DbConnection: { builder: fake.builder },
}))

const TOKEN_KEY = 'flowtrace.spacetimedb.token'

async function load(mode: 'dummy' | 'workos') {
  vi.stubEnv('VITE_AUTH_MODE', mode)
  vi.stubEnv('VITE_SPACETIMEDB_URI', 'ws://127.0.0.1:3100')
  vi.stubEnv('VITE_SPACETIMEDB_MODULE', 'ft-test')
  vi.resetModules()
  const auth = await import('./spacetime-auth')
  const backend = await import('./spacetime-backend')
  return { auth, backend }
}

// An in-memory localStorage: the test runtime's may be missing, and this one lists what was
// written.
const stored = new Map<string, string>()
const memoryStorage = {
  getItem: (k: string) => stored.get(k) ?? null,
  setItem: (k: string, v: string) => void stored.set(k, String(v)),
  removeItem: (k: string) => void stored.delete(k),
  clear: () => stored.clear(),
}
const storedValues = () => [...stored.values()]

beforeEach(() => {
  fake.built.length = 0
  fake.state.refuse = false
  fake.requestIds.length = 0
  fake.results.length = 0
  stored.clear()
  vi.stubGlobal('localStorage', memoryStorage)
})

afterEach(() => {
  vi.unstubAllEnvs()
  vi.unstubAllGlobals()
  vi.resetModules()
})

describe('the SpacetimeDB connection in workos auth mode', () => {
  it('connects with the WorkOS access token, not the saved device token', async () => {
    stored.set(TOKEN_KEY, 'old-device-token')
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve('workos-access-token'))
    expect(await backend.spacetimeIdentity()).toBe('c200identity')
    expect(fake.built.map((b) => b.token)).toEqual(['workos-access-token'])
  })

  it('never writes the WorkOS token to localStorage and leaves the device token alone', async () => {
    stored.set(TOKEN_KEY, 'old-device-token')
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve('workos-access-token'))
    await backend.spacetimeIdentity()
    expect(storedValues()).toEqual(['old-device-token'])
  })

  it('asks AuthKit for a fresh token when it reconnects', async () => {
    const { auth, backend } = await load('workos')
    const tokens = ['first-token', 'refreshed-token']
    auth.setAccessTokenSource(() => Promise.resolve(tokens.shift()))
    await backend.spacetimeIdentity()
    fake.built[0].onDisconnect()
    await backend.spacetimeIdentity()
    expect(fake.built.map((b) => b.token)).toEqual([
      'first-token',
      'refreshed-token',
    ])
  })

  it('does not connect at all when nobody is signed in', async () => {
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve(undefined))
    await expect(backend.spacetimeIdentity()).rejects.toBeInstanceOf(
      auth.SignInRequiredError,
    )
    expect(fake.built).toEqual([])
  })

  it('tries again after a refusal instead of keeping the failed connection', async () => {
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve(undefined))
    await expect(backend.spacetimeIdentity()).rejects.toThrow(
      'Sign in to open the books.',
    )
    auth.setAccessTokenSource(() => Promise.resolve('workos-access-token'))
    expect(await backend.spacetimeIdentity()).toBe('c200identity')
  })
})

describe('a connection the module refuses', () => {
  const token = `e30.${btoa(JSON.stringify({ iss: 'https://idp.example', sub: 'u', client_id: 'c1' }))}.sig`

  it('fails with the issuer and audience to trust instead of waiting forever', async () => {
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve(token))
    fake.state.refuse = true
    await expect(backend.spacetimeIdentity()).rejects.toThrow(
      'SpacetimeDB refused this sign-in: an operator must trust issuer https://idp.example for audience c1 on this database',
    )
  })

  it('connects on the next try once the operator trusts the issuer', async () => {
    const { auth, backend } = await load('workos')
    auth.setAccessTokenSource(() => Promise.resolve(token))
    fake.state.refuse = true
    await expect(backend.spacetimeIdentity()).rejects.toThrow()
    fake.state.refuse = false
    expect(await backend.spacetimeIdentity()).toBe('c200identity')
  })
})

describe('the SpacetimeDB connection in dummy auth mode', () => {
  it('presents the saved device token', async () => {
    stored.set(TOKEN_KEY, 'old-device-token')
    const { backend } = await load('dummy')
    await backend.spacetimeIdentity()
    expect(fake.built.map((b) => b.token)).toEqual(['old-device-token'])
  })

  it('saves the token the host mints for a new device identity', async () => {
    const { backend } = await load('dummy')
    await backend.spacetimeIdentity()
    expect(fake.built.map((b) => b.token)).toEqual([undefined])
    expect(stored.get(TOKEN_KEY)).toBe('host-minted-token')
  })

  it('ignores a registered WorkOS source', async () => {
    const { auth, backend } = await load('dummy')
    auth.setAccessTokenSource(() => Promise.resolve('workos-access-token'))
    await backend.spacetimeIdentity()
    expect(fake.built.map((b) => b.token)).toEqual([undefined])
  })
})

describe('organisationNames', () => {
  it('reads each organisation’s name from its profile row', async () => {
    const { backend } = await load('workos')
    expect(
      backend.organisationNames([
        {
          orgId: 'web1',
          data: JSON.stringify({ id: 'web1', name: 'Web One NGO' }),
        },
        { orgId: 'org-b', data: JSON.stringify({ id: 'org-b', name: ' ' }) },
        { orgId: 'org-c', data: 'not json' },
      ]),
    ).toEqual(new Map([['web1', 'Web One NGO']]))
  })
})

describe('a command on plain HTTP, where crypto.randomUUID is missing', () => {
  it('is sent with a UUID v4 request id and its result is read back', async () => {
    const real = globalThis.crypto
    vi.stubGlobal('crypto', {
      getRandomValues: real.getRandomValues.bind(real),
    })
    const { backend } = await load('dummy')
    // SpacetimeDB ignores the actor and takes it from the caller's identity.
    const actor = {
      id: 'bob',
      name: 'Bob',
      can_post: true,
      can_approve: true,
      can_close: false,
    }
    const result = await backend.spacetimeLedgerBackend.run(
      'org-a',
      { type: 'claim.approve', claimId: 'claim-1' },
      actor,
    )
    expect(result).toEqual({ type: 'none' })
    expect(fake.requestIds).toHaveLength(1)
    expect(fake.requestIds[0]).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    )
  })
})
