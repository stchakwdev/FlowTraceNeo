// Which backend serves the app, and this browser's SpacetimeDB memberships. Shared by
// ledger-client.ts (the books) and org-context.tsx (the organisation and the person), which
// would otherwise import each other.
import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { DEV_MODE } from './auth'
import type * as SpacetimeBackendModule from './ledger/spacetime-backend'

export type LedgerBackendKind = 'demo' | 'spacetimedb' | 'none'

/** VITE_LEDGER_BACKEND wins; unset, dummy auth mode means the demo books. */
export function ledgerBackendKind(
  setting: string | undefined,
  devMode: boolean,
): LedgerBackendKind {
  if (setting === 'spacetimedb') return 'spacetimedb'
  if (setting === 'demo' || devMode) return 'demo'
  return 'none'
}

/** Whose books the screens show; null when this identity belongs to none yet. */
export function ledgerOrgId(input: {
  kind: LedgerBackendKind
  contextOrgId: string | null
  configured: string | undefined
  memberOrgs: Array<string>
}): string | null {
  if (input.kind !== 'spacetimedb') return input.contextOrgId
  if (input.configured) return input.configured
  if (input.contextOrgId && input.memberOrgs.includes(input.contextOrgId))
    return input.contextOrgId
  return [...input.memberOrgs].sort().at(0) ?? null
}

export const BACKEND = ledgerBackendKind(
  import.meta.env.VITE_LEDGER_BACKEND as string | undefined,
  DEV_MODE,
)
export const CONFIGURED_ORG =
  (import.meta.env.VITE_SPACETIMEDB_ORG as string | undefined) || undefined

type SpacetimeModule = typeof SpacetimeBackendModule

// Lazy, so the demo bundle never loads the SpacetimeDB SDK.
const spacetimeLoaders = import.meta.glob<SpacetimeModule>(
  './ledger/spacetime-backend.ts',
)

export function loadSpacetime(): Promise<SpacetimeModule> {
  const load = Object.values(spacetimeLoaders).at(0)
  if (!load)
    throw new Error(
      'The SpacetimeDB ledger backend is not built into this app.',
    )
  return load()
}

export class BooksNotConnectedError extends Error {
  /** This browser's SpacetimeDB identity, to hand to an operator. */
  readonly identity: string | null

  constructor(identity: string | null = null) {
    super('The books are not connected for this organisation yet.')
    this.name = 'BooksNotConnectedError'
    this.identity = identity
  }
}

export const membershipKey = ['ledger', 'membership'] as const

/** This browser's SpacetimeDB identity and memberships, kept fresh. */
export function useSpacetimeMemberships() {
  const queryClient = useQueryClient()
  const enabled = BACKEND === 'spacetimedb'
  const query = useQuery({
    queryKey: membershipKey,
    queryFn: async () => (await loadSpacetime()).spacetimeMemberships(),
    enabled,
    staleTime: Infinity,
  })
  useEffect(() => {
    if (!enabled) return
    let unsubscribe: (() => void) | undefined
    let cancelled = false
    void loadSpacetime().then((m) => {
      if (cancelled) return
      unsubscribe = m.onSpacetimeMembershipChange(() =>
        queryClient.invalidateQueries({ queryKey: membershipKey }),
      )
    })
    return () => {
      cancelled = true
      unsubscribe?.()
    }
  }, [enabled, queryClient])
  return query
}
