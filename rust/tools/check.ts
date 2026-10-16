// Determinism and coverage check for the parity fixtures.
//
// 1. Runs the exporter twice, each in its own process, into two folders
//    under rust/out/, and compares them byte for byte.
// 2. Compares that output with the committed rust/fixtures (stale fixtures
//    fail: regenerate with `bun run export`).
// 3. Runs the coverage assertions (coverage.ts) over rust/fixtures.
//
// Prints the fixture count and "byte-identical" on success, exits 1 otherwise.
// The run folders are removed on success and kept on failure for diffing.
import fs from 'node:fs'
import path from 'node:path'
import { checkCoverage } from './coverage'
import { FIXTURES_DIR, OUT_DIR, TOOLS_DIR } from './src/paths'

const runRoot = path.join(OUT_DIR, 'fixture-check')

function fail(msg: string): never {
  console.error(`check: FAILED: ${msg}`)
  console.error(`check: run output kept in ${path.relative(process.cwd(), runRoot) || runRoot}`)
  process.exit(1)
}

function exportInto(dir: string): number {
  const started = performance.now()
  const r = Bun.spawnSync([process.execPath, '--no-install', path.join(TOOLS_DIR, 'export-fixtures.ts'), '--out', dir], {
    cwd: TOOLS_DIR,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  if (r.exitCode !== 0) {
    process.stderr.write(r.stdout)
    process.stderr.write(r.stderr)
    fail(`exporter exited with ${r.exitCode} writing ${dir}`)
  }
  return performance.now() - started
}

const jsonFiles = (dir: string) =>
  fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith('.json')).sort() : []

/** The first difference between two fixture folders, or null when byte-identical. */
function compareDirs(a: string, b: string): string | null {
  const fa = jsonFiles(a)
  const fb = jsonFiles(b)
  const onlyA = fa.filter((f) => !fb.includes(f))
  const onlyB = fb.filter((f) => !fa.includes(f))
  if (onlyA.length || onlyB.length)
    return `file sets differ: only in ${a}: [${onlyA.join(', ')}]; only in ${b}: [${onlyB.join(', ')}]`
  for (const f of fa) {
    const x = fs.readFileSync(path.join(a, f))
    const y = fs.readFileSync(path.join(b, f))
    if (x.equals(y)) continue
    const lx = x.toString('utf8').split('\n')
    const ly = y.toString('utf8').split('\n')
    const i = lx.findIndex((l, n) => l !== ly[n])
    const at = i === -1 ? lx.length : i
    return `${f} differs at line ${at + 1}:\n  ${a}: ${lx[at] ?? '<eof>'}\n  ${b}: ${ly[at] ?? '<eof>'}`
  }
  return null
}

const started = performance.now()
fs.rmSync(runRoot, { recursive: true, force: true })
const runA = path.join(runRoot, 'run-a')
const runB = path.join(runRoot, 'run-b')
const msA = exportInto(runA)
const msB = exportInto(runB)
const count = jsonFiles(runA).length
if (count === 0) fail('the exporter wrote no fixture files')

const twice = compareDirs(runA, runB)
if (twice) fail(`two runs of the exporter differ: ${twice}`)

const committed = compareDirs(runA, FIXTURES_DIR)
if (committed) fail(`rust/fixtures is stale (regenerate with \`bun run export\`): ${committed}`)

const { problems, summary } = await checkCoverage(FIXTURES_DIR)
if (problems.length) fail(`coverage:\n  - ${problems.join('\n  - ')}`)

fs.rmSync(runRoot, { recursive: true, force: true })
if (fs.existsSync(OUT_DIR) && fs.readdirSync(OUT_DIR).length === 0) fs.rmSync(OUT_DIR, { recursive: true })

const secs = (ms: number) => `${(ms / 1000).toFixed(2)}s`
console.log(
  `check: ${count} fixture files, byte-identical across two runs and with rust/fixtures ` +
    `(export ${secs(msA)} and ${secs(msB)}, total ${secs(performance.now() - started)})`,
)
console.log(
  `check: coverage ok: ${summary.commandTypes} command types, ${summary.reportFunctions} report functions, ` +
    `${summary.ledgerMethods} public Ledger methods, ${summary.rejectedCases} rejected cases from ` +
    `${summary.rejectionTests} TS tests`,
)
