//! Expense claims, staff spend paid back (`engine.ts:905-1004`).

use serde::{Deserialize, Serialize};

use super::documents::{NewCodedLine, PayInput, Settled};
use super::queries::js_number;
use super::setup::truthy;
use super::{EntryOptions, Error, Ledger, Permission, fail, has_text, js_trim, require};
use crate::errors::IssueCode;
use crate::types::{
    AccountId, Actor, AppliesTo, ApprovalAction, ApprovalStep, ClaimId, ClaimStatus, EntryInput,
    ExpenseClaim, IsoDate, LedgerState, Nullish, Payment, SourceType,
};

/// The input of `createClaim` (`engine.ts:909-916`): an [`ExpenseClaim`] without its number,
/// status, trail, entry and claimant, with `id` optional, the lines not yet given ids, and
/// `submit` to submit it at once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewClaim {
    /// Id; drawn from the env when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ClaimId>,
    /// Claim date; the claim posts on it.
    pub claim_date: IsoDate,
    /// Claim currency.
    pub currency: String,
    /// Units of base currency per unit of `currency`.
    #[serde(serialize_with = "crate::js_number::serialize")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// The TS literal of a claim status, for messages.
fn status_str(s: ClaimStatus) -> &'static str {
    match s {
        ClaimStatus::Draft => "draft",
        ClaimStatus::Submitted => "submitted",
        ClaimStatus::Approved => "approved",
        ClaimStatus::Rejected => "rejected",
        ClaimStatus::Posted => "posted",
        ClaimStatus::Reimbursed => "reimbursed",
    }
}

impl LedgerState {
    /// The index of a claim, for `claim(id)` before a change (`engine.ts:905-907`).
    fn claim_index(&self, id: &ClaimId) -> Result<usize, Error> {
        match self.claims.iter().position(|c| &c.id == id) {
            Some(i) => Ok(i),
            None => fail(IssueCode::NotFound, "Claim not found."),
        }
    }

    /// The claim at `index` if its status is `status`, else `INVALID_STATE` "Claim EC-0001 is
    /// draft." (`engine.ts:942`, `engine.ts:956`, `engine.ts:977`).
    fn claim_in(&self, index: usize, status: ClaimStatus) -> Result<&ExpenseClaim, Error> {
        let c = &self.claims[index];
        if c.status == status {
            Ok(c)
        } else {
            fail(
                IssueCode::InvalidState,
                format!("Claim {} is {}.", c.number, status_str(c.status)),
            )
        }
    }
}

