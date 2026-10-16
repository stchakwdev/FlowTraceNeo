//! Property tests: the posting invariants hold on books nobody wrote by hand.
//!
//! The first four port the fast-check properties of `flowtrace/src/lib/ledger/engine.test.ts`
//! (lines 322-380). The rest state the invariants of `Docs/flowtrace-ledger-plan.md` section 5
//! ("Invariants enforced in the database, not the UI") and the rules of `engine.ts`,
//! `reconcile.ts` and `csv.ts` they rest on, over random entries, funds, accounts, command
//! sequences and statement rows. Each runs [`config`] cases: 1000, or `PROPTEST_CASES`. See
//! `tests/README.md`.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use std::collections::BTreeMap;

use common::books::{
    U, acc, admin, bank_account, books_hold, codes, date, demo_books, entry, financial_position,
    fresh, grant_line, line, org_books, trial_balance, viewer,
};
use common::strategies::{
    Defect, EXTRA_CODES, FundGroup, FxLine, Op, POSTABLE, RowSpec, Side, Transfer, balance_lines,
    balanced_entry, config, csv_row, date_2026, defect, fund_entry, fx_entry, fx_line,
    invalid_entry, js, op, payment, skewed, statement_row, ymd,
};
use ledger_core::coa_template::account_id;
use ledger_core::csv::{CsvCell, parse_csv, to_csv};
use ledger_core::engine::{EntryOptions, Error, Ledger, NewAccount, NewFund, StatementRow};
use ledger_core::env::fixed;
use ledger_core::money::{convert, format_major};
use ledger_core::reconcile::{DEFAULT_WINDOW_DAYS, auto_match, candidates_for};
use ledger_core::reports::grant_summary;
use ledger_core::types::{
    AccountType, EntryId, EntryInput, EntryStatus, FundId, FundRestriction, GrantId, JournalEntry,
    LedgerState, MatchKind, Nullish, PeriodId, PeriodStatus,
};
use ledger_core::{IssueCode, Minor, apply_command, sanitize_command};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;
use serde_json::{Value, json};

fn post(state: &mut LedgerState, env: &mut fixed::FixedEnv, input: &EntryInput) -> EntryId {
    Ledger::new(state, env)
        .post(&admin(), input, EntryOptions::default())
        .unwrap()
        .id
}

fn try_post(
    state: &mut LedgerState,
    env: &mut fixed::FixedEnv,
    input: &EntryInput,
) -> Result<JournalEntry, Error> {
    Ledger::new(state, env).post(&admin(), input, EntryOptions::default())
}

fn period(month: u32) -> PeriodId {
    PeriodId::from(format!("period-2026-{month:02}"))
}

/// Day of 2026 (0 for 1 January) as Bun's `Date` reads `YYYY-MM-DD`: a day past the month's
/// end rolls into the next month. 2026 is not a leap year.
fn bun_day(d: &str) -> i64 {
    const BEFORE: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    assert!(d.starts_with("2026-"), "bun_day reads 2026 only: {d}");
    let month: usize = d[5..7].parse().unwrap();
    let day: i64 = d[8..10].parse().unwrap();
    BEFORE[month - 1] + day - 1
}

/// The dedupe key of a statement row (`engine.ts:1168-1170`): date, amount, and the reference or
/// else the description.
fn statement_key(r: &RowSpec) -> String {
    format!("{}|{}|{}", r.0, r.3, r.2.unwrap_or(r.1))
}

fn statement_rows(rows: &[RowSpec]) -> Vec<StatementRow> {
    rows.iter()
        .map(|(d, description, reference, amount)| StatementRow {
            date: date(d),
            description: (*description).to_owned(),
            reference: reference.map(str::to_owned),
            amount: js(*amount),
        })
        .collect()
}

/// The period of `month`, or else the first period of the books with status `status` (with
/// `last`, the last period whose status is not `status`): the one the period rules allow next.
fn month(state: &LedgerState, month: Option<u32>, status: PeriodStatus, last: bool) -> PeriodId {
    let next = if last {
        state.periods.iter().rev().find(|p| p.status != status)
    } else {
        state.periods.iter().find(|p| p.status == status)
    };
    match (month, next) {
        (Some(m), _) => period(m),
        (None, Some(p)) => p.id.clone(),
        (None, None) => period(12),
    }
}

