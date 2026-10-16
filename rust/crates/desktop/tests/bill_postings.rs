//! "Postings correct" (U6): bills driven through the in-process backend on the demo books, the
//! way the Spend screen drives them, and checked through the bill detail's view model. Post
//! and Pay on the approved demo bill (Staples Kenya, two funds) leave a balanced accrual,
//! move accounts payable by the bill total, and each payment reduces the payable and the bank
//! account by its amount, split across the funds as the engine split it
//! (`flowtrace/src/lib/ledger/engine.ts:1083-1121`, the JS `Math.round` share).

#![expect(clippy::unwrap_used, reason = "tests may unwrap")]

use desktop::accounts::Failure;
use desktop::backend::{InProcessBackend, LedgerBackend};
use desktop::bills::{BillAction, BillDetail, NewBillForm, PayForm, bill_detail, next_step};
use ledger_core::LedgerCommand;
use ledger_core::coa_template::account_id;
use ledger_core::demo::{DEMO_AS_OF, DEMO_ORG_ID};
use ledger_core::money::js_round;
use ledger_core::types::{BillStatus, IsoDate, LedgerState, OrganizationId};

fn as_of() -> IsoDate {
    IsoDate::parse(DEMO_AS_OF).unwrap()
}

fn org() -> OrganizationId {
    OrganizationId::from(DEMO_ORG_ID)
}

fn detail(backend: &InProcessBackend, number: &str) -> (LedgerState, BillDetail) {
    let state = LedgerState::clone(&backend.state(&org()).unwrap());
    let bill = state.bills.iter().find(|b| b.number == number).unwrap();
    let actor = backend.actor(&org()).unwrap();
    let detail = bill_detail(&state, &bill.id, Some(&actor), &as_of()).unwrap();
    (state, detail)
}

/// Net base-currency balance (debits less credits) of `account` over every posted line.
fn balance(state: &LedgerState, account: &str) -> i64 {
    let id = account_id(account);
    state
        .lines
        .iter()
        .filter(|l| l.account_id == id)
        .filter(|l| {
            state
                .entry(&l.entry_id)
                .is_some_and(|e| e.status == ledger_core::types::EntryStatus::Posted)
        })
        .map(|l| l.base_amount)
        .sum()
}

#[test]
fn post_and_pay_the_approved_demo_bill() {
    let mut backend = InProcessBackend::with_demo_and_empty().unwrap();
    let (before, staples) = detail(&backend, "SK-INV-30418");
    let payable = before.settings.system_accounts.accounts_payable.clone();
    let payable_code = before.account(&payable).unwrap().code.clone();
    assert_eq!(staples.bill.status, BillStatus::Approved);
    assert_eq!(staples.actions, [BillAction::Post]);
    let total = staples.total;
    assert_eq!(total, 20_050_000);

    // Post.
    let post = BillAction::Post.command(&staples.bill.id).unwrap();
    backend.submit(&org(), post).unwrap();
    let (posted, d) = detail(&backend, "SK-INV-30418");
    assert_eq!(d.bill.status, BillStatus::Posted);
    assert_eq!(d.outstanding, Some(total));
    assert_eq!(d.actions, [BillAction::Pay]);
    let accrual = d.posting.as_ref().unwrap();
    assert!(accrual.balanced(), "{accrual:?}");
    assert_eq!(accrual.movement(&payable), -total);
    assert_eq!(
        balance(&posted, &payable_code) - balance(&before, &payable_code),
        -total,
        "the payable moved by the bill total"
    );
    assert!(d.trail.iter().any(|t| t.what == format!("Posted as {}", accrual.detail.number)));

    // A part payment from the bank.
    let mut pay = PayForm::new(d.outstanding, &as_of());
    assert_eq!(pay.amount, "200500.00");
    pay.amount = "100,000".into();
    pay.cash_account = Some(account_id("1120"));
    pay.reference = "EFT 9001".into();
    let command = pay.command(&d.bill.id, total).unwrap();
    backend.submit(&org(), command).unwrap();
    let (paid_part, d) = detail(&backend, "SK-INV-30418");
    let amount = 10_000_000;
    assert_eq!(d.bill.status, BillStatus::Posted, "a part payment leaves it posted");
    assert_eq!(d.outstanding, Some(total - amount));
    assert_eq!(d.paid, amount);
    assert_eq!(d.payments.len(), 1);
    let p = &d.payments[0];
    let entry = p.entry.as_ref().unwrap();
    assert!(entry.balanced(), "{entry:?}");
    assert_eq!(entry.movement(&payable), amount);
    assert_eq!(entry.movement(&account_id("1120")), -amount);
    assert_eq!(
        balance(&paid_part, &payable_code) - balance(&posted, &payable_code),
        amount
    );
    assert_eq!(
        balance(&paid_part, "1120") - balance(&posted, "1120"),
        -amount
    );

    // The split the engine made: the education fund first, its share rounded as JS rounds it,
    // the unrestricted fund the rest.
    let split: Vec<_> = p.fund_split.iter().map(|(f, _, m)| (f.as_str(), *m)).collect();
    let education = 18_600_000_f64;
    #[expect(clippy::cast_possible_truncation, reason = "a whole share below the amount")]
    let first = js_round(1e7 * education / 20_050_000.0) as i64;
    assert_eq!(first, 9_276_808);
    assert_eq!(split.len(), 2);
    assert_eq!(split[0].1, first);
    assert_eq!(split[1].1, amount - first);
    assert_eq!(split.iter().map(|s| s.1).sum::<i64>(), amount);

    // The rest: the bill becomes paid.
    let mut rest = PayForm::new(d.outstanding, &as_of());
    assert_eq!(rest.amount, "100500.00");
    rest.cash_account = Some(account_id("1120"));
    let command = rest.command(&d.bill.id, total - amount).unwrap();
    backend.submit(&org(), command).unwrap();
    let (paid, d) = detail(&backend, "SK-INV-30418");
    assert_eq!(d.bill.status, BillStatus::Paid);
    assert_eq!(d.outstanding, Some(0));
    assert!(d.actions.is_empty());
    assert_eq!(d.payments.len(), 2);
    let cleared: i64 = d
        .payments
        .iter()
        .flat_map(|p| p.fund_split.iter().map(|s| s.2))
        .sum();
    assert_eq!(cleared, total);
    assert_eq!(
        balance(&paid, &payable_code) - balance(&before, &payable_code),
        0,
        "posted then paid in full: the payable is back where it was"
    );
    assert_eq!(balance(&paid, "1120") - balance(&before, "1120"), -total);
}

