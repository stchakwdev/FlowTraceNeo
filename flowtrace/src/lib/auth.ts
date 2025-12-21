// WorkOS client ID from environment
export const workosClientId = import.meta.env.VITE_WORKOS_CLIENT_ID || ''
export const isWorkosConfigured = !!workosClientId

// Auth redirect paths
export const AUTH_ROUTES = {
  signIn: '/sign-in',
  signUp: '/sign-up',
  callback: '/api/auth/callback',
  afterAuth: '/dashboard',
  afterSignOut: '/',
} as const

// User type from WorkOS
export interface AuthUser {
  id: string
  email: string
  firstName?: string
  lastName?: string
  profilePictureUrl?: string
  organizationId?: string
}

// Auth state type
export interface AuthState {
  user: AuthUser | null
  isLoading: boolean
  isAuthenticated: boolean
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
