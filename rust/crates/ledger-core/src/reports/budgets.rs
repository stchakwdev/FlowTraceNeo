//! Budget versus actual, grant summaries and the trust-score signals (`reports.ts:453-622`).

use serde::{Deserialize, Serialize};

use super::{Basis, ReportError};
use crate::money::Minor;
use crate::types::{Account, BudgetId, BudgetLineId, DimensionId, GrantId, IsoDate, LedgerState};

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
    let _ = used;
    todo!("reports.ts:469-473")
}

/// `budgetVsActual(state, opts)` (`reports.ts:475-529`): one row per budget line that passes the
/// filters, in book order.
#[must_use]
pub fn budget_vs_actual(
    state: &LedgerState,
    opts: &BudgetVsActualOptions,
) -> Vec<BudgetVsActualRow> {
    let _ = (state, opts);
    todo!("reports.ts:475-529")
}

/// `grantSummary(state, grantId, asOf)` (`reports.ts:555-584`).
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
    let _ = (state, grant_id, as_of);
    todo!("reports.ts:555-584")
}

/// `ledgerSignals(state, asOf)` (`reports.ts:599-622`).
#[must_use]
pub fn ledger_signals(state: &LedgerState, as_of: Option<&IsoDate>) -> LedgerSignals {
    let _ = (state, as_of);
    todo!("reports.ts:599-622")
}
