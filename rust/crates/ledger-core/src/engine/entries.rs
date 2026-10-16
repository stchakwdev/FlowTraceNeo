//! Journal entries: drafts, posting and reversal (`engine.ts:470-662`).

use super::validate::{blocking, override_of};
use super::{EntryOptions, Error, Ledger, Permission, fail, has_text, js_trim, require};
use crate::errors::{IssueCode, LedgerError};
use crate::money::{AMOUNT, Minor, assert_minor, convert};
use crate::types::{
    AccountId, Actor, AppliesTo, ApprovalAction, ApprovalStep, BillId, BillStatus, ClaimId,
    ClaimStatus, DimensionId, EntryId, EntryInput, EntryStatus, FlowNodeId, FundId, GrantId,
    IsoDate, JournalEntry, JournalLine, LedgerState, LineId, LineInput, Nullish, PeriodId,
    SourceType,
};

/// A journal line before its id is drawn (`buildLines`, `engine.ts:474-497`).
struct LineValues {
    account_id: AccountId,
    debit: Minor,
    credit: Minor,
    currency: String,
    fx_rate: f64,
    base_amount: Minor,
    fund_id: FundId,
    grant_id: Option<GrantId>,
    dimension_ids: Vec<DimensionId>,
    flow_node_id: Option<FlowNodeId>,
    memo: Option<String>,
}

/// An entry checked and ready to store: its date and its lines.
struct Prepared {
    date: IsoDate,
    lines: Vec<LineValues>,
}

/// What reversing a document's entry does to the document (`engine.ts:617-644`).
enum OnReversed {
    /// A manual or other entry: nothing.
    Nothing,
    /// A bill's entry: the bill becomes void.
    VoidBill(BillId),
    /// A claim's entry: the claim is rejected, with a note on its trail.
    RejectClaim(ClaimId),
    /// A bill payment: a paid bill is owed again.
    ReopenBill(BillId),
    /// A claim reimbursement: a reimbursed claim is owed again.
    ReopenClaim(ClaimId),
}

/// The TS literal of an entry status, for messages.
fn status_str(s: EntryStatus) -> &'static str {
    match s {
        EntryStatus::Draft => "draft",
        EntryStatus::Posted => "posted",
        EntryStatus::Reversed => "reversed",
    }
}

impl LedgerState {
    /// The stored values of each input line (`buildLines`, `engine.ts:474-497`): currency
    /// defaults to base with rate 1, missing amounts are 0, and the base amount is
    /// `convert(debit - credit, rate)`, frozen here.
    ///
    /// Drafts are not validated, so this is where a draft line can still be refused: a foreign
    /// line without a positive rate is the `RangeError` `convert` throws in TS
    /// (`money.ts:37-39`). A fractional or unsafe amount is refused the same way
    /// (`MoneyError::NotMinorUnits`), where TS would store the fraction in a draft; a Rust
    /// journal line holds integer minor units only.
    fn line_values(&self, input: &EntryInput) -> Result<Vec<LineValues>, Error> {
        let base = &self.settings.base_currency;
        input
            .lines
            .iter()
            .map(|l| {
                let currency = l.currency.clone().unwrap_or_else(|| base.clone());
                let fx_rate = if &currency == base {
                    1.0
                } else {
                    l.fx_rate.unwrap_or(f64::NAN)
                };
                let debit = assert_minor(l.debit.unwrap_or(0.0), AMOUNT)?;
                let credit = assert_minor(l.credit.unwrap_or(0.0), AMOUNT)?;
                Ok(LineValues {
                    account_id: l.account_id.clone(),
                    debit,
                    credit,
                    base_amount: convert(debit - credit, fx_rate)?,
                    currency,
                    fx_rate,
                    fund_id: l.fund_id.clone(),
                    grant_id: l.grant_id.value().cloned(),
                    dimension_ids: l.dimension_ids.clone().unwrap_or_default(),
                    flow_node_id: l.flow_node_id.value().cloned(),
                    memo: l.memo.value().cloned(),
                })
            })
            .collect()
    }