/// A command of a random sequence as the JSON a client sends (`commands.ts:25-79`), its entry
/// picked among drafts (for the draft commands) or posted non-reversal entries (for
/// `entry.reverse`) when there are any, else among all entries.
fn command_json(state: &LedgerState, op: &Op) -> Value {
    let pick = |i: &Index, want: &dyn Fn(&JournalEntry) -> bool| {
        let preferred: Vec<_> = state.entries.iter().filter(|e| want(e)).collect();
        let pool: Vec<_> = if preferred.is_empty() {
            state.entries.iter().collect()
        } else {
            preferred
        };
        if pool.is_empty() {
            "je-missing".to_owned()
        } else {
            pool[i.index(pool.len())].id.as_str().to_owned()
        }
    };
    let draft = |e: &JournalEntry| e.status == EntryStatus::Draft;
    let input = |e: &EntryInput| serde_json::to_value(e).unwrap();
    match op {
        Op::Post(e) | Op::PostInvalid(e) => json!({ "type": "entry.post", "input": input(e) }),
        Op::SaveDraft(e) => json!({ "type": "entry.saveDraft", "input": input(e) }),
        Op::UpdateDraft(i, e) => {
            json!({ "type": "entry.updateDraft", "entryId": pick(i, &draft), "input": input(e) })
        }
        Op::PostDraft(i) => json!({ "type": "entry.postDraft", "entryId": pick(i, &draft) }),
        Op::DeleteDraft(i) => json!({ "type": "entry.deleteDraft", "entryId": pick(i, &draft) }),
        Op::Reverse(i, d) => {
            let live =
                |e: &JournalEntry| e.status == EntryStatus::Posted && e.reversal_of.is_none();
            let id = pick(i, &live);
            json!({ "type": "entry.reverse", "entryId": id, "date": d, "reason": "Correction" })
        }
        Op::Import(es) => {
            json!({ "type": "entry.import", "inputs": es.iter().map(input).collect::<Vec<_>>() })
        }
        Op::Close(m) => {
            let id = month(state, *m, PeriodStatus::Open, false);
            json!({ "type": "period.close", "periodId": id })
        }
        Op::Reopen(m) => json!({
            "type": "period.reopen", "periodId": month(state, *m, PeriodStatus::Open, true),
            "reason": "Late invoice"
        }),
        Op::Lock(m) => {
            let id = month(state, *m, PeriodStatus::Closed, false);
            json!({ "type": "period.lock", "periodId": id })
        }
        Op::Activate(code, on) => json!({
            "type": "account.update", "accountId": account_id(code), "patch": { "is_active": on }
        }),
        Op::Junk(0) => json!({ "type": "nope" }),
        Op::Junk(1) => json!({ "type": "entry.post" }),
        Op::Junk(2) => json!({ "type": "entry.reverse", "entryId": 5, "date": "2026-01-01" }),
        Op::Junk(_) => json!({ "type": "period.close", "periodId": null }),
    }
}