impl Ledger<'_> {
    /// One step on the claim's approval trail, at the env's now.
    fn approval(
        &self,
        actor: &Actor,
        action: ApprovalAction,
        note: Option<String>,
    ) -> ApprovalStep {
        ApprovalStep {
            at: self.env.now(),
            by: actor.id.clone(),
            action,
            note,
        }
    }

    /// `submitClaim` after its checks (`engine.ts:943-946`): submitted, with a `submitted` step
    /// and one audit id (`claim.submit`, detail the number).
    fn store_submitted(&mut self, actor: &Actor, index: usize) -> ExpenseClaim {
        let step = self.approval(actor, ApprovalAction::Submitted, None);
        let c = &mut self.state.claims[index];
        c.status = ClaimStatus::Submitted;
        c.approvals.push(step);
        let claim = c.clone();
        self.audit(actor, "claim.submit", claim.id.as_str(), &claim.number);
        claim
    }

    /// `createClaim` (`engine.ts:909-937`): records a claim by the actor as a draft, or submits
    /// it at once with `submit`. Any member can claim; no permission is needed. The lines pass
    /// the coded-line checks; the purpose cannot be blank (`INVALID_INPUT`, stored as given); a
    /// given id must be unused (`DUPLICATE`). Draws the claim id (when none is given), takes the
    /// next `EC` number, draws one id per line, and with `submit` the ids of
    /// [`Ledger::submit_claim`]. Returns the claim as stored, submitted or not.
    ///
    /// # Errors
    ///
    /// The coded-line errors of [`Ledger::create_bill`]; `INVALID_INPUT`; `DUPLICATE`.
    pub fn create_claim(&mut self, actor: &Actor, input: NewClaim) -> Result<ExpenseClaim, Error> {
        let amounts = self.state.check_coded_lines(&input.lines)?;
        if !has_text(&input.purpose) {
            return fail(IssueCode::InvalidInput, "Say what the money was spent on.");
        }
        if truthy(input.id.as_ref()).is_some_and(|id| self.state.claims.iter().any(|c| &c.id == id))
        {
            return fail(IssueCode::Duplicate, "A claim with this id already exists.");
        }
        let id = match input.id {
            Some(id) => id,
            None => ClaimId::from(self.env.new_id()),
        };
        let number = self.next_number("EC");
        let lines = self.coded_lines(input.lines, amounts);
        let claim = ExpenseClaim {
            id,
            number,
            claimant: actor.id.clone(),
            claimant_name: actor.name.clone(),
            claim_date: input.claim_date,
            currency: input.currency,
            fx_rate: input.fx_rate,
            purpose: input.purpose,
            lines,
            status: ClaimStatus::Draft,
            approvals: Vec::new(),
            receipt_names: input.receipt_names,
            entry_id: None,
            captured_offline: input.captured_offline,
        };
        self.state.claims.push(claim.clone());
        if input.submit == Some(true) {
            // The claimant submits their own draft, so submitClaim's checks hold.
            return Ok(self.store_submitted(actor, self.state.claims.len() - 1));
        }
        Ok(claim)
    }

    /// `submitClaim` (`engine.ts:939-947`): the claimant submits a draft claim; no other
    /// permission is needed. Adds a `submitted` step to the trail and draws one audit id
    /// (`claim.submit`, detail the number).
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `PERMISSION` for anyone but the claimant; `INVALID_STATE` unless a draft.
    pub fn submit_claim(&mut self, actor: &Actor, id: &ClaimId) -> Result<ExpenseClaim, Error> {
        let index = self.state.claim_index(id)?;
        if self.state.claims[index].claimant != actor.id {
            return fail(
                IssueCode::Permission,
                "Only the claimant can submit a claim.",
            );
        }
        self.state.claim_in(index, ClaimStatus::Draft)?;
        Ok(self.store_submitted(actor, index))
    }

    /// `approveClaim` (`engine.ts:949-973`): approval posts the claim, so budget against actual
    /// moves the moment a manager approves. Needs `can_approve`; claimants cannot approve their
    /// own claims. The accrual (Dr expense per line, Cr staff claims payable per fund, any
    /// rounding to exchange differences) is dated the claim date, memo
    /// `Expense claim <number> · <claimant name> · <purpose>`, source the claim; the override
    /// reason works as for [`Ledger::post`], whose checks apply. The claim becomes posted with
    /// the entry's id and gets two steps, `approved` (the note trimmed, null when blank) and
    /// `posted` (note the entry number). Draws the ids of [`Ledger::post`], then one audit id
    /// (`claim.approve`, detail the number).
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
        require(actor, Permission::Approve)?;
        let index = self.state.claim_index(id)?;
        let c = self.state.claim_in(index, ClaimStatus::Submitted)?;
        if c.claimant == actor.id {
            return fail(
                IssueCode::SelfApproval,
                "You cannot approve your own claim.",
            );
        }
        let input = EntryInput {
            date: c.claim_date.as_str().to_owned(),
            memo: format!(
                "Expense claim {} \u{b7} {} \u{b7} {}",
                c.number, c.claimant_name, c.purpose
            ),
            source_type: Some(SourceType::ExpenseClaim),
            source_id: Nullish::Value(c.id.as_str().to_owned()),
            override_reason: override_reason.map(str::to_owned).into(),
            lines: self.state.accrual_lines(
                &c.lines,
                &c.currency,
                c.fx_rate,
                &self.state.settings.system_accounts.claims_payable,
                None,
            )?,
        };
        let prepared = self.check_post(actor, &input, EntryOptions::default())?;
        let entry = self.store_posted(actor, &input, prepared);
        let note = note
            .map(js_trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned);
        let approved = self.approval(actor, ApprovalAction::Approved, note);
        let posted = self.approval(actor, ApprovalAction::Posted, Some(entry.number.clone()));
        let c = &mut self.state.claims[index];
        c.entry_id = Some(entry.id);
        c.status = ClaimStatus::Posted;
        c.approvals.extend([approved, posted]);
        let claim = c.clone();
        self.audit(actor, "claim.approve", claim.id.as_str(), &claim.number);
        Ok(claim)
    }

    /// `rejectClaim` (`engine.ts:975-984`): rejects a submitted claim with a note. Needs
    /// `can_approve`. Adds a `rejected` step with the note trimmed and draws one audit id
    /// (`claim.reject`, detail the note as given).
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
        require(actor, Permission::Approve)?;
        let index = self.state.claim_index(id)?;
        self.state.claim_in(index, ClaimStatus::Submitted)?;
        if !has_text(note) {
            return fail(
                IssueCode::InvalidInput,
                "Tell the claimant why the claim is rejected.",
            );
        }
        let step = self.approval(
            actor,
            ApprovalAction::Rejected,
            Some(js_trim(note).to_owned()),
        );
        let c = &mut self.state.claims[index];
        c.status = ClaimStatus::Rejected;
        c.approvals.push(step);
        let claim = c.clone();
        self.audit(actor, "claim.reject", claim.id.as_str(), note);
        Ok(claim)
    }

    /// `reimburseClaim` (`engine.ts:986-1004`): pays a posted claim back in full, whatever is
    /// still owed, from a cash account, clearing staff claims payable fund by fund (memo
    /// `Reimbursement of <number> · <claimant name>`, then ` · <reference>` when one is given),
    /// at the claim's rate. The claimant cannot pay out their own claim. The claim becomes
    /// reimbursed with a `reimbursed` step (note the reference as given). Draws the ids of
    /// [`Ledger::post`], then the payment id; no audit event.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND`; `INVALID_STATE` unless posted; `SELF_APPROVAL` for the claimant; then those
    /// of the payment ([`Ledger::pay_bill`]).
    pub fn reimburse_claim(
        &mut self,
        actor: &Actor,
        id: &ClaimId,
        input: &ReimburseInput,
    ) -> Result<Payment, Error> {
        let index = self.state.claim_index(id)?;
        let c = &self.state.claims[index];
        if c.status != ClaimStatus::Posted {
            return fail(
                IssueCode::InvalidState,
                format!(
                    "Claim {} must be approved before it is reimbursed.",
                    c.number
                ),
            );
        }
        if c.claimant == actor.id {
            return fail(
                IssueCode::SelfApproval,
                "You cannot pay out your own claim.",
            );
        }
        let coded = c.lines.clone();
        let currency = c.currency.clone();
        let payable = self.state.settings.system_accounts.claims_payable.clone();
        let doc = Settled {
            doc: AppliesTo::ExpenseClaim(id.clone()),
            coded: &coded,
            currency: &currency,
            rate: c.fx_rate,
            payable: &payable,
            memo: format!("Reimbursement of {} \u{b7} {}", c.number, c.claimant_name),
        };
        let owed = self.state.outstanding(&doc.doc)?;
        let reference = input.reference.value().map(String::as_str);
        let payment = self.pay(
            actor,
            &doc,
            &PayInput {
                date: &input.date,
                cash_account_id: &input.cash_account_id,
                amount: js_number(owed),
                reference,
                fx_rate: None,
            },
        )?;
        let step = self.approval(
            actor,
            ApprovalAction::Reimbursed,
            reference.map(str::to_owned),
        );
        let c = &mut self.state.claims[index];
        c.status = ClaimStatus::Reimbursed;
        c.approvals.push(step);
        Ok(payment)
    }
}
