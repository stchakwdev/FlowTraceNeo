'use client'

import { createContext, useContext, useEffect, useState } from 'react'
import { useAuth } from '@workos-inc/authkit-react'
import {
  DEV_MODE,
  DEV_ORG_ID,
  createOrUpdateProfile,
  getOrganization,
  getProfile,
} from './supabase'
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
  const { user, isLoading: authLoading } = useAuth()
  const [state, setState] = useState<Omit<OrgContextValue, 'refetch'>>({
    organizationId: null,
    organization: null,
    profile: null,
    isLoading: true,
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

    // Wait for auth to finish loading
    if (authLoading) return

    // No user - clear state
    if (!user) {
      setState({
        organizationId: null,
        organization: null,
        profile: null,
        isLoading: false,
        error: null,
      })
      return
    }

    try {
      // Get or create user profile
      let profileData = await getProfile(user.id)

      if (!profileData) {
        // Create profile for new users
        const fullName =
          user.firstName && user.lastName
            ? `${user.firstName} ${user.lastName}`
            : user.firstName || user.lastName || undefined

        await createOrUpdateProfile(user.id, user.email || '', fullName)

        // Fetch the created profile
        profileData = await getProfile(user.id)
      }

      // Get organization if user has one
      let org: Organization | null = null
      if (profileData?.organization_id) {
        try {
          org = await getOrganization(profileData.organization_id)
        } catch {
          // Organization not found - user needs to be assigned one
          console.warn('User organization not found')
        }
      }

      setState({
        organizationId: profileData?.organization_id || null,
        organization: org,
        profile: profileData,
        isLoading: false,
        error: null,
      })
    } catch (error) {
      console.error('Error loading org context:', error)
      setState({
        organizationId: null,
        organization: null,
        profile: null,
        isLoading: false,
        error: error as Error,
      })
    }
  }

  useEffect(() => {
    loadOrgContext()
  }, [user, authLoading])

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
