//! FlowTrace Books, the desktop app: GPUI with gpui-kit over the Rust ledger core.
//!
//! Layout (see `README.md`): [`backend`] holds the books behind the [`backend::LedgerBackend`]
//! trait, [`model`] the shell's state in plain structs, [`theme`] the `DESIGN.md` tokens, and
//! the GPUI side is [`store`] (the one store entity), [`shell`] (the window's root view),
//! [`panels`] (one content panel per nav entry) and [`app`] (start-up).

pub mod access_groups;
pub mod accounts;
pub mod app;
pub mod backend;
pub mod badge;
pub mod bills;
pub mod campaigns;
pub mod chains;
pub mod claims;
pub mod config;
pub mod dashboard;
pub mod entry_form;
pub mod flows;
pub mod general_ledger;
pub mod grants;
pub mod journal;
pub mod large_books;
pub mod model;
pub mod panels;
pub mod proofs;
pub mod reconcile;
pub mod settings;
pub mod shell;
pub mod signin;
pub mod statements;
pub mod store;
pub mod theme;
pub mod transactions;
pub mod trust;
pub mod verification;
pub mod web_format;
