'use client'

import { createContext, useContext, useEffect, useState } from 'react'
import { useAppAuth } from './auth-context'
import { DEV_MODE, DEV_ORG_ID, supabase } from './supabase'
import { demoOrganization, profiles as demoProfiles } from './demo/dataset'
import type { AuthUser } from './auth'
import type { ReactNode } from 'react'
import type { Organization, Profile } from './supabase'

interface OrgContextValue {
  organizationId: string | null
  organization: Organization | null
  profile: Profile | null
  user: AuthUser | null
  isLoading: boolean
  error: Error | null
  refetch: () => Promise<void>
}

const OrgContext = createContext<OrgContextValue>({
  organizationId: null,
  organization: null,
  profile: null,
  user: null,
  isLoading: true,
  error: null,
  refetch: async () => {},
})

// Dummy auth mode: the demo org and its admin profile from the bundled dataset
const DEV_ORGANIZATION: Organization = demoOrganization
const DEV_PROFILE: Profile = demoProfiles[0]

export function OrgProvider({ children }: { children: ReactNode }) {
  const { user, organizationId: authOrgId, loading: authLoading } = useAppAuth()
  const [state, setState] = useState<Omit<OrgContextValue, 'refetch' | 'user'>>(
    {
      organizationId: DEV_MODE ? DEV_ORG_ID : null,
      organization: DEV_MODE ? DEV_ORGANIZATION : null,
      profile: DEV_MODE ? DEV_PROFILE : null,
      isLoading: !DEV_MODE,
      error: null,
    },
  )

  const loadOrgContext = async () => {
    // Dev bypass mode - use mock organization
    if (DEV_MODE) {
      setState({
        organizationId: DEV_ORG_ID,
        organization: DEV_ORGANIZATION,
        profile: DEV_PROFILE,
        isLoading: false,
        error: null,
      })
      return
    }

    // Wait for auth to load
    if (authLoading) return

    // Not authenticated - clear org context
    if (!user) {
      setState((prev) => ({
        ...prev,
        organizationId: null,
        organization: null,
        profile: null,
        isLoading: false,
      }))
      return
    }

    try {
      // Try to load profile from Supabase using the auth user ID
      const { data: profile } = await supabase
        .from('profiles')
        .select('*')
        .eq('id', user.id)
        .single()

      let orgId = profile?.organization_id ?? authOrgId ?? null

      // If no profile or no org, auto-provision: create profile + link to demo org
      if (!profile || !orgId) {
        orgId = DEV_ORG_ID
        const { data: upsertedProfile } = await supabase
          .from('profiles')
          .upsert(
            {
              id: user.id,
              email: user.email,
              full_name:
                [user.firstName, user.lastName].filter(Boolean).join(' ') ||
                null,
              organization_id: orgId,
              role: 'admin' as const,
            },
            { onConflict: 'id' },
          )
          .select()
          .single()

        if (upsertedProfile) {
          const { data: org } = await supabase
            .from('organizations')
            .select('*')
            .eq('id', orgId)
            .single()

          setState({
            organizationId: orgId,
            organization: org ?? DEV_ORGANIZATION,
            profile: upsertedProfile,
            isLoading: false,
            error: null,
          })
          return
        }
      }

      // Load organization from Supabase
      if (orgId) {
        const { data: org } = await supabase
          .from('organizations')
          .select('*')
          .eq('id', orgId)
          .single()

        setState({
          organizationId: orgId,
          organization: org ?? DEV_ORGANIZATION,
          profile,
          isLoading: false,
          error: null,
        })
      } else {
        // No org found anywhere — fall back to demo org
        setState({
          organizationId: DEV_ORG_ID,
          organization: DEV_ORGANIZATION,
          profile: null,
          isLoading: false,
          error: null,
        })
      }
    } catch {
      // Supabase unreachable — fall back to demo org so the UI still works
      setState({
        organizationId: DEV_ORG_ID,
        organization: DEV_ORGANIZATION,
        profile: null,
        isLoading: false,
        error: null,
      })
    }
  }

  useEffect(() => {
    loadOrgContext()
  }, [user?.id, authLoading])

  return (
    <OrgContext.Provider value={{ ...state, user, refetch: loadOrgContext }}>
      {children}
    </OrgContext.Provider>
  )
}

export function useOrg() {
  return useContext(OrgContext)
}

// Helper to check if user has required role
export function useHasRole(requiredRoles: Array<Profile['role']>) {
  const { profile } = useOrg()
  if (!profile) return false
  return requiredRoles.includes(profile.role)
}

// Helper to check if user is admin or manager
export function useCanManage() {
  return useHasRole(['admin', 'manager'])
}
