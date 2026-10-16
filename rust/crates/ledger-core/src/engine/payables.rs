//! Accounts payable: vendors, bills and bill payments (`engine.ts:806-903`).

use serde::{Deserialize, Serialize};

use super::documents::{NewCodedLine, PayInput, Settled};
use super::{EntryOptions, Error, Ledger, Permission, fail, has_text, require};
use crate::errors::IssueCode;
use crate::types::{
    AccountId, Actor, AppliesTo, Bill, BillId, BillStatus, EntryInput, FlowNodeId, IsoDate,
    JournalEntry, LedgerState, Nullish, Payment, SourceType, Vendor, VendorId,
};

/// The input of `addVendor` (`engine.ts:810`): a [`Vendor`] with `id` optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewVendor {
    /// Id; drawn from the env when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<VendorId>,
    /// Display name; it cannot be blank, and is stored as given.
    pub name: String,
    /// The flow node this vendor stands for, if any.
    pub node_id: Option<FlowNodeId>,
    /// Free-text payment details.
    pub payment_details: Option<String>,
}

/// The input of `createBill` (`engine.ts:822-828`): a [`Bill`] without its status, entry,
/// approver and creator, with `id` optional and the lines not yet given ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewBill {
    /// Id; drawn from the env when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<BillId>,
    /// Vendor billed by; it must exist.
    pub vendor_id: VendorId,
    /// The vendor's bill number, stored as given.
    pub number: String,
    /// Bill date; the bill posts on it.
    pub bill_date: IsoDate,
    /// Due date.
    pub due_date: IsoDate,
    /// Bill currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(serialize_with = "crate::js_number::serialize")]
    pub fx_rate: f64,
    /// Coded lines; at least one.
    pub lines: Vec<NewCodedLine>,
    /// Proof references.
    pub proof_ids: Vec<String>,
}

/// The input of `payBill` (`engine.ts:891`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PaymentInput {
    /// Payment date, as the entry date; the posting checks it.
    pub date: String,
    /// The bank, mobile money or cash account the money left.
    pub cash_account_id: AccountId,
    /// Amount in the bill's currency: whole minor units, more than zero, at most what is owed.
    pub amount: f64,
    /// Bank or mobile-money reference.
    #[serde(default)]
    pub reference: Nullish<String>,
    /// Today's rate for a foreign bill; the bill's rate when absent.
    #[serde(default)]
    pub fx_rate: Option<f64>,
}

/// The TS literal of a bill status, for messages.
fn status_str(s: BillStatus) -> &'static str {
    match s {
        BillStatus::Draft => "draft",
        BillStatus::Approved => "approved",
        BillStatus::Posted => "posted",
        BillStatus::Paid => "paid",
        BillStatus::Void => "void",
    }
}

impl LedgerState {
    /// The index of a bill, for `bill(id)` before a change (`engine.ts:818-820`).
    fn bill_index(&self, id: &BillId) -> Result<usize, Error> {
        match self.bills.iter().position(|b| &b.id == id) {
            Some(i) => Ok(i),
            None => fail(IssueCode::NotFound, "Bill not found."),
        }
    }
}

