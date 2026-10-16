// Fixture export entry point, run in the child process export-fixtures.ts
// sets up. The guards load first, before any flowtrace module runs.
import './guards'
import fs from 'node:fs'
import path from 'node:path'
import { nonFinite } from './canonical'
import { exportCommands } from './commands'
import { exportDemo } from './demo'
import { written } from './fixture'
import { exportPrimitives } from './primitives'
import { exportQueries } from './queries'
import { exportReconcile } from './reconcile'
import { exportRejected } from './rejected'
import { exportReports } from './reports'
import { exportTransparency } from './transparency'
import { FIXTURES_DIR } from './paths'

const outArg = process.argv.indexOf('--out')
const outDir = path.resolve(outArg === -1 ? FIXTURES_DIR : process.argv[outArg + 1])
fs.mkdirSync(outDir, { recursive: true })
// Regenerate from scratch: a fixture that is no longer produced must not linger.
for (const f of fs.readdirSync(outDir)) if (f.endsWith('.json')) fs.rmSync(path.join(outDir, f))

exportPrimitives(outDir)
const demo = exportDemo(outDir)
const afterCommands = exportCommands(outDir, demo.state)
exportQueries(outDir, demo.state)
exportReports(outDir, { 'demo-books': demo.state, 'commands-final': afterCommands })
exportReconcile(outDir, demo.state, demo.preAutoMatch)
const rejected = exportRejected(outDir)
await exportTransparency(outDir)

// Every timestamp the engine writes (audit `at`, entry `created_at` and
// `posted_at`, approval `at`) and every env `now` must be a frozen now. The
// transparency fixtures carry the demo dataset's own timestamps; the ones the
// demo API writes there come from its frozen clock (transparency.ts).
const FROZEN = new Set(['2026-02-04T09:00:00.000Z', '2026-03-01T00:00:00.000Z'])
let stamps = 0
for (const { file } of written) {
  if (file.startsWith('transparency-')) continue
  const text = fs.readFileSync(path.join(outDir, file), 'utf8')
  for (const m of text.matchAll(/"(at|created_at|posted_at|now)": "([^"]*)"/g)) {
    stamps++
    if (!FROZEN.has(m[2])) throw new Error(`${file}: ${m[1]} ${m[2]} is not a frozen now`)
  }
}

const kb = (n: number) => `${(n / 1024).toFixed(0)} KiB`
const total = written.reduce((n, w) => n + w.bytes, 0)
console.log(`export: ${written.length} fixture files (${kb(total)}) in ${outDir}`)
console.log(`export: ${stamps} timestamps, all frozen`)
console.log(`export: demo backfill ${demo.steps.length} steps; rejected cases ${rejected.total} (${rejected.fromTests} from the TS tests)`)
if (nonFinite.length) console.log(`export: ${nonFinite.length} non-finite numbers written as strings`)
