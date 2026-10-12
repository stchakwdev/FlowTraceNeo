import { describe, expect, it } from 'vitest'
import { getUserDisplayName, getUserInitials, mapWorkOSUser } from './auth'
import type { User } from '@workos/authkit-tanstack-react-start'

const workosUser = {
  id: 'user_01',
  email: 'amina@example.org',
  firstName: 'Amina',
  lastName: null,
  profilePictureUrl: null,
} as unknown as User

describe('mapWorkOSUser', () => {
  it('returns null without a user', () => {
    expect(mapWorkOSUser(null, 'org_01')).toBeNull()
  })

  it('maps nulls to undefined and takes organizationId from the session', () => {
    expect(mapWorkOSUser(workosUser, 'org_01')).toEqual({
      id: 'user_01',
      email: 'amina@example.org',
      firstName: 'Amina',
      lastName: undefined,
      profilePictureUrl: undefined,
      organizationId: 'org_01',
    })
  })

  it('leaves organizationId undefined when the session has none', () => {
    expect(mapWorkOSUser(workosUser)?.organizationId).toBeUndefined()
  })
})

describe('getUserDisplayName', () => {
  it('prefers full name, then first name, then email local part', () => {
    expect(
      getUserDisplayName({ id: '1', email: 'a@x.org', firstName: 'Amina', lastName: 'Otieno' }),
    ).toBe('Amina Otieno')
    expect(getUserDisplayName({ id: '1', email: 'a@x.org', firstName: 'Amina' })).toBe('Amina')
    expect(getUserDisplayName({ id: '1', email: 'amina@x.org' })).toBe('amina')
    expect(getUserDisplayName(null)).toBe('Guest')
  })
})

describe('getUserInitials', () => {
  it('builds uppercase initials with fallbacks', () => {
    expect(
      getUserInitials({ id: '1', email: 'a@x.org', firstName: 'amina', lastName: 'otieno' }),
    ).toBe('AO')
    expect(getUserInitials({ id: '1', email: 'zed@x.org' })).toBe('Z')
    expect(getUserInitials(null)).toBe('G')
  })
})
