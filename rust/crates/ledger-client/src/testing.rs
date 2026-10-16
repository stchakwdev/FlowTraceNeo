//! Fixtures the unit tests share.

#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use ledger_core::types::LedgerState;

/// `rust/fixtures/demo-books.json#/state`, the demo organisation's books.
pub fn demo_state() -> LedgerState {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-books.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    serde_json::from_value(fixture["state"].clone()).unwrap()
}
