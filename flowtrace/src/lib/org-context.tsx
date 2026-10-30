'use client'

import { createContext, useContext, useEffect, useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useRouterState } from '@tanstack/react-router'
import { useAppAuth } from './auth-context'
import { AUTH_MODE, DEV_MODE } from './auth'
import { DEV_ORG_ID } from './supabase'
import { demoOrganization, profiles as demoProfiles } from './demo/dataset'
import { loadTransparency } from './data'
import { orgContextFrom } from './ledger/transparency-model'
import {
  BACKEND,
  BooksNotConnectedError,
  CONFIGURED_ORG,
} from './spacetime-session'
import type { AuthUser } from './auth'
import type { ReactNode } from 'react'
import type { Organization, Profile } from './types'

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

// The demo books: the demo org and its admin profile from the bundled dataset
const DEV_ORGANIZATION: Organization = demoOrganization
const DEV_PROFILE: Profile = demoProfiles[0]

/** The query roots of the records the module serves (queries.ts queryKeys), refreshed on any change. */
const MODULE_QUERY_ROOTS = new Set([
  'organization',
  'campaigns',
  'campaign',
  'campaign-slug',
  'campaign-stats',
  'campaign-counts',
  'underfunded-campaigns',
  'campaign-report',
  'donations',
  'donation',
  'donor-donations',
  'donor-updates',
  'campaign-updates',
  // RW2: transactions, flow nodes and their visibility, the flow data, chains.
  'transactions',
  'transaction',
  'nodes',
  'org-stats',
  'flow-data',
  'node-visibility',
  'chains',
  'chain',
  'chain-counts',
  'chain-activity',
  'chain-trust-score',
  'chain-access-groups',
])

export const orgContextKey = ['ledger', 'org-context'] as const

/**
 * On SpacetimeDB the organisation and the person come from the module, never from the client:
 * the organisation and role from this identity's member row (`my_member`), the organisation
 * from `my_organization_profile` and `my_trust_score`, the person from `my_profile`. An
 * identity with no membership gets no organisation, only the not-connected notice with its
 * identity (as the books screens show it); nothing is provisioned.
 */
function ModuleOrgProvider({ children }: { children: ReactNode }) {
  const { user, organizationId: authOrgId, loading: authLoading } = useAppAuth()
  const queryClient = useQueryClient()
  // Only the signed-in app connects: the donate page, embed and badge read the public tables
  // over HTTP (ledger/public-sql.ts) and must not open a connection or mint an identity.
  const inApp = useRouterState({
    select: (r) => r.matches.some((m) => m.routeId === '/_authenticated'),
  })
  const enabled = inApp && (DEV_MODE || (!authLoading && !!user))
  const snapshot = useQuery({
    queryKey: orgContextKey,
    queryFn: async () => (await loadTransparency()).orgSnapshot(),
    enabled,
    staleTime: Infinity,
  })

  useEffect(() => {
    if (!enabled) return
    let unsubscribe: (() => void) | undefined
    let cancelled = false
    void loadTransparency().then((m) => {
      if (cancelled) return
      unsubscribe = m.onTransparencyChange(() => {
        void queryClient.invalidateQueries({ queryKey: orgContextKey })
        void queryClient.invalidateQueries({
          predicate: (q) => MODULE_QUERY_ROOTS.has(String(q.queryKey[0])),
        })
      })
    })
    return () => {
      cancelled = true
      unsubscribe?.()
    }
  }, [enabled, queryClient])

  const data = snapshot.data
  const ctx = data
    ? orgContextFrom({
        memberships: data.memberships,
        configured: CONFIGURED_ORG,
        contextOrgId: authOrgId,
      })
    : null

  // A WorkOS person's name and email go to their module profile once, so co-members see who
  // they are. A dummy-mode device identity is nobody in particular, so it sets none.
  const needsProfile =
    AUTH_MODE === 'workos' && !!ctx && !!data && !data.person && !!user
  useEffect(() => {
    if (!needsProfile) return
    const name = [user.firstName, user.lastName].filter(Boolean).join(' ')
    void loadTransparency()
      .then((m) => m.setProfile(name, user.email))
      .catch(() => {
        // Not fatal: the member row's name stands in.
      })
  }, [needsProfile, user])

  const value = useMemo((): OrgContextValue => {
    const refetch = async () => {
      await snapshot.refetch()
    }
    if (!enabled)
      return {
        organizationId: null,
        organization: null,
        profile: null,
        user,
        isLoading: authLoading,
        error: null,
        refetch,
      }
    if (!data)
      return {
        organizationId: null,
        organization: null,
        profile: null,
        user,
        isLoading: snapshot.isLoading,
        error: snapshot.error,
        refetch,
      }
    if (!ctx)
      return {
        organizationId: null,
        organization: null,
        profile: null,
        user,
        isLoading: false,
        error: new BooksNotConnectedError(data.identity),
        refetch,
      }
    const actor = ctx.membership.actor
    return {
      organizationId: ctx.orgId,
      organization: data.organizations.get(ctx.orgId) ?? null,
      profile: {
        id: actor.id,
        organization_id: ctx.orgId,
        email: user?.email ?? '',
        full_name: data.person?.display_name ?? actor.name,
        phone: null,
        role: ctx.role,
        created_at: '',
      },
      user,
      isLoading: false,
      error: null,
      refetch,
    }
  }, [enabled, data, ctx, user, authLoading, snapshot])

  return <OrgContext.Provider value={value}>{children}</OrgContext.Provider>
}

/** The demo books serve the demo organisation; with no backend there is none (Supabase is gone). */
function StaticOrgProvider({ children }: { children: ReactNode }) {
  const { user } = useAppAuth()
  const value = useMemo(
    (): OrgContextValue =>
      BACKEND === 'demo'
        ? {
            organizationId: DEV_ORG_ID,
            organization: DEV_ORGANIZATION,
            profile: DEV_PROFILE,
            user,
            isLoading: false,
            error: null,
            refetch: async () => {},
          }
        : {
            organizationId: null,
            organization: null,
            profile: null,
            user,
            isLoading: false,
            error: new BooksNotConnectedError(),
            refetch: async () => {},
          },
    [user],
  )
  return <OrgContext.Provider value={value}>{children}</OrgContext.Provider>
}

export const OrgProvider =
  BACKEND === 'spacetimedb' ? ModuleOrgProvider : StaticOrgProvider

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
