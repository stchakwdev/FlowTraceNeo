// The archive name release.sh promises (rust/README.md, "Release build"):
// flowtrace-books-<version>-<os>-<arch>.tar.gz, the version being the one crates/desktop/Cargo.toml
// inherits from the workspace. `--name` builds nothing, so this stays fast and offline.
//
//   cd rust/tools && ~/.bun/bin/bun test
import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { arch, platform } from 'node:os'
import { join } from 'node:path'

const rust = join(import.meta.dir, '..', '..')
const script = join(rust, 'tools', 'release.sh')

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
