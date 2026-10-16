//! Task C5 acceptance: every case of the fifteen `report-*.json` fixtures (one per export of
//! `flowtrace/src/lib/ledger/reports.ts`, 344 cases) is reproduced by `ledger_core::reports`,
//! equal as JSON values (money exactly, ratios and months as the same `f64`), and each fixture
//! file reprints byte-equal with the Rust results in place of the TS ones.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::sync::LazyLock;

use common::replay::{arg, assert_json_eq, opt, to_json};
use common::{cases, fixture, fixture_path, fixture_text, js_number};
use ledger_core::reports::{
    self, ReportError, activities, balances, budget_vs_actual, cash_flow, day_before, descendants,
    financial_position, functional_expenses, general_ledger, grant_summary, ledger_signals,
    natural, posted_lines, trial_balance, warning_level,
};
use ledger_core::types::{GrantId, IsoDate, LedgerState, NormalBalance};
use serde::Serialize;
use serde_json::Value;

/// Every report fixture with its case count (`rust/fixtures/README.md`, "Files").
const REPORTS: [(&str, usize); 15] = [
    ("report-activities", 26),
    ("report-balances", 50),
    ("report-budget-vs-actual", 27),
    ("report-cash-flow", 13),
    ("report-day-before", 40),
    ("report-descendants", 26),
    ("report-financial-position", 20),
    ("report-functional-expenses", 26),
    ("report-general-ledger", 8),
    ("report-grant-summary", 45),
    ("report-ledger-signals", 12),
    ("report-natural", 6),
    ("report-posted-lines", 12),
    ("report-trial-balance", 24),
    ("report-warning-level", 9),
];

/// The two sets of books report cases read (`states` in each fixture).
struct Books {
    demo: LedgerState,
    commands: LedgerState,
}

static BOOKS: LazyLock<Books> = LazyLock::new(|| {
    let load = |file: &str, pointer: &str| -> LedgerState {
        serde_json::from_value(fixture(file).pointer(pointer).unwrap().clone()).unwrap()
    };
    Books {
        demo: load("demo-books", "/state"),
        commands: load("commands-accepted", "/final_state"),
    }
});

/// The books a case names: `demo-books` or `commands-final`.
fn books(case: &Value) -> &'static LedgerState {
    match case["state"].as_str() {
        Some("demo-books") => &BOOKS.demo,
        Some("commands-final") => &BOOKS.commands,
        other => panic!("no books named {other:?}"),
    }
}

fn ok<T: Serialize>(r: Result<T, ReportError>) -> Value {
    to_json(&r.unwrap())
}

/// Runs the case's report through `ledger_core::reports` and returns its result as JSON.
fn run(function: &str, case: &Value) -> Value {
    let a = &case["args"];
    match function {
        "dayBefore" => ok(day_before(&arg(a, 0))),
        "natural" => {
            let side: NormalBalance =
                serde_json::from_value(a[0]["normal_balance"].clone()).unwrap();
            to_json(&natural(side, arg(a, 1)))
        }
        "warningLevel" => to_json(&warning_level(js_number(&a[0]))),
        "descendants" => to_json(&descendants(books(case), &arg(a, 0))),
        "postedLines" => to_json(&posted_lines(books(case), &opt(a, 0))),
        "balances" => to_json(&balances(books(case), &opt(a, 0))),
        "trialBalance" => {
            let as_of: Option<IsoDate> = opt(a, 0);
            to_json(&trial_balance(books(case), as_of.as_ref(), opt(a, 1)))
        }
        "generalLedger" => ok(general_ledger(books(case), &opt(a, 0))),
        "financialPosition" => to_json(&financial_position(books(case), &arg(a, 0), opt(a, 1))),
        "activities" => to_json(&activities(books(case), &arg(a, 0), &arg(a, 1), opt(a, 2))),
        "functionalExpenses" => to_json(&functional_expenses(
            books(case),
            &arg(a, 0),
            &arg(a, 1),
            opt(a, 2),
        )),
        "cashFlow" => ok(cash_flow(books(case), &arg(a, 0), &arg(a, 1))),
        "budgetVsActual" => to_json(&budget_vs_actual(books(case), &opt(a, 0))),
        "grantSummary" => ok(grant_summary(books(case), &arg(a, 0), &arg(a, 1))),
        "ledgerSignals" => {
            let as_of: Option<IsoDate> = opt(a, 0);
            to_json(&ledger_signals(books(case), as_of.as_ref()))
        }
        other => panic!("no report named {other}"),
    }
}

/// Checks every case of `file` and that it has `expected` of them.
fn check(file: &str, expected: usize) {
    let f = fixture(file);
    let function = f["function"].as_str().unwrap();
    if let Some(states) = f["states"].as_object() {
        // The books each case names are the ones loaded in BOOKS.
        for (name, pointer) in states {
            let want = match name.as_str() {
                "demo-books" => "demo-books.json#/state",
                "commands-final" => "commands-accepted.json#/final_state",
                other => panic!("{file}: unknown books {other}"),
            };
            assert_eq!(pointer, want, "{file}: books {name}");
        }
    }
    let list = cases(&f, "cases");
    for (i, case) in list.iter().enumerate() {
        let what = format!(
            "{file} case {i}: {function}({}) on {}",
            case["args"], case["state"]
        );
        assert!(case.get("error").is_none(), "{what}: TS threw");
        assert_json_eq(&run(function, case), &case["result"], &what);
    }
    assert_eq!(list.len(), expected, "{file}: case count");
}

