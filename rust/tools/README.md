# rust/tools

Bun scripts around the Rust workspace: the parity fixtures from the TS ledger engine and the
ledger-module checks on the local SpacetimeDB server. bun is not on the non-interactive PATH:
use `~/.bun/bin/bun`. Install once with `~/.bun/bin/bun install --frozen-lockfile`
(auto-install is off, `bunfig.toml`).

| Script | What it does | Details |
|---|---|---|
| `bun run export` | Writes `rust/fixtures/*.json` from the TS engine in `flowtrace/src/lib/ledger` | `rust/fixtures/README.md` |
| `bun run check` | Exports twice into `rust/out/`, requires both runs and `rust/fixtures` byte-identical, then runs `coverage` | `rust/fixtures/README.md` |
| `bun run coverage` | The fixture coverage assertions alone | `coverage.ts` |
| `bun run module-replay` | Replays the 37-command fixture through the module on the local server and compares every result and the final books (needs the wasm built) | `rust/crates/ledger-module/README.md` |
| `bun run integration` | Builds, publishes `ft-s2-<timestamp>`, loads the demo books and command log, checks rows, balances, trial balance, refusals, views and a repeated request id with SQL, deletes the database; under five minutes | `rust/crates/ledger-module/README.md#integration` |

Shared code lives in `src/`: `module-server.ts` (CLI, HTTP and SQL against the local server
only, rows back to books through the TS client codec) and `demo-replay.ts` (the demo
organisation, `import_books`, the command log and the fixture-to-module id map) for the two
module scripts; the rest is the fixture exporter.
