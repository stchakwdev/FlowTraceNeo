# ledger-module

The FlowTrace ledger as a SpacetimeDB 2.12 server module, the Rust form of
`spacetimedb/src/index.ts`. It has the TS module's twenty private tables column for
column, the nineteen `my_*` views, `init`, `add_operator`, `remove_operator`,
`create_org_ledger`, `add_member`, `remove_member` and `run_command`. `run_command`
checks membership and JSON, then answers "not implemented yet"; task S4 runs the
command through ledger-core. Why the tables look the way they do:
`../../docs/adr/0003-module-tables.md`.

## Checks

The crate is a workspace member, so the workspace checks cover it on the host target:

```sh
cd rust
cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked
cargo test -p ledger-module          # the module's 74 host tests alone
```

## Build the wasm module

Either command, from `rust/`. Both write `target/wasm32-unknown-unknown/release/ledger_module.wasm`.

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
~/.local/bin/spacetime build -p crates/ledger-module
```

`spacetime build` runs the same cargo build, then looks for `wasm-opt`. It is not installed
on Omarchy, so the module is published unoptimised.

Measured on Omarchy (16 threads, Rust 1.99.0), 2026-10-16:

| Measure | Value |
|---|---|
| Release wasm build, cold for the wasm target (host deps already built) | 11.4 s |
| Same, after touching `src/lib.rs` | 1.4 s |
| `spacetime build -p crates/ledger-module` with the build already done | 0.2 s |
| `ledger_module.wasm`, release, no `wasm-opt` | 558,411 bytes (545 KiB) |

## Publish, call, inspect and delete (local server only)

Only ever the local server (`local`, `http://127.0.0.1:3100`), and only with a database name
that carries your task id. Never maincloud. Never stop or restart the service. Delete what
you publish. The commands below use `ft-s1-scaffold`; change the name to your own task's.

```sh
S=~/.local/bin/spacetime   # the CLI is not on the non-interactive PATH
DB=ft-s1-scaffold

$S publish -s local -b target/wasm32-unknown-unknown/release/ledger_module.wasm -y $DB
$S sql -s local $DB "SELECT * FROM operator"          # init made the publisher an operator

# Arguments are JSON: strings carry their own quotes, identities are "0x..." hex strings.
$S call -s local $DB create_org_ledger '"org-s1"' '"KES"' 2026 '"Ama Owner"'
$S call -s local --anonymous $DB create_org_ledger '"org-x"' '"KES"' 2026 '"Anon"'   # rejected
$S call -s local $DB add_member '"org-s1"' '"0x00000000000000000000000000000000000000000000000000000000000000c1"' '"clerk-1"' '"Kofi Clerk"' true false false
$S call -s local $DB remove_member '"org-s1"' '"0x00000000000000000000000000000000000000000000000000000000000000c1"'
$S call -s local $DB add_operator '"0x...64 hex digits"'
$S call -s local $DB remove_operator '"0x...64 hex digits"'
$S call -s local $DB run_command '"org-s1"' '"req-1"' '"{\"type\":\"create_fund\",\"fund\":{}}"'

# The owner reads private tables and views; --anonymous is a caller who is a member of nothing.
$S sql -s local $DB "SELECT org_id, user_id, name, can_close FROM member"
$S sql -s local $DB "SELECT seq, data FROM audit_event"
$S sql -s local $DB "SELECT org_id, user_id FROM my_member"
$S sql -s local --anonymous $DB "SELECT org_id, user_id FROM my_member"   # no rows
$S logs -s local -n 50 $DB                             # reducer errors show as ERROR lines

$S delete -s local -y $DB
$S list -s local                                       # must not list $DB any more
```

A rejected call exits 1 with `Error: Response text: <the TS message>`. `-y` takes its
value only through `=`, so `-y $DB` still reads `$DB` as the database.
