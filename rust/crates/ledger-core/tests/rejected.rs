//! Tasks C2 to C4 acceptance: every rejection in `rejected-cases.json`, raised by an engine method
//! or by the command layer (`sanitize_command` then `apply_command`), reproduces the exact code, message and issues, and leaves the books unchanged. Each scenario
//! runs its steps in order on fresh `createOrgLedger('org', 'KES', 2026)` books with the test env.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::fixture;
use common::replay::{Ran, ported, run_command_step, run_method_step};
use ledger_core::env::fixed;
use ledger_core::types::LedgerState;

#[test]
fn rejected_cases_match_for_the_ported_methods() {
    let f = fixture("rejected-cases");
    let setup = fixture("setup");
    let (mut checked, mut live_ok, mut skipped) = (0, 0, 0);
    for (n, scenario) in f["scenarios"].as_array().unwrap().iter().enumerate() {
        let mut state: LedgerState =
            serde_json::from_value(setup["createOrgLedger"]["state"].clone()).unwrap();
        let mut env = fixed::test();
        let name = scenario["test"].as_str().unwrap_or("sanitiser");
        for (i, step) in scenario["steps"].as_array().unwrap().iter().enumerate() {
            let what = format!("scenario {n} ({name}) step {i}");
            let ran = if step["op"] == "command" {
                run_command_step(&mut state, &mut env, step, &what)
            } else {
                run_method_step(&mut state, &mut env, step, &what)
            };
            let rejected = step["outcome"] == "rejected";
            match ran {
                Ran::Rejected => checked += 1,
                Ran::Live => live_ok += 1,
                Ran::Delta => {
                    let method = step["method"].as_str().unwrap_or("");
                    assert!(
                        !ported(method),
                        "{what}: {method} is ported but did not run"
                    );
                    if rejected {
                        skipped += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "rejected-cases: {checked} rejections matched, {skipped} left to later tasks, {live_ok} accepted steps live"
    );
    assert_eq!(checked + skipped, 37);
    // All 37: the 32 from the TS tests (C3 added the five in claims and bills; C4 the unknown
    // command and the entry.post with a string `fx_rate`) and the sanitiser scenario's five (C4).
    assert_eq!((checked, skipped), (37, 0));
}
