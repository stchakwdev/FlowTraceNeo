//! Tasks C2 to C4 acceptance: the demo backfill and the accepted commands replay through the
//! engine step by step, commands through `sanitize_command` and `apply_command`. Steps whose method is ported run live and must reproduce the fixture's return value
//! and state delta exactly (ids, timestamps, audit events); the rest apply their recorded delta.
//! See `common/replay.rs`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeMap;

use common::fixture;
use common::replay::{
    C2_METHODS, C3_METHODS, Ran, assert_json_eq, run_command_step, run_method_step, to_json,
};
use ledger_core::env::fixed;
use ledger_core::types::LedgerState;

#[test]
fn demo_backfill_replays_through_the_engine() {
    let f = fixture("demo-backfill");
    let mut state: LedgerState = serde_json::from_value(f["initial_state"].clone()).unwrap();
    let mut env = fixed::demo();
    let mut ran: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for step in f["steps"].as_array().unwrap() {
        let method = step["method"].as_str().unwrap();
        let what = format!("demo-backfill step {}", step["seq"]);
        let kind = run_method_step(&mut state, &mut env, step, &what);
        assert_ne!(
            kind,
            Ran::Rejected,
            "{what}: the backfill has no rejections"
        );
        let counts = ran.entry(method).or_default();
        if kind == Ran::Live {
            counts.0 += 1;
        } else {
            counts.1 += 1;
            assert!(
                !C2_METHODS.contains(&method) && !C3_METHODS.contains(&method),
                "{what}: {method} is ported but did not run live"
            );
        }
    }
    let books = common::fixture("demo-books");
    assert_json_eq(
        &to_json(&state),
        &books["state"],
        "the replayed books equal demo-books.json#/state",
    );
    let live: usize = ran.values().map(|c| c.0).sum();
    let delta: usize = ran.values().map(|c| c.1).sum();
    eprintln!("demo-backfill: {live} steps live, {delta} applied as deltas, by method {ran:?}");
    assert_eq!(live + delta, 108);
    // C2: ensureFiscalYear 2, addFund 3, addDimension 13, post 16, closePeriod 12, lockPeriod 11.
    // C3: addGrant 4, addBudget 1, setBudgetLine 15, addVendor 5, createBill 3, approveBill 3,
    // postBill 2, payBill 1, createClaim 2, approveClaim 1, reimburseClaim 1.
    // Left to C6: addBankAccount 4, importStatement 2, matchStatementLine 6,
    // postFromStatementLine 1.
    assert_eq!((live, delta), (95, 13));
}

#[test]
fn accepted_commands_replay_through_the_engine() {
    let f = fixture("commands-accepted");
    let books = fixture("demo-books");
    let mut state: LedgerState = serde_json::from_value(books["state"].clone()).unwrap();
    let mut env = fixed::fixture();
    let (mut live, mut delta) = (0, 0);
    for (i, step) in f["steps"].as_array().unwrap().iter().enumerate() {
        let what = format!("commands-accepted step {i}");
        match run_command_step(&mut state, &mut env, step, &what) {
            Ran::Live => live += 1,
            Ran::Delta => delta += 1,
            Ran::Rejected => panic!("{what}: every command here was accepted"),
        }
    }
    assert_json_eq(
        &to_json(&state),
        &f["final_state"],
        "the replayed books equal commands-accepted.json#/final_state",
    );
    eprintln!("commands-accepted: {live} commands live, {delta} applied as deltas");
    // C2: entry.* 9 (post 2, saveDraft 2, updateDraft, postDraft, deleteDraft, reverse, import),
    // account.* 2, period.* 4, year.* 2. C3: grant.add, budgetLine.set 2, vendor.add, bill.* 4,
    // claim.* 6 (create 2, submit, reject, approve, reimburse). Left to C6: bank.add and the five
    // statement.* commands.
    assert_eq!((live, delta), (31, 6));
}
