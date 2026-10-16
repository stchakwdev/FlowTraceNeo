// Asserts that the fixtures cover the TS ledger engine. Everything required
// is enumerated from the TS sources and their exports, never from a list kept
// here, so a new command, report or method in flowtrace/src/lib/ledger fails
// this check until a fixture covers it.
//
// Usage: bun coverage.ts [fixtures-dir]   (default: rust/fixtures)
import fs from 'node:fs'
import path from 'node:path'
import { FIXTURES_DIR, LEDGER_DIR, ledgerModule } from './src/paths'

/** Exports deliberately not fixtured, each with the reason. */
export const EXCLUDED_EXPORTS: Record<string, string> = {
  defaultEnv: 'wall clock and random UUIDs; every fixture injects a frozen LedgerEnv instead (engine.ts:46)',
  Ledger: 'the class itself; each of its public methods is checked one by one below',
}

/** Test blocks with rejections that are property tests (C7 ports them; fast-check inputs are random). */
const PROPERTY_MARKER = 'fc.assert'

const TEST_FILES = ['engine.test.ts', 'reconcile.test.ts', 'review-regressions.test.ts']

export interface CoverageSummary {
  fixtureFiles: number
  commandTypes: number
  reportFunctions: number
  ledgerMethods: number
  rejectedCases: number
  rejectionTests: number
}

const kebab = (s: string) => s.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()

/** The command types `sanitizeCommand` accepts, read from commands.ts. */
export function commandTypesFromSource(): Array<string> {
  const src = fs.readFileSync(ledgerModule('commands'), 'utf8')
  const set = /const COMMAND_TYPES = new Set\(\[([\s\S]*?)\]\)/.exec(src)
  if (!set) throw new Error('COMMAND_TYPES not found in commands.ts')
  return [...set[1].matchAll(/'([^']+)'/g)].map((m) => m[1])
}

function unionTypesFromSource(): Array<string> {
  const src = fs.readFileSync(ledgerModule('commands'), 'utf8')
  const union = /export type LedgerCommand =([\s\S]*?)\n\nexport type CommandResult/.exec(src)
  if (!union) throw new Error('LedgerCommand union not found in commands.ts')
  return [...union[1].matchAll(/type: '([^']+)'/g)].map((m) => m[1])
}

