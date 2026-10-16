//! Task C4 acceptance: the command layer (`commands.ts`). The whole `commands-accepted.json` log
//! replays from the books it names through `sanitize_command` and `apply_command`: each
//! sanitised command, `CommandResult` and state delta equals the fixture's, and so do the final
//! books. The bank and statement commands (C6) are refused as not implemented and applied as
//! recorded deltas. Every command-layer rejection in `rejected-cases.json` matches exactly.
#![expect(clippy::unwrap_used, clippy::expect_used, reason = "tests may unwrap")]

mod common;

use common::fixture;
use common::replay::{
    C6_COMMANDS, apply_delta, assert_json_eq, diff_state, error_json, sync_env, to_json,
};
use ledger_core::commands::{CommandError, LedgerCommand, apply_command, sanitize_command};
use ledger_core::engine::{Error, Ledger};
use ledger_core::env::fixed;
use ledger_core::types::{Actor, LedgerState};
use serde_json::{Value, json};

/// The fields of an `account.update` patch the typed `AccountPatch` keeps; TS keeps the others
/// in the sanitised command and ignores them (`engine.ts:203-207`).
const PATCH_FIELDS: [&str; 3] = ["name", "is_active", "expense_function"];

/// The fixture's sanitised command as the typed command serialises it.
fn expected_sanitized(step: &Value) -> Value {
    let mut expected = step["sanitized"].clone();
    if expected["type"] == "account.update" {
        expected["patch"]
            .as_object_mut()
            .unwrap()
            .retain(|k, _| PATCH_FIELDS.contains(&k.as_str()));
    }
    expected
}

#[test]
fn command_log_replays_from_the_books_it_names() {
    let f = fixture("commands-accepted");
    assert_eq!(f["initial_state"], "demo-books.json#/state");
    let mut state: LedgerState =
        serde_json::from_value(fixture("demo-books")["state"].clone()).unwrap();
    let mut env = fixed::fixture();
    let (mut live, mut delta) = (0, 0);
    for (i, step) in f["steps"].as_array().unwrap().iter().enumerate() {
        let what = format!("commands-accepted step {i} {}", step["command"]["type"]);
        let cmd = sanitize_command(&step["command"])
            .unwrap_or_else(|e| panic!("{what}: sanitize_command refused it: {e}"));
        assert_json_eq(
            &to_json(&cmd),
            &expected_sanitized(step),
            &format!("{what}: sanitised"),
        );
        let actor: Actor = serde_json::from_value(step["actor"].clone()).unwrap();
        let before = to_json(&state);
        match apply_command(&mut Ledger::new(&mut state, &mut env), &actor, cmd) {
            Ok(result) => {
                assert_json_eq(
                    &to_json(&result),
                    &step["result"],
                    &format!("{what}: result"),
                );
                assert_json_eq(
                    &diff_state(&before, &to_json(&state)),
                    &step["delta"],
                    &format!("{what}: delta"),
                );
                live += 1;
            }
            Err(CommandError::NotImplemented(kind)) => {
                assert!(C6_COMMANDS.contains(&kind), "{what}: {kind} refused");
                assert_eq!(Some(kind), step["command"]["type"].as_str());
                assert_json_eq(&to_json(&state), &before, &format!("{what}: books changed"));
                let mut books = before;
                apply_delta(&mut books, &step["delta"]);
                state = serde_json::from_value(books).unwrap();
                sync_env(&mut env, &step["delta"]);
                sync_env(&mut env, &step["result"]);
                delta += 1;
            }
            Err(CommandError::Engine(e)) => panic!("{what}: rejected with {e}"),
        }
    }
    assert_json_eq(
        &to_json(&state),
        &f["final_state"],
        "the replayed books equal commands-accepted.json#/final_state",
    );
    eprintln!("command log: {live} commands live, {delta} C6 commands applied as deltas");
    assert_eq!((live, delta), (31, 6));
}

