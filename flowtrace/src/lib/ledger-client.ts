// The app's handle on the books. Picks the backend (bundled demo, or
// SpacetimeDB when VITE_LEDGER_BACKEND=spacetimedb, in either auth mode),
// loads the organisation's ledger state into TanStack Query, and sends
// commands with the signed-in member as the actor.
//
// On SpacetimeDB the member row of this browser's identity is the actor, as
// the module derives it. In dummy auth mode there is no signed-in
// organisation, so the books are VITE_SPACETIMEDB_ORG when set, else the
// first organisation the identity (from the saved token) is a member of.
import { useEffect, useMemo } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { DEV_MODE } from './auth'
import { useOrg } from './org-context'
import { useUserPermissions } from './queries'
import { DEMO_ACTOR, DEMO_AS_OF, demoLedgerBackend } from './demo/ledger'
import { Ledger, actorFromPermissions } from './ledger'
import type * as SpacetimeBackendModule from './ledger/spacetime-backend'
import type {
  Actor,
  CommandResult,
  LedgerBackend,
  LedgerCommand,
} from './ledger'

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

/** Whose books the screens show; null when this device belongs to none yet. */
export function ledgerOrgId(input: {
  kind: LedgerBackendKind
  devMode: boolean
  contextOrgId: string | null
  configured: string | undefined
  memberOrgs: Array<string>
}): string | null {
  if (input.kind !== 'spacetimedb' || !input.devMode) return input.contextOrgId
  if (input.configured) return input.configured
  return [...input.memberOrgs].sort().at(0) ?? null
}

const BACKEND = ledgerBackendKind(
  import.meta.env.VITE_LEDGER_BACKEND as string | undefined,
  DEV_MODE,
)
/** The books are the bundled demo snapshot (tag them "Demo data"). */
export const DEMO_BOOKS = BACKEND === 'demo'
const CONFIGURED_ORG =
  (import.meta.env.VITE_SPACETIMEDB_ORG as string | undefined) || undefined

type SpacetimeModule = typeof SpacetimeBackendModule

// Lazy, so the demo bundle never loads the SpacetimeDB SDK.
const spacetimeLoaders = import.meta.glob<SpacetimeModule>(
  './ledger/spacetime-backend.ts',
)

function loadSpacetime(): Promise<SpacetimeModule> {
  const load = Object.values(spacetimeLoaders).at(0)
  if (!load) throw new Error('The SpacetimeDB ledger backend is not built into this app.')
  return load()
}

let backendPromise: Promise<LedgerBackend | null> | null = null

export function getLedgerBackend(): Promise<LedgerBackend | null> {
  backendPromise ??= (async () => {
    if (BACKEND === 'spacetimedb') return (await loadSpacetime()).spacetimeLedgerBackend
    if (BACKEND === 'demo') return demoLedgerBackend
    return null
  })()
  return backendPromise
}

export const ledgerKey = (orgId: string) => ['ledger', orgId] as const
const membershipKey = ['ledger', 'membership'] as const

/** The date the books are reported "as of". The demo is a snapshot. */
export function useLedgerAsOf() {
  return BACKEND === 'demo' ? DEMO_AS_OF : new Date().toISOString().slice(0, 10)
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

/** This browser's SpacetimeDB identity and memberships, kept fresh. */
function useSpacetimeMemberships() {
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

/** The organisation whose books the screens read, once it is known. */
function useLedgerTarget() {
  const { organizationId } = useOrg()
  const membership = useSpacetimeMemberships()
  const memberships = membership.data?.memberships ?? []
  const orgId = ledgerOrgId({
    kind: BACKEND,
    devMode: DEV_MODE,
    contextOrgId: organizationId,
    configured: CONFIGURED_ORG,
    memberOrgs: memberships.map((m) => m.orgId),
  })
  const waitsForMembership =
    BACKEND === 'spacetimedb' && DEV_MODE && !CONFIGURED_ORG
  return {
    orgId,
    ready: waitsForMembership ? membership.isSuccess : !!orgId,
    identity: membership.data?.identity ?? null,
    actor:
      BACKEND === 'spacetimedb'
        ? (memberships.find((m) => m.orgId === orgId)?.actor ?? null)
        : undefined,
    membershipError: membership.error,
  }
}

/** The organisation's ledger state, kept fresh by the backend's subscription. */
export function useLedgerState() {
  const target = useLedgerTarget()
  const organizationId = target.orgId
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ledgerKey(organizationId ?? ''),
    queryFn: async () => {
      const backend = await getLedgerBackend()
      if (!backend || !organizationId) throw new BooksNotConnectedError(target.identity)
      return backend.load(organizationId)
    },
    enabled: target.ready,
    staleTime: 60 * 1000,
  })

  useEffect(() => {
    if (!organizationId) return
    let unsubscribe: (() => void) | undefined
    let cancelled = false
    getLedgerBackend().then((backend) => {
      if (cancelled || !backend?.subscribe) return
      unsubscribe = backend.subscribe(organizationId, (state) =>
        queryClient.setQueryData(ledgerKey(organizationId), structuredClone(state)),
      )
    })
    return () => {
      cancelled = true
      unsubscribe?.()
    }
  }, [organizationId, queryClient])

  if (target.membershipError && !query.data)
    return { ...query, isLoading: false, error: target.membershipError }
  return query
}

/**
 * The ledger as a read-only engine view: lookups (`account`, `grant`,
 * `periodFor`…) and `validateEntry` for live form feedback. Never call its
 * write methods from the UI; use useLedgerCommand.
 */
export function useLedger() {
  const query = useLedgerState()
  const ledger = useMemo(
    () => (query.data ? new Ledger(query.data) : null),
    [query.data],
  )
  return { ...query, ledger, state: query.data ?? null }
}

/** The signed-in member as the engine sees them. */
export function useLedgerActor(): Actor | null {
  const { profile, user } = useOrg()
  const target = useLedgerTarget()
  const userId = profile?.id ?? user?.id ?? ''
  const fromProfile = !DEV_MODE && BACKEND !== 'spacetimedb'
  const { data: perms } = useUserPermissions(fromProfile ? userId : '')
  // SpacetimeDB acts as the member row whatever the client says.
  if (target.actor !== undefined) return target.actor
  if (DEV_MODE) return DEMO_ACTOR
  if (!perms || !userId) return null
  return actorFromPermissions(
    { id: userId, name: profile?.full_name ?? user?.email ?? 'Member' },
    perms,
  )
}

/** Sends one command; on success the ledger query refreshes. */
export function useLedgerCommand() {
  const { orgId: organizationId } = useLedgerTarget()
  const actor = useLedgerActor()
  const queryClient = useQueryClient()
  return useMutation<CommandResult, Error, LedgerCommand>({
    mutationFn: async (cmd) => {
      const backend = await getLedgerBackend()
      if (!backend || !organizationId) throw new BooksNotConnectedError()
      if (!actor) throw new Error('Your permissions are still loading. Try again in a moment.')
      return backend.run(organizationId, cmd, actor)
    },
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ledgerKey(organizationId ?? '') }),
  })
}

/** The id a command created, when it created one. */
export const createdId = (r: CommandResult | undefined) =>
  r?.type === 'id' ? r.id : undefined
