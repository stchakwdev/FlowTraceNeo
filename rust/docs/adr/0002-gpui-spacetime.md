# ADR 0002: GPUI with gpui-kit on SpacetimeDB, proven by the F3 spike

Status: accepted · 2026-10-16

## Context

FlowTrace Books is a native desktop ERP in Rust (`PRIMER.md` section 1): GPUI with
gpui-kit for the UI, a Rust SpacetimeDB module for storage, and a Rust client SDK in
between. Before any screen (U1 to U10) or the module (S1) is built, task F3 had to show
on the primary machine (Omarchy: Hyprland on Wayland, AMD Radeon with Vulkan) that
gpui-kit 0.7.3 opens a window and renders a large table, and that a GPUI window can show
live rows from a SpacetimeDB 2.12 database.

The spike is a second binary, `desktop-spike`, in `crates/desktop`
(`src/bin/desktop-spike/`), next to the untouched product stub `src/main.rs`. Its module
is `spikes/stdb-spike/`: one public table `spike_row` and one reducer `upsert_rows` that
inserts or updates a range of rows. The module is in `[workspace] exclude` in
`rust/Cargo.toml`: it builds to wasm through the spacetime CLI, and keeping it out keeps
the workspace checks fast. It was published only to the local server as `ft-f3-spike`
and deleted afterwards. `ledger-core`, `ledger-module` and `ledger-client` are unchanged.

The window shows a status line, a text input with an "Add row" button that calls the
reducer, and a gpui-kit `DataTable` over 10,000 rows. Calling the reducer from the CLI
(`spacetime call -s local ft-f3-spike upsert_rows 3 1 '"Edited from the CLI"' 99999`),
inserting row 10001 the same way and deleting row 5 with `spacetime sql` all reached the
table within a second. `docs/screens/spike.png` is the final build on Wayland after one
more CLI update, to row 2, five seconds after start: the status line counts the 10,000
initial inserts as one batch and the update as a second.

## Decision

1. **Crates.** These resolved and worked together on Rust 1.99.0:

   | Crate | Version | Features | Source |
   |---|---|---|---|
   | `gpui-kit` | 0.7.3 | default (`component`, `assets`) | <https://gpui-kit.com/docs/installation> |
   | `gpui-pre` (re-exported as `gpui_kit::*`) | =0.3.9, pinned by gpui-kit | gpui-kit turns on `wayland`, `x11`, `font-kit`, `runtime_shaders` on `gpui-pre-platform` | gpui-kit 0.7.3 `Cargo.toml` |
   | `gpui-component`, `gpui-base`, `gpui-kit-assets` | 0.7.3 (gpui-kit asks for ^0.7.1) | none added | |
   | `spacetimedb-sdk` | 2.12.0 | default | <https://spacetimedb.com/docs/clients/rust> |
   | `spacetimedb` (module) | =2.12.0 | default | <https://spacetimedb.com/docs/quickstarts/rust/> |
   | `async-channel` | 2.5.0 | | already in GPUI's tree |

   Applications list `gpui-kit` alone and never `gpui-pre`: the kit pins its GPUI
   snapshot itself, so a direct `gpui-pre` line would only add a second place to keep
   in step. `gpui-kit`, `spacetimedb-sdk` and `async-channel` are in
   `[workspace.dependencies]` so `ledger-client` takes the same SDK version. GPUI renders
   through wgpu 29.0.4 (`gpui-pre-wgpu`) on Vulkan.

2. **Wayland, with X11 as a working fallback.** The window opened natively on Wayland on
   the first try (Hyprland 0.56.2, Mesa `vulkan-radeon` 26.2.2). gpui-kit has no `x11`
   or `wayland` feature of its own; it compiles both backends in and GPUI picks one at
   run time from the environment (`WAYLAND_DISPLAY`, else `DISPLAY`). Started with
   `env -u WAYLAND_DISPLAY DISPLAY=:0`, the same binary ran under XWayland and rendered
   the same table. Pre-decision P2 did not fire.

