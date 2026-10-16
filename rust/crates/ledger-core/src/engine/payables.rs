//! Accounts payable: vendors, bills and bill payments (`engine.ts:806-903`).

use serde::Deserialize;

use super::documents::NewCodedLine;
use super::{Error, Ledger};
use crate::types::{
    AccountId, Actor, Bill, BillId, FlowNodeId, IsoDate, JournalEntry, Nullish, Payment, Vendor,
    VendorId,
};

/// The input of `addVendor` (`engine.ts:810`): a [`Vendor`] with `id` optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewVendor {
    /// Id; drawn from the env when absent.
    #[serde(default)]
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewBill {
    /// Id; drawn from the env when absent.
    #[serde(default)]
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

impl Ledger<'_> {
    /// `addVendor` (`engine.ts:810-816`): adds a vendor. Needs `can_post`; the name cannot be
    /// blank (`INVALID_INPUT`). No uniqueness check and no audit event, as in TS; draws the id
    /// only when none is given.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn add_vendor(&mut self, actor: &Actor, input: NewVendor) -> Result<Vendor, Error> {
        let _ = (actor, input);
        todo!()
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
        let _ = (actor, input);
        todo!()
    }

    /// `approveBill` (`engine.ts:847-857`): approves a draft bill. Needs `can_approve`; the bill
    /// must exist (`NOT_FOUND`) and be a draft (`INVALID_STATE`); its creator cannot approve it
    /// (`SELF_APPROVAL`). One audit id (`bill.approve`, detail the number).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above.
    pub fn approve_bill(&mut self, actor: &Actor, id: &BillId) -> Result<Bill, Error> {
        let _ = (actor, id);
        todo!()
    }

    /// `postBill` (`engine.ts:859-875`): posts an approved bill as an accrual (Dr expense per
    /// line, Cr accounts payable per fund) dated the bill date, memo `Bill <number> · <vendor>`,
    /// source the bill. The bill becomes posted with the entry's id. Draws the ids of
    /// [`Ledger::post`].
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
        let _ = (actor, id, override_reason);
        todo!()
    }

    /// `payBill` (`engine.ts:888-903`): pays a posted bill, in part or in full; a bill with
    /// nothing left owed becomes paid.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `INVALID_STATE` unless posted; then those of the payment.
    pub fn pay_bill(
        &mut self,
        actor: &Actor,
        id: &BillId,
        input: &PaymentInput,
    ) -> Result<Payment, Error> {
        let _ = (actor, id, input);
        todo!()
    }
}
