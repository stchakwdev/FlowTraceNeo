# FlowTrace Books (Rust)

FlowTrace Books is a native desktop bookkeeping app for NGOs: chart of accounts and periods,
journal, general ledger, expense claims and bills, grants and budgets, bank and mobile-money
reconciliation, and financial statements. It is written in Rust with GPUI (through gpui-kit),
and keeps its books either in memory (the demo) or in a SpacetimeDB database on maincloud.

This folder is a Cargo workspace. The web app in `../flowtrace/` is the reference the Rust
ledger was built to match, and is never edited from here.

Every command below runs from this folder (`rust/`) unless it says otherwise.

## The crates

| Crate | What it is | Read next |
|---|---|---|
| `crates/ledger-core` | The ledger engine: money, accounts, entries, posting rules, commands, reports, reconciliation, importers, demo books. Pure Rust, no I/O, no clock or random ids of its own (they come in through `LedgerEnv`). At parity with the TypeScript engine in `flowtrace/src/lib/ledger/`. | `docs/parity.md`, `crates/ledger-core/tests/README.md` |
| `crates/ledger-module` | The SpacetimeDB 2.12 server module: tables, `my_*` views and reducers; every write is `run_command`, which runs ledger-core. Builds to wasm for the server and as a normal library for the client. | `crates/ledger-module/README.md`, `docs/module-release.md` |
| `crates/ledger-client` | The SpacetimeDB client: connects, subscribes, rebuilds each organisation's books from the rows, submits commands, keeps the token file. | `crates/ledger-client/README.md` |
| `crates/desktop` | The app, FlowTrace Books: the shell, seven screens, and the `LedgerBackend` trait with an in-process backend (demo) and a SpacetimeDB backend (live). | `crates/desktop/README.md` |

`tools/` holds the bun scripts that export the parity fixtures from the TypeScript engine and
check the module on a server, plus `release.sh`. `fixtures/` holds the exported fixtures.

## Before you start

- Rust comes from `rust-toolchain.toml` (1.99.0, with rustfmt and clippy); rustup installs it
  on the first cargo command. `cargo --version` should print 1.99.0 here.
- Linux needs GPUI's build libraries. On Debian or Ubuntu, the list the CI uses
  (`../.github/workflows/rust.yml`):
  `sudo apt-get install clang libfontconfig-dev libwayland-dev libxkbcommon-x11-dev libx11-xcb-dev libssl-dev libzstd-dev`.
  macOS needs the Xcode command line tools.
- A first debug build of the desktop takes about 80 seconds on 16 threads and needs
  about 6.5 GiB of memory (`docs/adr/0002-gpui-spacetime.md`, Measurements); lower `-j` on a
  smaller machine.
- For `tools/`: bun (`~/.bun/bin/bun` if it is not on your PATH). For the module: the
  SpacetimeDB CLI 2.12 (`spacetime --version`), and `rustup target add wasm32-unknown-unknown`.

## Run the demo

```sh
cargo run -p desktop
```

It opens Demo NGO Kenya, demo books built by `ledger_core::demo::build_demo_ledger`, plus an
empty "New organisation", both in memory: anything you post is gone when the window closes.
Keys `1` to `7` switch between Accounts, Journal, General ledger, Spend, Grants, Reconcile and
Reports while no field has focus. A few launch options, used for screenshots
(`crates/desktop/src/main.rs` lists them all):

```sh
cargo run -p desktop -- --theme dark --entry 3        # dark, General ledger
cargo run -p desktop -- --report activities           # Reports, statement of activities
cargo run -p desktop -- --screen connect              # the Connection screen
```

The app opens the demo only when no server is named, by flag or by config file (next
section). If you have a config file and want the demo, point it at an empty config dir:
`XDG_CONFIG_HOME=$(mktemp -d) cargo run -p desktop` on Linux.

## Connect to the live module

The ledger module runs on maincloud as `flowtrace-ledger-dev` (a development database; see
`docs/module-release.md`). Open it with:

```sh
cargo run -p desktop -- --server https://maincloud.spacetimedb.com --database flowtrace-ledger-dev
```

or put the same in the config file, `~/.config/flowtrace-books/config.toml` on Linux and
`~/Library/Application Support/flowtrace-books/config.toml` on macOS:

```toml
server = "https://maincloud.spacetimedb.com"
database = "flowtrace-ledger-dev"     # the default when server is set
token_file = "/path/to/token"         # optional
```

Flags win over the file; a connect from the Connection screen writes `server` and `database`
back. Your identity is a token the server issues on the first connect, kept in
`token-<server host>` next to `config.toml` (or in `token_file`), mode 0600, never printed. Keep
the file to stay the same identity; delete it to become a new one.

