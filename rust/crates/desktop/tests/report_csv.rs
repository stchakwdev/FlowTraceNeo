//! The exported CSV of every report, parsed back, carries the totals the TS engine computed:
//! for each `report-*.json` case on the demo books, the CSV built at that case's parameters is
//! read with `ledger_core::csv::parse_csv` and its total rows are compared, in minor units, with
//! the case's result (`rust/fixtures/README.md`; the CSV layout is `reportCsv` in
//! `flowtrace/src/components/ledger/reports/statements.tsx`). Failures name the case index.

#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::path::PathBuf;

use desktop::statements::{ReportKind, ReportParams, export_csv, to_fixed};
use ledger_core::LedgerState;
use ledger_core::csv::parse_csv;
use ledger_core::reports::Basis;
use ledger_core::types::{GrantId, IsoDate};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn demo_books() -> LedgerState {
    serde_json::from_value(fixture("demo-books")["state"].clone()).unwrap()
}

/// The demo-books cases of `report-<name>.json`, with their index in the file.
fn demo_cases(name: &str) -> Vec<(usize, Value)> {
    let file = fixture(&format!("report-{name}"));
    let cases: Vec<(usize, Value)> = file["cases"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, case)| case["state"] == "demo-books")
        .collect();
    assert!(!cases.is_empty(), "report-{name}.json has demo-books cases");
    cases
}

fn date(v: &Value) -> IsoDate {
    IsoDate::parse(v.as_str().unwrap()).unwrap()
}

fn basis(v: &Value) -> Basis {
    match v.as_str().unwrap() {
        "cash" => Basis::Cash,
        _ => Basis::Accrual,
    }
}

/// A CSV amount (`csvAmount`, two decimals, `-` for negatives) back in minor units.
fn minor(text: &str) -> i64 {
    let (sign, digits) = text.strip_prefix('-').map_or((1, text), |rest| (-1, rest));
    let (whole, cents) = digits.split_once('.').unwrap();
    assert_eq!(cents.len(), 2, "two decimals in {text:?}");
    sign * (whole.parse::<i64>().unwrap() * 100 + cents.parse::<i64>().unwrap())
}

fn amount(v: &Value) -> i64 {
    v.as_i64().unwrap()
}

/// The parsed CSV of `params` on the demo books, exported twice to show it is stable.
fn exported(state: &LedgerState, params: &ReportParams) -> Vec<Vec<String>> {
    let first = export_csv(state, params).unwrap();
    let second = export_csv(state, params).unwrap();
    assert_eq!(first, second, "two exports of {params:?} are equal");
    assert!(first.ends_with("\r\n"));
    parse_csv(&first)
}

/// The first row whose cell `at` is `label`.
fn row<'a>(rows: &'a [Vec<String>], at: usize, label: &str, case: usize) -> &'a [String] {
    rows.iter()
        .find(|r| r.get(at).is_some_and(|c| c == label))
        .unwrap_or_else(|| panic!("case {case}: no row {label:?} in column {at}"))
}

fn point(kind: ReportKind, to: IsoDate, b: Basis) -> ReportParams {
    ReportParams {
        kind,
        from: to.clone(),
        to,
        basis: b,
        grant: None,
    }
}

fn range(kind: ReportKind, from: IsoDate, to: IsoDate, b: Basis) -> ReportParams {
    ReportParams {
        kind,
        from,
        to,
        basis: b,
        grant: None,
    }
}

