//! FlowTrace Books, the desktop app: GPUI with gpui-kit over the Rust ledger core.
//!
//! Layout (see `README.md`): [`backend`] holds the books behind the [`backend::LedgerBackend`]
//! trait, [`model`] the shell's state in plain structs, [`theme`] the `DESIGN.md` tokens, and
//! the GPUI views are [`store`], [`shell`] and [`panels`].

pub mod backend;
pub mod model;
pub mod theme;
