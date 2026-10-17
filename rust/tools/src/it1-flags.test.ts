// The IT1 command line (it1.ts): the flags, their defaults, and the database guard that keeps a
// run off anything but a `-dev` database or the local server. No network.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import path from 'node:path'
import { checkDatabase, parseIt1Flags } from './it1-flags'
import { RUST_DIR } from './paths'

const MAINCLOUD = 'https://maincloud.spacetimedb.com'
const DEV = ['--url', MAINCLOUD, '--db', 'flowtrace-ledger-dev']

describe('checkDatabase', () => {
  test('takes a database whose name ends in -dev on any server', () => {
    expect(() => checkDatabase('flowtrace-ledger-dev', false)).not.toThrow()
    expect(() => checkDatabase('books-dev', false)).not.toThrow()
  })

  test('refuses the production database and any name not ending in -dev', () => {
    for (const db of ['flowtrace-ledger', 'flowtrace-ledger-prod', 'dev-books', 'flowtrace-ledger-dev2', 'flowtrace-ledger-dev ', '']) {
      expect(() => checkDatabase(db, false)).toThrow('Refusing')
    }
  })

  test('takes any ft- name on the local server, and only there', () => {
    expect(() => checkDatabase('ft-it1-20261017', true)).not.toThrow()
    expect(() => checkDatabase('flowtrace-ledger', true)).toThrow('Refusing')
  })
})

describe('parseIt1Flags', () => {
  test('defaults: dummy auth, no offline step, the local web app, the release desktop', () => {
    const f = parseIt1Flags(DEV, '20261017120000')
    expect(f).toEqual({
      url: MAINCLOUD,
      db: 'flowtrace-ledger-dev',
      local: false,
      auth: 'dummy',
      offline: 'none',
      webUrl: 'http://localhost:3000',
      desktop: path.join(RUST_DIR, 'target/release/desktop'),
      probe: path.join(RUST_DIR, 'target/release/examples/it1_probe'),
      keep: false,
      headed: false,
      stamp: '20261017120000',
    })
  })

  test('reads every flag', () => {
    const f = parseIt1Flags(
      [...DEV, '--auth', 'workos', '--offline-emulated', '--web-url', 'http://127.0.0.1:3001', '--desktop', '/opt/desktop', '--headed'],
      's',
    )
    expect(f.auth).toBe('workos')
    expect(f.offline).toBe('emulated')
    expect(f.webUrl).toBe('http://127.0.0.1:3001')
    expect(f.desktop).toBe('/opt/desktop')
    expect(f.headed).toBe(true)
    expect(parseIt1Flags([...DEV, '--phone'], 's').offline).toBe('phone')
  })

  test('--url and --db are required off the local server', () => {
    expect(() => parseIt1Flags(['--db', 'flowtrace-ledger-dev'], 's')).toThrow('--url')
    expect(() => parseIt1Flags(['--url', MAINCLOUD], 's')).toThrow('--db')
  })

  test('refuses the production database', () => {
    expect(() => parseIt1Flags(['--url', MAINCLOUD, '--db', 'flowtrace-ledger'], 's')).toThrow('Refusing database flowtrace-ledger')
  })

  test('--local: the local server and a database of its own, named for the run', () => {
    const f = parseIt1Flags(['--local'], '20261017120000')
    expect(f.local).toBe(true)
    expect(f.url).toBe('http://127.0.0.1:3100')
    expect(f.db).toBe('ft-it1-20261017120000')
    expect(parseIt1Flags(['--local', '--keep'], 's').keep).toBe(true)
  })

  test('--local refuses a server that is not local', () => {
    expect(() => parseIt1Flags(['--local', '--url', MAINCLOUD], 's')).toThrow('local server')
  })

  test('--keep only applies to --local', () => {
    expect(() => parseIt1Flags([...DEV, '--keep'], 's')).toThrow('--keep')
  })

  test('refuses unknown flags, missing values and bad choices', () => {
    expect(() => parseIt1Flags([...DEV, '--existing'], 's')).toThrow('Unknown flag --existing')
    expect(() => parseIt1Flags(['--url', '--db', 'x-dev'], 's')).toThrow('--url needs a value')
    expect(() => parseIt1Flags([...DEV, '--auth', 'saml'], 's')).toThrow('--auth is dummy or workos')
    expect(() => parseIt1Flags([...DEV, 'stray'], 's')).toThrow('Unexpected argument stray')
  })

  test('--phone and --offline-emulated exclude each other', () => {
    expect(() => parseIt1Flags([...DEV, '--phone', '--offline-emulated'], 's')).toThrow('either --phone or --offline-emulated')
  })
})