#[test]
fn trial_balance_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("trial-balance") {
        let args = &case["args"];
        // The screen always has an as-at date; the fixture's no-date cases are not a screen.
        if args[0].is_null() {
            continue;
        }
        let rows = exported(
            &state,
            &point(ReportKind::TrialBalance, date(&args[0]), basis(&args[1])),
        );
        let r = &case["result"];
        let totals = row(&rows, 1, "Totals", i);
        assert_eq!(minor(&totals[2]), amount(&r["debit"]), "case {i} debit");
        assert_eq!(minor(&totals[3]), amount(&r["credit"]), "case {i} credit");
        let difference = row(&rows, 1, "Difference", i);
        assert_eq!(minor(&difference[2]), amount(&r["difference"]), "case {i}");
        let accounts = rows
            .iter()
            .skip_while(|r| r.first().is_none_or(|c| c != "Code"))
            .skip(1)
            .take_while(|r| r.first().is_some_and(|c| !c.is_empty()))
            .count();
        assert_eq!(
            accounts,
            r["rows"].as_array().unwrap().len(),
            "case {i} account rows"
        );
    }
}

#[test]
fn financial_position_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("financial-position") {
        let args = &case["args"];
        let rows = exported(
            &state,
            &point(
                ReportKind::FinancialPosition,
                date(&args[0]),
                basis(&args[1]),
            ),
        );
        let r = &case["result"];
        let line = |label: &str| minor(&row(&rows, 2, label, i)[3]);
        assert_eq!(line("Total assets"), amount(&r["totalAssets"]), "case {i}");
        assert_eq!(
            line("Total liabilities"),
            amount(&r["totalLiabilities"]),
            "case {i}"
        );
        assert_eq!(
            line("Without donor restrictions"),
            amount(&r["netAssets"]["unrestricted"]),
            "case {i}"
        );
        assert_eq!(
            line("With donor restrictions"),
            amount(&r["netAssets"]["restricted"]),
            "case {i}"
        );
        assert_eq!(
            line("Total net assets"),
            amount(&r["totalNetAssets"]),
            "case {i}"
        );
        assert_eq!(
            line("Total liabilities and net assets"),
            amount(&r["totalLiabilities"]) + amount(&r["totalNetAssets"]),
            "case {i}"
        );
        assert_eq!(line("Difference"), amount(&r["difference"]), "case {i}");
    }
}

#[test]
fn activities_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("activities") {
        let args = &case["args"];
        let rows = exported(
            &state,
            &range(
                ReportKind::Activities,
                date(&args[0]),
                date(&args[1]),
                basis(&args[2]),
            ),
        );
        let r = &case["result"];
        for (label, key) in [
            ("Total income", "totalIncome"),
            ("Total expenses", "totalExpenses"),
            ("Change in net assets", "changeInNetAssets"),
        ] {
            let cells = row(&rows, 2, label, i);
            let want = &r[key];
            assert_eq!(
                minor(&cells[3]),
                amount(&want["unrestricted"]),
                "case {i} {label}"
            );
            assert_eq!(
                minor(&cells[4]),
                amount(&want["restricted"]),
                "case {i} {label}"
            );
            assert_eq!(minor(&cells[5]), amount(&want["total"]), "case {i} {label}");
        }
        for fund in r["byFund"].as_array().unwrap() {
            let name = fund["fund"]["name"].as_str().unwrap();
            let cells = row(&rows, 0, name, i);
            assert_eq!(cells[1], fund["fund"]["restriction"].as_str().unwrap());
            assert_eq!(minor(&cells[2]), amount(&fund["income"]), "case {i} {name}");
            assert_eq!(
                minor(&cells[3]),
                amount(&fund["expenses"]),
                "case {i} {name}"
            );
            assert_eq!(minor(&cells[4]), amount(&fund["change"]), "case {i} {name}");
        }
    }
}

