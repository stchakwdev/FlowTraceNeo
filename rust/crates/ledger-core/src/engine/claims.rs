//! Expense claims, staff spend paid back (`engine.ts:905-1004`).

use serde::Deserialize;

use super::documents::NewCodedLine;
use super::{Error, Ledger};
use crate::types::{AccountId, Actor, ClaimId, ExpenseClaim, IsoDate, Nullish, Payment};

/// The input of `createClaim` (`engine.ts:909-916`): an [`ExpenseClaim`] without its number,
/// status, trail, entry and claimant, with `id` optional, the lines not yet given ids, and
/// `submit` to submit it at once.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewClaim {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<ClaimId>,
    /// Claim date; the claim posts on it.
    pub claim_date: IsoDate,
    /// Claim currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    pub fx_rate: f64,
    /// What the money was spent on; it cannot be blank, and is stored as given.
    pub purpose: String,
    /// Coded lines; at least one.
    pub lines: Vec<NewCodedLine>,
    /// Receipt file names.
    pub receipt_names: Vec<String>,
    /// Set when captured offline and synced later.
    pub captured_offline: bool,
    /// Submit the claim as soon as it is created.
    #[serde(default)]
    pub submit: Option<bool>,
}

/// The input of `reimburseClaim` (`engine.ts:989`): the amount is whatever is still owed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReimburseInput {
    /// Payment date, as the entry date; the posting checks it.
    pub date: String,
    /// The bank, mobile money or cash account the money left.
    pub cash_account_id: AccountId,
    /// Bank or mobile-money reference.
    #[serde(default)]
    pub reference: Nullish<String>,
}

impl Ledger<'_> {
    /// `createClaim` (`engine.ts:909-937`): records a claim as a draft, or submits it with
    /// `submit`. Any member can claim; no permission is needed.
    ///
    /// # Errors
    ///
    /// The coded-line errors; `INVALID_INPUT` for a blank purpose; `DUPLICATE` for a used id.
    pub fn create_claim(&mut self, actor: &Actor, input: NewClaim) -> Result<ExpenseClaim, Error> {
        let _ = (actor, input);
        todo!()
    }

    /// `submitClaim` (`engine.ts:939-947`): the claimant submits a draft claim.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `PERMISSION` for anyone but the claimant; `INVALID_STATE` unless a draft.
    pub fn submit_claim(&mut self, actor: &Actor, id: &ClaimId) -> Result<ExpenseClaim, Error> {
        let _ = (actor, id);
        todo!()
    }

    /// `approveClaim` (`engine.ts:949-973`): approval posts the claim (Dr expense, Cr staff
    /// claims payable).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` unless submitted; `SELF_APPROVAL`; then those
    /// of [`Ledger::post`].
    pub fn approve_claim(
        &mut self,
        actor: &Actor,
        id: &ClaimId,
        note: Option<&str>,
        override_reason: Option<&str>,
    ) -> Result<ExpenseClaim, Error> {
        let _ = (actor, id, note, override_reason);
        todo!()
    }

    /// `rejectClaim` (`engine.ts:975-984`): rejects a submitted claim with a note.
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` unless submitted; `INVALID_INPUT` for a blank
    /// note.
    pub fn reject_claim(
        &mut self,
        actor: &Actor,
        id: &ClaimId,
        note: &str,
    ) -> Result<ExpenseClaim, Error> {
        let _ = (actor, id, note);
        todo!()
    }

    /// `reimburseClaim` (`engine.ts:986-1004`): pays a posted claim back in full.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `INVALID_STATE` unless posted; `SELF_APPROVAL` for the claimant; then those
    /// of the payment.
    pub fn reimburse_claim(
        &mut self,
        actor: &Actor,
        id: &ClaimId,
        input: &ReimburseInput,
    ) -> Result<Payment, Error> {
        let _ = (actor, id, input);
        todo!()
    }
}
