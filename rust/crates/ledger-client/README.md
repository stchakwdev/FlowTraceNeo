# ledger-client

The desktop's way to the books: the Rust counterpart of
`flowtrace/src/lib/ledger/spacetime-backend.ts` and `spacetime-codec.ts`. It connects to the
ledger module (`../ledger-module`) on SpacetimeDB 2.12, subscribes to every `my_*` view, keeps
its own row cache, rebuilds a `ledger_core::LedgerState` per organisation and submits commands.
The desktop's `LedgerBackend` trait belongs to the desktop crate (U1); U10 adapts this client
to it. The client's API:

```rust
let client = SpacetimeClient::connect(
    ClientConfig::new("http://127.0.0.1:3100", "flowtrace-ledger-dev")
        .with_token_file(TokenFile::new(path)),
)?;                                                    // returns at once; one thread connects
client.status();                                       // Connecting, Connected, Ready, Disconnected
client.identity();                                     // Option<String>, 64 hex digits
client.organisations();                                // Vec<Organisation { org_id, settings, members }>
client.state(org_id)?;                                 // the rebuilt LedgerState
let rx = client.subscribe();                           // ClientEvent: Status, Changed { orgs }, Error
client.submit(org_id, &command)?;                      // blocking: Result<CommandResult, ClientError>
client.send(org_id, &command);                         // CommandTicket: .result().await or .wait()
client.create_org_ledger(org_id, "KES", 2026, name);   // Ticket<()>, operator only
client.import_books(org_id, &state);                   // Ticket<()>, operator only
client.transparency(org_id)?;                          // OrgTransparency: profile, groups, proofs, campaigns, ...
client.profiles()?;                                    // Vec<Profile>: own and co-members'
client.chains()?;                                      // Vec<ChainDetails>: both sides, names and scores
client.chain_activity(chain_id)?;                      // Vec<ChainActivity>, newest first
client.service();                                      // Option<ServiceInfo>: when the caller is a service
client.public_organizations()?;                        // public tables: any connection, anonymous too
client.public_campaigns()?;
client.public_badges()?;
client.disconnect();
```

The transparency views (MOD1) take the same path as the books (pre-decision P7): their rows go
into their own cache (`src/transparency.rs`), `transparency` rebuilds the module's
`ledger_module::transparency::OrgTransparency` through the module's own row codec, and each
applied message that touched them sends one `ClientEvent::TransparencyChanged { orgs, profiles,
chains, public, service }` next to the ledger's `Changed`. MOD2's organisation views (flow nodes,
transactions, visibility, trust score and history, flags, embed config) are part of
`OrgTransparency`; chains, the service row and the public tables have their own reads. Proofs arrive already cut to what the caller's access groups
allow (`visible_as` on each). The client has no write calls for them yet; DT1 to DT4 add what
their screens need.

A subscriber's first event is the current `ClientEvent::Status`. After that it gets one
`ClientEvent::Changed { orgs }` per applied message (initial subscription or transaction).
`send` resolves after the rows the command changed are in the cache, so `state` read after it
shows the change; `submit` is `send(..).wait()`, so call it off the UI thread. A broken ledger
rule comes back as `ClientError::Ledger(LedgerError)` with the engine's issues (decoded from the
reducer's `LEDGER_ERROR <issues JSON>`), any other reducer refusal as
`ClientError::Rejected(text)`, no answer within 15 s as `ClientError::Timeout`.

## How it runs

`SpacetimeClient::connect` starts one thread, `ledger-client`, and returns at once. The thread
builds the connection and drives it with its own loop over `advance_one_message_blocking`, not
the SDK's `run_threaded`, which panics its thread on a disconnect that is not clean; here a lost
connection becomes `ConnectionStatus::Disconnected { error: Some(..) }` and every pending ticket
fails with `ClientError::Disconnected`. SDK row and reducer callbacks run on that thread and only
send on an internal channel. After each message the loop applies the row changes as one batch,
sends one `ClientEvent::Changed { orgs }` per batch to every subscriber, then resolves the
tickets whose `command_result` row (matched by request id, `{connection id}-{n}`) or reducer
error arrived. The desktop's store entity follows ADR 0002: own a receiver, drain it in batches
in a `cx.spawn` task, call `state(org)` for the organisations named, `cx.notify()` once.

Rows are kept as `ledger_module::tables` structs and rebuilt with the module's own
`codec::assemble_state`, so client and module cannot read rows differently. That is why
`ledger-module` is also built as an `rlib`.

## Token

`TokenFile::new(path)` keeps the connection token in the file the caller names: loaded before
connecting (no file means a new identity), saved after every connect, written through a sibling
`.tmp` file with mode 0600. The token never appears in a log, an event or an error; `Debug`
shows the path only. Without a token file every connect is a new identity.

## Generated bindings

`src/module_bindings/` is `spacetime generate` output for the module, checked in. After any
change to the module's tables, views or reducers, from `rust/`:

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
rm -rf crates/ledger-client/src/module_bindings
~/.local/bin/spacetime generate --lang rust \
  -b target/wasm32-unknown-unknown/release/ledger_module.wasm \
  -o crates/ledger-client/src/module_bindings -y
cargo fmt -p ledger-client
```

Never edit the files by hand. `src/lib.rs` marks the module
`#[allow(clippy::pedantic, missing_docs, reason = ...)]` (an `expect` would break whenever a
regenerate stops tripping one of them).

## Tests

```sh
cd rust
cargo test -p ledger-client     # unit tests: rebuild, decoding, cache, token, tickets; offline
```

`tests/live_transparency.rs` (a member's read, MOD2 views included) and `tests/live_public.rs`
(an anonymous read of the public tables) are steps of the local transparency proof and only run
from `rust/tools/transparency-proof.ts`, which sets `FT_MOD1_DB`, `FT_MOD1_EXPECTED`,
`FT_MOD2_DB` and `FT_MOD2_PUBLIC`.

The live test (`tests/live.rs`) is `#[ignore]`d and also needs `LEDGER_CLIENT_LIVE=1`. It
publishes the module to the local server only (`http://127.0.0.1:3100`) as `ft-s3-test`
(`LEDGER_CLIENT_LIVE_DB` for another name), deletes the database at the end, pass or fail, and
prints `spacetime list -s local`:

```sh
cargo build --release --locked --target wasm32-unknown-unknown -p ledger-module
LEDGER_CLIENT_LIVE=1 cargo test -p ledger-client --test live -- --ignored --nocapture
```

It connects a client with a fresh token file, has the CLI identity that published the module
make it an operator, opens the demo organisation and imports `fixtures/demo-books.json#/state`
through the client, checks the rebuilt books equal the fixture and the trial balance and
financial position equal every demo-books case of their report fixtures, runs the checks of
`tests/common/mod.rs` (one accepted `vendor.add` with its `Changed` event, one refused unbalanced
entry), reconnects with the saved token as the same identity, and checks a stranger sees nothing
and is refused. A run takes about 1.2 s.
