//! Task C5: the reports foot and cross-foot on the demo books (`reports.ts:1-4`, and
//! `Docs/flowtrace-ledger-plan.md` Phase 4, "the reports from the seeded data foot and
//! cross-foot"). The fixtures already imply most of this; these tests say it by name, on both
//! bases and over every as-of date and range the fixtures use. The commands' final books (with
//! drafts, a reversal, a two-fund USD bill and a year-end close) are checked too where the
//! identity holds for them.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::fixture;
use ledger_core::reports::{
    Basis, activities, cash_flow, day_before, financial_position, functional_expenses,
    grant_summary, trial_balance,
};
use ledger_core::types::{AccountType, IsoDate, LedgerState};

const BASES: [Basis; 2] = [Basis::Accrual, Basis::Cash];

/// As-of dates of the report fixtures: the 2025 close, month ends, the demo as-of date, year end.
const AS_OF: [&str; 5] = [
    "2025-12-31",
    "2026-01-31",
    "2026-02-04",
    "2026-02-28",
    "2026-12-31",
];

/// Ranges inside 2026, after the opening balances (dated 2025-12-31) that post to net assets
/// directly, so activity in the range is all the change in net assets.
const RANGES: [(&str, &str); 5] = [
    ("2026-01-01", "2026-01-31"),
    ("2026-02-01", "2026-02-28"),
    ("2026-01-01", "2026-02-04"),
    ("2026-01-01", "2026-12-31"),
    ("2026-12-01", "2026-12-31"),
];

fn date(s: &str) -> IsoDate {
    IsoDate::parse(s).unwrap()
}

fn demo() -> LedgerState {
    serde_json::from_value(fixture("demo-books")["state"].clone()).unwrap()
}

fn commands_final() -> LedgerState {
    serde_json::from_value(fixture("commands-accepted")["final_state"].clone()).unwrap()
}

#[test]
fn trial_balance_debits_equal_credits() {
    for state in [demo(), commands_final()] {
        for basis in BASES {
            let dates = AS_OF.map(|d| Some(date(d)));
            for as_of in dates.iter().chain([&None]) {
                let tb = trial_balance(&state, as_of.as_ref(), basis);
                let what = format!("{as_of:?} {basis:?}");
                assert!(tb.debit > 0, "{what}: empty trial balance");
                assert_eq!(tb.debit, tb.credit, "{what}");
                assert_eq!(tb.difference, 0, "{what}");
                let debits: i64 = tb.rows.iter().map(|r| r.debit).sum();
                let credits: i64 = tb.rows.iter().map(|r| r.credit).sum();
                assert_eq!((debits, credits), (tb.debit, tb.credit), "{what}: columns");
                assert!(tb.rows.iter().all(|r| (r.debit == 0) != (r.credit == 0)));
            }
        }
    }
}

#[test]
fn financial_position_assets_equal_liabilities_plus_net_assets() {
    for state in [demo(), commands_final()] {
        for basis in BASES {
            for as_of in AS_OF {
                let fp = financial_position(&state, &date(as_of), basis);
                let what = format!("{as_of} {basis:?}");
                assert!(fp.total_assets > 0, "{what}: no assets");
                assert_eq!(
                    fp.total_assets,
                    fp.total_liabilities + fp.total_net_assets,
                    "{what}"
                );
                assert_eq!(fp.difference, 0, "{what}");
                assert_eq!(
                    fp.total_net_assets,
                    fp.net_assets.unrestricted + fp.net_assets.restricted,
                    "{what}: net assets by restriction"
                );
                let assets: i64 = fp.assets.iter().map(|r| r.amount).sum();
                let liabilities: i64 = fp.liabilities.iter().map(|r| r.amount).sum();
                assert_eq!(assets, fp.total_assets, "{what}: asset rows");
                assert_eq!(liabilities, fp.total_liabilities, "{what}: liability rows");
            }
        }
    }
}

