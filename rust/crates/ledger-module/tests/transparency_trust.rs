//! The trust formula against `rust/fixtures/transparency-trust-score.json`, which pins the web's
//! `calculateTrustScore` and `getTrustGrade` (`flowtrace/src/lib/supabase.ts:603-621`) on a grid of
//! inputs, on the seed history and on the demo (`calculateTrustScoreAPI`, `api.ts:406-472`).
//!
//! The module's 32/24/24/20 formula equals the web's whenever the model has not scored and no flag
//! is confirmed (`ledger_module::transparency::trust`, module docs). With `m = 100` it does not:
//! the score is `0.8 × web + 20`, so the demo's 77 reads 82; this file pins that too.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use std::path::PathBuf;

use ledger_core::money::js_round;
use ledger_module::transparency::trust::{
    PENALTY_PER_FLAG, calculate_trust_score, compute, formula, trust_grade,
};
use ledger_module::transparency::ts;
use serde_json::Value;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/transparency-trust-score.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn ints(row: &Value) -> Vec<i64> {
    row.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect()
}

#[expect(clippy::cast_precision_loss, reason = "rates are 0 to 100")]
fn f(x: i64) -> f64 {
    x as f64
}

#[test]
fn the_web_formula_matches_every_pinned_output_float_for_float() {
    let fx = fixture();
    let cases = fx["formula_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 13 * 13 * 13);
    for case in cases {
        let [v, d, c, score] = ints(case)[..] else {
            panic!("case {case}")
        };
        assert_eq!(calculate_trust_score(f(v), f(d), f(c)), score, "{case}");
    }
    for case in fx["history_cases"].as_array().unwrap() {
        let [v, d, c, score, stored] = ints(case)[..] else {
            panic!("case {case}")
        };
        assert_eq!(calculate_trust_score(f(v), f(d), f(c)), score, "{case}");
        assert_eq!(score, stored, "the seed history follows the formula");
    }
}

#[test]
fn the_grade_matches_every_score() {
    for case in fixture()["grade_cases"].as_array().unwrap() {
        let score = case[0].as_i64().unwrap();
        assert_eq!(trust_grade(score), case[1].as_str().unwrap(), "{score}");
    }
}

#[test]
fn an_unscored_model_with_no_confirmed_flags_leaves_the_web_score_unchanged() {
    let fx = fixture();
    for case in fx["formula_cases"].as_array().unwrap() {
        let [v, d, c, score] = ints(case)[..] else {
            panic!("case {case}")
        };
        assert_eq!(formula(v, d, c, None, 0).1, score, "{case}");
    }
    // Every input the rates and consistency can take, not only the grid.
    for v in 0..=100 {
        for d in 0..=100 {
            for c in 0..=100 {
                assert_eq!(
                    formula(v, d, c, None, 0).1,
                    calculate_trust_score(f(v), f(d), f(c)),
                    "{v} {d} {c}"
                );
            }
        }
    }
}

#[test]
fn the_demo_scores_77_from_its_own_records_as_the_web_does() {
    let fx = fixture();
    let demo = &fx["demo"];
    let details = &demo["details"];
    let dataset: Value = serde_json::from_str(
        &std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/transparency-dataset.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let org = dataset["demo_org_id"].as_str().unwrap();
    let t = ts::dataset(org, &dataset["store"]).unwrap();
    let row = compute(&t, org, "2026-02-04T09:00:00.000Z", "module");
    assert_eq!(row.score, demo["trust_score"].as_i64().unwrap());
    assert_eq!(row.score, 77);
    assert_eq!(
        (
            row.components.verification,
            row.components.documentation,
            row.components.consistency,
            row.inputs.total_transactions,
            row.inputs.verified_transactions,
            row.inputs.documented_transactions
        ),
        (
            details["verification_rate"].as_i64().unwrap(),
            details["documentation_rate"].as_i64().unwrap(),
            details["consistency_score"].as_i64().unwrap(),
            details["total_transactions"].as_u64().unwrap(),
            details["verified_transactions"].as_u64().unwrap(),
            details["documented_transactions"].as_u64().unwrap()
        )
    );
    assert_eq!(row.grade, "B");
    // The neutral model reads as the web's unrounded score: 0.4 x 75 + 0.3 x 75 + 0.3 x 82.
    assert!((row.components.model - 77.1).abs() < 1e-9);
}

#[test]
fn with_m_at_100_the_new_formula_is_not_the_web_score() {
    // The demo: 75, 75, 82 scores 77 on the web formula and 82 with m = 100 and no flags.
    assert_eq!(formula(75, 75, 82, Some(100.0), 0), (100.0, 82));
    // In general it is 0.8 x the web's unrounded score + 20.
    for (v, d, c) in [(0, 0, 80), (50, 50, 50), (92, 85, 82), (100, 100, 100)] {
        let web = f(v) * 0.4 + f(d) * 0.3 + f(c) * 0.3;
        #[expect(clippy::cast_possible_truncation, reason = "a score fits i64")]
        let expected = js_round(0.8 * web + 20.0) as i64;
        assert_eq!(formula(v, d, c, Some(100.0), 0).1, expected, "{v} {d} {c}");
    }
    assert_eq!(formula(100, 100, 100, Some(100.0), 0).1, 100);
}

#[test]
fn each_confirmed_flag_takes_five_points_off_the_model_down_to_zero() {
    assert!((PENALTY_PER_FLAG - 5.0).abs() < f64::EPSILON);
    assert_eq!(formula(80, 80, 80, Some(90.0), 2).0, 80.0);
    assert_eq!(formula(80, 80, 80, Some(90.0), 30).0, 0.0);
    // 0.32 x 80 + 0.24 x 80 + 0.24 x 80 + 0.2 x 80 = 80.
    assert_eq!(formula(80, 80, 80, Some(90.0), 2).1, 80);
    // An unscored model is neutral before the penalty: the demo with one confirmed flag loses
    // one point of total (0.2 x 5).
    let (m, score) = formula(75, 75, 82, None, 1);
    assert!((m - 72.1).abs() < 1e-9);
    assert_eq!(score, 76);
}
