//! The acceptance test of C1: every ledger state in the fixtures deserialises into
//! `LedgerState` and serialises back to the same JSON, and re-printing the whole fixture file with
//! the Rust value in place reproduces the file bytes.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::{find_objects, fixture, fixture_text};
use ledger_core::types::{EntryInput, LedgerState};
use serde_json::Value;

/// Every state fixture: file and JSON pointer. `rust/fixtures/README.md` lists the first four;
/// the last two are the `createOrgLedger` and `emptyLedger` states of `setup.json`.
const STATES: [(&str, &str); 6] = [
    ("demo-books", "/state"),
    ("commands-accepted", "/final_state"),
    ("demo-backfill", "/initial_state"),
    ("reconcile-auto-match", "/state_before_demo_auto_match"),
    ("setup", "/createOrgLedger/state"),
    ("setup", "/emptyLedger/0/result"),
];

/// Sorts every object's keys, so printing does not depend on serde_json's `preserve_order`
/// feature, which another workspace crate may switch on.
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
fn every_state_fixture_round_trips_as_a_json_value() {
    for (file, pointer) in STATES {
        let f = fixture(file);
        let original = f.pointer(pointer).unwrap();
        let state: LedgerState = serde_json::from_value(original.clone())
            .unwrap_or_else(|e| panic!("{file}#{pointer}: {e}"));
        let back = serde_json::to_value(&state).unwrap();
        assert_eq!(&back, original, "{file}#{pointer}");
    }
}

#[test]
fn every_state_fixture_reprints_byte_equal() {
    for (file, pointer) in STATES {
        let mut f = fixture(file);
        let state: LedgerState =
            serde_json::from_value(f.pointer(pointer).unwrap().clone()).unwrap();
        *f.pointer_mut(pointer).unwrap() = serde_json::to_value(&state).unwrap();
        // Byte equality holds because every state number is an integer or a short decimal
        // (`fx_rate` 129.37) that ryu and ECMAScript print alike, and both printers leave
        // non-ASCII text unescaped. See "Formatting notes" in rust/fixtures/README.md.
        assert!(
            print(&f) == fixture_text(file),
            "{file}#{pointer} does not reprint byte-equal"
        );
    }
}

#[test]
fn the_state_fixtures_are_every_state_in_the_fixtures() {
    // A state is any object holding `settings`, `accounts` and `counters`.
    let mut found = 0;
    for file in [
        "demo-books",
        "commands-accepted",
        "demo-backfill",
        "reconcile-auto-match",
        "setup",
        "rejected-cases",
        "engine-queries",
    ] {
        let f = fixture(file);
        let mut states = Vec::new();
        find_objects(
            &f,
            &|m| {
                m.contains_key("settings")
                    && m.contains_key("accounts")
                    && m.contains_key("counters")
            },
            &mut states,
        );
        found += states.len();
    }
    assert_eq!(found, STATES.len());
}

#[test]
fn unknown_state_fields_are_rejected() {
    let f = fixture("setup");
    let mut state = f.pointer("/emptyLedger/0/result").unwrap().clone();
    state["surprise"] = Value::Bool(true);
    assert!(serde_json::from_value::<LedgerState>(state).is_err());

    let mut state = f.pointer("/createOrgLedger/state").unwrap().clone();
    state["accounts"][0]["surprise"] = Value::Bool(true);
    assert!(serde_json::from_value::<LedgerState>(state).is_err());
}

#[test]
fn every_posted_entry_input_round_trips() {
    // `post` takes (actor, EntryInput); inputs leave optional fields out rather than null them.
    let f = fixture("rejected-cases");
    let mut steps = Vec::new();
    find_objects(
        &f,
        &|m| m.get("method").and_then(Value::as_str) == Some("post"),
        &mut steps,
    );
    assert!(!steps.is_empty());
    for step in steps {
        let input = &step["args"][1];
        let parsed: EntryInput =
            serde_json::from_value(input.clone()).unwrap_or_else(|e| panic!("{input}: {e}"));
        assert_eq!(&serde_json::to_value(&parsed).unwrap(), input);
    }
}