A new identity belongs to no organisation, so the Connection screen says so and shows no books.
An operator of the database opens books and adds members with the SpacetimeDB CLI
(`docs/module-release.md`, "Operators and members"). Run with `--timings on` to see your full
identity on stderr. While the server is unreachable the app shows the last books it saw under an
offline notice and refuses writes; try it with:

```sh
cargo run -p desktop -- --server http://127.0.0.1:9 --timings on
```

More on configuration, the token and offline behaviour: `crates/desktop/README.md`,
"Configuration, token file and offline behaviour".

## Run the checks

The same command CI runs (`../.github/workflows/rust.yml`):

```sh
cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked
```

One crate at a time: `cargo test -p ledger-core` (or `ledger-module`, `ledger-client`,
`desktop`). Tests that need a SpacetimeDB server are ignored by default and documented in the
module and client READMEs; with the local server running, the whole module check is:

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
cd tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run integration
```

It publishes a throwaway `ft-s2-<timestamp>` database to the local server, replays the demo books
and the 37-command fixture, checks rows, balances, views and refusals, and deletes it.

## The fixture pipeline

ledger-core is tested against JSON fixtures produced by the TypeScript engine itself, so the two
cannot quietly drift:

1. `tools/export-fixtures.ts` imports the TS engine from `../flowtrace/src/lib/ledger/`, runs
   every case and writes `fixtures/*.json` (27 files; `fixtures/README.md` describes each).
2. `tools/check.ts` exports twice and requires both runs and the committed fixtures to be
   byte-identical, then checks coverage.
3. The Rust tests in `crates/ledger-core/tests/` replay each fixture and compare exactly;
   `parity_table.rs` fails if `docs/parity.md` misses a TS export.

```sh
cd tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run check
```

Run it after any change to the TS engine; commit the changed fixtures with the Rust change that
makes them pass. It is not in CI yet (`docs/handoff.md`).

## Release build

```sh
tools/release.sh
```

builds the desktop in release for this machine, strips it and writes
`dist/flowtrace-books-<version>-<os>-<arch>.tar.gz` (the binary as `flowtrace-books` and an
`INSTALL.txt`) with a `.sha256` next to it, then checks the archive and that the binary starts.
The version is the workspace version that `crates/desktop/Cargo.toml` inherits;
`tools/release.sh --name` prints the archive name without building (`tools/src/release.test.ts`
checks it; `cd tools && ~/.bun/bin/bun test`). It is a shell script rather than a `cargo xtask`
because it only strings cargo, strip and tar together, and an xtask crate would be one more
workspace member for every check to compile.

Measured on Linux (Omarchy, x86_64, 16 threads, Rust 1.99.0), 2026-10-16:

| Measure | Value |
|---|---|
| `tools/release.sh` from an empty `target/` | 2 min 34 s wall (cargo 2 min 32 s), 32 min user CPU |
| Same, release build already done | 2 s |
| `target/release/desktop`, unstripped | 71,137,120 bytes (67.8 MiB) |
| `flowtrace-books` in the archive, stripped | 51,313,936 bytes (48.9 MiB) |
| `flowtrace-books-0.1.0-linux-x86_64.tar.gz` | 19,082,226 bytes (18.2 MiB) |

On the user's MacBook a release build of the same code took 1 min 54 s (Q3 check).

Pushing a tag `v*` runs the same script on Linux and macOS in CI and uploads both archives as
workflow artifacts (job `release` in `../.github/workflows/rust.yml`). It creates no GitHub
Release and publishes nothing.

## Documents

| Document | What it holds |
|---|---|
| `docs/handoff.md` | Where things stand, known gaps and differences from the web app, decisions the user confirmed, next steps |
| `docs/module-release.md` | Building the wasm, how maincloud dev was published, operators and members, what production needs |
| `docs/parity.md` | Every TS ledger export, its Rust counterpart, the test that proves it, known differences |
| `docs/adr/0002-gpui-spacetime.md` | Why GPUI with gpui-kit and the SpacetimeDB client, the store-entity data pattern, build costs |
| `docs/adr/0003-module-tables.md` | Why the module keeps the TS module's row shape |
| `docs/screens/` | Screenshots of every screen (`u1-*` to `u10-*`); the desktop README says which flow each shows |
| `fixtures/README.md` | Each fixture file, the frozen environments, float and formatting rules |
| `../Docs/adr/0001-native-ledger.md`, `../Docs/flowtrace-ledger-plan.md` | The ledger's domain rules, above everything here except the TS engine |
| `../PRIMER.md` | The plan this work followed (section 8) |
