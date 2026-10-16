# ledger-module

The FlowTrace ledger as a SpacetimeDB 2.12 server module, the Rust form of
`spacetimedb/src/index.ts`. It has the TS module's twenty private tables column for
column, the nineteen `my_*` views, `init`, `add_operator`, `remove_operator`,
`create_org_ledger`, `add_member`, `remove_member`, `run_command` and `import_books`. Why the
tables look the way they do: `../../docs/adr/0003-module-tables.md`.

## The reducers that write books

- `create_org_ledger(org_id, base_currency, fiscal_year, display_name)`: operator only. Seeds the
  books with ledger-core's `createOrgLedger` (NGO chart of accounts, unrestricted fund, twelve
  periods), writes them as rows from `seq` 0, makes the caller the owner and appends
  `ledger.create` (`index.ts:554-595`).
- `run_command(org_id, request_id, command_json)`: the single ledger write path
  (`index.ts:655-691`). Member check, JSON, `sanitize_command`, load the organisation's rows into
  a `LedgerState` (`codec`), `apply_command` with the member as actor, then write back only what
  changed (`src/write_back.rs`: insert new ids with fresh `seq`, update changed records under
  their `seq`, delete gone ids; posted entries change only by reversal, lines of posted entries
  never, the audit log is append-only) and upsert the caller's `command_result` row
  `{identity, org_id, request_id, result_json, at}`. Ids are `ctx.new_uuid_v4()` and the clock
  is the reducer timestamp in milliseconds, as `envFor` in TS. A refused command writes nothing;
  a ledger rule comes back as `LEDGER_ERROR <issues JSON>`, which the TS client turns back into a
  `LedgerError`. A repeated `request_id` runs again, as in TS.
- `import_books(org_id, state_json)`: operator only, books must exist (`create_org_ledger`
  first) and their settings must name `org_id`. Deletes every ledger row of the organisation
  and writes the given `LedgerState` as new rows, numbered on from `next_seq`; members and
  command results stay. No TS counterpart: it loads the demo books for end-to-end checks.

## Checks

The crate is a workspace member, so the workspace checks cover it on the host target:

```sh
cd rust
cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked
cargo test -p ledger-module          # the module's 103 host tests alone
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
| `ledger_module.wasm`, release, no `wasm-opt`, on main at 4e75c69 (scaffold) | 558,973 bytes (546 KiB) |
| Same, with ledger-core's engine (S4, on main at 47bac69) | 2,155,943 bytes (2.1 MiB) |

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
$S call -s local $DB run_command '"org-s1"' '"req-1"' '"{\"type\":\"vendor.add\",\"input\":{\"name\":\"Acme\",\"node_id\":null,\"payment_details\":\"Paybill 000000\"}}"'
$S sql -s local $DB "SELECT request_id, result_json FROM command_result"

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

Over HTTP (what the replay below uses): `POST /v1/identity` makes an identity and token;
`POST /v1/database/$DB/call/<reducer>` with `Authorization: Bearer <token>` and a JSON array of
arguments answers 200, or 530 with the reducer's error text; `POST /v1/database/$DB/sql` with
the query as body answers `[{schema, rows}]`. Over HTTP an `Identity` argument is a one-element
array `["0x..."]` and an `Option` column comes back as `[0, value]` (some) or `[1, []]` (none).

## Replay the command fixture on the local server

```sh
cd rust
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
cd tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run module-replay
```

`tools/module-replay.ts` publishes the module as `ft-s4-test` (`--db ft-<task>` for another
name), opens the demo organisation's books and checks them against
`fixtures/setup.json#/createOrgLedger`, adds the fixture's three actors as members, loads
`fixtures/demo-books.json#/state` with `import_books`, then sends each of the 37 commands of
`fixtures/commands-accepted.json` as its actor and compares every `command_result` and the final
books (read through the `my_*` views and the TS client codec) with the fixture. The module draws
UUIDs and stamps the reducer time where the fixture has `fx-0001`, ... and a frozen clock, so ids
are compared through a fixture-to-module map learned as records appear, and a reducer timestamp
stands in for the frozen one; everything else must be equal. It then checks four refusals and a
repeated request id, deletes the database (unless `--keep`) and prints `spacetime list -s local`.
It exits 1 on any difference. A run takes about 3 s. It shares `tools/src/module-server.ts` and
`tools/src/demo-replay.ts` with the integration run below.

## Integration

One command, with the local server up, proves the published module end to end (gate G3):

```sh
cd rust/tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run integration
```

`tools/module-integration.ts` builds the release wasm (cargo, waiting while two other cargo
builds run), publishes it as a fresh `ft-s2-<timestamp>` database, and prints one timed `pass` or
`FAIL` line per check:

- `init` made the publisher the only operator (the owner from `GET /v1/database/:name`);
- the demo organisation opens (`create_org_ledger`, three members); `import_books` loads
  `fixtures/demo-books.json#/state` (the demo backfill is engine-level, so it is not expressible
  as commands); `run_command` then replays the 37 commands of `fixtures/commands-accepted.json`
  with every result and the final books equal to the fixture;
- after each load, with `spacetime sql` as the owner: `COUNT(*)` per table and organisation
  against the fixture collections (plus `ledger_org`, `member`, `command_result`); from the
  `journal_line` rows of posted entries: debits equal credits in base currency overall, per fund
  and per entry; the accrual trial balance computed from the rows equals
  `fixtures/report-trial-balance.json` case 0 (demo books) and case 12 (after the command log);
- refusals with the TS text: a non-member `run_command`, a non-operator `create_org_ledger`, a
  non-operator `import_books` (Rust-only reducer, text from `src/rules.rs`), a member removing
  themselves;
- every `my_*` view gives a member exactly the organisation's table rows and `--anonymous` none;
- a repeated request id runs again and overwrites the caller's `command_result`;
- the database is deleted (also on failure; `--keep` keeps it) and the run fails at five minutes.

It takes about 4 s when the wasm is already built (about 15 s more for a cold wasm build) and
exits 1 on any failure. `--no-build` skips the cargo step; `--db ft-s2-<name>` picks the name.

The same run from cargo, ignored by default so the workspace checks stay offline:

```sh
FT_MODULE_INTEGRATION=1 cargo test -p ledger-module --test integration -- --ignored
```

Without `FT_MODULE_INTEGRATION` that test returns at once (`tests/integration.rs`).
