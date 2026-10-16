//! FlowTrace ledger: the SpacetimeDB 2.12 server module, the Rust form of
//! `spacetimedb/src/index.ts`.
//!
//! - [`tables`]: the TS module's twenty private tables, column for column.
//! - [`views`]: the `my_*` views that only return rows of the caller's organisations.
//! - [`reducers`]: `init`, operators, `create_org_ledger`, members, `run_command` (one ledger
//!   command through ledger-core) and `import_books` (an operator replaces an organisation's
//!   books).
//! - [`rules`], [`codec`], [`clock`], [`seed`], [`command`], [`write_back`]: the pure logic the
//!   reducers and views use, which the host tests cover.
//! - [`transparency`]: the transparency side (organisation and person profiles, access groups,
//!   proofs, campaigns, donations, donor updates): its tables, views, reducers and rules.
//!
//! Publish, call and delete commands for the local server: `README.md`. Table shape decisions:
//! `docs/adr/0003-module-tables.md`.
#![expect(
    missing_docs,
    reason = "the spacetimedb 2.12 table, view and reducer macros emit undocumented public items"
)]

pub mod clock;
pub mod codec;
pub mod command;
pub mod reducers;
pub mod rules;
pub mod seed;
pub mod tables;
pub mod transparency;
pub mod views;
pub mod write_back;

#[cfg(test)]
mod pipeline_tests;