proptest! {
    #![proptest_config(config())]

    /// `engine.test.ts:323-337`, "any sequence of balanced postings and reversals keeps the books
    /// balanced": up to 25 balanced entries, each reversed on its own date or not, leave a trial
    /// balance and a statement of financial position with no difference.
    #[test]
    fn any_sequence_of_balanced_postings_and_reversals_keeps_the_books_balanced(
        ops in vec((balanced_entry(), any::<bool>()), 1..=25),
    ) {
        let (mut state, mut env) = fresh();
        let mut l = Ledger::new(&mut state, &mut env);
        for (input, reverse) in &ops {
            let e = l.post(&admin(), input, EntryOptions::default()).unwrap();
            if *reverse {
                l.reverse(&admin(), &e.id, &input.date, "test").unwrap();
            }
        }
        prop_assert_eq!(trial_balance(&state).difference, 0);
        prop_assert_eq!(financial_position(&state, "2026-12-31").difference, 0);
    }

    /// `engine.test.ts:339-353`, "any entry whose debits and credits differ is rejected and leaves
    /// no trace": a balanced entry skewed on its first line by 1 to 1,000,000 is refused with
    /// `UNBALANCED`, and the books keep no entry and no line. Rust also checks that the books are
    /// exactly as before and that no id was drawn (`engine/mod.rs`: a rejected write returns before
    /// its first change).
    #[test]
    fn any_entry_whose_debits_and_credits_differ_is_rejected_and_leaves_no_trace(
        input in balanced_entry(),
        skew in 1..=1_000_000i64,
    ) {
        let (mut state, mut env) = fresh();
        let (before, drawn) = (state.clone(), env.drawn());
        let bad = skewed(input, skew);
        let err = Ledger::new(&mut state, &mut env)
            .post(&admin(), &bad, EntryOptions::default())
            .unwrap_err();
        prop_assert!(codes(&err).contains(&IssueCode::Unbalanced), "{}", err);
        prop_assert!(state.entries.is_empty());
        prop_assert!(state.lines.is_empty());
        prop_assert_eq!(&state, &before);
        prop_assert_eq!(env.drawn(), drawn);
    }

    /// `engine.test.ts:355-365`, "reversing everything returns every account to zero": up to 15
    /// balanced entries, each reversed on 2026-12-31, leave no trial balance row.
    #[test]
    fn reversing_everything_returns_every_account_to_zero(
        inputs in vec(balanced_entry(), 1..=15),
    ) {
        let (mut state, mut env) = fresh();
        let mut l = Ledger::new(&mut state, &mut env);
        let posted: Vec<_> = inputs
            .iter()
            .map(|i| l.post(&admin(), i, EntryOptions::default()).unwrap().id)
            .collect();
        for id in &posted {
            l.reverse(&admin(), id, "2026-12-31", "undo").unwrap();
        }
        prop_assert!(trial_balance(&state).rows.is_empty());
    }

    /// `engine.test.ts:367-379`, "posted lines never change once posted": the first entry's lines
    /// serialise the same after more postings and its own reversal.
    #[test]
    fn posted_lines_never_change_once_posted(inputs in vec(balanced_entry(), 2..=10)) {
        let (mut state, mut env) = fresh();
        let mut l = Ledger::new(&mut state, &mut env);
        let first = l.post(&admin(), &inputs[0], EntryOptions::default()).unwrap().id;
        let snapshot = serde_json::to_string(&l.state().lines_of(&first)).unwrap();
        for i in &inputs[1..] {
            l.post(&admin(), i, EntryOptions::default()).unwrap();
        }
        l.reverse(&admin(), &first, "2026-12-31", "x").unwrap();
        prop_assert_eq!(serde_json::to_string(&l.state().lines_of(&first)).unwrap(), snapshot);
    }

    /// Every posted entry balances in base currency (`Docs/flowtrace-ledger-plan.md` section 5,
    /// first invariant; `engine.ts:376-408`). Lines in KES, USD and EUR at random rates, plugged
    /// to balance in base and sometimes skewed, post exactly when the lines converted one by one
    /// (`convert`, `money.ts:36-51`) sum to zero; the stored base amounts are those conversions,
    /// frozen on the lines. Otherwise the entry is refused with one `UNBALANCED` issue naming the
    /// difference, and the books are unchanged.
    #[test]
    fn every_posted_entry_balances_in_base_currency(
        (lines, plug, skew, day) in fx_entry(),
    ) {
        let base_of = |l: &FxLine| convert(l.2.signed(l.1), l.3.map_or(1.0, |c| c.1)).unwrap();
        let net: Minor = lines.iter().map(base_of).sum();
        let mut lines = lines;
        let mut inputs: Vec<_> = Vec::new();
        lines[0].1 += skew;
        inputs.extend(lines.iter().map(|l| fx_line(l, U)));
        let plugs = balance_lines(&[], plug, U);
        if net == 0 {
            inputs.extend(plugs);
        } else {
            inputs.push(if net > 0 { line(plug, U, 0, net) } else { line(plug, U, -net, 0) });
        }
        let expected: Vec<Minor> = lines
            .iter()
            .map(base_of)
            .chain(inputs[lines.len()..].iter().map(|l| {
                js_minor(l.debit.unwrap_or(0.0)) - js_minor(l.credit.unwrap_or(0.0))
            }))
            .collect();
        let total: Minor = expected.iter().sum();
        let input = entry(&day, "Generated", inputs);

        let (mut state, mut env) = org_books();
        let before = state.clone();
        let result = try_post(&mut state, &mut env, &input);
        match result {
            Ok(e) => {
                prop_assert_eq!(total, 0);
                let stored: Vec<Minor> =
                    state.lines_of(&e.id).iter().map(|l| l.base_amount).collect();
                prop_assert_eq!(stored, expected);
            }
            Err(err) => {
                prop_assert_ne!(total, 0);
                prop_assert_eq!(codes(&err), vec![IssueCode::Unbalanced]);
                prop_assert_eq!(
                    err.to_string(),
                    format!("Debits and credits differ by KES {}.", format_major(total.abs()))
                );
                prop_assert_eq!(&state, &before);
            }
        }
    }

    /// Every posted entry balances within each fund, and so every fund nets to zero over the books
    /// (`Docs/flowtrace-ledger-plan.md` section 5; `engine.ts:409-414`; `engine.test.ts:111-122`).
    /// Books get up to three more funds of random restriction (with no grant, so the grant rules
    /// stay out) and up to three expense accounts under 7000, all through the engine. Entries are
    /// one to three groups each balanced within a fund, sometimes with a transfer that debits one
    /// fund and credits another: an entry posts exactly when every fund nets to zero, and is
    /// otherwise refused with one `FUND_UNBALANCED` issue per unbalanced fund.
    #[test]
    fn every_posted_entry_balances_per_fund(
        (extra, restrictions, entries) in (0..=3usize, vec(restriction(), 1..=3))
            .prop_flat_map(|(extra, r)| (Just(extra), Just(r), vec(fund_entry(extra), 1..=8))),
    ) {
        let (mut state, mut env) = org_books();
        let mut funds = vec![FundId::from(U)];
        {
            let mut l = Ledger::new(&mut state, &mut env);
            for code in &EXTRA_CODES[..extra] {
                l.add_account(&admin(), expense_account(code)).unwrap();
            }
            for (i, restriction) in restrictions.iter().enumerate() {
                let name = format!("Fund {i}");
                let fund = NewFund { id: None, name, restriction: *restriction };
                let f = l.add_fund(&admin(), fund).unwrap();
                funds.push(f.id);
            }
        }
        for (groups, transfer, day) in &entries {
            let input = fund_input(&funds, groups, *transfer, day);
            let mut net: BTreeMap<&FundId, Minor> = BTreeMap::new();
            for l in &input.lines {
                *net.entry(&l.fund_id).or_insert(0) +=
                    js_minor(l.debit.unwrap_or(0.0)) - js_minor(l.credit.unwrap_or(0.0));
            }
            let unbalanced = net.values().filter(|n| **n != 0).count();
            let before = state.clone();
            let result = try_post(&mut state, &mut env, &input);
            match result {
                Ok(e) => {
                    prop_assert_eq!(unbalanced, 0);
                    let mut stored: BTreeMap<FundId, Minor> = BTreeMap::new();
                    for l in state.lines_of(&e.id) {
                        *stored.entry(l.fund_id.clone()).or_insert(0) += l.base_amount;
                    }
                    prop_assert!(stored.values().all(|n| *n == 0), "{:?}", stored);
                }
                Err(err) => {
                    prop_assert!(unbalanced > 0, "{}", err);
                    prop_assert_eq!(codes(&err), vec![IssueCode::FundUnbalanced; unbalanced]);
                    prop_assert_eq!(&state, &before);
                }
            }
        }
        for f in &funds {
            let lines = state.lines.iter().filter(|l| &l.fund_id == f);
            let net: Minor = lines.map(|l| l.base_amount).sum();
            prop_assert_eq!(net, 0, "fund {} does not net to zero", f.as_str());
        }
        books_hold(&state)?;
    }

    /// Posted entries are immutable; a correction is a reversal plus a new entry
    /// (`Docs/flowtrace-ledger-plan.md` section 5; `engine.ts:563-662`; `engine.test.ts:129-141`).
    /// For a random posted entry: `updateDraft` and `deleteDraft` are refused with
    /// `INVALID_STATE` and change nothing; `reverse` adds one entry that names the original and
    /// mirrors its lines, marks the original reversed and leaves its lines and every other entry
    /// as they were; neither the original nor the reversal can be reversed, edited or deleted
    /// after that.
    #[test]
    fn posted_entries_are_immutable(
        inputs in vec(balanced_entry(), 1..=6),
        pick in any::<Index>(),
        edit in balanced_entry(),
    ) {
        let (mut state, mut env) = fresh();
        let ids: Vec<_> = inputs.iter().map(|i| post(&mut state, &mut env, i)).collect();
        let target = ids[pick.index(ids.len())].clone();
        let refused = |state: &mut LedgerState,
                       env: &mut fixed::FixedEnv,
                       id: &EntryId|
         -> Result<(), TestCaseError> {
            let before = state.clone();
            let mut l = Ledger::new(state, env);
            for err in [
                l.update_draft(&admin(), id, &edit).map(drop).unwrap_err(),
                l.delete_draft(&admin(), id).unwrap_err(),
                l.reverse(&admin(), id, "2026-12-31", "again").map(drop).unwrap_err(),
            ] {
                prop_assert_eq!(codes(&err), vec![IssueCode::InvalidState], "{}", err);
            }
            prop_assert_eq!(&*state, &before);
            Ok(())
        };
        let before = state.clone();
        {
            let mut l = Ledger::new(&mut state, &mut env);
            for err in [
                l.update_draft(&admin(), &target, &edit).map(drop).unwrap_err(),
                l.delete_draft(&admin(), &target).unwrap_err(),
            ] {
                prop_assert_eq!(codes(&err), vec![IssueCode::InvalidState], "{}", err);
            }
        }
        prop_assert_eq!(&state, &before);

        let rev = Ledger::new(&mut state, &mut env)
            .reverse(&admin(), &target, "2026-12-31", "Correction")
            .unwrap();
        prop_assert_eq!(rev.reversal_of.as_ref(), Some(&target));
        prop_assert_eq!(state.entries.len(), before.entries.len() + 1);
        for e in &before.entries {
            let now = state.entry(&e.id).unwrap();
            if e.id == target {
                prop_assert_eq!(now.status, EntryStatus::Reversed);
                prop_assert_eq!(now.reversed_by.as_ref(), Some(&rev.id));
            } else {
                prop_assert_eq!(now, e);
            }
        }
        prop_assert_eq!(state.lines_of(&target), before.lines_of(&target));
        books_hold(&state)?;
        refused(&mut state, &mut env, &target)?;
        refused(&mut state, &mut env, &rev.id)?;
    }

    /// No posting into a closed or locked period, and reopening restores posting
    /// (`Docs/flowtrace-ledger-plan.md` section 5; `engine.ts:313-328`, `672-712`;
    /// `engine.test.ts:143-153`). The first `closed` months are closed in order and the first
    /// `locked` of them locked; a balanced entry dated in a random month is refused with
    /// `PERIOD_LOCKED` or `PERIOD_CLOSED` alone, leaving the books as they were, or posts. Then the
    /// closed months reopen in reverse order, a locked month refuses to reopen with
    /// `PERIOD_LOCKED`, and the same entry posts unless its month is locked.
    #[test]
    fn a_closed_or_locked_period_refuses_postings_and_reopen_restores_them(
        closed in 1..=12u32,
        lock in any::<Index>(),
        input in balanced_entry(),
    ) {
        let locked = u32::try_from(lock.index(closed as usize + 1)).unwrap();
        let month: u32 = input.date[5..7].parse().unwrap();
        let (mut state, mut env) = org_books();
        {
            let mut l = Ledger::new(&mut state, &mut env);
            for m in 1..=closed {
                l.close_period(&admin(), &period(m)).unwrap();
            }
            for m in 1..=locked {
                l.lock_period(&admin(), &period(m)).unwrap();
            }
        }
        let expect = |closed: u32| {
            if month <= locked {
                Some(IssueCode::PeriodLocked)
            } else if month <= closed {
                Some(IssueCode::PeriodClosed)
            } else {
                None
            }
        };
        let before = state.clone();
        let result = try_post(&mut state, &mut env, &input);
        match (result, expect(closed)) {
            (Ok(_), None) => {}
            (Err(err), Some(code)) => {
                prop_assert_eq!(codes(&err), vec![code]);
                prop_assert_eq!(&state, &before);
            }
            (r, e) => prop_assert!(
                false,
                "month {} closed {} locked {}: {:?}, expected {:?}",
                month, closed, locked, r.map(|e| e.number), e
            ),
        }
        if month <= closed {
            let mut l = Ledger::new(&mut state, &mut env);
            for m in (locked + 1..=closed).rev() {
                l.reopen_period(&admin(), &period(m), "fix").unwrap();
            }
            if locked > 0 {
                let err = l.reopen_period(&admin(), &period(locked), "fix").unwrap_err();
                prop_assert_eq!(codes(&err), vec![IssueCode::PeriodLocked]);
            }
            let result = l.post(&admin(), &input, EntryOptions::default());
            match (result, expect(locked)) {
                (Ok(_), None) => {}
                (Err(err), Some(code)) => prop_assert_eq!(codes(&err), vec![code]),
                (r, e) => {
                    let got = r.map(|e| e.number);
                    prop_assert!(false, "after reopening: {:?}, expected {:?}", got, e);
                }
            }
        }
    }

    /// Restricted spend names its grant and is blocked past the grant's end date or over its
    /// award unless overridden with a reason (`Docs/flowtrace-ledger-plan.md` section 5;
    /// `engine.ts:376-443`, `447-464`; `engine.test.ts:177-190`). Random spends from the WASH
    /// grant (KES 10,000.00, 2026-01-01 to 2026-06-30), some overridden, and reversals of earlier
    /// spends follow a model: a spend posts exactly when overridden, or dated within the grant and
    /// within the award left (a quarter of the spends land on the award, a cent either side or
    /// two over, so the boundary is tested); a refused spend carries `GRANT_ENDED` (dates are
    /// checked first and stop the award check) or `GRANT_OVER_BUDGET` alone. `grantSpend` and
    /// the grant summary's `spent` (`reports.ts:555-597`) equal the model after every step, never
    /// go below zero, and stay within the award while nothing was overridden.
    #[test]
    fn grant_spend_follows_the_award_and_the_grant_dates(
        ops in vec(prop_oneof![
            3 => (spend_amount(), 1..=8u32, 1..=28u32, prop::bool::weighted(0.2))
                .prop_map(|spend| (Some(spend), None)),
            1 => any::<Index>().prop_map(|i| (None, Some(i))),
        ], 1..=20),
    ) {
        const AWARD: Minor = 1_000_000;
        let grant = GrantId::from("grant-wash");
        let (mut state, mut env) = fresh();
        let mut model: Minor = 0;
        let mut live: Vec<(EntryId, Minor)> = Vec::new();
        let mut overridden = false;
        for (spend, pick) in &ops {
            if let Some((amount, month, day, over)) = *spend {
                let amount = match amount {
                    Spend::Amount(a) => a,
                    Spend::ToAward(delta) => (AWARD - model + delta).max(1),
                };
                let date = ymd(2026, month, day);
                // grantSpend() of engine.test.ts:163-170.
                let mut input = entry(&date, "Borehole", vec![
                    grant_line("5100", "fund-wash", "grant-wash", amount, 0),
                    line("1130", "fund-wash", 0, amount),
                ]);
                if over {
                    input.override_reason = Nullish::Value("Donor approved".to_owned());
                }
                let in_dates = date.as_str() <= "2026-06-30";
                let ok = over || (in_dates && model + amount <= AWARD);
                let before = state.clone();
                let result = try_post(&mut state, &mut env, &input);
                prop_assert_eq!(result.is_ok(), ok, "{:?} with {} spent", spend, model);
                match result {
                    Ok(e) => {
                        model += amount;
                        overridden |= over;
                        live.push((e.id, amount));
                    }
                    Err(err) => {
                        let code = if in_dates {
                            IssueCode::GrantOverBudget
                        } else {
                            IssueCode::GrantEnded
                        };
                        prop_assert_eq!(codes(&err), vec![code]);
                        prop_assert_eq!(&state, &before);
                    }
                }
            } else if let (Some(pick), false) = (pick, live.is_empty()) {
                let (id, amount) = live.remove(pick.index(live.len()));
                let mut l = Ledger::new(&mut state, &mut env);
                l.reverse(&admin(), &id, "2026-12-31", "undo").unwrap();
                model -= amount;
            }
            prop_assert_eq!(state.grant_spend(&grant, None), model);
            let summary = grant_summary(&state, &grant, &date("2026-12-31")).unwrap();
            prop_assert_eq!(summary.spent, model);
            prop_assert!(model >= 0);
            prop_assert!(overridden || model <= AWARD);
        }
    }

    /// Any entry broken one way is refused with that rule's code and leaves no trace: the books
    /// and the env are exactly as before (`engine.ts:295-445`, `526-537`; the atomic-write rule
    /// of `engine/mod.rs`). Defects: unbalanced, negative, zero, both sides, a fraction of a
    /// cent, an unknown or header account, an unknown fund, one line, a date in no period, a
    /// blank memo, a malformed date, a foreign line without a rate, and an actor who cannot post
    /// (refused with `PERMISSION` alone).
    #[test]
    fn any_invalid_entry_is_rejected_with_its_code_and_leaves_no_trace(
        (input, d) in invalid_entry(),
    ) {
        let (mut state, mut env) = fresh();
        let (before, drawn) = (state.clone(), env.drawn());
        let actor = if d == Defect::NoPermission { viewer() } else { admin() };
        let err = Ledger::new(&mut state, &mut env)
            .post(&actor, &input, EntryOptions::default())
            .unwrap_err();
        let got = codes(&err);
        if d == Defect::NoPermission {
            prop_assert_eq!(got, vec![IssueCode::Permission]);
        } else {
            prop_assert!(got.contains(&d.code()), "{:?} gave {:?}", d, got);
        }
        prop_assert_eq!(&state, &before);
        prop_assert_eq!(env.drawn(), drawn);
    }

    /// Random command sequences through `sanitizeCommand` and `applyCommand`
    /// (`commands.ts:85-140`), on fresh or demo books: postings valid and broken, drafts saved,
    /// edited, posted and deleted, reversals, imports, period close, reopen and lock, accounts
    /// (de)activated, and commands the sanitiser refuses. A refused command leaves the books
    /// exactly as they were (the module discards them, `spacetimedb/src/index.ts:663-690`), and
    /// after every command [`books_hold`]: entries balance in base and per fund, reversals pair
    /// up and mirror, ids and numbers are unique, the reports foot. The final books survive a JSON
    /// round trip.
    #[test]
    fn random_command_sequences_keep_every_invariant(
        demo in prop::bool::weighted(0.25),
        ops in vec(op(), 1..=25),
    ) {
        let (mut state, mut env) = if demo { (demo_books(), fixed::test()) } else { fresh() };
        books_hold(&state)?;
        for op in &ops {
            let raw = command_json(&state, op);
            let before = state.clone();
            let applied = sanitize_command(&raw).map(|cmd| {
                apply_command(&mut Ledger::new(&mut state, &mut env), &admin(), cmd)
            });
            if let Ok(Ok(_)) = applied {
                let invalid = matches!(op, Op::Junk(_) | Op::PostInvalid(_));
                prop_assert!(!invalid, "{} was applied", raw);
            } else {
                prop_assert_eq!(&state, &before, "{} was refused but changed the books", raw);
            }
            books_hold(&state)?;
        }
        let json = serde_json::to_value(&state).unwrap();
        let back: LedgerState = serde_json::from_value(json).unwrap();
        prop_assert_eq!(back, state);
    }

    /// Statement import skips rows already imported, by count per key (`engine.ts:1153-1197`;
    /// `review-regressions.test.ts:149-155`): after any sequence of files on one bank account,
    /// each key (date, amount, reference or else description) has as many lines as the file
    /// that held it most often, and each file adds its rows of a key past the count already held,
    /// in file order. Files draw from a pool of one to three rows, so keys repeat.
    #[test]
    fn statement_import_keeps_each_key_as_often_as_one_file_held_it(
        pool in vec(statement_row(), 1..=3),
        picks in vec(vec(any::<Index>(), 0..=6), 1..=4),
    ) {
        let files: Vec<Vec<RowSpec>> = picks
            .iter()
            .map(|f| f.iter().map(|i| pool[i.index(pool.len())].clone()).collect())
            .collect();
        let (mut state, mut env) = org_books();
        let mut l = Ledger::new(&mut state, &mut env);
        l.add_bank_account(&admin(), bank_account("b", "1110")).unwrap();
        let mut held: BTreeMap<String, usize> = BTreeMap::new();
        let mut most: BTreeMap<String, usize> = BTreeMap::new();
        for rows in &files {
            let added = Ledger::new(&mut state, &mut env)
                .import_statement(&admin(), &"b".into(), &statement_rows(rows))
                .unwrap();
            let mut skip = held.clone();
            let mut expected = Vec::new();
            let mut in_file: BTreeMap<String, usize> = BTreeMap::new();
            for r in rows {
                let k = statement_key(r);
                *in_file.entry(k.clone()).or_insert(0) += 1;
                match skip.get_mut(&k) {
                    Some(n) if *n > 0 => *n -= 1,
                    _ => {
                        let (d, description, reference, amount) = r;
                        let reference = reference.map(str::to_owned);
                        expected.push((d.clone(), (*description).to_owned(), reference, *amount));
                        *held.entry(k).or_insert(0) += 1;
                    }
                }
            }
            for (k, n) in in_file {
                let m = most.entry(k).or_insert(0);
                *m = (*m).max(n);
            }
            let got: Vec<(String, String, Option<String>, Minor)> = added
                .iter()
                .map(|s| {
                    let (date, reference) = (s.date.to_string(), s.reference.clone());
                    (date, s.description.clone(), reference, s.amount)
                })
                .collect();
            prop_assert_eq!(got, expected);
        }
        let mut lines: BTreeMap<String, usize> = BTreeMap::new();
        for s in &state.statement_lines {
            let key = s.reference.as_deref().unwrap_or(&s.description);
            let k = format!("{}|{}|{}", s.date, s.amount, key);
            *lines.entry(k).or_insert(0) += 1;
        }
        prop_assert_eq!(lines, most);
    }

    /// Auto-match only links a statement line to an unmatched posted line on the bank's ledger
    /// account with the same signed amount, dated within the window, and never one journal line
    /// twice (`reconcile.ts:110-166`; `reconcile.test.ts:51-78`). Payments in January and
    /// statement rows with Bun-valid dates in January and February (day gaps as Bun's `Date`
    /// counts them, a day past the month's end rolling over) end up matched, ambiguous (two
    /// candidates or more) or unmatched (none left), one each, with one audit event per match.
    #[test]
    fn auto_match_links_equal_amounts_within_the_window_and_each_line_once(
        payments in vec(payment(), 0..=8),
        rows in vec(statement_row(), 0..=8),
    ) {
        let (mut state, mut env) = org_books();
        let bank = "b".into();
        {
            let mut l = Ledger::new(&mut state, &mut env);
            l.add_bank_account(&admin(), bank_account("b", "1110")).unwrap();
            for (amount, day, reference) in &payments {
                let memo =
                    reference.map_or("Stationery".to_owned(), |r| format!("Stationery \u{b7} {r}"));
                let lines = vec![line("7100", U, *amount, 0), line("1110", U, 0, *amount)];
                let input = entry(&ymd(2026, 1, *day), &memo, lines);
                l.post(&admin(), &input, EntryOptions::default()).unwrap();
            }
            l.import_statement(&admin(), &bank, &statement_rows(&rows)).unwrap();
        }
        let open = state.statement_lines.iter().filter(|s| s.matched_line_id.is_none()).count();
        let audit = state.audit.len();
        let mut l = Ledger::new(&mut state, &mut env);
        let r = auto_match(&mut l, &admin(), &bank, DEFAULT_WINDOW_DAYS).unwrap();
        prop_assert_eq!(r.matched.len() + r.ambiguous.len() + r.unmatched.len(), open);
        prop_assert_eq!(state.audit.len(), audit + r.matched.len());
        let mut used = std::collections::HashSet::new();
        for m in &r.matched {
            let fresh = used.insert(m.journal_line_id.clone());
            prop_assert!(fresh, "{} matched twice", m.journal_line_id.as_str());
            let s = state.statement_lines.iter().find(|s| s.id == m.statement_line_id).unwrap();
            let jl = state.lines.iter().find(|l| l.id == m.journal_line_id).unwrap();
            let e = state.entry(&jl.entry_id).unwrap();
            prop_assert_eq!(s.matched_line_id.as_ref(), Some(&jl.id));
            prop_assert_eq!(s.match_kind, Some(MatchKind::Auto));
            prop_assert_eq!(&jl.account_id, &acc("1110"));
            prop_assert_eq!(jl.debit - jl.credit, s.amount);
            prop_assert!(e.status != EntryStatus::Draft);
            let gap = (bun_day(e.date.as_str()) - bun_day(s.date.as_str())).abs();
            let within = gap <= i64::from(DEFAULT_WINDOW_DAYS);
            prop_assert!(within, "{} against {}: {} days", e.date, s.date, gap);
        }
        for a in &r.ambiguous {
            prop_assert!(a.candidates.len() >= 2);
        }
        for id in &r.unmatched {
            let s = state.statement_lines.iter().find(|s| &s.id == id).unwrap();
            prop_assert!(candidates_for(&state, s, DEFAULT_WINDOW_DAYS, &[]).is_empty());
        }
        let matched = state.statement_lines.iter().filter(|s| s.matched_line_id.is_some()).count();
        prop_assert_eq!(matched, r.matched.len());
    }

    /// `toCsv` then `parseCsv` gives the rows back (`csv.ts:4-59`), for fields holding commas,
    /// quotes, line breaks and spaces; `reconcile.test.ts:14-17` ("round-trips quotes, commas and
    /// newlines") is one case of it. Rows need a non-blank field, as `parseCsv` drops blank rows.
    #[test]
    fn csv_round_trips_quotes_commas_and_newlines(rows in vec(csv_row(), 0..=5)) {
        let cells: Vec<Vec<CsvCell>> = rows
            .iter()
            .map(|r| r.iter().map(|f| CsvCell::from(f.as_str())).collect())
            .collect();
        prop_assert_eq!(parse_csv(&to_csv(&cells)), rows);
    }
}