    /// An entry ready to store as a draft. A date that is not `YYYY-MM-DD` is refused with the
    /// message `validateEntry` gives it; TS stores such a draft as is, but
    /// [`JournalEntry::date`] is a validated [`IsoDate`].
    fn prepare(&self, input: &EntryInput) -> Result<Prepared, Error> {
        let lines = self.line_values(input)?;
        let date = IsoDate::parse(&input.date)
            .map_err(|_| LedgerError::fail(IssueCode::InvalidInput, "Enter the entry date."))?;
        Ok(Prepared { date, lines })
    }

    /// The id of the period covering `date`, or `''` as TS writes `periodFor(date)?.id ?? ''`.
    fn period_id_for(&self, date: &str) -> PeriodId {
        self.period_for(date)
            .map_or_else(|| PeriodId::from(""), |p| p.id.clone())
    }

    fn entry_index(&self, id: &EntryId) -> Result<usize, Error> {
        match self.entries.iter().position(|e| &e.id == id) {
            Some(i) => Ok(i),
            None => fail(IssueCode::NotFound, "Entry not found."),
        }
    }

    /// `docPayments(kind, id)` (`engine.ts:619-620`): posted payments settling the document.
    fn has_posted_payments(&self, doc: &AppliesTo) -> bool {
        self.payments.iter().any(|p| {
            &p.applies_to == doc
                && self
                    .entry(&p.entry_id)
                    .is_some_and(|e| e.status == EntryStatus::Posted)
        })
    }

    /// What reversing `e` does to its source document (`engine.ts:617-644`), checked before
    /// anything changes: a bill or claim with posted payments cannot have its own entry reversed.
    fn on_reversed(&self, e: &JournalEntry) -> Result<OnReversed, Error> {
        let source_id = e.source_id.as_deref().unwrap_or_default();
        Ok(match e.source_type {
            SourceType::Bill => {
                let b = self.bill(&BillId::from(source_id))?;
                if self.has_posted_payments(&AppliesTo::Bill(b.id.clone())) {
                    return fail(
                        IssueCode::InvalidState,
                        format!(
                            "Bill {} has payments. Reverse the payments first.",
                            b.number
                        ),
                    );
                }
                OnReversed::VoidBill(b.id.clone())
            }
            SourceType::ExpenseClaim => {
                let c = self.claim(&ClaimId::from(source_id))?;
                if self.has_posted_payments(&AppliesTo::ExpenseClaim(c.id.clone())) {
                    return fail(
                        IssueCode::InvalidState,
                        format!(
                            "Claim {} has been reimbursed. Reverse the reimbursement first.",
                            c.number
                        ),
                    );
                }
                OnReversed::RejectClaim(c.id.clone())
            }
            SourceType::Payment => OnReversed::ReopenBill(BillId::from(source_id)),
            SourceType::Reimbursement => OnReversed::ReopenClaim(ClaimId::from(source_id)),
            _ => OnReversed::Nothing,
        })
    }
}

