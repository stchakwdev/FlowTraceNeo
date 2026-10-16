// Where the books live. The demo backend keeps them in memory; the
// SpacetimeDB backend keeps them in a SpacetimeDB module whose reducers run
// the same engine (see ../../../../spacetimedb). Pages only see this.
import type { CommandResult, LedgerCommand } from './commands'
import type { Actor, LedgerState } from './types'

export interface LedgerBackend {
  readonly kind: 'demo' | 'spacetimedb'
  /** The organisation's full ledger state. */
  load: (organizationId: string) => Promise<LedgerState>
  /**
   * Applies one command. The demo backend trusts `actor`; SpacetimeDB
   * ignores it and derives the actor from the caller's identity.
   */
  run: (
    organizationId: string,
    cmd: LedgerCommand,
    actor: Actor,
  ) => Promise<CommandResult>
  /** Called with fresh state whenever the books change elsewhere. */
  subscribe?: (
    organizationId: string,
    onChange: (state: LedgerState) => void,
  ) => () => void
}
