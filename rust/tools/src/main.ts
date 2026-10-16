// Fixture export entry point (runs in the child process set up by
// export-fixtures.ts). Not implemented yet: the determinism check and the
// coverage assertions come first and fail until it is.
import fs from 'node:fs'
import path from 'node:path'
import { FIXTURES_DIR } from './paths'

const outArg = process.argv.indexOf('--out')
const outDir = path.resolve(outArg === -1 ? FIXTURES_DIR : process.argv[outArg + 1])
fs.mkdirSync(outDir, { recursive: true })
