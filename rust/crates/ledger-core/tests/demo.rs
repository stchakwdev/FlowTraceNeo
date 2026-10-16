//! Task C6 acceptance for `flowtrace/src/lib/demo/ledger.ts`: the demo books built in Rust from
//! nothing, with the demo env, equal `demo-books.json#/state` as a value and byte for byte after
//! the exporter's sorted-key reprint. The SpacetimeDB module and the desktop seed their demo
//! from this call.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::replay::{assert_json_eq, to_json};
use common::{fixture, fixture_text};
use ledger_core::demo::build_demo_ledger;
use serde_json::Value;

/// Sorts every object's keys, independent of the `serde_json` `preserve_order` feature.
fn sort_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.sort_keys();
            map.values_mut().for_each(sort_keys);
        }
        Value::Array(items) => items.iter_mut().for_each(sort_keys),
        _ => {}
    }
}

/// Prints JSON as the exporter writes it: keys sorted, two-space indent, trailing newline.
fn print(value: &Value) -> String {
    let mut sorted = value.clone();
    sort_keys(&mut sorted);
    let mut out = serde_json::to_string_pretty(&sorted).unwrap();
    out.push('\n');
    out
}

#[test]
fn rust_built_demo_books_equal_the_fixture() {
    let state = build_demo_ledger().unwrap();
    let books = fixture("demo-books");
    assert_json_eq(&to_json(&state), &books["state"], "buildDemoLedger() state");

    // Byte-equal: the whole fixture file reprinted with the Rust state in place.
    let mut file = books;
    file["state"] = to_json(&state);
    assert!(
        print(&file) == fixture_text("demo-books"),
        "demo-books.json reprints differently"
    );
}