3. **SDK drive mode: `run_threaded`, with every row callback sending on one channel to a
   GPUI task.** Both modes were built and measured (debug build, 10,000 rows, Wayland):

   | | `run_threaded` | `frame_tick` from a 16 ms GPUI timer |
   |---|---|---|
   | Idle CPU over 20 s | 0.0 % | 1.6 % |
   | Initial rows applied to the table after the connection starts | 63 to 68 ms (3 runs) | 132 to 153 ms (3 runs) |
   | Where the subscription is decoded | SDK thread | GPUI main thread |

   `run_threaded` costs nothing at idle because the window wakes only when a message
   arrives, and it keeps WebSocket decoding off the thread that paints. `frame_tick` polls
   whether or not anything changed and decodes the initial 10,000 rows on the main
   thread, which doubled the time to first rows. Callbacks are `Send + 'static` and run
   on the SDK thread in either mode, so they cannot touch GPUI; both modes need the
   channel anyway (SDK source `table.rs`, `on_insert` bounds).

4. **How the subscription maps onto a GPUI entity.** One entity owns the receiver. A
   task from `cx.spawn` awaits the first message, drains whatever else is queued with
   `try_recv`, and applies the batch inside one `this.update`, then `cx.notify()` once.
   The SDK fires `on_insert` for every initial row, and in threaded mode all 10,000
   were queued before the task woke, so they arrived as one batch and one repaint. In
   SDK 2.12 `on_applied` runs before the row callbacks of the same initial update (the
   client cache is already filled; `db_connection.rs`, `apply_update`), so "applied" on
   the channel does not mean the initial rows have been sent yet. The store treats every
   drained batch alike and an insert of a present key as a replace, so the order does not
   matter to it; `ledger-client` must not take a snapshot on `on_applied` and then also
   apply the inserts that follow without the same idempotence. The rows the table renders live in the `TableDelegate`
   inside the `TableState` entity, kept by `RowStore` (`src/bin/desktop-spike/rows.rs`):
   one row per key, ascending, in place by binary search for small batches and through
   a `BTreeMap` for batches of 32 or more. The view never reads the SDK client cache
   (`conn.db.spike_row().iter()`) while rendering: with the SDK on its own thread the
   cache can be ahead of the events the entity has applied, and one source of rows per
   screen keeps the table consistent with its own counts.

## Measurements

From `cargo clean` on Omarchy (16 threads, 44 GiB RAM), crates already in the local
registry cache, debug profile unless stated. `/usr/bin/time` is not installed on this
machine, so the build was run under a small `os.wait4` wrapper that reports the same
rusage figures as `time -v`, plus a sampler that summed the RSS of the whole build tree
every 0.25 s.

| Measure | Value |
|---|---|
| Clean debug build, `cargo build -p desktop` (both binaries, 587 crates compiled) | 77.2 s wall, 716 s user CPU |
| Incremental rebuild after a one-line change in `view.rs` | 3.2 s and 3.1 s (two runs) |
| Peak RSS of the largest single process, clean build (what `time -v` reports) | 3.31 GiB (3,472,128 KiB) |
| Same, incremental rebuild | 3.31 GiB (3,468,804 KiB): the link step, not rustc |
| Peak RSS of the whole build tree, clean build | 6.36 GiB (6,667,232 KiB) |
| Same, incremental rebuild | 3.70 GiB (3,883,152 KiB) |
| `desktop-spike` debug binary | 777.2 MiB (814,968,680 bytes); 101.6 MiB stripped |
| Product stub `desktop` debug binary | 4.3 MiB (4,535,592 bytes) |
| `target/` after the clean debug build | 4.8 GiB |
| Idle RSS, 10,000 rows shown, debug, Wayland | 186.6 to 186.8 MiB (191,096 to 191,260 KiB, 3 runs), 35 threads, 0.0 to 0.07 % CPU |
| Applying the 10,000-row initial batch to `RowStore` | about 5 ms debug, 0.8 ms release |
| Clean release build, separate target dir | 176.7 s wall, 1,771 s user; peak 1.87 GiB single process, 6.25 GiB tree |
| `desktop-spike` release binary | 57.7 MiB (60,539,824 bytes); 41.8 MiB stripped; 15.7 MiB stripped and `gzip -9` |
| Idle RSS, 10,000 rows shown, release, Wayland | 139.8 to 139.9 MiB (143,112 to 143,304 KiB, 2 runs); initial rows applied 25 ms after the connection starts |
| Idle CPU under XWayland (debug) | 0.6 % over 20 s, against 0.0 % on Wayland |

