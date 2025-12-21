'use client'

import { createContext, useContext, useEffect, useState } from 'react'
import { DEV_MODE, DEV_ORG_ID } from './supabase'
import type { ReactNode } from 'react'
import type { Organization, Profile } from './supabase'

interface OrgContextValue {
  organizationId: string | null
  organization: Organization | null
  profile: Profile | null
  isLoading: boolean
  error: Error | null
  refetch: () => Promise<void>
}

const OrgContext = createContext<OrgContextValue>({
  organizationId: null,
  organization: null,
  profile: null,
  isLoading: true,
  error: null,
  refetch: async () => {},
})

// Dev mode mock organization
const DEV_ORGANIZATION: Organization = {
  id: DEV_ORG_ID,
  name: 'Demo NGO Kenya',
  type: 'NGO',
  country: 'KE',
  currency: 'KES',
  trust_score: 87,
  verified: true,
  public_slug: 'demo-ngo',
  logo_url: null,
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
}

export function OrgProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<Omit<OrgContextValue, 'refetch'>>({
    organizationId: DEV_MODE ? DEV_ORG_ID : null,
    organization: DEV_MODE ? DEV_ORGANIZATION : null,
    profile: null,
    isLoading: !DEV_MODE,
    error: null,
  })

  const loadOrgContext = async () => {
    // Dev bypass mode - use mock organization
    if (DEV_MODE) {
      setState({
        organizationId: DEV_ORG_ID,
        organization: DEV_ORGANIZATION,
        profile: null,
        isLoading: false,
        error: null,
      })
      return
    }

    // When WorkOS is configured, this would load real user data
    // For now, just set not loading
    setState((prev) => ({ ...prev, isLoading: false }))
  }

  useEffect(() => {
    loadOrgContext()
  }, [])

  return (
    <OrgContext.Provider value={{ ...state, refetch: loadOrgContext }}>
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
