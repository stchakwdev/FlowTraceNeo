//! Budget versus actual, grant summaries and the trust-score signals (`reports.ts:453-622`).

use std::collections::{HashMap, HashSet};
use std::slice;

use serde::{Deserialize, Serialize};

use super::{
    Basis, Posted, ReportError, Scope, account_map, js, js_round, minor_of, months_between, posted,
    truthy,
};
use crate::money::{Minor, MoneyError, convert, sum};
use crate::types::{
    Account, AccountId, AccountType, BillStatus, Budget, BudgetId, BudgetLineId, ClaimStatus,
    CodedLine, DimensionId, FiscalPeriod, GrantId, IsoDate, LedgerState, LineId, PeriodId,
    SourceType,
};

/// `BudgetVsActualRow['status']` (`reports.ts:466`), from [`warning_level`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    /// `within`: under 80% used.
    Within,
    /// `warning`: 80% to 100% used.
    Warning,
    /// `over`: more than 100% used.
    Over,
}

/// The options of `budgetVsActual` (`reports.ts:477`). Ids are tested for truthiness, as in TS.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetVsActualOptions {
    /// Lines of this budget only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_id: Option<BudgetId>,
    /// Lines for this grant only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<GrantId>,
    /// Lines for this dimension value only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_id: Option<DimensionId>,
    /// Count spend up to this date at the latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<IsoDate>,
    /// Accrual (default) or the cash-basis view.
    #[serde(default)]
    pub basis: Basis,
}

/// `BudgetVsActualRow` (`reports.ts:457-467`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetVsActualRow {
    /// The budget line.
    pub line_id: BudgetLineId,
    /// Account (or `All expenses`), grant, dimension and period names joined by ` · `.
    pub label: String,
    /// The budget line's account; `None` (`null`) for all expense accounts or one not in the
    /// chart.
    pub account: Option<Account>,
    /// Budgeted amount.
    pub budget: Minor,
    /// Expense posted against it.
    pub actual: Minor,
    /// Budget minus actual.
    pub variance: Minor,
    /// Actual over budget; 0 with no budget and no spend, `Infinity` with spend and no budget.
    #[serde(with = "crate::js_number")]
    pub used: f64,
    /// [`warning_level`] of `used`.
    pub status: WarningLevel,
}

/// `GrantSummary` (`reports.ts:531-547`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantSummary {
    /// The grant.
    pub grant_id: GrantId,
    /// Award in minor units.
    pub award: Minor,
    /// Income received on the grant.
    pub received: Minor,
    /// Expense posted to the grant.
    pub spent: Minor,
    /// Approved bills not yet posted, and submitted claims, coded to the grant, in base currency.
    pub committed: Minor,
    /// Award minus spent minus committed.
    pub remaining: Minor,
    /// Spent plus committed over the award (0 with no award).
    #[serde(with = "crate::js_number")]
    pub used: f64,
    /// [`warning_level`] of `used`.
    pub status: WarningLevel,
    /// Average spend per elapsed month, rounded as `Math.round`.
    pub burn_per_month: Minor,
    /// Months of award left at the current burn; `None` (`null`) when not spending.
    #[serde(with = "crate::js_number::option")]
    pub months_left: Option<f64>,
    /// Months until the grant ends, not below 0.
    #[serde(with = "crate::js_number")]
    pub months_to_end: f64,
}

/// `LedgerSignals` (`reports.ts:590-597`): ledger inputs to the trust score, each 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSignals {
    /// Share of cash movement (by value) matched to a bank statement line.
    #[serde(with = "crate::js_number")]
    pub reconciled_share: f64,
    /// Share of budgeted spend whose budget line is not over.
    #[serde(with = "crate::js_number")]
    pub in_budget_share: f64,
    /// Share of spend coded to a grant.
    #[serde(with = "crate::js_number")]
    pub grant_coded_share: f64,
}

/// `warningLevel(used)` (`reports.ts:469-473`): over above 1, warning from 0.8, else within
/// (`NaN` is within, as in TS).
#[must_use]
pub fn warning_level(used: f64) -> WarningLevel {
    if used > 1.0 {
        WarningLevel::Over
    } else if used >= 0.8 {
        WarningLevel::Warning
    } else {
        WarningLevel::Within
    }
}