One experiment beyond the brief, in a separate target dir: the same clean debug build
with `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.

| Measure | Default debug info | `line-tables-only` |
|---|---|---|
| Clean build | 77.2 s | 67.1 s |
| Incremental rebuild | 3.2 s | 1.8 s |
| Peak RSS, largest single process, incremental | 3.31 GiB | 1.67 GiB (1,753,816 KiB) |
| Peak RSS, whole tree, clean / incremental | 6.36 / 3.70 GiB | 5.92 / 2.15 GiB (6,203,908 / 2,257,116 KiB) |
| `desktop-spike` binary | 777.2 MiB | 302.6 MiB (317,349,992 bytes) |
| `target/` | 4.8 GiB | 3.3 GiB |

Backtraces keep file and line numbers; a debugger loses local variables. Whether to set
`[profile.dev] debug = "line-tables-only"` in `rust/Cargo.toml` is left to the
coordinator; this ADR does not change the profile.

## Pattern for U1 to U10

Every screen reads one shared store entity and never the SDK. `ledger-client` builds
the connection, registers row callbacks that only send typed changes on an
`async_channel` channel, and drives the connection on its own thread. The desktop app
creates one store entity at start-up that owns the receiver and the rebuilt ledger
state; its pump task drains the channel in batches and notifies once per batch. Each
screen holds its own `TableState` entity, observes the store, and on each notification
refreshes its delegate's rows from the store, so a screen re-derives what it shows from
one place and the delegate renders only the visible rows. Writes go from a button
handler straight to a reducer call on the shared connection; the change comes back
through the same subscription, so screens never patch their rows optimistically. This is
where the `LedgerBackend` trait of `PRIMER.md` section 1 sits: the store is built on
whatever backend the app was started with.

```rust
// ledger-client: callbacks run on the SDK thread and only send.
pub fn connect(uri: &str, db: &str) -> Result<(Arc<DbConnection>, Receiver<Change>), Error>;

// desktop: one store entity owns the receiver and the state every screen reads.
pub struct LedgerStore { state: LedgerState, _pump: Task<()> }

impl LedgerStore {
    pub fn new(rx: Receiver<Change>, cx: &mut Context<Self>) -> Self {
        let pump = cx.spawn(async move |this, cx| {
            while let Ok(first) = rx.recv().await {
                let mut batch = vec![first];
                while let Ok(more) = rx.try_recv() {
                    batch.push(more);
                }
                let applied = this.update(cx, |store, cx| {
                    store.state.apply(batch);
                    cx.notify();
                });
                if applied.is_err() {
                    break;
                }
            }
        });
        Self { state: LedgerState::default(), _pump: pump }
    }
}

// a screen: observe the store, refresh the delegate, keep the Subscription in a field.
let refresh = cx.observe(&store, |screen: &mut Journal, store, cx| {
    let rows = store.read(cx).state.journal_rows();
    screen.table.update(cx, |table, cx| {
        table.delegate_mut().rows = rows;
        cx.notify();
    });
});
```

## What did not work, and gpui-kit surprises

- `Column::text_right()` has no effect in `DataTable`: the cell wrapper sizes the cell
  and ignores `Column::align`, and the default header does too (gpui-component 0.7.3
  `table/state.rs`, `render_cell`; `table/delegate.rs`, `render_th`). The delegate aligns
  its own content in `render_td` and `render_th`.
- Setting `theme.primary` through `Theme::update` leaves primary buttons black: buttons
  read `button_primary`, `button_primary_hover`, `button_primary_active` and
  `button_primary_foreground`. The spike sets both sets of fields from `DESIGN.md`.
- `open_window` already wraps the returned view in a `Root`
  (<https://gpui-kit.com/docs/getting-started>); returning another `Root` is wrong.
- `spacetime generate` output fails the workspace checks as written. It is not formatted
  to `rust/rustfmt.toml` (run `cargo fmt` after every generate), and it trips pedantic
  clippy and `missing_docs`, which the files' own `#![allow(unused, clippy::all)]` does
  not cover. The `mod module_bindings;` line carries
  `#[allow(clippy::pedantic, missing_docs, reason = "generated ...")]`; `#[expect]` would
  break whenever a regenerate stops tripping one of them.
