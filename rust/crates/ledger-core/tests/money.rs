//! Every case in `rust/fixtures/money.json` against `ledger_core::money`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeSet;

use common::{cases, fixture, js_number};
use ledger_core::money::{self, MoneyError};
use serde_json::{Value, json};

/// Checks one case: `result` against `Ok`, or `error` (a `RangeError`) against `Err`.
fn check(case: &Value, got: Result<Value, MoneyError>) {
    match (case.get("error"), got) {
        (Some(err), Err(e)) => {
            assert_eq!(err["name"], "RangeError", "{case}");
            assert_eq!(e.to_string(), err["message"].as_str().unwrap(), "{case}");
        }
        (None, Ok(v)) => assert_eq!(v, case["result"], "{case}"),
        (want, got) => panic!("{case}: wanted {want:?}, got {got:?}"),
    }
}

#[test]
fn money_fixture_covers_exactly_the_functions_tested_here() {
    let f = fixture("money");
    let covers: BTreeSet<&str> = f["covers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let tested = BTreeSet::from([
        "assertMinor",
        "convert",
        "fromMajor",
        "parseMoney",
        "sum",
        "toMajor",
    ]);
    assert_eq!(covers, tested);
}

#[test]
fn assert_minor_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "assertMinor") {
        let args = case["args"].as_array().unwrap();
        let what = args.get(1).map_or(money::AMOUNT, |v| v.as_str().unwrap());
        let got = money::assert_minor(js_number(&args[0]), what).map(|_| Value::Null);
        check(case, got);
    }
}

#[test]
fn parse_money_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "parseMoney") {
        let raw = case["args"][0].as_str().unwrap();
        check(case, Ok(json!(money::parse_money(raw))));
    }
}

#[test]
fn to_major_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "toMajor") {
        let minor = case["args"][0].as_i64().unwrap();
        assert_eq!(money::to_major(minor), js_number(&case["result"]), "{case}");
    }
}

#[test]
fn from_major_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "fromMajor") {
        let got = money::from_major(js_number(&case["args"][0])).map(|m| json!(m));
        check(case, got);
    }
}

#[test]
fn convert_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "convert") {
        let amount = case["args"][0].as_i64().unwrap();
        let rate = js_number(&case["args"][1]);
        check(case, money::convert(amount, rate).map(|m| json!(m)));
    }
}

#[test]
fn sum_matches_every_fixture_case() {
    let f = fixture("money");
    for case in cases(&f, "sum") {
        let xs: Vec<i64> = case["args"][0]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        check(case, Ok(json!(money::sum(xs))));
    }
}

#[test]
fn money_fixture_has_the_expected_case_count() {
    let f = fixture("money");
    let total: usize = [
        "assertMinor",
        "convert",
        "fromMajor",
        "parseMoney",
        "sum",
        "toMajor",
    ]
    .iter()
    .map(|k| cases(&f, k).len())
    .sum();
    assert_eq!(total, 59);
}

#[test]
fn js_round_breaks_ties_toward_positive_infinity() {
    // Math.round, as bun 1.x evaluates it: -2.5 -> -2, 2.5 -> 3, 0.49999999999999994 -> 0.
    assert_eq!(money::js_round(-2.5), -2.0);
    assert_eq!(money::js_round(2.5), 3.0);
    assert_eq!(money::js_round(-0.5), 0.0);
    assert_eq!(money::js_round(0.499_999_999_999_999_94), 0.0);
    assert!(money::js_round(f64::NAN).is_nan());
}

#[test]
fn format_major_matches_js_to_fixed_two() {
    // Expected strings are `(minor / 100).toFixed(2)` as bun 1.x prints them. Above about 2^46
    // the f64 quotient is not the nearest cent, so TS prints a neighbouring cent; Rust follows.
    let cases: [(i64, &str); 10] = [
        (1, "0.01"),
        (-5, "-0.05"),
        (0, "0.00"),
        (350_000_000, "3500000.00"),
        (7_036_874_417_766_413, "70368744177664.13"),
        (-7_036_874_417_766_413, "-70368744177664.13"),
        (7_036_874_417_766_401, "70368744177664.02"),
        (7_036_874_417_766_407, "70368744177664.06"),
        (7_036_874_417_766_412, "70368744177664.13"),
        (9_007_199_254_740_910, "90071992547409.09"),
    ];
    for (minor, want) in cases {
        assert_eq!(money::format_major(minor), want, "{minor}");
    }
}
