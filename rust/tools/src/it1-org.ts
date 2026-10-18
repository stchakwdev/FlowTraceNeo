// IT1's organisation: `it1-<stamp>`, new on every run (never web1-20261016), opened by an HTTP
// operator the CLI login adds, loaded with the demo books (rust/fixtures/demo-books.json#/state,
// the parity suite's starting books and the oracle's), and three members:
//
//   Alice  post, approve, close
//   Bob    post, approve
//   Carol  none
//
// In dummy auth mode each is a device identity the run mints (POST /v1/identity); the web gets
// the token through local storage (`flowtrace.spacetimedb.token`,
// flowtrace/src/lib/ledger/spacetime-backend.ts) and the desktop through a token file
// (`token_file` in its config.toml, rust/crates/desktop/README.md). In workos mode Alice and Bob
// are added by subject (`add_member_by_subject`, AUTH1) for the WorkOS user ids in
// rust/tools/.env.it1, so their user id is the WorkOS user id and their identity the one the
// module derives from issuer and subject; their tokens come from WorkOS later
// (src/it1-workos.ts). Carol has no WorkOS user and stays a device identity.
import fs from 'node:fs'
import path from 'node:path'
import { demoFixtures } from './demo-replay'
import type { ModuleServer, Obj, Who } from './module-server'

export interface It1Person {
  key: 'alice' | 'bob' | 'carol'
  /** The member row's user id, the engine's actor id. */
  userId: string
  name: string
  canPost: boolean
  canApprove: boolean
  canClose: boolean
}

export const PEOPLE: Array<It1Person> = [
  { key: 'alice', userId: 'it1-alice', name: 'Alice', canPost: true, canApprove: true, canClose: true },
  { key: 'bob', userId: 'it1-bob', name: 'Bob', canPost: true, canApprove: true, canClose: false },
  { key: 'carol', userId: 'it1-carol', name: 'Carol', canPost: false, canApprove: false, canClose: false },
]

/** The engine actor of a member row (rules.rs actor_of): what the oracle runs each command as. */
export const actorOf = (p: It1Person) => ({ id: p.userId, name: p.name, can_post: p.canPost, can_approve: p.canApprove, can_close: p.canClose })

export interface It1Org {
  org: string
  operator: Who
  people: Record<It1Person['key'], Who>
  /** Token files for the desktop and the probe, one per person, mode 0600. */
  tokenFiles: Record<It1Person['key'], string>
  /** The books as loaded: demo-books.json#/state with the organisation id replaced. */
  demoState: Obj
}

/** Workos mode: Alice and Bob by subject at a trusted issuer. */
export interface BySubject {
  issuer: string
  subjects: Partial<Record<It1Person['key'], string>>
}

/** The people of a run: in workos mode Alice's and Bob's user ids are their WorkOS user ids. */
export const runPeople = (bySubject?: BySubject): Array<It1Person> => PEOPLE.map((p) => ({ ...p, userId: bySubject?.subjects[p.key] ?? p.userId }))

/** The identity hex of a member row key, `{org_id}:{identity hex}` (tables.rs, Member). */
export function identityOfKey(key: string): string {
  const hex = key.slice(key.lastIndexOf(':') + 1).replace(/^0x/, '')
  if (!/^[0-9a-f]{64}$/.test(hex)) throw new Error(`not a member key: ${key}`)
  return hex
}

/**
 * Opens the books, adds the members and loads the demo books. `dir` is the run's output folder
 * (gitignored rust/out/it1/<stamp>); the token files go in `dir/tokens`. With `bySubject`, the
 * people it names join by subject, with an empty token until the run signs them in, and
 * `memberKey` reads their member row's key (the operator's view of the private `member` table).
 */
export async function openIt1Org(server: ModuleServer, org: string, dir: string, bySubject?: BySubject, memberKey?: (userId: string) => string): Promise<It1Org> {
  const fx = demoFixtures(org)
  const operator = await server.newIdentity('operator')
  server.cli(['call', '-s', server.cliServer, server.db, 'add_operator', JSON.stringify(`0x${operator.identity}`)])
  await server.mustCall(operator, 'create_org_ledger', [org, fx.settings.base_currency as string, 2026, 'IT1 Operator'])
  const people = {} as Record<It1Person['key'], Who>
  const tokenFiles = {} as Record<It1Person['key'], string>
  fs.mkdirSync(path.join(dir, 'tokens'), { recursive: true, mode: 0o700 })
  for (const p of runPeople(bySubject)) {
    tokenFiles[p.key] = path.join(dir, 'tokens', p.key)
    const subject = bySubject?.subjects[p.key]
    if (subject !== undefined && bySubject && memberKey) {
      await server.mustCall(operator, 'add_member_by_subject', [org, bySubject.issuer, subject, p.name, p.canPost, p.canApprove, p.canClose])
      people[p.key] = { label: p.name, identity: identityOfKey(memberKey(subject)), token: '' }
      continue
    }
    const who = await server.newIdentity(p.name)
    people[p.key] = who
    await server.mustCall(operator, 'add_member', [org, [`0x${who.identity}`], p.userId, p.name, p.canPost, p.canApprove, p.canClose])
    fs.writeFileSync(tokenFiles[p.key], who.token, { mode: 0o600 })
  }
  await server.mustCall(operator, 'import_books', [org, JSON.stringify(fx.demoState)])
  return { org, operator, people, tokenFiles, demoState: fx.demoState }
}
