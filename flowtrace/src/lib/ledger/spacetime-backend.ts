// The SpacetimeDB ledger backend. Loaded lazily (ledger-client.ts uses
// import.meta.glob), so the demo bundle never pulls in the SDK.
//
// - load: subscribes to the caller's `my_*` views for one organisation and
//   reassembles a LedgerState from the client cache.
// - run: calls the `run_command` reducer with the command as JSON. The
//   server derives the actor from the caller's identity (the `actor`
//   argument is ignored), applies the command with the same engine, and
//   rolls back on a LedgerError, which is rebuilt here with its issues.
//
// Config: VITE_SPACETIMEDB_URI (e.g. wss://maincloud.spacetimedb.com) and
// VITE_SPACETIMEDB_MODULE (the database name). The token the connection
// presents follows the auth mode (spacetime-auth.ts): in dummy mode a device
// token kept in localStorage, so the browser keeps one identity across
// visits; in workos mode the signed-in person's WorkOS access token, never
// stored, fetched fresh for every connection.
import { AUTH_MODE } from '../auth'
import { randomId } from '../random-id'
import { DbConnection } from './spacetime-bindings'
import { connectionToken, refusalMessage, workosAccessToken } from './spacetime-auth'
import {
  DOC_COLLECTIONS,
  DOC_COLLECTION_KEYS,
  LEDGER_ERROR_PREFIX,
  assembleState,
  emptyDocs,
} from './spacetime-codec'
import { LedgerError } from './errors'
import type { DocRow, DocTable, EntryRow, LineRow, OrgRow, OrgRows } from './spacetime-codec'
import type { Issue } from './errors'
import type { LedgerBackend } from './backend'
import type { CommandResult, LedgerCommand } from './commands'
import type { Actor, LedgerState } from './types'

const TOKEN_KEY = 'flowtrace.spacetimedb.token'
const RESULT_TIMEOUT_MS = 15_000
/**
 * The longest one command may take, connecting included. Without automatic reconnect the
 * SDK queues a reducer call on a socket that died without closing (an iPhone leaving
 * aeroplane mode) and never settles it, so a send must give up and reconnect instead.
 */
export const COMMAND_TIMEOUT_MS = 30_000

interface Rows<T> {
  iter: () => Iterable<T>
}
interface Watchable<T> {
  onInsert: (cb: (ctx: unknown, row: T) => void) => void
  removeOnInsert: (cb: (ctx: unknown, row: T) => void) => void
  onUpdate: (cb: (ctx: unknown, oldRow: T, newRow: T) => void) => void
  removeOnUpdate: (cb: (ctx: unknown, oldRow: T, newRow: T) => void) => void
  onDelete: (cb: (ctx: unknown, row: T) => void) => void
  removeOnDelete: (cb: (ctx: unknown, row: T) => void) => void
}
type ViewHandle<T> = Rows<T> & Watchable<T>

interface CommandResultRow {
  orgId: string
  requestId: string
  resultJson: string
}

function config() {
  const uri = import.meta.env.VITE_SPACETIMEDB_URI as string | undefined
  const moduleName = import.meta.env.VITE_SPACETIMEDB_MODULE as string | undefined
  if (!uri || !moduleName) {
    throw new Error('Set VITE_SPACETIMEDB_URI and VITE_SPACETIMEDB_MODULE to use the SpacetimeDB ledger.')
  }
  return { uri, moduleName }
}

function readToken(): string | undefined {
  try {
    return localStorage.getItem(TOKEN_KEY) ?? undefined
  } catch {
    return undefined
  }
}

function saveToken(token: string) {
  try {
    localStorage.setItem(TOKEN_KEY, token)
  } catch {
    // Private mode or blocked storage: the identity lasts this session only.
  }
}

let connection: Promise<DbConnection> | null = null
// Connections dropped as dead: their late disconnect must not reset a newer connection.
const dropped = new WeakSet<object>()
let membership: Promise<void> | null = null

