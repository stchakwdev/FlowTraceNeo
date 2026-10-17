// The IT1 command line (it1.ts). A run writes into the database it is pointed at, so the guard
// takes only a database whose name ends in `-dev` (flowtrace-ledger-dev on maincloud), or, with
// `--local`, a database of the run's own on the local server that the run publishes and deletes.
// The production database `flowtrace-ledger` is never a target (task IT1, hard limits).
import path from 'node:path'
import { LOCAL_URL } from './module-server'
import { RUST_DIR } from './paths'

export interface It1Flags {
  url: string
  db: string
  /** The local server: the run publishes `db` and deletes it at the end unless `keep`. */
  local: boolean
  auth: 'dummy' | 'workos'
  /** The offline claim: none, the user's Android phone, or Playwright's offline emulation. */
  offline: 'none' | 'phone' | 'emulated'
  webUrl: string
  desktop: string
  probe: string
  keep: boolean
  /** Show the browser window. */
  headed: boolean
  stamp: string
}

const VALUE_FLAGS = new Set(['url', 'db', 'auth', 'web-url', 'desktop'])
const BARE_FLAGS = new Set(['local', 'phone', 'offline-emulated', 'keep', 'headed'])

/** Throws unless `db` may take a run's writes: `-dev` anywhere, `ft-` names on the local server. */
export function checkDatabase(db: string, local: boolean) {
  if (local ? /^ft-[a-z0-9-]+$/.test(db) : /^[a-z0-9][a-z0-9-]*-dev$/.test(db)) return
  throw new Error(
    `Refusing database ${db}: IT1 writes only to a database whose name ends in -dev (flowtrace-ledger-dev), or to an ft- database of its own on the local server with --local.`,
  )
}

export function parseIt1Flags(argv: Array<string>, stamp: string): It1Flags {
  const values = new Map<string, string>()
  const bare = new Set<string>()
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (!a.startsWith('--')) throw new Error(`Unexpected argument ${a}.`)
    const name = a.slice(2)
    if (BARE_FLAGS.has(name)) bare.add(name)
    else if (VALUE_FLAGS.has(name)) {
      const v = argv[i + 1]
      if (v === undefined || v.startsWith('--')) throw new Error(`--${name} needs a value.`)
      values.set(name, v)
      i++
    } else throw new Error(`Unknown flag --${name}.`)
  }
  const local = bare.has('local')
  const url = values.get('url') ?? (local ? LOCAL_URL : '')
  const db = values.get('db') ?? (local ? `ft-it1-${stamp}` : '')
  if (local && !/^http:\/\/(127\.0\.0\.1|localhost):\d+$/.test(url)) throw new Error(`--local takes the local server only, not ${url}.`)
  if (url === '') throw new Error('--url <server> is required, for example https://maincloud.spacetimedb.com (or --local).')
  if (db === '') throw new Error('--db <name> is required, for example flowtrace-ledger-dev (or --local).')
  checkDatabase(db, local)
  if (bare.has('keep') && !local) throw new Error('--keep applies to --local only: a run never deletes a shared database.')
  const auth = values.get('auth') ?? 'dummy'
  if (auth !== 'dummy' && auth !== 'workos') throw new Error('--auth is dummy or workos.')
  if (bare.has('phone') && bare.has('offline-emulated')) throw new Error('Choose either --phone or --offline-emulated.')
  return {
    url,
    db,
    local,
    auth,
    offline: bare.has('phone') ? 'phone' : bare.has('offline-emulated') ? 'emulated' : 'none',
    webUrl: values.get('web-url') ?? 'http://localhost:3000',
    desktop: values.get('desktop') ?? path.join(RUST_DIR, 'target/release/desktop'),
    probe: path.join(RUST_DIR, 'target/release/examples/it1_probe'),
    keep: bare.has('keep'),
    headed: bare.has('headed'),
    stamp,
  }
}