#[test]
fn a_bill_entered_here_waits_for_another_approver() {
    let mut backend = InProcessBackend::with_demo_and_empty().unwrap();
    let state = LedgerState::clone(&backend.state(&org()).unwrap());
    let vendor = state.vendors.iter().find(|v| v.name == "Kenya Power").unwrap();
    let mut form = NewBillForm::new(&state, &as_of());
    form.vendor = Some(vendor.id.clone());
    form.number = "KPLC-0326-1".into();
    form.lines[0].description = "Electricity, February".into();
    form.lines[0].amount = "41,200".into();
    form.lines[0].account_id = Some(account_id("7300"));
    let command = form.command(&state).unwrap();
    backend.submit(&org(), command).unwrap();

    let (_, d) = detail(&backend, "KPLC-0326-1");
    assert_eq!(d.bill.status, BillStatus::Draft);
    assert_eq!(d.total, 4_120_000);
    assert_eq!(d.outstanding, None);
    // The demo user entered it, so the demo user cannot approve it.
    assert!(d.actions.is_empty());
    assert!(d.next.starts_with("You entered this bill"), "{}", d.next);
    // The engine agrees, with its own message.
    let refused = backend
        .submit(&org(), LedgerCommand::BillApprove {
            bill_id: d.bill.id.clone(),
        })
        .unwrap_err();
    assert_eq!(
        Failure::new("Not done", &refused).messages,
        ["You cannot approve a bill you entered."]
    );
    let actor = backend.actor(&org()).unwrap();
    assert_eq!(next_step(&d.bill, Some(&actor), None).1, []);
}

#[test]
fn a_new_vendor_is_listed() {
    let mut backend = InProcessBackend::with_demo_and_empty().unwrap();
    let before = desktop::bills::vendor_rows(&backend.state(&org()).unwrap()).len();
    let form = desktop::bills::NewVendorForm {
        name: "Nairobi Printers".into(),
        payment_details: "Paybill 400200".into(),
    };
    backend.submit(&org(), form.command().unwrap()).unwrap();
    let rows = desktop::bills::vendor_rows(&backend.state(&org()).unwrap());
    assert_eq!(rows.len(), before + 1);
    let added = rows.iter().find(|r| r.name == "Nairobi Printers").unwrap();
    assert_eq!(added.payment_details.as_deref(), Some("Paybill 400200"));
    assert_eq!((added.bills, added.outstanding), (0, 0));
}
