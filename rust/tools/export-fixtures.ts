// Exports the parity fixtures for the Rust ledger-core from the TS ledger
// engine in flowtrace/src/lib/ledger and the demo books in
// flowtrace/src/lib/demo/ledger.ts. Nothing under flowtrace/ is changed.
//
// Usage: bun export-fixtures.ts [--out <dir>]   (default: rust/fixtures)
//
// The work runs in a child process with a fixed environment:
// - NODE_PATH points at this folder's node_modules, so any package the
//   flowtrace modules pull in resolves to the version pinned here and is
//   never auto-installed;
// - TZ=UTC and LC_ALL=C.UTF-8, and no VITE_* variables, so nothing in the
//   output can depend on the machine.
import path from 'node:path'
import { TOOLS_NODE_MODULES } from './src/paths'

const CHILD = 'FLOWTRACE_FIXTURE_EXPORT_CHILD'

if (process.env[CHILD] === '1') {
  await import('./src/main')
} else {
  const env: Record<string, string> = {}
  for (const [k, v] of Object.entries(process.env)) if (v !== undefined && !k.startsWith('VITE_')) env[k] = v
  Object.assign(env, { NODE_PATH: TOOLS_NODE_MODULES, TZ: 'UTC', LC_ALL: 'C.UTF-8', LANG: 'C.UTF-8', [CHILD]: '1' })
  const child = Bun.spawnSync([process.execPath, '--no-install', path.resolve(import.meta.path), ...process.argv.slice(2)], {
    env,
    stdout: 'inherit',
    stderr: 'inherit',
  })
  process.exit(child.exitCode ?? 1)
}
