//! Reports over posted journal lines, mirroring `flowtrace/src/lib/ledger/reports.ts`.
//!
//! Every report reads `&LedgerState` only: no env, no writes. They read posted lines (reversed
//! entries count, offset by their reversals), and every one of them foots: the trial balance
//! nets to zero, the Statement of Financial Position balances, and the Statement of Activities'
//! change in net assets equals the change on the balance sheet (`reports.ts:1-4`).
//!
//! Outputs are owned values with the TS JSON shape: money in integer minor units, ratios and
//! months as `f64` written through [`crate::js_number`], keys camelCase where TS uses camelCase
//! (`asOf`, `totalAssets`) and records ([`Account`], [`JournalLine`]) in their snake_case form.
//! Sorting by account code or entry number is byte order, which is what `localeCompare` gives on
//! these keys (`rust/fixtures/README.md`, "Formatting notes").
//!
//! Where TS throws (a date that is not on the calendar, an unknown grant, an exchange rate
//! `convert` refuses), the Rust function returns [`ReportError`]. Where TS would read a field of
//! a missing account and throw a `TypeError` (a line on an account not in the chart, which the
//! engine never writes), the line is left out.

mod budgets;
mod statements;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::money::{Minor, MoneyError};
use crate::types::{
    Account, AccountId, DimensionId, FundId, GrantId, IsoDate, JournalEntry, JournalLine,
    LedgerState, NormalBalance,
};

pub use budgets::{
    BudgetVsActualOptions, BudgetVsActualRow, GrantSummary, LedgerSignals, WarningLevel,
    budget_vs_actual, grant_summary, ledger_signals, warning_level,
};
pub use statements::{
    Activities, ActivityRow, CashFlow, FinancialPosition, FunctionalExpenses, FunctionalRow,
    FunctionalTotals, FundActivity, GeneralLedger, GeneralLedgerRow, NetAssets, OperatingCash,
    RestrictionTotals, StatementRow, TrialBalance, TrialBalanceRow, activities, cash_flow,
    financial_position, functional_expenses, general_ledger, trial_balance,
};

/// `Basis` (`reports.ts:18`): the books are accrual; `Cash` is the cash-basis view of them.
///
/// Not [`crate::types::Basis`], which is the books' own setting and has only `accrual`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// `accrual`, the default.
    #[default]
    Accrual,
    /// `cash`: accruals dropped, settlements re-coded to the expense lines they paid.
    Cash,
}

/// `LineFilter` (`reports.ts:20-30`): which posted lines a report reads.
///
/// TS tests each filter for truthiness, so an empty id or an absent one means no filter; an
/// empty `account_ids` list (truthy in TS) matches no account.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineFilter {
    /// First date, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<IsoDate>,
    /// Last date, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<IsoDate>,
    /// These accounts and every account below them in the tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_ids: Option<Vec<AccountId>>,
    /// Lines charged to this fund.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fund_id: Option<FundId>,
    /// Lines charged to this grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_id: Option<GrantId>,
    /// Lines tagged with this dimension value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_id: Option<DimensionId>,
    /// Accrual (default) or the cash-basis view.
    #[serde(default)]
    pub basis: Basis,
    /// Leave out the year-end closing entries (for activity reports).
    #[serde(default)]
    pub exclude_closing: bool,
}

/// `ReportLine` (`reports.ts:32-34`): a posted line with its entry, written as the line's fields
/// plus `entry`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportLine {
    /// The line. On a cash basis a re-coded settlement line has the id `<line>:<coded line>`.
    #[serde(flatten)]
    pub line: JournalLine,
    /// Its entry.
    pub entry: JournalEntry,
}

