//! Rules of the C3 methods (grants, budgets, bills, payments, claims) that no fixture rejects,
//! with the messages written in `flowtrace/src/lib/ledger/engine.ts` (cited per case), and the
//! TS tests on documents that no fixture replays (`engine.test.ts:217-239`; the regressions of
//! `review-regressions.test.ts` are in `review_regressions.rs`). Each rejection must leave the
//! books and the env as they were.
#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

mod common;

use IssueCode::{
    Duplicate, InvalidInput, InvalidState, NegativeAmount, NotFound, Permission, SelfApproval,
    TooFewLines, UnknownAccount, UnknownFund,
};
use common::fixture;
use ledger_core::engine::{
    Error, Ledger, NewBill, NewBudgetLine, NewClaim, NewGrant, PaymentInput, ReimburseInput,
};
use ledger_core::env::fixed::{self, FixedEnv};
use ledger_core::types::{
    AccountId, Actor, AppliesTo, BillId, BillStatus, BudgetLineId, ClaimId, ClaimStatus, GrantId,
    LedgerState,
};
use ledger_core::{IssueCode, Minor};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn actor(id: &str, name: &str, approve: bool) -> Actor {
    serde_json::from_value(json!({
        "id": id, "name": name, "can_post": id != "u9", "can_approve": approve, "can_close": approve
    }))
    .unwrap()
}

/// `admin`, `clerk` and the field officer of the TS tests (`engine.test.ts:12-14`,
/// `engine.test.ts:243`).
fn admin() -> Actor {
    actor("u1", "Admin", true)
}
fn clerk() -> Actor {
    actor("u2", "Clerk", false)
}
fn officer() -> Actor {
    actor("u9", "Field Officer", false)
}

fn from<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}

/// `books()` of `review-regressions.test.ts:22-29` less the bank account (C6): fresh
/// `createOrgLedger` books with a restricted fund `f-r`, its grant `g` of 100,000.00 and a
/// vendor `v`, plus a budget `bud`.
fn books() -> (LedgerState, FixedEnv) {
    let mut state: LedgerState = from(fixture("setup")["createOrgLedger"]["state"].clone());
    let mut env = fixed::test();
    let mut l = Ledger::new(&mut state, &mut env);
    l.add_fund(
        &admin(),
        from(json!({ "id": "f-r", "name": "Restricted", "restriction": "temporarily_restricted" })),
    )
    .unwrap();
    l.add_grant(&admin(), grant(json!({}))).unwrap();
    l.add_vendor(
        &admin(),
        from(json!({ "id": "v", "name": "Vendor", "node_id": null, "payment_details": null })),
    )
    .unwrap();
    l.add_budget(
        &admin(),
        from(json!({ "id": "bud", "name": "Budget", "fiscal_year": 2026, "status": "draft" })),
    )
    .unwrap();
    (state, env)
}

fn with(base: Value, patch: Value) -> Value {
    let mut v = base;
    let Value::Object(patch) = patch else {
        panic!("a patch is an object")
    };
    for (k, x) in patch {
        v[k] = x;
    }
    v
}

fn grant(patch: Value) -> NewGrant {
    from(with(
        json!({
            "id": "g", "name": "Grant", "donor_node_id": null, "donor_name": "D", "fund_id": "f-r",
            "award": 10_000_000, "currency": "KES", "start_date": "2026-01-01",
            "end_date": "2026-12-31", "terms": ""
        }),
        patch,
    ))
}

fn coded(amount: Value, fund: &str, grant: Option<&str>) -> Value {
    let mut line = json!({
        "description": "x", "account_id": "acc-5500", "fund_id": fund, "grant_id": grant,
        "dimension_ids": []
    });
    line["amount"] = amount;
    line
}

fn bill(patch: Value) -> NewBill {
    from(with(
        json!({
            "vendor_id": "v", "number": "B", "bill_date": "2026-02-01", "due_date": "2026-03-01",
            "currency": "KES", "fx_rate": 1, "proof_ids": [],
            "lines": [coded(json!(300), "fund-unrestricted", None)]
        }),
        patch,
    ))
}

