import type { User } from '@workos/authkit-tanstack-react-start'

// Auth provider. Defaults to a dummy signed-in user so the app builds and runs
// without WorkOS; set VITE_AUTH_MODE=workos to turn real auth on.
export const AUTH_MODE: 'dummy' | 'workos' =
  import.meta.env.VITE_AUTH_MODE === 'workos' ? 'workos' : 'dummy'
export const DEV_MODE = AUTH_MODE === 'dummy'

// Auth redirect paths
export const AUTH_ROUTES = {
  signIn: '/sign-in',
  signUp: '/sign-up',
  callback: '/api/auth/callback',
  afterAuth: '/dashboard',
  afterSignOut: '/',
} as const

// User type for FlowTrace (mapped from WorkOS)
export interface AuthUser {
  id: string
  email: string
  firstName?: string
  lastName?: string
  profilePictureUrl?: string
  organizationId?: string
}

// Signed-in user in dummy auth mode
export const DUMMY_USER: AuthUser = {
  id: 'user_dummy',
  email: 'demo@flowtrace.dev',
  firstName: 'Demo',
  lastName: 'User',
}

// Auth state type
export interface AuthState {
  user: AuthUser | null
  isLoading: boolean
  isAuthenticated: boolean
}

// Map WorkOS User to our AuthUser interface
// organizationId comes from the WorkOS session, not the User object
export function mapWorkOSUser(
  user: User | null,
  organizationId?: string | null,
): AuthUser | null {
  if (!user) return null
  return {
    id: user.id,
    email: user.email,
    firstName: user.firstName ?? undefined,
    lastName: user.lastName ?? undefined,
    profilePictureUrl: user.profilePictureUrl ?? undefined,
    organizationId: organizationId ?? undefined,
  }
}

// Helper to get user display name
export function getUserDisplayName(user: AuthUser | null): string {
  if (!user) return 'Guest'
  if (user.firstName && user.lastName) {
    return `${user.firstName} ${user.lastName}`
  }
  if (user.firstName) return user.firstName
  return user.email.split('@')[0]
}

// Helper to get user initials for avatars
export function getUserInitials(user: AuthUser | null): string {
  if (!user) return 'G'
  if (user.firstName && user.lastName) {
    return `${user.firstName[0]}${user.lastName[0]}`.toUpperCase()
  }
  if (user.firstName) return user.firstName[0].toUpperCase()
  return user.email[0].toUpperCase()
}
