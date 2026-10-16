// Repo-relative names of the TS sources the fixtures cite.
export const LEDGER = 'flowtrace/src/lib/ledger'
export const DEMO = 'flowtrace/src/lib/demo/ledger.ts'
export const COMMAND_SOURCE = `${LEDGER}/commands.ts`
export const src = (file: string, fn?: string) => (fn ? `${LEDGER}/${file}#${fn}` : `${LEDGER}/${file}`)
