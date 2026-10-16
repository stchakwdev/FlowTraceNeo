//! `flowtrace/src/lib/ledger/review-regressions.test.ts`, one `#[test]` per TS `it`, named as
//! the TS test in snake case and citing its lines: the bugs found in review of the first ledger
//! version stay fixed. Each builds its books as the TS test does, from `books()`
//! (`review-regressions.test.ts:22-29`, here [`regression_books`]). Three of them are also
//! replayed step by step, with exact messages, from `rejected-cases.json` (`tests/rejected.rs`).
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use common::books::{
    U, acc, admin, clerk, date, entry, line, regression_books, total_expenses, trial_balance,
};
use common::strategies::js;
use ledger_core::commands::{CommandError, CommandResult, LedgerCommand};
use ledger_core::engine::{
    AccountPatch, EntryOptions, Error, Ledger, NewBill, NewCodedLine, PaymentInput, StatementRow,
};
use ledger_core::env::fixed::FixedEnv;
use ledger_core::reports::{Basis, grant_summary};
use ledger_core::types::{
    AccountType, Actor, AppliesTo, Bill, BillStatus, EntryInput, GrantId, LedgerState,
    NormalBalance, Nullish, PeriodId, PeriodStatus, SourceType,
};
use ledger_core::{IssueCode, Minor, apply_command, sanitize_command};
use serde_json::{Value, json};

/// `run(l, actor, cmd)` (`review-regressions.test.ts:30`): sanitise, then apply.
fn run(
    state: &mut LedgerState,
    env: &mut FixedEnv,
    actor: &Actor,
    raw: &Value,
) -> Result<CommandResult, IssueCode> {
    let cmd = sanitize_command(raw).map_err(|e| e.code())?;
    apply_command(&mut Ledger::new(state, env), actor, cmd).map_err(|e| match e {
        CommandError::Engine(e) => code(&e),
    })
}

/// `code(fn)` (`review-regressions.test.ts:31-39`): the rejection's code.
fn code(e: &Error) -> IssueCode {
    match e {
        Error::Ledger(e) => e.code(),
        Error::Money(e) => panic!("expected a LedgerError, got a RangeError: {e}"),
    }
}

/// `spend(amount)` (`review-regressions.test.ts:40-47`).
fn spend(amount: Minor) -> EntryInput {
    entry(
        "2026-02-01",
        "Spend",
        vec![line("7100", U, amount, 0), line("1110", U, 0, amount)],
    )
}

/// A coded bill line on 5500.
fn coded(amount: Minor, fund: &str, grant: Option<&str>) -> NewCodedLine {
    NewCodedLine {
        description: "x".to_owned(),
        account_id: acc("5500"),
        amount: js(amount),
        fund_id: fund.into(),
        grant_id: grant.map(GrantId::from),
        dimension_ids: Vec::new(),
    }
}

/// A bill from vendor `v`, dated 2026-02-01 and due 2026-03-01.
fn new_bill(number: &str, currency: &str, fx_rate: f64, lines: Vec<NewCodedLine>) -> NewBill {
    NewBill {
        id: None,
        vendor_id: "v".into(),
        number: number.to_owned(),
        bill_date: date("2026-02-01"),
        due_date: date("2026-03-01"),
        currency: currency.to_owned(),
        fx_rate,
        lines,
        proof_ids: Vec::new(),
    }
}

/// `bill(l, lines, currency, fx_rate)` (`review-regressions.test.ts:85-93`): entered by the
/// clerk, approved and posted by the admin.
fn bill(l: &mut Ledger<'_>, lines: Vec<NewCodedLine>, currency: &str, fx_rate: f64) -> Bill {
    let b = l
        .create_bill(&clerk(), new_bill("B", currency, fx_rate, lines))
        .unwrap();
    l.approve_bill(&admin(), &b.id).unwrap();
    l.post_bill(&admin(), &b.id, None).unwrap();
    l.state()
        .bills
        .iter()
        .find(|x| x.id == b.id)
        .unwrap()
        .clone()
}

/// A payment from 1110 on 2026-02-02.
fn payment(amount: Minor) -> PaymentInput {
    PaymentInput {
        date: "2026-02-02".to_owned(),
        cash_account_id: acc("1110"),
        amount: js(amount),
        reference: Nullish::Undefined,
        fx_rate: None,
    }
}

/// Net base amount on `account` in `fund`.
fn base(state: &LedgerState, account: &str, fund: &str) -> Minor {
    state
        .lines
        .iter()
        .filter(|l| l.account_id == acc(account) && l.fund_id.as_str() == fund)
        .map(|l| l.base_amount)
        .sum()
}

