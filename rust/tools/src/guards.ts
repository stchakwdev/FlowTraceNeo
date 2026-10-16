// Determinism guards, installed before any flowtrace module runs. The engine
// takes ids and time from an injected LedgerEnv (engine.ts:41-44); its
// default env reads the wall clock and crypto.randomUUID (engine.ts:46-49).
// Every fixture injects a frozen env, so any read of the clock or of a random
// source means a fixture would differ between runs: fail loudly instead.

const RealDate = Date

class GuardedDate extends RealDate {
  constructor(...args: Array<unknown>) {
    if (args.length === 0) throw new Error('determinism guard: new Date() reads the wall clock; inject a LedgerEnv')
    super(...(args as [string]))
  }
  static override now(): number {
    throw new Error('determinism guard: Date.now() reads the wall clock; inject a LedgerEnv')
  }
}

globalThis.Date = GuardedDate as DateConstructor
Math.random = () => {
  throw new Error('determinism guard: Math.random() is not deterministic')
}
Object.defineProperty(globalThis.crypto, 'randomUUID', {
  value: () => {
    throw new Error('determinism guard: crypto.randomUUID() is not deterministic; inject a LedgerEnv')
  },
})
