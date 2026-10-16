//! The Grants screen's view models, free of GPUI: the grants list with its totals and warnings,
//! one grant's detail, the budget-versus-actual view with its table and chart bars, and the
//! add-grant, add-line and change-line forms.
//!
//! Figures come from `ledger_core::reports::grant_summary` and `budget_vs_actual`
//! (`reports.ts:475-584`); wording, ordering and rounding follow the web screens
//! (`flowtrace/src/routes/_authenticated/grants/index.tsx`, `$id.tsx`, `budgets.tsx`). The web
//! has no add-grant form; that form's checks are this screen's own, and the engine's refusals
//! (`engine.ts:239-251`, `addGrant`) show as given.

use std::cmp::Ordering;

use ledger_core::LedgerCommand;
use ledger_core::Minor;
use ledger_core::engine::{NewBudgetLine, NewGrant};
use ledger_core::money::{js_round, parse_money};
use ledger_core::reports::{
    self, Basis, BudgetVsActualOptions, BudgetVsActualRow, GrantSummary, LineFilter, ReportError,
    WarningLevel,
};
use ledger_core::types::{
    AccountId, AccountType, Budget, BudgetId, BudgetLine, DimensionId, EntryId, FundId, Grant,
    GrantId, IsoDate, LedgerState, Nullish,
};

use crate::general_ledger::GlFilter;
use crate::statements::{
    Cell, KeyFigure, Row, RowKind, Table, format_compact, format_date, to_fixed,
};

/// The separator the web screens put between parts of a line.
const DOT: &str = " \u{b7} ";

/// The en dash the web shows for a missing figure.
const DASH: &str = "\u{2013}";

/// `STATUS_TEXT` of the grants list (`grants/index.tsx`).
#[must_use]
pub fn grant_status_text(level: WarningLevel) -> &'static str {
    match level {
        WarningLevel::Within => "Within award",
        WarningLevel::Warning => "80% used",
        WarningLevel::Over => "Over award",
    }
}

/// `STATUS_TEXT` of a budget line (`grants/budgets.tsx`, `BVA_STATUS` in `$id.tsx`).
#[must_use]
pub fn line_status_text(level: WarningLevel) -> &'static str {
    match level {
        WarningLevel::Within => "Within budget",
        WarningLevel::Warning => "80% or more used",
        WarningLevel::Over => "Over budget",
    }
}

/// `Math.round(x)` as a whole number for display.
#[expect(
    clippy::cast_possible_truncation,
    reason = "percentages and month counts of safe amounts, far inside i64"
)]
fn rounded(x: f64) -> i64 {
    js_round(x) as i64
}

/// A safe amount as the double TS holds.
#[expect(clippy::cast_precision_loss, reason = "a JS number, as TS holds it")]
fn js(n: Minor) -> f64 {
    n as f64
}

/// `Math.round((part / Math.max(1, award)) * 100)` (`grants/index.tsx`, `pct` in `$id.tsx`).
#[must_use]
pub fn share_percent(part: Minor, award: Minor) -> i64 {
    rounded(js(part) / js(award.max(1)) * 100.0)
}

/// `Math.round(used * 100)`, or `None` when `used` is not finite (spend with no budget,
/// `Number.isFinite(r.used)` in `grants/budgets.tsx`).
#[must_use]
pub fn used_percent(used: f64) -> Option<i64> {
    used.is_finite().then(|| rounded(used * 100.0))
}

/// `months(n)` (`grants/$id.tsx`): one decimal below 10 without a trailing `.0`, whole from
/// 10, and `month` for exactly 1.
#[must_use]
pub fn months(n: f64) -> String {
    let figure = if n < 10.0 {
        let fixed = to_fixed(n, 1);
        fixed
            .strip_suffix(".0")
            .map_or(fixed.clone(), str::to_owned)
    } else {
        rounded(n).to_string()
    };
    #[expect(clippy::float_cmp, reason = "n === 1, as the web compares")]
    let one = n == 1.0;
    format!("{figure} {}", if one { "month" } else { "months" })
}

/// `start to end`, both as `formatDate`.
fn span(grant: &Grant) -> String {
    format!(
        "{} to {}",
        format_date(&grant.start_date),
        format_date(&grant.end_date)
    )
}

/// The name of `grant`'s fund, `Fund` when it is not in the books (`fund?.name ?? 'Fund'`).
fn fund_name(state: &LedgerState, grant: &Grant) -> String {
    state
        .fund(&grant.fund_id)
        .map_or_else(|| "Fund".to_owned(), |f| f.name.clone())
}