/// `review-regressions.test.ts:50-57`: `account.update` keeps the type, code and normal balance
/// (only the name changes), so the posted spend still reads as expense.
#[test]
fn cannot_change_an_accounts_type_code_or_normal_balance() {
    let (mut state, mut env) = regression_books();
    Ledger::new(&mut state, &mut env)
        .post(&admin(), &spend(100), EntryOptions::default())
        .unwrap();
    let patch = json!({
        "type": "asset", "normal_balance": "debit", "code": "9", "id": "x", "name": "Stationery"
    });
    run(
        &mut state,
        &mut env,
        &admin(),
        &json!({ "type": "account.update", "accountId": acc("7100"), "patch": patch }),
    )
    .unwrap();
    let a = state.account(&acc("7100")).unwrap();
    assert_eq!(
        (
            a.account_type,
            a.code.as_str(),
            a.normal_balance,
            a.name.as_str()
        ),
        (
            AccountType::Expense,
            "7100",
            NormalBalance::Debit,
            "Stationery"
        )
    );
    assert_eq!(
        total_expenses(&state, "2026-01-01", "2026-12-31", Basis::Accrual),
        100
    );
}

/// `review-regressions.test.ts:59-64`: a command cannot post a manual entry as a system source.
#[test]
fn cannot_post_a_manual_entry_as_a_system_source() {
    let (mut state, mut env) = regression_books();
    let mut input = serde_json::to_value(spend(5_000)).unwrap();
    input["source_type"] = json!("year_end_close");
    input["source_id"] = json!("x");
    run(
        &mut state,
        &mut env,
        &admin(),
        &json!({ "type": "entry.post", "input": input }),
    )
    .unwrap();
    assert_eq!(state.entries[0].source_type, SourceType::Manual);
    assert_eq!(
        total_expenses(&state, "2026-01-01", "2026-12-31", Basis::Accrual),
        5_000
    );
}

/// `review-regressions.test.ts:66-73`: an unknown command is `INVALID_INPUT`, a string rate is
/// `FX_RATE_REQUIRED`, and a caller-chosen vendor id is replaced, so it cannot clash.
#[test]
fn rejects_unknown_commands_string_rates_and_caller_chosen_ids() {
    let (mut state, mut env) = regression_books();
    let before = state.clone();
    assert_eq!(
        run(&mut state, &mut env, &admin(), &json!({ "type": "nope" })).unwrap_err(),
        IssueCode::InvalidInput
    );
    let usd = |account: &str, side: &str| {
        let mut l = json!({ "account_id": acc(account), "currency": "USD", "fx_rate": "130" });
        l[side] = json!(1);
        l["fund_id"] = json!(U);
        l
    };
    let fx_string = json!({ "type": "entry.post", "input": {
        "date": "2026-02-01", "memo": "x", "lines": [usd("1110", "debit"), usd("4100", "credit")]
    } });
    assert_eq!(
        run(&mut state, &mut env, &admin(), &fx_string).unwrap_err(),
        IssueCode::FxRateRequired
    );
    assert_eq!(state, before);
    run(
        &mut state,
        &mut env,
        &admin(),
        &json!({ "type": "vendor.add", "input": {
            "id": "v", "name": "Twin", "node_id": null, "payment_details": null
        } }),
    )
    .unwrap();
    assert_eq!(
        state
            .vendors
            .iter()
            .filter(|v| v.id.as_str() == "v")
            .count(),
        1
    );
}

/// `review-regressions.test.ts:77-81`: segregation of duties.
#[test]
fn a_bill_cannot_be_approved_by_the_person_who_entered_it() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    let mut input = new_bill("1", "KES", 1.0, vec![coded(100, U, None)]);
    input.lines[0].account_id = acc("7100");
    let b = l.create_bill(&admin(), input).unwrap();
    assert_eq!(
        code(&l.approve_bill(&admin(), &b.id).unwrap_err()),
        IssueCode::SelfApproval
    );
}

/// `review-regressions.test.ts:95-99`: lines convert one by one and the payable as a total; the
/// cent between them goes to exchange differences, so the trial balance foots.
#[test]
fn posts_a_multi_line_foreign_currency_bill_with_rounding() {
    let (mut state, mut env) = regression_books();
    bill(
        &mut Ledger::new(&mut state, &mut env),
        vec![coded(1, U, None), coded(1, U, None)],
        "USD",
        1.5,
    );
    assert_eq!(trial_balance(&state).difference, 0);
    let rounding: Vec<_> = state
        .lines
        .iter()
        .filter(|l| l.account_id == acc("4400"))
        .map(|l| (l.credit, l.currency.as_str(), l.memo.as_deref()))
        .collect();
    assert_eq!(rounding, vec![(1, "KES", Some("Currency rounding"))]);
}

/// `review-regressions.test.ts:101-108`: 300 payments of one cent against a bill split 100 and
/// 200 across two funds leave every fund's payable at zero and the bill paid.
#[test]
fn partial_payments_clear_every_funds_payable_exactly() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    let b = bill(
        &mut l,
        vec![coded(100, U, None), coded(200, "f-r", Some("g"))],
        "KES",
        1.0,
    );
    for _ in 0..300 {
        l.pay_bill(&admin(), &b.id, &payment(1)).unwrap();
    }
    assert_eq!(l.state().bills[0].status, BillStatus::Paid);
    assert_eq!(l.state().outstanding(&AppliesTo::Bill(b.id)).unwrap(), 0);
    for fund in [U, "f-r"] {
        assert_eq!(base(&state, "2100", fund), 0, "{fund}");
    }
}

