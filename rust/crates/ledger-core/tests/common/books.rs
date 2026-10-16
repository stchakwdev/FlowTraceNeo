//! Books, actors and report sums shared by the property and regression tests.
//!
//! The books are built through the engine, as the TS tests build them: `fresh()` of
//! `engine.test.ts:24-40` and `books()` of `review-regressions.test.ts:22-29`. Report reads go
//! through `ledger_core::reports`, as the TS tests call `reports.ts`.

use std::collections::{BTreeMap, HashSet};
use std::sync::OnceLock;

use ledger_core::coa_template::{UNRESTRICTED_FUND_ID, account_id};
use ledger_core::demo::build_demo_ledger;
use ledger_core::engine::{Error, Ledger, NewBankAccount, NewFund, NewGrant, NewVendor};
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::reports::{self, Basis, FinancialPosition, TrialBalance};
use ledger_core::setup::create_org_ledger;
use ledger_core::types::{
    AccountId, Actor, ActorId, BankAccountKind, EntryInput, EntryStatus, FundId, FundRestriction,
    GrantId, IsoDate, JournalEntry, JournalLine, LedgerState, LineInput, Nullish, SourceType,
};
use ledger_core::{IssueCode, Minor};
use proptest::prelude::*;

/// The unrestricted fund (`U` in the TS tests).
pub const U: &str = UNRESTRICTED_FUND_ID;

fn actor(id: &str, name: &str, can_post: bool, can_approve: bool, can_close: bool) -> Actor {
    Actor {
        id: ActorId::from(id),
        name: name.to_owned(),
        can_post,
        can_approve,
        can_close,
    }
}

/// `admin` (`engine.test.ts:12`): every permission.
pub fn admin() -> Actor {
    actor("u1", "Admin", true, true, true)
}

/// `clerk` (`engine.test.ts:13`): posts, cannot approve or close.
pub fn clerk() -> Actor {
    actor("u2", "Clerk", true, false, false)
}

/// `viewer` (`engine.test.ts:14`): no permission.
pub fn viewer() -> Actor {
    actor("u3", "Viewer", false, false, false)
}

/// `A(code)`: the template account id `acc-<code>` (`coa-template.ts` `accountId`).
pub fn acc(code: &str) -> AccountId {
    account_id(code)
}

/// A line of `amount` minor units on one side, in base currency.
pub fn line(account: &str, fund: &str, debit: Minor, credit: Minor) -> LineInput {
    let side = |x: Minor| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a JS number input, as TS holds it"
        )]
        (x != 0).then_some(x as f64)
    };
    LineInput {
        account_id: acc(account),
        debit: side(debit),
        credit: side(credit),
        currency: None,
        fx_rate: None,
        fund_id: FundId::from(fund),
        grant_id: Nullish::Undefined,
        dimension_ids: None,
        flow_node_id: Nullish::Undefined,
        memo: Nullish::Undefined,
    }
}

/// `line` charged to a grant.
pub fn grant_line(
    account: &str,
    fund: &str,
    grant: &str,
    debit: Minor,
    credit: Minor,
) -> LineInput {
    LineInput {
        grant_id: Nullish::Value(GrantId::from(grant)),
        ..line(account, fund, debit, credit)
    }
}

/// An entry input with no source and no override.
pub fn entry(date: &str, memo: &str, lines: Vec<LineInput>) -> EntryInput {
    EntryInput {
        date: date.to_owned(),
        memo: memo.to_owned(),
        lines,
        source_type: None,
        source_id: Nullish::Undefined,
        override_reason: Nullish::Undefined,
    }
}

/// `simple(amount, date)` (`engine.test.ts:42-49`): office supplies paid from the bank account.
pub fn simple(amount: Minor, date: &str) -> EntryInput {
    entry(
        date,
        "Office supplies",
        vec![line("7100", U, amount, 0), line("1110", U, 0, amount)],
    )
}

/// `createOrgLedger('org', 'KES', 2026, testEnv())` (`setup.ts:6-25`, `engine.test.ts:16-19`).
pub fn org_books() -> (LedgerState, FixedEnv) {
    let mut env = fixed::test();
    let state = create_org_ledger("org", "KES", 2026, &mut env).unwrap();
    (state, env)
}

fn add_restricted_grant(l: &mut Ledger<'_>, fund: &str, grant: &str, award: f64, end_date: &str) {
    l.add_fund(
        &admin(),
        NewFund {
            id: Some(FundId::from(fund)),
            name: if fund == "fund-wash" {
                "WASH"
            } else {
                "Restricted"
            }
            .to_owned(),
            restriction: FundRestriction::TemporarilyRestricted,
        },
    )
    .unwrap();
    l.add_grant(
        &admin(),
        NewGrant {
            id: Some(GrantId::from(grant)),
            name: if grant == "grant-wash" {
                "Water grant"
            } else {
                "Grant"
            }
            .to_owned(),
            donor_node_id: None,
            donor_name: if grant == "grant-wash" {
                "Water Trust"
            } else {
                "D"
            }
            .to_owned(),
            fund_id: FundId::from(fund),
            award,
            currency: "KES".to_owned(),
            start_date: IsoDate::parse("2026-01-01").unwrap(),
            end_date: IsoDate::parse(end_date).unwrap(),
            terms: String::new(),
        },
    )
    .unwrap();
}

