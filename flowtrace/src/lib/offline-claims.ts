// Offline-first capture for expense claims. A field officer on a weak
// connection fills in a claim; when the phone is offline (or the send fails
// on the network) the `claim.create` input is kept in IndexedDB on the
// phone and sent when the connection comes back. Each queued claim carries
// its own id, so a claim that arrived but whose reply was lost is not
// created twice: the ledger answers DUPLICATE and the queue drops it.
import { useCallback, useEffect, useState } from 'react'
import { LedgerError } from './ledger'
import { randomId } from './random-id'
import { useLedgerActor, useLedgerCommand } from './ledger-client'
import type { LedgerCommand } from './ledger'

export type ClaimInput = Extract<LedgerCommand, { type: 'claim.create' }>['input']

export interface QueuedClaim {
  /** The claim id the ledger will use; also the queue key. */
  key: string
  queued_at: string
  input: ClaimInput & { id: string; captured_offline: true }
  /** Why the last attempt to send it failed, if it did. */
  last_error?: string | null
}

export interface FlushResult {
  sent: Array<string>
  failed: Array<{ key: string; error: string }>
}

const DB_NAME = 'flowtrace-offline'
const STORE = 'claims'
const CHANGE_EVENT = 'flowtrace:offline-claims'

let dbPromise: Promise<IDBDatabase> | null = null

function openDb(): Promise<IDBDatabase> {
  if (typeof indexedDB === 'undefined')
    return Promise.reject(new Error('This browser cannot keep claims offline.'))
  dbPromise ??= new Promise<IDBDatabase>((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: 'key' })
    }
    req.onsuccess = () => {
      const db = req.result
      // Another tab upgrading the schema closes this handle; reopen next time.
      db.onversionchange = () => {
        db.close()
        dbPromise = null
      }
      resolve(db)
    }
    req.onerror = () => {
      dbPromise = null
      reject(req.error ?? new Error('Could not open the offline store.'))
    }
  })
  return dbPromise
}

/** Closes the cached connection. Tests use this between cases. */
export async function closeOfflineDb() {
  if (!dbPromise) return
  const db = await dbPromise.catch(() => null)
  db?.close()
  dbPromise = null
}

function tx<T>(mode: IDBTransactionMode, fn: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return openDb().then(
    (db) =>
      new Promise<T>((resolve, reject) => {
        const t = db.transaction(STORE, mode)
        const req = fn(t.objectStore(STORE))
        t.oncomplete = () => resolve(req.result)
        t.onerror = () => reject(t.error ?? req.error)
        t.onabort = () => reject(t.error ?? new Error('The offline store was busy.'))
      }),
  )
}

function notify() {
  if (typeof window !== 'undefined') window.dispatchEvent(new Event(CHANGE_EVENT))
}

/** Keeps a claim on this device until it can be sent. */
export async function enqueueClaim(input: ClaimInput): Promise<QueuedClaim> {
  const id = input.id ?? randomId()
  const item: QueuedClaim = {
    key: id,
    queued_at: new Date().toISOString(),
    input: { ...input, id, captured_offline: true },
    last_error: null,
  }
  await tx('readwrite', (s) => s.put(item))
  notify()
  return item
}

/** Claims waiting on this device, oldest first. */
export async function listQueued(): Promise<Array<QueuedClaim>> {
  if (typeof indexedDB === 'undefined') return []
  const all = await tx<Array<QueuedClaim>>('readonly', (s) => s.getAll())
  return all.sort((a, b) => a.queued_at.localeCompare(b.queued_at))
}

export async function removeQueued(key: string): Promise<void> {
  await tx('readwrite', (s) => s.delete(key))
  notify()
}

async function markFailed(item: QueuedClaim, error: string) {
  await tx('readwrite', (s) => s.put({ ...item, last_error: error }))
}

let flushing: Promise<FlushResult> | null = null

/**
 * Sends each queued claim through `run`, oldest first, and removes it once
 * the ledger has it. A failure keeps the claim in the queue with the reason.
 * Concurrent calls share one pass.
 */
export function flushQueued(run: (input: QueuedClaim['input']) => Promise<unknown>): Promise<FlushResult> {
  flushing ??= (async () => {
    const result: FlushResult = { sent: [], failed: [] }
    try {
      for (const item of await listQueued()) {
        try {
          await run(item.input)
          await removeQueued(item.key)
          result.sent.push(item.key)
        } catch (e) {
          // Already in the books: an earlier send arrived but its reply did not.
          if (e instanceof LedgerError && e.code === 'DUPLICATE') {
            await removeQueued(item.key)
            result.sent.push(item.key)
            continue
          }
          const message = e instanceof Error ? e.message : String(e)
          await markFailed(item, message)
          result.failed.push({ key: item.key, error: message })
        }
      }
    } finally {
      flushing = null
      notify()
    }
    return result
  })()
  return flushing
}

/**
 * True when an error means "could not reach the books" rather than "the
 * books said no", so the claim should be queued rather than shown as wrong.
 */
export function isNetworkError(e: unknown) {
  if (typeof navigator !== 'undefined' && navigator.onLine === false) return true
  if (e instanceof LedgerError) return false
  if (e instanceof TypeError) return true
  const m = e instanceof Error ? e.message : String(e)
  return /network|failed to fetch|fetch failed|offline|timed? ?out|websocket|connection/i.test(m)
}

export const isOffline = () => typeof navigator !== 'undefined' && navigator.onLine === false

/**
 * The queue for this device, flushed on mount and whenever the browser
 * comes back online.
 */
export function useOfflineClaimSync() {
  const command = useLedgerCommand()
  const actor = useLedgerActor()
  const [queued, setQueued] = useState<Array<QueuedClaim>>([])
  const [syncing, setSyncing] = useState(false)
  const [lastResult, setLastResult] = useState<FlushResult | null>(null)
  const { mutateAsync } = command

  const refresh = useCallback(() => {
    listQueued()
      .then(setQueued)
      .catch(() => setQueued([]))
  }, [])

  const flush = useCallback(async () => {
    if (isOffline()) return null
    setSyncing(true)
    try {
      const r = await flushQueued((input) => mutateAsync({ type: 'claim.create', input }))
      setLastResult(r)
      return r
    } catch {
      return null
    } finally {
      setSyncing(false)
      refresh()
    }
  }, [mutateAsync, refresh])

  useEffect(() => {
    refresh()
    window.addEventListener(CHANGE_EVENT, refresh)
    return () => window.removeEventListener(CHANGE_EVENT, refresh)
  }, [refresh])

  // Commands need the signed-in member, so wait for them before sending.
  const ready = !!actor
  useEffect(() => {
    if (!ready) return
    flush()
    const onOnline = () => {
      flush()
    }
    window.addEventListener('online', onOnline)
    return () => window.removeEventListener('online', onOnline)
  }, [ready, flush])

  const discard = useCallback(async (key: string) => {
    await removeQueued(key)
  }, [])

  return { queued, syncing, lastResult, flush, discard }
}
