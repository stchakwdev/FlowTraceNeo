import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  SignInRequiredError,
  connectionToken,
  refusalMessage,
  setAccessTokenSource,
  workosAccessToken,
} from './spacetime-auth'

const device = () => 'device-token'
const workos = () => Promise.resolve('workos-token')

describe('connectionToken', () => {
  it('uses the saved device token in dummy mode and keeps the token the host returns', async () => {
    expect(await connectionToken('dummy', { device, workos })).toEqual({
      token: 'device-token',
      saveReturned: true,
    })
  })

  it('connects without a token in dummy mode when none is saved, so the host mints a device identity', async () => {
    expect(
      await connectionToken('dummy', { device: () => undefined, workos }),
    ).toEqual({ token: undefined, saveReturned: true })
  })

  it('never asks WorkOS in dummy mode', async () => {
    const ask = vi.fn(workos)
    await connectionToken('dummy', { device, workos: ask })
    expect(ask).not.toHaveBeenCalled()
  })

  it('uses the WorkOS access token in workos mode and never keeps what the host returns', async () => {
    expect(await connectionToken('workos', { device, workos })).toEqual({
      token: 'workos-token',
      saveReturned: false,
    })
  })

  it('never falls back to the device token in workos mode', async () => {
    await expect(
      connectionToken('workos', {
        device,
        workos: () => Promise.resolve(undefined),
      }),
    ).rejects.toBeInstanceOf(SignInRequiredError)
  })

  it('asks WorkOS again on every connection, so a reconnect gets a refreshed token', async () => {
    const tokens = ['first', 'refreshed']
    const ask = vi.fn(() => Promise.resolve(tokens.shift()))
    const a = await connectionToken('workos', { device, workos: ask })
    const b = await connectionToken('workos', { device, workos: ask })
    expect([a.token, b.token]).toEqual(['first', 'refreshed'])
  })
})

describe('workosAccessToken', () => {
  afterEach(() => setAccessTokenSource(null))

  it('reads the token from the source the auth bridge registered', async () => {
    setAccessTokenSource(() => Promise.resolve('from-authkit'))
    expect(await workosAccessToken()).toBe('from-authkit')
  })

  it('waits for the auth bridge when a connection starts before it registers', async () => {
    const pending = workosAccessToken(1_000)
    setAccessTokenSource(() => Promise.resolve('late'))
    expect(await pending).toBe('late')
  })

  it('gives up when nothing registers, which the connection reports as sign-in required', async () => {
    expect(await workosAccessToken(10)).toBeUndefined()
  })

  it('stops using a source once it is cleared on sign-out', async () => {
    setAccessTokenSource(() => Promise.resolve('old'))
    setAccessTokenSource(null)
    expect(await workosAccessToken(10)).toBeUndefined()
  })
})

const jwt = (payload: object) =>
  `e30.${btoa(JSON.stringify(payload)).replace(/=+$/, '').replace(/\+/g, '-').replace(/\//g, '_')}.sig`

describe('refusalMessage', () => {
  const iss = 'https://api.workos.com/user_management/client_01TEST'

  it('names the issuer and the client id of a WorkOS token, which an operator must trust', () => {
    expect(
      refusalMessage(
        'workos',
        jwt({ iss, sub: 'user_1', client_id: 'client_01TEST' }),
      ),
    ).toBe(
      `SpacetimeDB refused this sign-in: an operator must trust issuer ${iss} for audience client_01TEST on this database`,
    )
  })

  it('names the aud claim when the token has one', () => {
    expect(
      refusalMessage(
        'workos',
        jwt({ iss, sub: 'user_1', aud: ['flowtrace'], client_id: 'x' }),
      ),
    ).toBe(
      `SpacetimeDB refused this sign-in: an operator must trust issuer ${iss} for audience flowtrace on this database`,
    )
  })

  it('says device identities are off in dummy mode', () => {
    expect(refusalMessage('dummy', undefined)).toBe(
      'SpacetimeDB refused this device’s identity: device identities are turned off on this database',
    )
  })

  it('falls back to a plain refusal when the token cannot be read', () => {
    expect(refusalMessage('workos', 'not-a-jwt')).toBe(
      'SpacetimeDB refused the connection',
    )
  })
})