impl Ledger<'_> {
    /// `addVendor` (`engine.ts:810-816`): adds a vendor. Needs `can_post`; the name cannot be
    /// blank (`INVALID_INPUT`). No uniqueness check and no audit event, as in TS; draws the id
    /// only when none is given.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn add_vendor(&mut self, actor: &Actor, input: NewVendor) -> Result<Vendor, Error> {
        require(actor, Permission::Post)?;
        if !has_text(&input.name) {
            return fail(IssueCode::InvalidInput, "A vendor needs a name.");
        }
        let id = match input.id {
            Some(id) => id,
            None => VendorId::from(self.env.new_id()),
        };
        let v = Vendor {
            id,
            name: input.name,
            node_id: input.node_id,
            payment_details: input.payment_details,
        };
        self.state.vendors.push(v.clone());
        Ok(v)
    }

    /// `createBill` (`engine.ts:822-845`): enters a draft bill. Needs `can_post`; the vendor
    /// must exist (`NOT_FOUND`) and the lines pass the coded-line checks. No uniqueness check on
    /// the id, as in TS. Draws the bill id (when none is given), one id per line, then one audit
    /// id (`bill.create`, detail the bill number). The creator is the actor.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above and those of the coded lines (`TOO_FEW_LINES`,
    /// `NEGATIVE_AMOUNT`, `UNKNOWN_ACCOUNT`, `UNKNOWN_FUND`); [`Error::Money`] for a fractional
    /// line amount.
    pub fn create_bill(&mut self, actor: &Actor, input: NewBill) -> Result<Bill, Error> {
        require(actor, Permission::Post)?;
        if !self.state.vendors.iter().any(|v| v.id == input.vendor_id) {
            return fail(IssueCode::NotFound, "Vendor not found.");
        }
        let amounts = self.state.check_coded_lines(&input.lines)?;
        let id = match input.id {
            Some(id) => id,
            None => BillId::from(self.env.new_id()),
        };
        let lines = self.coded_lines(input.lines, amounts);
        let bill = Bill {
            id,
            vendor_id: input.vendor_id,
            number: input.number,
            bill_date: input.bill_date,
            due_date: input.due_date,
            currency: input.currency,
            fx_rate: input.fx_rate,
            status: BillStatus::Draft,
            lines,
            entry_id: None,
            proof_ids: input.proof_ids,
            created_by: actor.id.clone(),
            approved_by: None,
        };
        self.state.bills.push(bill.clone());
        self.audit(actor, "bill.create", bill.id.as_str(), &bill.number);
        Ok(bill)
    }

    /// `approveBill` (`engine.ts:847-857`): approves a draft bill. Needs `can_approve`; the bill
    /// must exist (`NOT_FOUND`) and be a draft (`INVALID_STATE`); its creator cannot approve it
    /// (`SELF_APPROVAL`). One audit id (`bill.approve`, detail the number).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn approve_bill(&mut self, actor: &Actor, id: &BillId) -> Result<Bill, Error> {
        require(actor, Permission::Approve)?;
        let index = self.state.bill_index(id)?;
        let b = &self.state.bills[index];
        if b.status != BillStatus::Draft {
            return fail(
                IssueCode::InvalidState,
                format!("Bill {} is {}.", b.number, status_str(b.status)),
            );
        }
        if b.created_by == actor.id {
            return fail(
                IssueCode::SelfApproval,
                "You cannot approve a bill you entered.",
            );
        }
        let b = &mut self.state.bills[index];
        b.status = BillStatus::Approved;
        b.approved_by = Some(actor.id.clone());
        let bill = b.clone();
        self.audit(actor, "bill.approve", bill.id.as_str(), &bill.number);
        Ok(bill)
    }

    /// `postBill` (`engine.ts:859-875`): posts an approved bill as an accrual (Dr expense per
    /// line, Cr accounts payable per fund, any rounding to exchange differences), dated the bill
    /// date, memo `Bill <number> · <vendor name>` (`Vendor` when the vendor is gone), source the
    /// bill, the expense and payable lines tagged with the vendor's flow node. The override reason lets grant date
    /// and award issues through, as for [`Ledger::post`], whose permission and checks apply. The
    /// bill becomes posted with the entry's id. Draws the ids of [`Ledger::post`]; no audit
    /// event of its own.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `INVALID_STATE` unless approved; then those of [`Ledger::post`].
    pub fn post_bill(
        &mut self,
        actor: &Actor,
        id: &BillId,
        override_reason: Option<&str>,
    ) -> Result<JournalEntry, Error> {
        let index = self.state.bill_index(id)?;
        let b = &self.state.bills[index];
        if b.status != BillStatus::Approved {
            return fail(
                IssueCode::InvalidState,
                format!("Approve bill {} before posting it.", b.number),
            );
        }
        let vendor = self.state.vendors.iter().find(|v| v.id == b.vendor_id);
        let node_id: Option<&FlowNodeId> = vendor.and_then(|v| v.node_id.as_ref());
        let input = EntryInput {
            date: b.bill_date.as_str().to_owned(),
            memo: format!(
                "Bill {} \u{b7} {}",
                b.number,
                vendor.map_or("Vendor", |v| v.name.as_str())
            ),
            source_type: Some(SourceType::Bill),
            source_id: Nullish::Value(b.id.as_str().to_owned()),
            override_reason: override_reason.map(str::to_owned).into(),
            lines: self.state.accrual_lines(
                &b.lines,
                &b.currency,
                b.fx_rate,
                &self.state.settings.system_accounts.accounts_payable,
                node_id,
            )?,
        };
        let prepared = self.check_post(actor, &input, EntryOptions::default())?;
        let entry = self.store_posted(actor, &input, prepared);
        let b = &mut self.state.bills[index];
        b.status = BillStatus::Posted;
        b.entry_id = Some(entry.id.clone());
        Ok(entry)
    }

    /// `payBill` (`engine.ts:888-903`): pays a posted bill, in part or in full, from a cash
    /// account, clearing accounts payable fund by fund (memo `Payment of bill <number>`, then
    /// ` · <reference>` when one is given). A bill with nothing left owed becomes paid. Draws
    /// the ids of [`Ledger::post`], then the payment id; no audit event.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `INVALID_STATE` unless posted; then `UNKNOWN_ACCOUNT` for a non-cash
    /// account, [`Error::Money`] for a fractional amount, `NEGATIVE_AMOUNT`, `INVALID_INPUT` for
    /// more than is owed, and those of [`Ledger::post`].
    pub fn pay_bill(
        &mut self,
        actor: &Actor,
        id: &BillId,
        input: &PaymentInput,
    ) -> Result<Payment, Error> {
        let index = self.state.bill_index(id)?;
        let b = &self.state.bills[index];
        if b.status != BillStatus::Posted {
            return fail(
                IssueCode::InvalidState,
                format!("Bill {} must be posted before it is paid.", b.number),
            );
        }
        let coded = b.lines.clone();
        let currency = b.currency.clone();
        let payable = self.state.settings.system_accounts.accounts_payable.clone();
        let doc = Settled {
            doc: AppliesTo::Bill(id.clone()),
            coded: &coded,
            currency: &currency,
            rate: b.fx_rate,
            payable: &payable,
            memo: format!("Payment of bill {}", b.number),
        };
        let payment = self.pay(
            actor,
            &doc,
            &PayInput {
                date: &input.date,
                cash_account_id: &input.cash_account_id,
                amount: input.amount,
                reference: input.reference.value().map(String::as_str),
                fx_rate: input.fx_rate,
            },
        )?;
        if self.state.outstanding(&doc.doc).is_ok_and(|owed| owed == 0) {
            self.state.bills[index].status = BillStatus::Paid;
        }
        Ok(payment)
    }
}
