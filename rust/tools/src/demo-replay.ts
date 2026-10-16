// The demo organisation on a published ledger-module, shared by module-replay.ts and
// module-integration.ts: open its books, load rust/fixtures/demo-books.json#/state with
// `import_books`, and replay the 37 steps of rust/fixtures/commands-accepted.json through
// `run_command`, comparing every result and the final books with the fixture.
//
// The module draws UUIDs and stamps the reducer time where the fixture has fx-0001, ... and a
// frozen clock (spacetimedb/src/index.ts:361-365). So the fixture ids are mapped to the module's
// ids as the records appear (each step's new records against the fixture delta's `added`),
// commands are sent with mapped ids, and the comparison treats ids through that map and a reducer
// timestamp in place of the frozen one as equal. Everything else must be equal exactly.
import fs from 'node:fs'
import path from 'node:path'
import { FIXTURES_DIR } from './paths'
import type { Json, ModuleServer, Obj, Who } from './module-server'

export const readFixture = (name: string) => JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, name), 'utf8')) as Obj

/** The demo organisation's id in the fixtures. */
export const DEMO_ORG = ((readFixture('demo-books.json').state as Obj).settings as Obj).organization_id as string

/**
 * The demo books and the command log. With `org`, every value equal to the demo organisation's
 * id (`demo-books.json#/state/settings/organization_id`, and the same field in `final_state`)
 * becomes `org`, so a run against an existing database opens its own organisation each time.
 */
export function demoFixtures(org?: string) {
  const load = (name: string) => {
    const text = fs.readFileSync(path.join(FIXTURES_DIR, name), 'utf8')
    return JSON.parse(org === undefined ? text : text.replaceAll(JSON.stringify(DEMO_ORG), JSON.stringify(org))) as Obj
  }
  const commands = load('commands-accepted.json')
  const demoState = load('demo-books.json').state as Obj
  const settings = demoState.settings as Obj
  return {
    commands,
    demoState,
    settings,
    org: settings.organization_id as string,
    frozenNow: (commands.env as Obj).now as string,
    steps: commands.steps as Array<Obj>,
    actors: commands.actors as Record<string, Obj>,
    collections: Object.keys(demoState).filter((k) => Array.isArray(demoState[k])),
  }
}

export type DemoFixtures = ReturnType<typeof demoFixtures>

const FX_ID = /^fx-\d{4}$/
const FX_TOKEN = /fx-\d{4}/g
const ISO_MS = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/

/** Fixture id -> module id, learned as records appear, and the comparison through it. */
export class IdMap {
  readonly ids = new Map<string, string>()
  private readonly taken = new Map<string, string>()
  readonly stats = { idsMapped: 0, timestampsNormalised: 0, stringsWithIds: 0 }
  readonly mismatches: Array<string> = []

  constructor(private readonly frozenNow: string) {}

  /** Walks a fixture record and the module's record in step, learning fixture -> module ids. */
  learn(expected: Json, actual: unknown, where: string) {
    if (typeof expected === 'string' && FX_ID.test(expected) && typeof actual === 'string') {
      const known = this.ids.get(expected)
      if (known !== undefined && known !== actual) this.mismatches.push(`${where}: ${expected} is ${known} and ${actual}`)
      const owner = this.taken.get(actual)
      if (owner !== undefined && owner !== expected) this.mismatches.push(`${where}: ${actual} is ${owner} and ${expected}`)
      this.ids.set(expected, actual)
      this.taken.set(actual, expected)
    } else if (Array.isArray(expected) && Array.isArray(actual)) {
      expected.forEach((e, i) => this.learn(e, actual[i], `${where}[${i}]`))
    } else if (expected && typeof expected === 'object' && actual && typeof actual === 'object') {
      for (const k of Object.keys(expected)) this.learn((expected as Obj)[k], (actual as Obj)[k], `${where}.${k}`)
    }
  }

  /** A fixture value with every fixture id replaced by the module's (unknown ones kept). */
  translate(v: Json): Json {
    if (typeof v === 'string') return v.replace(FX_TOKEN, (t) => this.ids.get(t) ?? t)
    if (Array.isArray(v)) return v.map((x) => this.translate(x))
    if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, this.translate(x)]))
    return v
  }

  /** Compares a fixture value with the module's: ids through the map, the frozen `now` against any reducer timestamp, all else exactly. */
  compare(expected: Json, actual: unknown, where: string) {
    const miss = (m: string) => this.mismatches.push(`${where}: ${m}`)
    if (typeof expected === 'string') {
      if (expected === this.frozenNow && typeof actual === 'string' && ISO_MS.test(actual)) {
        if (actual !== expected) this.stats.timestampsNormalised += 1
        return
      }
      const mapped = this.translate(expected)
      if (mapped !== expected) {
        this.stats.stringsWithIds += 1
        if (FX_ID.test(expected)) this.stats.idsMapped += 1
      }
      if (mapped !== actual) miss(`want ${JSON.stringify(mapped)} got ${JSON.stringify(actual)}`)
      return
    }
    if (Array.isArray(expected)) {
      if (!Array.isArray(actual) || actual.length !== expected.length) {
        miss(`want ${expected.length} items got ${Array.isArray(actual) ? actual.length : typeof actual}`)
        return
      }
      expected.forEach((e, i) => this.compare(e, actual[i], `${where}[${i}]`))
      return
    }
    if (expected && typeof expected === 'object') {
      if (!actual || typeof actual !== 'object' || Array.isArray(actual)) {
        miss(`want an object got ${JSON.stringify(actual)}`)
        return
      }
      const want = Object.keys(expected).sort()
      const got = Object.keys(actual).sort()
      if (want.join() !== got.join()) miss(`want keys ${want.join(',')} got ${got.join(',')}`)
      for (const k of want) this.compare((expected as Obj)[k], (actual as Obj)[k], `${where}.${k}`)
      return
    }
    if (expected !== actual) miss(`want ${JSON.stringify(expected)} got ${JSON.stringify(actual)}`)
  }
}

