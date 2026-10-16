//! FlowTrace Books, the desktop app: GPUI with gpui-kit over the Rust ledger core.
//!
//! Layout (see `README.md`): [`backend`] holds the books behind the [`backend::LedgerBackend`]
//! trait, [`model`] the shell's state in plain structs, [`theme`] the `DESIGN.md` tokens, and
//! the GPUI side is [`store`] (the one store entity), [`shell`] (the window's root view),
//! [`panels`] (one content panel per nav entry) and [`app`] (start-up).

pub mod accounts;
pub mod app;
pub mod backend;
pub mod model;
pub mod panels;
pub mod shell;
pub mod store;
pub mod theme;