/** Methods engine.ts declares `private`; the rest of Ledger.prototype is public API. */
function privateMethodsFromSource(): Set<string> {
  const src = fs.readFileSync(ledgerModule('engine'), 'utf8')
  return new Set([...src.matchAll(/^\s+private (\w+)\(/gm)].map((m) => m[1]))
}

interface TestBlock {
  file: string
  title: string
  rejections: number
  property: boolean
}

/** Every `it(...)` block of the TS tests, with how many rejections it asserts. */
export function testBlocks(): Array<TestBlock> {
  const out: Array<TestBlock> = []
  for (const file of TEST_FILES) {
    const src = fs.readFileSync(path.join(LEDGER_DIR, file), 'utf8')
    const starts = [...src.matchAll(/^\s*it\('((?:[^'\\]|\\.)*)', /gm)]
    starts.forEach((m, i) => {
      const end = i + 1 < starts.length ? starts[i + 1].index : src.length
      const body = src.slice(m.index, end)
      out.push({
        file,
        title: m[1],
        rejections: [...body.matchAll(/\bcodes?\(\(\) =>/g)].length,
        property: body.includes(PROPERTY_MARKER),
      })
    })
  }
  return out
}

type Json = null | boolean | number | string | Array<Json> | { [k: string]: Json }

function readFixtures(dir: string) {
  if (!fs.existsSync(dir)) return new Map<string, Record<string, Json>>()
  const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json')).sort()
  return new Map(files.map((f) => [f, JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8')) as Record<string, Json>]))
}

export async function checkCoverage(dir = FIXTURES_DIR): Promise<{ problems: Array<string>; summary: CoverageSummary }> {
  const problems: Array<string> = []
  const fixtures = readFixtures(dir)
  if (fixtures.size === 0) problems.push(`no fixture files in ${dir}`)

  // Every fixture names itself and what it covers.
  const covered = new Set<string>()
  for (const [file, f] of fixtures) {
    if (f.fixture !== file.replace(/\.json$/, '')) problems.push(`${file}: "fixture" must be "${file.replace(/\.json$/, '')}"`)
    if (!Array.isArray(f.covers) || f.covers.length === 0) problems.push(`${file}: "covers" must list what it covers`)
    else for (const c of f.covers) covered.add(String(c))
    if (!Array.isArray(f.sources) || f.sources.length === 0) problems.push(`${file}: "sources" must cite the TS files it came from`)
  }

  // 1. Every command type, accepted at least once.
  const types = commandTypesFromSource()
  const unionTypes = unionTypesFromSource()
  const missingFromUnion = types.filter((t) => !unionTypes.includes(t))
  const missingFromSet = unionTypes.filter((t) => !types.includes(t))
  if (missingFromUnion.length || missingFromSet.length)
    problems.push(`commands.ts: COMMAND_TYPES and LedgerCommand disagree (${[...missingFromUnion, ...missingFromSet].join(', ')})`)
  const { sanitizeCommand } = await import(ledgerModule('commands'))
  for (const t of types) {
    try {
      sanitizeCommand({ type: t })
    } catch (e) {
      if (e instanceof Error && e.message === 'Unknown command.') problems.push(`sanitizeCommand rejects listed type ${t}`)
    }
  }
  const accepted = fixtures.get('commands-accepted.json')
  const acceptedTypes = new Set<string>()
  for (const s of (accepted?.steps as Array<Record<string, Json>> | undefined) ?? []) {
    const cmd = s.command as Record<string, Json> | undefined
    if (cmd && s.outcome === 'ok' && typeof cmd.type === 'string') acceptedTypes.add(cmd.type)
  }
  for (const t of types) if (!acceptedTypes.has(t)) problems.push(`command ${t}: no accepted case in commands-accepted.json`)

  // 2. Every report function, one fixture file each.
  const reports = await import(ledgerModule('reports'))
  const reportFns = Object.keys(reports).filter((k) => typeof reports[k] === 'function').sort()
  for (const fn of reportFns) {
    const file = `report-${kebab(fn)}.json`
    const f = fixtures.get(file)
    if (!f) problems.push(`report ${fn}: missing ${file}`)
    else if (!Array.isArray(f.cases) || f.cases.length === 0) problems.push(`report ${fn}: ${file} has no cases`)
  }

  // 3. Every export of the ledger index, by name, in some fixture's "covers".
  const index = await import(ledgerModule('index'))
  for (const name of Object.keys(index).sort()) {
    if (name in EXCLUDED_EXPORTS) continue
    if (!covered.has(name)) problems.push(`export ${name}: not in any fixture's "covers"`)
  }

  // 4. Every public Ledger method.
  const privates = privateMethodsFromSource()
  const methods = Object.getOwnPropertyNames(index.Ledger.prototype)
    .filter((m) => m !== 'constructor' && !privates.has(m))
    .sort()
  for (const m of methods) if (!covered.has(`Ledger.${m}`)) problems.push(`method Ledger.${m}: not in any fixture's "covers"`)

  // 5. Every rejection asserted in the TS tests, case for case.
  const rejected = fixtures.get('rejected-cases.json')
  const scenarios = (rejected?.scenarios as Array<Record<string, Json>> | undefined) ?? []
  let rejectedCases = 0
  for (const s of scenarios) {
    const steps = (s.steps as Array<Record<string, Json>> | undefined) ?? []
    for (const st of steps) {
      if (st.outcome !== 'rejected') continue
      rejectedCases++
      const err = st.error as Record<string, Json> | undefined
      if (!err || typeof err.message !== 'string' || (err.name === 'LedgerError' && typeof err.code !== 'string'))
        problems.push(`rejected-cases.json: ${String(s.test)} has a rejected step without an error code and message`)
    }
  }
  const blocks = testBlocks().filter((b) => b.rejections > 0 && !b.property)
  for (const b of blocks) {
    const s = scenarios.find((x) => x.file === b.file && x.test === b.title)
    if (!s) {
      problems.push(`${b.file} "${b.title}": no scenario in rejected-cases.json`)
      continue
    }
    const n = ((s.steps as Array<Record<string, Json>>) ?? []).filter((x) => x.outcome === 'rejected').length
    if (n !== b.rejections) problems.push(`${b.file} "${b.title}": ${b.rejections} rejections in the test, ${n} in the fixture`)
  }

  // 6. README: one row per fixture file, and no row for a file that is gone.
  const readme = path.join(dir, 'README.md')
  if (!fs.existsSync(readme)) problems.push(`${readme} is missing`)
  else {
    const text = fs.readFileSync(readme, 'utf8')
    const rows = new Set([...text.matchAll(/^\| `([^`]+\.json)` \|/gm)].map((m) => m[1]))
    for (const f of fixtures.keys()) if (!rows.has(f)) problems.push(`README.md: no table row for ${f}`)
    for (const r of rows) if (!fixtures.has(r)) problems.push(`README.md: row for missing file ${r}`)
  }

  return {
    problems,
    summary: {
      fixtureFiles: fixtures.size,
      commandTypes: types.length,
      reportFunctions: reportFns.length,
      ledgerMethods: methods.length,
      rejectedCases,
      rejectionTests: blocks.length,
    },
  }
}

if (import.meta.main) {
  const dir = path.resolve(process.argv[2] ?? FIXTURES_DIR)
  const { problems, summary } = await checkCoverage(dir)
  if (problems.length) {
    console.error(`coverage: ${problems.length} problem(s) in ${dir}`)
    for (const p of problems) console.error(`  - ${p}`)
    process.exit(1)
  }
  console.log(
    `coverage: ${summary.fixtureFiles} fixture files cover ${summary.commandTypes} command types, ` +
      `${summary.reportFunctions} report functions, ${summary.ledgerMethods} public Ledger methods and ` +
      `${summary.rejectedCases} rejected cases from ${summary.rejectionTests} TS tests`,
  )
}