- The spacetimedb 2.12 `table` and `reducer` macros emit undocumented public items, so
  a module crate with `missing_docs = "warn"` needs a crate-level
  `#![expect(missing_docs, reason = ...)]`. S1 will meet the same.
- `run_threaded` panics its thread on any disconnect that is not clean (SDK 2.12
  `db_connection.rs`, `run_threaded`: `Err(e) => panic!("{e:?}")`). The window keeps
  running but stops receiving. Not exercised in the spike; `ledger-client` should run
  the same loop itself over `advance_one_message_blocking` and send the error on the
  channel instead of panicking.
- The "Add row" button was not clicked by automation. The only input tool installed is
  `wtype`, which types into whichever window has focus, and Hyprland 0.56 has no
  synthetic click. The reducer path was proven from the CLI, and the button calls the
  same generated `upsert_rows`.
- Omarchy's Hyprland rules give every window 0.985 opacity, and Hyprland 0.56 (Lua
  config) no longer accepts `hyprctl setprop`, so the wallpaper shows faintly through
  the screenshot. It is a compositor setting, not GPUI.
- `spacetime call` takes each argument as JSON: a string argument needs its own quotes,
  `'"Row"'`.

## Consequences

- U1 replaces `src/main.rs`, deletes `src/bin/desktop-spike/` and the `[[bin]]` entry,
  and keeps the store-entity pattern above. `spikes/stdb-spike/` can go with it.
- Debug builds need about 3.5 GiB of free memory just to link (1.7 GiB with
  `line-tables-only`), and a clean build about 6.5 GiB with 16 jobs; a smaller machine
  should lower `-j`. The 777 MiB debug binary is almost all debug info.
- The rust CI workflow (`.github/workflows/rust.yml`) installs the GPUI system libraries listed at
  <https://gpui-kit.com/docs/installation> (without WebKit and ALSA, which only
  gpui-shell and the `speech` feature need).
- Scrolling performance of the 10,000-row table was not measured: there is no safe way
  to drive the pointer on this machine. The table renders only visible rows
  (`TableState` uses a uniform list), and idle cost with 10,000 rows was nil.
- The SDK owns a tokio runtime (`spacetimedb-sdk` `db_connection.rs`,
  `enter_or_create_runtime`) next to GPUI's executor; the spike saw no conflict, and
  callbacks only send.

## 2026-10-16: how ledger-client implements the pattern (S3)

`crates/ledger-client` follows the pattern above with one change the warning in "What did not
work" asked for: it does not use `run_threaded`. Its thread runs its own loop over
`advance_one_message_blocking`; callbacks send on an internal channel; after each message the
loop applies that message's row changes to its own cache as one batch and sends one
`BackendEvent::Changed { orgs }`. So the desktop's store receives `BackendEvent`s, not row
changes, and reads books with `LedgerBackend::state(org)` instead of applying rows itself:

```rust
// desktop: the store entity owns a receiver from whichever backend the app started with.
let rx = backend.subscribe(); // first event: the current ConnectionStatus
// in the cx.spawn pump: drain a batch, then for each org in Changed { orgs }:
store.books.insert(org.clone(), backend.state(&org)?);
cx.notify();
```

Subscription rows map back to engine state one to one: the `my_*` views return the module's
table rows unchanged, the generated binding structs have the same fields as
`ledger_module::tables`, and the rebuild is the module's own `codec::assemble_state`.
Pre-decision P6 did not fire.
