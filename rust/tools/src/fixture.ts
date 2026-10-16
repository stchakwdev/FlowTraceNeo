// Writing one fixture file. Every file carries its own name, the TS sources
// that produced it, the exports and Ledger methods it covers (checked by
// coverage.ts) and the frozen environment it ran under.
import fs from 'node:fs'
import path from 'node:path'
import { toJsonText } from './canonical'

export interface FrozenEnv {
  now: string
  new_id: string
  defined_at: string
}

export interface FixtureBody {
  description: string
  sources: Array<string>
  covers: Array<string>
  env?: FrozenEnv | Record<string, FrozenEnv> | null
  [key: string]: unknown
}

export const written: Array<{ file: string; bytes: number }> = []

export function writeFixture(outDir: string, name: string, body: FixtureBody) {
  const text = toJsonText({ fixture: name, ...body, covers: [...new Set(body.covers)].sort() })
  const file = `${name}.json`
  fs.writeFileSync(path.join(outDir, file), text)
  written.push({ file, bytes: Buffer.byteLength(text) })
}

/** A LedgerEnv with a frozen clock and a counter for ids, and its description. */
export function frozenEnv(now: string, idOf: (n: number) => string, newIdDoc: string, definedAt: string) {
  let n = 0
  return {
    env: { now: () => now, newId: () => idOf(++n) },
    doc: { now, new_id: newIdDoc, defined_at: definedAt } satisfies FrozenEnv,
  }
}
