import { createContext, useContext, useMemo } from 'react'
import {
  AuthKitProvider,
  useAuth,
} from '@workos/authkit-tanstack-react-start/client'
import { DEV_MODE, DUMMY_USER, mapWorkOSUser } from './auth'
import type { ComponentProps, ReactNode } from 'react'
import type { AuthUser } from './auth'

export type InitialAuth = ComponentProps<typeof AuthKitProvider>['initialAuth']

export interface AppAuth {
  user: AuthUser | null
  organizationId: string | null
  loading: boolean
  signOut: () => Promise<void>
}

const DUMMY_AUTH: AppAuth = {
  user: DUMMY_USER,
  organizationId: null,
  loading: false,
  signOut: () => {
    window.location.href = '/'
    return Promise.resolve()
  },
}

const AppAuthContext = createContext<AppAuth>(DUMMY_AUTH)

// Provides auth to the app without components depending on WorkOS directly.
// In dummy mode AuthKit is never mounted, so no WorkOS config is needed.
export function AppAuthProvider({
  initialAuth,
  children,
}: {
  initialAuth?: InitialAuth
  children: ReactNode
}) {
  if (DEV_MODE) {
    return (
      <AppAuthContext.Provider value={DUMMY_AUTH}>
        {children}
      </AppAuthContext.Provider>
    )
  }

  return (
    <AuthKitProvider initialAuth={initialAuth}>
      <WorkOSAuthBridge>{children}</WorkOSAuthBridge>
    </AuthKitProvider>
  )
}

function WorkOSAuthBridge({ children }: { children: ReactNode }) {
  const { user, organizationId, loading, signOut } = useAuth()

  const value = useMemo<AppAuth>(
    () => ({
      user: mapWorkOSUser(user, organizationId),
      organizationId: organizationId ?? null,
      loading,
      signOut: async () => {
        await signOut()
      },
    }),
    [user, organizationId, loading, signOut],
  )

  return (
    <AppAuthContext.Provider value={value}>{children}</AppAuthContext.Provider>
  )
}

export function useAppAuth() {
  return useContext(AppAuthContext)
}
