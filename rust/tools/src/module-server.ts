// The ledger-module on the LOCAL SpacetimeDB server, shared by module-replay.ts and
// module-integration.ts: the CLI for publish, delete and owner-only calls, HTTP for identities,
// reducer calls and SQL, and the rows read back as books through the TS client codec.
//
// CLI reference: https://spacetimedb.com/docs/cli-reference (publish, call, sql, delete, list).
// HTTP reference: https://spacetimedb.com/docs/http/identity (POST /v1/identity) and
// https://spacetimedb.com/docs/http/database (call, sql, GET /v1/database/:name).
//
// Only ever the local server (http://127.0.0.1:3100): `localServer` refuses any other URL and any
// database name that does not start with `ft-`.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { DOC_COLLECTIONS, DOC_COLLECTION_KEYS, assembleState } from '../../../flowtrace/src/lib/ledger/spacetime-codec.ts'
import type { DocCollection, DocRow, EntryRow, LineRow, OrgRows } from '../../../flowtrace/src/lib/ledger/spacetime-codec.ts'
import { RUST_DIR } from './paths'

export type Json = null | boolean | number | string | Array<Json> | { [k: string]: Json }
export type Obj = { [k: string]: Json }
export type Row = Record<string, unknown>

export const SPACETIME = process.env.SPACETIME ?? path.join(os.homedir(), '.local/bin/spacetime')
export const WASM = path.join(RUST_DIR, 'target/wasm32-unknown-unknown/release/ledger_module.wasm')
export const LOCAL_URL = 'http://127.0.0.1:3100'

/** `--name value` and `--name` from the command line. */
export function args(argv = process.argv.slice(2)) {
  return {
    flag: (name: string) => argv.includes(`--${name}`),
    option: (name: string, fallback: string) => {
      const i = argv.indexOf(`--${name}`)
      return i >= 0 && argv[i + 1] ? argv[i + 1] : fallback
    },
  }
}

export interface Who {
  label: string
  identity: string
  token: string
}

interface SqlColumn {
  name: { some: string }
  algebraic_type: { Sum?: { variants: Array<{ name: { some: string } }> } }
}

interface SqlResult {
  schema: { elements: Array<SqlColumn> }
  rows: Array<Array<Json>>
}

/** The table or view a doc collection lives in (`budgetLines` -> `budget_line`). */
export const docTable = (k: DocCollection) => DOC_COLLECTIONS[k].replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)

/** Every LedgerState collection and its module table, entries and lines first. */
export const COLLECTION_TABLES: Array<[string, string]> = [
  ['entries', 'journal_entry'],
  ['lines', 'journal_line'],
  ...DOC_COLLECTION_KEYS.map((k): [string, string] => [k, docTable(k)]),
]

const big = (v: unknown) => BigInt(v as number)

