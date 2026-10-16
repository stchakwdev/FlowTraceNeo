//! Task C6 acceptance for `reconcile.ts`: `candidatesFor` and `autoMatch` reproduce every case
//! of `reconcile-auto-match.json` exactly (the candidates of each open statement line before the
//! run, the result, and the statement lines after it), on the demo books before and after the
//! demo's own run and on the `reconcile.test.ts` scenario.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::replay::{Ran, assert_json_eq, run_method_step, to_json};
use common::{cases, fixture};
use ledger_core::engine::Ledger;
use ledger_core::env::fixed;
use ledger_core::reconcile::{DEFAULT_WINDOW_DAYS, auto_match, candidates_for};
use ledger_core::types::{Actor, BankAccountId, LedgerState};
use serde_json::{Map, Value, json};

/// The books a case runs on, by its `state` label.
fn books(f: &Value, label: &str) -> LedgerState {
    let value = match label {
        "inline: state_before_demo_auto_match" => f["state_before_demo_auto_match"].clone(),
        "demo-books.json#/state" => fixture("demo-books")["state"].clone(),
        "inline: test_scenario" => {
            let mut state: LedgerState =
                serde_json::from_value(fixture("setup")["createOrgLedger"]["state"].clone())
                    .unwrap();
            let mut env = fixed::test();
            for (i, step) in f["test_scenario"]["steps"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let what = format!("reconcile test scenario step {i}");
                assert_eq!(
                    run_method_step(&mut state, &mut env, step, &what),
                    Ran::Live,
                    "{what}"
                );
            }
            return state;
        }
        other => panic!("unknown state {other}"),
    };
    serde_json::from_value(value).unwrap()
}

#[test]
fn auto_match_cases_match() {
    let f = fixture("reconcile-auto-match");
    for case in cases(&f, "cases") {
        let what = format!("autoMatch: {}", case["label"]);
        let mut state = books(&f, case["state"].as_str().unwrap());
        let bank: BankAccountId = serde_json::from_value(case["args"][0].clone()).unwrap();
        let window = case["args"][1]
            .as_u64()
            .map_or(DEFAULT_WINDOW_DAYS, |w| u32::try_from(w).unwrap());
        let actor: Actor = serde_json::from_value(case["actor"].clone()).unwrap();

        let mut before = Map::new();
        for s in state
            .statement_lines
            .iter()
            .filter(|s| s.bank_account_id == bank && s.matched_line_id.is_none())
        {
            before.insert(
                s.id.as_str().to_owned(),
                to_json(&candidates_for(&state, s, window, &[])),
            );
        }
        assert_json_eq(
            &Value::Object(before),
            &case["candidates_before"],
            &format!("{what}: candidates before"),
        );

        let mut env = fixed::test();
        let result = auto_match(
            &mut Ledger::new(&mut state, &mut env),
            &actor,
            &bank,
            window,
        );
        assert_json_eq(
            &to_json(&result.unwrap()),
            &case["result"],
            &format!("{what}: result"),
        );
        let after: Vec<Value> = state
            .statement_lines
            .iter()
            .filter(|s| s.bank_account_id == bank)
            .map(|s| json!({ "id": s.id, "match_kind": s.match_kind, "matched_line_id": s.matched_line_id }))
            .collect();
        assert_json_eq(
            &Value::Array(after),
            &case["statement_lines_after"],
            &format!("{what}: statement lines after"),
        );
    }
}
