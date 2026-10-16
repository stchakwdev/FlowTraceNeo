// The archive name release.sh promises (rust/README.md, "Release build"):
// flowtrace-books-<version>-<os>-<arch>.tar.gz, the version being the one crates/desktop/Cargo.toml
// inherits from the workspace. `--name` builds nothing, so this stays fast and offline.
// `--check <archive>` runs the script's archive content check alone, so the tests below pack
// small archives with a stand-in binary: the archive must carry crates/desktop/release/config.toml
// unchanged, and that file must name the production database.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { arch, platform, tmpdir } from 'node:os'
import { join } from 'node:path'

const rust = join(import.meta.dir, '..', '..')
const script = join(rust, 'tools', 'release.sh')
const releaseConfig = join(rust, 'crates', 'desktop', 'release', 'config.toml')

function run(...args: Array<string>) {
  const out = Bun.spawnSync(['bash', script, ...args], { stdout: 'pipe', stderr: 'pipe' })
  return { code: out.exitCode, stdout: out.stdout.toString(), stderr: out.stderr.toString() }
}

function workspaceVersion(): string {
  const desktop = readFileSync(join(rust, 'crates', 'desktop', 'Cargo.toml'), 'utf8')
  expect(desktop).toContain('version.workspace = true')
  const workspace = readFileSync(join(rust, 'Cargo.toml'), 'utf8')
  const pkg = workspace.slice(workspace.indexOf('[workspace.package]'))
  const version = /^version = "([^"]+)"$/m.exec(pkg)?.[1]
  if (version === undefined) throw new Error('no version in [workspace.package]')
  return version
}

describe('release.sh --name', () => {
  test('names the archive after the desktop version, the OS and the machine', () => {
    const os = platform() === 'darwin' ? 'macos' : platform()
    // uname -m spelling: node says x64 and arm64 where Linux says x86_64 and aarch64.
    const machine = { x64: 'x86_64', arm64: platform() === 'darwin' ? 'arm64' : 'aarch64' }[arch()] ?? arch()
    const out = run('--name')
    expect(out.code).toBe(0)
    expect(out.stdout).toBe(`flowtrace-books-${workspaceVersion()}-${os}-${machine}.tar.gz\n`)
  })

  test('refuses any other argument without building', () => {
    const out = run('--publish')
    expect(out.code).toBe(2)
    expect(out.stderr).toContain('unknown argument --publish')
  })
})

describe('the shipped config.toml', () => {
  test('names the production database on maincloud and no token file', () => {
    expect(readFileSync(releaseConfig, 'utf8')).toBe(
      'server = "https://maincloud.spacetimedb.com"\ndatabase = "flowtrace-ledger"\n',
    )
  })
})

describe('release.sh --check', () => {
  // Packs <name>/flowtrace-books, INSTALL.txt and (unless `config` is null) config.toml the way
  // release.sh does, and runs the check on it.
  function check(config: string | null) {
    const name = run('--name').stdout.trim().replace(/\.tar\.gz$/, '')
    const dir = mkdtempSync(join(tmpdir(), 'ft-release-check-'))
    try {
      const stage = join(dir, name)
      mkdirSync(stage)
      writeFileSync(join(stage, 'flowtrace-books'), '#!/bin/sh\n')
      writeFileSync(join(stage, 'INSTALL.txt'), 'FlowTrace Books\n')
      if (config !== null) writeFileSync(join(stage, 'config.toml'), config)
      const archive = join(dir, `${name}.tar.gz`)
      const tar = Bun.spawnSync(['tar', '-C', dir, '-czf', archive, name])
      expect(tar.exitCode).toBe(0)
      return run('--check', archive)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  }

  test('passes an archive with the binary, INSTALL.txt and the shipped config.toml', () => {
    const out = check(readFileSync(releaseConfig, 'utf8'))
    expect(out.stderr).toBe('')
    expect(out.code).toBe(0)
  })

  test('fails an archive without config.toml', () => {
    const out = check(null)
    expect(out.code).toBe(1)
    expect(out.stderr).toContain('archive holds')
  })

  test('fails an archive whose config.toml names another database', () => {
    const out = check('server = "https://maincloud.spacetimedb.com"\ndatabase = "flowtrace-ledger-dev"\n')
    expect(out.code).toBe(1)
    expect(out.stderr).toContain('config.toml differs')
  })

  test('fails when the archive is missing', () => {
    const out = run('--check', join(tmpdir(), 'ft-release-check-absent.tar.gz'))
    expect(out.code).toBe(1)
  })
})