export function localServer(db: string, url = LOCAL_URL) {
  if (!db.startsWith('ft-')) throw new Error(`Refusing database name ${db}: use ft-<task>.`)
  if (!/^http:\/\/(127\.0\.0\.1|localhost):\d+$/.test(url)) throw new Error(`Refusing ${url}: local server only.`)

  /** The CLI as the identity that published the module, with its exit status. */
  function cliStatus(argv: Array<string>): { ok: boolean; text: string } {
    const r = Bun.spawnSync([SPACETIME, ...argv], { stdout: 'pipe', stderr: 'pipe' })
    const out = `${r.stdout.toString()}${r.stderr.toString()}`.replace(/WARNING: This command is UNSTABLE[^\n]*\n\n?/g, '')
    return { ok: r.exitCode === 0, text: out.trim() }
  }

  /** The CLI's output; throws on a non-zero exit unless `mayFail`. */
  function cli(argv: Array<string>, mayFail = false): string {
    const r = cliStatus(argv)
    if (!r.ok && !mayFail) throw new Error(`spacetime ${argv.join(' ')} failed:\n${r.text}`)
    return r.text
  }

  function publish(wasm = WASM) {
    if (!fs.existsSync(wasm)) throw new Error(`No module at ${wasm}; build it first.`)
    cli(['delete', '-s', 'local', '-y', db], true)
    return cli(['publish', '-s', 'local', '-b', wasm, '-y', db])
  }

  const remove = () => cli(['delete', '-s', 'local', '-y', db])
  const list = () => cli(['list', '-s', 'local'])

  /** `GET /v1/database/:name`: the database's owner and address. */
  async function info(): Promise<{ owner_identity: string; database_identity: string }> {
    const res = await fetch(`${url}/v1/database/${db}`)
    if (!res.ok) throw new Error(`GET /v1/database/${db}: ${res.status} ${await res.text()}`)
    const body = (await res.json()) as { owner_identity: { __identity__: string } | string; database_identity: { __identity__: string } | string }
    const hex = (v: { __identity__: string } | string) => (typeof v === 'string' ? v : v.__identity__).replace(/^0x/, '')
    return { owner_identity: hex(body.owner_identity), database_identity: hex(body.database_identity) }
  }

  async function newIdentity(label: string): Promise<Who> {
    const res = await fetch(`${url}/v1/identity`, { method: 'POST' })
    if (!res.ok) throw new Error(`POST /v1/identity: ${res.status}`)
    const body = (await res.json()) as { identity: string; token: string }
    return { label, identity: body.identity, token: body.token }
  }

  /** A reducer call; `ok` false carries the reducer's error text (HTTP 530). */
  async function call(who: Who, reducer: string, argv: Array<Json>): Promise<{ ok: boolean; text: string }> {
    const res = await fetch(`${url}/v1/database/${db}/call/${reducer}`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${who.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(argv),
    })
    return { ok: res.ok, text: await res.text() }
  }

  async function mustCall(who: Who, reducer: string, argv: Array<Json>) {
    const r = await call(who, reducer, argv)
    if (!r.ok) throw new Error(`${reducer} as ${who.label} failed: ${r.text}`)
  }

  /**
   * SQL rows as objects keyed by column. An `Option` column is a sum value, which the HTTP API
   * writes as `[tag, value]` (tag by variant position) or `{ "some": value }`; it becomes the value
   * or undefined, as the TS SDK gives it to the codec.
   */
  async function sql(who: Who, query: string): Promise<Array<Row>> {
    const res = await fetch(`${url}/v1/database/${db}/sql`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${who.token}` },
      body: query,
    })
    if (!res.ok) throw new Error(`${query}: ${res.status} ${await res.text()}`)
    const [result] = (await res.json()) as Array<SqlResult>
    const columns = result.schema.elements.map((e) => {
      const variants = e.algebraic_type.Sum?.variants.map((v) => v.name.some)
      return { name: e.name.some, option: variants?.join() === 'some,none' }
    })
    const unwrap = (v: Json, option: boolean): unknown => {
      if (!option) return v
      if (Array.isArray(v)) return v[0] === 0 ? v[1] : undefined
      if (v && typeof v === 'object') return 'some' in v ? v.some : undefined
      throw new Error(`${query}: option value ${JSON.stringify(v)}`)
    }
    return result.rows.map((row) => Object.fromEntries(columns.map((c, i) => [c.name, unwrap(row[i], c.option)])))
  }

  /** The organisation's books as `who` sees them through the `my_*` views. */
  async function readBooks(who: Who, orgId: string) {
    const mine = (rows: Array<Row>) => rows.filter((r) => r.org_id === orgId)
    const [org] = mine(await sql(who, 'SELECT * FROM my_ledger_org'))
    if (!org) throw new Error(`${who.label} sees no ledger_org row for ${orgId}`)
    const entries: Array<EntryRow> = mine(await sql(who, 'SELECT * FROM my_journal_entry')).map((r) => ({
      key: r.key as string,
      orgId: r.org_id as string,
      id: r.id as string,
      seq: big(r.seq),
      number: r.number as string,
      date: r.date as string,
      periodId: r.period_id as string,
      memo: r.memo as string,
      sourceType: r.source_type as string,
      sourceId: r.source_id as string | undefined,
      status: r.status as string,
      reversalOf: r.reversal_of as string | undefined,
      reversedBy: r.reversed_by as string | undefined,
      overrideReason: r.override_reason as string | undefined,
      createdBy: r.created_by as string,
      createdAt: r.created_at as string,
      postedBy: r.posted_by as string | undefined,
      postedAt: r.posted_at as string | undefined,
    }))
    const lines: Array<LineRow> = mine(await sql(who, 'SELECT * FROM my_journal_line')).map((r) => ({
      key: r.key as string,
      orgId: r.org_id as string,
      id: r.id as string,
      seq: big(r.seq),
      entryId: r.entry_id as string,
      accountId: r.account_id as string,
      debit: big(r.debit),
      credit: big(r.credit),
      currency: r.currency as string,
      fxRate: r.fx_rate as number,
      baseAmount: big(r.base_amount),
      fundId: r.fund_id as string,
      grantId: r.grant_id as string | undefined,
      dimensionIds: r.dimension_ids as Array<string>,
      flowNodeId: r.flow_node_id as string | undefined,
      memo: r.memo as string | undefined,
    }))
    const docs = {} as Record<DocCollection, Array<DocRow>>
    for (const k of DOC_COLLECTION_KEYS) {
      docs[k] = mine(await sql(who, `SELECT * FROM my_${docTable(k)}`)).map((r) => ({
        key: r.key as string,
        orgId: r.org_id as string,
        id: r.id as string,
        seq: big(r.seq),
        data: r.data as string,
      }))
    }
    const rows: OrgRows = {
      org: {
        orgId,
        settingsJson: org.settings_json as string,
        countersJson: org.counters_json as string,
        nextSeq: big(org.next_seq),
      },
      entries,
      lines,
      docs,
    }
    // Through JSON, so the books compare as the fixture's plain JSON does.
    return JSON.parse(JSON.stringify(assembleState(rows))) as Obj
  }

  return { db, url, cli, cliStatus, publish, remove, list, info, newIdentity, call, mustCall, sql, readBooks }
}

export type LocalServer = ReturnType<typeof localServer>
