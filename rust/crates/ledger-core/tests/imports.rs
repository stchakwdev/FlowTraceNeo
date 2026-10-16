//! Task C6 acceptance for the file readers: `csv.ts` (`csv.json`), statement parsing in
//! `reconcile.ts` (`reconcile-parse-statement.json`) and the journal importers of `importers.ts`
//! (`importers-journal.json`). Every case matches exactly; the inputs marked `input_source:
//! synthetic` are made up, their outputs come from the TS functions.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::replay::{assert_json_eq, to_json};
use common::{cases, fixture};
use ledger_core::csv::{CsvCell, parse_csv, parse_csv_records, parse_statement_date, to_csv};
use ledger_core::engine::EntryOptions;
use ledger_core::importers::{detect_journal_source, parse_journal_csv};
use ledger_core::reconcile::{detect_format, parse_statement};
use ledger_core::types::{FundId, LedgerState};
use serde_json::Value;

fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}

fn headers(v: &Value) -> Vec<String> {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn parse_csv_matches() {
    let f = fixture("csv");
    for case in cases(&f, "parseCsv") {
        let what = format!("parseCsv({})", case["args"][0]);
        assert_json_eq(
            &to_json(&parse_csv(text(&case["args"][0]))),
            &case["result"],
            &what,
        );
    }
}

#[test]
fn parse_csv_records_matches() {
    let f = fixture("csv");
    for case in cases(&f, "parseCsvRecords") {
        let what = format!("parseCsvRecords({})", case["args"][0]);
        let got = parse_csv_records(text(&case["args"][0]));
        assert_json_eq(&to_json(&got), &case["result"], &what);
    }
}

#[test]
fn to_csv_matches() {
    let f = fixture("csv");
    for case in cases(&f, "toCsv") {
        let rows: Vec<Vec<CsvCell>> = serde_json::from_value(case["args"][0].clone()).unwrap();
        assert_eq!(
            to_csv(&rows),
            text(&case["result"]),
            "toCsv({})",
            case["args"][0]
        );
    }
}

#[test]
fn parse_statement_date_matches() {
    let f = fixture("csv");
    for case in cases(&f, "parseStatementDate") {
        let got = parse_statement_date(text(&case["args"][0]));
        assert_json_eq(
            &to_json(&got),
            &case["result"],
            &format!("parseStatementDate({})", case["args"][0]),
        );
    }
}

#[test]
fn detect_format_matches() {
    let f = fixture("reconcile-parse-statement");
    for case in cases(&f, "detect_format") {
        let got = detect_format(&headers(&case["args"][0]));
        assert_json_eq(
            &to_json(&got),
            &case["result"],
            &format!("detectFormat({})", case["args"][0]),
        );
    }
}

#[test]
fn parse_statement_matches() {
    let f = fixture("reconcile-parse-statement");
    for case in cases(&f, "cases") {
        let what = format!("parseStatement: {}", case["name"]);
        assert_json_eq(
            &to_json(&parse_statement(text(&case["text"]))),
            &case["result"],
            &what,
        );
    }
}

#[test]
fn detect_journal_source_matches() {
    let f = fixture("importers-journal");
    for case in cases(&f, "detect_journal_source") {
        let got = detect_journal_source(&headers(&case["args"][0]));
        assert_json_eq(
            &to_json(&got),
            &case["result"],
            &format!("detectJournalSource({})", case["args"][0]),
        );
    }
}

#[test]
fn parse_journal_csv_matches_and_validates() {
    let f = fixture("importers-journal");
    let state: LedgerState =
        serde_json::from_value(fixture("setup")["createOrgLedger"]["state"].clone()).unwrap();
    for case in cases(&f, "cases") {
        let what = format!("parseJournalCsv: {}", case["name"]);
        let fund: FundId = serde_json::from_value(case["args"]["fundId"].clone()).unwrap();
        let got = parse_journal_csv(text(&case["text"]), &state, &fund);
        assert_json_eq(&to_json(&got), &case["result"], &what);
        let issues: Vec<Value> = got
            .entries
            .iter()
            .map(|e| {
                to_json(
                    &state
                        .validate_entry(e, None, EntryOptions::default())
                        .unwrap(),
                )
            })
            .collect();
        assert_json_eq(
            &Value::Array(issues),
            &case["validate"],
            &format!("{what}: validate"),
        );
    }
}