/// `review-regressions.test.ts:110-121`: a bill with a payment cannot be reversed; reversing
/// the payment reopens the bill, and then reversing the bill voids it, leaving no cash-basis
/// expense.
#[test]
fn reversing_a_payment_reopens_the_bill_a_paid_bill_cannot_be_reversed() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    let b = bill(&mut l, vec![coded(100, U, None)], "KES", 1.0);
    let bill_entry = b.entry_id.clone().unwrap();
    let p = l.pay_bill(&admin(), &b.id, &payment(100)).unwrap();
    let err = l
        .reverse(&admin(), &bill_entry, "2026-02-03", "wrong")
        .unwrap_err();
    assert_eq!(code(&err), IssueCode::InvalidState);
    l.reverse(&admin(), &p.entry_id, "2026-02-03", "bounced")
        .unwrap();
    assert_eq!(l.state().bills[0].status, BillStatus::Posted);
    assert_eq!(
        l.state()
            .outstanding(&AppliesTo::Bill(b.id.clone()))
            .unwrap(),
        100
    );
    l.reverse(&admin(), &bill_entry, "2026-02-03", "duplicate bill")
        .unwrap();
    assert_eq!(l.state().bills[0].status, BillStatus::Void);
    assert_eq!(
        total_expenses(&state, "2026-01-01", "2026-12-31", Basis::Cash),
        0
    );
}

/// `review-regressions.test.ts:123-128`: an approved USD bill commits its grant in base currency
/// (USD 10.00 at 130 is KES 1,300.00).
#[test]
fn reports_commitments_in_base_currency() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    let b = l
        .create_bill(
            &clerk(),
            new_bill("U", "USD", 130.0, vec![coded(1_000, "f-r", Some("g"))]),
        )
        .unwrap();
    l.approve_bill(&admin(), &b.id).unwrap();
    let summary = grant_summary(&state, &GrantId::from("g"), &date("2026-02-01")).unwrap();
    assert_eq!(summary.committed, 130_000);
}

/// `review-regressions.test.ts:132-145`: the year closes with a deactivated account and a grant
/// refund (a net credit on a restricted expense line), and closes again after December is
/// reopened, the close reversed and a late invoice posted.
#[test]
fn closes_with_a_deactivated_account_and_a_grant_refund_and_again_after_reopening() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    l.post(&admin(), &spend(100), EntryOptions::default())
        .unwrap();
    let off = AccountPatch {
        is_active: Some(false),
        ..AccountPatch::default()
    };
    l.update_account(&admin(), &acc("7100"), &off).unwrap();
    let mut refund = entry(
        "2026-03-01",
        "Refund",
        vec![line("1130", "f-r", 50, 0), line("5500", "f-r", 0, 50)],
    );
    refund.lines[1].grant_id = Nullish::Value(GrantId::from("g"));
    l.post(&admin(), &refund, EntryOptions::default()).unwrap();
    let close = l.close_year(&admin(), 2026).unwrap().unwrap();
    l.reopen_period(&admin(), &PeriodId::from("period-2026-12"), "late invoice")
        .unwrap();
    l.reverse(&admin(), &close.id, "2026-12-31", "late invoice")
        .unwrap();
    let late = entry(
        "2026-12-30",
        "Spend",
        vec![line("7300", U, 25, 0), line("1110", U, 0, 25)],
    );
    l.post(&admin(), &late, EntryOptions::default()).unwrap();
    l.close_year(&admin(), 2026).unwrap();
    let tb = trial_balance(&state);
    let open: Vec<_> = tb
        .rows
        .iter()
        .filter(|r| {
            matches!(
                r.account.account_type,
                AccountType::Expense | AccountType::Income
            )
        })
        .map(|r| r.account.code.as_str())
        .collect();
    assert!(open.is_empty(), "income and expense left open: {open:?}");
}

/// `review-regressions.test.ts:149-155`: two identical rows in one file are both kept; the same
/// file again adds nothing; a file with one more adds one.
#[test]
fn keeps_identical_rows_within_one_file_skips_them_on_re_import() {
    let (mut state, mut env) = regression_books();
    let mut l = Ledger::new(&mut state, &mut env);
    let row = StatementRow {
        date: date("2026-02-01"),
        description: "ATM WITHDRAWAL".to_owned(),
        reference: None,
        amount: js(-100_000),
    };
    let bank = "b".into();
    let mut import = |n: usize| {
        l.import_statement(&admin(), &bank, &vec![row.clone(); n])
            .unwrap()
            .len()
    };
    assert_eq!(import(2), 2);
    assert_eq!(import(2), 0);
    assert_eq!(import(3), 1);
}

/// `review-regressions.test.ts:159-163`: `year.open` (applied without the sanitiser, as the TS
/// test does) opens the year's periods.
#[test]
fn opens_a_new_year_by_command() {
    let (mut state, mut env) = regression_books();
    apply_command(
        &mut Ledger::new(&mut state, &mut env),
        &admin(),
        LedgerCommand::YearOpen { year: 2027 },
    )
    .unwrap();
    assert_eq!(
        state.period_for("2027-06-15").map(|p| p.status),
        Some(PeriodStatus::Open)
    );
}