async function open(): Promise<DbConnection> {
  const { uri, moduleName } = config()
  const { token, saveReturned } = await connectionToken(AUTH_MODE, {
    device: readToken,
    workos: () => workosAccessToken(),
  })
  return new Promise<DbConnection>((resolve, reject) => {
    let connected = false
    DbConnection.builder()
      .withUri(uri)
      .withDatabaseName(moduleName)
      .withToken(token)
      .onConnect((conn, _identity, returned) => {
        connected = true
        // Only a device identity's token is kept; a WorkOS token never touches storage.
        if (saveReturned) saveToken(returned)
        resolve(conn)
      })
      .onConnectError((_ctx, error) => reject(error))
      .onDisconnect((ctx) => {
        if (dropped.has(ctx)) return
        connection = null
        subscriptions.clear()
        membership = null
        // The module's client_connected refused it: the host closes the socket before
        // onConnect, with no reason the SDK passes on.
        if (!connected) reject(new Error(refusalMessage(AUTH_MODE, token)))
      })
      .build()
  })
}

/** Forgets the current connection and closes it, so the next call opens a fresh one. */
function dropConnection() {
  const stale = connection
  connection = null
  subscriptions.clear()
  membership = null
  stale?.then(
    (conn) => {
      dropped.add(conn)
      conn.disconnect()
    },
    () => {},
  )
}

/** True when the module answered, so the connection itself is fine. */
const answeredByModule = (e: unknown) =>
  e instanceof LedgerError || (e instanceof Error && (e.name === 'SenderError' || e.name === 'InternalError'))

/** The browser's one SpacetimeDB connection, shared with spacetime-transparency.ts. */
export function connectSpacetime(): Promise<DbConnection> {
  return connect()
}

function connect(): Promise<DbConnection> {
  if (typeof window === 'undefined') {
    return Promise.reject(new Error('The SpacetimeDB ledger runs in the browser only.'))
  }
  // A refused or failed connection is not kept, so the next call tries again with a new token.
  connection ??= open().catch((error: unknown) => {
    connection = null
    throw error
  })
  return connection
}

// ---------------------------------------------------------------------------
// Views, by the accessor names of the generated bindings
// ---------------------------------------------------------------------------

const viewAccessor = (table: string) => `my${table[0].toUpperCase()}${table.slice(1)}`
const viewName = (table: string) => `my_${table.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)}`

const LEDGER_TABLES: Array<string> = ['ledgerOrg', 'journalEntry', 'journalLine', ...Object.values(DOC_COLLECTIONS)]

/** A `my_*` view of the connection by its table name (`campaign` is `my_campaign`). */
export function view<T>(conn: DbConnection, table: string): ViewHandle<T> {
  return (conn.db as unknown as Record<string, ViewHandle<T>>)[viewAccessor(table)]
}

const ORG_ID = /^[A-Za-z0-9_-]{1,128}$/

const subscriptions = new Map<string, Promise<void>>()

/** Subscribes once per organisation; resolves when the first rows arrive. */
function ensureSubscribed(conn: DbConnection, orgId: string): Promise<void> {
  if (!ORG_ID.test(orgId)) return Promise.reject(new Error(`Invalid organisation id ${orgId}.`))
  let sub = subscriptions.get(orgId)
  if (!sub) {
    sub = new Promise<void>((resolve, reject) => {
      const queries = LEDGER_TABLES.map((table) => `SELECT * FROM ${viewName(table)} WHERE org_id = '${orgId}'`)
      queries.push(`SELECT * FROM my_command_result`)
      conn
        .subscriptionBuilder()
        .onApplied(() => resolve())
        .onError((ctx) => {
          subscriptions.delete(orgId)
          reject(ctx.event ?? new Error('SpacetimeDB subscription failed.'))
        })
        .subscribe(queries)
    })
    subscriptions.set(orgId, sub)
  }
  return sub
}

function readState(conn: DbConnection, orgId: string): LedgerState {
  const mine = <T extends { orgId: string }>(table: string) =>
    [...view<T>(conn, table).iter()].filter((r) => r.orgId === orgId)
  const org = mine<OrgRow & { orgId: string }>('ledgerOrg')[0] as OrgRow | undefined
  if (!org) throw new Error(`No ledger for organisation ${orgId}, or you are not a member of it.`)
  const rows: OrgRows = {
    org,
    entries: mine<EntryRow>('journalEntry'),
    lines: mine<LineRow>('journalLine'),
    docs: emptyDocs(),
  }
  for (const k of DOC_COLLECTION_KEYS) rows.docs[k] = mine<DocRow>(DOC_COLLECTIONS[k] satisfies DocTable)
  return assembleState(rows)
}

/** A reducer error back into the engine's error, when it was one. */
function toLedgerError(e: unknown): unknown {
  const message = e instanceof Error ? e.message : String(e)
  if (!message.startsWith(LEDGER_ERROR_PREFIX)) return e
  try {
    return new LedgerError(JSON.parse(message.slice(LEDGER_ERROR_PREFIX.length)) as Array<Issue>)
  } catch {
    return e
  }
}

