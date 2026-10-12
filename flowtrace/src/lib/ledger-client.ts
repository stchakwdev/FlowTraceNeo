// The app's handle on the books. Picks the backend (bundled demo, or
// SpacetimeDB when VITE_LEDGER_BACKEND=spacetimedb), loads the
// organisation's ledger state into TanStack Query, and sends commands with
// the signed-in member as the actor.
import { useEffect, useMemo } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { DEV_MODE } from './auth'
import { useOrg } from './org-context'
import { useUserPermissions } from './queries'
import { DEMO_ACTOR, DEMO_AS_OF, demoLedgerBackend } from './demo/ledger'
import { Ledger, actorFromPermissions } from './ledger'
import type {
  Actor,
  CommandResult,
  LedgerBackend,
  LedgerCommand,
  LedgerState,
} from './ledger'

const BACKEND = (import.meta.env.VITE_LEDGER_BACKEND as string | undefined) ??
  (DEV_MODE ? 'demo' : 'none')

// Lazy, so the demo bundle never loads the SpacetimeDB SDK.
const spacetimeLoaders = import.meta.glob<{
  spacetimeLedgerBackend: LedgerBackend
}>('./ledger/spacetime-backend.ts')

let backendPromise: Promise<LedgerBackend | null> | null = null

export function getLedgerBackend(): Promise<LedgerBackend | null> {
  backendPromise ??= (async () => {
    if (BACKEND === 'spacetimedb') {
      const load = Object.values(spacetimeLoaders)[0]
      if (!load) throw new Error('The SpacetimeDB ledger backend is not built into this app.')
      return (await load()).spacetimeLedgerBackend
    }
    if (BACKEND === 'demo' || DEV_MODE) return demoLedgerBackend
    return null
  })()
  return backendPromise
}

export const ledgerKey = (orgId: string) => ['ledger', orgId] as const

/** The date the books are reported "as of". The demo is a snapshot. */
export function useLedgerAsOf() {
  return DEV_MODE ? DEMO_AS_OF : new Date().toISOString().slice(0, 10)
}

export class BooksNotConnectedError extends Error {
  constructor() {
    super('The books are not connected for this organisation yet.')
    this.name = 'BooksNotConnectedError'
  }
}

/** The organisation's ledger state, kept fresh by the backend's subscription. */
export function useLedgerState() {
  const { organizationId } = useOrg()
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: ledgerKey(organizationId ?? ''),
    queryFn: async () => {
      const backend = await getLedgerBackend()
      if (!backend) throw new BooksNotConnectedError()
      return backend.load(organizationId!)
    },
    enabled: !!organizationId,
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
    () => (query.data ? new Ledger(query.data as LedgerState) : null),
    [query.data],
  )
  return { ...query, ledger, state: query.data ?? null }
}

/** The signed-in member as the engine sees them. */
export function useLedgerActor(): Actor | null {
  const { profile, user } = useOrg()
  const userId = profile?.id ?? user?.id ?? ''
  const { data: perms } = useUserPermissions(DEV_MODE ? '' : userId)
  if (DEV_MODE) return DEMO_ACTOR
  if (!perms || !userId) return null
  return actorFromPermissions(
    { id: userId, name: profile?.full_name ?? user?.email ?? 'Member' },
    perms,
  )
}

/** Sends one command; on success the ledger query refreshes. */
export function useLedgerCommand() {
  const { organizationId } = useOrg()
  const actor = useLedgerActor()
  const queryClient = useQueryClient()
  return useMutation<CommandResult, Error, LedgerCommand>({
    mutationFn: async (cmd) => {
      const backend = await getLedgerBackend()
      if (!backend) throw new BooksNotConnectedError()
      if (!actor) throw new Error('Your permissions are still loading. Try again in a moment.')
      return backend.run(organizationId!, cmd, actor)
    },
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ledgerKey(organizationId ?? '') }),
  })
}

/** The id a command created, when it created one. */
export const createdId = (r: CommandResult | undefined) =>
  r?.type === 'id' ? r.id : undefined
