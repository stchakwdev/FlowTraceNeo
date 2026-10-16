//! U8 acceptance: the Reconcile screen's results equal `rust/fixtures/reconcile-auto-match.json`
//! on the demo books. Every case of the fixture that runs on `demo-books.json#/state` is replayed
//! the way the screen runs it: the candidates its view model lists for each open line
//! (`reconcile::candidates`) against the fixture's `candidates_before`, then Auto-match
//! (`statement.autoMatch`, `reconcile::auto_match_command`) submitted through the in-process
//! backend against the fixture's `result` and `statement_lines_after`, and the statuses the
//! view model derives after it (`reconcile::line_rows`).

#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::path::PathBuf;

use desktop::backend::{InProcessBackend, LedgerBackend};
use desktop::reconcile::{
    AUTO_WINDOW, LineStatus, MANUAL_WINDOW, auto_match_command, candidates, line_rows,
};
use ledger_core::CommandResult;
use ledger_core::demo::DEMO_ORG_ID;
use ledger_core::types::{BankAccountId, OrganizationId, StatementLineId};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reconcile-auto-match.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn org() -> OrganizationId {
    OrganizationId::from(DEMO_ORG_ID)
}

/// The demo-books cases: label, bank account, window.
fn demo_cases(f: &Value) -> Vec<&Value> {
    let cases: Vec<_> = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["state"] == "demo-books.json#/state")
        .collect();
    assert_eq!(cases.len(), 3, "the M-Pesa till twice and bank-main once");
    cases
}

/// The fixture's view of a candidate list: line, entry, day gap, reference match.
fn shown(list: &[desktop::reconcile::CandidateRow]) -> Value {
    Value::Array(
        list.iter()
            .map(|c| {
                json!([
                    c.journal_line.as_str(),
                    c.entry.as_str(),
                    c.day_gap,
                    c.reference_match
                ])
            })
            .collect(),
    )
}

fn expected(list: &Value) -> Value {
    Value::Array(
        list.as_array()
            .unwrap()
            .iter()
            .map(|c| {
                json!([
                    c["line"]["id"],
                    c["entry"]["id"],
                    c["dayGap"].as_f64().unwrap() as i64,
                    c["referenceMatch"]
                ])
            })
            .collect(),
    )
}

#[test]
fn candidates_shown_equal_the_fixture() {
    let f = fixture();
    let backend = InProcessBackend::with_demo_and_empty().unwrap();
    let state = backend.state(&org()).unwrap();
    let mut checked = 0;
    for case in demo_cases(&f) {
        let window = case["args"][1]
            .as_u64()
            .map_or(AUTO_WINDOW, |w| u32::try_from(w).unwrap());
        for (id, list) in case["candidates_before"].as_object().unwrap() {
            let line = StatementLineId::from(id.as_str());
            assert_eq!(
                shown(&candidates(&state, &line, window)),
                expected(list),
                "{}: {id}",
                case["label"]
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 7);

    // The match box lists candidates at the screen's manual window (7 days). On the till the
    // fixture's lists at 3 and at 10 days are the same, so the 7-day list must be that list too.
    let till = &demo_cases(&f)[0];
    for (id, list) in till["candidates_before"].as_object().unwrap() {
        let line = StatementLineId::from(id.as_str());
        assert_eq!(
            shown(&candidates(&state, &line, MANUAL_WINDOW)),
            expected(list),
            "manual window: {id}"
        );
    }
}

#[test]
fn auto_match_through_the_backend_equals_the_fixture() {
    let f = fixture();
    for case in demo_cases(&f) {
        if !case["args"][1].is_null() {
            continue; // the screen runs Auto-match at the engine's default window only
        }
        let what = case["label"].as_str().unwrap();
        let bank = BankAccountId::from(case["args"][0].as_str().unwrap());
        let mut backend = InProcessBackend::with_demo_and_empty().unwrap();
        assert_eq!(
            serde_json::to_value(backend.actor(&org()).unwrap()).unwrap(),
            case["actor"],
            "{what}: the screen acts as the fixture's actor"
        );
        let CommandResult::AutoMatch { result } =
            backend.submit(&org(), auto_match_command(&bank)).unwrap()
        else {
            panic!("{what}: not an auto-match result");
        };
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            case["result"],
            "{what}"
        );

        let state = backend.state(&org()).unwrap();
        let rows = line_rows(&state, &bank);
        let after = case["statement_lines_after"].as_array().unwrap();
        assert_eq!(rows.len(), after.len(), "{what}");
        for want in after {
            let id = want["id"].as_str().unwrap();
            let s = state
                .statement_lines
                .iter()
                .find(|s| s.id.as_str() == id)
                .unwrap();
            assert_eq!(
                json!({ "id": id, "match_kind": s.match_kind, "matched_line_id": s.matched_line_id }),
                *want,
                "{what}: {id}"
            );
            let row = rows.iter().find(|r| r.id.as_str() == id).unwrap();
            let status = match want["match_kind"].as_str() {
                Some("auto") => vec![LineStatus::Auto],
                Some(_) => vec![LineStatus::Manual, LineStatus::Posted],
                None => vec![LineStatus::Unmatched, LineStatus::Ambiguous],
            };
            assert!(
                status.contains(&row.status),
                "{what}: {id} is {:?}",
                row.status
            );
            if row.status == LineStatus::Auto {
                let number = &row.entry.as_ref().unwrap().1;
                assert!(number.starts_with("JE-"), "{what}: {id} shows {number}");
            }
        }
        for matched in result.matched {
            let row = rows
                .iter()
                .find(|r| r.id == matched.statement_line_id)
                .unwrap();
            assert_eq!(row.status, LineStatus::Auto, "{what}");
        }
    }
}