#[test]
fn every_command_type_round_trips_through_serde() {
    let f = fixture("commands-accepted");
    let mut kinds = Vec::new();
    for step in f["steps"].as_array().unwrap() {
        let expected = expected_sanitized(step);
        let cmd: LedgerCommand = serde_json::from_value(expected.clone()).unwrap();
        assert_json_eq(&to_json(&cmd), &expected, "LedgerCommand serde");
        let kind = expected["type"].as_str().unwrap().to_owned();
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    assert_eq!(kinds.len(), 32);
}

/// The command steps of `rejected-cases.json` the command layer itself decides: everything
/// `sanitize_command` refuses, and the `entry.post` whose string `fx_rate` reaches the engine.
fn command_layer_rejections() -> Vec<Value> {
    let f = fixture("rejected-cases");
    let mut steps = Vec::new();
    for scenario in f["scenarios"].as_array().unwrap() {
        for step in scenario["steps"].as_array().unwrap() {
            if step["op"] == "command" && step["outcome"] == "rejected" {
                steps.push(step.clone());
            }
        }
    }
    steps
}

#[test]
fn sanitiser_rejections_match() {
    let steps = command_layer_rejections();
    let mut checked = 0;
    for step in steps.iter().filter(|s| s["sanitized"].is_null()) {
        let what = format!("sanitize_command({})", step["command"]);
        let e = sanitize_command(&step["command"]).expect_err(&what);
        assert_json_eq(&error_json(&Error::Ledger(e)), &step["error"], &what);
        checked += 1;
    }
    // Two unknown commands (`{ type: 'nope' }` and `null`), an entry with no input, a memo that is
    // not a string, an import that is not a list, and an imported entry with no lines.
    assert_eq!(checked, 6);
}

#[test]
fn a_string_rate_is_refused_as_fx_rate_required() {
    let steps = command_layer_rejections();
    let step = steps
        .iter()
        .find(|s| !s["sanitized"].is_null())
        .expect("the string fx_rate case");
    let mut state: LedgerState =
        serde_json::from_value(fixture("setup")["createOrgLedger"]["state"].clone()).unwrap();
    let before = to_json(&state);
    let mut env = fixed::test();
    let cmd = sanitize_command(&step["command"]).unwrap();
    let actor: Actor = serde_json::from_value(step["actor"].clone()).unwrap();
    let Err(CommandError::Engine(e)) =
        apply_command(&mut Ledger::new(&mut state, &mut env), &actor, cmd)
    else {
        panic!("the string fx_rate entry was not refused by the engine");
    };
    assert_json_eq(&error_json(&e), &step["error"], "string fx_rate");
    assert_json_eq(&to_json(&state), &before, "books unchanged");
}

#[test]
fn caller_chosen_ids_are_dropped() {
    for kind in [
        "account.add",
        "grant.add",
        "vendor.add",
        "bank.add",
        "bill.create",
    ] {
        let f = fixture("commands-accepted");
        let step = f["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["command"]["type"] == kind)
            .unwrap();
        let mut raw = step["command"].clone();
        raw["input"]["id"] = json!("chosen-by-the-client");
        let cmd = to_json(&sanitize_command(&raw).unwrap());
        assert_eq!(cmd["input"].get("id"), None, "{kind}");
    }
    // A claim keeps its client id, so an offline resend is idempotent (`commands.ts:91-93`).
    let raw = json!({
        "type": "claim.create",
        "input": {
            "id": "claim-offline-1", "claim_date": "2026-02-04", "currency": "KES", "fx_rate": 1,
            "purpose": "x", "lines": [], "receipt_names": [], "captured_offline": true
        }
    });
    assert_eq!(
        to_json(&sanitize_command(&raw).unwrap())["input"]["id"],
        "claim-offline-1"
    );
}

#[test]
fn entries_from_a_client_are_manual_or_import() {
    let entry = json!({
        "date": "2026-02-01", "memo": "x", "lines": [],
        "source_type": "bill", "source_id": "bill-1"
    });
    for (raw, source) in [
        (json!({ "type": "entry.post", "input": entry }), "manual"),
        (
            json!({ "type": "entry.saveDraft", "input": entry }),
            "manual",
        ),
        (
            json!({ "type": "entry.updateDraft", "entryId": "e", "input": entry }),
            "manual",
        ),
    ] {
        let cmd = to_json(&sanitize_command(&raw).unwrap());
        assert_eq!(cmd["input"]["source_type"], source);
        assert_eq!(cmd["input"]["source_id"], Value::Null);
    }
    let raw = json!({ "type": "entry.import", "inputs": [entry, entry] });
    let cmd = to_json(&sanitize_command(&raw).unwrap());
    for input in cmd["inputs"].as_array().unwrap() {
        assert_eq!(input["source_type"], "import");
        assert_eq!(input["source_id"], Value::Null);
    }
}

#[test]
fn a_malformed_command_is_invalid_input() {
    for raw in [
        json!({ "type": "entry.reverse", "entryId": "e", "date": "2026-02-01" }),
        json!({ "type": "year.open", "year": "2027" }),
        json!({ "type": "bill.pay", "billId": "b" }),
    ] {
        let e = sanitize_command(&raw).expect_err("malformed");
        assert_eq!(to_json(&e.code()), "INVALID_INPUT", "{raw}");
    }
}

#[test]
fn a_failed_import_leaves_the_books_unchanged() {
    // The second draft is in USD with no rate, which `convert` refuses (`money.ts:36-39`);
    // TS discards the first draft with the books (`commands.ts:86-89`), Rust restores them.
    let mut state: LedgerState =
        serde_json::from_value(fixture("setup")["createOrgLedger"]["state"].clone()).unwrap();
    let before = to_json(&state);
    let mut env = fixed::test();
    let line = |debit: bool, currency: &str| {
        let mut l = json!({ "account_id": "acc-1110", "fund_id": "fund-unrestricted", "currency": currency });
        l[if debit { "debit" } else { "credit" }] = json!(100);
        l
    };
    let raw = json!({ "type": "entry.import", "inputs": [
        { "date": "2026-02-01", "memo": "ok", "lines": [line(true, "KES"), line(false, "KES")] },
        { "date": "2026-02-01", "memo": "no rate", "lines": [line(true, "USD"), line(false, "USD")] },
    ]});
    let actor: Actor = serde_json::from_value(json!({
        "id": "u1", "name": "Admin", "can_post": true, "can_approve": true, "can_close": true
    }))
    .unwrap();
    let cmd = sanitize_command(&raw).unwrap();
    let outcome = apply_command(&mut Ledger::new(&mut state, &mut env), &actor, cmd);
    assert!(
        matches!(outcome, Err(CommandError::Engine(Error::Money(_)))),
        "{outcome:?}"
    );
    assert_json_eq(
        &to_json(&state),
        &before,
        "books unchanged after a failed import",
    );
}
