//! The engine's reads over the books (`engine.ts:109-135`, `engine.ts:447-464`,
//! `engine.ts:539-561`). They never call the env.

use std::collections::HashSet;

use crate::errors::{IssueCode, LedgerError};
use crate::money::Minor;
use crate::types::{
    Account, AccountId, AccountType, AppliesTo, Bill, BillId, ClaimId, EntryId, EntryInput,
    EntryStatus, ExpenseClaim, FiscalPeriod, Fund, FundId, Grant, GrantId, JournalEntry,
    JournalLine, LedgerState, LineInput, Nullish,
};

/// A whole number of minor units as the JS number TS holds; exact, as amounts are safe integers.
#[expect(
    clippy::cast_precision_loss,
    reason = "amounts are safe integers, exact in f64"
)]
pub(super) fn js_number(minor: Minor) -> f64 {
    minor as f64
}

impl LedgerState {
    /// `account(id)` (`engine.ts:113-115`): the first account with this id.
    #[must_use]
    pub fn account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.iter().find(|a| &a.id == id)
    }

    /// `accountByCode(code)` (`engine.ts:116-118`): the first account with exactly this code.
    #[must_use]
    pub fn account_by_code(&self, code: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.code == code)
    }

    /// `fund(id)` (`engine.ts:119-121`).
    #[must_use]
    pub fn fund(&self, id: &FundId) -> Option<&Fund> {
        self.funds.iter().find(|f| &f.id == id)
    }

    /// `grant(id)` (`engine.ts:122-124`).
    #[must_use]
    pub fn grant(&self, id: &GrantId) -> Option<&Grant> {
        self.grants.iter().find(|g| &g.id == id)
    }

    /// `entry(id)` (`engine.ts:125-127`).
    #[must_use]
    pub fn entry(&self, id: &EntryId) -> Option<&JournalEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// `linesOf(entryId)` (`engine.ts:128-130`): the entry's lines in book order.
    #[must_use]
    pub fn lines_of(&self, entry_id: &EntryId) -> Vec<&JournalLine> {
        self.lines
            .iter()
            .filter(|l| &l.entry_id == entry_id)
            .collect()
    }

    /// `periodFor(date)` (`engine.ts:131-135`): the first period whose dates cover `date`,
    /// compared as strings, so any string is accepted and a malformed date finds nothing.
    #[must_use]
    pub fn period_for(&self, date: &str) -> Option<&FiscalPeriod> {
        self.periods
            .iter()
            .find(|p| p.start_date.as_str() <= date && date <= p.end_date.as_str())
    }

    /// `claim(id)` (`engine.ts:905-907`).
    ///
    /// # Errors
    ///
    /// `NOT_FOUND` "Claim not found." when there is no such claim.
    pub fn claim(&self, id: &ClaimId) -> Result<&ExpenseClaim, LedgerError> {
        self.claims
            .iter()
            .find(|c| &c.id == id)
            .ok_or_else(|| LedgerError::fail(IssueCode::NotFound, "Claim not found."))
    }

    /// `outstanding(kind, id)` (`engine.ts:877-886`): what is still owed on a bill or claim, in
    /// its currency: the sum of its lines less its payments whose entries are posted.
    ///
    /// # Errors
    ///
    /// `NOT_FOUND` "Bill not found." or "Claim not found." when there is no such document.
    pub fn outstanding(&self, doc: &AppliesTo) -> Result<Minor, LedgerError> {
        let _ = doc;
        todo!()
    }

    /// The private `bill(id)` (`engine.ts:818-820`), for reversals of bill entries.
    pub(super) fn bill(&self, id: &BillId) -> Result<&Bill, LedgerError> {
        self.bills
            .iter()
            .find(|b| &b.id == id)
            .ok_or_else(|| LedgerError::fail(IssueCode::NotFound, "Bill not found."))
    }

    /// `inputOf(entryId)` (`engine.ts:539-561`): the input that would recreate a stored entry,
    /// with every line field written out (currency, rate, null grant and so on).
    ///
    /// # Errors
    ///
    /// `NOT_FOUND` "Entry not found." when there is no such entry.
    pub fn input_of(&self, entry_id: &EntryId) -> Result<EntryInput, LedgerError> {
        let e = self
            .entry(entry_id)
            .ok_or_else(|| LedgerError::fail(IssueCode::NotFound, "Entry not found."))?;
        Ok(EntryInput {
            date: e.date.as_str().to_owned(),
            memo: e.memo.clone(),
            source_type: Some(e.source_type),
            source_id: e.source_id.clone().into(),
            override_reason: e.override_reason.clone().into(),
            lines: self
                .lines_of(entry_id)
                .into_iter()
                .map(|l| LineInput {
                    account_id: l.account_id.clone(),
                    debit: Some(js_number(l.debit)),
                    credit: Some(js_number(l.credit)),
                    currency: Some(l.currency.clone()),
                    fx_rate: Some(l.fx_rate),
                    fund_id: l.fund_id.clone(),
                    grant_id: l.grant_id.clone().into(),
                    dimension_ids: Some(l.dimension_ids.clone()),
                    flow_node_id: l.flow_node_id.clone().into(),
                    memo: Nullish::from(l.memo.clone()),
                })
                .collect(),
        })
    }

    /// `grantSpend(grantId, ignoreEntryId)` (`engine.ts:447-464`): posted (or reversed) net
    /// expense charged to the grant, in base currency, leaving out one entry when asked.
    #[must_use]
    pub fn grant_spend(&self, grant_id: &GrantId, ignore: Option<&EntryId>) -> Minor {
        let posted: HashSet<&EntryId> = self
            .entries
            .iter()
            .filter(|e| e.status != EntryStatus::Draft && Some(&e.id) != ignore)
            .map(|e| &e.id)
            .collect();
        self.lines
            .iter()
            .filter(|l| {
                l.grant_id.as_ref() == Some(grant_id)
                    && posted.contains(&l.entry_id)
                    && self
                        .account(&l.account_id)
                        .is_some_and(|a| a.account_type == AccountType::Expense)
            })
            .map(|l| l.base_amount)
            .sum()
    }
}