/// `fresh()` (`engine.test.ts:24-40`): org books with the restricted fund `fund-wash` and its
/// grant `grant-wash`, KES 10,000.00 from 2026-01-01 to 2026-06-30.
pub fn fresh() -> (LedgerState, FixedEnv) {
    let (mut state, mut env) = org_books();
    let mut l = Ledger::new(&mut state, &mut env);
    add_restricted_grant(&mut l, "fund-wash", "grant-wash", 1_000_000.0, "2026-06-30");
    (state, env)
}

/// `books()` (`review-regressions.test.ts:22-29`): org books with the restricted fund `f-r`, its
/// grant `g` of KES 100,000.00 for 2026, the vendor `v` and the bank account `b` on 1110.
pub fn regression_books() -> (LedgerState, FixedEnv) {
    let (mut state, mut env) = org_books();
    let mut l = Ledger::new(&mut state, &mut env);
    add_restricted_grant(&mut l, "f-r", "g", 10_000_000.0, "2026-12-31");
    l.add_vendor(
        &admin(),
        NewVendor {
            id: Some("v".into()),
            name: "Vendor".to_owned(),
            node_id: None,
            payment_details: None,
        },
    )
    .unwrap();
    l.add_bank_account(&admin(), bank_account("b", "1110"))
        .unwrap();
    (state, env)
}

/// The bank account of `review-regressions.test.ts:27` and `reconcile.test.ts:53`.
pub fn bank_account(id: &str, code: &str) -> NewBankAccount {
    NewBankAccount {
        id: Some(id.into()),
        name: "Main".to_owned(),
        kind: BankAccountKind::Bank,
        provider: "X".to_owned(),
        account_id: acc(code),
        currency: "KES".to_owned(),
        number_masked: "1".to_owned(),
    }
}

/// A date the tests write as a literal.
pub fn date(s: &str) -> IsoDate {
    IsoDate::parse(s).unwrap()
}

/// `trialBalance(state)` (`reports.ts:162-179`) on the accrual basis.
pub fn trial_balance(state: &LedgerState) -> TrialBalance {
    reports::trial_balance(state, None, Basis::Accrual)
}

/// The trial balance row of the account with `code` as (debit, credit), if its balance is not
/// zero (`trialBalance(l.state).rows.find((r) => r.account.code === code)`).
pub fn tb_row(state: &LedgerState, code: &str) -> Option<(Minor, Minor)> {
    trial_balance(state)
        .rows
        .into_iter()
        .find(|r| r.account.code == code)
        .map(|r| (r.debit, r.credit))
}

/// `financialPosition(state, asOf)` (`reports.ts:238-297`) on the accrual basis.
pub fn financial_position(state: &LedgerState, as_of: &str) -> FinancialPosition {
    reports::financial_position(state, &date(as_of), Basis::Accrual)
}

/// `activities(state, from, to, basis).totalExpenses.total` (`reports.ts:299-367`).
pub fn total_expenses(state: &LedgerState, from: &str, to: &str, basis: Basis) -> Minor {
    reports::activities(state, &date(from), &date(to), basis)
        .total_expenses
        .total
}

/// The demo books (`demo::build_demo_ledger`, equal to `demo-books.json#/state`), built once.
pub fn demo_books() -> LedgerState {
    static BOOKS: OnceLock<LedgerState> = OnceLock::new();
    BOOKS.get_or_init(|| build_demo_ledger().unwrap()).clone()
}

/// The issue codes of a rejection, in order (`codes` of `engine.test.ts:51-59`). Panics on a
/// `RangeError`, which no ledger rule raises.
pub fn codes(err: &Error) -> Vec<IssueCode> {
    match err {
        Error::Ledger(e) => e.issues().iter().map(|i| i.code).collect(),
        Error::Money(e) => panic!("expected a ledger rule, got a RangeError: {e}"),
    }
}

/// Fails unless `xs` has no repeats.
fn unique<'a>(what: &str, xs: impl IntoIterator<Item = &'a str>) -> Result<(), TestCaseError> {
    let mut seen = HashSet::new();
    for x in xs {
        prop_assert!(seen.insert(x), "{} {} appears twice", what, x);
    }
    Ok(())
}

/// The lines of each entry, in book order.
type LinesByEntry<'a> = BTreeMap<&'a str, Vec<&'a JournalLine>>;

