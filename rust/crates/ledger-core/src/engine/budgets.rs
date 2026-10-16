//! Budgets and budget lines (`engine.ts:775-804`). Budget against actual is a report (C5).

use serde::Deserialize;

use super::{Error, Ledger};
use crate::types::{
    AccountId, Actor, Budget, BudgetId, BudgetLine, BudgetLineId, BudgetStatus, DimensionId,
    FundId, GrantId, Nullish, PeriodId,
};

/// The input of `addBudget` (`engine.ts:780`): a [`Budget`] with `id` optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewBudget {
    /// Id; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<BudgetId>,
    /// Display name.
    pub name: String,
    /// Fiscal year budgeted.
    pub fiscal_year: i32,
    /// Draft or approved.
    pub status: BudgetStatus,
}

/// The input of `setBudgetLine` (`engine.ts:788`): a [`BudgetLine`] with `id` optional and the
/// amount as the raw JS number the caller sent.
///
/// The filters are [`Nullish`] because an update copies only the fields the input carries
/// (`Object.assign`, `engine.ts:796-797`): a left-out filter keeps its old value, a `null` one
/// clears it. A new line reads a left-out filter as `null`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewBudgetLine {
    /// The line to replace, or the id of the new line; drawn from the env when absent.
    #[serde(default)]
    pub id: Option<BudgetLineId>,
    /// Budget the line belongs to; it must exist.
    pub budget_id: BudgetId,
    /// An expense account, or null for every expense account.
    #[serde(default)]
    pub account_id: Nullish<AccountId>,
    /// Fund filter.
    #[serde(default)]
    pub fund_id: Nullish<FundId>,
    /// Grant filter.
    #[serde(default)]
    pub grant_id: Nullish<GrantId>,
    /// Dimension filter.
    #[serde(default)]
    pub dimension_id: Nullish<DimensionId>,
    /// A period, or null for the whole fiscal year.
    #[serde(default)]
    pub period_id: Nullish<PeriodId>,
    /// Budgeted amount in base currency: whole minor units, not negative.
    pub amount: f64,
}

impl Ledger<'_> {
    /// `addBudget` (`engine.ts:780-786`): adds a budget. Needs `can_post`; no uniqueness check,
    /// as in TS. Draws the budget id (when none is given), then one audit id (`budget.create`,
    /// detail the name).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] `PERMISSION`.
    pub fn add_budget(&mut self, actor: &Actor, input: NewBudget) -> Result<Budget, Error> {
        let _ = (actor, input);
        todo!()
    }

    /// `setBudgetLine` (`engine.ts:788-804`): adds a budget line, or replaces the fields of the
    /// line with the given id. Needs `can_post`. The amount must be whole minor units
    /// ([`Error::Money`], "Budget amount ...") and not negative (`NEGATIVE_AMOUNT`); the budget
    /// must exist (`NOT_FOUND`); a given account must be an expense account (`INVALID_INPUT`).
    /// No audit event. A new line draws its id only when none is given.
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above, [`Error::Money`] for a fractional amount.
    pub fn set_budget_line(
        &mut self,
        actor: &Actor,
        input: NewBudgetLine,
    ) -> Result<BudgetLine, Error> {
        let _ = (actor, input);
        todo!()
    }
}
