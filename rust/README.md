# FlowTrace Books (Rust)
Cargo workspace for the native desktop ERP, its ledger core and its SpacetimeDB backend. Check with `cd rust && cargo fmt --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked`. The web app in `flowtrace/` is the parity reference and is read-only from here.

| Crate | Holds |
|---|---|
| `crates/ledger-core` | money, types, engine, commands, reports, reconcile, importers, demo books; no I/O |
| `crates/ledger-module` | SpacetimeDB server module |
| `crates/ledger-client` | SpacetimeDB Rust client |
| `crates/desktop` | GPUI desktop app: shell, theme, `LedgerBackend` and screens (see its README) |

Decisions: `docs/adr/` (0002: GPUI, gpui-kit and the SpacetimeDB client, from the F3 spike). Screenshots: `docs/screens/`. The app runs with `cargo run -p desktop`; the F3 spike that ADR 0002 describes was removed in U1 and lives in git history.