/// A posted or reversed entry has two lines or more, which balance in base currency and within
/// each fund (`engine.ts:376-414`); every line has one side above zero, the other zero, and a base
/// amount of the same sign (or zero, when a tiny foreign amount converts to nothing).
fn entry_balances(e: &JournalEntry, lines: &[&JournalLine]) -> Result<(), TestCaseError> {
    prop_assert!(lines.len() >= 2, "{} has {} lines", e.number, lines.len());
    let mut by_fund: BTreeMap<&str, Minor> = BTreeMap::new();
    for l in lines {
        let one_side = (l.debit > 0 && l.credit == 0) || (l.credit > 0 && l.debit == 0);
        prop_assert!(
            one_side,
            "{} line {} has debit {} credit {}",
            e.number,
            l.id,
            l.debit,
            l.credit
        );
        let sign_ok = l.base_amount == 0 || (l.base_amount > 0) == (l.debit > 0);
        prop_assert!(
            sign_ok,
            "{} line {} base {} against debit {}",
            e.number,
            l.id,
            l.base_amount,
            l.debit
        );
        *by_fund.entry(l.fund_id.as_str()).or_insert(0) += l.base_amount;
    }
    let total: Minor = by_fund.values().sum();
    prop_assert_eq!(total, 0, "{} does not balance in base currency", e.number);
    for (fund, t) in by_fund {
        prop_assert_eq!(t, 0, "{} does not balance in fund {}", e.number, fund);
    }
    Ok(())
}

/// Reversals pair up: an entry is `reversed` exactly when it names the reversal that undid it,
/// and that reversal is a posted `reversal` entry naming it back whose lines mirror its lines
/// (`engine.ts:606-662`).
fn reversal_pairs(
    state: &LedgerState,
    e: &JournalEntry,
    by_entry: &LinesByEntry<'_>,
) -> Result<(), TestCaseError> {
    let lines_of = |id: &str| by_entry.get(id).cloned().unwrap_or_default();
    prop_assert_eq!(
        e.status == EntryStatus::Reversed,
        e.reversed_by.is_some(),
        "{} is {:?} with reversed_by {:?}",
        e.number,
        e.status,
        e.reversed_by
    );
    if let Some(by) = &e.reversed_by {
        let r = state.entry(by);
        prop_assert!(r.is_some(), "{} names a missing reversal", e.number);
        let r = r.unwrap();
        prop_assert_eq!(r.reversal_of.as_ref(), Some(&e.id));
        prop_assert_eq!(r.source_type, SourceType::Reversal);
        prop_assert_eq!(r.status, EntryStatus::Posted);
        let (lines, mirror) = (lines_of(e.id.as_str()), lines_of(by.as_str()));
        prop_assert_eq!(mirror.len(), lines.len(), "{} and its reversal", e.number);
        for (o, m) in lines.iter().zip(mirror) {
            prop_assert_eq!(
                (
                    &m.account_id,
                    &m.fund_id,
                    &m.grant_id,
                    m.debit,
                    m.credit,
                    m.base_amount
                ),
                (
                    &o.account_id,
                    &o.fund_id,
                    &o.grant_id,
                    o.credit,
                    o.debit,
                    -o.base_amount
                ),
                "{} line {} is not mirrored",
                e.number,
                o.id
            );
        }
    }
    if let Some(of) = &e.reversal_of {
        let o = state.entry(of);
        prop_assert!(
            o.is_some_and(|o| o.reversed_by.as_ref() == Some(&e.id)),
            "{} reverses {} which does not name it",
            e.number,
            of
        );
    }
    Ok(())
}

/// The invariants every reachable state of the books keeps (`Docs/flowtrace-ledger-plan.md`
/// section 5, "Invariants enforced in the database, not the UI"; `engine.ts:1-11`): every posted
/// or reversed entry balances ([`entry_balances`]) and reversals pair up ([`reversal_pairs`]); no
/// line outlives its entry; entry, line, audit and statement-line ids and entry numbers are
/// unique; the trial balance and the statement of financial position foot (`reports.ts:1-4`).
pub fn books_hold(state: &LedgerState) -> Result<(), TestCaseError> {
    let mut by_entry: LinesByEntry<'_> = BTreeMap::new();
    for l in &state.lines {
        by_entry.entry(l.entry_id.as_str()).or_default().push(l);
    }
    for id in by_entry.keys() {
        prop_assert!(
            state.entry(&(*id).into()).is_some(),
            "line of a missing entry {}",
            id
        );
    }
    for e in state
        .entries
        .iter()
        .filter(|e| e.status != EntryStatus::Draft)
    {
        entry_balances(e, &by_entry.get(e.id.as_str()).cloned().unwrap_or_default())?;
        reversal_pairs(state, e, &by_entry)?;
    }
    unique("entry id", state.entries.iter().map(|e| e.id.as_str()))?;
    unique(
        "entry number",
        state.entries.iter().map(|e| e.number.as_str()),
    )?;
    unique("line id", state.lines.iter().map(|l| l.id.as_str()))?;
    unique("audit id", state.audit.iter().map(|a| a.id.as_str()))?;
    unique(
        "statement line id",
        state.statement_lines.iter().map(|s| s.id.as_str()),
    )?;
    prop_assert_eq!(
        trial_balance(state).difference,
        0,
        "the trial balance does not foot"
    );
    let last = state
        .periods
        .iter()
        .map(|p| p.end_date.as_str())
        .max()
        .unwrap_or("2026-12-31");
    prop_assert_eq!(
        financial_position(state, last).difference,
        0,
        "the balance sheet does not balance"
    );
    Ok(())
}
