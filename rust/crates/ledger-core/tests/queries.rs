//! Tasks C2 and C3 acceptance: the engine's read methods over the demo books answer every case
//! in `engine-queries.json` (accounts, funds, grants, entries, periods, validation, claims and
//! what is outstanding on bills and claims), and the `engine.ts` setup exports in `setup.json`
//! match.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::replay::{C2_QUERIES, C3_QUERIES, assert_json_eq, error_json, query, to_json};
use common::{cases, fixture};
use ledger_core::engine::{empty_ledger, normal_balance_of};
use ledger_core::types::LedgerState;
use serde_json::Value;

#[test]
fn engine_queries_match_over_the_demo_books() {
    let f = fixture("engine-queries");
    let books = fixture("demo-books");
    let state: LedgerState = serde_json::from_value(books["state"].clone()).unwrap();
    let mut checked = 0;
    for (i, case) in cases(&f, "cases").iter().enumerate() {
        let method = case["method"].as_str().unwrap();
        let what = format!("engine-queries case {i} {method}({})", case["args"]);
        let Some(outcome) = query(&state, method, &case["args"]) else {
            assert!(
                !C2_QUERIES.contains(&method) && !C3_QUERIES.contains(&method),
                "{what}: ported but not dispatched"
            );
            continue;
        };
        match outcome {
            Ok(v) => assert_json_eq(&v, case.get("result").unwrap_or(&Value::Null), &what),
            Err(e) => assert_json_eq(&error_json(&e), &case["error"], &what),
        }
        checked += 1;
    }
    eprintln!("engine-queries: {checked} cases checked");
    // Every case: 58 from C2 and the 6 `outstanding` ones from C3.
    assert_eq!(checked, 64);
}

#[test]
fn normal_balance_of_matches() {
    let f = fixture("setup");
    for case in cases(&f, "normalBalanceOf") {
        let t = serde_json::from_value(case["args"][0].clone()).unwrap();
        assert_eq!(to_json(&normal_balance_of(t)), case["result"]);
    }
}

#[test]
fn empty_ledger_matches() {
    let f = fixture("setup");
    for case in cases(&f, "emptyLedger") {
        let settings = serde_json::from_value(case["args"][0].clone()).unwrap();
        assert_json_eq(
            &to_json(&empty_ledger(settings)),
            &case["result"],
            "emptyLedger",
        );
    }
}