#[test]
fn the_fixtures_hold_fifteen_report_files_and_344_cases() {
    let mut on_disk: Vec<String> = std::fs::read_dir(fixture_path("x").parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with("report-"))
        .map(|n| n.trim_end_matches(".json").to_owned())
        .collect();
    on_disk.sort();
    let listed: Vec<&str> = REPORTS.iter().map(|(f, _)| *f).collect();
    assert_eq!(on_disk, listed);
    assert_eq!(REPORTS.iter().map(|(_, n)| n).sum::<usize>(), 344);
}

#[test]
fn activities_matches_every_case() {
    check("report-activities", 26);
}

#[test]
fn balances_matches_every_case() {
    check("report-balances", 50);
}

#[test]
fn budget_vs_actual_matches_every_case() {
    check("report-budget-vs-actual", 27);
}

#[test]
fn cash_flow_matches_every_case() {
    check("report-cash-flow", 13);
}

#[test]
fn day_before_matches_every_case() {
    check("report-day-before", 40);
}

#[test]
fn descendants_matches_every_case() {
    check("report-descendants", 26);
}

#[test]
fn financial_position_matches_every_case() {
    check("report-financial-position", 20);
}

#[test]
fn functional_expenses_matches_every_case() {
    check("report-functional-expenses", 26);
}

#[test]
fn general_ledger_matches_every_case() {
    check("report-general-ledger", 8);
}

#[test]
fn grant_summary_matches_every_case() {
    check("report-grant-summary", 45);
}

#[test]
fn ledger_signals_matches_every_case() {
    check("report-ledger-signals", 12);
}

#[test]
fn natural_matches_every_case() {
    check("report-natural", 6);
}

#[test]
fn posted_lines_matches_every_case() {
    check("report-posted-lines", 12);
}

#[test]
fn trial_balance_matches_every_case() {
    check("report-trial-balance", 24);
}

#[test]
fn warning_level_matches_every_case() {
    check("report-warning-level", 9);
}

/// Sorts every object's keys, so printing does not depend on the `serde_json` `preserve_order`
/// feature, which another workspace crate switches on (as in `state_roundtrip.rs`).
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

/// The first line where two texts differ, with its number, for a readable failure.
fn first_difference(actual: &str, expected: &str) -> Option<String> {
    let mut a = actual.lines();
    let mut e = expected.lines();
    for n in 1.. {
        match (a.next(), e.next()) {
            (None, None) => return None,
            (x, y) if x == y => {}
            (x, y) => return Some(format!("line {n}: actual {x:?}, expected {y:?}")),
        }
    }
    None
}

#[test]
fn every_report_fixture_reprints_byte_equal_from_the_rust_results() {
    for (file, _) in REPORTS {
        let mut f = fixture(file);
        let function = f["function"].as_str().unwrap().to_owned();
        for case in f["cases"].as_array_mut().unwrap() {
            let result = run(&function, case);
            case["result"] = result;
        }
        let printed = print(&f);
        let text = fixture_text(file);
        if let Some(diff) = first_difference(&printed, &text) {
            panic!("{file} does not reprint byte-equal: {diff}");
        }
    }
}

#[test]
fn report_errors_cover_what_ts_throws_on() {
    let bad: IsoDate = IsoDate::parse("2026-13-01").unwrap();
    assert_eq!(day_before(&bad), Err(ReportError::InvalidDate(bad.clone())));
    let first: IsoDate = IsoDate::parse("0000-01-01").unwrap();
    assert_eq!(
        day_before(&first),
        Err(ReportError::InvalidDate(first.clone()))
    );
    // JS dates carry a day past the month's end into the next month (2026-02-30 is 2 March),
    // so the day before it is 1 March.
    let overflow = IsoDate::parse("2026-02-30").unwrap();
    assert_eq!(day_before(&overflow).unwrap().as_str(), "2026-03-01");
    let missing = GrantId::from("grant-missing");
    let as_of = IsoDate::parse("2026-02-04").unwrap();
    assert_eq!(
        grant_summary(&BOOKS.demo, &missing, &as_of),
        Err(ReportError::UnknownGrant(missing.clone()))
    );
    let filter = reports::LineFilter {
        from: Some(bad.clone()),
        ..reports::LineFilter::default()
    };
    assert_eq!(
        general_ledger(&BOOKS.demo, &filter).map(|_| ()),
        Err(ReportError::InvalidDate(bad.clone()))
    );
    assert_eq!(
        cash_flow(&BOOKS.demo, &bad, &as_of).map(|_| ()),
        Err(ReportError::InvalidDate(bad))
    );
}
