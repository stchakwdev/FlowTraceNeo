//! The Grants screen's view models, free of GPUI: the grants list with its totals and warnings,
//! one grant's detail, the budget-versus-actual view with its table and chart bars, and the
//! add-grant, add-line and change-line forms.
//!
//! Figures come from `ledger_core::reports::grant_summary` and `budget_vs_actual`
//! (`reports.ts:475-584`); wording, ordering and rounding follow the web screens
//! (`flowtrace/src/routes/_authenticated/grants/index.tsx`, `$id.tsx`, `budgets.tsx`).

use ledger_core::LedgerCommand;
use ledger_core::Minor;
use ledger_core::reports::{Basis, BudgetVsActualRow, GrantSummary, ReportError, WarningLevel};
use ledger_core::types::{
    AccountId, Budget, BudgetId, BudgetLine, DimensionId, EntryId, FundId, Grant, GrantId, IsoDate,
    LedgerState,
};

use crate::statements::{KeyFigure, Table};

/// `STATUS_TEXT` of the grants list (`grants/index.tsx`).
#[must_use]
pub fn grant_status_text(_level: WarningLevel) -> &'static str {
    ""
}

/// `STATUS_TEXT` of a budget line (`grants/budgets.tsx`, `BVA_STATUS` in `$id.tsx`).
#[must_use]
pub fn line_status_text(_level: WarningLevel) -> &'static str {
    ""
}

/// `Math.round((part / Math.max(1, award)) * 100)`.
#[must_use]
pub fn share_percent(_part: Minor, _award: Minor) -> i64 {
    0
}

/// `Math.round(used * 100)`, or `None` when `used` is not finite (spend with no budget).
#[must_use]
pub fn used_percent(_used: f64) -> Option<i64> {
    None
}

/// `months(n)` (`grants/$id.tsx`): one decimal below 10 without a trailing `.0`, whole above.
#[must_use]
pub fn months(_n: f64) -> String {
    String::new()
}

/// One grant of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantRow {
    /// The grant.
    pub grant: Grant,
    /// Its fund's name, `Fund` when the fund is not in the books.
    pub fund: String,
    /// `grantSummary` as at the screen's date.
    pub summary: GrantSummary,
}

impl GrantRow {
    /// `donor · fund · start to end`.
    #[must_use]
    pub fn subtitle(&self) -> String {
        String::new()
    }

    /// `Within award · 63% spent or committed`.
    #[must_use]
    pub fn status_line(&self) -> String {
        String::new()
    }

    /// `63% spent, 0% committed`, the coverage bar's label.
    #[must_use]
    pub fn coverage_label(&self) -> String {
        String::new()
    }
}

/// The grants as at `as_of`, most used first (`grants/index.tsx`, `rows`).
///
/// # Errors
///
/// The report's error when a grant's dates are off the calendar.
pub fn grant_rows(_state: &LedgerState, _as_of: &IsoDate) -> Result<Vec<GrantRow>, ReportError> {
    Ok(Vec::new())
}

/// The sums across all grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GrantTotals {
    /// Awards.
    pub award: Minor,
    /// Spent.
    pub spent: Minor,
    /// Committed.
    pub committed: Minor,
    /// Remaining.
    pub remaining: Minor,
}

/// The sums of `rows`.
#[must_use]
pub fn grant_totals(_rows: &[GrantRow]) -> GrantTotals {
    GrantTotals::default()
}

/// The key figures across all grants (`Across all grants`).
#[must_use]
pub fn totals_figures(_t: &GrantTotals, _base: &str) -> Vec<KeyFigure> {
    Vec::new()
}

/// Will the money last to the end date (`runway` in `grants/$id.tsx`)?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runway {
    /// Shown as a flag in signal orange.
    pub flag: bool,
    /// The sentence.
    pub text: String,
}

/// `runway(grant, s)` (`grants/$id.tsx`).
#[must_use]
pub fn runway(_grant: &Grant, _s: &GrantSummary) -> Runway {
    Runway {
        flag: false,
        text: String::new(),
    }
}

