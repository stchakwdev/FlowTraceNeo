// Every path the fixture tools use, resolved from this file so the tools run
// the same from any working directory.
import path from 'node:path'

export const TOOLS_DIR = path.resolve(import.meta.dir, '..')
export const RUST_DIR = path.resolve(TOOLS_DIR, '..')
export const REPO_DIR = path.resolve(RUST_DIR, '..')
export const FIXTURES_DIR = path.join(RUST_DIR, 'fixtures')
export const OUT_DIR = path.join(RUST_DIR, 'out')
export const LEDGER_DIR = path.join(REPO_DIR, 'flowtrace', 'src', 'lib', 'ledger')
export const DEMO_DIR = path.join(REPO_DIR, 'flowtrace', 'src', 'lib', 'demo')
export const TOOLS_NODE_MODULES = path.join(TOOLS_DIR, 'node_modules')

export const ledgerModule = (name: string) => path.join(LEDGER_DIR, `${name}.ts`)
export const demoModule = (name: string) => path.join(DEMO_DIR, `${name}.ts`)