/// `donor · fund · start to end` (`grants/index.tsx` row, `$id.tsx` lede).
fn lede(grant: &Grant, fund: &str) -> String {
    format!("{}{DOT}{fund}{DOT}{}", grant.donor_name, span(grant))
}

fn figure(value: String, label: impl Into<String>) -> KeyFigure {
    KeyFigure {
        value,
        label: label.into(),
    }
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
        lede(&self.grant, &self.fund)
    }

    /// `Within award · 63% spent or committed`.
    #[must_use]
    pub fn status_line(&self) -> String {
        format!(
            "{}{DOT}{}% spent or committed",
            grant_status_text(self.summary.status),
            rounded(self.summary.used * 100.0)
        )
    }

    /// `63% spent, 0% committed`, the coverage bar's label.
    #[must_use]
    pub fn coverage_label(&self) -> String {
        coverage_label(&self.summary)
    }
}

fn coverage_label(s: &GrantSummary) -> String {
    format!(
        "{}% spent, {}% committed",
        share_percent(s.spent, s.award),
        share_percent(s.committed, s.award)
    )
}

/// The grants as at `as_of`, most used first; equal ones keep book order, as the web's stable
/// `sort((a, b) => b.s.used - a.s.used)` does (`grants/index.tsx`, `rows`).
///
/// # Errors
///
/// The report's error when a grant's dates are off the calendar.
pub fn grant_rows(state: &LedgerState, as_of: &IsoDate) -> Result<Vec<GrantRow>, ReportError> {
    let mut rows = state
        .grants
        .iter()
        .map(|g| {
            Ok(GrantRow {
                grant: g.clone(),
                fund: fund_name(state, g),
                summary: reports::grant_summary(state, &g.id, as_of)?,
            })
        })
        .collect::<Result<Vec<_>, ReportError>>()?;
    rows.sort_by(|a, b| {
        b.summary
            .used
            .partial_cmp(&a.summary.used)
            .unwrap_or(Ordering::Equal)
    });
    Ok(rows)
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

/// The sums of `rows` (`totals` in `grants/index.tsx`).
#[must_use]
pub fn grant_totals(rows: &[GrantRow]) -> GrantTotals {
    rows.iter()
        .fold(GrantTotals::default(), |t, r| GrantTotals {
            award: t.award + r.summary.award,
            spent: t.spent + r.summary.spent,
            committed: t.committed + r.summary.committed,
            remaining: t.remaining + r.summary.remaining,
        })
}

/// The key figures across all grants (`Across all grants`).
#[must_use]
pub fn totals_figures(t: &GrantTotals, base: &str) -> Vec<KeyFigure> {
    vec![
        figure(format_compact(t.award), format!("Awarded, {base}")),
        figure(format_compact(t.spent), "Spent"),
        figure(format_compact(t.committed), "Committed"),
        figure(format_compact(t.remaining), "Remaining"),
    ]
}

/// Will the money last to the end date (`runway` in `grants/$id.tsx`)?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runway {
    /// Shown as a flag in signal orange.
    pub flag: bool,
    /// The sentence.
    pub text: String,
}