export interface DemoPeople {
  operator: Who
  stranger: Who
  /** The fixture's actors by key, members of the demo organisation. */
  who: Record<string, Who>
  /** The actor key of a fixture user id. */
  actorKey: (userId: string) => string
  /** `user_dummy`: can post, approve and close. */
  admin: Who
}

/**
 * Makes an operator, a stranger and the fixture's actors (POST /v1/identity); the CLI identity
 * that published the module adds the operator, who opens the demo organisation's books
 * (`create_org_ledger`) and adds the actors as members with the fixture's user ids and
 * permissions. Returns before any books are loaded.
 */
export async function openDemoOrg(server: ModuleServer, fx: DemoFixtures, displayName: string): Promise<DemoPeople> {
  const operator = await server.newIdentity('operator')
  const stranger = await server.newIdentity('stranger')
  const who: Record<string, Who> = {}
  for (const key of Object.keys(fx.actors)) who[key] = await server.newIdentity(key)
  const actorKey = (id: string) => Object.keys(fx.actors).find((k) => fx.actors[k].id === id) as string

  server.cli(['call', '-s', server.cliServer, server.db, 'add_operator', JSON.stringify(`0x${operator.identity}`)])
  await server.mustCall(operator, 'create_org_ledger', [fx.org, fx.settings.base_currency as string, 2026, displayName])
  for (const [key, a] of Object.entries(fx.actors)) {
    await server.mustCall(operator, 'add_member', [
      fx.org,
      // An Identity is a one-element product in SATS JSON over HTTP; the CLI takes the bare string.
      [`0x${who[key].identity}`],
      a.id as string,
      a.name as string,
      a.can_post as boolean,
      a.can_approve as boolean,
      a.can_close as boolean,
    ])
  }
  return { operator, stranger, who, actorKey, admin: who[actorKey('user_dummy')] }
}

/**
 * Withdraws the operator `openDemoOrg` added (`remove_operator`, as the CLI identity), so a run
 * against an existing database leaves its operator table as it found it.
 */
export function retireOperator(server: ModuleServer, people: DemoPeople) {
  server.cli(['call', '-s', server.cliServer, server.db, 'remove_operator', JSON.stringify(`0x${people.operator.identity}`)])
}

/**
 * Loads demo-books.json#/state with `import_books` (after the members: add_member appends
 * member.add audit events, which the import replaces) and compares the books read back.
 */
export async function importDemoBooks(server: ModuleServer, fx: DemoFixtures, people: DemoPeople, map: IdMap) {
  await server.mustCall(people.operator, 'import_books', [fx.org, JSON.stringify(fx.demoState)])
  const books = await server.readBooks(people.admin, fx.org)
  const before = map.mismatches.length
  map.compare(fx.demoState, books, 'import')
  return { books, equal: map.mismatches.length === before }
}

export interface StepOutcome {
  index: number
  kind: string
  actor: string
  ok: boolean
  equal: boolean
  detail: string
}

/**
 * Sends each step's command as its actor (`run_command`, request id `req-<step>`), learns the
 * new records' ids, and compares the actor's `my_command_result` row with the step's result.
 * Then compares the final books with `final_state`.
 */
export async function replayCommandLog(server: ModuleServer, fx: DemoFixtures, people: DemoPeople, map: IdMap, start: Obj) {
  let books = start
  const steps: Array<StepOutcome> = []
  for (const [i, step] of fx.steps.entries()) {
    const cmd = step.command as Obj
    const kind = cmd.type as string
    const key = people.actorKey((step.actor as Obj).id as string)
    const caller = people.who[key]
    const requestId = `req-${i}`
    const before = new Map(fx.collections.map((c) => [c, new Set((books[c] as Array<Obj>).map((x) => x.id as string))]))
    const r = await server.call(caller, 'run_command', [fx.org, requestId, JSON.stringify(map.translate(cmd))])
    const delta = step.delta as Obj
    if (!r.ok) {
      map.mismatches.push(`step ${i} ${kind}: refused: ${r.text}`)
      steps.push({ index: i, kind, actor: key, ok: false, equal: false, detail: r.text })
      continue
    }
    books = await server.readBooks(people.admin, fx.org)
    for (const c of fx.collections) {
      const added = ((delta[c] as Obj | undefined)?.added ?? []) as Array<Json>
      const fresh = (books[c] as Array<Obj>).filter((x) => !before.get(c)?.has(x.id as string))
      if (fresh.length !== added.length) map.mismatches.push(`step ${i} ${kind}: ${c} added ${fresh.length}, fixture ${added.length}`)
      added.forEach((e, j) => map.learn(e, fresh[j], `step ${i} ${c}`))
    }
    const [row] = await server.sql(caller, 'SELECT * FROM my_command_result')
    const before0 = map.mismatches.length
    if (row?.request_id !== requestId) map.mismatches.push(`step ${i}: command_result is ${String(row?.request_id)}, not ${requestId}`)
    else map.compare(step.result as Json, JSON.parse(row.result_json as string), `step ${i} result`)
    steps.push({ index: i, kind, actor: key, ok: true, equal: map.mismatches.length === before0, detail: String(row?.result_json) })
  }
  const before = map.mismatches.length
  map.compare(fx.commands.final_state as Json, books, 'final_state')
  return { books, steps, finalEqual: map.mismatches.length === before, finalDifferences: map.mismatches.length - before }
}
