//! Budgets and budget lines (`engine.ts:775-804`). Budget against actual is a report (C5).

use serde::{Deserialize, Serialize};

use super::setup::truthy;
use super::{Error, Ledger, Permission, fail, require};
use crate::errors::IssueCode;
use crate::money::assert_minor;
use crate::types::{
    AccountId, AccountType, Actor, Budget, BudgetId, BudgetLine, BudgetLineId, BudgetStatus,
    DimensionId, FundId, GrantId, Nullish, PeriodId,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewBudgetLine {
    /// The line to replace, or the id of the new line; drawn from the env when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<BudgetLineId>,
    /// Budget the line belongs to; it must exist.
    pub budget_id: BudgetId,
    /// An expense account, or null for every expense account.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub account_id: Nullish<AccountId>,
    /// Fund filter.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub fund_id: Nullish<FundId>,
    /// Grant filter.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub grant_id: Nullish<GrantId>,
    /// Dimension filter.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub dimension_id: Nullish<DimensionId>,
    /// A period, or null for the whole fiscal year.
    #[serde(default, skip_serializing_if = "Nullish::is_undefined")]
    pub period_id: Nullish<PeriodId>,
    /// Budgeted amount in base currency: whole minor units, not negative.
    #[serde(serialize_with = "crate::js_number::serialize")]
    pub amount: f64,
}

/// `Object.assign` of one nullable field: left out keeps the old value.
fn assign<T>(field: &mut Option<T>, value: Nullish<T>) {
    match value {
        Nullish::Undefined => {}
        Nullish::Null => *field = None,
        Nullish::Value(v) => *field = Some(v),
    }
}

/// A nullable field of a new line: left out or `null` is null.
fn or_null<T>(value: Nullish<T>) -> Option<T> {
    match value {
        Nullish::Value(v) => Some(v),
        Nullish::Undefined | Nullish::Null => None,
    }
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
        require(actor, Permission::Post)?;
        let id = match input.id {
            Some(id) => id,
            None => BudgetId::from(self.env.new_id()),
        };
        let b = Budget {
            id,
            name: input.name,
            fiscal_year: input.fiscal_year,
            status: input.status,
        };
        self.state.budgets.push(b.clone());
        self.audit(actor, "budget.create", b.id.as_str(), &b.name);
        Ok(b)
    }

    /// `setBudgetLine` (`engine.ts:788-804`): adds a budget line, or replaces the fields of the
    /// line with the given id. Needs `can_post`. The amount must be whole minor units
    /// ([`Error::Money`], "Budget amount ...") and not negative (`NEGATIVE_AMOUNT`); the budget
    /// must exist (`NOT_FOUND`); a given account must be an expense account (`INVALID_INPUT`).
    /// No audit event. A given id that names a line updates it in place, drawing nothing; a new
    /// line draws its id only when none is given (an empty id is kept, as TS keeps it).
    ///
    /// # Errors
    ///
    /// [`Error::Ledger`] with the codes above, [`Error::Money`] for a fractional amount.
    pub fn set_budget_line(
        &mut self,
        actor: &Actor,
        input: NewBudgetLine,
    ) -> Result<BudgetLine, Error> {
        require(actor, Permission::Post)?;
        let amount = assert_minor(input.amount, "Budget amount")?;
        if amount < 0 {
            return fail(IssueCode::NegativeAmount, "A budget cannot be negative.");
        }
        if !self.state.budgets.iter().any(|b| b.id == input.budget_id) {
            return fail(IssueCode::NotFound, "Budget not found.");
        }
        if let Some(account_id) = truthy(input.account_id.value())
            && self.state.account(account_id).map(|a| a.account_type) != Some(AccountType::Expense)
        {
            return fail(
                IssueCode::InvalidInput,
                "Budget lines are set on expense accounts.",
            );
        }
        let existing = truthy(input.id.as_ref())
            .and_then(|id| self.state.budget_lines.iter().position(|l| &l.id == id));
        if let Some(index) = existing {
            let l = &mut self.state.budget_lines[index];
            l.budget_id = input.budget_id;
            assign(&mut l.account_id, input.account_id);
            assign(&mut l.fund_id, input.fund_id);
            assign(&mut l.grant_id, input.grant_id);
            assign(&mut l.dimension_id, input.dimension_id);
            assign(&mut l.period_id, input.period_id);
            l.amount = amount;
            return Ok(l.clone());
        }
        let id = match input.id {
            Some(id) => id,
            None => BudgetLineId::from(self.env.new_id()),
        };
        let line = BudgetLine {
            id,
            budget_id: input.budget_id,
            account_id: or_null(input.account_id),
            fund_id: or_null(input.fund_id),
            grant_id: or_null(input.grant_id),
            dimension_id: or_null(input.dimension_id),
            period_id: or_null(input.period_id),
            amount,
        };
        self.state.budget_lines.push(line.clone());
        Ok(line)
    }
}