function waitForResult(conn: DbConnection, requestId: string) {
  const results = view<CommandResultRow>(conn, 'commandResult')
  let cancel = () => {}
  const promise = new Promise<CommandResult>((resolve, reject) => {
    const check = (row: CommandResultRow) => {
      if (row.requestId !== requestId) return
      cancel()
      resolve(JSON.parse(row.resultJson) as CommandResult)
    }
    const onInsert = (_ctx: unknown, row: CommandResultRow) => check(row)
    const onUpdate = (_ctx: unknown, _old: CommandResultRow, row: CommandResultRow) => check(row)
    const timer = setTimeout(() => {
      cancel()
      reject(new Error('The ledger did not confirm the command in time.'))
    }, RESULT_TIMEOUT_MS)
    cancel = () => {
      clearTimeout(timer)
      results.removeOnInsert(onInsert)
      results.removeOnUpdate(onUpdate)
    }
    results.onInsert(onInsert)
    results.onUpdate(onUpdate)
  })
  const cached = () => [...results.iter()].find((r) => r.requestId === requestId)
  return { promise, cached, cancel: () => cancel() }
}

export const spacetimeLedgerBackend: LedgerBackend = {
  kind: 'spacetimedb',

  load: async (organizationId) => {
    const conn = await connect()
    await ensureSubscribed(conn, organizationId)
    return readState(conn, organizationId)
  },

  run: async (organizationId, cmd: LedgerCommand) => {
    let waiter: ReturnType<typeof waitForResult> | undefined
    // Set at the deadline: a connect that finishes later must not send the command after all.
    let expired = false
    const send = async () => {
      const conn = await connect()
      await ensureSubscribed(conn, organizationId)
      if (expired) throw new Error('Timed out waiting for the ledger; reconnecting.')
      const requestId = randomId()
      waiter = waitForResult(conn, requestId)
      try {
        await conn.reducers.runCommand({ orgId: organizationId, requestId, commandJson: JSON.stringify(cmd) })
      } catch (e) {
        throw toLedgerError(e)
      }
      const row = waiter.cached()
      if (row) return JSON.parse(row.resultJson) as CommandResult
      return waiter.promise
    }
    let timer: ReturnType<typeof setTimeout> | undefined
    const deadline = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        expired = true
        reject(new Error('Timed out waiting for the ledger; reconnecting.'))
      }, COMMAND_TIMEOUT_MS)
    })
    try {
      return await Promise.race([send(), deadline])
    } catch (e) {
      // A dead or half-open socket: the next command opens a fresh one.
      if (!answeredByModule(e)) dropConnection()
      throw e
    } finally {
      clearTimeout(timer)
      waiter?.cancel()
    }
  },

  subscribe: (organizationId, onChange) => {
    let stopped = false
    let pending = false
    let detach = () => {}
    const emit = () => {
      if (pending || stopped) return
      pending = true
      // One callback per transaction, not one per row.
      setTimeout(() => {
        pending = false
        if (stopped) return
        void connection?.then((conn) => {
          try {
            onChange(readState(conn, organizationId))
          } catch {
            // The organisation is gone or access was revoked; load() reports it.
          }
        })
      }, 0)
    }
    void connect()
      .then(async (conn) => {
        await ensureSubscribed(conn, organizationId)
        if (stopped) return
        const onRow = (_ctx: unknown, row: { orgId: string }) => {
          if (row.orgId === organizationId) emit()
        }
        const onUpdate = (_ctx: unknown, _old: { orgId: string }, row: { orgId: string }) => {
          if (row.orgId === organizationId) emit()
        }
        const views = LEDGER_TABLES.map((table) => view<{ orgId: string }>(conn, table))
        for (const v of views) {
          v.onInsert(onRow)
          v.onUpdate(onUpdate)
          v.onDelete(onRow)
        }
        detach = () => {
          for (const v of views) {
            v.removeOnInsert(onRow)
            v.removeOnUpdate(onUpdate)
            v.removeOnDelete(onRow)
          }
        }
      })
      .catch(() => {
        // Connection errors surface through load() and run().
      })
    return () => {
      stopped = true
      detach()
    }
  },
}