impl Ledger<'_> {
    /// Stores a checked entry as a draft (`saveDraft` after its permission check,
    /// `engine.ts:500-524`). Draws, in order: the entry id, one id per line, the audit id
    /// (`entry.draft`, detail the memo). Takes the next `JE` number. Returns the entry's index.
    fn store_draft(&mut self, actor: &Actor, input: &EntryInput, prepared: Prepared) -> usize {
        let id = EntryId::from(self.env.new_id());
        let number = self.next_number("JE");
        let entry = JournalEntry {
            id: id.clone(),
            number,
            date: prepared.date,
            period_id: self.state.period_id_for(&input.date),
            memo: js_trim(&input.memo).to_owned(),
            source_type: input.source_type.unwrap_or(SourceType::Manual),
            source_id: input.source_id.value().cloned(),
            status: EntryStatus::Draft,
            reversal_of: None,
            reversed_by: None,
            override_reason: override_of(&input.override_reason).map(str::to_owned),
            created_by: actor.id.clone(),
            created_at: self.env.now(),
            posted_by: None,
            posted_at: None,
        };
        let memo = entry.memo.clone();
        self.state.entries.push(entry);
        let index = self.state.entries.len() - 1;
        self.store_lines(&id, prepared.lines);
        self.audit(actor, "entry.draft", id.as_str(), &memo);
        index
    }

    /// Appends an entry's lines, drawing one id each in order.
    fn store_lines(&mut self, entry_id: &EntryId, lines: Vec<LineValues>) {
        for l in lines {
            let line = JournalLine {
                id: LineId::from(self.env.new_id()),
                entry_id: entry_id.clone(),
                account_id: l.account_id,
                debit: l.debit,
                credit: l.credit,
                currency: l.currency,
                fx_rate: l.fx_rate,
                base_amount: l.base_amount,
                fund_id: l.fund_id,
                grant_id: l.grant_id,
                dimension_ids: l.dimension_ids,
                flow_node_id: l.flow_node_id,
                memo: l.memo,
            };
            self.state.lines.push(line);
        }
    }

    /// `post`'s checks (`engine.ts:528-530`): permission, then every blocking issue of
    /// `validateEntry` (overridable ones pass when the input carries an override reason).
    fn check_post(
        &self,
        actor: &Actor,
        input: &EntryInput,
        opts: EntryOptions,
    ) -> Result<Prepared, Error> {
        require(actor, Permission::Post)?;
        let issues = self.state.validate_entry(input, None, opts)?;
        if let Some(e) = LedgerError::new(blocking(issues, &input.override_reason)) {
            return Err(e.into());
        }
        self.state.prepare(input)
    }

    /// `post` after its checks (`engine.ts:531-536`): stores the draft, then marks it posted by
    /// the actor now and draws the `entry.post` audit id (detail the number).
    fn store_posted(
        &mut self,
        actor: &Actor,
        input: &EntryInput,
        prepared: Prepared,
    ) -> JournalEntry {
        let index = self.store_draft(actor, input, prepared);
        let now = self.env.now();
        let e = &mut self.state.entries[index];
        e.status = EntryStatus::Posted;
        e.posted_by = Some(actor.id.clone());
        e.posted_at = Some(now);
        let entry = e.clone();
        self.audit(actor, "entry.post", entry.id.as_str(), &entry.number);
        entry
    }

    /// `saveDraft` (`engine.ts:499-524`): saves an entry as a draft, not checked for balance.
    /// Needs `can_post`. The period is the one covering the date, or `''`; the memo and override
    /// reason are trimmed (a blank reason is null); the source defaults to `manual`. Draws the
    /// entry id, one id per line, then the `entry.draft` audit id.
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `INVALID_INPUT` for a date that is not `YYYY-MM-DD`; [`Error::Money`] for a
    /// line amount that is not whole minor units or a foreign line without a valid rate. See
    /// [`LedgerState::validate_entry`] for what posting checks.
    pub fn save_draft(&mut self, actor: &Actor, input: &EntryInput) -> Result<JournalEntry, Error> {
        require(actor, Permission::Post)?;
        let prepared = self.state.prepare(input)?;
        let index = self.store_draft(actor, input, prepared);
        Ok(self.state.entries[index].clone())
    }

    /// `post` (`engine.ts:526-537`): validates and posts a new entry in one step. Needs
    /// `can_post`; every blocking issue of [`LedgerState::validate_entry`] rejects it, all of
    /// them in one [`LedgerError`]. Draws the ids of [`Ledger::save_draft`], then the
    /// `entry.post` audit id.
    ///
    /// # Errors
    ///
    /// `PERMISSION`, or the blocking validation issues; [`Error::Money`] as `validate_entry`.
    pub fn post(
        &mut self,
        actor: &Actor,
        input: &EntryInput,
        opts: EntryOptions,
    ) -> Result<JournalEntry, Error> {
        let prepared = self.check_post(actor, input, opts)?;
        Ok(self.store_posted(actor, input, prepared))
    }

    /// `updateDraft` (`engine.ts:563-577`): replaces a draft's date, period, memo, override
    /// reason and lines; its source and number stay. Posted entries cannot be edited. Needs
    /// `can_post`. The old lines are removed and the new ones appended, drawing one id each,
    /// then the `entry.update` audit id (detail the memo).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` for a posted or reversed entry; the draft
    /// line errors of [`Ledger::save_draft`].
    pub fn update_draft(
        &mut self,
        actor: &Actor,
        id: &EntryId,
        input: &EntryInput,
    ) -> Result<JournalEntry, Error> {
        require(actor, Permission::Post)?;
        let index = self.state.entry_index(id)?;
        let e = &self.state.entries[index];
        if e.status != EntryStatus::Draft {
            return fail(
                IssueCode::InvalidState,
                format!(
                    "{} is posted and cannot be changed. Reverse it and post a correction.",
                    e.number
                ),
            );
        }
        let prepared = self.state.prepare(input)?;
        let period_id = self.state.period_id_for(&input.date);
        let e = &mut self.state.entries[index];
        e.date = prepared.date;
        e.period_id = period_id;
        js_trim(&input.memo).clone_into(&mut e.memo);
        e.override_reason = override_of(&input.override_reason).map(str::to_owned);
        let entry = e.clone();
        self.state.lines.retain(|l| &l.entry_id != id);
        self.store_lines(id, prepared.lines);
        self.audit(actor, "entry.update", id.as_str(), &entry.memo);
        Ok(entry)
    }

    /// `deleteDraft` (`engine.ts:579-587`): removes a draft and its lines. Needs `can_post`.
    /// One audit id (`entry.delete`, detail the number).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` for a posted or reversed entry.
    pub fn delete_draft(&mut self, actor: &Actor, id: &EntryId) -> Result<(), Error> {
        require(actor, Permission::Post)?;
        let index = self.state.entry_index(id)?;
        let e = &self.state.entries[index];
        if e.status != EntryStatus::Draft {
            return fail(
                IssueCode::InvalidState,
                format!("{} is posted and cannot be deleted.", e.number),
            );
        }
        let number = e.number.clone();
        self.state.entries.retain(|x| &x.id != id);
        self.state.lines.retain(|l| &l.entry_id != id);
        self.audit(actor, "entry.delete", id.as_str(), &number);
        Ok(())
    }

    /// `postDraft` (`engine.ts:589-604`): posts a draft after validating it as stored
    /// ([`LedgerState::input_of`]), leaving the entry itself out of grant spend. A non-blank
    /// `override_reason` replaces the stored one (trimmed). Needs `can_post`. Sets the period
    /// from the date, the poster and the time, then draws the `entry.post` audit id.
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND`; `INVALID_STATE` when already posted; the blocking validation
    /// issues.
    pub fn post_draft(
        &mut self,
        actor: &Actor,
        id: &EntryId,
        override_reason: Option<&str>,
    ) -> Result<JournalEntry, Error> {
        require(actor, Permission::Post)?;
        let index = self.state.entry_index(id)?;
        let e = &self.state.entries[index];
        if e.status != EntryStatus::Draft {
            return fail(
                IssueCode::InvalidState,
                format!("{} is already posted.", e.number),
            );
        }
        let mut input = self.state.input_of(id)?;
        if let Some(reason) = override_reason.map(js_trim).filter(|r| !r.is_empty()) {
            input.override_reason = Nullish::Value(reason.to_owned());
        }
        let issues = self
            .state
            .validate_entry(&input, Some(id), EntryOptions::default())?;
        if let Some(e) = LedgerError::new(blocking(issues, &input.override_reason)) {
            return Err(e.into());
        }
        let period_id = self.state.period_id_for(&input.date);
        let now = self.env.now();
        let e = &mut self.state.entries[index];
        e.status = EntryStatus::Posted;
        e.override_reason = input.override_reason.value().cloned();
        e.period_id = period_id;
        e.posted_by = Some(actor.id.clone());
        e.posted_at = Some(now);
        let entry = e.clone();
        self.audit(actor, "entry.post", id.as_str(), &entry.number);
        Ok(entry)
    }

    /// `reverse` (`engine.ts:606-662`): posts the mirror image of a posted entry and marks the
    /// original reversed; the original's lines are never touched. Needs `can_post`.
    ///
    /// The reversing entry is dated `date` (its period follows the date and must be open), has
    /// memo `Reversal of JE-0001: <reason>`, source `reversal` pointing at the original,
    /// override reason `Reversal`, and every line with debit and credit swapped (rate, fund,
    /// grant, tags and memo kept). It posts as a system entry, so grant limits, dates and
    /// inactive accounts cannot block it. It draws the ids of [`Ledger::post`]; then
    /// `reversal_of` and `reversed_by` link the two, the source document follows (a bill's
    /// entry voids the bill; a claim's entry rejects the claim with the note
    /// `Reversed: <reason>`; a payment or reimbursement sets a paid bill or reimbursed claim back
    /// to posted), and one more audit id is drawn (`entry.reverse`, detail the reason as given).
    ///
    /// # Errors
    ///
    /// `PERMISSION`; `NOT_FOUND` for the entry or its source document; `INVALID_STATE` unless
    /// the entry is posted, for a reversal of a reversal, and for a bill or claim with posted
    /// payments; `INVALID_INPUT` for a blank reason; the blocking validation issues of the
    /// reversing entry (a closed period, for example).
    pub fn reverse(
        &mut self,
        actor: &Actor,
        id: &EntryId,
        date: &str,
        reason: &str,
    ) -> Result<JournalEntry, Error> {
        require(actor, Permission::Post)?;
        let index = self.state.entry_index(id)?;
        let e = &self.state.entries[index];
        if e.status != EntryStatus::Posted {
            return fail(
                IssueCode::InvalidState,
                format!(
                    "{} is {} and cannot be reversed.",
                    e.number,
                    status_str(e.status)
                ),
            );
        }
        if !has_text(reason) {
            return fail(IssueCode::InvalidInput, "Give a reason for the reversal.");
        }
        if e.source_type == SourceType::Reversal {
            return fail(
                IssueCode::InvalidState,
                format!(
                    "{} is itself a reversal. Post a new entry instead.",
                    e.number
                ),
            );
        }
        let on_reversed = self.state.on_reversed(e)?;
        let number = e.number.clone();
        let original = self.state.input_of(id)?;
        let input = EntryInput {
            date: date.to_owned(),
            memo: format!("Reversal of {number}: {}", js_trim(reason)),
            source_type: Some(SourceType::Reversal),
            source_id: Nullish::Value(id.as_str().to_owned()),
            // A reversal reduces spend, so grant limits cannot block it (engine.ts:651).
            override_reason: Nullish::Value("Reversal".to_owned()),
            lines: original
                .lines
                .into_iter()
                .map(|l| LineInput {
                    debit: l.credit,
                    credit: l.debit,
                    ..l
                })
                .collect(),
        };
        let prepared = self.check_post(actor, &input, EntryOptions { system: true })?;
        // TS looks the paid document up only after posting and throws NOT_FOUND then, which its
        // callers discard; checking first gives the same outcome with no change.
        match &on_reversed {
            OnReversed::ReopenBill(b) => {
                self.state.bill(b)?;
            }
            OnReversed::ReopenClaim(c) => {
                self.state.claim(c)?;
            }
            OnReversed::Nothing | OnReversed::VoidBill(_) | OnReversed::RejectClaim(_) => {}
        }

        let mut rev = self.store_posted(actor, &input, prepared);
        rev.reversal_of = Some(id.clone());
        if let Some(r) = self.state.entries.iter_mut().find(|x| x.id == rev.id) {
            r.reversal_of = Some(id.clone());
        }
        let e = &mut self.state.entries[index];
        e.status = EntryStatus::Reversed;
        e.reversed_by = Some(rev.id.clone());
        self.apply_on_reversed(actor, on_reversed, reason);
        self.audit(actor, "entry.reverse", id.as_str(), reason);
        Ok(rev)
    }

    /// `onReversed` (`engine.ts:621-644`).
    fn apply_on_reversed(&mut self, actor: &Actor, on: OnReversed, reason: &str) {
        let now = self.env.now();
        match on {
            OnReversed::Nothing => {}
            OnReversed::VoidBill(id) => {
                if let Some(b) = self.state.bills.iter_mut().find(|b| b.id == id) {
                    b.status = BillStatus::Void;
                }
            }
            OnReversed::RejectClaim(id) => {
                if let Some(c) = self.state.claims.iter_mut().find(|c| c.id == id) {
                    c.status = ClaimStatus::Rejected;
                    c.approvals.push(ApprovalStep {
                        at: now,
                        by: actor.id.clone(),
                        action: ApprovalAction::Rejected,
                        note: Some(format!("Reversed: {}", js_trim(reason))),
                    });
                }
            }
            OnReversed::ReopenBill(id) => {
                if let Some(b) = self.state.bills.iter_mut().find(|b| b.id == id)
                    && b.status == BillStatus::Paid
                {
                    b.status = BillStatus::Posted;
                }
            }
            OnReversed::ReopenClaim(id) => {
                if let Some(c) = self.state.claims.iter_mut().find(|c| c.id == id)
                    && c.status == ClaimStatus::Reimbursed
                {
                    c.status = ClaimStatus::Posted;
                }
            }
        }
    }
}
