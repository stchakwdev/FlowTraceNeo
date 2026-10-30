#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use super::*;
use crate::backend::demo_transparency;
use ledger_module::transparency::flows::ProofStats;

fn snapshot(score: i64, at: &str) -> TrustHistory {
    TrustHistory {
        id: format!("h-{at}"),
        organization_id: "org".to_owned(),
        score,
        breakdown: std::collections::BTreeMap::new(),
        verification_rate: None,
        documentation_rate: None,
        consistency_score: None,
        calculated_at: at.to_owned(),
        model_score: None,
        grade: None,
    }
}

#[test]
fn the_demo_page_reads_the_module_row_and_its_eight_snapshots() {
    let t = demo_transparency().unwrap();
    let page = TrustPage::of(&t);
    let row = t.trust.as_ref().unwrap();
    assert_eq!(page.score, row.score);
    assert_eq!(page.grade, row.grade);
    assert_eq!(page.history_meta.as_deref(), Some("last 8 calculations"));
    assert_eq!(
        page.change,
        ScoreChange::Rows {
            since_previous: ("Since 1 Jan 2026".to_owned(), "+2 pts".to_owned()),
            since_first: ("Since 1 Jul 2025".to_owned(), "+25 pts".to_owned()),
            last: "as of 1 Feb 2026".to_owned(),
        }
    );
    let labels: Vec<&str> = page.components.iter().map(|c| c.label).collect();
    assert_eq!(
        labels,
        [
            "Verification",
            "Documentation",
            "Consistency",
            "Independent review"
        ]
    );
    let weights: Vec<&str> = page.components.iter().map(|c| c.weight.as_str()).collect();
    assert_eq!(
        weights,
        [
            "32% of score",
            "24% of score",
            "24% of score",
            "20% of score"
        ],
        "the module's four weights (P16), not the web's old 40/30/30"
    );
    assert_eq!(
        page.components[3].value, None,
        "the model part reads Not yet scored while neutral"
    );
    assert!(page.components[3].filled.abs() < f64::EPSILON);
}

#[test]
fn a_component_adds_its_weighted_points_and_rounds_its_value() {
    let row = ComponentRow::of(0, 87.0, 32, true);
    assert_eq!(row.value.as_deref(), Some("87"));
    assert_eq!(
        row.explain,
        "Share of movements whose proof has been checked and accepted. Adds 28 points."
    );
    let model = ComponentRow::of(3, 72.5, 20, false);
    assert_eq!(model.value, None);
    assert!(
        model.explain.ends_with(" Adds 15 points."),
        "{}",
        model.explain
    );
    let scored = ComponentRow::of(3, 72.5, 20, true);
    assert_eq!(scored.value.as_deref(), Some("73"), "Math.round, ties up");
}

#[test]
fn an_empty_organisation_has_no_score_parts_or_history() {
    let page = TrustPage::of(&OrgTransparency::default());
    assert_eq!(page.score, 0);
    assert_eq!(page.grade, "F");
    assert_eq!(page.calculated, None);
    assert!(page.components.is_empty());
    assert_eq!(page.history_meta, None);
    assert_eq!(page.chart, None);
    assert_eq!(page.change, ScoreChange::NotYet);
    assert!(page.actions.is_empty(), "Nothing outstanding");
}

#[test]
fn the_chart_floor_ticks_and_labels_follow_the_web() {
    let months = [
        "2025-07-01T00:00:00Z",
        "2025-08-01T00:00:00Z",
        "2025-09-01T00:00:00Z",
        "2025-10-01T00:00:00Z",
        "2025-11-01T00:00:00Z",
        "2025-12-01T00:00:00Z",
        "2026-01-01T00:00:00Z",
        "2026-02-01T00:00:00Z",
    ];
    let scores = [62, 68, 72, 75, 78, 82, 85, 87];
    let h: Vec<_> = scores
        .iter()
        .zip(months)
        .map(|(s, m)| snapshot(*s, m))
        .collect();
    let c = HistoryChart::of(&h).unwrap();
    assert_eq!(c.lo, 50, "floor((62 - 5) / 10) * 10");
    let ticks: Vec<i64> = c.ticks.iter().map(|t| t.0).collect();
    assert_eq!(
        ticks,
        [50, 75, 100],
        "step 25 while the floor is 50 or less"
    );
    assert!((c.ticks[0].1 - 100.0).abs() < 1e-9);
    assert!(c.ticks[2].1.abs() < 1e-9);
    let labels: Vec<&str> = c.labels.iter().map(|l| l.1.as_str()).collect();
    assert_eq!(
        labels,
        ["Aug 25", "Oct 25", "Dec 25", "Feb 26"],
        "every second label past six points, ending on the latest"
    );
    assert!((c.points[0].0).abs() < 1e-9);
    assert!((c.points[7].0 - 100.0).abs() < 1e-9);
    assert!((c.points[7].1 - 26.0).abs() < 1e-9, "(100 - 87) / 50");
    assert_eq!(c.last_score, 87);
    assert_eq!(
        c.summary,
        "Trust Score from 62 on 1 Jul 2025 to 87 on 1 Feb 2026"
    );

    let high = HistoryChart::of(&[snapshot(88, "2026-03-01"), snapshot(93, "2026-04-01")]).unwrap();
    assert_eq!(high.lo, 80);
    let ticks: Vec<i64> = high.ticks.iter().map(|t| t.0).collect();
    assert_eq!(ticks, [80, 90, 100], "step 10 above 50");
    assert_eq!(high.labels.len(), 2, "every label up to six points");

    let low = HistoryChart::of(&[snapshot(3, "2026-03-01")]).unwrap();
    assert_eq!(low.lo, 0, "never below zero");
    let ticks: Vec<i64> = low.ticks.iter().map(|t| t.0).collect();
    assert_eq!(ticks, [0, 25, 50, 75, 100]);
    assert!(
        (low.points[0].0 - 50.0).abs() < 1e-9,
        "one point sits in the middle"
    );

    let odd = HistoryChart::of(&[snapshot(57, "2026-03-01")]).unwrap();
    let ticks: Vec<i64> = odd.ticks.iter().map(|t| t.0).collect();
    assert_eq!(ticks, [50, 75, 100]);
    let sixty = HistoryChart::of(&[snapshot(66, "2026-03-01")]).unwrap();
    let ticks: Vec<i64> = sixty.ticks.iter().map(|t| t.0).collect();
    assert_eq!(ticks, [60, 70, 80, 90, 100]);
}

