//! Error codes and messages against `rust/fixtures/rejected-cases.json` and the `validateEntry`
//! issues in `engine-queries.json`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeSet;

use common::{find_objects, fixture};
use ledger_core::errors::{Issue, IssueCode, LedgerError};
use ledger_core::money::format_major;
use serde_json::Value;

/// Every `LedgerError` object in a fixture.
fn ledger_errors(f: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    find_objects(
        f,
        &|m| m.get("name").and_then(Value::as_str) == Some("LedgerError"),
        &mut out,
    );
    out
}

/// Every issue object (`code` and `message`, no `name`) in a fixture.
fn issues(f: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    find_objects(
        f,
        &|m| m.contains_key("code") && m.contains_key("message") && !m.contains_key("name"),
        &mut out,
    );
    out
}

#[test]
fn issue_code_has_every_ts_code_once() {
    // errors.ts:4-32 lists 28 codes.
    assert_eq!(IssueCode::ALL.len(), 28);
    let names: BTreeSet<String> = IssueCode::ALL
        .iter()
        .map(|c| {
            serde_json::to_value(c)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(names.len(), 28);
    for code in IssueCode::ALL {
        assert_eq!(code.as_str(), serde_json::to_value(code).unwrap());
    }
}

#[test]
fn every_rejected_code_exists_as_a_variant() {
    let mut seen = BTreeSet::new();
    for name in ["rejected-cases", "engine-queries"] {
        let f = fixture(name);
        for issue in issues(&f) {
            let code: IssueCode = serde_json::from_value(issue["code"].clone()).unwrap();
            assert_eq!(serde_json::to_value(code).unwrap(), issue["code"]);
            seen.insert(code.as_str());
        }
    }
    // 26 of the 28 codes appear in the fixtures; no TS test or validateEntry input raises
    // INACTIVE_ACCOUNT or GRANT_NOT_STARTED.
    assert_eq!(seen.len(), 26, "{seen:?}");
    assert!(!seen.contains("INACTIVE_ACCOUNT") && !seen.contains("GRANT_NOT_STARTED"));
}

#[test]
fn every_rejection_renders_its_fixture_message_byte_equal() {
    let mut checked = 0;
    for name in ["rejected-cases", "engine-queries"] {
        let f = fixture(name);
        for err in ledger_errors(&f) {
            let issues: Vec<Issue> = serde_json::from_value(err["issues"].clone()).unwrap();
            assert_eq!(serde_json::to_value(&issues).unwrap(), err["issues"]);
            let error = LedgerError::new(issues).unwrap();
            assert_eq!(error.to_string(), err["message"].as_str().unwrap());
            assert_eq!(serde_json::to_value(error.code()).unwrap(), err["code"]);
            checked += 1;
        }
    }
    // 37 rejections plus the NOT_FOUND errors of the read methods.
    assert!(checked >= 37, "{checked}");
}

#[test]
fn rejected_cases_hold_37_rejections() {
    let f = fixture("rejected-cases");
    assert_eq!(ledger_errors(&f).len(), 37);
}

#[test]
fn every_validate_entry_issue_round_trips() {
    let f = fixture("engine-queries");
    let all = issues(&f);
    assert!(!all.is_empty(), "no issues in engine-queries");
    for issue in all {
        let parsed: Issue = serde_json::from_value(issue.clone()).unwrap();
        assert_eq!(&serde_json::to_value(&parsed).unwrap(), issue);
    }
}

#[test]
fn money_in_messages_is_two_decimals_without_separators() {
    let f = fixture("rejected-cases");
    let messages: BTreeSet<&str> = ledger_errors(&f)
        .into_iter()
        .map(|e| e["message"].as_str().unwrap())
        .collect();
    // engine.ts:408 and engine.ts:438 format `(minor / 100).toFixed(2)` after the currency.
    let unbalanced = format!("Debits and credits differ by KES {}.", format_major(1));
    assert!(messages.contains(unbalanced.as_str()), "{unbalanced}");
    let over = format!(
        "This takes Water grant over its award by KES {}.",
        format_major(1)
    );
    assert!(messages.contains(over.as_str()), "{over}");
    assert_eq!(format_major(350_000_000), "3500000.00");
}

#[test]
fn ledger_error_joins_issue_messages_with_one_space() {
    let error = LedgerError::new(vec![
        Issue::new(IssueCode::LineZero, "Line 2: enter a debit or a credit.").on_line(1, false),
        Issue::new(
            IssueCode::Unbalanced,
            "Debits and credits differ by KES 0.01.",
        ),
    ])
    .unwrap();
    assert_eq!(error.code(), IssueCode::LineZero);
    assert_eq!(
        error.to_string(),
        "Line 2: enter a debit or a credit. Debits and credits differ by KES 0.01."
    );
    let single = LedgerError::fail(IssueCode::NotFound, "Entry not found.");
    assert_eq!(single.issues().len(), 1);
    assert_eq!(single.to_string(), "Entry not found.");
    assert!(LedgerError::new(Vec::new()).is_none());
}