fn claim(patch: Value) -> NewClaim {
    from(with(
        json!({
            "claim_date": "2026-02-03", "currency": "KES", "fx_rate": 1, "purpose": "Site visit",
            "receipt_names": [], "captured_offline": false,
            "lines": [coded(json!(250_000), "fund-unrestricted", None)]
        }),
        patch,
    ))
}

fn payment(amount: Value) -> PaymentInput {
    let mut input = json!({ "date": "2026-02-02", "cash_account_id": "acc-1110" });
    input["amount"] = amount;
    from(input)
}

fn reimbursement() -> ReimburseInput {
    from(json!({ "date": "2026-02-04", "cash_account_id": "acc-1150", "reference": "QAB12XYZ" }))
}

/// A bill entered by the clerk, approved by the admin and, with `post`, posted (the `bill`
/// helper of `review-regressions.test.ts:85-93`).
fn entered_bill(l: &mut Ledger<'_>, input: NewBill, post: bool) -> BillId {
    let b = l.create_bill(&clerk(), input).unwrap();
    if post {
        l.approve_bill(&admin(), &b.id).unwrap();
        l.post_bill(&admin(), &b.id, None).unwrap();
    }
    b.id
}

/// Runs `f` on `books` and asserts it is rejected with `code` (`None` for a `RangeError`) and
/// `message`, changing nothing and drawing no id.
fn rejects(
    books: &(LedgerState, FixedEnv),
    f: impl FnOnce(&mut Ledger<'_>) -> Result<(), Error>,
    code: Option<IssueCode>,
    message: &str,
) {
    let (mut state, mut env) = books.clone();
    let err = f(&mut Ledger::new(&mut state, &mut env)).unwrap_err();
    let got = match &err {
        Error::Ledger(e) => Some(e.code()),
        Error::Money(_) => None,
    };
    assert_eq!((got, err.to_string().as_str()), (code, message));
    assert_eq!(state, books.0, "{message}: the books changed");
    assert_eq!(env.drawn(), books.1.drawn(), "{message}: ids were drawn");
}

/// Books with one more step applied.
fn after(
    books: &(LedgerState, FixedEnv),
    f: impl FnOnce(&mut Ledger<'_>),
) -> (LedgerState, FixedEnv) {
    let (mut state, mut env) = books.clone();
    f(&mut Ledger::new(&mut state, &mut env));
    (state, env)
}

/// Net base amount of the lines on `account`, for one fund or all.
fn base(state: &LedgerState, account: &str, fund: Option<&str>) -> Minor {
    state
        .lines
        .iter()
        .filter(|l| l.account_id.as_str() == account)
        .filter(|l| fund.is_none_or(|f| l.fund_id.as_str() == f))
        .map(|l| l.base_amount)
        .sum()
}

/// The trial balance difference (`reports.ts` `trialBalance`): every line's base amount.
fn difference(state: &LedgerState) -> Minor {
    state.lines.iter().map(|l| l.base_amount).sum()
}

#[test]
fn grant_rules() {
    let b = books();
    let viewer = actor("u3", "Viewer", false);
    let viewer = Actor {
        can_post: false,
        ..viewer
    };
    // engine.ts:240 (require)
    rejects(
        &b,
        |l| l.add_grant(&viewer, grant(json!({ "id": "g2" }))).map(drop),
        Some(Permission),
        "Viewer does not have permission to post to the ledger.",
    );
    // engine.ts:241
    rejects(
        &b,
        |l| {
            l.add_grant(&admin(), grant(json!({ "id": "g2", "fund_id": "f-x" })))
                .map(drop)
        },
        Some(UnknownFund),
        "Fund not found.",
    );
    // engine.ts:242
    rejects(
        &b,
        |l| l.add_grant(&admin(), grant(json!({}))).map(drop),
        Some(Duplicate),
        "A grant with this id already exists.",
    );
    // engine.ts:243 (assertMinor)
    rejects(
        &b,
        |l| {
            l.add_grant(&admin(), grant(json!({ "id": "g2", "award": 10.5 })))
                .map(drop)
        },
        None,
        "Award must be a whole number of minor units.",
    );
    // engine.ts:244
    rejects(
        &b,
        |l| {
            l.add_grant(&admin(), grant(json!({ "id": "g2", "award": 0 })))
                .map(drop)
        },
        Some(InvalidInput),
        "The award must be positive.",
    );
    // engine.ts:245-246
    rejects(
        &b,
        |l| {
            l.add_grant(
                &admin(),
                grant(json!({ "id": "g2", "start_date": "2026-02-01", "end_date": "2026-01-31" })),
            )
            .map(drop)
        },
        Some(InvalidInput),
        "A grant cannot end before it starts.",
    );
}

#[test]
fn add_grant_draws_the_id_then_the_audit_id() {
    let (mut state, mut env) = books();
    let drawn = env.drawn();
    let g = Ledger::new(&mut state, &mut env)
        .add_grant(&admin(), grant(json!({ "id": null, "name": "Second" })))
        .unwrap();
    assert_eq!(g.id.as_str(), format!("id-{}", drawn + 1));
    let audit = state.audit.last().unwrap();
    assert_eq!(
        (
            audit.id.as_str(),
            audit.action.as_str(),
            audit.detail.as_str()
        ),
        (
            format!("id-{}", drawn + 2).as_str(),
            "grant.create",
            "Second"
        )
    );
}

#[test]
fn budget_line_rules() {
    let b = books();
    let line = |patch: Value| -> NewBudgetLine {
        from(with(
            json!({ "budget_id": "bud", "account_id": "acc-5500", "amount": 100 }),
            patch,
        ))
    };
    // engine.ts:790 (assertMinor)
    rejects(
        &b,
        |l| {
            l.set_budget_line(&admin(), line(json!({ "amount": 1.5 })))
                .map(drop)
        },
        None,
        "Budget amount must be a whole number of minor units.",
    );
    // engine.ts:791
    rejects(
        &b,
        |l| {
            l.set_budget_line(&admin(), line(json!({ "amount": -1 })))
                .map(drop)
        },
        Some(NegativeAmount),
        "A budget cannot be negative.",
    );
    // engine.ts:792-793
    rejects(
        &b,
        |l| {
            l.set_budget_line(&admin(), line(json!({ "budget_id": "bud-x" })))
                .map(drop)
        },
        Some(NotFound),
        "Budget not found.",
    );
    // engine.ts:794-795
    rejects(
        &b,
        |l| {
            l.set_budget_line(&admin(), line(json!({ "account_id": "acc-1110" })))
                .map(drop)
        },
        Some(InvalidInput),
        "Budget lines are set on expense accounts.",
    );
}

#[test]
fn set_budget_line_updates_only_the_fields_given() {
    let (mut state, mut env) = books();
    let mut l = Ledger::new(&mut state, &mut env);
    let first = l
        .set_budget_line(
            &admin(),
            from(json!({
                "budget_id": "bud", "account_id": "acc-5500", "grant_id": "g",
                "fund_id": "f-r", "amount": 100
            })),
        )
        .unwrap();
    assert_eq!(first.dimension_id, None);
    // engine.ts:796-799: Object.assign copies what the input carries; no new id is drawn.
    let updated = l
        .set_budget_line(
            &admin(),
            from(json!({ "id": first.id, "budget_id": "bud", "fund_id": null, "amount": 250 })),
        )
        .unwrap();
    assert_eq!(updated.id, first.id);
    assert_eq!(updated.amount, 250);
    assert_eq!(updated.grant_id, Some(GrantId::from("g")));
    assert_eq!(updated.account_id, Some(AccountId::from("acc-5500")));
    assert_eq!(updated.fund_id, None);
    assert_eq!(state.budget_lines, vec![updated]);
    // A new line with a given id keeps it (engine.ts:801).
    let given = Ledger::new(&mut state, &mut env)
        .set_budget_line(
            &admin(),
            from(json!({ "id": "bl-1", "budget_id": "bud", "account_id": null, "amount": 0 })),
        )
        .unwrap();
    assert_eq!(given.id, BudgetLineId::from("bl-1"));
}

#[test]
fn vendor_and_bill_rules() {
    let b = books();
    // engine.ts:812
    rejects(
        &b,
        |l| {
            l.add_vendor(
                &admin(),
                from(json!({ "name": " ", "node_id": null, "payment_details": null })),
            )
            .map(drop)
        },
        Some(InvalidInput),
        "A vendor needs a name.",
    );
    // engine.ts:830-831
    rejects(
        &b,
        |l| {
            l.create_bill(&clerk(), bill(json!({ "vendor_id": "v-x" })))
                .map(drop)
        },
        Some(NotFound),
        "Vendor not found.",
    );
    // engine.ts:1007 (checkCodedLines)
    rejects(
        &b,
        |l| {
            l.create_bill(&clerk(), bill(json!({ "lines": [] })))
                .map(drop)
        },
        Some(TooFewLines),
        "Add at least one line.",
    );
    // engine.ts:1009
    rejects(
        &b,
        |l| {
            let lines = json!([
                coded(json!(5), "fund-unrestricted", None),
                coded(json!(0.5), "f-r", None)
            ]);
            l.create_bill(&clerk(), bill(json!({ "lines": lines })))
                .map(drop)
        },
        None,
        "Line 2 amount must be a whole number of minor units.",
    );
    // engine.ts:1010
    rejects(
        &b,
        |l| {
            let lines = json!([coded(json!(0), "fund-unrestricted", None)]);
            l.create_bill(&clerk(), bill(json!({ "lines": lines })))
                .map(drop)
        },
        Some(NegativeAmount),
        "Line 1: the amount must be more than zero.",
    );
    // engine.ts:1012
    rejects(
        &b,
        |l| {
            let mut line = coded(json!(5), "fund-unrestricted", None);
            line["account_id"] = json!("acc-x");
            l.create_bill(&clerk(), bill(json!({ "lines": [line] })))
                .map(drop)
        },
        Some(UnknownAccount),
        "Line 1: choose an account.",
    );
    // engine.ts:1013
    rejects(
        &b,
        |l| {
            let lines = json!([coded(json!(5), "f-x", None)]);
            l.create_bill(&clerk(), bill(json!({ "lines": lines })))
                .map(drop)
        },
        Some(UnknownFund),
        "Line 1: choose a fund.",
    );
}

#[test]
fn bill_approval_and_posting_rules() {
    let b = books();
    let mut id = BillId::from("");
    let entered = after(&b, |l| id = entered_bill(l, bill(json!({})), false));
    // engine.ts:848 (require)
    rejects(
        &entered,
        |l| l.approve_bill(&clerk(), &id).map(drop),
        Some(Permission),
        "Clerk does not have permission to approve spend.",
    );
    // engine.ts:819
    rejects(
        &entered,
        |l| l.approve_bill(&admin(), &BillId::from("b-x")).map(drop),
        Some(NotFound),
        "Bill not found.",
    );
    // engine.ts:851
    let creator = Actor {
        can_approve: true,
        ..clerk()
    };
    rejects(
        &entered,
        |l| l.approve_bill(&creator, &id).map(drop),
        Some(SelfApproval),
        "You cannot approve a bill you entered.",
    );
    // engine.ts:861
    rejects(
        &entered,
        |l| l.post_bill(&admin(), &id, None).map(drop),
        Some(InvalidState),
        "Approve bill B before posting it.",
    );
    // engine.ts:893
    rejects(
        &entered,
        |l| l.pay_bill(&admin(), &id, &payment(json!(300))).map(drop),
        Some(InvalidState),
        "Bill B must be posted before it is paid.",
    );
    let approved = after(&entered, |l| {
        l.approve_bill(&admin(), &id).unwrap();
    });
    // engine.ts:850
    rejects(
        &approved,
        |l| l.approve_bill(&admin(), &id).map(drop),
        Some(InvalidState),
        "Bill B is approved.",
    );
    // engine.ts:154-163 through post (engine.ts:528)
    rejects(
        &approved,
        |l| l.post_bill(&officer(), &id, None).map(drop),
        Some(Permission),
        "Field Officer does not have permission to post to the ledger.",
    );
}

#[test]
fn payment_rules() {
    let b = books();
    let mut id = BillId::from("");
    let posted = after(&b, |l| id = entered_bill(l, bill(json!({})), true));
    // engine.ts:1076
    rejects(
        &posted,
        |l| {
            let mut p = payment(json!(100));
            p.cash_account_id = AccountId::from("acc-5500");
            l.pay_bill(&admin(), &id, &p).map(drop)
        },
        Some(UnknownAccount),
        "Choose the bank, mobile money or cash account the money left.",
    );
    // engine.ts:1077 (assertMinor)
    rejects(
        &posted,
        |l| l.pay_bill(&admin(), &id, &payment(json!(1.5))).map(drop),
        None,
        "Payment must be a whole number of minor units.",
    );
    // engine.ts:1079
    rejects(
        &posted,
        |l| l.pay_bill(&admin(), &id, &payment(json!(0))).map(drop),
        Some(NegativeAmount),
        "The payment must be more than zero.",
    );
    // engine.ts:1080
    rejects(
        &posted,
        |l| l.pay_bill(&admin(), &id, &payment(json!(301))).map(drop),
        Some(InvalidInput),
        "Only 3.00 is outstanding.",
    );
    // A payment posts through post (engine.ts:1123), so its checks apply.
    rejects(
        &posted,
        |l| {
            let mut p = payment(json!(100));
            p.date = "2027-02-02".to_owned();
            l.pay_bill(&admin(), &id, &p).map(drop)
        },
        Some(IssueCode::NoPeriod),
        "No fiscal period covers 2027-02-02.",
    );
}

#[test]
fn claim_rules() {
    let b = books();
    // engine.ts:918
    rejects(
        &b,
        |l| {
            l.create_claim(&officer(), claim(json!({ "purpose": " " })))
                .map(drop)
        },
        Some(InvalidInput),
        "Say what the money was spent on.",
    );
    let mut id = ClaimId::from("");
    let draft = after(&b, |l| {
        id = l.create_claim(&officer(), claim(json!({}))).unwrap().id;
    });
    // engine.ts:906
    rejects(
        &draft,
        |l| l.submit_claim(&officer(), &ClaimId::from("c-x")).map(drop),
        Some(NotFound),
        "Claim not found.",
    );
    // engine.ts:941
    rejects(
        &draft,
        |l| l.submit_claim(&admin(), &id).map(drop),
        Some(Permission),
        "Only the claimant can submit a claim.",
    );
    // engine.ts:956
    rejects(
        &draft,
        |l| l.approve_claim(&admin(), &id, None, None).map(drop),
        Some(InvalidState),
        "Claim EC-0001 is draft.",
    );
    // engine.ts:977
    rejects(
        &draft,
        |l| l.reject_claim(&admin(), &id, "No").map(drop),
        Some(InvalidState),
        "Claim EC-0001 is draft.",
    );
    // engine.ts:992
    rejects(
        &draft,
        |l| l.reimburse_claim(&admin(), &id, &reimbursement()).map(drop),
        Some(InvalidState),
        "Claim EC-0001 must be approved before it is reimbursed.",
    );
    let submitted = after(&draft, |l| {
        l.submit_claim(&officer(), &id).unwrap();
    });
    // engine.ts:942
    rejects(
        &submitted,
        |l| l.submit_claim(&officer(), &id).map(drop),
        Some(InvalidState),
        "Claim EC-0001 is submitted.",
    );
    // engine.ts:976 (require)
    rejects(
        &submitted,
        |l| l.reject_claim(&clerk(), &id, "No").map(drop),
        Some(Permission),
        "Clerk does not have permission to approve spend.",
    );
    // engine.ts:978
    rejects(
        &submitted,
        |l| l.reject_claim(&admin(), &id, " \t").map(drop),
        Some(InvalidInput),
        "Tell the claimant why the claim is rejected.",
    );
    // engine.ts:993: an approver who is the claimant cannot pay it out.
    let own = after(&b, |l| {
        let c = l
            .create_claim(&admin(), claim(json!({ "submit": true })))
            .unwrap();
        let approver = actor("u5", "Approver", true);
        l.approve_claim(&approver, &c.id, None, None).unwrap();
        id = c.id;
    });
    rejects(
        &own,
        |l| l.reimburse_claim(&admin(), &id, &reimbursement()).map(drop),
        Some(SelfApproval),
        "You cannot pay out your own claim.",
    );
}

#[test]
fn reject_claim_trims_the_note_on_the_trail_and_audits_it_as_given() {
    let (mut state, mut env) = books();
    let mut l = Ledger::new(&mut state, &mut env);
    let c = l
        .create_claim(&officer(), claim(json!({ "submit": true })))
        .unwrap();
    let c = l.reject_claim(&admin(), &c.id, " No receipts ").unwrap();
    assert_eq!(c.status, ClaimStatus::Rejected);
    let step = c.approvals.last().unwrap();
    assert_eq!(step.note.as_deref(), Some("No receipts"));
    let audit = state.audit.last().unwrap();
    assert_eq!(
        (audit.action.as_str(), audit.detail.as_str()),
        ("claim.reject", " No receipts ")
    );
}

/// `engine.test.ts:217-239`: a USD bill booked at 130 and paid at 128 clears payables in base
/// currency and books KES 200 of exchange gain.
#[test]
fn a_foreign_bill_paid_at_a_new_rate_books_the_fx_difference() {
    let (mut state, mut env) = books();
    let mut l = Ledger::new(&mut state, &mut env);
    let input = bill(json!({
        "currency": "USD", "fx_rate": 130, "bill_date": "2026-01-05",
        "lines": [coded(json!(10_000), "fund-unrestricted", None)]
    }));
    let id = entered_bill(&mut l, input, true);
    let mut p = payment(json!(10_000));
    p.date = "2026-02-01".to_owned();
    p.fx_rate = Some(128.0);
    let paid = l.pay_bill(&admin(), &id, &p).unwrap();
    assert_eq!(paid.fx_rate.to_bits(), 128.0_f64.to_bits());
    assert_eq!(difference(&state), 0);
    assert_eq!(base(&state, "acc-2100", None), 0);
    assert_eq!(base(&state, "acc-4400", None), -20_000);
    assert_eq!(state.bills[0].status, BillStatus::Paid);
}

/// `engine.test.ts:242-270`, past the rejections the fixture replays: approval posts the claim
/// against the grant, and reimbursement pays out exactly what is owed.
#[test]
fn claims_post_on_approval_and_reimburse_to_zero() {
    let (mut state, mut env) = books();
    let mut l = Ledger::new(&mut state, &mut env);
    let lines = json!([coded(json!(250_000), "f-r", Some("g"))]);
    let c = l
        .create_claim(&officer(), claim(json!({ "submit": true, "lines": lines })))
        .unwrap();
    assert_eq!(c.status, ClaimStatus::Submitted);
    let c = l
        .approve_claim(&admin(), &c.id, Some(" OK "), None)
        .unwrap();
    assert_eq!(c.status, ClaimStatus::Posted);
    let notes: Vec<_> = c.approvals.iter().map(|s| s.note.as_deref()).collect();
    assert_eq!(notes, vec![None, Some("OK"), Some("JE-0001")]);
    assert_eq!(l.state().grant_spend(&GrantId::from("g"), None), 250_000);
    let p = l
        .reimburse_claim(&admin(), &c.id, &reimbursement())
        .unwrap();
    assert_eq!(p.amount, 250_000);
    let c = l.state().claim(&c.id).unwrap().clone();
    assert_eq!(c.status, ClaimStatus::Reimbursed);
    assert_eq!(
        c.approvals.last().unwrap().note.as_deref(),
        Some("QAB12XYZ")
    );
    assert_eq!(
        l.state()
            .outstanding(&AppliesTo::ExpenseClaim(c.id.clone()))
            .unwrap(),
        0
    );
    assert_eq!(base(&state, "acc-2110", None), 0);
}