#[test]
fn activities_change_in_net_assets_ties_to_the_financial_position() {
    for state in [demo(), commands_final()] {
        for basis in BASES {
            for (from, to) in RANGES {
                let act = activities(&state, &date(from), &date(to), basis);
                let before = financial_position(&state, &day_before(&date(from)).unwrap(), basis);
                let after = financial_position(&state, &date(to), basis);
                let what = format!("{from}..{to} {basis:?}");
                assert_eq!(
                    act.change_in_net_assets.total,
                    after.total_net_assets - before.total_net_assets,
                    "{what}: total"
                );
                assert_eq!(
                    act.change_in_net_assets.unrestricted,
                    after.net_assets.unrestricted - before.net_assets.unrestricted,
                    "{what}: unrestricted"
                );
                assert_eq!(
                    act.change_in_net_assets.restricted,
                    after.net_assets.restricted - before.net_assets.restricted,
                    "{what}: restricted"
                );
                let by_fund: i64 = act.by_fund.iter().map(|f| f.change).sum();
                assert_eq!(
                    by_fund, act.change_in_net_assets.total,
                    "{what}: fund schedule"
                );
            }
        }
    }
}

#[test]
fn functional_expenses_total_equals_activities_expenses() {
    let state = demo();
    for basis in BASES {
        for (from, to) in RANGES {
            let fe = functional_expenses(&state, &date(from), &date(to), basis);
            let act = activities(&state, &date(from), &date(to), basis);
            let what = format!("{from}..{to} {basis:?}");
            assert_eq!(fe.totals.total, act.total_expenses.total, "{what}");
            assert_eq!(
                fe.totals.program + fe.totals.management + fe.totals.fundraising,
                fe.totals.total,
                "{what}: by function"
            );
        }
    }
    let ytd = functional_expenses(
        &state,
        &date("2026-01-01"),
        &date("2026-02-04"),
        Basis::Accrual,
    );
    assert!(ytd.totals.total > 0, "the demo books spend in 2026");
}

#[test]
fn cash_flow_ties_to_the_change_in_cash() {
    for state in [demo(), commands_final()] {
        let cash_at = |as_of: &IsoDate| -> i64 {
            financial_position(&state, as_of, Basis::Accrual)
                .assets
                .iter()
                .filter(|r| r.account.is_cash)
                .map(|r| r.amount)
                .sum()
        };
        for (from, to) in RANGES.iter().chain(&[("2025-12-01", "2025-12-31")]) {
            let cf = cash_flow(&state, &date(from), &date(to)).unwrap();
            let what = format!("{from}..{to}");
            assert_eq!(cf.opening + cf.net, cf.closing, "{what}");
            assert_eq!(cf.difference, 0, "{what}");
            assert_eq!(
                cf.operating.net + cf.investing + cf.financing,
                cf.net,
                "{what}: sections"
            );
            assert_eq!(cf.closing, cash_at(&date(to)), "{what}: closing cash");
            assert_eq!(
                cf.opening,
                cash_at(&day_before(&date(from)).unwrap()),
                "{what}: opening cash"
            );
        }
    }
}

#[test]
fn grant_reports_tie_to_grant_spend() {
    let state = demo();
    let year_end = date("2026-12-31");
    let mut spent_total = 0;
    for grant in &state.grants {
        let summary = grant_summary(&state, &grant.id, &year_end).unwrap();
        let what = grant.id.as_str();
        // No entry is dated after 2026-02-04 and the demo books hold no year-end close, so the
        // engine's grant spend (every posted expense line on the grant) is the year's spend.
        assert_eq!(summary.spent, state.grant_spend(&grant.id, None), "{what}");
        assert_eq!(
            summary.remaining,
            summary.award - summary.spent - summary.committed,
            "{what}: remaining"
        );
        spent_total += summary.spent;
    }
    assert!(spent_total > 0, "the demo grants have spend");
    // Every expense line coded to a grant is in exactly one grant summary.
    let coded: i64 = state
        .lines
        .iter()
        .filter(|l| l.grant_id.is_some())
        .filter(|l| {
            state
                .account(&l.account_id)
                .is_some_and(|a| a.account_type == AccountType::Expense)
        })
        .map(|l| l.base_amount)
        .sum();
    assert_eq!(spent_total, coded);
}