/// `budgetVsActual(state, opts)` (`reports.ts:475-529`): one row per budget line that passes the
/// filters, in book order.
///
/// A line's actual is the expense posted in its period (or its budget's fiscal year, from the
/// year's first period in book order to its last), capped at `opts.to`, closing entries left
/// out, on its account and everything below it (every expense account when it has none), its
/// fund, grant and dimension.
#[must_use]
pub fn budget_vs_actual(
    state: &LedgerState,
    opts: &BudgetVsActualOptions,
) -> Vec<BudgetVsActualRow> {
    let accounts = account_map(state);
    // `new Map(...)`: the last record with an id wins.
    let periods: HashMap<&PeriodId, &FiscalPeriod> =
        state.periods.iter().map(|p| (&p.id, p)).collect();
    let budgets: HashMap<&BudgetId, &Budget> = state.budgets.iter().map(|b| (&b.id, b)).collect();
    let expense_ids: Vec<AccountId> = state
        .accounts
        .iter()
        .filter(|a| a.account_type == AccountType::Expense)
        .map(|a| a.id.clone())
        .collect();
    let budget_id = truthy(opts.budget_id.as_ref());
    let grant_id = truthy(opts.grant_id.as_ref());
    let dimension_id = truthy(opts.dimension_id.as_ref());
    state
        .budget_lines
        .iter()
        .filter(|bl| {
            budget_id.is_none_or(|id| &bl.budget_id == id)
                && grant_id.is_none_or(|id| bl.grant_id.as_ref() == Some(id))
                && dimension_id.is_none_or(|id| bl.dimension_id.as_ref() == Some(id))
        })
        .map(|bl| {
            let budget = budgets.get(&bl.budget_id).copied();
            let period = truthy(bl.period_id.as_ref()).and_then(|id| periods.get(id).copied());
            let year: Vec<&FiscalPeriod> = state
                .periods
                .iter()
                .filter(|p| budget.is_some_and(|b| p.fiscal_year == b.fiscal_year))
                .collect();
            let from = period
                .map(|p| &p.start_date)
                .or_else(|| year.first().map(|p| &p.start_date));
            let mut to = period
                .map(|p| &p.end_date)
                .or_else(|| year.last().map(|p| &p.end_date));
            if let Some(cap) = opts.to.as_ref()
                && to.is_none_or(|t| cap < t)
            {
                to = Some(cap);
            }
            let account_id = truthy(bl.account_id.as_ref());
            let scope = Scope {
                from,
                to,
                account_ids: Some(account_id.map_or(expense_ids.as_slice(), slice::from_ref)),
                fund_id: truthy(bl.fund_id.as_ref()),
                grant_id: truthy(bl.grant_id.as_ref()),
                dimension_id: truthy(bl.dimension_id.as_ref()),
                basis: opts.basis,
                exclude_closing: true,
            };
            let actual = sum(posted(state, &scope)
                .iter()
                .filter(|p| is_type(&accounts, p, AccountType::Expense))
                .map(|p| p.line.base_amount));
            let account = account_id
                .and_then(|id| accounts.get(id))
                .map(|a| Account::clone(a));
            let used = if bl.amount > 0 {
                js(actual) / js(bl.amount)
            } else if actual > 0 {
                f64::INFINITY
            } else {
                0.0
            };
            let grant = truthy(bl.grant_id.as_ref())
                .and_then(|id| state.grants.iter().find(|g| &g.id == id));
            let dimension = truthy(bl.dimension_id.as_ref())
                .and_then(|id| state.dimensions.iter().find(|d| &d.id == id));
            let account_label = account.as_ref().map_or_else(
                || "All expenses".to_owned(),
                |a| format!("{} {}", a.code, a.name),
            );
            let label = [
                Some(account_label.as_str()),
                grant.map(|g| g.name.as_str()),
                dimension.map(|d| d.name.as_str()),
                period.map(|p| p.name.as_str()),
            ]
            .into_iter()
            .flatten()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" \u{b7} ");
            BudgetVsActualRow {
                line_id: bl.id.clone(),
                label,
                account,
                budget: bl.amount,
                actual,
                variance: bl.amount - actual,
                used,
                status: warning_level(used),
            }
        })
        .collect()
}