#[test]
fn a_month_label_is_short_month_and_two_digit_year() {
    assert_eq!(month_label("2025-09-01T00:00:00Z"), "Sep 25");
    assert_eq!(month_label("2026-12-31"), "Dec 26");
}

fn stats(
    total: u64,
    pending: u64,
    flagged: u64,
    with_proofs: u64,
    pending_proofs: u64,
) -> OrgStats {
    OrgStats {
        total,
        verified: total - pending - flagged,
        pending,
        flagged,
        verification_rate: 0,
        proof_stats: ProofStats {
            total_proofs: with_proofs,
            verified_proofs: 0,
            pending_proofs,
            rejected_proofs: 0,
            total_trust_points: 0,
            transactions_with_proofs: with_proofs,
            proof_coverage_rate: flows_rate(with_proofs, total),
        },
    }
}

fn flows_rate(part: u64, whole: u64) -> i64 {
    ledger_module::transparency::flows::rate(part, whole)
}

#[test]
fn raise_actions_say_what_would_lift_the_score() {
    let a = raise_actions(&stats(10, 3, 1, 7, 1), 32, 24, Some("tx-9"));
    let keys: Vec<&str> = a.iter().map(|x| x.key).collect();
    assert_eq!(keys, ["pending", "flagged", "unproven", "proofs"]);
    assert_eq!(a[0].title, "3 movements are awaiting proof");
    assert_eq!(
        a[0].detail,
        "Attach a receipt, photo or statement to each. Evidencing all of them adds up to 10 points."
    );
    assert_eq!(
        a[1].link,
        Some((
            "Review the flagged movement",
            RaiseLink::Transaction("tx-9".to_owned())
        ))
    );
    assert_eq!(a[1].title, "1 movement is flagged");
    assert_eq!(a[2].title, "3 movements have no proof attached");
    assert_eq!(
        a[2].detail,
        "Documentation is 70% covered and counts for 24% of the score."
    );
    assert_eq!(a[3].title, "1 proof is waiting for review");
    assert_eq!(
        a[3].detail,
        "A reviewer can accept them from each transaction\u{2019}s page."
    );

    let two_flags = raise_actions(&stats(4, 0, 2, 4, 0), 32, 24, None);
    assert_eq!(
        two_flags[0].link,
        Some((
            "Review flagged movements",
            RaiseLink::Transactions(TransactionStatus::Flagged)
        ))
    );
    assert!(raise_actions(&stats(5, 0, 0, 5, 0), 32, 24, None).is_empty());
}

#[test]
fn the_recalculate_error_names_the_reason_when_there_is_one() {
    assert_eq!(
        recalc_error("offline"),
        "The score could not be recalculated (offline). The figures below are still current; try again in a moment."
    );
    assert!(recalc_error("").starts_with("The score could not be recalculated. "));
}

#[test]
fn ledger_signals_read_the_demo_books_as_percentages() {
    let state = ledger_core::demo::build_demo_ledger().unwrap();
    let as_of = IsoDate::parse(ledger_core::demo::DEMO_AS_OF).unwrap();
    let s = signals(&state, &as_of);
    let labels: Vec<&str> = s.iter().map(|x| x.label).collect();
    assert_eq!(
        labels,
        [
            "Cash movement reconciled",
            "Spend within budget",
            "Spend coded to a grant"
        ]
    );
    let raw = ledger_signals(&state, Some(&as_of));
    assert_eq!(
        s[0].value,
        format!("{}%", js_round(raw.reconciled_share * 100.0))
    );
    assert!(s.iter().all(|x| x.value.ends_with('%')));
}