/// `runway(grant, s)` (`grants/$id.tsx`): ended; no spend to project from; runs out more than a
/// month early; or more than three months and half again late, with the unspent share.
#[must_use]
pub fn runway(grant: &Grant, s: &GrantSummary) -> Runway {
    let end = format_date(&grant.end_date);
    let say = |flag: bool, text: String| Runway { flag, text };
    if s.months_to_end <= 0.0 {
        return say(false, format!("The grant ended on {end}."));
    }
    let Some(left) = s.months_left else {
        return say(
            false,
            "No spend is posted yet, so there is no rate to project from.".to_owned(),
        );
    };
    let gap = left - s.months_to_end;
    if gap < -1.0 {
        return say(
            true,
            format!(
                "At this rate the award runs out in about {}, {} before the grant ends on {end}.",
                months(left),
                months(-gap)
            ),
        );
    }
    if left > s.months_to_end * 1.5 && gap > 3.0 {
        let unspent = (s.award - s.spent - rounded(js(s.burn_per_month) * s.months_to_end)).max(0);
        let pct = if s.award > 0 {
            rounded(js(unspent) / js(s.award) * 100.0)
        } else {
            0
        };
        return say(
            true,
            format!(
                "At this rate about {pct}% of the award ({}) is still unspent when the grant \
                 ends on {end}.",
                format_compact(unspent)
            ),
        );
    }
    say(
        false,
        format!("At this rate the award lasts to about the end date, {end}."),
    )
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

/// The detail of grant `id` as at `as_of` (`GrantDetail` in `grants/$id.tsx`): the summary's
/// figures, the runway, the grant's budget lines (every budget, up to `as_of`) and its posted
/// lines up to `as_of`, closing entries left out, newest first. The lines go through the
/// General ledger's filter (`GlFilter` with the grant and `to`).
///
/// # Errors
///
/// [`ReportError::UnknownGrant`] when the grant is not in the books, and the summary's other
/// errors.
pub fn grant_detail(
    state: &LedgerState,
    id: &GrantId,
    as_of: &IsoDate,
) -> Result<GrantDetail, ReportError> {
    let grant = state
        .grant(id)
        .ok_or_else(|| ReportError::UnknownGrant(id.clone()))?;
    let s = reports::grant_summary(state, id, as_of)?;
    let fund = fund_name(state, grant);
    let figures = vec![
        figure(
            format_compact(s.award),
            format!("Awarded, {}", grant.currency),
        ),
        figure(format_compact(s.spent), "Spent"),
        figure(format_compact(s.committed), "Committed"),
        figure(format_compact(s.remaining), "Remaining"),
    ];
    let more = vec![
        figure(format_compact(s.received), "Received from the donor"),
        figure(
            format_compact(s.burn_per_month),
            "Spent per month, on average",
        ),
        figure(
            s.months_left.map_or_else(|| DASH.to_owned(), months),
            "Money lasts, at that rate",
        ),
        figure(
            months(js_round(s.months_to_end * 10.0) / 10.0),
            "Until the grant ends",
        ),
    ];
    let budget = budget_rows(
        state,
        &BudgetVsActualOptions {
            grant_id: Some(id.clone()),
            to: Some(as_of.clone()),
            ..BudgetVsActualOptions::default()
        },
    );
    let filter = LineFilter {
        exclude_closing: true,
        ..GlFilter {
            grant: Some(id.clone()),
            to: Some(as_of.clone()),
            ..GlFilter::default()
        }
        .to_line_filter(state)
    };
    let mut lines: Vec<GrantLine> = reports::posted_lines(state, &filter)
        .into_iter()
        .map(|l| {
            let account = state.account(&l.line.account_id);
            GrantLine {
                entry_id: l.entry.id,
                date: l.entry.date,
                entry_number: l.entry.number,
                code: account.map(|a| a.code.clone()).unwrap_or_default(),
                account: account.map_or_else(|| "Account".to_owned(), |a| a.name.clone()),
                memo: l.line.memo.unwrap_or(l.entry.memo),
                debit: l.line.base_amount.max(0),
                credit: (-l.line.base_amount).max(0),
            }
        })
        .collect();
    // b.entry.date.localeCompare(a.entry.date) || b.entry.number.localeCompare(a.entry.number);
    // byte order equals localeCompare on ISO dates and entry numbers.
    lines.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| b.entry_number.cmp(&a.entry_number))
    });
    Ok(GrantDetail {
        grant: grant.clone(),
        lede: lede(grant, &fund),
        coverage_label: coverage_label(&s),
        runway: runway(grant, &s),
        terms: if grant.terms.is_empty() {
            "No terms recorded.".to_owned()
        } else {
            grant.terms.clone()
        },
        summary: s,
        figures,
        more,
        budget,
        lines,
    })
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
    /// The groups in page order.
    pub const ALL: [Self; 3] = [Self::Grant, Self::Running, Self::Department];

    /// `GROUP_TITLE`.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Grant => "Grant budgets",
            Self::Running => "Unrestricted running costs",
            Self::Department => "Department allocations",
        }
    }

    /// `groupOf(l)`: a grant line, a department allocation, or a running cost.
    #[must_use]
    pub fn of(line: &BudgetLine) -> Self {
        if line
            .grant_id
            .as_ref()
            .is_some_and(|g| !g.as_str().is_empty())
        {
            Self::Grant
        } else if line
            .dimension_id
            .as_ref()
            .is_some_and(|d| !d.as_str().is_empty())
            && line
                .account_id
                .as_ref()
                .is_none_or(|a| a.as_str().is_empty())
        {
            Self::Department
        } else {
            Self::Running
        }
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
        let used = used_percent(self.row.used).map_or_else(
            || "spent with no budget".to_owned(),
            |p| format!("{p}% used"),
        );
        format!("{}{DOT}{used}", line_status_text(self.row.status))
    }

    /// The Used column: `74%`, or an en dash with spend and no budget (the web's CSV leaves
    /// it empty).
    #[must_use]
    pub fn used_cell(&self) -> String {
        used_percent(self.row.used).map_or_else(|| DASH.to_owned(), |p| format!("{p}%"))
    }

    /// `Over by` or `Left` (`r.variance < 0`).
    #[must_use]
    pub fn variance_label(&self) -> &'static str {
        if self.row.variance < 0 {
            "Over by"
        } else {
            "Left"
        }
    }
}