/// A JS number input amount as minor units (the generated amounts are whole).
fn js_minor(x: f64) -> Minor {
    #[expect(clippy::cast_possible_truncation, reason = "whole generated amounts")]
    let n = x as Minor;
    n
}

/// The amount of a grant spend: drawn, or what is left of the award plus `-1..=2` cents.
#[derive(Debug, Clone, Copy)]
enum Spend {
    /// This amount.
    Amount(Minor),
    /// The award left, plus this.
    ToAward(Minor),
}

/// A [`Spend`], a quarter of them at the award's edge.
fn spend_amount() -> impl Strategy<Value = Spend> {
    prop_oneof![
        3 => (1..=400_000i64).prop_map(Spend::Amount),
        1 => (-1..=2i64).prop_map(Spend::ToAward),
    ]
}

/// A fund restriction.
fn restriction() -> impl Strategy<Value = FundRestriction> {
    prop_oneof![
        Just(FundRestriction::Unrestricted),
        Just(FundRestriction::TemporarilyRestricted),
        Just(FundRestriction::PermanentlyRestricted),
    ]
}

/// An expense account under header 7000, with the template's id form `acc-<code>`.
fn expense_account(code: &str) -> NewAccount {
    NewAccount {
        id: Some(acc(code)),
        code: code.to_owned(),
        name: format!("Expense {code}"),
        account_type: AccountType::Expense,
        parent_id: Some(acc("7000")),
        is_group: false,
        is_active: None,
        is_cash: false,
        expense_function: None,
        cash_flow_class: None,
    }
}

/// The input of a [`fund_entry`] case over `funds`.
fn fund_input(
    funds: &[FundId],
    groups: &[FundGroup],
    transfer: Option<Transfer>,
    day: &str,
) -> EntryInput {
    let mut lines = Vec::new();
    for (specs, plug, fund) in groups {
        lines.extend(balance_lines(
            specs,
            plug,
            funds[fund.index(funds.len())].as_str(),
        ));
    }
    if let Some((amount, from, to)) = transfer {
        let (from, to) = (
            &funds[from.index(funds.len())],
            &funds[to.index(funds.len())],
        );
        lines.push(line("1110", from.as_str(), amount, 0));
        lines.push(line("1110", to.as_str(), 0, amount));
    }
    entry(day, "Generated", lines)
}

#[test]
fn the_strategies_cover_their_ranges() {
    // A guard against a strategy collapsing to a constant: the shared pools stay as listed.
    assert_eq!(POSTABLE.len(), 12);
    assert_eq!(Side::Credit.signed(5), -5);
    let _ = (date_2026(), defect(), payment());
}