/// `grantSummary(state, grantId, asOf)` (`reports.ts:555-584`): what the grant has received and
/// spent up to `as_of` (closing entries left out), what is committed on approved bills and
/// submitted claims coded to it (converted to base currency at the document's rate), and its
/// burn rate over the months elapsed since it started (at least one).
///
/// # Errors
///
/// [`ReportError::UnknownGrant`] when there is no such grant; [`ReportError::InvalidDate`] when
/// `as_of` or one of the grant's dates is not a calendar date (TS returns `NaN` months);
/// [`ReportError::Money`] when a committed document's rate is refused by `convert`.
pub fn grant_summary(
    state: &LedgerState,
    grant_id: &GrantId,
    as_of: &IsoDate,
) -> Result<GrantSummary, ReportError> {
    let grant = state
        .grants
        .iter()
        .find(|g| &g.id == grant_id)
        .ok_or_else(|| ReportError::UnknownGrant(grant_id.clone()))?;
    let accounts = account_map(state);
    let lines = posted(
        state,
        &Scope {
            grant_id: truthy(Some(grant_id)),
            to: Some(as_of),
            exclude_closing: true,
            ..Scope::default()
        },
    );
    let total_of = |t: AccountType| {
        sum(lines
            .iter()
            .filter(|p| is_type(&accounts, p, t))
            .map(|p| p.line.base_amount))
    };
    let spent = total_of(AccountType::Expense);
    let received = -total_of(AccountType::Income);
    // Documents can be in another currency; commitments count in base.
    let coded_to = |lines: &[CodedLine], rate: f64| -> Result<Minor, MoneyError> {
        lines
            .iter()
            .filter(|l| l.grant_id.as_ref() == Some(grant_id))
            .map(|l| convert(l.amount, rate))
            .sum()
    };
    let mut committed = 0;
    for bill in state
        .bills
        .iter()
        .filter(|b| b.status == BillStatus::Approved)
    {
        committed += coded_to(&bill.lines, bill.fx_rate)?;
    }
    for claim in state
        .claims
        .iter()
        .filter(|c| c.status == ClaimStatus::Submitted)
    {
        committed += coded_to(&claim.lines, claim.fx_rate)?;
    }
    let used = if grant.award > 0 {
        js(spent + committed) / js(grant.award)
    } else {
        0.0
    };
    let until = if as_of < &grant.end_date {
        as_of
    } else {
        &grant.end_date
    };
    let elapsed = months_between(&grant.start_date, until)?.max(1.0);
    let burn_per_month = minor_of(js_round(js(spent) / elapsed));
    let months_left =
        (burn_per_month > 0).then(|| (js(grant.award - spent) / js(burn_per_month)).max(0.0));
    let months_to_end = months_between(as_of, &grant.end_date)?.max(0.0);
    Ok(GrantSummary {
        grant_id: grant_id.clone(),
        award: grant.award,
        received,
        spent,
        committed,
        remaining: grant.award - spent - committed,
        used,
        status: warning_level(used),
        burn_per_month,
        months_left,
        months_to_end,
    })
}

/// `ledgerSignals(state, asOf)` (`reports.ts:599-622`), up to `as_of` (all dates when `None`),
/// closing entries left out: the value of bank-account lines (opening balances left out) matched
/// to a statement line over all of it; the actual of budget lines not over budget over all
/// budgeted actual (1 when nothing is budgeted); expense coded to a grant over all expense.
#[must_use]
pub fn ledger_signals(state: &LedgerState, as_of: Option<&IsoDate>) -> LedgerSignals {
    let accounts = account_map(state);
    let lines = posted(
        state,
        &Scope {
            to: as_of,
            exclude_closing: true,
            ..Scope::default()
        },
    );
    let reconcilable: HashSet<&AccountId> =
        state.bank_accounts.iter().map(|b| &b.account_id).collect();
    let cash: Vec<&Posted<'_>> = lines
        .iter()
        .filter(|p| {
            reconcilable.contains(&p.line.account_id) && p.entry.source_type != SourceType::Opening
        })
        .collect();
    let matched: HashSet<&LineId> = state
        .statement_lines
        .iter()
        .filter_map(|s| truthy(s.matched_line_id.as_ref()))
        .collect();
    let cash_value = sum(cash.iter().map(|p| p.line.base_amount.abs()));
    let matched_value = sum(cash
        .iter()
        .filter(|p| matched.contains(&p.line.id))
        .map(|p| p.line.base_amount.abs()));

    let spend: Vec<&Posted<'_>> = lines
        .iter()
        .filter(|p| is_type(&accounts, p, AccountType::Expense))
        .collect();
    let spend_value = sum(spend.iter().map(|p| p.line.base_amount));
    let grant_value = sum(spend
        .iter()
        .filter(|p| truthy(p.line.grant_id.as_ref()).is_some())
        .map(|p| p.line.base_amount));

    let bva = budget_vs_actual(
        state,
        &BudgetVsActualOptions {
            to: as_of.cloned(),
            ..BudgetVsActualOptions::default()
        },
    );
    let budgeted = sum(bva.iter().map(|r| r.actual));
    let in_budget = sum(bva
        .iter()
        .filter(|r| r.status != WarningLevel::Over)
        .map(|r| r.actual));

    LedgerSignals {
        reconciled_share: share(matched_value, cash_value),
        in_budget_share: if budgeted > 0 {
            share(in_budget, budgeted)
        } else {
            1.0
        },
        grant_coded_share: share(grant_value, spend_value),
    }
}

/// `share(a, b)` (`reports.ts:616`): `a / b` within 0 to 1, or 0 when `b` is not positive.
fn share(a: Minor, b: Minor) -> f64 {
    if b > 0 {
        (js(a) / js(b)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// `accounts.get(l.account_id)?.type === t`.
fn is_type(accounts: &HashMap<&AccountId, &Account>, p: &Posted<'_>, t: AccountType) -> bool {
    accounts
        .get(&p.line.account_id)
        .is_some_and(|a| a.account_type == t)
}
