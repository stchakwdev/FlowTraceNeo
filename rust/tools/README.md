# rust/tools

Bun scripts around the Rust workspace: the parity fixtures from the TS ledger engine and the
ledger-module checks on the local SpacetimeDB server (or, with `--existing`, on a published
database such as maincloud dev). bun is not on the non-interactive PATH:
use `~/.bun/bin/bun`. Install once with `~/.bun/bin/bun install --frozen-lockfile`
(auto-install is off, `bunfig.toml`).

| Script | What it does | Details |
|---|---|---|
| `bun run export` | Writes `rust/fixtures/*.json` from the TS engine in `flowtrace/src/lib/ledger`, and the transparency fixtures from the demo dataset and demo API in `flowtrace/src/lib/demo` | `rust/fixtures/README.md` |
| `bun run check` | Exports twice into `rust/out/`, requires both runs and `rust/fixtures` byte-identical, then runs `coverage` | `rust/fixtures/README.md` |
| `bun run coverage` | The fixture coverage assertions alone | `coverage.ts` |
| `bun run module-replay` | Replays the 37-command fixture through the module on the local server and compares every result and the final books (needs the wasm built) | `rust/crates/ledger-module/README.md` |
| `bun run integration` | Builds, publishes `ft-s2-<timestamp>`, loads the demo books and command log, checks rows, balances, trial balance, refusals, views and a repeated request id with SQL, deletes the database; under five minutes | `rust/crates/ledger-module/README.md#integration` |
| `bun run transparency-proof` | MOD1 and MOD2 on the local server: publishes `ft-mod2-<stamp>` (over main's module with `--base-wasm`), loads the demo books and the transparency fixtures, runs all 19 MOD1 and 23 MOD2 reducers (a service among them, writing over HTTP as itself), reads back through SQL, the public tables with no `Authorization` header, and ledger-client as a member and anonymously, deletes the database | `rust/crates/ledger-module/README.md` |
| `bun run auth-proof` | Starts two test OpenID Connect issuers on 127.0.0.1, publishes `ft-auth1-<timestamp>`, trusts one, and shows over WebSocket and HTTP: one identity per subject, untrusted issuer and audience refused, device identities on and off, a registered service let in either way, a member added by subject; deletes the database (needs the wasm built and `bun install` in `flowtrace/`) | `rust/docs/module-release.md` section 5 |
| `bun run it1` | IT1: two people on the web (Playwright) and the desktop against a fresh `it1-<stamp>` organisation on maincloud dev (`--url`, `--db` ending in `-dev`) or the local server (`--local`): entries, cross approval of claims and bills, statement import and Auto-match, refusals on both clients, an offline claim, and the five reports from the web DOM, the desktop and a ledger-core replay of the run's commands, required byte-identical | `../docs/integration-test.md` |
| `bun test` | Unit tests of the module tools, offline | `src/*.test.ts` |

`bun test` runs the offline unit tests in `src/*.test.ts` (the remote-mode refusals and flags).

CI runs `bun test` and then `bun run check` in the `fixtures` job of `.github/workflows/rust.yml`;
on a drift, run `bun run export` here and commit the changed `rust/fixtures/*.json`.

## Remote mode (an existing database, gate G3)

`module-replay` and `integration` can run against a database that is already published, for
example `flowtrace-ledger-dev` on maincloud:

```sh
cd rust/tools
~/.bun/bin/bun run integration --existing --url https://maincloud.spacetimedb.com --db flowtrace-ledger-dev
~/.bun/bin/bun run module-replay --existing --url https://maincloud.spacetimedb.com --db flowtrace-ledger-dev
```

**Warning: each run writes a demo organisation (about 450 rows) into whatever database it is
pointed at, and leaves it there.** Never point it at a production database.

- `--existing` needs both `--url` and `--db`, refuses `--keep`, skips the build, and never
  publishes, deletes, renames or lists: `remoteServer` in `src/module-server.ts` throws on those,
  and its CLI wrapper only runs `call` and `sql` on that server and database.
- The CLI login must be an operator of the database (its publisher is, through `init`): it adds
  the run's HTTP operator with `add_operator` and withdraws it with `remove_operator` at the end.
- Each run opens a new organisation with a random id (printed as `organisation for this run`);
  every count and view check is scoped to it. The `init` check becomes "the owner is an
  operator", since other runs may have used the database.
- The integration run ends with the `run_command` round-trip times and the rows it left.
- The client side is `rust/crates/ledger-client/tests/live.rs` with
  `LEDGER_CLIENT_LIVE_EXISTING=1`; see its doc comment.

Shared code lives in `src/`: `module-server.ts` (CLI, HTTP and SQL against the local server,
or calls and reads only on an existing database; rows back to books through the TS client codec) and `demo-replay.ts` (the demo
organisation, `import_books`, the command log and the fixture-to-module id map) for the two
module scripts; the rest is the fixture exporter.