/// One posted line charged to a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantLine {
    /// The entry.
    pub entry_id: EntryId,
    /// Entry date.
    pub date: IsoDate,
    /// Entry number.
    pub entry_number: String,
    /// Account code.
    pub code: String,
    /// Account name.
    pub account: String,
    /// The line's memo, else the entry's.
    pub memo: String,
    /// `max(0, base_amount)`.
    pub debit: Minor,
    /// `max(0, -base_amount)`.
    pub credit: Minor,
}

/// One grant's page.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantDetail {
    /// The grant.
    pub grant: Grant,
    /// `donor · fund · start to end`.
    pub lede: String,
    /// `grantSummary` as at the screen's date.
    pub summary: GrantSummary,
    /// Awarded, Spent, Committed, Remaining.
    pub figures: Vec<KeyFigure>,
    /// Received, burn per month, months left, months to the end.
    pub more: Vec<KeyFigure>,
    /// `38% spent, 0% committed`.
    pub coverage_label: String,
    /// The runway sentence.
    pub runway: Runway,
    /// The terms, or `No terms recorded.`
    pub terms: String,
    /// The grant's budget lines against actual.
    pub budget: Vec<BudgetRow>,
    /// Posted lines charged to it, newest first.
    pub lines: Vec<GrantLine>,
}

/// The detail of grant `id` as at `as_of`.
///
/// # Errors
///
/// [`ReportError::UnknownGrant`] when the grant is not in the books.
pub fn grant_detail(
    _state: &LedgerState,
    id: &GrantId,
    _as_of: &IsoDate,
) -> Result<GrantDetail, ReportError> {
    Err(ReportError::UnknownGrant(id.clone()))
}

/// The three groups of the budget page (`groupOf` in `grants/budgets.tsx`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Lines for a grant.
    Grant,
    /// Unrestricted running costs.
    Running,
    /// Department allocations: a dimension and no account.
    Department,
}

impl Group {
    /// `GROUP_TITLE`.
    #[must_use]
    pub fn title(self) -> &'static str {
        ""
    }
}

/// One budget line against actual.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetRow {
    /// The report row.
    pub row: BudgetVsActualRow,
    /// The budget line.
    pub line: BudgetLine,
    /// Its group.
    pub group: Group,
    /// The grant's name for a grant line.
    pub sub: Option<String>,
}

impl BudgetRow {
    /// `Within budget · 74% used`, or `... · spent with no budget`.
    #[must_use]
    pub fn status_line(&self) -> String {
        String::new()
    }

    /// The Used column: `74%`, or an en dash with spend and no budget.
    #[must_use]
    pub fn used_cell(&self) -> String {
        String::new()
    }

    /// `Over by` or `Left`.
    #[must_use]
    pub fn variance_label(&self) -> &'static str {
        ""
    }
}

/// The budget page's choices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BudgetFilter {
    /// The budget; `None` for the default one.
    pub budget: Option<BudgetId>,
    /// Lines for this grant only.
    pub grant: Option<GrantId>,
    /// Lines for this dimension value only.
    pub dimension: Option<DimensionId>,
    /// Accrual or cash.
    pub basis: Basis,
}

/// The sums of the budget page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BudgetTotals {
    /// Budgeted.
    pub budget: Minor,
    /// Spent.
    pub actual: Minor,
    /// Budget minus actual.
    pub variance: Minor,
    /// Lines 80% used.
    pub warning: usize,
    /// Lines over budget.
    pub over: usize,
}

impl BudgetTotals {
    /// Spent over budgeted, as a whole percentage; 0 with nothing budgeted.
    #[must_use]
    pub fn used_percent(&self) -> i64 {
        0
    }
}

/// A bar of the budget chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarKind {
    /// The budgeted amount.
    Budget,
    /// The actual spend.
    Actual,
}

/// One bar of the budget chart: two per line, budget then actual.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// The band label, unique in the chart.
    pub band: String,
    /// Budget or actual.
    pub kind: BarKind,
    /// The amount in major units.
    pub major: f64,
    /// The line's status, for the actual bar's colour.
    pub status: WarningLevel,
}

/// The budget page.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetView {
    /// Budgets, latest fiscal year first.
    pub budgets: Vec<Budget>,
    /// The budget shown.
    pub budget: Budget,
    /// Its lines, grouped: grant, running, department.
    pub rows: Vec<BudgetRow>,
    /// Sums.
    pub totals: BudgetTotals,
}

