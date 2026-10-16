# TanStack Start + shadcn/ui

This is a template for a new TanStack Start project with React, TypeScript, and shadcn/ui.

## Running the Books screens on SpacetimeDB

By default the Books screens (Spend, Grants, Ledger, Reconcile, Reports) run on the bundled
demo books, which reset on reload. To run them on the Rust ledger module instead, put this in
an untracked `.env.local` (never commit it):

```sh
VITE_LEDGER_BACKEND=spacetimedb
VITE_SPACETIMEDB_URI=wss://maincloud.spacetimedb.com
VITE_SPACETIMEDB_MODULE=flowtrace-ledger-dev
# Optional: which organisation's books to open. Unset, the app opens the first
# organisation this browser's identity is a member of.
# VITE_SPACETIMEDB_ORG=org-example
```

Then `bun run dev`. This works in dummy auth mode as well as with WorkOS. The browser keeps
its SpacetimeDB identity in local storage (`flowtrace.spacetimedb.token`), so each browser
profile is one identity. A new identity belongs to no organisation: the Books screens say
"The books are not connected yet" and show the identity (`0x` and 64 hex digits). An
operator opens books and adds the identity as a member, for example:

```sh
S=~/.local/bin/spacetime
$S call -s maincloud flowtrace-ledger-dev create_org_ledger '"org-example"' '"KES"' 2026 '"Ama Owner"'
$S call -s maincloud flowtrace-ledger-dev add_member '"org-example"' '"0x<identity>"' '"ama"' '"Ama Owner"' true true true
```

The screens pick the new membership up without a reload. The member row is the actor: the
user id, name and the post, approve and close permissions given here are what the module
checks on every command, so approving a claim or a bill needs a second identity. The full
access rules are in `rust/docs/module-release.md` section 3.

The client bindings in `src/lib/ledger/spacetime-bindings/` are generated from the Rust
module. After a module change:

```sh
cd rust && cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
cd ../flowtrace && rm -rf src/lib/ledger/spacetime-bindings
~/.local/bin/spacetime generate --lang typescript \
  -b ../rust/target/wasm32-unknown-unknown/release/ledger_module.wasm \
  -o src/lib/ledger/spacetime-bindings -y
bunx prettier --write src/lib/ledger/spacetime-bindings
```
