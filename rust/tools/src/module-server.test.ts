// The remote mode of module-server.ts: an existing database is only ever called and read, never
// published, deleted, renamed or listed, and the flags that pick it. No network: every refusal
// happens before the CLI or HTTP is touched.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { cliServerFor, localServer, parseTarget, remoteServer } from './module-server'

const MAINCLOUD = 'https://maincloud.spacetimedb.com'
const DEV = 'flowtrace-ledger-dev'

describe('remoteServer', () => {
  test('targets an existing database on any server and under any name', () => {
    const s = remoteServer(DEV, MAINCLOUD)
    expect(s.db).toBe(DEV)
    expect(s.url).toBe(MAINCLOUD)
    expect(s.existing).toBe(true)
    expect(s.cliServer).toBe('maincloud')
  })

  test('has no publish', () => {
    expect(() => remoteServer(DEV, MAINCLOUD).publish()).toThrow(`publish is not available on the existing database ${DEV}`)
  })

  test('has no remove', () => {
    expect(() => remoteServer(DEV, MAINCLOUD).remove()).toThrow(`remove is not available on the existing database ${DEV}`)
  })

  test('has no list', () => {
    expect(() => remoteServer(DEV, MAINCLOUD).list()).toThrow(`list is not available on the existing database ${DEV}`)
  })

  test('refuses publish, delete, rename and list through the raw CLI as well', () => {
    const s = remoteServer(DEV, MAINCLOUD)
    for (const sub of ['publish', 'delete', 'rename', 'list']) {
      expect(() => s.cli([sub, '-s', 'maincloud', '-y', DEV])).toThrow(`Refusing spacetime ${sub}`)
      expect(() => s.cliStatus([sub, '-s', 'maincloud', DEV])).toThrow(`Refusing spacetime ${sub}`)
    }
  })

  test('refuses a CLI call aimed at another server or database', () => {
    const s = remoteServer(DEV, MAINCLOUD)
    expect(() => s.cli(['call', '-s', 'local', DEV, 'add_operator', '"0x00"'])).toThrow('Refusing spacetime call -s local')
    expect(() => s.cli(['sql', '-s', 'maincloud', 'flowtrace-ledger', 'SELECT 1'])).toThrow('Refusing spacetime sql on flowtrace-ledger')
  })

  test('needs a database name and an http(s) URL', () => {
    expect(() => remoteServer('', MAINCLOUD)).toThrow('needs a database name')
    expect(() => remoteServer(DEV, 'maincloud')).toThrow('Refusing maincloud: not an http(s) URL')
  })
})

describe('localServer keeps its guard', () => {
  test('refuses maincloud and names without ft-', () => {
    expect(() => localServer('ft-x', MAINCLOUD)).toThrow('local server only')
    expect(() => localServer(DEV)).toThrow('use ft-<task>')
  })

  test('is not an existing database', () => {
    const s = localServer('ft-x')
    expect(s.existing).toBe(false)
    expect(s.cliServer).toBe('local')
  })
})

describe('cliServerFor', () => {
  test('names maincloud and the local server by their CLI nicknames, others by URL', () => {
    expect(cliServerFor(MAINCLOUD)).toBe('maincloud')
    expect(cliServerFor(`${MAINCLOUD}/`)).toBe('maincloud')
    expect(cliServerFor('http://127.0.0.1:3100')).toBe('local')
    expect(cliServerFor('https://example.org')).toBe('https://example.org')
  })
})

describe('parseTarget', () => {
  test('without flags: the local server and the default name, published by the script', () => {
    expect(parseTarget([], 'ft-s4-test')).toEqual({ existing: false, url: 'http://127.0.0.1:3100', db: 'ft-s4-test' })
  })

  test('--db and --url without --existing stay local mode', () => {
    expect(parseTarget(['--db', 'ft-a', '--url', 'http://localhost:3000'], 'ft-b')).toEqual({ existing: false, url: 'http://localhost:3000', db: 'ft-a' })
  })

  test('--existing with --url and --db targets that database', () => {
    expect(parseTarget(['--url', MAINCLOUD, '--db', DEV, '--existing'], 'ft-b')).toEqual({ existing: true, url: MAINCLOUD, db: DEV })
  })

  test('--existing needs --url and --db, never the defaults', () => {
    expect(() => parseTarget(['--existing', '--db', DEV], 'ft-b')).toThrow('--existing needs --url')
    expect(() => parseTarget(['--existing', '--url', MAINCLOUD], 'ft-b')).toThrow('--existing needs --db')
  })

  test('--existing refuses --keep, which only applies to a database the script publishes', () => {
    expect(() => parseTarget(['--existing', '--url', MAINCLOUD, '--db', DEV, '--keep'], 'ft-b')).toThrow('--keep')
  })

  test('a flag value cannot be another flag', () => {
    expect(() => parseTarget(['--existing', '--url', '--db', DEV], 'ft-b')).toThrow('--existing needs --url')
  })
})
