//! FlowTrace ledger: the SpacetimeDB 2.12 server module, the Rust form of
//! `spacetimedb/src/index.ts`.
//!
//! - [`tables`]: the TS module's twenty private tables, column for column.
//! - [`views`]: the `my_*` views that only return rows of the caller's organisations.
//! - [`reducers`]: `init`, operators, `create_org_ledger`, members, and `run_command`, whose
//!   body (running a command through ledger-core) is task S4.
//! - [`rules`], [`codec`], [`clock`], [`seed`]: the pure logic the reducers and views use, which
//!   the host tests cover.
//!
//! Publish, call and delete commands for the local server: `README.md`. Table shape decisions:
//! `docs/adr/0003-module-tables.md`.
#![expect(
    missing_docs,
    reason = "the spacetimedb 2.12 table, view and reducer macros emit undocumented public items"
)]

pub mod clock;
pub mod codec;
pub mod reducers;
pub mod rules;
pub mod seed;
pub mod tables;
pub mod views;
