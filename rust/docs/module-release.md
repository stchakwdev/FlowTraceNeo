# Releasing the ledger module

How `crates/ledger-module` (the SpacetimeDB 2.12 server module) is built, how the maincloud dev
database was published, how people get access to it, and what a production publish will need.
Day-to-day commands against the local server are in `../crates/ledger-module/README.md`; this
page is about maincloud.

Rule for agents and scripts: **nothing here is run against maincloud by an agent.** Publishing,
deleting or renaming a maincloud database is the user's step, at a terminal logged in to
SpacetimeDB through the web. Agents publish only to the local server, under a name carrying
their task id, and delete it afterwards.

## 1. Build the wasm

From `rust/`, either command (both write
`target/wasm32-unknown-unknown/release/ledger_module.wasm`):

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
~/.local/bin/spacetime build -p crates/ledger-module
```

The `wasm32-unknown-unknown` target comes with the pinned toolchain (`rust-toolchain.toml`,
1.99.0); if it is missing, `rustup target add wasm32-unknown-unknown`. `spacetime build` looks
for `wasm-opt` afterwards and publishes unoptimised without it. Sizes and times are in the
module README (2.1 MiB unoptimised, about 11 s cold).

Before publishing anywhere, the module must pass the workspace checks and the local end-to-end
run (gate G3 in `PRIMER.md`):

```sh
cd rust
cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked
cd tools && ~/.bun/bin/bun install --frozen-lockfile && ~/.bun/bin/bun run integration
```

After any change to tables, views or reducers, regenerate the client bindings
(`../crates/ledger-client/README.md`, "Generated bindings") in the same change, or the desktop
reads the old shape.

## 2. How maincloud dev was published

The dev database is `flowtrace-ledger-dev` on maincloud
(`https://maincloud.spacetimedb.com`, dashboard <https://spacetimedb.com/flowtrace-ledger-dev>).
The user published it on 2026-10-16 at 16:24 UTC, from `rust/` on main, after
`spacetime login` (a browser login to their SpacetimeDB account):

```sh
spacetime publish -s maincloud flowtrace-ledger-dev -p rust/crates/ledger-module -y
```

(written from the repository root; `-p` builds the module first). The name was chosen by
pre-decision P8: dev is disposable. The module's `init` reducer made the publishing identity the
first operator (`src/reducers.rs`, `init`), so the CLI login that published it is the operator.

Thread t-0024 then checked the published database without publishing anything, through the
tools' remote mode (`../tools/README.md`, "Remote mode"): `integration` 20 of 20 checks,
`module-replay` 37 of 37 commands, and the client live test against it. Each of those runs
leaves a demo organisation in the database; three were there after that check.

Updating dev later is the same command after a merge to main. The CLI asks before a change that
breaks existing clients or would delete data; `-y` with no value answers yes to every prompt,
including the delete-data one, so for anything but a throwaway database leave `-y` out and read
the prompts. Never add `-c` / `--delete-data`.

## 3. Operators and members

Two levels of access, both kept in the module's private tables (`src/rules.rs`):

| Who | Can | Granted by |
|---|---|---|
| Operator | open books for a new organisation (`create_org_ledger`), load books (`import_books`), add and remove operators | `init` (the publisher), then `add_operator` by an existing operator |
| Member of an organisation | read that organisation's rows through the `my_*` views, `run_command` as that member; with `can_post`, `can_approve`, `can_close` as given | `create_org_ledger` makes the caller the owner (all three); then `add_member` by a member who can close |

A new identity (a desktop that just connected with a fresh token file) is a member of nothing
and sees no organisations. The Connection screen shows the first eight hex digits of its
identity; the full 64 digits are on stderr when the app runs with `--timings on`
(`desktop: status Connected { identity: "..." }`, `crates/desktop/src/store.rs`). The identity is
public; the token file next to it is not, and is never sent to anyone. Then, logged in as an
operator or a closing member, from any directory:

```sh
S=~/.local/bin/spacetime
DB=flowtrace-ledger-dev

# Open books for an organisation (operator): org id, base currency, fiscal year, your name.
$S call -s maincloud $DB create_org_ledger '"org-example"' '"KES"' 2026 '"Ama Owner"'

# Add a member (a member who can close): org, identity, user id, name, can_post, can_approve, can_close.
$S call -s maincloud $DB add_member '"org-example"' '"0x<64 hex digits>"' '"kofi"' '"Kofi Clerk"' true false false

# Remove a member; nobody can remove themselves or drop their own close permission.
$S call -s maincloud $DB remove_member '"org-example"' '"0x<64 hex digits>"'

# Operators (operator only); the last operator cannot be removed.
$S call -s maincloud $DB add_operator '"0x<64 hex digits>"'
$S call -s maincloud $DB remove_operator '"0x<64 hex digits>"'
```

Arguments are JSON: a string carries its own quotes and an identity is a `"0x..."` string. A
refusal exits 1 with the rule's message (`Error: Response text: ...`). `add_member` on an
existing member updates their permissions; a member's user id cannot change once given. The
module stores no organisation name: the desktop lists organisations by id.

These calls write to a shared database, so in the project they are the user's step on
maincloud, as publishing is. The same calls with `-s local` are how threads test.

## 4. What a production publish will need

Not done, and not to be run by an agent. The user decides each item:

1. **A name.** Still to be chosen by the user (pre-decision P8 queued it). Database names match
   `^[a-z0-9]+(-[a-z0-9]+)*$`. The desktop's default database is `flowtrace-ledger-dev`
   (`crates/desktop/src/config.rs`); a production build would either change that default or ship
   a `config.toml` naming the production database.
2. **The three open decisions** in `handoff.md` confirmed first, because two of them shape stored
   data: timestamps in milliseconds (every audit and engine timestamp written), and
   `float_roundtrip` (how stored rates are parsed). Changing either after real books exist means
   a migration.
3. **Which identity publishes.** The publisher becomes the only operator through `init`. Use an
   account meant to own production (an organisation account, `--organization`, if the team
   should share it), not a personal dev login.
4. **No test data.** Never point `rust/tools` `integration` or `module-replay --existing`, or the
   client live test, at production: each run writes a demo organisation and leaves it there.
5. **The command**, from a clean checkout of the tagged commit, after the checks above, without
   `-y` so every prompt is read:

   ```sh
   spacetime publish -s maincloud <production-name> -p rust/crates/ledger-module
   ```

6. **Later publishes** update the schema of a database that holds real books. The CLI prompts
   before a change that breaks existing clients and before anything that would destroy data
   (`spacetime publish --help`, options `--break-clients`, `--delete-data`, `--yes`). Never use
   `--delete-data` on production; a breaking change needs a migration plan first.
7. **Backups and retention** of the maincloud database are not covered by anything in this
   repository yet.
