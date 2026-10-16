//! The ledger engine, mirroring the `Ledger` class of `flowtrace/src/lib/ledger/engine.ts`.
//!
//! Skeleton for the failing parity tests; the next commit fills it in.

use serde::Deserialize;

use crate::env::LedgerEnv;
use crate::errors::{Issue, LedgerError};
use crate::money::{Minor, MoneyError};
use crate::types::{
    Account, AccountId, AccountType, Actor, CashFlowClass, ClaimId, DimensionId, DimensionKind,
    DimensionValue, EntryId, EntryInput, ExpenseClaim, ExpenseFunction, FiscalPeriod, FlowNodeId,
    Fund, FundId, FundRestriction, Grant, GrantId, JournalEntry, JournalLine, LedgerState,
    NormalBalance, Nullish, OrgLedgerSettings, PeriodId,
};

/// Why an engine call failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A ledger rule.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    /// A `RangeError` from `money.ts`.
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// Options of `validateEntry` and `post`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct EntryOptions {
    /// A system posting.
    #[serde(default)]
    pub system: bool,
}

/// `addAccount` input.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewAccount {
    /// Id.
    #[serde(default)]
    pub id: Option<AccountId>,
    /// Code.
    pub code: String,
    /// Name.
    pub name: String,
    /// Type.
    #[serde(rename = "type")]
    pub account_type: AccountType,
    /// Parent.
    #[serde(default)]
    pub parent_id: Option<AccountId>,
    /// Header account.
    pub is_group: bool,
    /// Active.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// Cash.
    pub is_cash: bool,
    /// Function.
    #[serde(default)]
    pub expense_function: Option<ExpenseFunction>,
    /// Cash-flow class.
    #[serde(default)]
    pub cash_flow_class: Option<CashFlowClass>,
}

/// `updateAccount` patch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct AccountPatch {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Active.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// Function.
    #[serde(default)]
    pub expense_function: Nullish<ExpenseFunction>,
}

/// `addFund` input.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewFund {
    /// Id.
    #[serde(default)]
    pub id: Option<FundId>,
    /// Name.
    pub name: String,
    /// Restriction.
    pub restriction: FundRestriction,
}

/// `addDimension` input.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewDimension {
    /// Id.
    #[serde(default)]
    pub id: Option<DimensionId>,
    /// Kind.
    pub kind: DimensionKind,
    /// Name.
    pub name: String,
    /// Node.
    #[serde(default)]
    pub node_id: Option<FlowNodeId>,
}

/// `normalBalanceOf`.
#[must_use]
pub fn normal_balance_of(_account_type: AccountType) -> NormalBalance {
    todo!()
}

/// `emptyLedger`.
#[must_use]
pub fn empty_ledger(_settings: OrgLedgerSettings) -> LedgerState {
    todo!()
}

/// The engine over one organisation's books.
pub struct Ledger<'a> {
    state: &'a mut LedgerState,
    env: &'a mut dyn LedgerEnv,
}

#[allow(
    missing_docs,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value
)]
impl<'a> Ledger<'a> {
    pub fn new(state: &'a mut LedgerState, env: &'a mut dyn LedgerEnv) -> Self {
        Self { state, env }
    }
    #[must_use]
    pub fn state(&self) -> &LedgerState {
        let _ = &self.env;
        self.state
    }
    pub fn add_account(&mut self, _actor: &Actor, _input: NewAccount) -> Result<Account, Error> {
        todo!()
    }
    pub fn update_account(
        &mut self,
        _actor: &Actor,
        _id: &AccountId,
        _patch: &AccountPatch,
    ) -> Result<Account, Error> {
        todo!()
    }
    pub fn add_fund(&mut self, _actor: &Actor, _input: NewFund) -> Result<Fund, Error> {
        todo!()
    }
    pub fn add_dimension(
        &mut self,
        _actor: &Actor,
        _input: NewDimension,
    ) -> Result<DimensionValue, Error> {
        todo!()
    }
    pub fn open_fiscal_year(&mut self, _actor: &Actor, _year: i32) -> Result<(), Error> {
        todo!()
    }
    pub fn ensure_fiscal_year(&mut self, _year: i32) {
        todo!()
    }
    pub fn save_draft(
        &mut self,
        _actor: &Actor,
        _input: &EntryInput,
    ) -> Result<JournalEntry, Error> {
        todo!()
    }
    pub fn post(
        &mut self,
        _actor: &Actor,
        _input: &EntryInput,
        _opts: EntryOptions,
    ) -> Result<JournalEntry, Error> {
        todo!()
    }
    pub fn update_draft(
        &mut self,
        _actor: &Actor,
        _id: &EntryId,
        _input: &EntryInput,
    ) -> Result<JournalEntry, Error> {
        todo!()
    }
    pub fn delete_draft(&mut self, _actor: &Actor, _id: &EntryId) -> Result<(), Error> {
        todo!()
    }
    pub fn post_draft(
        &mut self,
        _actor: &Actor,
        _id: &EntryId,
        _override_reason: Option<&str>,
    ) -> Result<JournalEntry, Error> {
        todo!()
    }
    pub fn reverse(
        &mut self,
        _actor: &Actor,
        _id: &EntryId,
        _date: &str,
        _reason: &str,
    ) -> Result<JournalEntry, Error> {
        todo!()
    }
    pub fn close_period(&mut self, _actor: &Actor, _id: &PeriodId) -> Result<FiscalPeriod, Error> {
        todo!()
    }
    pub fn reopen_period(
        &mut self,
        _actor: &Actor,
        _id: &PeriodId,
        _reason: &str,
    ) -> Result<FiscalPeriod, Error> {
        todo!()
    }
    pub fn lock_period(&mut self, _actor: &Actor, _id: &PeriodId) -> Result<FiscalPeriod, Error> {
        todo!()
    }
    pub fn close_year(
        &mut self,
        _actor: &Actor,
        _year: i32,
    ) -> Result<Option<JournalEntry>, Error> {
        todo!()
    }
}

#[allow(missing_docs, clippy::missing_errors_doc)]
impl LedgerState {
    #[must_use]
    pub fn account(&self, _id: &AccountId) -> Option<&Account> {
        todo!()
    }
    #[must_use]
    pub fn account_by_code(&self, _code: &str) -> Option<&Account> {
        todo!()
    }
    #[must_use]
    pub fn fund(&self, _id: &FundId) -> Option<&Fund> {
        todo!()
    }
    #[must_use]
    pub fn grant(&self, _id: &GrantId) -> Option<&Grant> {
        todo!()
    }
    #[must_use]
    pub fn entry(&self, _id: &EntryId) -> Option<&JournalEntry> {
        todo!()
    }
    #[must_use]
    pub fn lines_of(&self, _id: &EntryId) -> Vec<&JournalLine> {
        todo!()
    }
    #[must_use]
    pub fn period_for(&self, _date: &str) -> Option<&FiscalPeriod> {
        todo!()
    }
    pub fn input_of(&self, _id: &EntryId) -> Result<EntryInput, LedgerError> {
        todo!()
    }
    #[must_use]
    pub fn grant_spend(&self, _grant: &GrantId, _ignore: Option<&EntryId>) -> Minor {
        todo!()
    }
    pub fn validate_entry(
        &self,
        _input: &EntryInput,
        _ignore: Option<&EntryId>,
        _opts: EntryOptions,
    ) -> Result<Vec<Issue>, Error> {
        todo!()
    }
    pub fn claim(&self, _id: &ClaimId) -> Result<&ExpenseClaim, LedgerError> {
        todo!()
    }
}