/// What a report cannot answer, where the TS function throws.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReportError {
    /// A date whose month is outside 1-12 or whose day is outside 1-31, or a result before year
    /// 0000. TS builds an `Invalid Date` from it (`reports.ts:202`, `reports.ts:550-551`):
    /// `dayBefore` throws a `RangeError` and `grantSummary` would return `NaN` months.
    #[error("{0} is not a calendar date.")]
    InvalidDate(IsoDate),
    /// `grantSummary` for a grant that does not exist; TS throws a `TypeError` reading its award
    /// (`reports.ts:556`).
    #[error("Grant {0} does not exist.")]
    UnknownGrant(GrantId),
    /// `convert` refused a document's rate or amount (`reports.ts:563`, `money.ts:36-51`).
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// `descendants(state, accountId)` (`reports.ts:39-52`): the account plus every account below it
/// in the tree, in the order TS adds them to its `Set` (the account first, then pass by pass in
/// chart order). An unknown id gives just itself.
#[must_use]
pub fn descendants(state: &LedgerState, account_id: &AccountId) -> Vec<AccountId> {
    let _ = (state, account_id);
    todo!("reports.ts:39-52")
}

/// `postedLines(state, f)` (`reports.ts:54-82`): lines of every entry that is not a draft
/// (reversed entries count, offset by their reversals), in book order, then filtered by `f`.
///
/// With [`Basis::Cash`], accruals are dropped and each bill payment or claim reimbursement is
/// re-coded to the expense lines it settled, pro rata (`reports.ts:84-139`).
#[must_use]
pub fn posted_lines(state: &LedgerState, f: &LineFilter) -> Vec<ReportLine> {
    let _ = (state, f);
    todo!("reports.ts:59-82")
}

/// `balances(state, f)` (`reports.ts:141-146`): signed debit-minus-credit balance per account in
/// base currency, for every account with a line in `f` (a zero balance included).
#[must_use]
pub fn balances(state: &LedgerState, f: &LineFilter) -> BTreeMap<AccountId, Minor> {
    let _ = (state, f);
    todo!("reports.ts:142-146")
}

/// `natural(a, signed)` (`reports.ts:148-150`): a balance shown the natural way round, credit
/// accounts positive.
#[must_use]
pub fn natural(normal_balance: NormalBalance, signed: Minor) -> Minor {
    let _ = (normal_balance, signed);
    todo!("reports.ts:149-150")
}

/// `dayBefore(d)` (`reports.ts:201-205`): the previous calendar day, in UTC.
///
/// # Errors
///
/// [`ReportError::InvalidDate`] for a date that is not on the calendar (see
/// [`ReportError::InvalidDate`]) and for `0000-01-01`, whose day before TS cannot print as a date.
pub fn day_before(d: &IsoDate) -> Result<IsoDate, ReportError> {
    let _ = d;
    todo!("reports.ts:201-205")
}

/// A posted line borrowed from the books, or owned when the cash basis re-coded it.
#[derive(Debug, Clone)]
struct Posted<'a> {
    line: Cow<'a, JournalLine>,
    entry: &'a JournalEntry,
}

/// A [`LineFilter`] borrowed, with its truthiness applied.
#[derive(Debug, Clone, Copy, Default)]
struct Scope<'f> {
    from: Option<&'f IsoDate>,
    to: Option<&'f IsoDate>,
    account_ids: Option<&'f [AccountId]>,
    fund_id: Option<&'f FundId>,
    grant_id: Option<&'f GrantId>,
    dimension_id: Option<&'f DimensionId>,
    basis: Basis,
    exclude_closing: bool,
}

/// `new Map(state.accounts.map((a) => [a.id, a]))`: the last account with an id wins, as in a
/// JS `Map` built from the chart.
fn account_map(state: &LedgerState) -> HashMap<&AccountId, &Account> {
    state.accounts.iter().map(|a| (&a.id, a)).collect()
}

/// `restrictedFunds(state)` (`reports.ts:234-236`): every fund that is not unrestricted.
fn restricted_funds(state: &LedgerState) -> HashSet<&FundId> {
    let _ = state;
    todo!("reports.ts:234-236")
}
