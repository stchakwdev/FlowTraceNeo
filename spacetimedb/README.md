# FlowTrace ledger: SpacetimeDB module

The server side of the double-entry ledger. It is a TypeScript SpacetimeDB
module (SpacetimeDB 2.11) that runs the same engine as the demo,
`../flowtrace/src/lib/ledger`, inside reducers.

## How it works

- **Shared engine.** `src/index.ts` imports the engine with relative paths
  (`../../flowtrace/src/lib/ledger/...`). `spacetime build` bundles those files
  into `dist/bundle.js`, so nothing is copied. Keep the engine free of `@/`
  imports and framework code, or the bundle breaks.
- **Row shape.** `../flowtrace/src/lib/ledger/spacetime-codec.ts` converts
  between `LedgerState` and rows. The module and the client both use it.
  - `journal_entry` and `journal_line` have typed columns, with amounts as `i64`
    minor units.
  - Every other collection is a JSON doc row: `account`, `fund`, `grant`,
    `dimension`, `fiscal_period`, `budget`, `budget_line`, `vendor`, `bill`,
    `payment`, `expense_claim`, `bank_account`, `statement_line` and
    `audit_event`.
  - `ledger_org` holds the settings and counters.
  - Every row is keyed `${org_id}:${id}`, and a `seq` column keeps array order.
- **Members and permissions.** The `member` table holds identity, org,
  user_id, name and the `can_post`, `can_approve` and `can_close` flags. The
  reducers derive the actor from `ctx.sender` through this table. A
  client-supplied actor is never trusted.
- **Read access.** All tables are private. Clients read through the per-user
  views (`my_journal_entry` and the others). A view returns rows only for orgs
  where the caller is a member.

### Reducers

| Reducer | Who can call it | What it does |
| --- | --- | --- |
| `create_org_ledger(org_id, base_currency, fiscal_year, display_name)` | Anyone | Seeds the chart of accounts, the unrestricted fund and 12 periods. The caller becomes an owner with all three permissions. |
| `add_member(org_id, identity, user_id, name, can_post, can_approve, can_close)` | `can_close` | Adds a member, or changes an existing member's permissions. |
| `remove_member(org_id, identity)` | `can_close` | Removes a member. |
| `run_command(org_id, request_id, command_json)` | Members | Runs one write (details below). |

`run_command` is the only way to write to the books:

1. It loads the org's rows into a `LedgerState`.
2. It runs `applyCommand` with a deterministic env: `ctx.timestamp`, plus
   UUIDs from `ctx.newUuidV4()`.
3. It writes back only what changed.
4. It stores the result in `command_result`, which the caller reads through
   `my_command_result`.

If the engine throws a `LedgerError`, the reducer fails with
`LEDGER_ERROR [issues JSON]` and the whole transaction rolls back. The client
turns that message back into a `LedgerError`.

The write-back also checks every change, whatever the engine did:

- Audit events cannot be changed or deleted.
- Lines of a posted entry cannot be changed or deleted.
- A posted entry can only be marked reversed.

## Build and run locally

```sh
curl -sSf https://install.spacetimedb.com | sh   # installs ~/.local/bin/spacetime
cd spacetimedb
bun install
bun run typecheck
spacetime build                                  # writes dist/bundle.js

spacetime start                                  # local server on :3000, leave it running
spacetime publish flowtrace-ledger --server local --yes
spacetime call flowtrace-ledger create_org_ledger '"my-org"' '"KES"' 2026 '"Me"' --server local
spacetime sql flowtrace-ledger "SELECT number, status FROM journal_entry" --server local
```

Run the app against it. `flowtrace/.env.local` is gitignored; never commit it.

```sh
VITE_LEDGER_BACKEND=spacetimedb
VITE_SPACETIMEDB_URI=ws://127.0.0.1:3000
VITE_SPACETIMEDB_MODULE=flowtrace-ledger
```

### After a schema change

Regenerate the client bindings and check that the app still typechecks:

```sh
bun run generate        # writes ../flowtrace/src/lib/ledger/spacetime-bindings
cd ../flowtrace && bunx tsc --noEmit
```

If the schema change is breaking, a local database can be wiped with
`spacetime publish ... --delete-data=always`.

## Publish to Maincloud

```sh
spacetime login                                   # opens the browser
cd spacetimedb
spacetime publish flowtrace-ledger --server maincloud --yes
```

The database name must be unique on Maincloud. If `flowtrace-ledger` is taken,
pick another name and use it below.

## Vercel

Set these for the `flow-trace-neo` project, in Production and Preview. None of
them are secrets.

```sh
vercel env add VITE_SPACETIMEDB_URI       # wss://maincloud.spacetimedb.com
vercel env add VITE_SPACETIMEDB_MODULE    # flowtrace-ledger
vercel env add VITE_LEDGER_BACKEND        # spacetimedb
```

Then redeploy. When `VITE_LEDGER_BACKEND` is unset, the app uses the
in-memory demo ledger and the SpacetimeDB SDK is never loaded.

## Client

`flowtrace/src/lib/ledger/spacetime-backend.ts` exports
`spacetimeLedgerBackend`, which implements `LedgerBackend`.

- `load` subscribes to the org's `my_*` views and reassembles the state.
- `subscribe` pushes fresh state once per transaction.
- `run` calls `run_command` and waits for the matching `command_result` row.

Each browser gets an anonymous SpacetimeDB identity, and its token is kept in
localStorage. Before that identity can see or write an org's books, an owner
must add it with `add_member`. `spacetimeIdentity()` returns the identity hex
to hand to the owner.

To use WorkOS identities instead, configure WorkOS as an OIDC issuer for
SpacetimeDB and pass its token with `withToken`. The creator's `user_id` then
becomes the JWT `sub`.

## Known limits

- Each `run_command` loads all of the org's rows. That is fine for an NGO's
  books (thousands of entries). Very large ledgers will want per-collection
  loading.
- Any identity can claim an unused `org_id` with `create_org_ledger`. Once
  real auth is in place, gate this, for example by requiring a JWT, or with an
  allowlist table.

## Who can open books

The identity that publishes the module becomes the first **operator**
(the `init` reducer records it). Only operators can call
`create_org_ledger`, so nobody can claim an organisation id before its owner
does. An operator can allow another identity with
`spacetime call <module> add_operator '"<identity hex>"'`.
