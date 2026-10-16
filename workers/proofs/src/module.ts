/**
 * Reads from the ledger module (the SpacetimeDB database) over HTTP SQL
 * (https://spacetimedb.com/docs/http/database#post-v1databasename_or_identitysql).
 * The tables are private, so these queries run as the `MODULE_TOKEN` identity. The Worker only
 * reads; proofs are recorded by the client through the module's `create_verification` reducer.
 */
import type { Caller } from './auth'
import type { Env } from './env'
import { HttpError } from './http'
import { isOrgId, sqlString } from './keys'
import { decodeResponse, type Row } from './sql'
import type { AccessGroup, GroupMember, Member } from './visibility'

/** A `Verification` record (`rust/crates/ledger-module/src/transparency/types.rs:282`) as stored in `verification.data`. */
export interface Verification {
  id: string
  transaction_id?: string | null
  claim_id?: string | null
  bill_id?: string | null
  type?: string
  object_key?: string | null
  sha256?: string | null
  thumbnail_key?: string | null
  status?: string
  visibility_level?: string
  /** Shown on the public route. Absent means false. */
  published?: boolean
}

/**
 * The token the Worker reads the module with.
 * P17: swap to the service-role token when MOD2 lands. On dev this is the operator (owner) CLI
 * identity's token; MOD2 adds a `service` role that may read the private tables, and only this
 * function needs to change.
 */
export function moduleToken(env: Env): string {
  return env.MODULE_TOKEN
}

/** Run one SQL statement and return its rows. */
export async function sql(env: Env, query: string): Promise<Row[]> {
  const url = `${env.SPACETIMEDB_URL.replace(/\/+$/, '')}/v1/database/${encodeURIComponent(
    env.SPACETIMEDB_DATABASE,
  )}/sql`
  let res: Response
  try {
    res = await fetch(url, {
      method: 'POST',
      headers: { Authorization: `Bearer ${moduleToken(env)}`, 'Content-Type': 'text/plain' },
      body: query,
    })
  } catch {
    throw new HttpError(502, 'The ledger could not be reached. Try again shortly.')
  }
  if (!res.ok) {
    throw new HttpError(502, 'The ledger could not be read. Try again shortly.')
  }
  return decodeResponse(await res.json())
}

function requireOrg(org: string): void {
  if (!isOrgId(org)) throw new HttpError(400, 'That organisation id is not valid.')
}

function asMember(r: Row): Member {
  return {
    key: String(r.key ?? ''),
    identity: String(r.identity ?? ''),
    org_id: String(r.org_id ?? ''),
    user_id: String(r.user_id ?? ''),
    name: String(r.name ?? ''),
    can_post: r.can_post === true,
    can_approve: r.can_approve === true,
    can_close: r.can_close === true,
  }
}

/** The organisation's `member` rows. */
export async function members(env: Env, org: string): Promise<Member[]> {
  requireOrg(org)
  const rows = await sql(env, `SELECT * FROM member WHERE org_id = ${sqlString(org)}`)
  return rows.map(asMember)
}

/** The caller's member row: same identity, or for WorkOS `user_id == sub`. */
export function findMember(list: Member[], caller: Caller): Member | null {
  if (caller.kind === 'spacetimedb') {
    return list.find((m) => m.identity.toLowerCase() === caller.identity) ?? null
  }
  return list.find((m) => m.user_id === caller.userId) ?? null
}

/** The `data` JSON of doc rows, skipping any that do not parse. */
export function docData<T>(rows: Row[]): T[] {
  const out: T[] = []
  for (const r of rows) {
    if (typeof r.data !== 'string') continue
    try {
      out.push(JSON.parse(r.data) as T)
    } catch {
      // A malformed record is skipped rather than failing the request.
    }
  }
  return out
}

type DocTable = 'access_group' | 'access_group_member' | 'verification'

async function docs<T>(env: Env, table: DocTable, org: string): Promise<T[]> {
  requireOrg(org)
  return docData<T>(await sql(env, `SELECT * FROM ${table} WHERE org_id = ${sqlString(org)}`))
}

export const accessGroups = (env: Env, org: string) => docs<AccessGroup>(env, 'access_group', org)
export const groupMembers = (env: Env, org: string) =>
  docs<GroupMember>(env, 'access_group_member', org)
export const verifications = (env: Env, org: string) =>
  docs<Verification>(env, 'verification', org)

const PUBLIC_CACHE_MS = 60_000
/** A verification with the organisation its row belongs to (the row's `org_id` column). */
export type OrgVerification = Verification & { row_org_id: string }

let publicCache: { at: number; source: string; rows: OrgVerification[] } | null = null

/**
 * Every verification, for the public route. This is a full table scan, cached for 60 s: a known
 * scaling limit. MOD2's public projection (published proofs only) should replace it.
 */
export async function allVerifications(env: Env, now = Date.now()): Promise<OrgVerification[]> {
  const source = `${env.SPACETIMEDB_URL}|${env.SPACETIMEDB_DATABASE}`
  if (publicCache && publicCache.source === source && now - publicCache.at < PUBLIC_CACHE_MS) {
    return publicCache.rows
  }
  const rows: OrgVerification[] = []
  for (const r of await sql(env, 'SELECT * FROM verification')) {
    const [record] = docData<Verification>([r])
    if (record && typeof r.org_id === 'string') rows.push({ ...record, row_org_id: r.org_id })
  }
  publicCache = { at: now, source, rows }
  return rows
}

/** Forget the public cache (tests). */
export function clearPublicCache(): void {
  publicCache = null
}
