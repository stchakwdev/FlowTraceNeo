//! Rules of the C2 methods that no fixture rejects, with the messages written in
//! `flowtrace/src/lib/ledger/engine.ts` (cited per case), on fresh `createOrgLedger` books. Each
//! rejection must leave the books and the env as they were.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::fixture;
use ledger_core::IssueCode;
use ledger_core::engine::{AccountPatch, Error, Ledger, NewAccount};
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::types::{AccountId, AccountType, Actor, LedgerState, Nullish, PeriodId};

fn books() -> LedgerState {
    serde_json::from_value(fixture("setup")["createOrgLedger"]["state"].clone()).unwrap()
}

fn admin() -> Actor {
    serde_json::from_value(serde_json::json!({
        "id": "u1", "name": "Admin", "can_post": true, "can_approve": true, "can_close": true
    }))
    .unwrap()
}

fn account(code: &str, account_type: AccountType, parent: Option<&str>) -> NewAccount {
    NewAccount {
        id: None,
        code: code.to_owned(),
        name: "New account".to_owned(),
        account_type,
        parent_id: parent.map(AccountId::from),
        is_group: false,
        is_active: None,
        is_cash: false,
        expense_function: None,
        cash_flow_class: None,
    }
}

/// Runs `f` on fresh books and asserts it is rejected with `code` and `message`, changing
/// nothing and drawing no id.
fn rejects(f: impl FnOnce(&mut Ledger<'_>) -> Result<(), Error>, code: IssueCode, message: &str) {
    let mut state = books();
    let mut env: FixedEnv = fixed::test();
    let err = f(&mut Ledger::new(&mut state, &mut env)).unwrap_err();
    let Error::Ledger(err) = err else {
        panic!("expected a LedgerError, got {err}")
    };
    assert_eq!((err.code(), err.to_string().as_str()), (code, message));
    assert_eq!(state, books(), "{message}: the books changed");
    assert_eq!(env.drawn(), 0, "{message}: ids were drawn");
}

#[test]
fn add_account_rules() {
    // engine.ts:177-178
    rejects(
        |l| {
            l.add_account(&admin(), account(" ", AccountType::Expense, None))
                .map(drop)
        },
        IssueCode::InvalidInput,
        "An account needs a code and a name.",
    );
    // engine.ts:180-181
    rejects(
        |l| {
            l.add_account(&admin(), account("1110", AccountType::Asset, None))
                .map(drop)
        },
        IssueCode::Duplicate,
        "Account code 1110 is already in use.",
    );
    // engine.ts:184
    rejects(
        |l| {
            l.add_account(
                &admin(),
                account("7999", AccountType::Expense, Some("acc-x")),
            )
            .map(drop)
        },
        IssueCode::UnknownAccount,
        "The parent account does not exist.",
    );
    // engine.ts:185-186
    rejects(
        |l| {
            l.add_account(
                &admin(),
                account("7999", AccountType::Expense, Some("acc-7100")),
            )
            .map(drop)
        },
        IssueCode::InvalidInput,
        "The parent must be a header account.",
    );
    // engine.ts:187-188
    rejects(
        |l| {
            l.add_account(
                &admin(),
                account("7999", AccountType::Asset, Some("acc-7000")),
            )
            .map(drop)
        },
        IssueCode::InvalidInput,
        "A sub-account must have its parent\u{2019}s type.",
    );
}

#[test]
fn add_account_trims_and_draws_the_id_then_the_audit_id() {
    let mut state = books();
    let mut env = fixed::test();
    let mut new = account(" 7999 ", AccountType::Expense, Some("acc-7000"));
    new.name = " Bank charges ".to_owned();
    let a = Ledger::new(&mut state, &mut env)
        .add_account(&admin(), new)
        .unwrap();
    assert_eq!(
        (a.id.as_str(), a.code.as_str(), a.name.as_str()),
        ("id-1", "7999", "Bank charges")
    );
    let audit = state.audit.last().unwrap();
    assert_eq!(
        (
            audit.id.as_str(),
            audit.action.as_str(),
            audit.detail.as_str()
        ),
        ("id-2", "account.create", "7999 Bank charges")
    );
}

#[test]
fn update_account_rules() {
    let patch = |f: fn(&mut AccountPatch)| {
        let mut p = AccountPatch::default();
        f(&mut p);
        p
    };
    // engine.ts:209
    rejects(
        |l| {
            l.update_account(&admin(), &"acc-x".into(), &AccountPatch::default())
                .map(drop)
        },
        IssueCode::NotFound,
        "Account not found.",
    );
    // engine.ts:210-211
    rejects(
        |l| {
            let p = patch(|p| p.name = Some("  ".into()));
            l.update_account(&admin(), &"acc-7100".into(), &p).map(drop)
        },
        IssueCode::InvalidInput,
        "An account needs a name.",
    );
    // engine.ts:218
    rejects(
        |l| {
            let p = patch(|p| p.expense_function = Nullish::Null);
            l.update_account(&admin(), &"acc-1110".into(), &p).map(drop)
        },
        IssueCode::InvalidInput,
        "Only expense accounts have a function.",
    );
    // engine.ts:219-220
    rejects(
        |l| {
            let p = patch(|p| p.expense_function = Nullish::Null);
            l.update_account(&admin(), &"acc-7100".into(), &p).map(drop)
        },
        IssueCode::InvalidInput,
        "Choose programme, management or fundraising.",
    );
    // engine.ts:223-224
    rejects(
        |l| {
            let p = patch(|p| p.is_active = Some(false));
            l.update_account(&admin(), &"acc-2100".into(), &p).map(drop)
        },
        IssueCode::InvalidInput,
        "2100 Accounts payable is used by the system and cannot be deactivated.",
    );
}

#[test]
fn update_account_audits_the_applied_patch_as_json() {
    let mut state = books();
    let mut env = fixed::test();
    let p = AccountPatch {
        name: Some(" Office \"supplies\" ".to_owned()),
        is_active: Some(false),
        ..AccountPatch::default()
    };
    Ledger::new(&mut state, &mut env)
        .update_account(&admin(), &"acc-7100".into(), &p)
        .unwrap();
    // JSON.stringify(clean), keys in the order engine.ts:215-221 sets them.
    assert_eq!(
        state.audit.last().unwrap().detail,
        r#"{"name":"Office \"supplies\"","is_active":false}"#
    );
}

#[test]
fn period_and_year_rules() {
    let clerk: Actor = serde_json::from_value(serde_json::json!({
        "id": "u2", "name": "Clerk", "can_post": true, "can_approve": false, "can_close": false
    }))
    .unwrap();
    // engine.ts:265-266
    rejects(
        |l| l.open_fiscal_year(&admin(), 1999),
        IssueCode::InvalidInput,
        "Choose a year between 2000 and 2100.",
    );
    // engine.ts:154-163
    rejects(
        |l| l.open_fiscal_year(&clerk, 2027),
        IssueCode::Permission,
        "Clerk does not have permission to close periods.",
    );
    // engine.ts:669
    rejects(
        |l| {
            l.lock_period(&admin(), &PeriodId::from("period-1999-01"))
                .map(drop)
        },
        IssueCode::NotFound,
        "Period not found.",
    );
    // engine.ts:708
    rejects(
        |l| {
            l.lock_period(&admin(), &PeriodId::from("period-2026-01"))
                .map(drop)
        },
        IssueCode::InvalidState,
        "Close Jan 2026 before locking it.",
    );
    // engine.ts:693
    rejects(
        |l| {
            l.reopen_period(&admin(), &PeriodId::from("period-2026-01"), "x")
                .map(drop)
        },
        IssueCode::InvalidState,
        "Jan 2026 is already open.",
    );
    // engine.ts:723
    rejects(
        |l| l.close_year(&admin(), 2030).map(drop),
        IssueCode::NotFound,
        "There are no periods for 2030.",
    );
}

#[test]
fn closing_an_empty_year_closes_its_periods_with_nothing_to_post() {
    let mut state = books();
    let mut env = fixed::test();
    let entry = Ledger::new(&mut state, &mut env)
        .close_year(&admin(), 2026)
        .unwrap();
    assert!(entry.is_none());
    assert!(
        state
            .periods
            .iter()
            .all(|p| p.status != ledger_core::types::PeriodStatus::Open)
    );
    let audit = state.audit.last().unwrap();
    // engine.ts:762-772
    assert_eq!(
        (
            audit.action.as_str(),
            audit.subject.as_str(),
            audit.detail.as_str()
        ),
        ("year.close", "2026", "nothing to close")
    );
    assert_eq!(env.drawn(), 1);
}