impl BudgetView {
    /// The budget-versus-actual table: a heading per group, a row per line and a totals row.
    #[must_use]
    pub fn table(&self) -> Table {
        Table {
            title: None,
            first_column: String::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            empty: None,
        }
    }

    /// Budgeted, Spent, Lines 80% used, Lines over budget.
    #[must_use]
    pub fn figures(&self, _base: &str) -> Vec<KeyFigure> {
        Vec::new()
    }

    /// The chart's bars.
    #[must_use]
    pub fn bars(&self) -> Vec<Bar> {
        Vec::new()
    }
}

/// The budget page for `filter` as at `as_of`; `None` when the books hold no budget.
#[must_use]
pub fn budget_view(
    _state: &LedgerState,
    _filter: &BudgetFilter,
    _as_of: &IsoDate,
) -> Option<BudgetView> {
    None
}

/// What the add-grant form found wrong before sending.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantErrors {
    /// Under the name.
    pub name: Option<String>,
    /// Under the donor.
    pub donor: Option<String>,
    /// Under the fund.
    pub fund: Option<String>,
    /// Under the award.
    pub award: Option<String>,
    /// Under the start date.
    pub start: Option<String>,
    /// Under the end date.
    pub end: Option<String>,
}

/// The add-grant form.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewGrantForm {
    /// Name as typed.
    pub name: String,
    /// Donor as typed.
    pub donor: String,
    /// The fund.
    pub fund: Option<FundId>,
    /// Award as typed, in major units.
    pub award: String,
    /// Currency code as typed.
    pub currency: String,
    /// Start date as typed.
    pub start: String,
    /// End date as typed.
    pub end: String,
    /// Terms as typed.
    pub terms: String,
}

impl NewGrantForm {
    /// An empty form in the books' base currency.
    #[must_use]
    pub fn new(_state: &LedgerState) -> Self {
        Self::default()
    }

    /// The `grant.add` command, or what to fix first.
    ///
    /// # Errors
    ///
    /// [`GrantErrors`] for blank or malformed fields.
    pub fn command(&self) -> Result<LedgerCommand, GrantErrors> {
        Err(GrantErrors::default())
    }
}

/// The add-line form (`AddLine` in `grants/budgets.tsx`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetLineForm {
    /// The budget the line goes in.
    pub budget: BudgetId,
    /// An expense account, `None` for all.
    pub account: Option<AccountId>,
    /// A fund, `None` for any.
    pub fund: Option<FundId>,
    /// A grant, `None` for none.
    pub grant: Option<GrantId>,
    /// A dimension value, `None` for none.
    pub dimension: Option<DimensionId>,
    /// Amount as typed.
    pub amount: String,
}

impl BudgetLineForm {
    /// An empty form for `budget`.
    #[must_use]
    pub fn new(budget: BudgetId) -> Self {
        Self {
            budget,
            account: None,
            fund: None,
            grant: None,
            dimension: None,
            amount: String::new(),
        }
    }

    /// Choosing a fund drops a grant of another fund.
    pub fn set_fund(&mut self, _fund: Option<FundId>, _state: &LedgerState) {}

    /// The `budgetLine.set` command, or the amount's error.
    ///
    /// # Errors
    ///
    /// The message under the amount.
    pub fn command(&self) -> Result<LedgerCommand, String> {
        Err(String::new())
    }
}

/// The expense accounts a line can be set on, `code · name` in code order.
#[must_use]
pub fn expense_choices(_state: &LedgerState) -> Vec<(AccountId, String)> {
    Vec::new()
}

/// Changing one line's amount (the Change button of `grants/budgets.tsx`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEdit {
    /// The line as it was.
    pub line: BudgetLine,
    /// Its label.
    pub label: String,
    /// The amount as typed, prefilled with the budget to two places.
    pub value: String,
}

impl LineEdit {
    /// The edit of `row`.
    #[must_use]
    pub fn new(row: &BudgetRow) -> Self {
        Self {
            line: row.line.clone(),
            label: row.row.label.clone(),
            value: String::new(),
        }
    }

    /// The `budgetLine.set` command with the line's fields and the new amount, and the note
    /// shown once it is saved; or the amount's error.
    ///
    /// # Errors
    ///
    /// The message under the field.
    pub fn command(&self, _base: &str) -> Result<(LedgerCommand, String), String> {
        Err(String::new())
    }
}