#[test]
fn functional_expenses_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("functional-expenses") {
        let args = &case["args"];
        let rows = exported(
            &state,
            &range(
                ReportKind::FunctionalExpenses,
                date(&args[0]),
                date(&args[1]),
                basis(&args[2]),
            ),
        );
        let t = &case["result"]["totals"];
        let totals = row(&rows, 1, "Total expenses", i);
        assert_eq!(minor(&totals[2]), amount(&t["program"]), "case {i}");
        assert_eq!(minor(&totals[3]), amount(&t["management"]), "case {i}");
        assert_eq!(minor(&totals[4]), amount(&t["fundraising"]), "case {i}");
        assert_eq!(minor(&totals[5]), amount(&t["total"]), "case {i}");
        let ratio = case["result"]["programRatio"].as_f64().unwrap();
        assert_eq!(
            row(&rows, 1, "Programme ratio", i)[2],
            to_fixed(ratio, 4),
            "case {i}"
        );
    }
}

#[test]
fn cash_flow_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("cash-flow") {
        let args = &case["args"];
        let rows = exported(
            &state,
            &range(
                ReportKind::CashFlow,
                date(&args[0]),
                date(&args[1]),
                Basis::Accrual,
            ),
        );
        let r = &case["result"];
        let line = |label: &str| minor(&row(&rows, 1, label, i)[2]);
        assert_eq!(
            line("Cash received"),
            amount(&r["operating"]["receipts"]),
            "case {i}"
        );
        assert_eq!(
            line("Cash paid"),
            amount(&r["operating"]["payments"]),
            "case {i}"
        );
        assert_eq!(
            line("Net cash from operating activities"),
            amount(&r["operating"]["net"]),
            "case {i}"
        );
        assert_eq!(
            line("Net cash from investing activities"),
            amount(&r["investing"]),
            "case {i}"
        );
        assert_eq!(
            line("Net cash from financing activities"),
            amount(&r["financing"]),
            "case {i}"
        );
        assert_eq!(line("Net change in cash"), amount(&r["net"]), "case {i}");
        assert_eq!(line("Opening cash"), amount(&r["opening"]), "case {i}");
        assert_eq!(line("Closing cash"), amount(&r["closing"]), "case {i}");
        assert_eq!(line("Difference"), amount(&r["difference"]), "case {i}");
    }
}

#[test]
fn grant_report_csv_totals_match_the_fixture() {
    let state = demo_books();
    for (i, case) in demo_cases("grant-summary") {
        let args = &case["args"];
        let params = ReportParams {
            grant: Some(GrantId::from(args[0].as_str().unwrap())),
            ..point(ReportKind::Grant, date(&args[1]), Basis::Accrual)
        };
        let rows = exported(&state, &params);
        let r = &case["result"];
        let line = |label: &str| minor(&row(&rows, 0, label, i)[1]);
        assert_eq!(line("Award"), amount(&r["award"]), "case {i}");
        assert_eq!(line("Received"), amount(&r["received"]), "case {i}");
        assert_eq!(line("Spent"), amount(&r["spent"]), "case {i}");
        assert_eq!(line("Committed"), amount(&r["committed"]), "case {i}");
        assert_eq!(line("Remaining"), amount(&r["remaining"]), "case {i}");
        assert_eq!(
            line("Burn per month"),
            amount(&r["burnPerMonth"]),
            "case {i}"
        );
        assert_eq!(
            row(&rows, 0, "Used", i)[1],
            to_fixed(r["used"].as_f64().unwrap(), 4),
            "case {i}"
        );
        assert_eq!(
            row(&rows, 0, "Status", i)[1],
            r["status"].as_str().unwrap(),
            "case {i}"
        );
    }
}

#[test]
fn an_export_written_twice_is_byte_equal() {
    let state = demo_books();
    let params = point(
        ReportKind::TrialBalance,
        IsoDate::parse("2026-02-04").unwrap(),
        Basis::Accrual,
    );
    let dir = std::env::temp_dir().join(format!("desktop-u9-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.csv");
    let b = dir.join("b.csv");
    desktop::statements::write_export(&state, &params, &a).unwrap();
    desktop::statements::write_export(&state, &params, &b).unwrap();
    let (a_bytes, b_bytes) = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!a_bytes.is_empty());
    assert_eq!(a_bytes, b_bytes);
}