/// `budgetVsActual` with each row's line, group and grant name (`rows` in `budgets.tsx`), in
/// report order. A row whose line is gone (it cannot be) is skipped.
fn budget_rows(state: &LedgerState, opts: &BudgetVsActualOptions) -> Vec<BudgetRow> {
    reports::budget_vs_actual(state, opts)
        .into_iter()
        .filter_map(|row| {
            let line = state
                .budget_lines
                .iter()
                .rev()
                .find(|l| l.id == row.line_id)?
                .clone();
            let group = Group::of(&line);
            let sub = (group == Group::Grant).then(|| {
                line.grant_id
                    .as_ref()
                    .and_then(|g| state.grant(g))
                    .map_or_else(|| "Grant".to_owned(), |g| g.name.clone())
            });
            Some(BudgetRow {
                row,
                line,
                group,
                sub,
            })
        })
        .collect()
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
    /// Spent over budgeted, as a whole percentage; 0 with nothing budgeted (`budgets.tsx`).
    #[must_use]
    pub fn used_percent(&self) -> i64 {
        if self.budget > 0 {
            rounded(js(self.actual) / js(self.budget) * 100.0)
        } else {
            0
        }
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
    /// Its lines, grouped: grant (by grant, in order of first line), running, department.
    pub rows: Vec<BudgetRow>,
    /// Sums.
    pub totals: BudgetTotals,
}

impl BudgetView {
    /// The budget-versus-actual table: a heading per group, a row per line with its status
    /// under the label, and a totals row.
    #[must_use]
    pub fn table(&self) -> Table {
        let mut rows = Vec::new();
        for group in Group::ALL {
            let mut lines = self.rows.iter().filter(|r| r.group == group).peekable();
            if lines.peek().is_none() {
                continue;
            }
            rows.push(row(RowKind::Heading, group.title(), Vec::new()));
            rows.extend(lines.map(|r| Row {
                detail: Some(r.status_line()),
                ..row(
                    RowKind::Line,
                    &r.row.label,
                    vec![
                        Cell::Money(r.row.budget),
                        Cell::Money(r.row.actual),
                        Cell::Money(r.row.variance),
                        Cell::Text(r.used_cell()),
                    ],
                )
            }));
        }
        let t = &self.totals;
        rows.push(row(
            RowKind::Total,
            "Total",
            vec![
                Cell::Money(t.budget),
                Cell::Money(t.actual),
                Cell::Money(t.variance),
                Cell::Text(format!("{}%", t.used_percent())),
            ],
        ));
        let empty = self.rows.is_empty();
        Table {
            title: None,
            first_column: "Line".to_owned(),
            columns: ["Budget", "Actual", "Variance", "Used"]
                .map(str::to_owned)
                .to_vec(),
            rows: if empty { Vec::new() } else { rows },
            empty: empty.then(|| {
                "No budget lines match. Clear the filters to see the whole budget.".to_owned()
            }),
        }
    }

    /// Budgeted, Spent, Lines 80% used, Lines over budget (`budgets.tsx` key figures).
    #[must_use]
    pub fn figures(&self, base: &str) -> Vec<KeyFigure> {
        let t = &self.totals;
        vec![
            figure(format_compact(t.budget), format!("Budgeted, {base}")),
            figure(format_compact(t.actual), "Spent"),
            figure(t.warning.to_string(), "Lines 80% used"),
            figure(t.over.to_string(), "Lines over budget"),
        ]
    }

    /// The chart's bars: per line, its budget then its actual, labelled by line number and
    /// account code (`all` for every expense account).
    #[must_use]
    pub fn bars(&self) -> Vec<Bar> {
        self.rows
            .iter()
            .enumerate()
            .flat_map(|(i, r)| {
                let code = r
                    .row
                    .account
                    .as_ref()
                    .map_or_else(|| "all".to_owned(), |a| a.code.clone());
                let bar = |kind: BarKind, amount: Minor, word: &str| Bar {
                    band: format!("{} {code} {word}", i + 1),
                    kind,
                    major: js(amount) / 100.0,
                    status: r.row.status,
                };
                [
                    bar(BarKind::Budget, r.row.budget, "budget"),
                    bar(BarKind::Actual, r.row.actual, "actual"),
                ]
            })
            .collect()
    }
}

fn row(kind: RowKind, label: &str, cells: Vec<Cell>) -> Row {
    Row {
        kind,
        code: None,
        label: label.to_owned(),
        detail: None,
        cells,
    }
}

/// The budget page for `filter` as at `as_of`; `None` when the books hold no budget
/// (`No budget set up` in `budgets.tsx`). The budget is the chosen one, else the one for
/// `as_of`'s year, else the latest.
#[must_use]
pub fn budget_view(
    state: &LedgerState,
    filter: &BudgetFilter,
    as_of: &IsoDate,
) -> Option<BudgetView> {
    let mut budgets = state.budgets.clone();
    budgets.sort_by(|a, b| b.fiscal_year.cmp(&a.fiscal_year));
    let year: i32 = as_of.as_str().get(..4)?.parse().ok()?;
    let budget = filter
        .budget
        .as_ref()
        .and_then(|id| budgets.iter().find(|b| &b.id == id))
        .or_else(|| budgets.iter().find(|b| b.fiscal_year == year))
        .or_else(|| budgets.first())?
        .clone();
    let rows = budget_rows(
        state,
        &BudgetVsActualOptions {
            budget_id: Some(budget.id.clone()),
            grant_id: filter.grant.clone(),
            dimension_id: filter.dimension.clone(),
            to: Some(as_of.clone()),
            basis: filter.basis,
        },
    );
    // `grouped`: grant lines by grant in order of first appearance, then running, then
    // department.
    let mut ordered: Vec<BudgetRow> = Vec::with_capacity(rows.len());
    for group in Group::ALL {
        let mut subs: Vec<Option<String>> = Vec::new();
        for r in rows.iter().filter(|r| r.group == group) {
            if !subs.contains(&r.sub) {
                subs.push(r.sub.clone());
            }
        }
        for sub in subs {
            ordered.extend(
                rows.iter()
                    .filter(|r| r.group == group && r.sub == sub)
                    .cloned(),
            );
        }
    }
    let budget_sum: Minor = ordered.iter().map(|r| r.row.budget).sum();
    let actual: Minor = ordered.iter().map(|r| r.row.actual).sum();
    let count = |level: WarningLevel| ordered.iter().filter(|r| r.row.status == level).count();
    let totals = BudgetTotals {
        budget: budget_sum,
        actual,
        variance: budget_sum - actual,
        warning: count(WarningLevel::Warning),
        over: count(WarningLevel::Over),
    };
    Some(BudgetView {
        budgets,
        budget,
        rows: ordered,
        totals,
    })
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

/// The add-grant form. It checks only that each field is there and well formed; the engine
/// checks the rest (a positive award, the fund, the end after the start) and its message shows.
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
    pub fn new(state: &LedgerState) -> Self {
        Self {
            currency: state.settings.base_currency.clone(),
            ..Self::default()
        }
    }

    /// The `grant.add` command (`commands.ts`, `{ type: 'grant.add'; input: Omit<Grant, 'id'> }`),
    /// or what to fix first.
    ///
    /// # Errors
    ///
    /// [`GrantErrors`] for blank or malformed fields.
    pub fn command(&self) -> Result<LedgerCommand, GrantErrors> {
        let blank = |s: &str, message: &str| s.trim().is_empty().then(|| message.to_owned());
        let date = |s: &str| IsoDate::parse(s.trim()).ok();
        let award = parse_money(&self.award);
        let date_error = |s: &str| {
            date(s)
                .is_none()
                .then(|| "Enter a date as YYYY-MM-DD.".to_owned())
        };
        let errors = GrantErrors {
            name: blank(&self.name, "Enter the grant name."),
            donor: blank(&self.donor, "Enter the donor's name."),
            fund: self
                .fund
                .is_none()
                .then(|| "Choose the fund the grant sits in.".to_owned()),
            award: if self.award.trim().is_empty() {
                Some("Enter the award.".to_owned())
            } else {
                award
                    .is_none()
                    .then(|| "Enter an amount, like 250000.".to_owned())
            },
            start: date_error(&self.start),
            end: date_error(&self.end),
        };
        let (Some(fund_id), Some(award), Some(start_date), Some(end_date)) =
            (self.fund.clone(), award, date(&self.start), date(&self.end))
        else {
            return Err(errors);
        };
        if errors != GrantErrors::default() {
            return Err(errors);
        }
        let currency = self.currency.trim();
        Ok(LedgerCommand::GrantAdd {
            input: NewGrant {
                id: None,
                name: self.name.trim().to_owned(),
                donor_node_id: None,
                donor_name: self.donor.trim().to_owned(),
                fund_id,
                award: js(award),
                currency: currency.to_owned(),
                start_date,
                end_date,
                terms: self.terms.trim().to_owned(),
            },
        })
    }
}

/// `value || null`: a chosen id, or `null`.
fn or_null<T>(value: Option<T>) -> Nullish<T> {
    value.map_or(Nullish::Null, Nullish::Value)
}

/// The amount check of the add and change forms (`budgets.tsx`): money of zero or more.
fn budget_amount(raw: &str) -> Result<Minor, String> {
    parse_money(raw)
        .filter(|m| *m >= 0)
        .ok_or_else(|| "Enter an amount of zero or more, like 250000.".to_owned())
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

    /// Choosing a fund drops a grant of another fund (the Fund select's `onValueChange`).
    pub fn set_fund(&mut self, fund: Option<FundId>, state: &LedgerState) {
        if let (Some(f), Some(g)) = (&fund, &self.grant)
            && state.grant(g).map(|g| &g.fund_id) != Some(f)
        {
            self.grant = None;
        }
        self.fund = fund;
    }

    /// The `budgetLine.set` command for a new whole-year line, or the amount's error
    /// (`submit` in `AddLine`).
    ///
    /// # Errors
    ///
    /// The message under the amount.
    pub fn command(&self) -> Result<LedgerCommand, String> {
        if self.amount.trim().is_empty() {
            return Err("Enter the budgeted amount.".to_owned());
        }
        let amount = budget_amount(&self.amount)?;
        Ok(LedgerCommand::BudgetLineSet {
            input: NewBudgetLine {
                id: None,
                budget_id: self.budget.clone(),
                account_id: or_null(self.account.clone()),
                fund_id: or_null(self.fund.clone()),
                grant_id: or_null(self.grant.clone()),
                dimension_id: or_null(self.dimension.clone()),
                period_id: Nullish::Null,
                amount: js(amount),
            },
        })
    }
}

/// The expense accounts a line can be set on: active posting expense accounts in code order,
/// as `code · name` (`expenseOptions` in `AddLine`).
#[must_use]
pub fn expense_choices(state: &LedgerState) -> Vec<(AccountId, String)> {
    let mut accounts: Vec<_> = state
        .accounts
        .iter()
        .filter(|a| a.account_type == AccountType::Expense && !a.is_group && a.is_active)
        .collect();
    accounts.sort_by(|a, b| a.code.cmp(&b.code));
    accounts
        .into_iter()
        .map(|a| (a.id.clone(), format!("{}{DOT}{}", a.code, a.name)))
        .collect()
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
    /// The edit of `row`, prefilled with `(r.budget / 100).toFixed(2)`.
    #[must_use]
    pub fn new(row: &BudgetRow) -> Self {
        Self {
            line: row.line.clone(),
            label: row.row.label.clone(),
            value: to_fixed(js(row.row.budget) / 100.0, 2),
        }
    }

    /// The `budgetLine.set` command with the line's fields and the new amount
    /// (`{ ...row.line, amount }`), and the note shown once it is saved; or the amount's error
    /// (`saveEdit` in `budgets.tsx`).
    ///
    /// # Errors
    ///
    /// The message under the field.
    pub fn command(&self, base: &str) -> Result<(LedgerCommand, String), String> {
        let amount = budget_amount(&self.value)?;
        let l = &self.line;
        let command = LedgerCommand::BudgetLineSet {
            input: NewBudgetLine {
                id: Some(l.id.clone()),
                budget_id: l.budget_id.clone(),
                account_id: or_null(l.account_id.clone()),
                fund_id: or_null(l.fund_id.clone()),
                grant_id: or_null(l.grant_id.clone()),
                dimension_id: or_null(l.dimension_id.clone()),
                period_id: or_null(l.period_id.clone()),
                amount: js(amount),
            },
        };
        let saved = format!(
            "Budget for {} set to {base} {}.",
            self.label,
            format_compact(amount)
        );
        Ok((command, saved))
    }
}