/** Opens books for a new organisation; the caller becomes its owner. */
export async function createSpacetimeOrgLedger(
  organizationId: string,
  baseCurrency: string,
  fiscalYear: number,
  displayName: string,
) {
  const conn = await connect()
  await conn.reducers.createOrgLedger({ orgId: organizationId, baseCurrency, fiscalYear, displayName })
}

/** The identity of this browser's connection, to hand to an org admin. */
export async function spacetimeIdentity() {
  const conn = await connect()
  return conn.identity?.toHexString() ?? null
}

interface MemberRow {
  identity: { toHexString: () => string }
  orgId: string
  userId: string
  name: string
  canPost: boolean
  canApprove: boolean
  canClose: boolean
}

/** One organisation this browser's identity belongs to, as the engine's actor. */
export interface SpacetimeMembership {
  orgId: string
  actor: Actor
  /** The organisation's profile name (MOD1's `my_organization_profile`), when it has one. */
  orgName: string | null
}

interface ProfileRow {
  orgId: string
  data: string
}

/** Organisation id to profile name, from `organization_profile` rows (JSON `{ id, name, ... }`). */
export function organisationNames(rows: Iterable<ProfileRow>): Map<string, string> {
  const names = new Map<string, string>()
  for (const row of rows) {
    try {
      const name = (JSON.parse(row.data) as { name?: unknown }).name
      if (typeof name === 'string' && name.trim()) names.set(row.orgId, name.trim())
    } catch {
      // A malformed profile names nothing; the masthead shows the organisation id.
    }
  }
  return names
}

/** The member row as the actor the module derives for run_command (rust/crates/ledger-module/src/rules.rs:136, actor_of). */
export function actorFromMember(row: MemberRow): Actor {
  return {
    id: row.userId,
    name: row.name,
    can_post: row.canPost,
    can_approve: row.canApprove,
    can_close: row.canClose,
  }
}

function ensureMembership(conn: DbConnection): Promise<void> {
  membership ??= new Promise<void>((resolve, reject) => {
    conn
      .subscriptionBuilder()
      .onApplied(() => resolve())
      .onError((ctx) => {
        membership = null
        reject(ctx.event ?? new Error('SpacetimeDB subscription failed.'))
      })
      .subscribe(['SELECT * FROM my_member'])
    // Organisation names on their own, so a database published before MOD1 (no
    // my_organization_profile) still lists memberships; the names just stay absent.
    conn
      .subscriptionBuilder()
      .onError(() => {})
      .subscribe(['SELECT * FROM my_organization_profile'])
  })
  return membership
}

function profileRows(conn: DbConnection): Iterable<ProfileRow> {
  return view<ProfileRow>(conn, 'organizationProfile').iter()
}

/**
 * This browser's identity and the organisations it is a member of. A fresh
 * identity belongs to none until an operator or closing member adds it
 * (rust/docs/module-release.md section 3).
 */
export async function spacetimeMemberships(): Promise<{
  identity: string | null
  memberships: Array<SpacetimeMembership>
}> {
  const conn = await connect()
  await ensureMembership(conn)
  const identity = conn.identity?.toHexString() ?? null
  const names = organisationNames(profileRows(conn))
  const memberships = [...view<MemberRow>(conn, 'member').iter()]
    .filter((row) => row.identity.toHexString() === identity)
    .map((row) => ({
      orgId: row.orgId,
      actor: actorFromMember(row),
      orgName: names.get(row.orgId) ?? null,
    }))
  return { identity, memberships }
}

/** Calls back when this identity's memberships, or its organisations' names, change. */
export function onSpacetimeMembershipChange(onChange: () => void): () => void {
  let stopped = false
  let detach = () => {}
  const onRow = () => onChange()
  const onUpdate = () => onChange()
  void connect()
    .then(async (conn) => {
      await ensureMembership(conn)
      if (stopped) return
      const watched = [view<MemberRow>(conn, 'member'), view<ProfileRow>(conn, 'organizationProfile')]
      for (const v of watched as Array<ViewHandle<unknown>>) {
        v.onInsert(onRow)
        v.onUpdate(onUpdate)
        v.onDelete(onRow)
      }
      detach = () => {
        for (const v of watched as Array<ViewHandle<unknown>>) {
          v.removeOnInsert(onRow)
          v.removeOnUpdate(onUpdate)
          v.removeOnDelete(onRow)
        }
      }
    })
    .catch(() => {
      // Connection errors surface through spacetimeMemberships().
    })
  return () => {
    stopped = true
    detach()
  }
}
